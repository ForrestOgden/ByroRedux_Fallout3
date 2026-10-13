//! M47.2 — links scripts set: `ObjectReference.SetLinkedRef(apRef,
//! apKeyword = None)` ("Sets the reference linked to this one under the
//! specified keyword"), read back by `GetLinkedRef`.
//!
//! A reference's authored links are its `XLKR` pairs
//! ([`crate::scene::SceneAliasCandidate::linked_refs`]); a script's link
//! overrides the authored one under the same keyword, and a `None` target
//! clears it. Keyed by FormID so it outlives the cell that despawns the
//! entity. Saved.

use std::collections::HashMap;

use byroredux_core::ecs::resource::Resource;
use byroredux_core::ecs::world::World;

/// The authored encoding of the unkeyworded link (`XLKR` keyword 0).
const NO_KEYWORD: u32 = 0xFFFF_FFFF;

/// Every link a script set, by (source reference, keyword). The keyword of the
/// unkeyworded link is `0`; a target of `0` is a cleared link.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct LinkedRefOverrides {
    links: HashMap<(u32, u32), u32>,
}

impl Resource for LinkedRefOverrides {}

impl LinkedRefOverrides {
    /// Link `source` to `target` under `keyword` (`0`: none); `target` `0`
    /// clears the link.
    pub fn set(&mut self, source: u32, keyword: u32, target: u32) {
        self.links.insert((source, keyword), target);
    }

    /// What a script made of `source`'s link under `keyword`: `Some(0)` when
    /// it cleared it, `None` when no script touched it.
    pub fn get(&self, source: u32, keyword: u32) -> Option<u32> {
        self.links.get(&(source, keyword)).copied()
    }
}

/// The reference `source` is linked to under `keyword` (`0`: the unkeyworded
/// link): a script's link, else the authored one on the loaded `source` entity.
pub fn linked_ref(world: &World, source: u32, keyword: u32) -> Option<u32> {
    if let Some(target) = world
        .try_resource::<LinkedRefOverrides>()
        .and_then(|links| links.get(source, keyword))
    {
        return (target != 0).then_some(target);
    }
    let entity = crate::condition::resolve_entity_by_global_form_id(world, source)?;
    world
        .get::<crate::scene::SceneAliasCandidate>(entity)?
        .linked_refs
        .iter()
        .find(|(link, _)| *link == keyword || (keyword == 0 && *link == NO_KEYWORD))
        .map(|(_, target)| *target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cleared_link_reads_as_none_and_an_untouched_one_as_unset() {
        let mut links = LinkedRefOverrides::default();
        assert_eq!(links.get(1, 0), None);
        links.set(1, 0, 7);
        assert_eq!(links.get(1, 0), Some(7));
        links.set(1, 0, 0);
        assert_eq!(links.get(1, 0), Some(0));
        assert_eq!(links.get(1, 5), None, "another keyword");
    }
}
