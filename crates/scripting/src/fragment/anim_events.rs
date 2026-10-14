//! M47.2 — `RegisterForAnimationEvent` / `UnregisterForAnimationEvent` and
//! the `OnAnimationEvent(akSource, asEventName)` they deliver.
//!
//! The CK pages: a script (a form, alias or active magic effect) registers for
//! a named animation event of a reference; "the event will only go to the
//! specific effect, alias, or form that registered and will not be relayed to
//! attached aliases or effects"; "aliases and quests will automatically
//! unregister for this event when the quest stops"; the register call returns
//! true "if it was successfully able to register for the event. If false,
//! something went wrong (the animation graph may not be loaded quite yet)".
//! "The game will allow you to register ANY animation event, valid or invalid",
//! so a registration is never checked against the graph's event names.
//!
//! The animation events that exist here are the ones a placed object's
//! behavior graph raises ([`crate::animation_graph`]): the event a script's
//! accepted `PlayAnimation` sends, and the events a state's clip triggers
//! fire as it plays. [`animation_event_system`] drains each graph's raised
//! events and queues the `OnAnimationEvent` of every registration for that
//! reference and event, as a separate run, like a custom event's receivers.
//! Not modelled: `OnAnimationEventUnregistered` (a graph unloading drops the
//! registration silently), actors' graphs (none runs), and the sound events
//! (`SoundPlay.*`) an animation notifies.

use super::*;
use crate::translate::effects::VmStored;

/// One script's registration for an animation event of a reference.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct AnimationEventRegistration {
    /// The object the registering script is attached to.
    pub owner: u32,
    pub script: String,
    /// The reference whose graph raises the event.
    pub sender: u32,
    /// The event as the script wrote it.
    pub event: String,
}

/// A registration change a script run asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationEventRequest {
    Register { sender: u32, event: String },
    Unregister { sender: u32, event: String },
}

/// Every script's animation event registrations. Saved.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct AnimationEventRegistrations {
    entries: Vec<AnimationEventRegistration>,
}

impl Resource for AnimationEventRegistrations {}

impl AnimationEventRegistrations {
    pub fn entries(&self) -> &[AnimationEventRegistration] {
        &self.entries
    }

    /// Apply `request` from `owner`'s instance of `script`.
    pub fn request(&mut self, owner: u32, script: &str, request: &AnimationEventRequest) {
        let same = |e: &AnimationEventRegistration, sender: u32, event: &str| {
            e.owner == owner
                && e.script.eq_ignore_ascii_case(script)
                && e.sender == sender
                && e.event.eq_ignore_ascii_case(event)
        };
        match request {
            AnimationEventRequest::Register { sender, event } => {
                if !self.entries.iter().any(|e| same(e, *sender, event)) {
                    self.entries.push(AnimationEventRegistration {
                        owner,
                        script: script.to_owned(),
                        sender: *sender,
                        event: event.clone(),
                    });
                }
            }
            AnimationEventRequest::Unregister { sender, event } => self.entries.retain(|e| !same(e, *sender, event)),
        }
    }

    /// Drop every registration the owners in `stopped` hold.
    pub(super) fn drop_owners(&mut self, stopped: &[u32]) {
        self.entries.retain(|e| !stopped.contains(&e.owner));
    }
}

/// Deliver the events the object graphs raised this frame to the scripts
/// registered for them.
pub fn animation_event_system(world: &World, _dt: f32) {
    // Always drain, so a graph nobody listens to does not accumulate events.
    let raised: Vec<(EntityId, Vec<String>)> = {
        let Some(mut graphs) = world.query_mut::<crate::ObjectAnimationGraph>() else {
            return;
        };
        graphs
            .iter_mut()
            .filter_map(|(entity, graph)| {
                let events = graph.take_raised();
                (!events.is_empty()).then_some((entity, events))
            })
            .collect()
    };
    let registered: Vec<AnimationEventRegistration> = match world.try_resource::<AnimationEventRegistrations>() {
        Some(registrations) if !registrations.entries.is_empty() => registrations.entries.clone(),
        _ => return,
    };
    let owners = super::owners::ScriptOwners::new(world);
    for (entity, events) in raised {
        let Some(sender) = crate::condition::entity_global_form_id(world, entity) else {
            continue;
        };
        for event in events {
            let receivers: Vec<EventRegistration> = registered
                .iter()
                .filter(|r| r.sender == sender && r.event.eq_ignore_ascii_case(&event))
                .filter(|r| {
                    // A quest's (or alias's) registration delivers while the
                    // quest runs; a reference's while it is loaded.
                    let standing = {
                        let stages = world.resource::<QuestStageState>();
                        owners.standing(world, &stages, r.owner)
                    };
                    match standing {
                        Some(standing) => standing,
                        None => ReferenceScriptIndex::lookup(world, r.owner).is_some(),
                    }
                })
                .map(|r| EventRegistration {
                    owner: r.owner,
                    script: r.script.clone(),
                    sender: super::DistanceTarget::Reference(sender),
                    event: "OnAnimationEvent".to_owned(),
                    remote: false,
                })
                .collect();
            events::enqueue(world, &receivers, sender, &[VmStored::Str(event)]);
        }
    }
}

