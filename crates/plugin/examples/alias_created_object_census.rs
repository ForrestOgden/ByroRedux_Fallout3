//! Census the Created Object quest aliases (`ALCO` + `ALCA` + `ALCL`) of a
//! master: the create mode (`ALCA` high bit: 0x0000 At / 0x8000 In, xEdit),
//! the base record types instantiated, and whether the target alias is the
//! player-forced one.
//!
//! Evidence harness for M47.2's created-object alias fill.
//!
//! Usage:
//!   cargo run --release -p byroredux-plugin --example alias_created_object_census -- <ESM> [...]

use std::collections::BTreeMap;

use byroredux_plugin::esm::parse_esm;
use byroredux_plugin::esm::records::AliasFillType;

fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path)?;
        let index = parse_esm(&bytes)?;
        let mut by_mode: BTreeMap<u16, usize> = BTreeMap::new();
        let mut by_base: BTreeMap<(u16, String), usize> = BTreeMap::new();
        let mut into_player = 0usize;
        let mut samples = Vec::new();
        for quest in index.quests.values() {
            for alias in &quest.aliases {
                let Some(AliasFillType::CreatedObject {
                    base,
                    target_alias,
                    create_mode,
                    level,
                }) = alias.fill_type
                else {
                    continue;
                };
                *by_mode.entry(create_mode).or_default() += 1;
                let kind = index
                    .record_types
                    .get(&base)
                    .map(|t| String::from_utf8_lossy(t).to_string())
                    .unwrap_or_else(|| "????".into());
                *by_base.entry((create_mode, kind)).or_default() += 1;
                let target = quest.aliases.iter().find(|a| a.alias_id == i32::from(target_alias));
                let player_target = target.is_some_and(|t| {
                    matches!(t.fill_type, Some(AliasFillType::ForcedReference(0x14)))
                });
                if player_target {
                    into_player += 1;
                }
                if samples.len() < 12 {
                    samples.push(format!(
                        "{} alias {} base {base:08X} -> alias {target_alias} mode {create_mode:#06x} level {level} player_target={player_target}",
                        quest.editor_id, alias.alias_id
                    ));
                }
            }
        }
        println!("=== {path}");
        println!("  by create mode: {by_mode:?}");
        println!("  by (mode, base type): {by_base:?}");
        println!("  targeting the PlayerRef-forced alias: {into_player}");
        for s in samples {
            println!("  {s}");
        }
    }
    Ok(())
}
