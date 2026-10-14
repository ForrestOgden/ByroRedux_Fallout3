//! Print an interior cell's linked-reference edges that touch the given references
//! (as source or target), with the source's base and game-space position: how a
//! trap is wired to the trigger that springs it.
//!
//! Usage: cargo run --release -p byroredux-plugin --example cell_ref_links -- <ESM> <cell editor id> <ref hex>...

use byroredux_plugin::esm::parse_esm;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("an ESM path");
    let cell = args.next().expect("a cell editor id").to_ascii_lowercase();
    let wanted: Vec<u32> = args.map(|b| u32::from_str_radix(b.trim_start_matches("0x"), 16).expect("hex")).collect();
    let index = parse_esm(&std::fs::read(&path)?)?;
    let data = index.cells.cells.get(&cell).expect("the cell");
    let name = |base: u32| index.cells.statics.get(&base).map(|s| s.editor_id.clone()).unwrap_or_default();
    for r in data.references.iter().filter(|r| wanted.contains(&r.form_id)) {
        for instance in r.script_instance.iter().flat_map(|s| s.scripts.iter()) {
            println!("    REFR script {}: {} properties", instance.name, instance.properties.len());
            for property in &instance.properties {
                println!("      {} = {:?}", property.name, property.value);
            }
        }
        println!("POS {:08X} ({} base {:08X}) pos {:?}", r.form_id, name(r.base_form_id), r.base_form_id, r.position);
    }
    for r in &data.references {
        for link in &r.linked_refs {
            if wanted.contains(&r.form_id) || wanted.contains(&link.target) {
                println!(
                    "REFR {:08X} ({} base {:08X}) pos {:?} -> {:08X} keyword {:08X}",
                    r.form_id,
                    name(r.base_form_id),
                    r.base_form_id,
                    r.position,
                    link.target,
                    link.keyword
                );
            }
        }
    }
    for r in &data.references {
        for parent in &r.activate_parents {
            if wanted.is_empty() || wanted.contains(&r.form_id) || wanted.contains(&parent.reference) {
                println!(
                    "ACTIVATE-PARENT {:08X} ({} base {:08X}) <- parent {:08X} delay {} parent-only {}",
                    r.form_id,
                    name(r.base_form_id),
                    r.base_form_id,
                    parent.reference,
                    parent.delay,
                    r.parent_activate_only
                );
            }
        }
    }
    Ok(())
}
