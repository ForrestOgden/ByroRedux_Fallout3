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
