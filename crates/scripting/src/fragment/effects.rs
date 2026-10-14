//! The effect interpreter: reference resolution plus `apply_effect` and its
//! quest-scoped and batch wrappers.
//!
//! Split out of `fragment.rs` (#3854).

use super::*;
use crate::condition::entity_global_form_id;
use crate::translate::effects::{FragmentGuard, ValueExpr};

/// Resolve an effect's [`QuestRef`] to a concrete quest FormID.
/// `Self`/`GetOwningQuest` resolve to the advancing quest; a
/// `Quest Property` resolves through the quest's VMAD (when supplied).
fn resolve_quest(
    via: &QuestRef,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
) -> Option<QuestFormId> {
    match via {
        QuestRef::SelfRef | QuestRef::OwningQuest => Some(context),
        // M47.2 — through the `::Name_var` normalization `ObjectRef` already
        // had; a decompiled fragment reads an auto-property through its
        // backing variable.
        QuestRef::Property(name) => vmad?
            .scripts
            .iter()
            .find_map(|s| s.object_form_id(crate::translate::compose::vmad_property_name(name)))
            .map(QuestFormId),
    }
}

/// Resolve a `Quest`/`Self`-typed effect's `QuestRef`, logging a `debug`
/// line on failure. Shared by every quest-scoped `apply_effect` arm so
/// the "skip, never guess" contract has one call site.
fn resolve_quest_logged(
    via: &QuestRef,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
) -> Option<QuestFormId> {
    let quest = resolve_quest(via, context, vmad);
    if quest.is_none() {
        log::debug!("fragment effect skipped: unresolved quest ref {via:?}");
    }
    quest
}

/// Resolve an [`ObjectRef::Property`] to its VMAD-bound FormID. Requires
/// an `Object`-typed property with `alias == -1` — an alias-bound entry
/// (`ReferenceAlias Property`) needs the quest-alias-fill subsystem to
/// resolve correctly, so it declines rather than trusting the raw
/// `form_id` sitting next to a live alias index.
///
/// #2186 — the `alias == -1` check itself now lives in
/// `ScriptInstance::object_form_id`, which `resolve_quest` also uses.
/// This function open-coded the strict match while `object_form_id`
/// stayed lax, so the two sibling resolvers disagreed; sharing one
/// accessor is what keeps them from drifting apart again.
pub(crate) fn resolve_property_form_id(
    vmad: Option<&ScriptInstanceData>,
    name: &str,
) -> Option<u32> {
    vmad?.scripts.iter().find_map(|s| s.object_form_id(name))
}

/// The number of items an [`crate::translate::effects::ItemCount`] names: the
/// literal, or the global's / run-time value truncated toward zero (Papyrus's
/// `Float` → `Int` cast). `None` — the effect skips — for an unresolved read
/// or a negative value.
fn resolve_item_count(
    count: &crate::translate::effects::ItemCount,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    stages: &QuestStageState,
    deferred: &DeferredFragmentEffects,
) -> Option<u32> {
    use crate::translate::effects::ItemCount;
    let value = match count {
        ItemCount::Literal(count) => return Some(*count),
        ItemCount::Global(global) => {
            let form_id = resolve_property_form_id(vmad, global.property_name())?;
            world
                .try_resource::<crate::Globals>()
                .and_then(|globals| globals.get(form_id))?
        }
        ItemCount::Value(value) => evaluate_value(value, context, vmad, world, stages, deferred)?,
    };
    let count = value.trunc();
    if !(0.0..=u32::MAX as f32).contains(&count) {
        log::debug!("fragment item count skipped: {count:?} = {value}");
        return None;
    }
    Some(count as u32)
}

/// Resolve an [`ObjectRef`] all the way to a live entity. Direct VMAD object
/// properties go through FormID → loaded entity; alias-bound properties go
/// through the owning quest's [`crate::scene::SceneActorBindings`] snapshot
/// (`bindings`, captured by [`DeferredFragmentEffects::new`] before the
/// quest-state guards are taken — #2660 / SCR-D6-NEW11-03 — rather than
/// acquired live here, which would nest a `SceneActorBindings` read inside
/// that scope). `debug`-logs and returns `None` at whichever hop fails — an
/// unresolved object is a runtime data fact (not loaded, not spawned yet),
/// not a shape the fragment failed to understand.
fn resolve_object(
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    context: QuestFormId,
    via: &ObjectRef,
    bindings: &crate::scene::SceneActorBindings,
) -> Option<byroredux_core::ecs::storage::EntityId> {
    if let ObjectRef::Linked { .. } = via {
        let target = linked_target(vmad, world, context, via, bindings)?;
        let entity = crate::condition::resolve_entity_by_global_form_id(world, target);
        if entity.is_none() {
            log::debug!("fragment effect skipped: linked ref {target:08X} has no live entity");
        }
        return entity;
    }
    let name = via.property_name();
    let Some(value) = vmad?
        .scripts
        .iter()
        .find_map(|script| script.property(name))
        .map(|property| &property.value)
    else {
        log::debug!("fragment effect skipped: object ref '{name}' has no VMAD binding");
        return None;
    };
    let entity = match value {
        PropertyValue::Object { form_id, alias: -1 } => {
            crate::condition::resolve_entity_by_global_form_id(world, *form_id)
        }
        PropertyValue::Object { alias, .. } if *alias >= 0 => {
            bindings.resolve(context, i32::from(*alias))
        }
        _ => None,
    };
    if entity.is_none() {
        log::debug!("fragment effect skipped: object ref '{name}' has no live entity");
    }
    entity
}

/// The global FormID an [`ObjectRef`] names — the key of the FormID-keyed
/// ledgers ([`crate::ReferenceLockState`], #4136) and the player-move
/// destination.
///
/// Direct VMAD FormID property first, alias-bound receiver second —
/// the same two-step the `Enable`/`Disable` arm uses, and for the same
/// reason: the cheap world-free path stays unchanged for the case that
/// already worked, and the alias arm is strictly additive. Returns `None`
/// when neither resolves (for a lock change the live component change still
/// stands and only its persistence is lost).
/// M47.2 — the FormID an [`ObjectRef::Linked`] names: the source
/// reference's `XLKR` link under the keyword (or the unkeyworded one), read
/// from the live source's [`crate::scene::SceneAliasCandidate`]. The target
/// itself need not be loaded.
fn linked_target(
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    context: QuestFormId,
    via: &ObjectRef,
    bindings: &crate::scene::SceneActorBindings,
) -> Option<u32> {
    let ObjectRef::Linked { from, keyword } = via else {
        return None;
    };
    let keyword = match keyword {
        Some(keyword) => resolve_property_form_id(vmad, keyword.property_name())?,
        None => 0,
    };
    let source = resolve_object(vmad, world, context, from, bindings)?;
    let target = match entity_global_form_id(world, source) {
        Some(form) => crate::linked_ref(world, form, keyword),
        None => None,
    };
    if target.is_none() {
        log::debug!("fragment effect skipped: no linked ref under keyword {keyword:08X}");
    }
    target
}

fn object_form_id(
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    context: QuestFormId,
    target: &ObjectRef,
    deferred: &DeferredFragmentEffects,
) -> Option<u32> {
    if let ObjectRef::Linked { .. } = target {
        return linked_target(vmad, world, context, target, &deferred.scene_actor_bindings);
    }
    resolve_property_form_id(vmad, target.property_name()).or_else(|| {
        let entity = resolve_object(vmad, world, context, target, &deferred.scene_actor_bindings)?;
        entity_global_form_id(world, entity)
    })
}

/// M47.2 — the reference an [`ActorRef`] names, as the FormID identity a
/// Papyrus `==` compares (`0x14` for the player); `None` for an unfilled
/// alias or a missing property — Papyrus's `None`.
fn reference_identity(
    via: &ActorRef,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    deferred: &DeferredFragmentEffects,
) -> Option<u32> {
    let player = world
        .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
        .map(|player| player.0);
    let identity = |entity: EntityId| {
        if Some(entity) == player {
            Some(0x14)
        } else {
            entity_global_form_id(world, entity)
        }
    };
    match via {
        ActorRef::Player => Some(0x14),
        ActorRef::Speaker => deferred.speaker.and_then(identity),
        ActorRef::Object(object) => resolve_property_form_id(vmad, object.property_name())
            .or_else(|| {
                let entity =
                    resolve_object(vmad, world, context, object, &deferred.scene_actor_bindings)?;
                identity(entity)
            }),
    }
}

fn resolve_actor(
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    context: QuestFormId,
    via: &ActorRef,
    bindings: &crate::scene::SceneActorBindings,
    speaker: Option<EntityId>,
) -> Option<byroredux_core::ecs::storage::EntityId> {
    match via {
        ActorRef::Player => world
            .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
            .map(|player| player.0),
        ActorRef::Object(via) => resolve_object(vmad, world, context, via, bindings),
        // M47.2 — the line's speaker, supplied by the dialogue caller. A
        // fragment run without one (a quest or scene fragment never lowers
        // this; a speaker-less test call) skips the effect.
        ActorRef::Speaker => {
            if speaker.is_none() {
                log::debug!("fragment effect skipped: no dialogue speaker in this run");
            }
            speaker
        }
    }
}

/// #5298 — the actor half of [`resolve_actor`] for effects whose runtime is
/// NPC-only (`StartCombat` / `StopCombat` / `SetUnconscious`). The
/// lowering-time check (`actor == ActorRef::Player`) is syntactic: Skyrim's
/// ubiquitous `Actor Property PlayerRef Auto` lowers as
/// `ActorRef::Object(..)` (VMAD `Object { form_id: 0x14, alias: -1 }`), and
/// since #4694 that resolves to the player entity at runtime — so the static
/// check alone let `PlayerRef.SetUnconscious(true)` clear the player's
/// `restrained` flag mid-cinematic and `PlayerRef.StartCombat` steer the
/// player's Transform (#4323's impact). Declining on the *resolved* identity
/// here catches the property spelling, a player-filled quest alias, and
/// `Game.GetPlayer()` alike. The player stays legal as a combat **target**,
/// which is a different argument of a different arm.
fn resolve_npc_actor(
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    context: QuestFormId,
    via: &ActorRef,
    bindings: &crate::scene::SceneActorBindings,
    speaker: Option<EntityId>,
) -> Option<byroredux_core::ecs::storage::EntityId> {
    let entity = resolve_actor(vmad, world, context, via, bindings, speaker)?;
    let player = world
        .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
        .map(|player| player.0);
    if player == Some(entity) {
        log::debug!(
            "fragment effect skipped: the resolved actor is the player, whose \
             combat/unconscious runtime is unmodeled (#5298)"
        );
        return None;
    }
    Some(entity)
}

pub(crate) fn actors_3d_loaded(
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    context: QuestFormId,
    actors: &[ActorRef],
    bindings: &crate::scene::SceneActorBindings,
) -> bool {
    actors.iter().all(|actor| {
        // Latent waits never carry the speaker (the lowerer declines a
        // speaker fragment with a latent tail), so none is needed here.
        resolve_actor(vmad, world, context, actor, bindings, None)
            .is_some_and(|entity| world.has::<Transform>(entity))
    })
}

fn update_actor_cinematic_state(
    world: &World,
    actor: byroredux_core::ecs::storage::EntityId,
    update: impl FnOnce(&mut crate::ActorCinematicState),
) -> bool {
    let Some(mut states) = world.query_mut::<crate::ActorCinematicState>() else {
        return false;
    };
    if states.get_mut(actor).is_none() {
        states.insert(actor, crate::ActorCinematicState::default());
    }
    let Some(state) = states.get_mut(actor) else {
        return false;
    };
    update(state);
    true
}

fn exit_cart_idle_property(seat: u8) -> Option<&'static str> {
    match seat {
        1 => Some("IdleCartPassengerAExit"),
        2 => Some("IdleCartPassengerBExit"),
        3 => Some("IdleCartPassengerDExit"),
        4 => Some("IdleCartPassengerCExit"),
        5 => Some("IdleCartDriverExit"),
        _ => None,
    }
}

#[derive(Debug)]
enum DeferredCinematicPresentationEffect {
    SetSittingRotation(f32),
    RegisterPlayerAnimationEvent {
        event: crate::CinematicAnimationEvent,
        quest: QuestFormId,
        image_space_modifiers: Vec<crate::ImageSpaceModifierApplication>,
    },
    SetInChargen {
        disable_saving: bool,
        disable_waiting: bool,
        show_controls_disabled_message: bool,
    },
    ShowRaceMenu,
    RequestSave {
        auto: bool,
    },
}

/// Resource snapshots and side effects that bracket a fragment's canonical
/// quest-stage/objective guard scope.
///
/// Construct the batch before taking those guards, then apply it after they
/// drop. This prevents nested acquisitions of `QuestDefinitionRegistry` and
/// the presentation/alias-binding resources (#2269, #2539), and — as of
/// #2660 (SCR-D6-NEW11-03) — of `SceneActorBindings` itself: every
/// `resolve_object` alias lookup previously called `world.try_resource::
/// <SceneActorBindings>()` directly, nesting a read acquisition inside the
/// `(QuestStageState, QuestObjectiveState)` scope even after #2539 stopped
/// nesting the *write* side (`mark_scene_actor_bindings_dirty`). Reads now
/// go through the snapshot below instead.
///
/// This does NOT eliminate every nested acquisition inside that scope —
/// `PlayerControlState` (3 writes) and 12 component-storage acquisitions
/// (`Inventory` for `AddItem`, `GlobalTransform`+`Transform` for `MoveTo`,
/// and others across `apply_effect`'s match arms) are direct ECS mutations
/// the effects perform, not alias lookups, and stay nested. See
/// `apply_effect`'s doc for the current complete list and the exclusive-
/// scheduling invariant that makes it safe today.
/// One queued edit to [`crate::ReferenceLockState`]. Mirrors the three
/// shapes the live `Locked` component can take after a scripted effect,
/// so the ledger and the component cannot drift (#4136).
#[derive(Debug, Clone, Copy)]
enum DeferredLockChange {
    Locked {
        lock_level: u8,
        key_form_id: Option<u32>,
    },
    Unlocked,
}

#[derive(Debug)]
pub struct DeferredFragmentEffects {
    quest_definitions: Option<crate::QuestDefinitionRegistry>,
    /// #2660 (SCR-D6-NEW11-03) — snapshot of `SceneActorBindings`, cloned
    /// before the quest-state guards so `resolve_object` can resolve
    /// alias-bound `ObjectRef`s without a nested resource acquisition.
    /// `SceneActorBindings::resolve` only reads; nothing inside the guard
    /// scope can invalidate this snapshot mid-cascade (the table is only
    /// ever rebuilt by a separate, later-scheduled system after this one's
    /// `scene_actor_bindings_dirty` flag below is applied), so a snapshot
    /// taken once at the top of the exclusive system is behaviorally
    /// identical to a live read throughout.
    pub(crate) scene_actor_bindings: crate::scene::SceneActorBindings,
    /// M47.2 — the speaker of the dialogue line whose fragment is running
    /// (`ActorRef::Speaker`). `None` for every quest / scene fragment.
    pub(crate) speaker: Option<EntityId>,
    cinematic_presentation: Vec<DeferredCinematicPresentationEffect>,
    scene_actor_bindings_dirty: bool,
    /// `(target, activator)` pairs from `Effect::Activate`, handed to
    /// [`PendingFragmentActivations`] so they are delivered at the head of
    /// the *next* frame (#2654) — see that resource's docs.
    activations: Vec<(EntityId, EntityId)>,
    pub(super) reference_enable_changes: Vec<(u32, bool)>,
    /// #4136 — scripted lock changes, FormID-keyed for the same reason
    /// `reference_enable_changes` above is: the state has to outlive the
    /// reference's cell, and a component cannot survive its own entity's
    /// despawn on cell unload.
    reference_lock_changes: Vec<(u32, DeferredLockChange)>,
    /// `Effect::SetEnemy` / `Effect::SetAlly` calls — `(faction,
    /// other_faction, faction's reaction, other's reaction)` in global
    /// FormIDs — applied to `FactionRelations` after the quest-state guards
    /// drop. That resource follows the same nested-acquisition caution as
    /// `CinematicPresentationState` above (#2269), not the "direct component
    /// storage, stays nested" exception.
    faction_relations: Vec<(
        u32,
        u32,
        byroredux_sdk::relationships::CombatReaction,
        byroredux_sdk::relationships::CombatReaction,
    )>,
    /// M47.2 — `Effect::ChangeFaction` calls, applied to `FactionRanks` and
    /// recorded in [`crate::ScriptedFactionChanges`] after the guards drop.
    faction_changes: Vec<(EntityId, u32, crate::FactionChange)>,
    /// M47.2 — `Effect::ForceAlias` overrides, recorded in
    /// [`crate::AliasOverrides`] after the quest-state guards drop (the alias
    /// refresh takes `SceneActorBindings` before `QuestStageState`).
    alias_overrides: Vec<(QuestFormId, i32, crate::AliasOverride)>,
    /// M47.2 — `Effect::SendStoryEvent` events, queued for the Story Manager
    /// after the guards drop.
    story_events: Vec<crate::story_manager::StoryEvent>,
    /// M47.2 — `Effect::Kill` requests, queued in
    /// [`crate::story_manager::PendingScriptKills`] after the guards drop.
    kills: Vec<(EntityId, Option<EntityId>, bool)>,
    /// M47.2 — `Effect::RewardPlayerXp` grants, queued in
    /// [`crate::story_manager::PendingPlayerXp`] after the guards drop.
    player_xp: Vec<(u32, bool)>,
    /// M47.2 — `Effect::IncrementStat`s, applied to [`crate::MiscStats`]
    /// after the guards drop.
    misc_stats: Vec<(String, i32)>,
    /// M47.2 — `Effect::AddAchievement` IDs, applied to
    /// [`crate::Achievements`] after the guards drop.
    achievements: Vec<i32>,
    /// M47.2 — `Effect::AdvanceSkill`s, queued on
    /// [`crate::PendingSkillAdvances`] after the guards drop.
    skill_advances: Vec<(String, f32)>,
    /// M47.2 — `Effect::SetLinkedRef`s: (source, keyword, target), applied to
    /// [`crate::LinkedRefOverrides`] after the guards drop.
    linked_ref_changes: Vec<(u32, u32, u32)>,
    /// M47.2 — `Effect::SetLocRefType`s: (reference, ref type or 0), applied to
    /// [`crate::LocRefTypeOverrides`] after the guards drop.
    loc_ref_type_changes: Vec<(u32, u32)>,
    /// M47.2 — `Effect::SetCrimeFaction`s: (actor reference, faction or 0),
    /// applied to [`crate::CrimeFactionOverrides`] after the guards drop.
    pub(crate) crime_faction_changes: Vec<(u32, u32)>,
    /// M47.2 — `SetBribed` / `SetIntimidated` calls: (actor reference, outcome,
    /// flagged), applied to [`crate::SpeechcraftOutcomes`] after the guards drop.
    pub(crate) speechcraft_changes: Vec<(u32, crate::SpeechcraftOutcome, bool)>,
    /// M47.2 — `ChangeAnimArchetype` calls: (actor reference, keyword or 0),
    /// applied to [`crate::AnimArchetypeOverrides`] after the guards drop.
    pub(crate) anim_archetype_changes: Vec<(u32, u32)>,
    /// M47.2 — form list edits a run made, applied to
    /// [`crate::FormListAdditions`] after the guards drop (and visible to the
    /// run's own later reads).
    pub(crate) form_list_changes: Vec<crate::FormListChange>,
    /// M47.2 — the handle a run draws `PlaceAtMe` FormIDs through, taken
    /// before the quest-state guards so the VM never locks
    /// [`crate::CreatedReferences`].
    pub(super) scripted_ids: Option<crate::ScriptedIds>,
    /// M47.2 — the owner ids of the alias script instances and what each is
    /// attached to, snapshotted so a run knows what `Self` is.
    pub(super) alias_owners: Arc<HashMap<u32, (u32, i16)>>,
    /// M47.2 — `PlaceAtMe` creations: (FormID, base, placed at,
    /// delete when able), recorded on [`crate::CreatedReferences`] after the
    /// guards drop.
    pub(super) created_references: Vec<(u32, u32, u32, bool)>,
    /// M47.2 — follower-flag writes: (actor FormID, teammate (with can-do-
    /// favor) or doing-favor), applied to [`crate::FollowerState`] after the
    /// guards drop.
    follower_changes: Vec<(u32, FollowerChange)>,
    /// M47.2 — distance registrations a Papyrus VM run asked for: the owner,
    /// the script, the change.
    pub(super) distance_requests: Vec<(u32, String, super::DistanceRequest)>,
    /// M47.2 — hit event registration changes, applied to
    /// [`super::HitRegistrations`] after the guards drop.
    pub(super) hit_requests: Vec<(u32, String, super::HitRequest)>,
    /// `RegisterForAnimationEvent` changes, applied to
    /// [`super::AnimationEventRegistrations`] after the guards drop.
    pub(super) animation_event_requests: Vec<(u32, String, super::AnimationEventRequest)>,
    /// M47.2 — `Sound.Play` calls, queued on [`crate::PendingSoundPlays`] after
    /// the guards drop.
    pub(super) sound_plays: Vec<crate::SoundPlay>,
    /// `Game.ShakeCamera` / `Game.ShakeController` calls, placed as the player's
    /// transient command markers once the guards drop.
    pub(super) camera_shakes: Vec<crate::papyrus_demo::CameraShakeCommand>,
    pub(super) controller_shakes: Vec<crate::papyrus_demo::ControllerRumbleCommand>,
    /// `ProcessTrapHit` calls: `(target, trap, damage)`, queued as the target's
    /// `HitEvent` once the guards drop.
    pub(super) trap_hits: Vec<(byroredux_core::ecs::EntityId, byroredux_core::ecs::EntityId, f32)>,
    /// M47.2 — `SetNoFavorAllowed` / `SetActorCause` calls, applied to
    /// [`crate::ReferenceFlags`] after the guards drop.
    pub(super) reference_flag_changes: Vec<crate::ReferenceFlagChange>,
    /// M47.2 — `SetActorOwner` / `SetFactionOwner` calls: (reference, owner or
    /// 0), applied to [`crate::OwnerOverrides`] after the guards drop.
    pub(crate) owner_changes: Vec<(u32, u32)>,
    /// M47.2 — faction crime gold edits, applied to [`crate::FactionCrimeGold`]
    /// after the guards drop (and visible to the run's own later reads).
    pub(crate) crime_gold_changes: Vec<crate::CrimeGoldChange>,
    /// M47.2 — `Effect::SetLocationKeywordData`s: (location, keyword, data),
    /// applied to [`crate::LocationKeywordData`] after the guards drop.
    location_keyword_changes: Vec<(u32, u32, f32)>,
    /// M47.2 — reference-collection members a Papyrus VM run wrote back,
    /// applied to [`crate::AliasCollections`] after the guards drop.
    pub(super) alias_collections: Vec<(crate::alias_collections::CollectionKey, Vec<u32>)>,
    /// M47.2 — custom / remote event registration changes a Papyrus VM run
    /// asked for: the owner, the script, the change.
    pub(super) event_requests: Vec<(u32, String, super::EventRequest)>,
    /// Which collection aliases the quests leave to their scripts, taken
    /// before any quest-state lock (see [`crate::CollectionAliasKinds`]).
    pub(super) collection_kinds: std::sync::Arc<HashMap<crate::alias_collections::CollectionKey, bool>>,
    /// M47.2 — `Effect::SetLocal` values for this fragment run.
    locals: HashMap<String, f32>,
    /// M47.2 — `Effect::SetReferenceVariable` writes, applied to the
    /// references' `ScriptVariables` after the guards drop.
    reference_variables: Vec<(EntityId, String, f32)>,
    /// M47.2 — messages to show, formatted after the guards drop (text
    /// replacement reads the alias bindings).
    messages: Vec<PendingMessage>,
    /// M47.2 — the vendor an `Effect::ShowBarterMenu` asked to trade with.
    barter: Option<EntityId>,
    /// M47.2 — `Effect::DeleteWhenAble` references, flagged in
    /// [`crate::CreatedReferences`] after the guards drop.
    deletions: Vec<u32>,
    /// M47.2 — `Effect::ChangeKeyword`s, applied to
    /// [`crate::keywords::ScriptedKeywords`] after the guards drop.
    keyword_changes: Vec<(EntityId, u32, crate::keywords::KeywordChange)>,
    /// M47.2 — `Effect::AddToMap` additions, recorded in
    /// [`crate::MapMarkers`] after the guards drop.
    pub(crate) map_markers: Vec<(u32, bool)>,
    /// M47.2 — `SetScanned` / `AddToMapScanned` flags: (reference, scanned),
    /// applied to [`crate::ScannedReferences`] after the guards drop.
    pub(crate) scanned_changes: Vec<(u32, bool)>,
    /// M47.2 — `Effect::SetLocationCleared` changes, recorded in
    /// [`crate::LocationClearState`] after the guards drop.
    location_clears: Vec<(u32, bool)>,
    /// M47.2 — `Effect::SetBaseProtection` writes `(base, protected,
    /// value)`, recorded in [`crate::ActorBaseProtectionOverrides`] after
    /// the guards drop.
    base_protection: Vec<(u32, bool, bool)>,
    /// M47.2 — `Effect::SetRelationshipRank` writes by base pair, recorded in
    /// [`crate::ScriptedRelationships`] after the guards drop.
    relationship_changes: Vec<((u32, u32), i8)>,
    /// M47.2 — `Effect::BlockActivation` changes, recorded in
    /// [`super::ReferenceActivationBlocks`] after the guards drop.
    activation_blocks: Vec<(u32, bool)>,
    /// M47.2 — the scenes playing when the fragment started, for
    /// `FragmentGuard::ScenePlaying`: snapshotted before the quest-state
    /// guards (scene playback takes `ScenePlayer` before `QuestStageState`).
    running_scenes: std::collections::HashSet<u32>,
    /// M47.2 — every quest's VMAD, for `Effect::QuestScope`: snapshotted (an
    /// `Arc` clone) before the quest-state guards.
    pub(super) quest_scripts: std::sync::Arc<HashMap<QuestFormId, ScriptInstanceData>>,
    /// M47.2 — the Papyrus VM's scripts ([`QuestStageFragments::vm_scripts`]),
    /// snapshotted with the quest VMADs.
    pub(super) vm_scripts: std::sync::Arc<HashMap<String, std::sync::Arc<byroredux_papyrus::ast::Script>>>,
    /// M47.2 — `Effect::SetQuestVariable` writes, applied to
    /// [`crate::QuestScriptVariables`] after the quest-state guards drop.
    pub(super) quest_variables: Vec<(QuestFormId, byroredux_plugin::esm::records::condition::ConditionStringId, f32)>,
    /// M47.2 — the Papyrus VM's object script-variable writes, applied to
    /// [`crate::QuestScriptObjects`] after the quest-state guards drop.
    pub(super) quest_objects: Vec<(
        QuestFormId,
        byroredux_plugin::esm::records::condition::ConditionStringId,
        crate::ScriptObject,
    )>,
    /// M47.2 — update registrations Papyrus VM runs asked for: the owner,
    /// the script, the change.
    pub(super) script_updates: Vec<(u32, String, super::UpdateRequest)>,
    provider_steps: Vec<DeferredProviderFragmentStep>,
}

#[derive(Debug)]
struct DeferredProviderFragmentStep {
    call: crate::translate::effects::FragmentProviderCall,
    context: QuestFormId,
    vmad: Option<ScriptInstanceData>,
    tail: Vec<Effect>,
}

impl DeferredFragmentEffects {
    /// Snapshot fragment resources before acquiring the quest-state guards.
    ///
    /// Production callers take this snapshot before acquiring quest-state
    /// guards. `QuestDefinitionRegistry::clone()` is an O(1) `Arc`
    /// refcount bump (#2659 / SCR-D6-NEW11-02), not a deep copy — that's
    /// what keeps each fragment snapshot cheap on every load-order size.
    /// `SceneActorBindings::clone()` (#2660) is a real `HashMap` clone, but
    /// the table only holds bound aliases for currently-active quests, not
    /// the whole load order — negligible next to the per-frame ECS work
    /// this system already does.
    pub fn new(world: &World) -> Self {
        Self {
            quest_definitions: world
                .try_resource::<crate::QuestDefinitionRegistry>()
                .map(|definitions| definitions.clone()),
            scene_actor_bindings: world
                .try_resource::<crate::scene::SceneActorBindings>()
                .map(|bindings| bindings.clone())
                .unwrap_or_default(),
            speaker: None,
            cinematic_presentation: Vec::new(),
            scene_actor_bindings_dirty: false,
            activations: Vec::new(),
            reference_enable_changes: Vec::new(),
            reference_lock_changes: Vec::new(),
            faction_relations: Vec::new(),
            quest_variables: Vec::new(),
            quest_objects: Vec::new(),
            script_updates: Vec::new(),
            faction_changes: Vec::new(),
            alias_overrides: Vec::new(),
            activation_blocks: Vec::new(),
            map_markers: Vec::new(),
            location_clears: Vec::new(),
            base_protection: Vec::new(),
            relationship_changes: Vec::new(),
            story_events: Vec::new(),
            kills: Vec::new(),
            player_xp: Vec::new(),
            misc_stats: Vec::new(),
            achievements: Vec::new(),
            skill_advances: Vec::new(),
            linked_ref_changes: Vec::new(),
            loc_ref_type_changes: Vec::new(),
            crime_faction_changes: Vec::new(),
            speechcraft_changes: Vec::new(),
            anim_archetype_changes: Vec::new(),
            scanned_changes: Vec::new(),
            form_list_changes: Vec::new(),
            scripted_ids: world
                .try_resource::<crate::CreatedReferences>()
                .map(|created| created.scripted_ids()),
            created_references: Vec::new(),
            alias_owners: world
                .try_resource::<crate::AliasScriptOwners>()
                .map(|owners| owners.table())
                .unwrap_or_default(),
            follower_changes: Vec::new(),
            distance_requests: Vec::new(),
            hit_requests: Vec::new(),
            animation_event_requests: Vec::new(),
            sound_plays: Vec::new(),
            camera_shakes: Vec::new(),
            controller_shakes: Vec::new(),
            trap_hits: Vec::new(),
            reference_flag_changes: Vec::new(),
            owner_changes: Vec::new(),
            crime_gold_changes: Vec::new(),
            location_keyword_changes: Vec::new(),
            alias_collections: Vec::new(),
            event_requests: Vec::new(),
            collection_kinds: world
                .try_resource::<crate::CollectionAliasKinds>()
                .map(|kinds| std::sync::Arc::clone(&kinds.0))
                .unwrap_or_default(),
            locals: HashMap::new(),
            reference_variables: Vec::new(),
            messages: Vec::new(),
            barter: None,
            deletions: Vec::new(),
            keyword_changes: Vec::new(),
            quest_scripts: world
                .try_resource::<super::QuestScriptInstances>()
                .map(|instances| instances.0.clone())
                .unwrap_or_default(),
            vm_scripts: world
                .try_resource::<super::QuestStageFragments>()
                .map(|fragments| fragments.vm_scripts())
                .unwrap_or_default(),
            running_scenes: world
                .query::<crate::scene::ScenePlayer>()
                .map(|players| {
                    players
                        .iter()
                        .filter(|(_, player)| player.is_running())
                        .map(|(_, player)| player.scene_form_id)
                        .collect()
                })
                .unwrap_or_default(),
            provider_steps: Vec::new(),
        }
    }

    /// Apply queued mutations after releasing the `QuestStageState` /
    /// `QuestObjectiveState` guards passed to [`apply_effects`].
    pub fn apply(self, world: &World) -> Vec<QuestStageAdvanced> {
        self.apply_at_depth(world, 0)
    }

    fn apply_at_depth(mut self, world: &World, depth: usize) -> Vec<QuestStageAdvanced> {
        // #3946 — the barrier cap used to be tested here, and returned
        // before the four non-provider flushes below. That was partial
        // application: the `deferred` arriving at this depth was filled by
        // a tail whose `stages`/`objectives` writes had *already been
        // committed* under the caller's guards, so bailing here advanced
        // the quest while silently discarding the scene-binding,
        // activation, reference-enable and cinematic effects that were
        // queued alongside it. The cap now gates the recursion at its
        // source, below, the way `MAX_CASCADE` gates the cascade loop:
        // by declining to start work it cannot finish. Depth can therefore
        // never reach the cap here.
        debug_assert!(
            depth < MAX_PROVIDER_FRAGMENT_BARRIERS,
            "recursion is gated before descending; depth {depth} should be unreachable"
        );
        for message in std::mem::take(&mut self.messages) {
            match message {
                PendingMessage::Show(form_id, args) => {
                    if !crate::messages::show_message(world, form_id, &args) {
                        log::debug!("fragment Show skipped: no message {form_id:08X}");
                    }
                }
                PendingMessage::Notify(text) => crate::messages::notify(world, text),
            }
        }
        for form_id in std::mem::take(&mut self.deletions) {
            let created = world
                .try_resource_mut::<crate::CreatedReferences>()
                .is_some_and(|mut created| created.delete_when_able(form_id));
            if !created {
                // CK *Delete*: "Only references created in game can actually
                // be deleted."
                log::debug!("fragment DeleteWhenAble: {form_id:08X} is not a created reference");
            }
        }
        if let Some(vendor) = self.barter.take() {
            crate::menus::request_barter(world, vendor);
        }
        for (object, keyword, change) in std::mem::take(&mut self.keyword_changes) {
            if !crate::keywords::change_keyword(world, object, keyword, change) {
                log::debug!("fragment {change:?}Keyword skipped: the reference has no FormID");
            }
        }
        if !self.reference_variables.is_empty() {
            if let Some(mut variables) = world.query_mut::<crate::vm_state::ScriptVariables>() {
                for (entity, variable, value) in std::mem::take(&mut self.reference_variables) {
                    if variables.get_mut(entity).is_none() {
                        variables.insert(entity, crate::vm_state::ScriptVariables::default());
                    }
                    variables
                        .get_mut(entity)
                        .expect("present or inserted")
                        .set_by_name(&variable, value);
                }
            }
        }
        if !self.event_requests.is_empty() {
            if let Some(mut registrations) = world.try_resource_mut::<super::EventRegistrations>() {
                for (owner, script, request) in self.event_requests.drain(..) {
                    registrations.request(owner, &script, &request);
                }
            }
            self.event_requests.clear();
        }
        if !self.alias_collections.is_empty() {
            if let Some(mut collections) = world.try_resource_mut::<crate::AliasCollections>() {
                for (key, members) in self.alias_collections.drain(..) {
                    collections.set(key, members);
                }
            }
            self.alias_collections.clear();
        }
        if !self.location_keyword_changes.is_empty() {
            if let Some(mut data) = world.try_resource_mut::<crate::LocationKeywordData>() {
                for (location, keyword, value) in self.location_keyword_changes.drain(..) {
                    data.set(location, keyword, value);
                }
            }
            self.location_keyword_changes.clear();
        }
        if !self.reference_flag_changes.is_empty() {
            if let Some(mut flags) = world.try_resource_mut::<crate::ReferenceFlags>() {
                for change in self.reference_flag_changes.drain(..) {
                    flags.apply(change);
                }
            }
            self.reference_flag_changes.clear();
        }
        if !self.owner_changes.is_empty() {
            if let Some(mut owners) = world.try_resource_mut::<crate::OwnerOverrides>() {
                for (reference, owner) in self.owner_changes.drain(..) {
                    owners.set(reference, owner);
                }
            }
            self.owner_changes.clear();
        }
        if !self.crime_gold_changes.is_empty() {
            if let Some(mut gold) = world.try_resource_mut::<crate::FactionCrimeGold>() {
                for change in self.crime_gold_changes.drain(..) {
                    gold.apply(change);
                }
            }
            self.crime_gold_changes.clear();
        }
        if !self.camera_shakes.is_empty() || !self.controller_shakes.is_empty() {
            let player = world
                .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
                .map(|player| player.0);
            if let Some(player) = player {
                if let Some(mut shakes) = world.query_mut::<crate::papyrus_demo::CameraShakeCommand>() {
                    if let Some(shake) = self.camera_shakes.last() {
                        shakes.insert(player, *shake);
                    }
                }
                if let Some(mut shakes) = world.query_mut::<crate::papyrus_demo::ControllerRumbleCommand>() {
                    if let Some(shake) = self.controller_shakes.last() {
                        shakes.insert(player, *shake);
                    }
                }
            }
            self.camera_shakes.clear();
            self.controller_shakes.clear();
        }
        if !self.trap_hits.is_empty() {
            if let Some(mut events) = world.query_mut::<crate::HitEvent>() {
                for (target, trap, damage) in self.trap_hits.drain(..) {
                    events.insert(
                        target,
                        crate::HitEvent {
                            aggressor: trap,
                            source: trap,
                            projectile: 0,
                            damage,
                            power_attack: false,
                            sneak_attack: false,
                            bash_attack: false,
                            blocked: false,
                        },
                    );
                }
            }
            self.trap_hits.clear();
        }
        if !self.sound_plays.is_empty() {
            if let Some(mut pending) = world.try_resource_mut::<crate::PendingSoundPlays>() {
                pending.0.append(&mut self.sound_plays);
            }
            self.sound_plays.clear();
        }
        if !self.hit_requests.is_empty() {
            if let Some(mut registrations) = world.try_resource_mut::<super::HitRegistrations>() {
                for (owner, script, request) in self.hit_requests.drain(..) {
                    registrations.request(owner, &script, &request);
                }
            }
            self.hit_requests.clear();
        }
        if !self.animation_event_requests.is_empty() {
            if let Some(mut registrations) = world.try_resource_mut::<super::AnimationEventRegistrations>() {
                for (owner, script, request) in self.animation_event_requests.drain(..) {
                    registrations.request(owner, &script, &request);
                }
            }
            self.animation_event_requests.clear();
        }
        if !self.distance_requests.is_empty() {
            if let Some(mut watches) = world.try_resource_mut::<super::DistanceWatches>() {
                for (owner, script, request) in self.distance_requests.drain(..) {
                    watches.request(owner, &script, request);
                }
            }
            self.distance_requests.clear();
        }
        if !self.follower_changes.is_empty() {
            if let Some(mut state) = world.try_resource_mut::<crate::FollowerState>() {
                for (actor, change) in self.follower_changes.drain(..) {
                    match change {
                        FollowerChange::Teammate { teammate, can_do_favor } => {
                            state.set_teammate(actor, teammate, can_do_favor)
                        }
                        FollowerChange::DoingFavor(doing) => state.set_doing_favor(actor, doing),
                    }
                }
            }
            self.follower_changes.clear();
        }
        if !self.created_references.is_empty() {
            match world.try_resource_mut::<crate::CreatedReferences>() {
                Some(mut created) => {
                    for (form_id, base, at, delete_when_able) in self.created_references.drain(..) {
                        created.create_scripted(form_id, base, at, delete_when_able);
                    }
                }
                None => log::debug!("PlaceAtMe dropped: CreatedReferences is unavailable"),
            }
        }
        if !self.linked_ref_changes.is_empty() {
            if let Some(mut links) = world.try_resource_mut::<crate::LinkedRefOverrides>() {
                for (source, keyword, target) in self.linked_ref_changes.drain(..) {
                    links.set(source, keyword, target);
                }
            }
            self.linked_ref_changes.clear();
        }
        if !self.crime_faction_changes.is_empty() {
            if let Some(mut overrides) = world.try_resource_mut::<crate::CrimeFactionOverrides>() {
                for (actor, faction) in self.crime_faction_changes.drain(..) {
                    overrides.set(actor, faction);
                }
            }
            self.crime_faction_changes.clear();
        }
        if !self.speechcraft_changes.is_empty() {
            if let Some(mut outcomes) = world.try_resource_mut::<crate::SpeechcraftOutcomes>() {
                for (actor, outcome, flagged) in self.speechcraft_changes.drain(..) {
                    outcomes.set(actor, outcome, flagged);
                }
            }
            self.speechcraft_changes.clear();
        }
        if !self.scanned_changes.is_empty() {
            if let Some(mut scanned) = world.try_resource_mut::<crate::ScannedReferences>() {
                for (reference, flagged) in self.scanned_changes.drain(..) {
                    scanned.set(reference, flagged);
                }
            }
            self.scanned_changes.clear();
        }
        if !self.anim_archetype_changes.is_empty() {
            if let Some(mut overrides) = world.try_resource_mut::<crate::AnimArchetypeOverrides>() {
                for (actor, keyword) in self.anim_archetype_changes.drain(..) {
                    overrides.set(actor, keyword);
                }
            }
            self.anim_archetype_changes.clear();
        }
        if !self.form_list_changes.is_empty() {
            if let Some(mut additions) = world.try_resource_mut::<crate::FormListAdditions>() {
                for change in self.form_list_changes.drain(..) {
                    additions.apply(change);
                }
            }
            self.form_list_changes.clear();
        }
        if !self.loc_ref_type_changes.is_empty() {
            if let Some(mut types) = world.try_resource_mut::<crate::LocRefTypeOverrides>() {
                for (reference, ref_type) in self.loc_ref_type_changes.drain(..) {
                    types.set(reference, ref_type);
                }
            }
            self.loc_ref_type_changes.clear();
            // An alias may now fill by the new type.
            crate::scene::mark_scene_actor_bindings_dirty(world);
        }
        if !self.skill_advances.is_empty() {
            if let Some(mut pending) = world.try_resource_mut::<crate::PendingSkillAdvances>() {
                pending.0.append(&mut self.skill_advances);
            }
            self.skill_advances.clear();
        }
        if !self.achievements.is_empty() {
            if let Some(mut awarded) = world.try_resource_mut::<crate::Achievements>() {
                for id in self.achievements.drain(..) {
                    if awarded.award(id) {
                        log::info!("achievement {id} awarded");
                    }
                }
            }
            self.achievements.clear();
        }
        if !self.misc_stats.is_empty() {
            if let Some(mut stats) = world.try_resource_mut::<crate::MiscStats>() {
                for (stat, amount) in self.misc_stats.drain(..) {
                    stats.increment(&stat, amount);
                }
            }
            self.misc_stats.clear();
        }
        if !self.player_xp.is_empty() {
            if let Some(mut pending) =
                world.try_resource_mut::<crate::story_manager::PendingPlayerXp>()
            {
                pending.0.append(&mut self.player_xp);
            }
        }
        if !self.kills.is_empty() {
            if let Some(mut pending) =
                world.try_resource_mut::<crate::story_manager::PendingScriptKills>()
            {
                pending.0.append(&mut self.kills);
            }
        }
        if !self.story_events.is_empty() {
            if let Some(mut pending) =
                world.try_resource_mut::<crate::story_manager::PendingScriptStoryEvents>()
            {
                pending.0.append(&mut self.story_events);
            }
        }
        if !self.map_markers.is_empty() {
            if let Some(mut markers) = world.try_resource_mut::<crate::MapMarkers>() {
                for (marker, fast_travel) in std::mem::take(&mut self.map_markers) {
                    markers.add(marker, fast_travel);
                }
            }
        }
        if !self.relationship_changes.is_empty() {
            if let Some(mut ranks) = world.try_resource_mut::<crate::ScriptedRelationships>() {
                for (pair, rank) in std::mem::take(&mut self.relationship_changes) {
                    ranks.0.insert(pair, rank);
                }
            }
        }
        if !self.base_protection.is_empty() {
            if let Some(mut overrides) = world.try_resource_mut::<crate::ActorBaseProtectionOverrides>()
            {
                for (base, protected, value) in std::mem::take(&mut self.base_protection) {
                    overrides.set(base, protected, value);
                }
            }
        }
        if !self.location_clears.is_empty() {
            if let Some(mut state) = world.try_resource_mut::<crate::LocationClearState>() {
                for (location, cleared) in std::mem::take(&mut self.location_clears) {
                    state.set_cleared(location, cleared);
                }
            }
        }
        if !self.activation_blocks.is_empty() {
            if let Some(mut blocks) = world.try_resource_mut::<super::ReferenceActivationBlocks>() {
                for (reference, blocked) in std::mem::take(&mut self.activation_blocks) {
                    blocks.set_blocked(reference, blocked);
                }
            }
        }
        if !self.alias_overrides.is_empty() {
            if let Some(mut overrides) = world.try_resource_mut::<crate::AliasOverrides>() {
                for (quest, alias_id, value) in std::mem::take(&mut self.alias_overrides) {
                    overrides.set(quest, alias_id, value);
                }
            }
        }
        if self.scene_actor_bindings_dirty {
            crate::scene::mark_scene_actor_bindings_dirty(world);
        }
        if !self.activations.is_empty() {
            match world.try_resource_mut::<PendingFragmentActivations>() {
                Some(mut pending) => pending.0.append(&mut self.activations),
                None => log::debug!(
                    "fragment Activate dropped: PendingFragmentActivations is unavailable"
                ),
            }
        }
        if !self.reference_enable_changes.is_empty() {
            if let Some(mut state) = world.try_resource_mut::<ReferenceEnableState>() {
                for (form_id, enabled) in self.reference_enable_changes.drain(..) {
                    state.set_enabled(form_id, enabled);
                }
            }
        }
        if !self.reference_lock_changes.is_empty() {
            match world.try_resource_mut::<crate::ReferenceLockState>() {
                Some(mut state) => {
                    for (form_id, change) in self.reference_lock_changes.drain(..) {
                        match change {
                            DeferredLockChange::Locked {
                                lock_level,
                                key_form_id,
                            } => state.set_locked(form_id, lock_level, key_form_id),
                            DeferredLockChange::Unlocked => state.set_unlocked(form_id),
                        }
                    }
                }
                None => log::debug!(
                    "fragment lock change dropped from the persistent ledger: \
                     ReferenceLockState is unavailable — the live component still \
                     changed, but the change will not survive a cell reload (#4136)"
                ),
            }
        }
        if !self.cinematic_presentation.is_empty() {
            if let Some(mut state) = world.try_resource_mut::<crate::CinematicPresentationState>() {
                for effect in self.cinematic_presentation.drain(..) {
                    match effect {
                        DeferredCinematicPresentationEffect::SetSittingRotation(degrees) => {
                            state.sitting_rotation_degrees = degrees;
                        }
                        DeferredCinematicPresentationEffect::RegisterPlayerAnimationEvent {
                            event,
                            quest,
                            image_space_modifiers,
                        } => {
                            state.register_player_animation_event(
                                event,
                                quest,
                                image_space_modifiers,
                            );
                        }
                        DeferredCinematicPresentationEffect::SetInChargen {
                            disable_saving,
                            disable_waiting,
                            show_controls_disabled_message,
                        } => {
                            state.set_in_chargen(
                                disable_saving,
                                disable_waiting,
                                show_controls_disabled_message,
                            );
                        }
                        DeferredCinematicPresentationEffect::ShowRaceMenu => {
                            state.show_race_menu();
                        }
                        DeferredCinematicPresentationEffect::RequestSave { auto } => {
                            state.request_save(auto);
                        }
                    }
                }
            }
        }
        if !self.faction_changes.is_empty() {
            let player = world
                .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
                .map(|player| player.0);
            for (actor, faction, change) in std::mem::take(&mut self.faction_changes) {
                // `RemoveFromAllFactions` names no faction: the ledger records
                // each one the actor was in as removed.
                let left: Vec<u32> = match change {
                    crate::FactionChange::RemoveAll => world
                        .get::<byroredux_core::ecs::components::FactionRanks>(actor)
                        .map(|ranks| ranks.0.iter().map(|(faction, _)| *faction).collect())
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                let rank = {
                    let Some(mut storage) =
                        world.query_mut::<byroredux_core::ecs::components::FactionRanks>()
                    else {
                        log::debug!("faction change skipped: FactionRanks never registered");
                        continue;
                    };
                    if storage.get_mut(actor).is_none() {
                        storage.insert(actor, byroredux_core::ecs::components::FactionRanks::new());
                    }
                    change.apply(storage.get_mut(actor).expect("present or inserted"), faction)
                };
                // The ledger key: PlayerRef for the player, else the placed
                // reference's FormID. An actor with neither keeps the live
                // edit only (nothing re-derives it under a stable identity).
                let reference = if player == Some(actor) {
                    Some(crate::PLAYER_REFERENCE)
                } else {
                    entity_global_form_id(world, actor)
                };
                match reference {
                    Some(reference) => {
                        if let Some(mut ledger) = world.try_resource_mut::<crate::ScriptedFactionChanges>() {
                            if change == crate::FactionChange::RemoveAll {
                                for faction in left {
                                    ledger.record(reference, faction, None);
                                }
                            } else {
                                ledger.record(reference, faction, rank);
                            }
                        }
                    }
                    None => log::debug!(
                        "faction change on {actor:?} not persisted: the actor has no reference identity"
                    ),
                }
            }
        }
        if !self.quest_variables.is_empty() {
            match world.try_resource_mut::<crate::QuestScriptVariables>() {
                Some(mut variables) => {
                    for (quest, variable, value) in self.quest_variables.drain(..) {
                        variables.set(quest, variable, value);
                    }
                }
                None => {
                    log::debug!(
                        "{} quest-variable write(s) dropped: QuestScriptVariables not installed",
                        self.quest_variables.len()
                    );
                    self.quest_variables.clear();
                }
            }
        }
        if !self.quest_objects.is_empty() {
            match world.try_resource_mut::<crate::QuestScriptObjects>() {
                Some(mut objects) => {
                    for (quest, variable, value) in self.quest_objects.drain(..) {
                        objects.set(quest, variable, value);
                    }
                }
                None => {
                    log::debug!("object script-variable writes dropped: QuestScriptObjects not installed");
                    self.quest_objects.clear();
                }
            }
        }
        if !self.script_updates.is_empty() {
            match world.try_resource_mut::<super::ScriptUpdates>() {
                Some(mut updates) => {
                    for (owner, script, request) in self.script_updates.drain(..) {
                        updates.request(owner, &script, request);
                    }
                }
                None => {
                    log::debug!("update registrations dropped: ScriptUpdates not installed");
                    self.script_updates.clear();
                }
            }
        }
        if !self.faction_relations.is_empty() {
            // #4414 — the relation is saved, so it is keyed by portable
            // identity. Resolve under the identity guard alone, then take
            // the relations guard: never both held at once.
            let resolved = match world.try_resource::<crate::LoadOrderIdentity>() {
                Some(identity) => self
                    .faction_relations
                    .drain(..)
                    .filter_map(|(faction, other, reaction, other_reaction)| {
                        let pair = identity.form_ref(faction).zip(identity.form_ref(other));
                        if pair.is_none() {
                            log::warn!(
                                "SetEnemy/SetAlly {faction:08X} -> {other:08X} dropped: a \
                                 faction has no owning plugin in the load order"
                            );
                        }
                        pair.map(|(faction, other)| (faction, other, reaction, other_reaction))
                    })
                    .collect::<Vec<_>>(),
                None => {
                    log::warn!(
                        "{} SetEnemy/SetAlly call(s) dropped: no LoadOrderIdentity",
                        self.faction_relations.len()
                    );
                    self.faction_relations.clear();
                    Vec::new()
                }
            };
            if let Some(mut relations) = world.try_resource_mut::<crate::FactionRelations>() {
                for (faction, other, reaction, other_reaction) in resolved {
                    relations.set_reaction(faction, other, reaction);
                    relations.set_reaction(other, faction, other_reaction);
                }
            }
        }
        if self.provider_steps.is_empty() {
            return Vec::new();
        }
        let Some(callback) = world
            .try_resource::<crate::PapyrusProviderRuntime>()
            .and_then(|runtime| runtime.callback())
        else {
            log::warn!("deferred fragment provider calls dropped: provider host is unavailable");
            return Vec::new();
        };

        // Loop-invariant in `depth`, so it is computed once and warns once
        // rather than per remaining step.
        let at_barrier_limit = depth + 1 >= MAX_PROVIDER_FRAGMENT_BARRIERS;
        if at_barrier_limit {
            log::warn!(
                "fragment provider continuation reached {MAX_PROVIDER_FRAGMENT_BARRIERS} \
                 barriers; declining deeper tails (their effects are not applied)"
            );
        }

        let mut advances = Vec::new();
        for step in std::mem::take(&mut self.provider_steps) {
            if let Err(error) = callback(
                step.call.principal.as_ref(),
                &step.call.route,
                &step.call.arguments,
            ) {
                log::warn!("deferred fragment provider call aborted: {error}");
                continue;
            }
            if step.tail.is_empty() {
                continue;
            }
            if at_barrier_limit {
                // Skip the tail *whole*. Running `apply_effects` first and
                // bailing afterwards is what produced the partial state
                // this fix removes: the provider call above is the barrier
                // itself and has already happened, but nothing downstream
                // of it is committed.
                continue;
            }
            let mut deferred = Self::new(world);
            let resumed = {
                let (mut stages, mut objectives) =
                    world.resource_2_mut::<QuestStageState, QuestObjectiveState>();
                apply_effects(
                    &step.tail,
                    step.context,
                    step.vmad.as_ref(),
                    world,
                    &mut stages,
                    &mut objectives,
                    &mut deferred,
                )
            };
            advances.extend(resumed);
            advances.extend(deferred.apply_at_depth(world, depth + 1));
        }
        advances
    }
}

/// A follower-flag write a fragment queued.
#[derive(Debug, Clone, Copy)]
pub(super) enum FollowerChange {
    Teammate { teammate: bool, can_do_favor: bool },
    DoingFavor(bool),
}

/// Execute one fragment as an ordered unit. Any provider barrier is flushed
/// after releasing quest-state guards before the caller starts another
/// independent fragment.
pub(crate) fn apply_fragment_guard_free(
    world: &World,
    effects: &[Effect],
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
) -> Vec<QuestStageAdvanced> {
    apply_fragment_guard_free_as(world, effects, context, vmad, None)
}

/// [`apply_fragment_guard_free`] for a dialogue line's fragment, with the
/// line's `speaker` bound for `ActorRef::Speaker` (M47.2).
pub(crate) fn apply_fragment_guard_free_as(
    world: &World,
    effects: &[Effect],
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    speaker: Option<EntityId>,
) -> Vec<QuestStageAdvanced> {
    let mut deferred = DeferredFragmentEffects::new(world);
    deferred.speaker = speaker;
    let mut advances = {
        let (mut stages, mut objectives) =
            world.resource_2_mut::<QuestStageState, QuestObjectiveState>();
        apply_effects(
            effects,
            context,
            vmad,
            world,
            &mut stages,
            &mut objectives,
            &mut deferred,
        )
    };
    advances.extend(deferred.apply(world));
    advances
}

/// Consume quest-journal entries produced while one guard-free fragment ran.
/// The journal supplies canonical interleaving (including any host-side stage
/// changes); direct advances are retained only as a loss-recovery fallback.
pub(crate) fn poll_fragment_generated_advances(
    world: &World,
    direct: Vec<QuestStageAdvanced>,
) -> Vec<QuestStageAdvanced> {
    let read = world
        .resource_mut::<QuestStageState>()
        .poll_quest_events(FRAGMENT_QUEST_EVENT_SUBSCRIBER);
    if read.missed_events > 0 {
        log::error!(
            target: "scripting::quest_fragments",
            "fragment subscriber missed {} retained quest transition(s) during guard-free execution",
            read.missed_events
        );
    }
    let mut observed = read
        .events
        .into_iter()
        .map(|sequenced| sequenced.event)
        .collect::<Vec<_>>();
    let mut observed_counts = HashMap::new();
    for event in &observed {
        *observed_counts
            .entry((event.quest, event.previous_stage, event.new_stage))
            .or_insert(0usize) += 1;
    }
    for event in direct {
        let count = observed_counts
            .entry((event.quest, event.previous_stage, event.new_stage))
            .or_default();
        if *count > 0 {
            *count -= 1;
        } else {
            observed.push(event);
        }
    }
    observed
}

/// Read one entity's [`Transform`] as an owned copy, holding no guard on
/// return (#3250).
pub(crate) fn copied_transform(world: &World, entity: EntityId) -> Option<Transform> {
    // #3250 — `World::get` returns an owning read guard. Copy the component
    // before another Transform lookup so a queued writer can never strand a
    // recursive read behind the first guard.
    world.get::<Transform>(entity).map(|transform| *transform)
}

/// Apply one effect to the canonical stage/objective state (or, for the
/// object-targeting variants, to the live ECS world). Returns a
/// [`QuestStageAdvanced`] when the effect was a `SetStage` (so the caller
/// can cascade), or `None` otherwise / when a target can't resolve.
///
/// **Nested-lock safety depends on exclusive scheduling.** What this
/// function runs *inside* is one `resource_2_mut::<QuestStageState,
/// QuestObjectiveState>()` taken by [`apply_fragment_guard_free`] — a
/// single TypeId-sorted paired acquisition, scoped to one fragment's
/// effects and dropped before `deferred.apply(world)` runs. It is NOT held
/// across the cascade loop, and `QuestStageFragments` is not held at all:
/// [`quest_fragment_dispatch_system`] clones it before taking any quest
/// resource, precisely to avoid a read→write nested order.
///
/// Everything below is therefore nested under those two guards and nothing
/// else. Listed by where it is taken rather than as a hand count, because
/// the count is what rotted (#3949 — the previous version of this block
/// said "12 component-storage acquisitions" and named three resources that
/// are no longer all held together):
///
///   - **directly in the match arms** — `Globals` (write, `SetGlobalValue`),
///     `PlayerControlState` (write ×3, the `SetPlayerControls` family),
///     `EquipItemCatalog` (read), `SceneRegistry` (read),
///     `PapyrusPlayerEntity` (read), and the component storages
///     `Inventory`, `Transform` (read and write), `GlobalTransform`,
///     `ActorControlState`, `EvaluatePackageRequest`, `HorseTetherState`,
///     `MotionTypeChangeRequest` (×2), `SceneStartRequest`,
///     `SceneStopRequest`, `Locked` (write ×2, the `SetLocked` /
///     `SetLockLevel` pair — #3159), `AiCombatState` (write,
///     `Effect::StartCombat` — MQ101's dragon-attack/keep-escape combat
///     gate), `AmbientEngagement` (write, `Effect::StartCombat` taking
///     ownership of an ambient fight — #5046; taken after the
///     `AiCombatState` guard drops), `PlayerMoveRequest` (write,
///     `Effect::MovePlayerTo` — M47.2; taken alone, the engine drains it),
///     `Perks` (write, `Effect::ChangePerk` — M47.2), `ActorValueNames`
///     (read, released before `ActorValues`, `Effect::SetActorValueNamed` and
///     `Effect::DamageActorValue` — M47.2), `SceneAliasCandidate` (read, `Effect::SetBaseProtection`'s
///     actor base — M47.2), `EditorLocation` (read,
///     `Effect::MoveToEditorLocation` — M47.2; copied out and released
///     before the `Transform` write)
///   - **via [`resolve_actor`]** — `PapyrusPlayerEntity` (read)
///   - **via [`crate::condition::entity_global_form_id`]** — `FormIdPool` (read)
///   - **via [`update_actor_cinematic_state`]** — `ActorCinematicState`
///   - **via `crate::magic::{add_spell, remove_spell}`** (the
///     `AddSpell`/`RemoveSpell` arms, applied inline rather than deferred —
///     no cycle: every quest-resource system is exclusive) — `SpellList`
///     (write), then through `apply_constant_modifiers`:
///     `SpellCatalog` (read) and `ActorValues` (write), each released
///     before the next is taken
///   - **via `crate::magic::{add_race_spells, remove_race_spells}`** (the
///     #4415 racial pair, same inline shape) — `RaceSpells` (read, cloned
///     out and released) plus the `add_spell`/`remove_spell` chain above
///     once per racial spell
///
/// so ~19 distinct storage/resource types across ~24 sites. "Directly in
/// the match arms" is now one delegation hop away: #4340 moved each family
/// of arms into an `apply_<family>_effect` helper below (globals,
/// inventory, placement, scene, lock ledger, player control, vehicle
/// cinematics, AI/combat), plus the pre-existing [`resolve_actor`],
/// [`crate::condition::entity_global_form_id`], [`update_actor_cinematic_state`] and
/// [`apply_quest_scoped_effect`]. Nothing about the nesting changed — the
/// helpers run inside the same two guards, this list still inventories
/// every lock any of them takes, and the #3949 scan below reads their
/// bodies together with this function's. The `crate::magic` call-outs are
/// one hop *further* away (a different module, which the fragment-body
/// scan is structurally blind to — #5071), so the scan also reads the
/// five `magic.rs` functions those arms reach; the deferral option was
/// declined because the arms are already exclusive-lane-safe and deferring
/// would change ordering semantics P4 relies on. The
/// `FragmentExecutionQueue`
/// write is *not* in this list: it belongs to the latent-continuation path
/// in the dispatch system, outside these guards.
///
/// This is only safe because every system that touches the quest resources
/// is registered `add_exclusive` in `byroredux/src/boot/schedule/` (parallel
/// systems never run concurrently with an exclusive one), so no other
/// holder can ever form the other half of an ABBA cycle. Adding a new
/// nested component/resource lock here, or moving
/// [`quest_fragment_dispatch_system`] (or any sibling quest-resource
/// system) onto the parallel lane, needs the same analysis re-derived —
/// see SCR-D6-NEW3-03 / #2126, and record the change under the house-rule
/// doc #2270 asks for.
///
/// `SceneActorBindings` used to be part of this residual list too (every
/// `resolve_object` alias lookup read it live) but is now resolved through
/// the pre-lock snapshot in `deferred` instead (#2660) — see
/// [`DeferredFragmentEffects`]'s doc for why a snapshot is safe here.
/// Quest-definition reads use the same snapshot, and presentation /
/// alias-binding *mutations* are appended to it rather than acquired here.
/// Every production caller constructs the batch before taking quest
/// resource guards and applies it only after those guards have dropped
/// (#2269, #2539, #2660).
pub(crate) fn apply_effect(
    effect: &Effect,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    stages: &mut QuestStageState,
    objectives: &mut QuestObjectiveState,
    deferred: &mut DeferredFragmentEffects,
) -> Option<QuestStageAdvanced> {
    match effect {
        Effect::SetGlobalValue { global, value } => {
            // Read first: the value may read `Globals` itself (`G.Mod(x)`).
            let value = evaluate_value(value, context, vmad, world, stages, deferred)?;
            let form_id = resolve_property_form_id(vmad, global.property_name())?;
            if let Some(mut globals) = world.try_resource_mut::<crate::Globals>() {
                globals.set(form_id, value);
            } else {
                log::debug!(
                    "fragment SetValue skipped: Globals resource is unavailable for {form_id:08X}"
                );
            }
            None
        }
        Effect::AddItem { .. } | Effect::RemoveItem { .. } | Effect::EquipItem { .. } => {
            apply_inventory_effect(effect, context, vmad, world, stages, deferred)
        }
        Effect::MoveTo { .. }
        | Effect::MoveToEditorLocation { .. }
        | Effect::MovePlayerTo { .. }
        | Effect::Disable { .. }
        | Effect::Enable { .. } => {
            apply_placement_effect(effect, context, vmad, world, deferred)
        }
        Effect::StartScene { .. }
        | Effect::StopScene { .. }
        | Effect::Activate { .. }
        | Effect::SetOpen { .. } => {
            apply_scene_effect(effect, context, vmad, world, stages, deferred)
        }
        Effect::SetLocked { .. } | Effect::SetLockLevel { .. } => {
            apply_lock_effect(effect, context, vmad, world, deferred)
        }
        Effect::SetPlayerRestrained { .. }
        | Effect::SetPlayerControls { .. }
        | Effect::SetPlayerAiDriven { .. }
        | Effect::SetHudCartMode { .. }
        | Effect::SetSittingRotation { .. }
        | Effect::SetInChargen { .. }
        | Effect::ShowRaceMenu
        | Effect::RequestSave { .. }
        | Effect::RegisterPlayerAnimationEvent { .. } => {
            apply_player_control_effect(effect, context, vmad, world, deferred)
        }
        Effect::PlayIdle { .. }
        | Effect::SetVehicle { .. }
        | Effect::TetherToHorse { .. }
        | Effect::SetMotionType { .. }
        | Effect::ExitCart { .. } => {
            apply_vehicle_cinematic_effect(effect, context, vmad, world, deferred)
        }
        Effect::SendStoryEvent {
            keyword,
            location,
            references,
            values,
        } => {
            let keyword = resolve_property_form_id(vmad, keyword.property_name())?;
            log::debug!(
                "fragment SendStoryEvent: quest {:08X} raises SCPT keyword {keyword:08X}",
                context.0
            );
            // A location property, or a location alias's bound location.
            let location = match location {
                None => None,
                Some(location) => resolve_location(vmad, world, context, location),
            };
            let [first, second] = references
                .clone()
                .map(|actor| actor.and_then(|actor| guard_actor(&actor, context, vmad, world, deferred)));
            // `R1` holds an entity: the first reference, else the player
            // (flagged absent).
            let carrier = first.or_else(|| {
                world
                    .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
                    .map(|player| player.0)
            })?;
            deferred.story_events.push(
                crate::story_manager::StoryEvent {
                    mnemonic: *b"SCPT",
                    reference_1: carrier,
                    reference_2: second,
                    location_1: location,
                    location_2: None,
                    scripted: Some(crate::story_manager::ScriptedStoryEvent {
                        keyword,
                        values: *values,
                        has_reference_1: first.is_some(),
                    }),
                },
            );
            None
        }
        Effect::AddToMap { marker, fast_travel } => {
            let Some(reference) = object_form_id(vmad, world, context, marker, deferred) else {
                log::debug!("fragment AddToMap skipped: the marker does not resolve");
                return None;
            };
            deferred.map_markers.push((reference, *fast_travel));
            None
        }
        Effect::ShowMessage { message, args } => {
            let form_id = resolve_property_form_id(vmad, message.property_name())?;
            deferred.messages.push(PendingMessage::Show(form_id, args.clone()));
            None
        }
        Effect::ShowHelpMessage { message } => {
            let form_id = resolve_property_form_id(vmad, message.property_name())?;
            deferred.messages.push(PendingMessage::Show(form_id, Vec::new()));
            None
        }
        Effect::Notification { text } => {
            deferred.messages.push(PendingMessage::Notify(text.clone()));
            None
        }
        Effect::DeleteWhenAble { object } => {
            let form_id = object_form_id(vmad, world, context, object, deferred)?;
            deferred.deletions.push(form_id);
            None
        }
        Effect::ShowBarterMenu { vendor } => {
            let vendor = guard_actor(vendor, context, vmad, world, deferred)?;
            deferred.barter = Some(vendor);
            None
        }
        Effect::ChangeKeyword {
            object,
            keyword,
            change,
        } => {
            let object = guard_actor(object, context, vmad, world, deferred)?;
            let keyword = resolve_property_form_id(vmad, keyword.property_name())?;
            deferred.keyword_changes.push((object, keyword, *change));
            None
        }
        Effect::SetReferenceVariable {
            object,
            variable,
            value,
        } => {
            let Some(entity) =
                resolve_object(vmad, world, context, object, &deferred.scene_actor_bindings)
            else {
                log::debug!("fragment reference variable write skipped: the reference is not loaded");
                return None;
            };
            deferred
                .reference_variables
                .push((entity, variable.clone(), *value));
            None
        }
        Effect::SetLocal { local, value } => {
            let value = evaluate_value(value, context, vmad, world, stages, deferred)?;
            deferred.locals.insert(local.clone(), value);
            None
        }
        Effect::RewardPlayerXp { amount, direct } => {
            let amount = resolve_item_count(amount, context, vmad, world, stages, deferred)?;
            deferred.player_xp.push((amount, *direct));
            None
        }
        Effect::SetPlayerTeammate {
            actor,
            teammate,
            can_do_favor,
        } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let form = crate::condition::entity_global_form_id(world, actor)?;
            deferred.follower_changes.push((
                form,
                FollowerChange::Teammate {
                    teammate: *teammate,
                    can_do_favor: *can_do_favor,
                },
            ));
            None
        }
        Effect::SetDoingFavor { actor, doing } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let form = crate::condition::entity_global_form_id(world, actor)?;
            deferred.follower_changes.push((form, FollowerChange::DoingFavor(*doing)));
            None
        }
        Effect::SetLinkedRef {
            object,
            target,
            keyword,
        } => {
            let source = object_form_id(vmad, world, context, object, deferred)?;
            let target = match target {
                Some(target) => object_form_id(vmad, world, context, target, deferred)?,
                None => 0,
            };
            let keyword = match keyword {
                Some(keyword) => resolve_property_form_id(vmad, keyword.property_name())?,
                None => 0,
            };
            deferred.linked_ref_changes.push((source, keyword, target));
            None
        }
        Effect::SetLocRefType { object, ref_type } => {
            let reference = object_form_id(vmad, world, context, object, deferred)?;
            let ref_type = match ref_type {
                Some(ref_type) => resolve_property_form_id(vmad, ref_type.property_name())?,
                None => 0,
            };
            deferred.loc_ref_type_changes.push((reference, ref_type));
            None
        }
        Effect::AdvanceSkill { skill, magnitude } => {
            deferred.skill_advances.push((skill.clone(), *magnitude));
            None
        }
        Effect::AddAchievement { id } => {
            deferred.achievements.push(*id);
            None
        }
        Effect::IncrementStat { stat, amount } => {
            deferred.misc_stats.push((stat.clone(), *amount));
            None
        }
        Effect::PlayerCaps { delta } => {
            // `Game.GetCaps()` is `Game.GetForm(15)` in FO4's `Game.psc`.
            const CAPS: u32 = 0x0000_000F;
            let player = world
                .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
                .map(|player| player.0)?;
            match delta.cmp(&0) {
                std::cmp::Ordering::Greater => add_items(world, player, CAPS, *delta as u32),
                std::cmp::Ordering::Less => {
                    remove_items(world, player, CAPS, delta.unsigned_abs(), None);
                }
                std::cmp::Ordering::Equal => {}
            }
            None
        }
        Effect::SetRelationshipRank { actor, other, rank } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let other = guard_actor(other, context, vmad, world, deferred)?;
            let (Some(actor), Some(other)) = (
                crate::relationships::actor_base(world, actor),
                crate::relationships::actor_base(world, other),
            ) else {
                log::debug!("fragment SetRelationshipRank skipped: an actor has no base");
                return None;
            };
            deferred
                .relationship_changes
                .push((crate::relationships::pair(actor, other), *rank));
            None
        }
        Effect::Kill { actor, killer, silent } => {
            let victim = guard_actor(actor, context, vmad, world, deferred)?;
            let killer = killer
                .as_ref()
                .and_then(|killer| guard_actor(killer, context, vmad, world, deferred));
            deferred.kills.push((victim, killer, *silent));
            None
        }
        Effect::SetBaseProtection {
            base,
            protected,
            value,
        } => {
            let base = match base {
                crate::translate::effects::ActorBaseRef::OfActor(actor) => {
                    let actor = guard_actor(actor, context, vmad, world, deferred)?;
                    world
                        .get::<crate::scene::SceneAliasCandidate>(actor)
                        .map(|candidate| candidate.base_form_id)
                }
                crate::translate::effects::ActorBaseRef::Form(form) => {
                    resolve_property_form_id(vmad, form.property_name())
                }
            };
            let Some(base) = base else {
                log::debug!("fragment SetEssential / SetProtected skipped: no actor base");
                return None;
            };
            deferred.base_protection.push((base, *protected, *value));
            None
        }
        Effect::SetLocationCleared { location, cleared } => {
            let Some(location) = resolve_location(vmad, world, context, location) else {
                log::debug!("fragment SetCleared skipped: the location does not resolve");
                return None;
            };
            deferred.location_clears.push((location, *cleared));
            None
        }
        Effect::BlockActivation { object, blocked } => {
            let Some(reference) = object_form_id(vmad, world, context, object, deferred) else {
                log::debug!("fragment BlockActivation skipped: the reference does not resolve");
                return None;
            };
            deferred.activation_blocks.push((reference, *blocked));
            None
        }
        Effect::ForceLocationTo { alias, location } => {
            let name = alias.property_name();
            let Some((quest, alias_id)) = alias_property(vmad, context, name) else {
                log::debug!("fragment ForceLocationTo skipped: '{name}' is not an alias property");
                return None;
            };
            let location = resolve_property_form_id(vmad, location.property_name())?;
            deferred
                .alias_overrides
                .push((quest, alias_id, crate::AliasOverride::ForcedLocation(location)));
            deferred.scene_actor_bindings_dirty = true;
            None
        }
        Effect::ForceAlias { alias, target } => {
            let name = alias.property_name();
            let Some((quest, alias_id)) = alias_property(vmad, context, name) else {
                log::debug!("fragment ForceRefTo/Clear skipped: '{name}' is not an alias property");
                return None;
            };
            let value = match target {
                None => crate::AliasOverride::Cleared,
                Some(target) => {
                    crate::AliasOverride::Forced(actor_reference_form_id(target, context, vmad, world, deferred)?)
                }
            };
            deferred.alias_overrides.push((quest, alias_id, value));
            deferred.scene_actor_bindings_dirty = true;
            None
        }
        Effect::SetLocationKeywordData { location, keyword, value } => {
            let location = resolve_property_form_id(vmad, location.property_name())?;
            let keyword = resolve_property_form_id(vmad, keyword.property_name())?;
            let value = evaluate_value(value, context, vmad, world, stages, deferred)?;
            deferred.location_keyword_changes.push((location, keyword, value));
            None
        }
        Effect::SetActorValue {
            actor,
            actor_value,
            value,
        } => {
            let actor = resolve_actor(
                vmad,
                world,
                context,
                actor,
                &deferred.scene_actor_bindings,
                deferred.speaker,
            )?;
            let avif = resolve_property_form_id(vmad, actor_value.property_name())?;
            let value = evaluate_value(value, context, vmad, world, stages, deferred)?;
            set_actor_value_base(world, actor, avif, value);
            None
        }
        Effect::SetActorValueNamed { actor, name, value } => {
            let actor = resolve_actor(
                vmad,
                world,
                context,
                actor,
                &deferred.scene_actor_bindings,
                deferred.speaker,
            )?;
            let Some(avif) = world
                .try_resource::<crate::ActorValueNames>()
                .and_then(|names| names.resolve(name))
            else {
                log::debug!("fragment SetAV skipped: no actor value named '{name}'");
                return None;
            };
            let value = evaluate_value(value, context, vmad, world, stages, deferred)?;
            set_actor_value_base(world, actor, avif, value);
            None
        }
        Effect::DamageActorValue {
            actor,
            actor_value,
            amount,
            restore,
        } => {
            let actor = resolve_actor(
                vmad,
                world,
                context,
                actor,
                &deferred.scene_actor_bindings,
                deferred.speaker,
            )?;
            let avif = match actor_value {
                crate::translate::effects::ActorValueArg::Form(form) => {
                    resolve_property_form_id(vmad, form.property_name())?
                }
                crate::translate::effects::ActorValueArg::Named(name) => world
                    .try_resource::<crate::ActorValueNames>()
                    .and_then(|names| names.resolve(name))?,
            };
            let amount = evaluate_value(amount, context, vmad, world, stages, deferred)?;
            let mut values = world.query_mut::<byroredux_core::ecs::components::ActorValues>()?;
            if values.get_mut(actor).is_none() {
                values.insert(actor, byroredux_core::ecs::components::ActorValues::new());
            }
            let values = values.get_mut(actor).expect("present or inserted");
            if *restore {
                values.restore(avif, amount);
            } else {
                values.apply_damage(avif, amount);
            }
            None
        }
        Effect::ChangePerk { actor, perk, add } => {
            let actor = resolve_actor(
                vmad,
                world,
                context,
                actor,
                &deferred.scene_actor_bindings,
                deferred.speaker,
            )?;
            let perk = resolve_property_form_id(vmad, perk.property_name())?;
            // `Perks` is direct component storage, written in place like
            // `ActorValues` above.
            let mut perks = world.query_mut::<byroredux_core::character::Perks>()?;
            if perks.get_mut(actor).is_none() {
                perks.insert(actor, byroredux_core::character::Perks::default());
            }
            let owned = perks.get_mut(actor).expect("present or inserted");
            if !*add {
                owned.remove(perk);
            } else if owned.rank(perk) == 0 {
                owned.set_rank(perk, 1);
            }
            None
        }
        Effect::ChangeFaction {
            actor,
            faction,
            change,
        } => {
            let actor = resolve_actor(
                vmad,
                world,
                context,
                actor,
                &deferred.scene_actor_bindings,
                deferred.speaker,
            )?;
            let faction = match change {
                crate::FactionChange::RemoveAll => 0,
                _ => resolve_property_form_id(vmad, faction.property_name())?,
            };
            deferred.faction_changes.push((actor, faction, *change));
            None
        }
        Effect::SetQuestVariable {
            quest,
            variable,
            value,
        } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            deferred.quest_variables.push((
                quest,
                crate::QuestScriptVariables::property_key(variable),
                *value,
            ));
            None
        }
        Effect::CopyQuestVariable { quest, from, to } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            let from = crate::QuestScriptVariables::property_key(from);
            let to = crate::QuestScriptVariables::property_key(to);
            // The same pending-then-written chain `ModQuestVariable` reads.
            let value = deferred
                .quest_variables
                .iter()
                .rev()
                .find(|(q, k, _)| *q == quest && *k == from)
                .map(|(_, _, value)| *value)
                .or_else(|| crate::quest_variable(world, quest, from))?;
            deferred.quest_variables.push((quest, to, value));
            None
        }
        Effect::ModQuestVariable {
            quest,
            variable,
            delta,
        } => {
            // Read through the same pending-then-written-then-authored chain
            // a guard uses, so an earlier write in this fragment counts.
            let quest = resolve_quest_logged(quest, context, vmad)?;
            let key = crate::QuestScriptVariables::property_key(variable);
            let current = deferred
                .quest_variables
                .iter()
                .rev()
                .find(|(q, k, _)| *q == quest && *k == key)
                .map(|(_, _, value)| *value)
                .or_else(|| crate::quest_variable(world, quest, key))
                .unwrap_or(0.0);
            deferred.quest_variables.push((quest, key, current + delta));
            None
        }
        Effect::SetEnemy { .. }
        | Effect::SetAlly { .. }
        | Effect::StartCombat { .. }
        | Effect::StopCombat { .. }
        | Effect::StopCombatAlarm { .. }
        | Effect::SetCrimeFaction { .. }
        | Effect::SetUnconscious { .. }
        | Effect::SetGhost { .. }
        | Effect::EvaluatePackage { .. } => {
            apply_ai_combat_effect(effect, context, vmad, world, deferred)
        }
        Effect::AddSpell { actor, spell } | Effect::RemoveSpell { actor, spell } => {
            let actor = resolve_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            let spell = resolve_property_form_id(vmad, spell.property_name())?;
            let adding = matches!(effect, Effect::AddSpell { .. });
            let changed = if adding {
                crate::magic::add_spell(world, actor, spell)
            } else {
                crate::magic::remove_spell(world, actor, spell)
            };
            // #4822 — never drop the result silently. `magic` itself warns
            // when the actor has no `SpellList` (a lost effect); what is left
            // here is Papyrus' own idempotent no-op.
            if !changed {
                let verb = if adding { "AddSpell" } else { "RemoveSpell" };
                log::debug!(
                    "quest {context:?}: {verb}({spell:08X}) on entity {actor} changed nothing"
                );
            }
            None
        }
        // #4415 — the script-defined racial pair. The racial set was
        // level-resolved and stamped as `RaceSpells` at spawn, so the apply
        // needs no index; each member goes through the same add/remove the
        // scripted `AddSpell` uses (constant modifiers included).
        Effect::AddRaceSpells { actor } | Effect::RemoveRaceSpells { actor } => {
            let actor = resolve_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            let adding = matches!(effect, Effect::AddRaceSpells { .. });
            let changed = if adding {
                crate::magic::add_race_spells(world, actor)
            } else {
                crate::magic::remove_race_spells(world, actor)
            };
            if !changed {
                let verb = if adding {
                    "AddRaceSpells"
                } else {
                    "RemoveRaceSpells"
                };
                log::debug!("quest {context:?}: {verb} on entity {actor} changed nothing");
            }
            None
        }
        Effect::Wait { .. } | Effect::WaitForActors3DLoaded { .. } | Effect::WaitWhile { .. } => {
            None
        }
        Effect::Conditional { .. } | Effect::QuestScope { .. } | Effect::Interpret { .. } => {
            unreachable!("conditional and quest-scoped effects are expanded by apply_effects")
        }
        _ => apply_quest_scoped_effect(effect, context, vmad, stages, objectives, deferred),
    }
}

/// `RemoveItem`'s runtime (M47.2): take up to `count` of `item_form_id` from
/// `source`'s stacks, first-authored first, and move them to `destination` or
/// drop them.
///
/// Rows are zeroed in place, never removed, so the inventory indices that
/// `EquipmentSlots` holds stay valid (the `consume_item` / `transfer_loot`
/// discipline); an emptied row's equipment is released. A fully drained
/// stack that carries a per-instance identity travels whole to the
/// destination, or returns its instance to the [`ItemInstancePool`] when the
/// items vanish. Script `OnItemRemoved` / `OnItemAdded` see the move. Every
/// guard is scoped: the inventory write, then equipment, then the pool, then
/// the event batches — none is held across another.
pub fn remove_items(
    world: &World,
    source: EntityId,
    item_form_id: u32,
    count: u32,
    destination: Option<EntityId>,
) {
    use byroredux_core::ecs::components::InventoryIndex;
    if count == 0 || destination == Some(source) {
        return;
    }
    let mut removed = 0u32;
    let mut emptied: Vec<InventoryIndex> = Vec::new();
    let mut freed_instances = Vec::new();
    {
        let Some(mut inventories) = world.query_mut::<Inventory>() else {
            return;
        };
        if let Some(destination) = destination {
            // Validate the destination before taking anything: a missing
            // destination loses no items.
            if inventories.get_mut(destination).is_none() {
                inventories.insert(destination, Inventory::new());
            }
        }
        let Some(inventory) = inventories.get_mut(source) else {
            log::debug!("fragment RemoveItem skipped: container has no Inventory");
            return;
        };
        let mut moved: Vec<ItemStack> = Vec::new();
        for (index, stack) in inventory.items.iter_mut().enumerate() {
            if removed == count {
                break;
            }
            if stack.base_form_id != item_form_id || stack.count == 0 {
                continue;
            }
            let take = stack.count.min(count - removed);
            removed += take;
            if take == stack.count {
                // The whole row goes: its instance identity moves with it
                // (or is freed), and its equipment is released below.
                let whole = std::mem::replace(stack, ItemStack::new(item_form_id, 0));
                if let Ok(index) = u32::try_from(index) {
                    emptied.push(InventoryIndex(index));
                }
                if destination.is_some() {
                    moved.push(whole);
                } else if let Some(instance) = whole.instance {
                    freed_instances.push(instance);
                }
            } else {
                stack.count -= take;
                if destination.is_some() {
                    moved.push(ItemStack::new(item_form_id, take));
                }
            }
        }
        if let Some(destination) = destination {
            inventories
                .get_mut(destination)
                .expect("validated under the same guard")
                .items
                .extend(moved);
        }
    }
    if removed == 0 {
        return;
    }
    if !emptied.is_empty() {
        if let Some(mut equipment) = world.query_mut::<EquipmentSlots>() {
            if let Some(slots) = equipment.get_mut(source) {
                for index in &emptied {
                    slots.release(*index);
                }
            }
        }
    }
    if !freed_instances.is_empty() {
        if let Some(mut pool) =
            world.try_resource_mut::<byroredux_core::ecs::resources::ItemInstancePool>()
        {
            for instance in freed_instances {
                pool.release(instance);
            }
        }
    }
    let transfer = |added| crate::ItemTransfer {
        item_form_id,
        count: removed,
        added,
        stolen: false,
    };
    crate::emit_item_transfers(world, source, [transfer(false)]);
    if let Some(destination) = destination {
        crate::emit_item_transfers(world, destination, [transfer(true)]);
    }
}

/// `AddItem` / `EquipItem` — the `Inventory` + `EquipmentSlots` arms.
///
/// #4340 — split out of [`apply_effect`], whose arms are now one-line
/// delegates. The bodies moved verbatim; every lock they take stays
/// named in `apply_effect`'s nested-lock residual list, which the #3949
/// scan now reads across these helpers too.
fn apply_inventory_effect(
    effect: &Effect,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    stages: &QuestStageState,
    deferred: &mut DeferredFragmentEffects,
) -> Option<QuestStageAdvanced> {
    match effect {
        Effect::AddItem {
            container,
            item,
            count,
        } => {
            let container_entity = resolve_actor(
                vmad,
                world,
                context,
                container,
                &deferred.scene_actor_bindings,
                deferred.speaker,
            )?;
            let item_form_id = resolve_property_form_id(vmad, item.property_name())?;
            let count = &resolve_item_count(count, context, vmad, world, stages, deferred)?;
            add_items(world, container_entity, item_form_id, *count);
            None
        }
        Effect::RemoveItem {
            container,
            item,
            count,
            destination,
        } => {
            let bindings = &deferred.scene_actor_bindings;
            let source = resolve_actor(vmad, world, context, container, bindings, deferred.speaker)?;
            let item_form_id = resolve_property_form_id(vmad, item.property_name())?;
            let count = &resolve_item_count(count, context, vmad, world, stages, deferred)?;
            let destination = match destination {
                Some(via) => Some(resolve_actor(vmad, world, context, via, bindings, deferred.speaker)?),
                None => None,
            };
            remove_items(world, source, item_form_id, *count, destination);
            None
        }
        Effect::EquipItem {
            actor,
            item,
            silent: _,
        } => {
            let actor = resolve_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            let item_form_id = resolve_property_form_id(vmad, item.property_name())?;
            let Some(slot_mask) = world
                .try_resource::<crate::EquipItemCatalog>()
                .and_then(|catalog| catalog.slot_mask(item_form_id))
            else {
                log::debug!(
                    "fragment EquipItem skipped: item {item_form_id:08X} has no armor biped-slot metadata"
                );
                return None;
            };
            let Some((mut inventories, mut equipment)) =
                world.query_2_mut_mut::<Inventory, EquipmentSlots>()
            else {
                log::debug!(
                    "fragment EquipItem skipped: inventory/equipment storage is unavailable"
                );
                return None;
            };
            let Some(inventory) = inventories.get_mut(actor) else {
                log::debug!("fragment EquipItem skipped: actor has no Inventory");
                return None;
            };
            let Some(index) = inventory
                .items
                .iter()
                .position(|stack| stack.base_form_id == item_form_id && stack.count > 0)
                .and_then(|index| u32::try_from(index).ok())
                .map(InventoryIndex)
            else {
                log::debug!(
                    "fragment EquipItem skipped: actor does not carry item {item_form_id:08X}"
                );
                return None;
            };
            if equipment.get_mut(actor).is_none() {
                equipment.insert(actor, EquipmentSlots::new());
            }
            let slots = equipment
                .get_mut(actor)
                .expect("just present or inserted above");
            if slots.is_equipped(index) {
                return None;
            }
            let displaced = slots.equip(slot_mask, index);
            let mut changes = displaced
                .into_iter()
                .filter(|displaced| !slots.is_equipped(*displaced))
                .filter_map(|displaced| {
                    inventory
                        .get(displaced)
                        .map(|stack| crate::EquipmentChange {
                            item_form_id: stack.base_form_id,
                            equipped: false,
                        })
                })
                .collect::<Vec<_>>();
            changes.push(crate::EquipmentChange {
                item_form_id,
                equipped: true,
            });
            drop(inventories);
            drop(equipment);
            crate::emit_equipment_changes(world, actor, changes);
            None
        }
        _ => unreachable!("apply_inventory_effect received a non-inventory effect"),
    }
}

/// Add `count` of `item_form_id` to `container`'s inventory (creating one: in
/// Bethesda's runtime every object can receive items), emitting `OnItemAdded`.
pub fn add_items(world: &World, container: EntityId, item_form_id: u32, count: u32) {
    let Some(mut inventories) = world.query_mut::<Inventory>() else {
        log::debug!("fragment AddItem skipped: Inventory component never registered");
        return;
    };
    if inventories.get_mut(container).is_none() {
        // An interior-mutable insert onto an already-registered storage, not
        // a structural `&mut World` mutation.
        inventories.insert(container, Inventory::new());
    }
    inventories
        .get_mut(container)
        .expect("just present or inserted above")
        .push(ItemStack::new(item_form_id, count));
    drop(inventories);
    // `OnItemAdded` sees a scripted add like any other (M47.2).
    crate::emit_item_transfers(
        world,
        container,
        [crate::ItemTransfer {
            item_form_id,
            count,
            added: true,
            stolen: false,
        }],
    );
}

/// `MoveTo` / `Disable` / `Enable` — placement and reference-enable state.
///
/// #4340 — split out of [`apply_effect`], whose arms are now one-line
/// delegates. The bodies moved verbatim; every lock they take stays
/// named in `apply_effect`'s nested-lock residual list, which the #3949
/// scan now reads across these helpers too.
fn apply_placement_effect(
    effect: &Effect,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    deferred: &mut DeferredFragmentEffects,
) -> Option<QuestStageAdvanced> {
    match effect {
        Effect::MoveTo {
            moved,
            destination,
            offset,
            match_rotation,
        } => {
            let moved_entity =
                resolve_object(vmad, world, context, moved, &deferred.scene_actor_bindings)?;
            let destination_entity = resolve_object(
                vmad,
                world,
                context,
                destination,
                &deferred.scene_actor_bindings,
            )?;
            let Some((translation, rotation)) = world
                .get::<GlobalTransform>(destination_entity)
                .map(|gt| (gt.translation, gt.rotation))
            else {
                log::debug!("fragment MoveTo skipped: destination entity has no GlobalTransform");
                return None;
            };
            // The offset is in Gamebryo's Z-up world axes.
            let translation = translation + byroredux_core::math::Vec3::from(byroredux_core::math::coord::zup_to_yup_pos(*offset));
            let Some(mut transforms) = world.query_mut::<Transform>() else {
                log::debug!("fragment MoveTo skipped: Transform component never registered");
                return None;
            };
            // Unlike AddItem, a "moved" entity with no Transform isn't a
            // container the effect can adopt — it isn't a placed spatial
            // entity at all, so this declines rather than fabricating one.
            let Some(transform) = transforms.get_mut(moved_entity) else {
                log::debug!("fragment MoveTo skipped: moved entity has no Transform");
                return None;
            };
            log::debug!(
                "fragment MoveTo (quest {:08X}): {moved_entity:?} -> {destination_entity:?} at {translation:?}",
                context.0
            );
            transform.translation = translation;
            if *match_rotation {
                transform.rotation = rotation;
            }
            None
        }
        Effect::MoveToEditorLocation { moved } => {
            let moved_entity =
                resolve_object(vmad, world, context, moved, &deferred.scene_actor_bindings)?;
            let Some(placed) = world.get::<crate::EditorLocation>(moved_entity).map(|placed| *placed) else {
                log::debug!("fragment MoveToMyEditorLocation skipped: the reference has no editor location");
                return None;
            };
            let mut transforms = world.query_mut::<Transform>()?;
            let transform = transforms.get_mut(moved_entity)?;
            log::debug!(
                "fragment MoveToMyEditorLocation (quest {:08X}): {moved_entity:?} -> {:?}",
                context.0,
                placed.translation
            );
            transform.translation = placed.translation;
            transform.rotation = placed.rotation;
            None
        }
        Effect::MovePlayerTo { destination } => {
            let Some(form_id) = object_form_id(vmad, world, context, destination, deferred)
            else {
                log::debug!("fragment MoveTo (player) skipped: destination does not resolve");
                return None;
            };
            if let Some(mut request) = world.try_resource_mut::<super::PlayerMoveRequest>() {
                request.0 = Some(form_id);
            }
            None
        }
        Effect::Disable {
            object,
            fade_out: _,
        }
        | Effect::Enable { object, fade_in: _ } => {
            // #3278 (SCR-D5-2026-08-24-01) — `prim_disable` classifies its
            // receiver through the same `receiver_object` as
            // `AddItem`/`MoveTo`/`EquipItem`, so it can bind to a
            // quest-alias-filled `ObjectReference Property`. Dispatch used
            // only the strict `resolve_property_form_id`, which sees direct
            // VMAD FormID properties and nothing else — so
            // `<AliasBoundMarker>.Disable()` silently declined in exactly the
            // cases where the same alias-bound receiver resolves fine for
            // every sibling effect.
            //
            // Direct-FormID first, alias-aware second: that keeps the cheap,
            // world-free path unchanged for the case that already worked and
            // makes the alias arm strictly additive. `ReferenceEnableState`
            // is FormID-keyed (so the state outlives the reference's cell),
            // hence the trip back through `entity_global_form_id` —
            // [`object_form_id`], which also reads an unloaded linked ref.
            let form_id = object_form_id(vmad, world, context, object, deferred)?;
            // #3489 — the symmetric counterpart to Disable, sharing this arm
            // (same receiver resolution, same sink) rather than duplicating
            // it: `Effect::Enable` is the only other variant reaching here,
            // so this is exhaustive over the pattern above, not a guess.
            let enabled = matches!(effect, Effect::Enable { .. });
            deferred.reference_enable_changes.push((form_id, enabled));
            None
        }
        _ => unreachable!("apply_placement_effect received a non-placement effect"),
    }
}

/// `StartScene` / `StopScene` / `Activate` / `SetOpen` — the scene-registry
/// arms. `StartScene` doubles as Papyrus' quest `Start`/`Stop` (the
/// decompiled AST does not retain a property's declared type), which is why
/// this family is the one that needs `stages` and can return an advance.
///
/// #4340 — split out of [`apply_effect`], whose arms are now one-line
/// delegates. The bodies moved verbatim; every lock they take stays
/// named in `apply_effect`'s nested-lock residual list, which the #3949
/// scan now reads across these helpers too.
fn apply_scene_effect(
    effect: &Effect,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    stages: &mut QuestStageState,
    deferred: &mut DeferredFragmentEffects,
) -> Option<QuestStageAdvanced> {
    match effect {
        Effect::StartScene { scene } | Effect::StopScene { scene } => {
            let scene_form_id = resolve_property_form_id(vmad, scene.property_name())?;
            let scene_entity = world
                .try_resource::<crate::scene::SceneRegistry>()
                .and_then(|registry| registry.scene_entity(scene_form_id));
            if let Some(scene_entity) = scene_entity {
                if matches!(effect, Effect::StartScene { .. }) {
                    if let Some(mut requests) = world.query_mut::<crate::scene::SceneStartRequest>()
                    {
                        requests.insert(scene_entity, crate::scene::SceneStartRequest);
                    }
                } else if let Some(mut requests) =
                    world.query_mut::<crate::scene::SceneStopRequest>()
                {
                    requests.insert(scene_entity, crate::scene::SceneStopRequest);
                }
                return None;
            }

            // `Start`/`Stop` are shared by Scene and Quest in Papyrus, while
            // the decompiled AST does not retain a property's declared type.
            // Resolve the FormID against installed definitions at dispatch.
            let quest = QuestFormId(scene_form_id);
            let (start_up_stage, shut_down_stage) = {
                let definitions = deferred.quest_definitions.as_ref()?;
                if !definitions.contains(quest) {
                    log::debug!(
                        "fragment Start/Stop skipped: '{}' resolved to unknown form {scene_form_id:08X}",
                        scene.property_name()
                    );
                    return None;
                }
                (
                    definitions.start_up_stage(quest),
                    definitions.shut_down_stage(quest),
                )
            };
            if matches!(effect, Effect::StartScene { .. }) {
                let event = stages.start_quest(quest, start_up_stage);
                if event.is_some() {
                    deferred.scene_actor_bindings_dirty = true;
                }
                event
            } else {
                let event = stages
                    .is_running(quest)
                    .then(|| {
                        shut_down_stage.map(|stage| {
                            let previous_stage = stages.set_stage(quest, stage);
                            QuestStageAdvanced {
                                quest,
                                previous_stage,
                                new_stage: stage,
                                stage_set: true,
                            }
                        })
                    })
                    .flatten();
                if stages.stop(quest) {
                    deferred.scene_actor_bindings_dirty = true;
                }
                event
            }
        }
        Effect::Activate { target, activator } => {
            let target_entity =
                resolve_object(vmad, world, context, target, &deferred.scene_actor_bindings)?;
            let activator = match activator {
                Some(activator) => resolve_object(
                    vmad,
                    world,
                    context,
                    activator,
                    &deferred.scene_actor_bindings,
                )?,
                None => world
                    .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
                    .map(|player| player.0)
                    .unwrap_or(target_entity),
            };
            // Queue rather than insert: `quest_fragment_dispatch_system` is
            // the LAST producer of `ActivateEvent` in Stage::Update, but
            // most consumers (container loot, rumble, quest_advance,
            // two_state_activator, mg07) are scheduled earlier and
            // `event_cleanup_system` drains the marker at Stage::Late the
            // same frame — so a directly-inserted marker reached none of
            // them, ever (#2654). `fragment_activation_flush_system` turns
            // this into a real `ActivateEvent` at the head of the next
            // frame, ahead of every consumer.
            deferred.activations.push((target_entity, activator));
            None
        }
        Effect::SetOpen { target, open } => {
            let target_entity =
                resolve_object(vmad, world, context, target, &deferred.scene_actor_bindings)?;
            if !crate::vm_state::set_two_state_open(world, target_entity, *open) {
                log::debug!(
                    "fragment SetOpen skipped: '{}' is not a recognized two-state activator",
                    target.property_name()
                );
            }
            None
        }
        _ => unreachable!("apply_scene_effect received a non-scene effect"),
    }
}

/// `SetLocked` / `SetLockLevel` — the `Locked` component and the persistent
/// lock ledger (#3159 / #4136 / #4329 / #4330).
///
/// #4340 — split out of [`apply_effect`], whose arms are now one-line
/// delegates. The bodies moved verbatim; every lock they take stays
/// named in `apply_effect`'s nested-lock residual list, which the #3949
/// scan now reads across these helpers too.
fn apply_lock_effect(
    effect: &Effect,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    deferred: &mut DeferredFragmentEffects,
) -> Option<QuestStageAdvanced> {
    match effect {
        Effect::SetLocked { target, locked } => {
            // #3159 — the removal half that did not exist. `Locked` had one
            // insert (the cell loader's XLOC stamp) and one read (the
            // interaction gate) and nothing that cleared it, so an authored
            // lock was a one-way door for the session and any fragment
            // containing `Lock(false)` declined wholesale — discarding its
            // sibling stage/objective effects too.
            let Some(target_entity) =
                resolve_object(vmad, world, context, target, &deferred.scene_actor_bindings)
            else {
                // #4330 — no loaded entity. The ledger exists to outlive the
                // entity (the `ReferenceEnableState` posture, #3278), so an
                // *unlock* of a direct VMAD reference is still recorded and
                // takes effect when its cell next loads. A re-lock is not:
                // with no live component there is no authored `XLOC` level or
                // key to record, and inventing one would overwrite both.
                if !*locked {
                    if let Some(form_id) = resolve_property_form_id(vmad, target.property_name()) {
                        deferred
                            .reference_lock_changes
                            .push((form_id, DeferredLockChange::Unlocked));
                    }
                }
                return None;
            };
            // Through `query_mut`, not `World::insert`/`remove`: this system
            // holds `&World`, so structural mutation is unavailable, but
            // inserting into and removing from an *existing* storage is not
            // structural. `byroredux/src/boot/world.rs` pre-registers `Locked` so the storage is
            // there even in a session whose cells authored no XLOC at all —
            // otherwise the very first scripted lock would silently no-op.
            let Some(mut locks) = world.query_mut::<Locked>() else {
                log::debug!("fragment Lock skipped: Locked storage never registered");
                return None;
            };
            if *locked {
                // Re-locking something the cell loader never stamped: there is
                // no authored `XLOC` to recover a level or key from, so the
                // lock is recorded at its least-restrictive shape rather than
                // inventing a difficulty. A re-lock of a previously-locked
                // object keeps whatever it already carried.
                if locks.get(target_entity).is_none() {
                    locks.insert(
                        target_entity,
                        Locked {
                            lock_level: 0,
                            key_form_id: None,
                        },
                    );
                }
            } else {
                locks.remove(target_entity);
            }
            // #4136 — mirror the resulting component state into the
            // persistent ledger. Read back rather than re-deriving from
            // `*locked`: the re-lock branch above deliberately keeps
            // whatever the entity already carried, so the component is the
            // only place that knows the final level/key. Recording the
            // *outcome* is what keeps the two from drifting.
            let recorded = match locks.get(target_entity) {
                Some(l) => DeferredLockChange::Locked {
                    lock_level: l.lock_level,
                    key_form_id: l.key_form_id,
                },
                None => DeferredLockChange::Unlocked,
            };
            drop(locks);
            if let Some(form_id) = object_form_id(vmad, world, context, target, deferred) {
                deferred.reference_lock_changes.push((form_id, recorded));
            }
            None
        }
        Effect::SetLockLevel { target, level } => {
            // Difficulty only — never locks or unlocks (see the effect's
            // doc). An unlocked object has no `Locked` component to carry a
            // level, so this is a no-op there rather than an implicit lock.
            let target_entity =
                resolve_object(vmad, world, context, target, &deferred.scene_actor_bindings)?;
            let mut recorded = None;
            if let Some(mut locks) = world.query_mut::<Locked>() {
                if let Some(state) = locks.get_mut(target_entity) {
                    state.lock_level = *level;
                    // #4329 — record the whole outcome, key included, the
                    // same read-back `SetLocked` does. Queuing only the level
                    // let the ledger invent `key_form_id: None` for a
                    // reference with no prior override, so a keyed door's
                    // entry lost its key while the live component kept it —
                    // and the ledger wins over `XLOC` on the next load.
                    recorded = Some(DeferredLockChange::Locked {
                        lock_level: state.lock_level,
                        key_form_id: state.key_form_id,
                    });
                } else {
                    log::debug!(
                        "fragment SetLockLevel skipped: '{}' is not locked, so there is \
                         no lock record to set a difficulty on",
                        target.property_name()
                    );
                }
            }
            // #4136 — only record when the live component actually took the
            // change. Queuing it unconditionally would let the ledger
            // re-lock a door on the next cell load that `SetLockLevel`
            // explicitly declined to lock here.
            if let Some(recorded) = recorded {
                if let Some(form_id) = object_form_id(vmad, world, context, target, deferred) {
                    deferred.reference_lock_changes.push((form_id, recorded));
                }
            }
            None
        }
        _ => unreachable!("apply_lock_effect received a non-lock effect"),
    }
}

/// Player control, chargen and presentation: the `PlayerControlState` /
/// `ActorControlState` arms plus the ones that only queue a deferred
/// presentation effect.
///
/// #4340 — split out of [`apply_effect`], whose arms are now one-line
/// delegates. The bodies moved verbatim; every lock they take stays
/// named in `apply_effect`'s nested-lock residual list, which the #3949
/// scan now reads across these helpers too.
fn apply_player_control_effect(
    effect: &Effect,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    deferred: &mut DeferredFragmentEffects,
) -> Option<QuestStageAdvanced> {
    match effect {
        Effect::SetPlayerRestrained { restrained } => {
            let Some(player) = world
                .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
                .map(|player| player.0)
            else {
                log::debug!("fragment SetRestrained skipped: player entity is unavailable");
                return None;
            };
            // #5017 — through the setter, so restraining clears unconscious.
            if !crate::update_actor_control(world, player, |state| {
                state.set_restrained(*restrained)
            }) {
                log::debug!("fragment SetRestrained skipped: actor-control storage is unavailable");
            }
            None
        }
        Effect::SetPlayerControls { enabled, selection } => {
            if let Some(mut controls) = world.try_resource_mut::<crate::PlayerControlState>() {
                controls.apply_selection(*enabled, *selection);
            }
            None
        }
        Effect::SetPlayerAiDriven { ai_driven } => {
            if let Some(mut controls) = world.try_resource_mut::<crate::PlayerControlState>() {
                controls.ai_driven = *ai_driven;
            }
            None
        }
        Effect::SetHudCartMode { cart_mode } => {
            if let Some(mut controls) = world.try_resource_mut::<crate::PlayerControlState>() {
                controls.hud_cart_mode = *cart_mode;
            }
            None
        }
        Effect::SetSittingRotation { degrees } => {
            deferred.cinematic_presentation.push(
                DeferredCinematicPresentationEffect::SetSittingRotation(*degrees),
            );
            None
        }
        Effect::SetInChargen {
            disable_saving,
            disable_waiting,
            show_controls_disabled_message,
        } => {
            deferred.cinematic_presentation.push(
                DeferredCinematicPresentationEffect::SetInChargen {
                    disable_saving: *disable_saving,
                    disable_waiting: *disable_waiting,
                    show_controls_disabled_message: *show_controls_disabled_message,
                },
            );
            None
        }
        Effect::ShowRaceMenu => {
            deferred
                .cinematic_presentation
                .push(DeferredCinematicPresentationEffect::ShowRaceMenu);
            None
        }
        Effect::RequestSave { auto } => {
            deferred
                .cinematic_presentation
                .push(DeferredCinematicPresentationEffect::RequestSave { auto: *auto });
            None
        }
        Effect::RegisterPlayerAnimationEvent { event } => {
            let image_space_modifiers = match event {
                crate::CinematicAnimationEvent::PlayImod => {
                    ["PlayerAlduinIMOD", "CGDragonAttackBlurLong"]
                        .into_iter()
                        .filter_map(|property| resolve_property_form_id(vmad, property))
                        .map(|form_id| crate::ImageSpaceModifierApplication {
                            form_id,
                            strength: 1.0,
                        })
                        .collect()
                }
                crate::CinematicAnimationEvent::IdleFurnitureExit
                | crate::CinematicAnimationEvent::ExitCartEnd => Vec::new(),
            };
            deferred.cinematic_presentation.push(
                DeferredCinematicPresentationEffect::RegisterPlayerAnimationEvent {
                    event: *event,
                    quest: context,
                    image_space_modifiers,
                },
            );
            None
        }
        _ => unreachable!("apply_player_control_effect received a non-player-control effect"),
    }
}

/// The cart/vehicle cinematic arms — `ActorCinematicState`,
/// `HorseTetherState` and the `MotionTypeChangeRequest` bridge.
///
/// #4340 — split out of [`apply_effect`], whose arms are now one-line
/// delegates. The bodies moved verbatim; every lock they take stays
/// named in `apply_effect`'s nested-lock residual list, which the #3949
/// scan now reads across these helpers too.
fn apply_vehicle_cinematic_effect(
    effect: &Effect,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    deferred: &mut DeferredFragmentEffects,
) -> Option<QuestStageAdvanced> {
    match effect {
        Effect::PlayIdle { actor, idle } => {
            let actor = resolve_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            let idle_form_id = resolve_property_form_id(vmad, idle.property_name())?;
            if !update_actor_cinematic_state(world, actor, |state| {
                state.request_idle(idle_form_id);
            }) {
                log::debug!("fragment PlayIdle skipped: cinematic actor storage is unavailable");
            }
            None
        }
        Effect::SetVehicle { actor, vehicle } => {
            let actor = resolve_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            let vehicle = match vehicle {
                Some(vehicle) => Some(resolve_object(
                    vmad,
                    world,
                    context,
                    vehicle,
                    &deferred.scene_actor_bindings,
                )?),
                None => None,
            };
            let relative_pose = vehicle.and_then(|vehicle| {
                let actor_transform = copied_transform(world, actor)?;
                let vehicle_transform = copied_transform(world, vehicle)?;
                if vehicle_transform.scale.abs() <= f32::EPSILON {
                    return None;
                }
                let inverse_rotation = vehicle_transform.rotation.conjugate();
                Some((
                    inverse_rotation
                        * (actor_transform.translation - vehicle_transform.translation)
                        / vehicle_transform.scale,
                    inverse_rotation * actor_transform.rotation,
                ))
            });
            if !update_actor_cinematic_state(world, actor, |state| {
                state.vehicle = vehicle;
                if vehicle.is_some() {
                    state.cart_seat = None;
                }
                state.vehicle_local_translation = relative_pose.map(|pose| pose.0);
                state.vehicle_local_rotation = relative_pose.map(|pose| pose.1);
            }) {
                log::debug!("fragment SetVehicle skipped: cinematic actor storage is unavailable");
            }
            None
        }
        Effect::TetherToHorse { cart, horse } => {
            let cart = resolve_object(vmad, world, context, cart, &deferred.scene_actor_bindings)?;
            let horse =
                resolve_object(vmad, world, context, horse, &deferred.scene_actor_bindings)?;
            let Some(cart_transform) = copied_transform(world, cart) else {
                log::debug!("fragment TetherToHorse skipped: cart has no Transform");
                return None;
            };
            let Some(horse_transform) = copied_transform(world, horse) else {
                log::debug!("fragment TetherToHorse skipped: horse has no Transform");
                return None;
            };
            if horse_transform.scale.abs() <= f32::EPSILON {
                log::debug!("fragment TetherToHorse skipped: horse has zero scale");
                return None;
            }
            let inverse_rotation = horse_transform.rotation.conjugate();
            let state = crate::HorseTetherState {
                horse,
                horse_local_translation: inverse_rotation
                    * (cart_transform.translation - horse_transform.translation)
                    / horse_transform.scale,
                horse_local_rotation: inverse_rotation * cart_transform.rotation,
                route_target_form_id: None,
            };
            if let Some(mut tethers) = world.query_mut::<crate::HorseTetherState>() {
                tethers.insert(cart, state);
                log::info!("TetherToHorse attached cart entity {cart} to horse entity {horse}");
            }
            // The native engine realizes this relation through Havok. Redux's
            // deterministic transform follower needs the cart to stop being
            // pulled back from a dynamic Rapier body after PostUpdate, so make
            // the tethered cart keyframed through the same one-shot bridge as
            // authored SetMotionType calls.
            if let Some(mut requests) = world.query_mut::<crate::MotionTypeChangeRequest>() {
                requests.insert(
                    cart,
                    crate::MotionTypeChangeRequest {
                        motion_type: byroredux_core::ecs::components::MotionType::Keyframed,
                        allow_activate: true,
                    },
                );
            }
            None
        }
        Effect::SetMotionType {
            target,
            motion_type,
            allow_activate,
        } => {
            let target =
                resolve_object(vmad, world, context, target, &deferred.scene_actor_bindings)?;
            if let Some(mut requests) = world.query_mut::<crate::MotionTypeChangeRequest>() {
                requests.insert(
                    target,
                    crate::MotionTypeChangeRequest {
                        motion_type: *motion_type,
                        allow_activate: *allow_activate,
                    },
                );
            }
            None
        }
        Effect::ExitCart { actor, seat } => {
            let actor =
                resolve_object(vmad, world, context, actor, &deferred.scene_actor_bindings)?;
            let idle_form_id = exit_cart_idle_property(*seat)
                .and_then(|property| resolve_property_form_id(vmad, property));
            let attachment = world
                .get::<crate::ActorCinematicState>(actor)
                .and_then(|state| Some((state.vehicle?, state.vehicle_local_rotation?)));
            let exit_root_motion_rotation = world.query::<Transform>().and_then(|transforms| {
                attachment
                    .and_then(|(vehicle, local_rotation)| {
                        transforms
                            .get(vehicle)
                            .map(|vehicle| vehicle.rotation * local_rotation)
                    })
                    .or_else(|| transforms.get(actor).map(|actor| actor.rotation))
            });
            if !update_actor_cinematic_state(world, actor, |state| {
                state.vehicle = None;
                state.vehicle_local_translation = None;
                state.vehicle_local_rotation = None;
                state.cart_seat = Some(*seat);
                state.awaited_event = Some(crate::CinematicAnimationEvent::ExitCartEnd);
                state.exit_root_motion_rotation = exit_root_motion_rotation;
                if let Some(idle_form_id) = idle_form_id {
                    state.request_idle(idle_form_id);
                }
            }) {
                log::debug!("fragment ExitCart skipped: cinematic actor storage is unavailable");
            }
            None
        }
        _ => unreachable!("apply_vehicle_cinematic_effect received a non-vehicle-cinematic effect"),
    }
}

/// `SetEnemy` / `StartCombat` / `StopCombat` / `EvaluatePackage` — faction
/// relations, `AiCombatState` and package re-evaluation.
///
/// #4340 — split out of [`apply_effect`], whose arms are now one-line
/// delegates. The bodies moved verbatim; every lock they take stays
/// named in `apply_effect`'s nested-lock residual list, which the #3949
/// scan now reads across these helpers too.
fn apply_ai_combat_effect(
    effect: &Effect,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    deferred: &mut DeferredFragmentEffects,
) -> Option<QuestStageAdvanced> {
    match effect {
        Effect::SetEnemy {
            faction,
            other_faction,
            self_neutral,
            other_neutral,
        } => {
            use byroredux_sdk::relationships::CombatReaction;
            // #4318 — a `true` flag makes that direction neutral, not hostile.
            let reaction = |neutral: bool| {
                if neutral {
                    CombatReaction::Neutral
                } else {
                    CombatReaction::Enemy
                }
            };
            let faction = resolve_property_form_id(vmad, faction.property_name())?;
            let other_faction = resolve_property_form_id(vmad, other_faction.property_name())?;
            deferred.faction_relations.push((
                faction,
                other_faction,
                reaction(*self_neutral),
                reaction(*other_neutral),
            ));
            None
        }
        Effect::SetAlly {
            faction,
            other_faction,
            self_friend,
            other_friend,
        } => {
            use byroredux_sdk::relationships::CombatReaction;
            // CK *SetAlly - Faction*: a `true` flag makes that direction a
            // friend, `false` an ally.
            let reaction = |friend: bool| {
                if friend {
                    CombatReaction::Friend
                } else {
                    CombatReaction::Ally
                }
            };
            let faction = resolve_property_form_id(vmad, faction.property_name())?;
            let other_faction = resolve_property_form_id(vmad, other_faction.property_name())?;
            deferred.faction_relations.push((
                faction,
                other_faction,
                reaction(*self_friend),
                reaction(*other_friend),
            ));
            None
        }
        Effect::StartCombat { actor, target } => {
            // #5298 — the combatant must be an NPC; the target may be the
            // player (`Attacker.StartCombat(PlayerRef)` is the common shape).
            let actor =
                resolve_npc_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            // M47.2 — a ghost "will not enter combat" (CS / GECK *SetGhost*).
            if crate::is_ghost(world, actor) {
                log::debug!("fragment StartCombat skipped: the actor is a ghost");
                return None;
            }
            let target =
                resolve_actor(vmad, world, context, target, &deferred.scene_actor_bindings, deferred.speaker)?;
            if let Some(mut states) = world.query_mut::<crate::AiCombatState>() {
                states.insert(
                    actor,
                    crate::AiCombatState {
                        target,
                        attack_cooldown_remaining: 0.0,
                    },
                );
            }
            // #5046 — a scripted combat is never dropped for lost contact
            // (#4816). The ambient pass keys "this fight is mine" on its
            // marker's target matching the current one, so re-issuing combat
            // against the target it already chose must clear the marker, or
            // the fight stays on the ambient grace timer and auto-drops.
            if let Some(mut markers) = world.query_mut::<crate::AmbientEngagement>() {
                markers.remove(actor);
            }
            None
        }
        Effect::StopCombat { actor } => {
            let actor =
                resolve_npc_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            if let Some(mut states) = world.query_mut::<crate::AiCombatState>() {
                states.remove(actor);
            }
            None
        }
        Effect::SetCrimeFaction { actor, faction } => {
            let actor = resolve_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            let reference = if world
                .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
                .is_some_and(|player| player.0 == actor)
            {
                0x14
            } else {
                crate::condition::entity_global_form_id(world, actor)?
            };
            let faction = match faction {
                Some(faction) => resolve_property_form_id(vmad, faction.property_name())?,
                None => 0,
            };
            if faction != 0 {
                // "If the actor does not currently belong to the crime faction, it
                // will be added to that faction."
                deferred.faction_changes.push((actor, faction, crate::FactionChange::Add));
            }
            deferred.crime_faction_changes.push((reference, faction));
            None
        }
        Effect::StopCombatAlarm { actor } => {
            let target = resolve_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            if let Some(mut states) = world.query_mut::<crate::AiCombatState>() {
                let against: Vec<EntityId> = states
                    .iter()
                    .filter(|(_, state)| state.target == target)
                    .map(|(fighter, _)| fighter)
                    .collect();
                for fighter in against {
                    states.remove(fighter);
                }
            }
            None
        }
        Effect::SetGhost { actor, ghost } => {
            let bindings = &deferred.scene_actor_bindings;
            let actor = resolve_actor(vmad, world, context, actor, bindings, deferred.speaker)?;
            if !crate::update_actor_control(world, actor, |state| state.ghost = *ghost) {
                log::debug!("fragment SetGhost skipped: actor-control storage is unavailable");
                return None;
            }
            // A ghost "will not enter combat".
            if *ghost {
                if let Some(mut states) = world.query_mut::<crate::AiCombatState>() {
                    states.remove(actor);
                }
            }
            None
        }
        Effect::SetUnconscious { actor, unconscious } => {
            let actor =
                resolve_npc_actor(vmad, world, context, actor, &deferred.scene_actor_bindings, deferred.speaker)?;
            if !crate::update_actor_control(world, actor, |state| {
                state.set_unconscious(*unconscious)
            }) {
                log::debug!(
                    "fragment SetUnconscious skipped: actor-control storage is unavailable"
                );
                return None;
            }
            // An unconscious actor cannot fight (#5017). Its ambient package
            // is suspended by the engine's AI pass; waking leaves both to
            // re-derive (faction hostility, package selection).
            if *unconscious {
                if let Some(mut states) = world.query_mut::<crate::AiCombatState>() {
                    states.remove(actor);
                }
            }
            None
        }
        Effect::EvaluatePackage { actor } => {
            let bindings = &deferred.scene_actor_bindings;
            let actor = resolve_actor(vmad, world, context, actor, bindings, deferred.speaker)?;
            if let Some(mut requests) = world.query_mut::<crate::EvaluatePackageRequest>() {
                requests.insert(actor, crate::EvaluatePackageRequest);
            }
            None
        }
        _ => unreachable!("apply_ai_combat_effect received a non-ai/combat effect"),
    }
}

/// The original quest-scoped effect arms (`SetStage`/objectives), split
/// out so [`apply_effect`]'s object-targeting arms — which need `world`
/// but no `QuestRef` resolution — read cleanly above.
fn apply_quest_scoped_effect(
    effect: &Effect,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    stages: &mut QuestStageState,
    objectives: &mut QuestObjectiveState,
    deferred: &mut DeferredFragmentEffects,
) -> Option<QuestStageAdvanced> {
    match effect {
        Effect::SetStage { quest, stage } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            let allow_repeated = deferred
                .quest_definitions
                .as_ref()
                .is_some_and(|definitions| definitions.allows_repeated_stages(quest));
            if !allow_repeated && stages.get_stage_done(quest, *stage) {
                return None;
            }
            let was_started = stages.is_started(quest);
            let previous_stage = stages.set_stage(quest, *stage);
            if !was_started {
                deferred.scene_actor_bindings_dirty = true;
            }
            Some(QuestStageAdvanced {
                quest,
                previous_stage,
                new_stage: *stage,
                stage_set: true,
            })
        }
        Effect::StartQuest { quest } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            let start_up_stage = deferred
                .quest_definitions
                .as_ref()
                .and_then(|definitions| definitions.start_up_stage(quest));
            let event = stages.start_quest(quest, start_up_stage);
            if event.is_some() {
                deferred.scene_actor_bindings_dirty = true;
            }
            event
        }
        Effect::StopQuest { quest } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            let shut_down_stage = deferred
                .quest_definitions
                .as_ref()
                .and_then(|definitions| definitions.shut_down_stage(quest));
            let event = stages
                .is_running(quest)
                .then(|| {
                    shut_down_stage.map(|stage| {
                        let previous_stage = stages.set_stage(quest, stage);
                        QuestStageAdvanced {
                            quest,
                            previous_stage,
                            new_stage: stage,
                            stage_set: true,
                        }
                    })
                })
                .flatten();
            if stages.stop(quest) {
                deferred.scene_actor_bindings_dirty = true;
            }
            event
        }
        Effect::CompleteQuest { quest } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            if stages.complete(quest) {
                deferred.scene_actor_bindings_dirty = true;
            }
            None
        }
        Effect::ResetQuest { quest } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            stages.reset(quest);
            objectives.reset(quest);
            deferred.scene_actor_bindings_dirty = true;
            None
        }
        Effect::SetQuestActive { quest, active } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            stages.set_active(quest, *active);
            None
        }
        Effect::SetObjectiveDisplayed {
            quest,
            objective,
            displayed,
        } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            objectives.set_displayed(quest, *objective, *displayed);
            None
        }
        Effect::SetObjectiveCompleted {
            quest,
            objective,
            completed,
        } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            objectives.set_completed(quest, *objective, *completed);
            None
        }
        Effect::SetObjectiveFailed {
            quest,
            objective,
            failed,
        } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            objectives.set_failed(quest, *objective, *failed);
            None
        }
        Effect::CompleteAllObjectives { quest } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            let authored = deferred
                .quest_definitions
                .as_ref()
                .map(|definitions| definitions.objectives(quest).to_vec())
                .unwrap_or_default();
            if authored.is_empty() {
                objectives.complete_all(quest);
            } else {
                objectives.complete_all_authored(quest, authored);
            }
            None
        }
        Effect::FailAllObjectives { quest } => {
            let quest = resolve_quest_logged(quest, context, vmad)?;
            let authored = deferred
                .quest_definitions
                .as_ref()
                .map(|definitions| definitions.objectives(quest).to_vec())
                .unwrap_or_default();
            if authored.is_empty() {
                objectives.fail_all(quest);
            } else {
                objectives.fail_all_authored(quest, authored);
            }
            None
        }
        Effect::Conditional { .. }
        | Effect::QuestScope { .. }
        | Effect::SetGlobalValue { .. }
        | Effect::AddItem { .. }
        | Effect::RemoveItem { .. }
        | Effect::EquipItem { .. }
        | Effect::MoveTo { .. }
        | Effect::MovePlayerTo { .. }
        | Effect::Disable { .. }
        | Effect::Enable { .. }
        | Effect::StartScene { .. }
        | Effect::StopScene { .. }
        | Effect::Activate { .. }
        | Effect::SetOpen { .. }
        | Effect::SetLocked { .. }
        | Effect::SetLockLevel { .. }
        | Effect::SetPlayerRestrained { .. }
        | Effect::SetPlayerControls { .. }
        | Effect::SetPlayerAiDriven { .. }
        | Effect::SetHudCartMode { .. }
        | Effect::SetInChargen { .. }
        | Effect::ShowRaceMenu
        | Effect::RequestSave { .. }
        | Effect::SetQuestVariable { .. }
        | Effect::ModQuestVariable { .. }
        | Effect::CopyQuestVariable { .. }
        | Effect::SetActorValueNamed { .. }
        | Effect::DamageActorValue { .. }
        | Effect::ChangeFaction { .. }
        | Effect::SetActorValue { .. }
        | Effect::ForceAlias { .. }
        | Effect::BlockActivation { .. }
        | Effect::AddToMap { .. }
        | Effect::SetLocationCleared { .. }
        | Effect::SetBaseProtection { .. }
        | Effect::Kill { .. }
        | Effect::SetRelationshipRank { .. }
        | Effect::RewardPlayerXp { .. }
        | Effect::IncrementStat { .. }
        | Effect::AddAchievement { .. }
        | Effect::AdvanceSkill { .. }
        | Effect::SetLinkedRef { .. }
        | Effect::SetLocRefType { .. }
        | Effect::MoveToEditorLocation { .. }
        | Effect::SetLocationKeywordData { .. }
        | Effect::ForceLocationTo { .. }
        | Effect::SetPlayerTeammate { .. }
        | Effect::SetDoingFavor { .. }
        | Effect::PlayerCaps { .. }
        | Effect::SetLocal { .. }
        | Effect::SetReferenceVariable { .. }
        | Effect::ShowMessage { .. }
        | Effect::ShowHelpMessage { .. }
        | Effect::Notification { .. }
        | Effect::ShowBarterMenu { .. }
        | Effect::DeleteWhenAble { .. }
        | Effect::ChangeKeyword { .. }
        | Effect::SendStoryEvent { .. }
        | Effect::ChangePerk { .. }
        | Effect::SetEnemy { .. }
        | Effect::StartCombat { .. }
        | Effect::StopCombat { .. }
        | Effect::StopCombatAlarm { .. }
        | Effect::SetCrimeFaction { .. }
        | Effect::SetUnconscious { .. }
        | Effect::SetGhost { .. }
        | Effect::SetAlly { .. }
        | Effect::AddSpell { .. }
        | Effect::RemoveSpell { .. }
        | Effect::PlayIdle { .. }
        | Effect::SetVehicle { .. }
        | Effect::TetherToHorse { .. }
        | Effect::SetMotionType { .. }
        | Effect::SetSittingRotation { .. }
        | Effect::ExitCart { .. }
        | Effect::RegisterPlayerAnimationEvent { .. }
        | Effect::EvaluatePackage { .. }
        | Effect::Wait { .. }
        | Effect::WaitForActors3DLoaded { .. }
        | Effect::WaitWhile { .. }
        | Effect::Interpret { .. }
        | Effect::AddRaceSpells { .. }
        | Effect::RemoveRaceSpells { .. }
        | Effect::ProviderCall(_) => {
            unreachable!("object-targeting effects are handled by apply_effect directly")
        }
    }
}

/// Evaluate one [`FragmentGuard`] (M47.2) against live quest, objective,
/// global and quest-variable state. `None` when its quest or global does not
/// resolve — the caller then declines the whole `Conditional`.
///
/// A quest-variable read sees a write queued earlier in the same fragment
/// (`deferred.quest_variables`, applied only once the quest guards drop) —
/// Papyrus assignments are immediate. `Globals` and the quest-variable
/// resources are read under the quest-state guards, the same nested reads
/// the `SetGlobalValue` arm already takes.
/// A message an effect raised, shown once the quest guards drop.
#[derive(Debug)]
enum PendingMessage {
    Show(u32, Vec<f32>),
    Notify(String),
}

/// M47.2 — a [`ValueExpr`]'s run-time number; `None` when something it reads
/// does not resolve (the guard then declines the conditional). Quest-variable
/// and relationship reads see a write made earlier in the same fragment.
/// An actor's current value of `avif` — the `GetActorValue` condition read.
fn actor_value_now(world: &World, actor: EntityId, avif: u32) -> f32 {
    crate::condition::evaluate_function(
        crate::condition::ConditionFunction::GetActorValue,
        &byroredux_plugin::esm::records::condition::Condition {
            param_1: avif,
            ..Default::default()
        },
        actor,
        world,
    )
}

/// The (quest, alias id) the script property `name` is bound to, a quest-less
/// binding (form 0) meaning the running quest `context`.
fn alias_property(vmad: Option<&ScriptInstanceData>, context: QuestFormId, name: &str) -> Option<(QuestFormId, i32)> {
    vmad?
        .scripts
        .iter()
        .find_map(|script| script.property(name))
        .and_then(|property| match property.value {
            PropertyValue::Object { form_id, alias } if alias >= 0 => Some((
                if form_id != 0 { QuestFormId(form_id) } else { context },
                i32::from(alias),
            )),
            _ => None,
        })
}

pub(super) fn evaluate_value(
    value: &ValueExpr,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    stages: &QuestStageState,
    deferred: &DeferredFragmentEffects,
) -> Option<f32> {
    Some(match value {
        ValueExpr::Literal(value) => *value,
        ValueExpr::Local(local) => deferred.locals.get(local).copied().unwrap_or(0.0),
        ValueExpr::Stage(via) => f32::from(stages.get_stage(resolve_quest_logged(via, context, vmad)?)),
        ValueExpr::Global(global) => {
            let form_id = resolve_property_form_id(vmad, global.property_name())?;
            world
                .try_resource::<crate::Globals>()
                .and_then(|globals| globals.get(form_id))?
        }
        ValueExpr::QuestVariable(via, variable) => {
            let quest = resolve_quest_logged(via, context, vmad)?;
            let key = crate::QuestScriptVariables::property_key(variable);
            let pending = deferred
                .quest_variables
                .iter()
                .rev()
                .find(|(q, k, _)| *q == quest && *k == key)
                .map(|(_, _, value)| *value);
            pending
                .or_else(|| crate::quest_variable(world, quest, key))
                .unwrap_or(0.0)
        }
        ValueExpr::ItemCount(container, item) => {
            let container = guard_actor(container, context, vmad, world, deferred)?;
            let item = resolve_property_form_id(vmad, item.property_name())?;
            let count: u32 = world
                .get::<Inventory>(container)
                .map(|inventory| {
                    inventory
                        .items
                        .iter()
                        .filter(|stack| stack.base_form_id == item)
                        .map(|stack| stack.count)
                        .sum()
                })
                .unwrap_or(0);
            count as f32
        }
        ValueExpr::Distance(from, to) => {
            use byroredux_core::ecs::components::GlobalTransform;
            let from = guard_actor(from, context, vmad, world, deferred)?;
            let to = guard_actor(to, context, vmad, world, deferred)?;
            // Like the GetDistance condition: a reference with no placement
            // reads as infinitely far.
            let from = world.get::<GlobalTransform>(from).map(|t| t.translation);
            let to = world.get::<GlobalTransform>(to).map(|t| t.translation);
            match (from, to) {
                (Some(from), Some(to)) => (from - to).length(),
                _ => f32::MAX,
            }
        }
        ValueExpr::RelationshipRank(actor, other) => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let other = guard_actor(other, context, vmad, world, deferred)?;
            let pair = crate::relationships::actor_base(world, actor)
                .zip(crate::relationships::actor_base(world, other))
                .map(|(a, b)| crate::relationships::pair(a, b));
            // A `SetRelationshipRank` earlier in the same fragment is visible.
            let pending = pair.and_then(|pair| {
                deferred
                    .relationship_changes
                    .iter()
                    .rev()
                    .find(|(changed, _)| *changed == pair)
                    .map(|&(_, rank)| rank)
            });
            f32::from(
                pending.unwrap_or_else(|| crate::relationships::relationship_rank(world, actor, other)),
            )
        }
        ValueExpr::ActorValue(actor, actor_value) => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let avif = resolve_property_form_id(vmad, actor_value.property_name())?;
            actor_value_now(world, actor, avif)
        }
        ValueExpr::ActorValueNamed(actor, name) => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let avif = world
                .try_resource::<crate::ActorValueNames>()
                .and_then(|names| names.resolve(name))?;
            actor_value_now(world, actor, avif)
        }
        ValueExpr::ActorValueReading(reading, actor, arg) => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let avif = match arg {
                crate::translate::effects::ActorValueArg::Form(form) => {
                    resolve_property_form_id(vmad, form.property_name())?
                }
                crate::translate::effects::ActorValueArg::Named(name) => world
                    .try_resource::<crate::ActorValueNames>()
                    .and_then(|names| names.resolve(name))?,
            };
            use crate::condition::{actor_value_view, ActorValueView};
            match reading {
                crate::translate::effects::ActorValueReading::Base => {
                    actor_value_view(world, actor, avif, ActorValueView::Base)
                }
                // Current over the undamaged ceiling; an actor value the
                // actor lacks reads 0, the absent-AV default.
                crate::translate::effects::ActorValueReading::Percent => {
                    let ceiling = actor_value_view(world, actor, avif, ActorValueView::Undamaged);
                    if ceiling > 0.0 {
                        (actor_value_view(world, actor, avif, ActorValueView::Current) / ceiling).clamp(0.0, 1.0)
                    } else {
                        0.0
                    }
                }
            }
        }
        ValueExpr::LocationKeywordData(location, keyword) => {
            let location = resolve_property_form_id(vmad, location.property_name())?;
            let keyword = resolve_property_form_id(vmad, keyword.property_name())?;
            // A write earlier in the same run is visible.
            deferred
                .location_keyword_changes
                .iter()
                .rev()
                .find(|(changed, key, _)| *changed == location && *key == keyword)
                .map(|(_, _, value)| *value)
                .unwrap_or_else(|| crate::location_keyword_data(world, location, keyword))
        }
        // A read sees the increments queued earlier in the same run.
        ValueExpr::MiscStat(stat) => {
            let stored = world.try_resource::<crate::MiscStats>().map_or(0, |stats| stats.get(stat));
            let pending: i32 = deferred
                .misc_stats
                .iter()
                .filter(|(queued, _)| queued.eq_ignore_ascii_case(stat))
                .map(|(_, amount)| *amount)
                .sum();
            stored.saturating_add(pending) as f32
        }
        ValueExpr::Level(actor) => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            crate::condition::evaluate_function(
                crate::condition::ConditionFunction::GetLevel,
                &byroredux_plugin::esm::records::condition::Condition::default(),
                actor,
                world,
            )
        }
        ValueExpr::RandomInt(min, max) => {
            // `Utility.RandomInt` draws from the seeded quest RNG the Story
            // Manager and `GetRandomPercent` share; unresolved without one.
            if max < min {
                return Some(*min as f32);
            }
            let span = (i64::from(*max) - i64::from(*min) + 1) as usize;
            let pick = world
                .try_resource_mut::<crate::story_manager::StoryManagerRng>()
                .map(|mut rng| rng.pick(span))?;
            (i64::from(*min) + pick as i64) as f32
        }
        ValueExpr::Arith(left, op, right) => op.apply(
            evaluate_value(left, context, vmad, world, stages, deferred)?,
            evaluate_value(right, context, vmad, world, stages, deferred)?,
        )?,
    })
}

pub(super) fn evaluate_guard(
    guard: &FragmentGuard,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    stages: &QuestStageState,
    objectives: &QuestObjectiveState,
    deferred: &DeferredFragmentEffects,
) -> Option<bool> {
    let quest = |via: &QuestRef| resolve_quest_logged(via, context, vmad);
    Some(match guard {
        FragmentGuard::StageDone { quest: via, stage, done } => {
            stages.get_stage_done(quest(via)?, *stage) == *done
        }
        FragmentGuard::QuestRunning { quest: via, running } => {
            stages.is_running(quest(via)?) == *running
        }
        FragmentGuard::Stage {
            quest: via,
            comparison,
            value,
        } => comparison.apply(
            evaluate_value(&ValueExpr::Stage(via.clone()), context, vmad, world, stages, deferred)?,
            *value,
        ),
        FragmentGuard::ObjectiveDisplayed {
            quest: via,
            objective,
            expected,
        } => objectives.get(quest(via)?, *objective).displayed == *expected,
        FragmentGuard::ObjectiveCompleted {
            quest: via,
            objective,
            expected,
        } => objectives.get(quest(via)?, *objective).completed == *expected,
        FragmentGuard::Global {
            global,
            comparison,
            value,
        } => comparison.apply(
            evaluate_value(&ValueExpr::Global(global.clone()), context, vmad, world, stages, deferred)?,
            *value,
        ),
        FragmentGuard::QuestVariable {
            quest: via,
            variable,
            comparison,
            value,
        } => comparison.apply(
            evaluate_value(
                &ValueExpr::QuestVariable(via.clone(), variable.clone()),
                context,
                vmad,
                world,
                stages,
                deferred,
            )?,
            *value,
        ),
        FragmentGuard::ActorDead { actor, dead } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            world.has::<byroredux_core::ecs::components::Dead>(actor) == *dead
        }
        FragmentGuard::ActorBleedingOut { actor, expected } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            crate::bleedout::is_bleeding_out(world, actor) == *expected
        }
        FragmentGuard::ActorInCombat { actor, expected } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            crate::is_in_combat(world, actor) == *expected
        }
        FragmentGuard::ActorFollower { actor, flag, expected } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let form = crate::condition::entity_global_form_id(world, actor)?;
            // A write earlier in the same run is visible.
            let mut value = world
                .try_resource::<crate::FollowerState>()
                .is_some_and(|state| state.read(form, *flag));
            for (changed, change) in &deferred.follower_changes {
                if *changed != form {
                    continue;
                }
                match (change, flag) {
                    (FollowerChange::Teammate { teammate, .. }, crate::FollowerFlag::Teammate) => value = *teammate,
                    (FollowerChange::DoingFavor(doing), crate::FollowerFlag::DoingFavor) => value = *doing,
                    _ => {}
                }
            }
            value == *expected
        }
        FragmentGuard::ActorEssential { actor, expected } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            crate::bleedout::is_essential(world, actor) == *expected
        }
        FragmentGuard::ReferenceEnabled { object, expected } => {
            let form_id = object_form_id(vmad, world, context, object, deferred)?;
            // An `Enable()` / `Disable()` earlier in the same fragment is
            // visible.
            let enabled = match deferred
                .reference_enable_changes
                .iter()
                .rev()
                .find(|(changed, _)| *changed == form_id)
            {
                Some(&(_, enabled)) => enabled,
                None => super::reference_enabled(world, form_id),
            };
            enabled == *expected
        }
        FragmentGuard::InFaction {
            actor,
            faction,
            expected,
        } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let faction = resolve_property_form_id(vmad, faction.property_name())?;
            let member = crate::condition::evaluate_function(
                crate::condition::ConditionFunction::GetInFaction,
                &byroredux_plugin::esm::records::condition::Condition {
                    param_1: faction,
                    ..Default::default()
                },
                actor,
                world,
            ) == 1.0;
            member == *expected
        }
        FragmentGuard::HasLocRefType {
            actor,
            ref_type,
            expected,
        } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let ref_type = resolve_property_form_id(vmad, ref_type.property_name())?;
            let reference = crate::condition::entity_global_form_id(world, actor).unwrap_or(0);
            let has = crate::has_loc_ref_type(world, actor, reference, ref_type);
            has == *expected
        }
        FragmentGuard::Loaded3D { object, expected } => {
            // A reference with no live entity is not loaded.
            let form_id = object_form_id(vmad, world, context, object, deferred)?;
            let loaded = crate::condition::resolve_entity_by_global_form_id(world, form_id)
                .is_some_and(|entity| {
                    crate::condition::evaluate_function(
                        crate::condition::ConditionFunction::HasLoaded3D,
                        &byroredux_plugin::esm::records::condition::Condition::default(),
                        entity,
                        world,
                    ) == 1.0
                });
            loaded == *expected
        }
        FragmentGuard::RelationshipRank {
            actor,
            other,
            comparison,
            value,
        } => comparison.apply(
            evaluate_value(
                &ValueExpr::RelationshipRank(actor.clone(), other.clone()),
                context,
                vmad,
                world,
                stages,
                deferred,
            )?,
            *value,
        ),
        FragmentGuard::Distance {
            from,
            to,
            comparison,
            value,
        } => comparison.apply(
            evaluate_value(
                &ValueExpr::Distance(from.clone(), to.clone()),
                context,
                vmad,
                world,
                stages,
                deferred,
            )?,
            *value,
        ),
        FragmentGuard::Compare {
            left,
            comparison,
            right,
        } => {
            let left = evaluate_value(left, context, vmad, world, stages, deferred)?;
            let right = evaluate_value(right, context, vmad, world, stages, deferred)?;
            comparison.apply(left, right)
        }
        FragmentGuard::HasPerk {
            actor,
            perk,
            expected,
        } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let perk = resolve_property_form_id(vmad, perk.property_name())?;
            let owned = world
                .get::<byroredux_core::character::Perks>(actor)
                .is_some_and(|perks| perks.rank(perk) > 0);
            owned == *expected
        }
        FragmentGuard::ItemCount {
            container,
            item,
            comparison,
            value,
        } => comparison.apply(
            evaluate_value(
                &ValueExpr::ItemCount(container.clone(), item.clone()),
                context,
                vmad,
                world,
                stages,
                deferred,
            )?,
            *value,
        ),
        FragmentGuard::QuestCompleted { quest: via, completed } => {
            stages.is_completed(quest(via)?) == *completed
        }
        FragmentGuard::ScenePlaying { scene, playing } => {
            let scene = resolve_property_form_id(vmad, scene.property_name())?;
            deferred.running_scenes.contains(&scene) == *playing
        }
        FragmentGuard::SameReference {
            left,
            right,
            expected,
        } => {
            let left = reference_identity(left, context, vmad, world, deferred);
            let right = right
                .as_ref()
                .and_then(|right| reference_identity(right, context, vmad, world, deferred));
            (left == right) == *expected
        }
        FragmentGuard::HasKeyword {
            object,
            keyword,
            expected,
        } => {
            let object = guard_actor(object, context, vmad, world, deferred)?;
            let keyword = resolve_property_form_id(vmad, keyword.property_name())?;
            // This fragment's own pending change wins, as it will once applied.
            let pending = deferred
                .keyword_changes
                .iter()
                .rev()
                .find(|(changed, k, _)| *changed == object && *k == keyword)
                .map(|(_, _, change)| *change);
            let has = match pending {
                Some(crate::keywords::KeywordChange::Add) => true,
                Some(crate::keywords::KeywordChange::Remove) => false,
                Some(crate::keywords::KeywordChange::Reset) | None => {
                    crate::keywords::has_keyword(world, object, keyword)
                }
            };
            has == *expected
        }
        FragmentGuard::LocationKeyword {
            actor,
            keyword,
            expected,
        } => {
            let actor = guard_actor(actor, context, vmad, world, deferred)?;
            let keyword = resolve_property_form_id(vmad, keyword.property_name())?;
            let location = crate::location::current_location(world, actor)?;
            let has = world
                .try_resource::<crate::location::LocationTable>()
                .is_some_and(|table| table.has_keyword(location, keyword));
            has == *expected
        }
        FragmentGuard::MapMarkerVisible { marker, expected } => {
            let marker = object_form_id(vmad, world, context, marker, deferred)?;
            let pending = deferred.map_markers.iter().any(|(added, _)| *added == marker);
            (pending || crate::map_marker_visible(world, marker)) == *expected
        }
        FragmentGuard::MapMarkerFastTravel { marker, expected } => {
            let marker = object_form_id(vmad, world, context, marker, deferred)?;
            let pending = deferred
                .map_markers
                .iter()
                .any(|(added, travel)| *added == marker && *travel);
            (pending || crate::map_marker_fast_travel(world, marker)) == *expected
        }
        FragmentGuard::LocationCleared { location, expected } => {
            let location = resolve_location(vmad, world, context, location)?;
            // A `SetCleared` earlier in the same fragment is visible.
            let cleared = match deferred.location_clears.iter().rev().find(|(set, _)| *set == location) {
                Some(&(_, cleared)) => cleared,
                None => crate::location_cleared(world, location),
            };
            cleared == *expected
        }
        FragmentGuard::Any(alternatives) => {
            // Every guard is evaluated (no short-circuit), so an unresolvable
            // one declines the conditional exactly as it does in a
            // conjunction.
            let mut any = false;
            for alternative in alternatives {
                let mut all = true;
                for guard in alternative {
                    all &= evaluate_guard(guard, context, vmad, world, stages, objectives, deferred)?;
                }
                any |= all;
            }
            any
        }
    })
}

/// The reference FormID an [`ActorRef`] names: `0x14` for the player, else the
/// reference's own FormID (a resident entity's identity, or a remote alias
/// stub's candidate FormID).
fn actor_reference_form_id(
    actor: &ActorRef,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    deferred: &DeferredFragmentEffects,
) -> Option<u32> {
    if matches!(actor, ActorRef::Player) {
        return Some(crate::PLAYER_REFERENCE);
    }
    if let ActorRef::Object(object) = actor {
        if let Some(form_id) = resolve_property_form_id(vmad, object.property_name()) {
            return Some(form_id);
        }
    }
    let entity = guard_actor(actor, context, vmad, world, deferred)?;
    entity_global_form_id(world, entity).or_else(|| {
        world
            .get::<crate::scene::SceneAliasCandidate>(entity)
            .map(|candidate| candidate.reference_form_id)
    })
}

/// Set `actor`'s base `avif`, creating `ActorValues` if it has none.
/// `ActorValues` is direct component storage — written in place, the same
/// nested-acquisition exception the inventory arms take.
fn set_actor_value_base(world: &World, actor: EntityId, avif: u32, value: f32) {
    let Some(mut values) = world.query_mut::<byroredux_core::ecs::components::ActorValues>()
    else {
        return;
    };
    if values.get_mut(actor).is_none() {
        values.insert(actor, byroredux_core::ecs::components::ActorValues::new());
    }
    values
        .get_mut(actor)
        .expect("present or inserted")
        .set_base(avif, value);
}

/// The `LCTN` a location-typed [`ObjectRef`] names: a direct property's
/// FormID, or the location bound to the location alias the property points
/// at.
fn resolve_location(
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    context: QuestFormId,
    location: &ObjectRef,
) -> Option<u32> {
    let value = vmad?
        .scripts
        .iter()
        .find_map(|script| script.property(location.property_name()))
        .map(|property| &property.value)?;
    match value {
        PropertyValue::Object { form_id, alias: -1 } => Some(*form_id),
        PropertyValue::Object { form_id, alias } if *alias >= 0 => {
            let quest = if *form_id != 0 {
                QuestFormId(*form_id)
            } else {
                context
            };
            world
                .try_resource::<crate::LocationAliasBindings>()?
                .resolve(quest, i32::from(*alias))
        }
        _ => None,
    }
}

/// The actor a guard reads, resolved like an effect's receiver.
fn guard_actor(
    actor: &ActorRef,
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    deferred: &DeferredFragmentEffects,
) -> Option<EntityId> {
    resolve_actor(
        vmad,
        world,
        context,
        actor,
        &deferred.scene_actor_bindings,
        deferred.speaker,
    )
}

/// Apply a whole fragment's effects, returning the chained
/// [`QuestStageAdvanced`]s its `SetStage`s produced. Resource reads use the
/// pre-lock snapshot in `deferred`; queued mutations must be applied only after
/// releasing the quest-state guards passed here.
pub fn apply_effects(
    effects: &[Effect],
    context: QuestFormId,
    vmad: Option<&ScriptInstanceData>,
    world: &World,
    stages: &mut QuestStageState,
    objectives: &mut QuestObjectiveState,
    deferred: &mut DeferredFragmentEffects,
) -> Vec<QuestStageAdvanced> {
    let mut advances = Vec::new();
    // #3935 — an explicit cursor stack, not recursion. `Effect::Conditional`
    // used to build `branch ++ effects[index + 1..]` and call `apply_effects`
    // on it, so a fragment with N *sequential* conditionals cost N stack
    // frames and cloned the whole remaining program at each one — O(N^2)
    // live `Effect` clones. `MAX_CONDITIONAL_DEPTH` bounds conditional
    // *nesting* at lowering, not the sequential count, and a `.pex` function
    // may carry 65 535 instructions, so N was bounded only by the wire
    // format. Descending now pushes two borrowed slices (the branch and the
    // rest of the current program) and loops; nothing is cloned, and the
    // stack holds slices, not frames.
    let mut pending: Vec<&[Effect]> = Vec::new();
    let mut current: &[Effect] = effects;
    loop {
        let mut halted = false;
        for (index, effect) in current.iter().enumerate() {
            if let Effect::ProviderCall(call) = effect {
                deferred.provider_steps.push(DeferredProviderFragmentStep {
                    call: call.clone(),
                    context,
                    vmad: vmad.cloned(),
                    tail: remaining_program(current, index, &pending),
                });
                halted = true;
                break;
            }
            // M47.2 — a fragment the lowering declined, run by the Papyrus VM.
            if let Effect::Interpret {
                script,
                function,
                args,
                resume,
            } = effect
            {
                // The frame's interpreter budget is spent: this run, and the
                // rest of the program behind it, start on a later frame.
                if super::budget::exhausted(world) {
                    let mut tail = vec![effect.clone()];
                    tail.extend(remaining_program(current, index, &pending));
                    if super::budget::defer(world, context, vmad, tail).is_ok() {
                        halted = true;
                        break;
                    }
                }
                let (run, suspended) = super::vm::run_fragment(
                    script,
                    function,
                    args,
                    resume.as_ref(),
                    context,
                    vmad,
                    world,
                    stages,
                    objectives,
                    deferred,
                );
                advances.extend(run);
                // A `Utility.Wait`: the run resumes there, then the rest of
                // the program follows.
                if let Some((seconds, state)) = suspended {
                    let mut tail = vec![Effect::Interpret {
                        script: script.clone(),
                        function: function.clone(),
                        args: Vec::new(),
                        resume: Some(state),
                    }];
                    tail.extend(remaining_program(current, index, &pending));
                    if let Some(mut queue) = world.try_resource_mut::<FragmentExecutionQueue>() {
                        queue.pending.push(PendingFragmentExecution {
                            context,
                            vmad: vmad.cloned(),
                            effects: tail,
                            remaining_seconds: seconds,
                            resume_when: FragmentResumeCondition::DelayElapsed,
                        });
                    } else {
                        log::debug!("Papyrus VM continuation dropped: FragmentExecutionQueue is unavailable");
                    }
                    halted = true;
                    break;
                }
                continue;
            }
            // M47.2 — a quest-script body inlined into an INFO or scene
            // fragment runs against its quest: that quest is the context and
            // its VMAD binds the properties. It holds nothing latent, so it
            // runs to completion here.
            if let Effect::QuestScope {
                quest,
                effects: scoped,
                bindings,
            } = effect
            {
                let Some(quest) = resolve_quest_logged(quest, context, vmad) else {
                    continue;
                };
                let scripts = deferred.quest_scripts.clone();
                let Some(quest_vmad) = scripts.get(&quest) else {
                    log::debug!("fragment QuestScope skipped: quest {:08X} has no VMAD", quest.0);
                    continue;
                };
                // M47.2 — the inlined function's object arguments that name
                // this fragment's properties: the quest's VMAD plus one
                // script carrying each as the parameter's property, valued
                // as the fragment's.
                let bound;
                let quest_vmad = if bindings.is_empty() {
                    quest_vmad
                } else {
                    let mut merged = quest_vmad.clone();
                    merged.scripts.push(byroredux_plugin::esm::records::script_instance::ScriptInstance {
                        name: "<inline arguments>".into(),
                        status: 0,
                        properties: bindings
                            .iter()
                            .filter_map(|(parameter, argument)| {
                                let value = vmad?
                                    .scripts
                                    .iter()
                                    .find_map(|script| script.property(argument))?
                                    .value
                                    .clone();
                                Some(byroredux_plugin::esm::records::script_instance::ScriptProperty {
                                    name: parameter.clone(),
                                    status: 1,
                                    value,
                                })
                            })
                            .collect(),
                    });
                    bound = merged;
                    &bound
                };
                advances.extend(apply_effects(
                    scoped,
                    quest,
                    Some(quest_vmad),
                    world,
                    stages,
                    objectives,
                    deferred,
                ));
                continue;
            }
            if let Effect::Conditional {
                guards,
                then_effects,
                else_effects,
            } = effect
            {
                // #3785 — `is_some_and` collapsed "the guard evaluated false" and
                // "the guard's quest ref could not be resolved at all" into the
                // same `false`, and unlike every sibling `resolve_quest_logged`
                // caller (which simply skips the one effect via `?`), `false`
                // here is NOT inert — it selects `else_effects` and runs them.
                // An unresolvable guard must decline the WHOLE Conditional
                // (neither branch), matching the decline-on-unmodeled-reference
                // invariant every other site already follows.
                let mut resolved = true;
                let passes = guards.iter().all(|guard| {
                    match evaluate_guard(guard, context, vmad, world, stages, objectives, deferred) {
                        Some(pass) => pass,
                        None => {
                            resolved = false;
                            false
                        }
                    }
                });
                if !resolved {
                    // `resolve_quest_logged` already emitted a debug line per
                    // unresolved guard (correct for its inert callers); this is
                    // the one site where the consequence is a chosen branch, so
                    // it gets its own louder diagnostic.
                    log::warn!(
                        "fragment effect: Conditional guard's quest ref could not be resolved — \
                     declining the whole Conditional (neither then_effects nor else_effects run)"
                    );
                    continue;
                }
                let branch = if passes { then_effects } else { else_effects };
                // Push the continuation first, then the branch: the stack pops
                // the branch next, so the visible order is still
                // `branch ++ rest-of-program`, exactly as the flattened
                // recursion produced.
                pending.push(&current[index + 1..]);
                pending.push(branch.as_slice());
                break;
            }
            // M47.2 — a `While … Wait` poll: while its guards hold, suspend
            // with this effect at the head of the continuation, so the resume
            // checks again.
            if let Effect::WaitWhile {
                guards,
                poll_seconds,
            } = effect
            {
                let mut resolved = true;
                let holds = guards.iter().all(|guard| {
                    match evaluate_guard(guard, context, vmad, world, stages, objectives, deferred) {
                        Some(pass) => pass,
                        None => {
                            resolved = false;
                            false
                        }
                    }
                });
                if !resolved {
                    log::debug!("fragment WaitWhile ended: a guard could not be read");
                }
                if holds {
                    let mut tail = vec![effect.clone()];
                    tail.extend(remaining_program(current, index, &pending));
                    if let Some(mut queue) = world.try_resource_mut::<FragmentExecutionQueue>() {
                        queue.pending.push(PendingFragmentExecution {
                            context,
                            vmad: vmad.cloned(),
                            effects: tail,
                            remaining_seconds: *poll_seconds,
                            resume_when: FragmentResumeCondition::DelayElapsed,
                        });
                    } else {
                        log::debug!(
                            "fragment continuation dropped: FragmentExecutionQueue is unavailable"
                        );
                    }
                    halted = true;
                    break;
                }
                continue;
            }
            let suspension = match effect {
                Effect::Wait { seconds } => Some((*seconds, FragmentResumeCondition::DelayElapsed)),
                Effect::WaitForActors3DLoaded {
                    actors,
                    poll_seconds,
                } if !actors_3d_loaded(
                    vmad,
                    world,
                    context,
                    actors,
                    &deferred.scene_actor_bindings,
                ) =>
                {
                    Some((
                        *poll_seconds,
                        FragmentResumeCondition::Actors3DLoaded {
                            actors: actors.clone(),
                            poll_seconds: *poll_seconds,
                            elapsed_seconds: 0.0,
                        },
                    ))
                }
                Effect::WaitForActors3DLoaded { .. } => None,
                _ => None,
            };
            if let Some((remaining_seconds, resume_when)) = suspension {
                let tail = remaining_program(current, index, &pending);
                if !tail.is_empty() {
                    if let Some(mut queue) = world.try_resource_mut::<FragmentExecutionQueue>() {
                        queue.pending.push(PendingFragmentExecution {
                            context,
                            vmad: vmad.cloned(),
                            effects: tail,
                            remaining_seconds,
                            resume_when,
                        });
                    } else {
                        log::debug!(
                            "fragment continuation dropped: FragmentExecutionQueue is unavailable"
                        );
                    }
                }
                halted = true;
                break;
            }
            if matches!(effect, Effect::WaitForActors3DLoaded { .. }) {
                continue;
            }
            if let Some(advance) =
                apply_effect(effect, context, vmad, world, stages, objectives, deferred)
            {
                advances.push(advance);
            }
        }
        if halted {
            break;
        }
        // Either the current slice ran to its end, or a Conditional just
        // pushed its branch — both continue at the top of the stack.
        match pending.pop() {
            Some(next) => current = next,
            None => break,
        }
    }
    advances
}

/// The program still to run after `current[index]`: the rest of the current
/// slice followed by everything still on the cursor stack, in execution
/// order (#3935).
///
/// A suspension or provider barrier hands its continuation to another frame
/// (`FragmentExecutionQueue` / `DeferredFragmentEffects`), which outlives
/// the borrowed slices, so this is the one place the remainder is cloned —
/// once, at the point it actually has to be owned. `pending` is a stack, so
/// its execution order is back-to-front.
fn remaining_program(current: &[Effect], index: usize, pending: &[&[Effect]]) -> Vec<Effect> {
    let mut tail = current[index + 1..].to_vec();
    for slice in pending.iter().rev() {
        tail.extend_from_slice(slice);
    }
    tail
}
