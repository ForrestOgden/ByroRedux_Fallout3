//! Census of FO4+ RACE subgraph data and the `AnimArchetype` keywords: which
//! archetype keywords the master names, which NPC_/RACE records author them,
//! and how a race's MT entries key on them.
//!
//! Evidence harness for M47.2's `ChangeAnimArchetype` consumer.
//!
//! Usage:
//!   cargo run --release -p byroredux-plugin --example subgraph_census -- <ESM> [RACE editor id]

use std::collections::BTreeMap;

use byroredux_plugin::esm::parse_esm;
use byroredux_plugin::esm::records::SubgraphRole;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("an ESM path");
    let focus = args.next();
    let bytes = std::fs::read(&path)?;
    let index = parse_esm(&bytes)?;
    let name = |id: u32| {
        index
            .keywords
            .get(&id)
            .map_or_else(|| format!("{id:08X}"), |k| format!("{} ({id:08X})", k.editor_id))
    };
    let archetypes: Vec<_> = index
        .keywords
        .values()
        .filter(|k| k.editor_id.to_ascii_lowercase().starts_with("animarch"))
        .collect();
    println!("{} keywords, {} named AnimArch*:", index.keywords.len(), archetypes.len());
    for k in &archetypes {
        println!("  {:08X} {}", k.form_id, k.editor_id);
    }
    let arch_ids: Vec<u32> = archetypes.iter().map(|k| k.form_id).collect();
    let mut npc_with = 0;
    let mut npc_multi = 0;
    for npc in index.npcs.values() {
        let n = npc.keywords.iter().filter(|k| arch_ids.contains(k)).count();
        npc_with += usize::from(n > 0);
        npc_multi += usize::from(n > 1);
    }
    println!("NPC_ with an archetype keyword: {npc_with} ({npc_multi} with more than one); of {}", index.npcs.len());
    let race_with = index.races.values().filter(|r| r.keywords.iter().any(|k| arch_ids.contains(k))).count();
    println!("RACE with an archetype keyword: {race_with} of {}", index.races.len());
    let mut roles: BTreeMap<String, usize> = BTreeMap::new();
    let (mut with_data, mut with_template, mut with_additive) = (0, 0, 0);
    for race in index.races.values() {
        with_data += usize::from(!race.subgraph_data.is_empty());
        with_template += usize::from(race.subgraph_template.is_some());
        with_additive += usize::from(race.subgraph_additive.is_some());
        for d in &race.subgraph_data {
            *roles.entry(format!("{:?}", d.role)).or_default() += 1;
        }
    }
    println!("races: {with_data} with data, {with_template} with template, {with_additive} additive; entries by role {roles:?}");
    if let Some(focus) = focus {
        for race in index.races.values().filter(|r| r.editor_id == focus || format!("{:08X}", r.form_id) == focus) {
            println!("RACE {} {:08X}: template {:?} additive {:?}", race.editor_id, race.form_id,
                race.subgraph_template.map(name), race.subgraph_additive.map(name));
            println!("  race keywords: {:?}", race.keywords.iter().map(|k| name(*k)).collect::<Vec<_>>());
            for d in race.subgraph_data.iter().filter(|d| d.role == SubgraphRole::Mt) {
                println!("  MT {} keys {:?} targets {:?} paths {:?}", d.behavior_graph,
                    d.actor_keywords.iter().map(|k| name(*k)).collect::<Vec<_>>(),
                    d.target_keywords.iter().map(|k| name(*k)).collect::<Vec<_>>(), d.animation_paths);
            }
        }
        for race in index.races.values().filter(|r| r.subgraph_additive.is_some_and(|a| index.races.get(&a).is_some_and(|x| x.editor_id == focus))) {
            println!("ADDITIVE onto {focus}: {} ({} entries)", race.editor_id, race.subgraph_data.len());
        }
    }
    Ok(())
}
