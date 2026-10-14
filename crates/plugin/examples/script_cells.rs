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
                    *hits.entry(name).or_default() += 1;
                }
            }
        }
        if !hits.is_empty() {
            println!("{cell_key}: {hits:?}");
        }
    }
    Ok(())
}
