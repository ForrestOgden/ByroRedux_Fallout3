//! Print the script properties of one placed reference (its own VMAD, then its base
//! record's) and its encounter zone: the authored inputs to a script's computed values.
//!
//! ```text
//! cargo run --release -p byroredux-scripting --example refr_script_props -- <ESM> <REFR formid hex>
//! ```

use byroredux_plugin::esm::parse_esm;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let esm = args.next().expect("usage: refr_script_props <ESM> <REFR formid hex>");
    let wanted = u32::from_str_radix(args.next().expect("a REFR formid").trim_start_matches("0x"), 16)?;
    let index = parse_esm(&std::fs::read(&esm)?)?;
    let cells = index
        .cells
        .cells
        .values()
        .chain(index.cells.exterior_cells.values().flat_map(|grid| grid.values()));
    for cell in cells {
        for placed in &cell.references {
            if placed.form_id & 0x00FF_FFFF != wanted & 0x00FF_FFFF {
                continue;
            }
            println!("REFR {:08X} in cell {:08X}, base {:08X}; XEZN {:?}; cell encounter zone {:?}", placed.form_id, cell.form_id, placed.base_form_id, placed.encounter_zone_form, cell.encounter_zone_form);
            if let Some(zone) = placed.encounter_zone_form.or(cell.encounter_zone_form).and_then(|id| index.encounter_zones.get(&id)) {
                println!("  zone {zone:?}");
            }
            let own = placed.script_instance.iter();
            let base = index.base_record_script_instance(placed.base_form_id).into_iter();
            for (source, vmad) in own.map(|v| ("REFR", v)).chain(base.map(|v| ("base", v))) {
                for script in &vmad.scripts {
                    println!("  [{source}] script {}", script.name);
                    for property in &script.properties {
                        println!("    {} = {:?}", property.name, property.value);
                    }
                }
            }
        }
    }
    Ok(())
}
