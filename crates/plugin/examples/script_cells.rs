//! Which interior cells place a reference carrying a named Papyrus script (on
//! the REFR's own VMAD).
//!
//! Usage: cargo run --release -p byroredux-plugin --example script_cells -- <ESM> <script> [...]

use byroredux_plugin::esm::parse_esm;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("an ESM path");
    let wanted: Vec<String> = args.map(|s| s.to_ascii_lowercase()).collect();
    let index = parse_esm(&std::fs::read(&path)?)?;
    for (cell_key, cell) in &index.cells.cells {
        let mut hits = std::collections::BTreeMap::<String, usize>::new();
        for r in &cell.references {
            let Some(instance) = &r.script_instance else { continue };
            for script in &instance.scripts {
                let name = script.name.to_ascii_lowercase();
                if wanted.contains(&name) {
                    *hits.entry(name.clone()).or_default() += 1;
                    if std::env::var("SHOW_REFS").is_ok() {
                        let parents: Vec<String> =
                            r.activate_parents.iter().map(|p| format!("{:08X}", p.reference)).collect();
                        println!("    {cell_key} {name} REFR {:08X} activate parents {parents:?}", r.form_id);
                    }
                }
            }
        }
        if !hits.is_empty() {
            println!("{cell_key}: {hits:?}");
        }
    }
    Ok(())
}
