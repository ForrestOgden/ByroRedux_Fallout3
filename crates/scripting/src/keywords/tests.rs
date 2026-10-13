use super::*;
use byroredux_plugin::esm::records::{NpcRecord, RaceRecord};

const DREMORA_RACE: u32 = 0x0001_3740;
const DREMORA: u32 = 0x0001_6F7D;
const ACTOR_TYPE_DAEDRA: u32 = 0x0001_3797;
const ACTOR_TYPE_NPC: u32 = 0x0001_3794;
const INJURED: u32 = 0x0009_1234;

fn world_with_dremora() -> (World, EntityId) {
    let mut world = World::new();
    crate::register(&mut world);
    let mut index = EsmIndex::default();
    index.races.insert(
        DREMORA_RACE,
        RaceRecord {
            form_id: DREMORA_RACE,
            keywords: vec![ACTOR_TYPE_DAEDRA],
            ..Default::default()
        },
    );
    index.npcs.insert(
        DREMORA,
        NpcRecord {
            form_id: DREMORA,
            race_form_id: DREMORA_RACE,
            keywords: vec![ACTOR_TYPE_NPC],
            ..Default::default()
        },
    );
    install_keyword_table(&mut world, &index);
    let entity = world.spawn();
    world.insert(
        entity,
        crate::scene::SceneAliasCandidate {
            base_form_id: DREMORA,
            ..Default::default()
        },
    );
    (world, entity)
}

/// An actor has its own `KWDA`, its race's, and what scripts add; a script's
/// `RemoveKeyword` overrides the base, and `ResetKeyword` undoes both.
#[test]
fn actors_have_their_race_keywords_and_scripts_override() {
    let (world, dremora) = world_with_dremora();
    assert!(has_keyword(&world, dremora, ACTOR_TYPE_NPC));
    assert!(has_keyword(&world, dremora, ACTOR_TYPE_DAEDRA), "the race's");
    assert!(!has_keyword(&world, dremora, INJURED));
    // No FormID to key a change by: refused rather than lost silently.
    assert!(!change_keyword(&world, dremora, INJURED, KeywordChange::Add));

    let mut world = world;
    use byroredux_core::form_id::{FormIdPair, FormIdPool, LocalFormId, PluginId};
    world.insert_resource(FormIdPool::new());
    let id = world.resource_mut::<FormIdPool>().intern(FormIdPair {
        plugin: PluginId::from_filename("Skyrim.esm"),
        local: LocalFormId(0x0001_6F7E),
    });
    world.insert(dremora, byroredux_core::ecs::components::FormIdComponent(id));
    assert!(change_keyword(&world, dremora, INJURED, KeywordChange::Add));
    assert!(has_keyword(&world, dremora, INJURED));
    assert!(change_keyword(&world, dremora, ACTOR_TYPE_DAEDRA, KeywordChange::Remove));
    assert!(!has_keyword(&world, dremora, ACTOR_TYPE_DAEDRA));
    assert!(change_keyword(&world, dremora, ACTOR_TYPE_DAEDRA, KeywordChange::Reset));
    assert!(has_keyword(&world, dremora, ACTOR_TYPE_DAEDRA), "back to the race's");
}
