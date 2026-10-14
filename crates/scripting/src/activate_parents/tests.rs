use super::*;
use byroredux_core::ecs::components::FormIdComponent;
use byroredux_core::form_id::{FormIdPair, FormIdPool, LocalFormId, PluginId};

fn table(edges: &[(u32, u32, f32)], parent_only: &[u32]) -> ActivateParentTable {
    let mut table = ActivateParentTable::default();
    for &(parent, child, delay) in edges {
        table.add(parent, child, delay);
    }
    table.parent_only.extend(parent_only.iter().copied());
    table
}

/// A world with one entity per form, in the order given.
fn world_with(forms: &[u32], table: ActivateParentTable) -> (World, Vec<EntityId>) {
    let mut world = World::new();
    crate::register(&mut world);
    world.register::<FormIdComponent>();
    let mut pool = FormIdPool::default();
    let mut entities = Vec::new();
    for &form in forms {
        let id = pool.intern(FormIdPair { plugin: PluginId::from_filename("Skyrim.esm"), local: LocalFormId(form) });
        let entity = world.spawn();
        world.insert(entity, FormIdComponent(id));
        entities.push(entity);
    }
    world.insert_resource(pool);
    world.insert_resource(table);
    world.insert_resource(PendingParentActivations::default());
    world.insert_resource(crate::fragment::PendingFragmentActivations::default());
    (world, entities)
}

fn queued(world: &World) -> Vec<(EntityId, EntityId)> {
    world.resource::<crate::fragment::PendingFragmentActivations>().0.clone()
}

const LEVER: u32 = 0x10;
const GATE: u32 = 0x11;
const TRAP: u32 = 0x12;

/// Activating a parent queues an activation of each loaded child, with the parent
/// as the activator ("not the originally activating NPC"); a delayed child waits
/// out its seconds first.
#[test]
fn a_parents_activation_reaches_its_children_after_their_delay() {
    let (world, e) = world_with(&[LEVER, GATE, TRAP], table(&[(LEVER, GATE, 0.0), (LEVER, TRAP, 2.0)], &[]));
    let (lever, gate, trap) = (e[0], e[1], e[2]);
    world.query_mut::<crate::ActivateEvent>().unwrap().insert(lever, crate::ActivateEvent { activator: 99 });
    activate_parent_relay_system(&world, 0.5);
    assert_eq!(queued(&world), [(gate, lever)], "the undelayed child at once, activated by the parent");
    world.query_mut::<crate::ActivateEvent>().unwrap().remove(lever);
    activate_parent_relay_system(&world, 1.0);
    assert_eq!(queued(&world).len(), 1, "1.5 s of 2 s");
    activate_parent_relay_system(&world, 0.6);
    assert_eq!(queued(&world), [(gate, lever), (trap, lever)], "the delayed child once its 2 s are up");
}

/// A child that is not loaded is skipped, and an activation with no table
/// entry relays nothing.
#[test]
fn unloaded_children_and_unrelated_activations_relay_nothing() {
    let (world, e) = world_with(&[LEVER, 0x99], table(&[(LEVER, GATE, 0.0)], &[]));
    world.query_mut::<crate::ActivateEvent>().unwrap().insert(e[1], crate::ActivateEvent { activator: 1 });
    activate_parent_relay_system(&world, 0.0);
    world.query_mut::<crate::ActivateEvent>().unwrap().remove(e[1]);
    world.query_mut::<crate::ActivateEvent>().unwrap().insert(e[0], crate::ActivateEvent { activator: 1 });
    activate_parent_relay_system(&world, 0.0);
    assert!(queued(&world).is_empty(), "the gate is not loaded");
}

/// A relay that would close a cycle is skipped.
#[test]
fn a_cycle_is_not_relayed() {
    let (world, e) = world_with(&[LEVER, GATE], table(&[(LEVER, GATE, 0.0), (GATE, LEVER, 0.0)], &[]));
    world.query_mut::<crate::ActivateEvent>().unwrap().insert(e[0], crate::ActivateEvent { activator: 1 });
    activate_parent_relay_system(&world, 0.0);
    assert!(queued(&world).is_empty(), "each is the other's ancestor");
}

#[test]
fn parent_activate_only_is_per_reference() {
    let table = table(&[(LEVER, GATE, 0.0)], &[GATE]);
    assert!(table.takes_parent_activation_only(GATE));
    assert!(!table.takes_parent_activation_only(LEVER));
    assert_eq!(table.children_of(LEVER), [(GATE, 0.0)]);
}
