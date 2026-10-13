//! A placed reference's explicit owner (M47.2): `GetFactionOwner` and
//! `GetActorOwner`.
//!
//! CK *GetFactionOwner - ObjectReference*: "Gets the Faction that owns this
//! object. Will return None if the object isn't owned by a faction", and its
//! notes: it "returns only the explicit owner of an object reference and None
//! when an explicit owner is not found" — a container's or cell's owner is
//! inherited and not returned. *GetActorOwner* is the same for an ActorBase.
//! The `XOWN` form is a faction or an actor base; [`OwnerKinds`] tells which.
//! The cell loader stamps [`ExplicitOwner`] at spawn; both are re-derived.

use std::collections::HashSet;
use std::sync::Arc;

use byroredux_core::ecs::resource::Resource;
use byroredux_core::ecs::sparse_set::SparseSetStorage;
use byroredux_core::ecs::storage::{Component, EntityId};
use byroredux_core::ecs::world::World;
use byroredux_plugin::esm::records::EsmIndex;

/// The `XOWN` form a placed reference carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplicitOwner(pub u32);

impl Component for ExplicitOwner {
    type Storage = SparseSetStorage<Self>;
}

/// Which forms are factions and which are actor bases.
#[derive(Debug, Default, Clone)]
pub struct OwnerKinds {
    factions: Arc<HashSet<u32>>,
    actors: Arc<HashSet<u32>>,
}

impl Resource for OwnerKinds {}

/// Install the load order's faction and actor-base forms.
pub fn install_owner_kinds(world: &mut World, index: &EsmIndex) {
    world.insert_resource(OwnerKinds {
        factions: Arc::new(index.factions.keys().copied().collect()),
        actors: Arc::new(index.npcs.keys().copied().collect()),
    });
}

/// The faction explicitly owning `entity`, if its owner is a faction.
pub fn faction_owner(world: &World, entity: EntityId) -> Option<u32> {
    let owner = world.get::<ExplicitOwner>(entity)?.0;
    world.try_resource::<OwnerKinds>()?.factions.contains(&owner).then_some(owner)
}

/// The actor base explicitly owning `entity`, if its owner is one.
pub fn actor_owner(world: &World, entity: EntityId) -> Option<u32> {
    let owner = world.get::<ExplicitOwner>(entity)?.0;
    world.try_resource::<OwnerKinds>()?.actors.contains(&owner).then_some(owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use byroredux_plugin::esm::records::{FactionRecord, NpcRecord};

    #[test]
    fn the_owner_is_a_faction_or_an_actor_base_never_both() {
        let mut index = EsmIndex::default();
        index.factions.insert(
            0xF,
            FactionRecord {
                form_id: 0xF,
                editor_id: String::new(),
                full_name: String::new(),
                flags: 0,
                relations: Vec::new(),
                ranks: Vec::new(),
                reputation: None,
                vendor: None,
            },
        );
        index.npcs.insert(0xA, NpcRecord::default());
        let mut world = World::new();
        world.register::<ExplicitOwner>();
        install_owner_kinds(&mut world, &index);
        let (by_faction, by_actor, unowned) = (world.spawn(), world.spawn(), world.spawn());
        world.insert(by_faction, ExplicitOwner(0xF));
        world.insert(by_actor, ExplicitOwner(0xA));
        assert_eq!((faction_owner(&world, by_faction), actor_owner(&world, by_faction)), (Some(0xF), None));
        assert_eq!((faction_owner(&world, by_actor), actor_owner(&world, by_actor)), (None, Some(0xA)));
        assert_eq!((faction_owner(&world, unowned), actor_owner(&world, unowned)), (None, None));
    }
}
