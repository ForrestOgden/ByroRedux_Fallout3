//! Reference animation graphs (M47.2) — what the animation-graph natives answer
//! while the engine runs no Havok behavior graph for a reference.
//!
//! `ObjectReference.PlayAnimation(event)` "sends the named event to the object's
//! animation graph" (CK *PlayAnimation*), `PlayAnimationAndWait` does that and
//! waits for a named event back, `RegisterForAnimationEvent` subscribes to events
//! the graph raises, and `SetAnimationVariable*` writes a graph variable. Skyrim
//! and Fallout 4 drive levers, traps, doors and rigged props this way, from
//! `.hkx` behavior graphs the engine does not execute: the `hkx` crate decodes
//! skeletons and clips only ("without loading or executing a behavior graph").
//!
//! The CK documents what these calls return when no graph answers, and that is
//! what a reference gets here:
//!
//! - `PlayAnimation` returns "true if the animation successfully started
//!   playing" — false;
//! - `PlayAnimationAndWait` "returns false" when the animation fails to play or
//!   "unloads before the end event is received", so it does not wait;
//! - `RegisterForAnimationEvent` returns false when it "was not successfully
//!   able to register for the event (the animation graph may not be loaded
//!   quite yet)";
//! - `UnregisterForAnimationEvent` and `SetAnimationVariable*` return nothing
//!   and have no graph to act on.
//!
//! The scripts that call them therefore run their own logic (state, stages,
//! enable / disable) with the animation absent, which is what the engine shows
//! for these references today. When a behavior-graph runtime lands, its
//! per-reference answer replaces [`graph_loaded`].

/// Whether the reference has an animation graph loaded to take events.
pub fn graph_loaded() -> bool {
    false
}

/// `Actor.ChangeAnimArchetype(keyword)` (Fallout 4; CK: "adds the specified
/// AnimArchetype keyword to this actor", and "removes any existing AnimArchetype
/// keyword from the actor (sets actor back to 'Neutral')" when called with none).
/// The archetype picks which animation set the actor's graph runs, so it is
/// recorded per actor reference for the graph runtime to read; the authored
/// archetype is the base `NPC_`'s keyword. Saved.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct AnimArchetypeOverrides {
    set: std::collections::BTreeMap<u32, u32>,
}

impl byroredux_core::ecs::resource::Resource for AnimArchetypeOverrides {}

impl AnimArchetypeOverrides {
    /// Set `actor`'s archetype keyword; `0` is "back to Neutral".
    pub fn set(&mut self, actor: u32, keyword: u32) {
        self.set.insert(actor, keyword);
    }

    /// The archetype a script gave `actor`: `Some(0)` after a clear, `None`
    /// when no script changed it.
    pub fn get(&self, actor: u32) -> Option<u32> {
        self.set.get(&actor).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_archetype_overrides_and_zero_is_neutral() {
        let mut overrides = AnimArchetypeOverrides::default();
        assert_eq!(overrides.get(7), None);
        overrides.set(7, 0x22E49);
        assert_eq!(overrides.get(7), Some(0x22E49));
        overrides.set(7, 0);
        assert_eq!(overrides.get(7), Some(0));
    }
}
