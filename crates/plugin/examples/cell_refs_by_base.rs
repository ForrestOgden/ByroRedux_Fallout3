//! Print the placed references of an interior cell whose base is one of the given
//! form ids, with their game-space position: where to put the player to touch them.
//!
//! Usage: cargo run --release -p byroredux-plugin --example cell_refs_by_base -- <ESM> <cell editor id> <base hex>...

use byroredux_plugin::esm::parse_esm;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("an ESM path");
    let cell = args.next().expect("a cell editor id").to_ascii_lowercase();
    let bases: Vec<u32> = args.map(|b| u32::from_str_radix(b.trim_start_matches("0x"), 16).expect("hex")).collect();
    let index = parse_esm(&std::fs::read(&path)?)?;
    let data = index.cells.cells.get(&cell).expect("the cell");
    for r in data.references.iter().filter(|r| bases.contains(&r.base_form_id)) {
        let name = index.cells.statics.get(&r.base_form_id).map(|s| s.editor_id.as_str()).unwrap_or("?");
        println!("REFR {:08X} base {:08X} {name} pos {:?} rot {:?}", r.form_id, r.base_form_id, r.position, r.rotation);
    }
    Ok(())
}
