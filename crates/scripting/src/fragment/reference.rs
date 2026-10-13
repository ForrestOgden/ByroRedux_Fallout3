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
//! `OnTriggerEnter(akActionRef)`. Each runs as an `Effect::Interpret` owned
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
/// delivers the first three.
pub const DELIVERED_EVENTS: [&str; 5] = ["OnInit", "OnLoad", "OnCellLoad", "OnActivate", "OnTriggerEnter"];

pub(super) fn is_delivered_event(name: &str) -> bool {
    DELIVERED_EVENTS.iter().any(|event| event.eq_ignore_ascii_case(name))
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
    let mut attached = world.query_mut::<PapyrusReferenceScripts>().expect("registered above");
    match attached.get_mut(entity) {
        Some(scripts) => scripts.0.push(script),
        None => {
            attached.insert(entity, PapyrusReferenceScripts(vec![script]));
        }
    }
}

/// Where whether `script` has run its `OnInit` on a reference is kept, among
/// the reference's script variables.
fn initialized_key(script: &str) -> ConditionStringId {
    ConditionStringId::from_text(&format!("::init::{}", script.to_ascii_lowercase()))
}

/// One event delivery.
struct Delivery {
    owner: u32,
    script: ReferenceScript,
    event: &'static str,
    args: Vec<VmStored>,
}

/// Deliver this frame's events to the reference scripts the VM runs.
pub fn reference_script_event_system(world: &World, _dt: f32) {
    let deliveries = collect_deliveries(world);
    if deliveries.is_empty() {
        return;
    }
    let mut advances = Vec::new();
    for delivery in deliveries {
        let owner = QuestFormId(delivery.owner);
        if delivery.event == "OnInit" {
            let key = initialized_key(&delivery.script.script);
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
            delivery.script.script,
            delivery.event
        );
        let effect = Effect::Interpret {
            script: delivery.script.script.clone(),
            function: delivery.event.to_owned(),
            args: delivery.args,
            resume: None,
        };
        advances.extend(apply_fragment_guard_free(
            world,
            std::slice::from_ref(&effect),
            owner,
            Some(&delivery.script.vmad),
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
    let mut deliveries = Vec::new();
    for (entity, scripts) in attached.iter() {
        let loaded = cell_loads.as_ref().is_some_and(|q| q.get(entity).is_some());
        let activator = activations.as_ref().and_then(|q| q.get(entity)).map(|event| event.activator);
        let triggerers: Vec<EntityId> = triggers
            .as_ref()
            .and_then(|q| q.get(entity))
            .map(|event| event.triggerers.clone())
            .unwrap_or_default();
        if !loaded && activator.is_none() && triggerers.is_empty() {
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
                        script: script.clone(),
                        event,
                        args,
                    });
                }
            };
            if loaded {
                for event in ["OnInit", "OnLoad", "OnCellLoad"] {
                    deliver(event, Vec::new());
                }
            }
            if let Some(activator) = activator {
                deliver("OnActivate", vec![reference(activator)]);
            }
            for &triggerer in &triggerers {
                deliver("OnTriggerEnter", vec![reference(triggerer)]);
            }
        }
    }
    deliveries
}
