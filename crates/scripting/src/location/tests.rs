use super::*;
use byroredux_plugin::esm::cell::CellData;
use byroredux_plugin::esm::records::LctnRecord;

use crate::papyrus_demo::PapyrusPlayerEntity;

pub(crate) const TAMRIEL: u32 = 0x0001_30FF;
pub(crate) const WHITERUN_HOLD: u32 = 0x0001_6772;
pub(crate) const WHITERUN: u32 = 0x0001_8A56;
pub(crate) const DRAGONSREACH: u32 = 0x0001_F871;
pub(crate) const LOC_TYPE_CITY: u32 = 0x0001_3168;
pub(crate) const LOC_TYPE_HOLD: u32 = 0x0001_6771;
pub(crate) const DRAGONSREACH_CELL: u32 = 0x0001_6BDF;
pub(crate) const PERSISTENT_REF: u32 = 0x000A_0001;
pub(crate) const BOSS_REF_TYPE: u32 = 0x0001_30F8;
pub(crate) const BOSS_REF: u32 = 0x000A_0002;

/// Tamriel ⊃ Whiterun Hold ⊃ Whiterun ⊃ Dragonsreach, with Dragonsreach's
/// cell tagged and one persistent reference listed under Whiterun.
pub(crate) fn whiterun_index() -> EsmIndex {
    let mut index = EsmIndex::default();
    for (form_id, parent, keywords, refs) in [
        (TAMRIEL, None, vec![], vec![]),
        (WHITERUN_HOLD, Some(TAMRIEL), vec![LOC_TYPE_HOLD], vec![]),
        (
            WHITERUN,
            Some(WHITERUN_HOLD),
            vec![LOC_TYPE_CITY],
            vec![(PERSISTENT_REF, WHITERUN)],
        ),
        (DRAGONSREACH, Some(WHITERUN), vec![], vec![]),
    ] {
        index.locations.insert(
            form_id,
            LctnRecord {
                form_id,
                parent,
                keywords,
                editor_location_refs: refs,
                ref_type_refs: if form_id == DRAGONSREACH {
                    vec![(BOSS_REF_TYPE, BOSS_REF)]
                } else {
                    vec![]
                },
                ..Default::default()
            },
        );
    }
    index.location_ref_types.insert(
        BOSS_REF_TYPE,
        byroredux_plugin::esm::records::MinimalEsmRecord {
            form_id: BOSS_REF_TYPE,
            editor_id: "Boss".into(),
            ..Default::default()
        },
    );
    index.cells.cells.insert(
        "whiterundragonsreach".into(),
        CellData {
            form_id: DRAGONSREACH_CELL,
            location_form: Some(DRAGONSREACH),
            ..Default::default()
        },
    );
    index
}

#[test]
fn hierarchy_queries_walk_the_parent_chain() {
    let table = LocationTable::from_index(&whiterun_index());
    assert_eq!(
        table.locations(),
        &[TAMRIEL, WHITERUN_HOLD, WHITERUN, DRAGONSREACH]
    );
    assert!(table.is_within(DRAGONSREACH, DRAGONSREACH));
    assert!(table.is_within(DRAGONSREACH, WHITERUN_HOLD));
    assert!(
        !table.is_within(WHITERUN, DRAGONSREACH),
        "a parent is not within its child"
    );
    assert!(table.has_keyword(WHITERUN, LOC_TYPE_CITY));
    assert!(
        !table.has_keyword(DRAGONSREACH, LOC_TYPE_CITY),
        "keywords are not inherited"
    );
    assert_eq!(
        table.with_keyword(DRAGONSREACH, LOC_TYPE_HOLD),
        Some(WHITERUN_HOLD)
    );
    assert_eq!(table.with_keyword(TAMRIEL, LOC_TYPE_HOLD), None);
    assert_eq!(table.cell_location(DRAGONSREACH_CELL), Some(DRAGONSREACH));
    assert_eq!(table.listed_editor_location(PERSISTENT_REF), Some(WHITERUN));
    assert_eq!(table.refs_of_type(DRAGONSREACH, BOSS_REF_TYPE).collect::<Vec<_>>(), vec![BOSS_REF]);
    assert_eq!(table.refs_of_type(WHITERUN, BOSS_REF_TYPE).count(), 0);
}

#[test]
fn a_cyclic_parent_chain_terminates() {
    let mut index = EsmIndex::default();
    for (form_id, parent) in [(1, 2), (2, 1)] {
        index.locations.insert(
            form_id,
            LctnRecord {
                form_id,
                parent: Some(parent),
                ..Default::default()
            },
        );
    }
    let table = LocationTable::from_index(&index);
    assert!(table.is_within(1, 2));
    assert!(!table.is_within(1, 3));
}

#[test]
fn current_location_follows_the_cell_root_and_the_published_player_location() {
    let mut world = World::new();
    install_location_table(&mut world, &whiterun_index());
    let cell_root = world.spawn();
    world.insert(cell_root, CellFormId(DRAGONSREACH_CELL));
    let guard = world.spawn();
    world.insert(guard, CellRoot(cell_root));
    let player = world.spawn();
    world.insert_resource(PapyrusPlayerEntity(player));

    assert_eq!(current_location(&world, guard), Some(DRAGONSREACH));
    assert_eq!(editor_location(&world, guard), Some(DRAGONSREACH));
    assert_eq!(
        current_location(&world, player),
        None,
        "untagged until published"
    );
    world.resource_mut::<PlayerLocation>().0 = Some(WHITERUN);
    assert_eq!(current_location(&world, player), Some(WHITERUN));
    assert_eq!(editor_location(&world, player), None);
    let stray = world.spawn();
    assert_eq!(current_location(&world, stray), None);
}

#[test]
fn a_location_is_loaded_while_one_of_its_own_cells_is() {
    let mut world = World::new();
    install_location_table(&mut world, &whiterun_index());
    assert_eq!(location_loaded(&world, DRAGONSREACH), Some(false), "no cell is resident");
    let cell_root = world.spawn();
    world.insert(cell_root, CellFormId(DRAGONSREACH_CELL));
    assert_eq!(location_loaded(&world, DRAGONSREACH), Some(true));
    assert_eq!(location_loaded(&world, WHITERUN), Some(false), "a child's cell does not load its parent");
    assert_eq!(location_loaded(&world, 0x0DEA_D000), None, "not a location");
}

mod conditions {
    use super::*;
    use crate::condition::{evaluate_condition, ConditionContext};
    use crate::quest_stages::QuestFormId;
    use crate::scene::SceneActorBindings;
    use byroredux_plugin::esm::records::condition::{
        ComparisonOp, Condition, ConditionValue, RunOn,
    };

    const QUEST: QuestFormId = QuestFormId(0x0004_CF45);

    fn holds(function_index: u32, param_1: u32, param_2: u32) -> Condition {
        Condition {
            function_index,
            comparator: ComparisonOp::Eq,
            comparand: ConditionValue::Literal(1.0),
            param_1,
            param_2,
            run_on: RunOn::Subject,
            ..Default::default()
        }
    }

    /// A guard standing in Dragonsreach, a persistent reference listed under
    /// Whiterun, and the quest's location alias 3 bound to Whiterun.
    fn world() -> (World, EntityId, EntityId) {
        let mut world = World::new();
        crate::register(&mut world);
        world.insert_resource(byroredux_core::form_id::FormIdPool::new());
        install_location_table(&mut world, &whiterun_index());
        let cell_root = world.spawn();
        world.insert(cell_root, CellFormId(DRAGONSREACH_CELL));
        let guard = world.spawn();
        world.insert(guard, CellRoot(cell_root));
        let persistent = world.spawn();
        let form = world
            .resource_mut::<byroredux_core::form_id::FormIdPool>()
            .intern(byroredux_core::form_id::FormIdPair {
                plugin: byroredux_core::form_id::PluginId::from_filename("Skyrim.esm"),
                local: byroredux_core::form_id::LocalFormId(PERSISTENT_REF),
            });
        world.insert(
            persistent,
            byroredux_core::ecs::components::FormIdComponent(form),
        );
        world
            .resource_mut::<LocationAliasBindings>()
            .bind(QUEST, 3, WHITERUN);
        world
            .resource_mut::<SceneActorBindings>()
            .bind(QUEST, 1, guard);
        (world, guard, persistent)
    }

    #[test]
    fn current_and_editor_location_tests_walk_the_hierarchy() {
        let (world, guard, persistent) = world();
        let on = |entity, condition: Condition| {
            evaluate_condition(
                &condition,
                &world,
                &ConditionContext::for_subject(entity).with_quest(QUEST),
            )
        };
        assert!(
            on(guard, holds(359, WHITERUN_HOLD, 0)),
            "GetInCurrentLoc parent"
        );
        assert!(!on(guard, holds(359, 0x0BAD, 0)));
        assert!(on(guard, holds(360, 3, 0)), "GetInCurrentLocAlias");
        assert!(
            !on(guard, holds(562, LOC_TYPE_CITY, 0)),
            "keywords are not inherited"
        );
        assert!(
            on(guard, holds(565, DRAGONSREACH, 0)),
            "editor location = placed cell"
        );
        assert!(
            on(persistent, holds(565, WHITERUN, 0)),
            "listed persistent ref"
        );
        assert!(!on(persistent, holds(565, DRAGONSREACH, 0)));
        assert!(on(persistent, holds(567, 3, 0)), "GetIsEditorLocAlias");
        assert!(on(guard, holds(566, 1, 0)), "GetIsAliasRef");
        assert!(!on(persistent, holds(566, 1, 0)));
        assert!(on(guard, holds(605, 3, WHITERUN)), "LocAliasIsLocation");
        assert!(!on(guard, holds(605, 3, WHITERUN_HOLD)));
        // No owning quest: alias tests fail rather than read another quest.
        assert!(!evaluate_condition(
            &holds(566, 1, 0),
            &world,
            &ConditionContext::for_subject(guard)
        ));
    }

    #[test]
    fn a_location_subject_tests_the_location_itself() {
        let (world, _, _) = world();
        let on = |location, condition: Condition| {
            evaluate_condition(
                &condition,
                &world,
                &ConditionContext::for_subject(EntityId::MAX)
                    .with_quest(QUEST)
                    .with_location_subject(location),
            )
        };
        assert!(on(WHITERUN, holds(562, LOC_TYPE_CITY, 0)));
        assert!(!on(DRAGONSREACH, holds(562, LOC_TYPE_CITY, 0)));
        // "returns 1 if the parent of the location is (or has a parent of)".
        assert!(on(DRAGONSREACH, holds(359, WHITERUN, 0)));
        assert!(!on(WHITERUN, holds(359, WHITERUN, 0)));
        assert!(on(DRAGONSREACH, holds(360, 3, 0)));
        assert!(
            !on(DRAGONSREACH, holds(565, WHITERUN, 0)),
            "no editor location"
        );
    }
}

mod ref_type_and_shared_editor_location {
    use super::*;
    use crate::condition::{evaluate_condition, ConditionContext};
    use crate::quest_stages::QuestFormId;
    use crate::scene::{SceneActorBindings, SceneAliasCandidate};
    use byroredux_plugin::esm::records::condition::{
        ComparisonOp, Condition, ConditionValue, RunOn,
    };

    const QUEST: QuestFormId = QuestFormId(0x0004_CF46);

    fn holds(function_index: u32, param_1: u32, param_2: u32) -> Condition {
        Condition {
            function_index,
            comparator: ComparisonOp::Eq,
            comparand: ConditionValue::Literal(1.0),
            param_1,
            param_2,
            run_on: RunOn::Subject,
            ..Default::default()
        }
    }

    #[test]
    fn ref_types_and_shared_editor_locations() {
        let mut world = World::new();
        crate::register(&mut world);
        install_location_table(&mut world, &whiterun_index());
        let cell_root = world.spawn();
        world.insert(cell_root, CellFormId(DRAGONSREACH_CELL));
        let guard = world.spawn();
        world.insert(guard, CellRoot(cell_root));
        // A remote stub for the persistent ref: editor location Whiterun.
        let stub = world.spawn();
        world.insert(
            stub,
            SceneAliasCandidate {
                reference_form_id: PERSISTENT_REF,
                base_form_id: 0xB1,
                linked_refs: Vec::new(),
                location_ref_types: Vec::new(),
            },
        );
        world.resource_mut::<SceneActorBindings>().bind(QUEST, 1, stub);
        let ctx = ConditionContext::for_subject(guard).with_quest(QUEST);
        let on_guard = |condition: Condition| evaluate_condition(&condition, &world, &ctx);
        let on_location = |location, condition: Condition| {
            evaluate_condition(
                &condition,
                &world,
                &ConditionContext::for_subject(EntityId::MAX)
                    .with_quest(QUEST)
                    .with_location_subject(location),
            )
        };

        assert!(on_guard(holds(563, BOSS_REF_TYPE, 0)), "Dragonsreach has a boss");
        assert!(on_location(DRAGONSREACH, holds(563, BOSS_REF_TYPE, 0)));
        assert!(!on_location(WHITERUN, holds(563, BOSS_REF_TYPE, 0)));
        world
            .resource_mut::<crate::fragment::ReferenceEnableState>()
            .set_enabled(BOSS_REF, false);
        assert!(
            !on_location(DRAGONSREACH, holds(563, BOSS_REF_TYPE, 0)),
            "a disabled boss does not count"
        );

        // The guard (editor location Dragonsreach) and the stub (Whiterun):
        // different locations, the same city.
        assert!(!on_guard(holds(181, 1, 0)));
        assert!(on_guard(holds(181, 1, LOC_TYPE_CITY)));
        assert!(on_guard(holds(181, 1, LOC_TYPE_HOLD)));
        // As a fill condition: the keyworded location containing the alias's
        // editor location.
        assert!(on_location(WHITERUN_HOLD, holds(181, 1, LOC_TYPE_HOLD)));
        assert!(!on_location(WHITERUN, holds(181, 1, LOC_TYPE_HOLD)), "wrong keyword");
        assert!(!on_location(DRAGONSREACH, holds(181, 1, 0)), "does not contain Whiterun");
    }

    /// M47.2 — CK *Cell Reset*: a location clears once every `Boss` ref is
    /// dead, or when a script calls `SetCleared`; 250 / 598 read it.
    #[test]
    fn a_location_clears_when_every_boss_is_dead() {
        const SECOND_BOSS: u32 = 0x000A_0003;
        let mut index = whiterun_index();
        index
            .locations
            .get_mut(&DRAGONSREACH)
            .unwrap()
            .ref_type_refs
            .push((BOSS_REF_TYPE, SECOND_BOSS));
        let mut world = World::new();
        crate::register(&mut world);
        world.insert_resource(byroredux_core::form_id::FormIdPool::new());
        install_location_table(&mut world, &index);
        let spawn = |world: &mut World, reference: u32| {
            let entity = world.spawn();
            let form = world
                .resource_mut::<byroredux_core::form_id::FormIdPool>()
                .intern(byroredux_core::form_id::FormIdPair {
                    plugin: byroredux_core::form_id::PluginId::from_filename("Skyrim.esm"),
                    local: byroredux_core::form_id::LocalFormId(reference),
                });
            world.insert(entity, byroredux_core::ecs::components::FormIdComponent(form));
            entity
        };
        let first = spawn(&mut world, BOSS_REF);
        let second = spawn(&mut world, SECOND_BOSS);
        let bystander = spawn(&mut world, PERSISTENT_REF);
        world
            .resource_mut::<LocationAliasBindings>()
            .bind(QUEST, 4, DRAGONSREACH);
        let cleared = |world: &World, condition: Condition| {
            evaluate_condition(
                &condition,
                world,
                &ConditionContext::for_subject(EntityId::MAX).with_quest(QUEST),
            )
        };

        note_actor_death(&world, bystander);
        note_actor_death(&world, first);
        assert!(!location_cleared(&world, DRAGONSREACH), "one boss still lives");
        note_actor_death(&world, second);
        note_actor_death(&world, second);
        assert!(location_cleared(&world, DRAGONSREACH));
        assert!(!location_cleared(&world, WHITERUN), "clearing is not inherited");
        assert!(cleared(&world, holds(250, DRAGONSREACH, 0)));
        assert!(!cleared(&world, holds(250, WHITERUN, 0)));
        assert!(cleared(&world, holds(598, 4, 0)));
        assert!(!cleared(&world, holds(598, 3, 0)));

        // `SetCleared(false)` un-clears; the dead bosses stay dead, so the
        // location is not cleared again until a script says so.
        world
            .resource_mut::<LocationClearState>()
            .set_cleared(DRAGONSREACH, false);
        assert!(!cleared(&world, holds(250, DRAGONSREACH, 0)));
        world
            .resource_mut::<LocationClearState>()
            .set_cleared(WHITERUN, true);
        assert!(cleared(&world, holds(250, WHITERUN, 0)));
    }
}
