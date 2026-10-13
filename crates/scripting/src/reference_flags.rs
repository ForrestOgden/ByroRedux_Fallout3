//! Per-reference flags scripts set that other systems will read (M47.2).
//!
//! - `SetNoFavorAllowed(abNoFavor)` — CK: "Controls whether the player can ask a
//!   follower to operate this object. This function can be used to prevent a
//!   follower from activating an activator, looting a container, or picking the
//!   lock on a door."
//! - `SetActorCause(akActor)` — CK: "Sets the actor as the actor cause of this
//!   object" (who is responsible for what the object does; `None` clears it).
//!
//! Neither has a consumer yet — followers do no favors and objects do no damage
//! on their own here — so this is the state those systems will read, keyed by
//! the reference's FormID so it outlives the cell that despawns the entity.
//! Saved.

use std::collections::{HashMap, HashSet};

use byroredux_core::ecs::resource::Resource;

/// The flags scripts set on references.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct ReferenceFlags {
    no_favor: HashSet<u32>,
    actor_cause: HashMap<u32, u32>,
}

impl Resource for ReferenceFlags {}

/// A change a script run asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceFlagChange {
    NoFavor { reference: u32, no_favor: bool },
    ActorCause { reference: u32, actor: Option<u32> },
}

impl ReferenceFlags {
    pub fn apply(&mut self, change: ReferenceFlagChange) {
        match change {
            ReferenceFlagChange::NoFavor { reference, no_favor } => {
                if no_favor {
                    self.no_favor.insert(reference);
                } else {
                    self.no_favor.remove(&reference);
                }
            }
            ReferenceFlagChange::ActorCause { reference, actor } => match actor {
                Some(actor) => {
                    self.actor_cause.insert(reference, actor);
                }
                None => {
                    self.actor_cause.remove(&reference);
                }
            },
        }
    }

    /// Whether a script forbade favors on the reference.
    pub fn no_favor_allowed(&self, reference: u32) -> bool {
        self.no_favor.contains(&reference)
    }

    /// The actor a script made responsible for the reference.
    pub fn actor_cause(&self, reference: u32) -> Option<u32> {
        self.actor_cause.get(&reference).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_set_and_clear_per_reference() {
        let mut flags = ReferenceFlags::default();
        flags.apply(ReferenceFlagChange::NoFavor { reference: 1, no_favor: true });
        flags.apply(ReferenceFlagChange::ActorCause { reference: 1, actor: Some(9) });
        assert!(flags.no_favor_allowed(1) && !flags.no_favor_allowed(2));
        assert_eq!(flags.actor_cause(1), Some(9));
        flags.apply(ReferenceFlagChange::NoFavor { reference: 1, no_favor: false });
        flags.apply(ReferenceFlagChange::ActorCause { reference: 1, actor: None });
        assert!(!flags.no_favor_allowed(1));
        assert_eq!(flags.actor_cause(1), None);
    }
}
