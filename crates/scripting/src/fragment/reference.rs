//! M47.2 — Papyrus reference scripts on the VM: a script attached to a placed
//! reference (its own `VMAD` or its base record's) that no recognizer models
//! runs its events through [`super::vm`].
//!
//! The cell loader attaches such a script when [`check_reference_script`]
//! accepts every handler it has of the [`DELIVERED_EVENTS`]; the
//! [`PapyrusReferenceScripts`] component names it with its properties. The
//! script's own state — its variables and its Papyrus state — lives in the
//! saved script-variable stores under the reference's FormID, so it outlives
//! the cell unload that despawns the entity, as the component, re-derived at
//! the next attach, does not need to.
//!
//! [`reference_script_event_system`] turns the engine's event markers into
//! Papyrus events (CK *OnInit*, *OnLoad - ObjectReference*, *OnCellLoad -
//! ObjectReference*): a reference's cell load delivers `OnInit` the first
//! time the reference loads, then `OnLoad` and `OnCellLoad` every time;
//! `ActivateEvent` delivers `OnActivate(akActionRef)`; a trigger crossing
//! `OnTriggerEnter(akActionRef)`; a `HitEvent` `OnHit`; an `OnDeathEvent`
//! `OnDeath(akKiller)`. Each runs as an `Effect::Interpret` owned
//! by the reference, through the fragment runtime — so its natives, its
//! waits and its quest-stage advances behave as a quest fragment's do.
//!
//! Not modelled: `OnInit` holding back the script's other events while it
//! waits, and the variable reset a cell reset brings.

use super::*;
use crate::translate::effects::VmStored;
use byroredux_core::ecs::sparse_set::SparseSetStorage;
use byroredux_core::ecs::storage::Component;
use byroredux_plugin::esm::records::condition::ConditionStringId;
use byroredux_plugin::esm::records::script_instance::ScriptInstance;

/// The events reference scripts receive, in the order one cell load
/// delivers the first four (CK *OnCellAttach*: "reliably called before
/// OnCellLoad"; attach and load are one moment here).
pub const DELIVERED_EVENTS: [&str; 8] = [
    "OnInit",
    "OnCellAttach",
    "OnLoad",
    "OnCellLoad",
    "OnActivate",
    "OnTriggerEnter",
    "OnHit",
    "OnDeath",
];

/// An event the runtime delivers — to a reference, or an update to the
/// script that registered for it. One the script does not handle in its
/// current state does nothing.
pub(super) fn is_delivered_event(name: &str) -> bool {
    DELIVERED_EVENTS
        .iter()
        .chain(&["OnUpdate", "OnUpdateGameTime", "OnTimer", "OnTimerGameTime"])
        .any(|event| event.eq_ignore_ascii_case(name))
}

pub use super::vm::check_reference_script;

/// One Papyrus script the VM runs on a reference.
#[derive(Debug, Clone)]
pub struct ReferenceScript {
    /// The script's name, as the VM's script store keys it.
    pub script: String,
    /// The script's properties: its own `VMAD` entry.
    pub vmad: Arc<ScriptInstanceData>,
    /// The [`DELIVERED_EVENTS`] it handles.
    pub events: Vec<String>,
}

impl ReferenceScript {
    fn handles(&self, event: &str) -> bool {
        self.events.iter().any(|handled| handled.eq_ignore_ascii_case(event))
    }
}

/// M47.2 — the Papyrus scripts the VM runs on this reference.
#[derive(Debug, Clone, Default)]
pub struct PapyrusReferenceScripts(pub Vec<ReferenceScript>);

impl Component for PapyrusReferenceScripts {
    type Storage = SparseSetStorage<Self>;
}

/// FormID → entity for the references that carry [`PapyrusReferenceScripts`]
/// (#5763). `script_update_system` and the VM's cross-reference calls used to
/// find a reference by scanning every attached entity (and `entity_of` scans
/// every FormID-tagged one) each time; this answers in one hash lookup.
///
/// Maintained on attach: [`attach_reference_script`] marks the set changed, and
/// the next lookup rebuilds the map once from the attached components — the
/// FormID is read then, so a reference whose `FormIdComponent` lands after its
/// scripts is still found. Unloading is not tracked: a cell unload despawns
/// the entity, and [`ReferenceScriptIndex::lookup`] re-checks the entity it
/// finds, so a despawned or reused one reads as not loaded. A reload attaches
/// again and the next lookup picks it up. Transient: never saved.
#[derive(Debug, Default)]
pub struct ReferenceScriptIndex {
    by_form: HashMap<u32, EntityId>,
    /// Bumped by every attach; the map is current when `built == attached`.
    attached: u64,
    built: u64,
}

impl Resource for ReferenceScriptIndex {}

impl ReferenceScriptIndex {
    fn note_attached(&mut self) {
        self.attached += 1;
    }

    /// Rebuild the map when an attach has happened since it was built.
    fn refresh(world: &World) {
        let stale = world
            .try_resource::<ReferenceScriptIndex>()
            .is_some_and(|index| index.built != index.attached);
        if !stale {
            return;
        }
        let mut by_form = HashMap::new();
        if let Some(attached) = world.query::<PapyrusReferenceScripts>() {
            for (entity, _) in attached.iter() {
                if let Some(form) = crate::condition::entity_global_form_id(world, entity) {
                    by_form.insert(form, entity);
                }
            }
        }
        if let Some(mut index) = world.try_resource_mut::<ReferenceScriptIndex>() {
            index.by_form = by_form;
            index.built = index.attached;
        }
    }

    /// The loaded reference with FormID `form` that carries VM scripts;
    /// `None` when there is none (never attached, or its cell unloaded).
    pub fn lookup(world: &World, form: u32) -> Option<EntityId> {
        Self::refresh(world);
        let entity = world.try_resource::<ReferenceScriptIndex>()?.by_form.get(&form).copied()?;
        // Re-check what the map says: the entity may have been despawned (its
        // cell unloaded) or its id reused since the map was built.
        world.get::<PapyrusReferenceScripts>(entity)?;
        (crate::condition::entity_global_form_id(world, entity) == Some(form)).then_some(entity)
    }
}

/// Attach `instance` — a script [`check_reference_script`] accepted, handling
/// `events`, with the `scripts` its runs need — to `entity`.
pub fn attach_reference_script(
    world: &mut World,
    entity: EntityId,
    instance: &ScriptInstance,
    events: Vec<String>,
    scripts: Vec<Arc<byroredux_papyrus::ast::Script>>,
) {
    {
        let mut fragments = world.resource_mut::<QuestStageFragments>();
        for script in scripts {
            fragments.insert_vm_script(script);
        }
    }
    // The stores the script's state lives in.
    if world.try_resource::<crate::QuestScriptVariables>().is_none() {
        world.insert_resource(crate::QuestScriptVariables::default());
    }
    if world.try_resource::<crate::QuestScriptObjects>().is_none() {
        world.insert_resource(crate::QuestScriptObjects::default());
    }
    world.register::<PapyrusReferenceScripts>();
    let script = ReferenceScript {
        script: instance.name.clone(),
        vmad: Arc::new(ScriptInstanceData {
            version: 5,
            object_format: 2,
            scripts: vec![instance.clone()],
        }),
        events,
    };
    {
        let mut attached = world.query_mut::<PapyrusReferenceScripts>().expect("registered above");
        match attached.get_mut(entity) {
            Some(scripts) => scripts.0.push(script),
            None => {
                attached.insert(entity, PapyrusReferenceScripts(vec![script]));
            }
        }
    }
    if world.try_resource::<ReferenceScriptIndex>().is_none() {
        world.insert_resource(ReferenceScriptIndex::default());
    }
    world.resource_mut::<ReferenceScriptIndex>().note_attached();
}

/// Where whether `script` has run its `OnInit` on a reference is kept, among
/// the reference's script variables.
fn initialized_key(script: &str) -> ConditionStringId {
    ConditionStringId::from_text(&format!("::init::{}", script.to_ascii_lowercase()))
}

/// One event delivery.
struct Delivery {
    owner: u32,
    /// The script's name and properties; its handled-event list stays with the
    /// component (cloning it per delivery was most of a burst's cost).
    script: String,
    vmad: Arc<ScriptInstanceData>,
    event: &'static str,
    args: Vec<VmStored>,
}

/// Deliver this frame's events to the reference scripts the VM runs.
pub fn reference_script_event_system(world: &World, _dt: f32) {
    super::events::prune_stopped_quests(world);
    relay_engine_events(world);
    let deliveries = collect_deliveries(world);
    if deliveries.is_empty() {
        return;
    }
    let mut advances = Vec::new();
    for delivery in deliveries {
        let owner = QuestFormId(delivery.owner);
        if delivery.event == "OnInit" {
            let key = initialized_key(&delivery.script);
            let Some(mut variables) = world.try_resource_mut::<crate::QuestScriptVariables>() else {
                continue;
            };
            if variables.get(owner, key).is_some() {
                continue;
            }
            variables.set(owner, key, 1.0);
        }
        log::debug!(
            "reference {:08X}: {}.{}",
            delivery.owner,
            delivery.script,
            delivery.event
        );
        let effect = Effect::Interpret {
            script: delivery.script.clone(),
            function: delivery.event.to_owned(),
            args: delivery.args,
            resume: None,
        };
        // The interpreter's frame budget is spent: the rest of this frame's
        // deliveries queue for the next one, in order, unrun.
        let effect = if super::budget::exhausted(world) {
            match super::budget::defer(world, owner, Some(&delivery.vmad), vec![effect]) {
                Ok(()) => continue,
                Err(mut effects) => effects.remove(0),
            }
        } else {
            effect
        };
        advances.extend(apply_fragment_guard_free(
            world,
            std::slice::from_ref(&effect),
            owner,
            Some(&delivery.vmad),
        ));
    }
    if advances.is_empty() {
        return;
    }
    let Some(player) = world
        .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
        .map(|player| player.0)
    else {
        return;
    };
    crate::quest_stages::push_quest_stage_advances(world, player, advances);
}

/// The deliveries this frame's markers make, snapshotted before any runs:
/// a run takes the quest guards and may write the marker storages.
fn collect_deliveries(world: &World) -> Vec<Delivery> {
    // Pattern B: this system owns `OnDeathEvent`; drain it first, whatever
    // follows.
    let deaths: HashMap<EntityId, Option<EntityId>> = world
        .query::<crate::OnDeathEvent>()
        .map(|q| q.iter().map(|(entity, event)| (entity, event.killer)).collect())
        .unwrap_or_default();
    crate::scene::playback::drain::<crate::OnDeathEvent>(world);
    let Some(attached) = world.query::<PapyrusReferenceScripts>() else {
        return Vec::new();
    };
    let player = world
        .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
        .map(|player| player.0);
    let form_of = |entity: EntityId| -> Option<u32> {
        if Some(entity) == player {
            return Some(0x14);
        }
        crate::condition::entity_global_form_id(world, entity)
    };
    let cell_loads = world.query::<crate::OnCellLoadEvent>();
    let activations = world.query::<crate::ActivateEvent>();
    let triggers = world.query::<crate::OnTriggerEnterEvent>();
    let hits = world.query::<crate::HitEvent>();
    let weapons = world.query::<byroredux_core::ecs::components::EquippedWeapon>();
    let mut deliveries = Vec::new();
    for (entity, scripts) in attached.iter() {
        let loaded = cell_loads.as_ref().is_some_and(|q| q.get(entity).is_some());
        let activator = activations.as_ref().and_then(|q| q.get(entity)).map(|event| event.activator);
        let triggerers: Vec<EntityId> = triggers
            .as_ref()
            .and_then(|q| q.get(entity))
            .map(|event| event.triggerers.clone())
            .unwrap_or_default();
        let death = deaths.get(&entity).copied();
        let hit = hits.as_ref().and_then(|q| q.get(entity)).copied();
        if !loaded && activator.is_none() && triggerers.is_empty() && death.is_none() && hit.is_none() {
            continue;
        }
        let Some(owner) = form_of(entity) else {
            log::debug!("reference script events on {entity:?} skipped: the reference has no FormID");
            continue;
        };
        let reference = |entity: EntityId| VmStored::Form(form_of(entity).unwrap_or(0));
        for script in &scripts.0 {
            let mut deliver = |event: &'static str, args: Vec<VmStored>| {
                if script.handles(event) {
                    deliveries.push(Delivery {
                        owner,
                        script: script.script.clone(),
                        vmad: Arc::clone(&script.vmad),
                        event,
                        args,
                    });
                }
            };
            if loaded {
                for event in ["OnInit", "OnCellAttach", "OnLoad", "OnCellLoad"] {
                    deliver(event, Vec::new());
                }
            }
            if let Some(activator) = activator {
                deliver("OnActivate", vec![reference(activator)]);
            }
            for &triggerer in &triggerers {
                deliver("OnTriggerEnter", vec![reference(triggerer)]);
            }
            // `OnHit(akAggressor, akSource, akProjectile, abPowerAttack,
            // abSneakAttack, abBashAttack, abHitBlocked)`: the source is the
            // aggressor's equipped weapon (None unarmed — no weapon form is
            // invented); projectiles are not modelled.
            if let Some(hit) = hit {
                let source = weapons
                    .as_ref()
                    .and_then(|q| q.get(hit.aggressor))
                    .map_or(VmStored::None, |weapon| VmStored::Form(weapon.base_form_id));
                deliver(
                    "OnHit",
                    vec![
                        reference(hit.aggressor),
                        source,
                        VmStored::None,
                        VmStored::Bool(hit.power_attack),
                        VmStored::Bool(hit.sneak_attack),
                        VmStored::Bool(hit.bash_attack),
                        VmStored::Bool(hit.blocked),
                    ],
                );
            }
            if let Some(killer) = death {
                let killer = killer.map_or(VmStored::None, reference);
                deliver("OnDeath", vec![killer]);
            }
        }
    }
    deliveries
}

/// Relay this frame's engine events to the scripts registered for them
/// remotely (`RegisterForRemoteEvent`): the event a reference received, handled
/// by the registrant with the sender first and the event's own arguments after.
/// Read without draining: the consumers below and end-of-frame cleanup own the
/// markers.
fn relay_engine_events(world: &World) {
    let any = world
        .try_resource::<super::EventRegistrations>()
        .is_some_and(|registrations| registrations.entries().iter().any(|e| e.remote));
    if !any {
        return;
    }
    let player = world
        .try_resource::<crate::papyrus_demo::PapyrusPlayerEntity>()
        .map(|player| player.0);
    let form_of = |entity: EntityId| -> Option<u32> {
        if Some(entity) == player {
            return Some(0x14);
        }
        crate::condition::entity_global_form_id(world, entity)
    };
    let reference = |entity: EntityId| VmStored::Form(form_of(entity).unwrap_or(0));
    let mut relays: Vec<(u32, &'static str, Vec<VmStored>)> = Vec::new();
    if let Some(events) = world.query::<crate::ActivateEvent>() {
        for (entity, event) in events.iter() {
            if let Some(form) = form_of(entity) {
                relays.push((form, "OnActivate", vec![reference(event.activator)]));
            }
        }
    }
    if let Some(events) = world.query::<crate::OnTriggerEnterEvent>() {
        for (entity, event) in events.iter() {
            if let Some(form) = form_of(entity) {
                for &triggerer in &event.triggerers {
                    relays.push((form, "OnTriggerEnter", vec![reference(triggerer)]));
                }
            }
        }
    }
    if let Some(events) = world.query::<crate::OnDeathEvent>() {
        for (entity, event) in events.iter() {
            if let Some(form) = form_of(entity) {
                relays.push((form, "OnDeath", vec![event.killer.map_or(VmStored::None, reference)]));
            }
        }
    }
    if let Some(events) = world.query::<crate::HitEvent>() {
        for (entity, hit) in events.iter() {
            if let Some(form) = form_of(entity) {
                relays.push((
                    form,
                    "OnHit",
                    vec![
                        reference(hit.aggressor),
                        VmStored::None,
                        VmStored::None,
                        VmStored::Bool(hit.power_attack),
                        VmStored::Bool(hit.sneak_attack),
                        VmStored::Bool(hit.bash_attack),
                        VmStored::Bool(hit.blocked),
                    ],
                ));
            }
        }
    }
    for (sender, event, args) in relays {
        super::events::relay_remote(world, sender, event, &args);
    }
}
