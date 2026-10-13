//! Scripting subsystem — ECS-native event system.
//!
//! Replaces the Papyrus VM with native ECS patterns:
//! - Script state → component fields
//! - Script logic → ECS systems
//! - Events → transient marker components
//! - Timers → ScriptTimer component + tick system
//!
//! The core lifecycle: event marker appears → systems process it →
//! cleanup removes it at end of frame.

pub mod actor_value_names;
pub mod alias_overrides;
pub mod bleedout;
pub mod cinematic;
pub mod cleanup;
pub mod combat;
pub mod compatibility;
pub mod condition;
pub mod editor_location;
pub mod created_references;
pub mod dialogue;
pub mod equipment;
pub mod events;
pub mod faction_changes;
pub mod fragment;
pub mod globals;
pub mod load_order;
pub mod map_markers;
pub mod keywords;
pub mod menus;
pub mod alias_collections;
pub mod followers;
pub mod linked_refs;
pub mod messages;
pub mod skills;
pub mod misc_stats;
pub mod location;
pub mod magic;
pub mod obscript;
pub mod obscript_quests;
pub mod obscript_runtime;
pub mod obscript_vm;
pub mod package;
pub mod perk_entry_points;
pub mod papyrus_demo;
pub mod papyrus_provider;
pub mod player_control;
pub mod quest_stages;
pub mod quest_variables;
pub mod relationships;
pub mod recurring_update;
pub mod registry;
pub mod scene;
pub mod story_manager;
pub mod timer;
pub mod translate;
pub mod trigger;
pub mod vm_state;

pub use cinematic::{
    dispatch_player_cinematic_animation_event, image_space_modifier_system,
    install_image_space_modifiers, ActorCinematicState, CinematicAnimationEvent,
    CinematicPresentationState, HorseTetherState, ImageSpace, ImageSpaceBase, ImageSpaceModifier,
    ImageSpaceModifierApplication, MotionTypeChangeRequest,
};
pub use cleanup::event_cleanup_system;
pub use story_manager::{
    build_story_manager_tree, emit_change_location_on_key_change, install_story_manager,
    story_manager_dispatch_system, SmNodeRuntime, SmTree, SmTreeNode, StoryClock, StoryEvent,
    StoryEventAliasFill, StoryEventFill, StoryLocationCursor, StoryManagerNodeState, StoryManagerRng,
};
pub use combat::{is_in_combat, AiCombatState, AmbientEngagement, FactionReactionOverride, FactionRelations};
pub use compatibility::{
    analyze_obscript_bytecode_compatibility, analyze_pex_compatibility,
    analyze_source_compatibility, classify_method_call, classify_static_call,
    record_compatibility_report, source_alias, CompatibilityDisposition, CompatibilityFinding,
    CompatibilityMatch, CompatibilityRegistry, CompatibilityReport, CompatibilitySummaryEntry,
    SourceAlias, SourceCompatibilityFinding, SourceCompatibilityReport,
};
pub use condition::{
    evaluate as evaluate_condition_list, evaluate_condition, evaluate_function, ConditionContext,
    ConditionFunction,
};
pub use dialogue::{
    estimate_dialogue_duration, install_dialogue_quest_conditions, install_dialogue_records,
    note_info_spoken,
    scene_dialogue_system, select_first_info, ActiveDialogueLine, DialogueLine,
    DialogueLineCompletionBatch, DialoguePlayback, DialoguePresentationEvent,
    DialoguePresentationEventBatch, DialogueQuestConditions, DialogueQuestPriorities,
    DialogueRandomState,
    DialogueRegistry, DialogueSpokenInfoForms,
};
pub use equipment::{
    emit_equipment_changes, emit_item_transfers, install_equip_item_catalog, EquipItemCatalog,
};
pub use events::{
    ActivateEvent, AnimationTextKeyEvent, AnimationTextKeyEvents, EquipmentChange,
    EquipmentEventBatch, HitEvent, ItemEventBatch, ItemTransfer, OnCellLoadEvent, OnDeathEvent, OnInitEvent,
    OnTriggerEnterEvent, RippleEvent, SplashEvent, TimerExpired,
};
pub use fragment::{
    add_items, remove_items, apply_effects, apply_spoken_info_fragment, fragment_activation_flush_system, fragment_idle_forms,
    fragment_continuation_system, populate_owned_info_fragments_from_pex_with_providers,
    populate_owned_quest_fragments_from_pex_detailed_with_providers,
    populate_owned_quest_fragments_from_script_with_providers,
    populate_owned_scene_fragments_from_pex_detailed_with_providers,
    populate_owned_scene_fragments_from_script_with_providers, populate_quest_fragments_from_pex,
    populate_quest_fragments_from_pex_detailed, populate_scene_fragments_from_pex,
    populate_scene_fragments_from_pex_detailed, quest_fragment_dispatch_system,
    scene_fragment_dispatch_system, DeferredFragmentEffects, DialogueInfoFragments,
    FragmentExecutionQueue, FragmentPexTranslation, LockOverride, OwnedFragmentProviders,
    PendingFragmentActivations, PlayerMoveRequest, QuestStageFragments, ReferenceActivationBlocks,
    ReferenceEnableState, default_activation_blocked, install_authored_disabled_references,
    reference_enabled, AuthoredDisabledReferences, install_quest_script_instances,
    QuestScriptInstances,
    ReferenceLockState, ReferenceScriptState, SceneFragments,
    attach_reference_script, check_reference_script, reference_script_event_system,
    PapyrusReferenceScripts, ReferenceScript, ReferenceScriptIndex, VmFrameBudget, VmFrameUse, DELIVERED_EVENTS,
    script_update_system, ScriptUpdates, UpdateClock, UpdateRegistration, UpdateRequest,
    EventRegistration, EventRegistrations, EventRequest, REMOTE_EVENTS, distance_event_system, DistanceRequest, DistanceTarget, DistanceWatch, DistanceWatches,
    hit_event_system, HitFilter, HitRegistration, HitRegistrations, HitRequest,
    raise_unload_events, ActorWatch, PendingUnloadDeliveries,
    alias_script_event_system, check_alias_script, install_quest_alias_scripts, is_alias_owner, AliasScript,
    AliasScriptOwners, QuestAliasScripts, ALIAS_OWNER_BASE, DELIVERED_ALIAS_EVENTS,
    package_fragment_system, populate_package_fragments, report_package_transition, PackageFragmentSet,
    PackageFragments, PackageTransition, PendingPackageTransitions,
};
pub use faction_changes::{
    reapply_scripted_faction_changes, FactionChange, ScriptedFactionChanges, PLAYER_REFERENCE,
};
pub use globals::Globals;
pub use alias_collections::{AliasCollections, CollectionAliasKinds};
pub use editor_location::EditorLocation;
pub use linked_refs::{linked_ref, LinkedRefOverrides};
pub use skills::{advance_skill, PendingSkillAdvances, SkillProgress, SkillScaling};
pub use followers::{FollowerFlag, FollowerFlags, FollowerState};
pub use misc_stats::{Achievements, MiscStats};
pub use actor_value_names::{install_actor_value_names, ActorValueNames};
pub use alias_overrides::{AliasOverride, AliasOverrides};
pub use created_references::{
    CreatedCell, CreatedDeletion, CreatedPlacement, CreatedReference, CreatedReferences,
    ScriptedIds, CREATED_FORM_ID_BASE, SCRIPTED_FORM_ID_BASE,
};
pub use load_order::LoadOrderIdentity;
pub use map_markers::{
    install_map_markers, map_marker_fast_travel, map_marker_visible, AuthoredMapMarkers,
    MapMarkers,
};
pub use location::{
    current_location, editor_location, install_location_table, location_cleared,
    note_actor_death, LocationAliasBindings, LocationClearState, LocationTable, PlayerLocation,
};
pub use magic::{
    add_race_spells, add_spell, remove_race_spells, remove_spell, CanonicalSpell,
    ConstantModifier, RaceSpells, SpellCatalog, SpellList,
};
pub use obscript::{
    decode_extender_calls, legacy_load_order_call, ObscriptArgument, ObscriptCall, ObscriptDecode,
    ObscriptDialect, ObscriptLoadOrderCallError,
};
pub use obscript_runtime::{
    attach_legacy_obscript_program, compile_legacy_obscript_bytecode_program,
    compile_legacy_obscript_program, legacy_obscript_load_order_system,
    set_extension_script_function_invoker, set_legacy_obscript_content_catalog,
    ExtensionScriptFunctionCallback, ExtensionScriptFunctionInvoker, LegacyObscriptAssignment,
    LegacyObscriptCall, LegacyObscriptContentCatalog, LegacyObscriptEvent, LegacyObscriptProgram,
    LegacyObscriptStatement,
};
pub use package::{
    install_package_linked_references, install_package_records, install_package_target_directions,
    install_package_target_positions, scene_package_system, ActiveScenePackageAction,
    EvaluatePackageRequest, PackageRegistry, PackageTargetRegistry, ScenePackageCommand,
    ScenePackageCompletionBatch, ScenePackageEvent, ScenePackageEventBatch, ScenePackagePlayback,
};
pub use papyrus_provider::{
    attach_owned_papyrus_provider_program, attach_papyrus_provider_program, lower_provider_call,
    lower_provider_program, papyrus_provider_system, queue_papyrus_mod_event,
    set_papyrus_provider_entity_resolver, set_papyrus_provider_form_resolver,
    set_papyrus_provider_mod_event_publisher, set_papyrus_provider_runtime, PapyrusModEventRuntime,
    PapyrusModEventSender, PapyrusProviderArgument, PapyrusProviderCallback,
    PapyrusProviderCatalog, PapyrusProviderCatalogError, PapyrusProviderCondition,
    PapyrusProviderContinuationQueue, PapyrusProviderEntityResolver, PapyrusProviderEvent,
    PapyrusProviderFormResolver, PapyrusProviderInvocation, PapyrusProviderLowerError,
    PapyrusProviderModEventPublisher, PapyrusProviderProgram, PapyrusProviderProgramError,
    PapyrusProviderRoute, PapyrusProviderRuntime, PapyrusProviderStatement,
    TypedPapyrusProviderCall,
};
pub use relationships::{
    install_relationships, relationship_rank, AuthoredRelationships, ScriptedRelationships,
};
pub use messages::{
    install_message_catalog, MessageCatalog, PendingScriptMessages, ScriptMessage,
};
pub use bleedout::{
    bleedout_system, install_bleedout_rules, is_bleeding_out, is_essential, is_protected,
    spare_from_death, ActorBaseProtectionOverrides, ActorProtection, Bleedout, BleedoutRules,
};
pub use player_control::{
    collect_ghosts, collect_unconscious, is_ghost, is_unconscious, update_actor_control, ActorControlState, PlayerControlSelection,
    PlayerControlState,
};
pub use quest_stages::{
    install_engine_start_quest, install_quest_alias_readiness_gate, install_start_game_quests,
    quest_alias_readiness_stage_system, quest_startup_system, quest_terminal_stage_system,
    resolve_quest_objective_targets, resolve_quest_targets, QuestAliasReadinessGate,
    QuestDefinitionRegistry, QuestEventRead, QuestEventSubscriberId, QuestFormId, QuestStatus,
    SequencedQuestStageAdvanced, StartGameQuestRegistry, QUEST_EVENT_RETENTION,
};
pub use quest_variables::{
    install_quest_script_variables, quest_variable, QuestScriptObjects, QuestScriptVariableDefaults,
    QuestScriptVariables, ScriptObject,
};
pub use recurring_update::{recurring_update_tick_system, OnUpdateEvent, RecurringUpdate};
pub use registry::{ScriptRegistry, ScriptSpawnFn};
pub use scene::{
    install_scene_quest_aliases, install_scene_records, mark_scene_actor_bindings_dirty,
    quest_alias_diagnostics, quest_alias_refresh_system, refresh_scene_actor_bindings,
    running_quests_binding_entity, running_quest_bound_entities,
    running_quest_bound_entities_cached, scene_playback_system, ActiveSceneAction,
    InventoryItemForms, QuestAliasDiagnostic, QuestAliasInjectedOverlays, QuestAliasInjectionState,
    QuestAliasResolutionState, QuestAliasRuntimeOverlays, RemoteSceneActorStub,
    RunningQuestBoundCache, SceneActionCompletionBatch, SceneActorBindings,
    SceneAliasCandidate, SceneEvent, SceneEventBatch, SceneFragmentInvocation,
    SceneFragmentInvocationBatch, ScenePlaybackState, ScenePlayer, SceneQuestAliasRegistry,
    SceneRegistry, SceneStartRequest, SceneStopRequest,
};
pub use timer::{timer_tick_system, ScriptTimer};
pub use translate::{
    translate_pex, translate_pex_detailed, translate_pex_detailed_with_providers, translate_script,
    CanonicalEvent, PexTranslation, RecognizeCtx, Recognized, ScriptSource,
};
pub use translate::effects::Effect;
pub use trigger::{
    base_form_advance_is_eligible, scene_phase_awaited_stage, trigger_detection_system,
    TriggerShape, TriggerVolume,
};
pub use vm_state::{
    two_state_activator_system, ScriptVariables, TwoStateActivator, TwoStateTransition,
    TwoStateTransitionBatch,
};

use byroredux_core::ecs::world::World;

/// Register all scripting component storages in the world.
///
/// Call during setup so that `query_mut()` works for event markers
/// before any entity has triggered an event.
pub fn register(world: &mut World) {
    world.insert_resource(CompatibilityRegistry::default());
    // M47.2 — the scripted faction-membership ledger (saved; a restore
    // replaces this default).
    if world.try_resource::<ScriptedFactionChanges>().is_none() {
        world.insert_resource(ScriptedFactionChanges::default());
    }
    // M47.2 — script relationship ranks (saved).
    if world.try_resource::<ScriptedRelationships>().is_none() {
        world.insert_resource(ScriptedRelationships::default());
    }
    // M47.2 — cleared locations (saved).
    if world.try_resource::<LocationClearState>().is_none() {
        world.insert_resource(LocationClearState::default());
    }
    // M47.2 — scripted AddToMap (saved).
    if world.try_resource::<MapMarkers>().is_none() {
        world.insert_resource(MapMarkers::default());
    }
    // M47.2 — scripted IncrementStat misc stats (saved).
    if world.try_resource::<MiscStats>().is_none() {
        world.insert_resource(MiscStats::default());
    }
    // M47.2 — alias scripts: the authored ones (derived) and their owner ids
    // and started quests (saved).
    if world.try_resource::<QuestAliasScripts>().is_none() {
        world.insert_resource(QuestAliasScripts::default());
    }
    if world.try_resource::<AliasScriptOwners>().is_none() {
        world.insert_resource(AliasScriptOwners::default());
    }
    // M47.2 — what the reference scripts last saw of the actors (transient).
    if world.try_resource::<ActorWatch>().is_none() {
        world.insert_resource(ActorWatch::default());
    }
    // M47.2 — unload events awaiting their frame (transient).
    if world.try_resource::<PendingUnloadDeliveries>().is_none() {
        world.insert_resource(PendingUnloadDeliveries::default());
    }
    // M47.2 — Papyrus hit event registrations (saved).
    if world.try_resource::<HitRegistrations>().is_none() {
        world.insert_resource(HitRegistrations::default());
    }
    // M47.2 — scripted SetLinkedRef links (saved).
    if world.try_resource::<LinkedRefOverrides>().is_none() {
        world.insert_resource(LinkedRefOverrides::default());
    }
    // M47.2 — Game.AdvanceSkill: banked usage XP (saved) and the advances
    // scripts queued (transient).
    if world.try_resource::<SkillProgress>().is_none() {
        world.insert_resource(SkillProgress::default());
    }
    if world.try_resource::<PendingSkillAdvances>().is_none() {
        world.insert_resource(PendingSkillAdvances::default());
    }
    // M47.2 — script-filled reference-collection alias members (saved).
    if world.try_resource::<AliasCollections>().is_none() {
        world.insert_resource(AliasCollections::default());
    }
    // M47.2 — scripted SetPlayerTeammate / SetDoingFavor flags (saved).
    if world.try_resource::<FollowerState>().is_none() {
        world.insert_resource(FollowerState::default());
    }
    // M47.2 — scripted AddAchievement awards (saved).
    if world.try_resource::<Achievements>().is_none() {
        world.insert_resource(Achievements::default());
    }
    // M47.2 — scripted ForceRefTo / Clear alias overrides (saved).
    if world.try_resource::<AliasOverrides>().is_none() {
        world.insert_resource(AliasOverrides::default());
    }
    // M47.2 — created-object references (saved, like the ledger above).
    if world.try_resource::<CreatedReferences>().is_none() {
        world.insert_resource(CreatedReferences::default());
    }
    // M47.2 — quest location-alias bindings (saved, like the ledger above).
    if world.try_resource::<LocationAliasBindings>().is_none() {
        world.insert_resource(LocationAliasBindings::default());
    }
    // M47.2 — the Papyrus interpreter's per-frame allowance (#5763); transient,
    // renewed by `event_cleanup_system` at the end of every frame.
    if world.try_resource::<VmFrameBudget>().is_none() {
        world.insert_resource(VmFrameBudget::default());
    }
    world.register::<ActivateEvent>();
    world.register::<HitEvent>();
    world.register::<SplashEvent>();
    world.register::<RippleEvent>();
    world.register::<TimerExpired>();
    world.register::<AnimationTextKeyEvents>();
    world.register::<ScriptTimer>();
    // #4415 — actor spell lists (stamped at spawn, mutated by AddSpell /
    // RemoveSpell effects through `query_mut`, so pre-registered).
    world.register::<SpellList>();
    // #4415 — the racial SPLO set `AddRaceSpells`/`RemoveRaceSpells` use.
    world.register::<RaceSpells>();
    // Canonical event markers. OnInitEvent + OnCellLoadEvent +
    // OnTriggerEnterEvent + EquipmentEventBatch join the existing
    // ActivateEvent / HitEvent / TimerExpired in the script-event
    // catalog. Emit sites land per-phase:
    //   * OnCellLoadEvent — emitted by the cell loader's
    //     `attach_script_for_refr` (Phase 5).
    //   * OnTriggerEnterEvent — emitted by `trigger_detection_system`
    //     on player entry (M47.2).
    //   * EquipmentEventBatch — emitted by player and scripted equip paths.
    // All four are one-frame transients drained by `event_cleanup_system`.
    world.register::<OnCellLoadEvent>();
    world.register::<OnInitEvent>();
    world.register::<OnTriggerEnterEvent>();
    world.register::<OnDeathEvent>();
    world.register::<EquipmentEventBatch>();
    // P3 — item add/remove channel (future `OnItemAdded`/`OnItemRemoved`).
    // Emitted by loot transfers (whole/selective) and world pickups; drained
    // by `event_cleanup_system` like the equipment batch above. No reader
    // yet (#4713).
    world.register::<ItemEventBatch>();
    // M47.2 — trigger-volume storage. The cell loader attaches a
    // `TriggerVolume` to each invisible trigger REFR; `trigger_detection_system`
    // emits `OnTriggerEnterEvent` on player entry, which the quest-advance
    // dispatch consumes (the `default*Trigger` family).
    trigger::register(world);
    combat::register(world);
    obscript_runtime::register(world);
    papyrus_provider::register(world);
    recurring_update::register(world);
    quest_stages::register(world);
    // #5366 — Story Manager dispatch (transient StoryEvent markers).
    story_manager::register(world);
    // M47.2 (b2) — quest-stage fragment dispatch. Registers the
    // QuestStageAdvanced component storage (via quest_advance below it's
    // also registered, idempotent) and the QuestStageFragments /
    // QuestObjectiveState resources the dispatcher reads.
    fragment::register(world);
    // Skyrim+ SCEN orchestration: immutable definitions live in a resource,
    // while one ScenePlayer component per record owns mutable playback state.
    scene::register(world);
    // DIAL/INFO selection and persistent subtitle state consume the scene
    // runtime's dialogue starts and return canonical action completions.
    dialogue::register(world);
    // Skyrim+ PACK template resolution and scene-owned package execution.
    package::register(world);
    equipment::register(world);
    player_control::register(world);
    bleedout::register(world);
    keywords::register(world);
    menus::register(world);
    messages::register(world);
    cinematic::register(world);
    vm_state::register(world);
    // M47.0 Phase 1 — register the R5 prototype storages so
    // `papyrus_demo` scripts can attach their state components when
    // their owning REFR spawns. Without this call, `query_mut::<…>`
    // returns None for every script-state component and the demo
    // systems no-op. The cell-loader-driven attach lands in Phase 3;
    // this Phase 1 step is the minimum plumbing that makes the demo
    // surface live at runtime. See docs/engine/m47-0-design.md.
    papyrus_demo::register(world);
    log::info!("Scripting subsystem initialized (ECS events + timers + papyrus_demo)");
}
