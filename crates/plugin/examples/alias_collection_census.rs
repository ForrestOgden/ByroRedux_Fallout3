//! Census the reference-collection aliases (FO4 / Starfield `ALCS`) of a
//! master: their fill types, whether they carry match conditions, and the
//! injected data (packages, factions, inventory) that would apply to every
//! member.
//!
//! Evidence harness for M47.2's reference-collection fill.
//!
//! Usage:
//!   cargo run --release -p byroredux-plugin --example alias_collection_census -- <ESM> [...]

use std::collections::BTreeMap;

use byroredux_plugin::esm::parse_esm;
use byroredux_plugin::esm::records::AliasFillType;

fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path)?;
        let index = parse_esm(&bytes)?;
        let mut by_fill: BTreeMap<&'static str, usize> = BTreeMap::new();
        let (mut total, mut with_conditions, mut with_packages, mut with_factions, mut with_items) =
            (0, 0, 0, 0, 0);
        for quest in index.quests.values() {
            for alias in quest.aliases.iter().filter(|a| a.is_collection) {
                total += 1;
                let fill = match &alias.fill_type {
                    None => "none (find matching)",
                    Some(AliasFillType::ForcedReference(_)) => "forced reference",
                    Some(AliasFillType::UniqueActor(_)) => "unique actor",
                    Some(AliasFillType::CreatedObject { .. }) => "created object",
                    Some(AliasFillType::ExternalAlias { .. }) => "external alias",
                    Some(AliasFillType::LocationAliasReference { .. }) => "location alias reference",
                    Some(AliasFillType::NearAlias { .. }) => "near alias",
                    Some(AliasFillType::FromEvent { .. }) => "from event",
                    Some(AliasFillType::ForcedLocation(_)) => "forced location",
                };
                *by_fill.entry(fill).or_default() += 1;
                with_conditions += usize::from(!alias.match_conditions.is_empty());
                with_packages += usize::from(!alias.injected.packages.is_empty());
                with_factions += usize::from(!alias.injected.factions.is_empty());
                with_items += usize::from(!alias.injected.inventory.is_empty());
            }
        }
        println!("=== {path}: {total} collection aliases");
        println!("  by fill: {by_fill:?}");
        println!(
            "  with match conditions {with_conditions}, packages {with_packages}, \
             factions {with_factions}, inventory {with_items}"
        );
    }
    Ok(())
}
