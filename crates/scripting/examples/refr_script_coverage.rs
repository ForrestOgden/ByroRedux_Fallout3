//! M47.2 Phase 3 metric — how many vanilla *scripted references* the
//! recognizer chain gives working ECS behavior: every placed REFR / ACHR whose
//! own `VMAD`, or whose base record's `VMAD`, attaches a Papyrus script; a
//! reference counts as covered when at least one of its scripts is
//! recognized. The per-family table (script name → references carrying it) is
//! what the "top-N families" recognizer work is driven by.
//!
//! ```bash
//! cargo run --release -p byroredux-scripting --example refr_script_coverage -- \
//!     "<Skyrim SE>/Data/Skyrim.esm" "<Skyrim SE>/Data/Skyrim - Misc.bsa"
//! ```

use std::collections::HashMap;

use byroredux_bsa::{Ba2Archive, BsaArchive};
use byroredux_plugin::esm::parse_esm;
use byroredux_plugin::esm::records::script_instance::ScriptInstanceData;
use byroredux_scripting::translate_pex;

enum Archive {
    Bsa(BsaArchive),
    Ba2(Ba2Archive),
}

impl Archive {
    fn open(path: &str) -> std::io::Result<Self> {
        if path.to_ascii_lowercase().ends_with(".ba2") {
            Ok(Archive::Ba2(Ba2Archive::open(path)?))
        } else {
            Ok(Archive::Bsa(BsaArchive::open(path)?))
        }
    }
    fn extract(&self, path: &str) -> std::io::Result<Vec<u8>> {
        match self {
            Archive::Bsa(a) => a.extract(path),
            Archive::Ba2(a) => a.extract(path),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let esm = args.next().expect("usage: refr_script_coverage <ESM> <scripts archive>");
    let archive = Archive::open(&args.next().expect("scripts archive"))?;
    let index = parse_esm(&std::fs::read(&esm)?)?;

    // Script name (lowercased) → (references carrying it, recognized?).
    let mut families: HashMap<String, (usize, Option<bool>)> = HashMap::new();
    let (mut scripted, mut covered) = (0usize, 0usize);
    let mut recognized_cache: HashMap<String, bool> = HashMap::new();
    let mut recognize = |name: &str, vmad: &ScriptInstanceData| -> bool {
        let key = name.to_ascii_lowercase();
        if let Some(&hit) = recognized_cache.get(&key) {
            return hit;
        }
        let path = format!("scripts\\{}.pex", key.replace(':', "\\"));
        let hit = archive
            .extract(&path)
            .ok()
            .and_then(|bytes| translate_pex(&bytes, index.game, Some(vmad), None))
            .is_some();
        recognized_cache.insert(key, hit);
        hit
    };

    let cells = index
        .cells
        .cells
        .values()
        .chain(index.cells.exterior_cells.values().flat_map(|grid| grid.values()));
    for cell in cells {
        for placed in &cell.references {
            let mut vmads: Vec<&ScriptInstanceData> = Vec::new();
            vmads.extend(placed.script_instance.as_ref());
            vmads.extend(index.base_record_script_instance(placed.base_form_id));
            if vmads.iter().all(|vmad| vmad.scripts.is_empty()) {
                continue;
            }
            scripted += 1;
            let mut any = false;
            for vmad in vmads {
                for script in &vmad.scripts {
                    let hit = recognize(&script.name, vmad);
                    any |= hit;
                    let family = families.entry(script.name.to_ascii_lowercase()).or_default();
                    family.0 += 1;
                    family.1 = Some(hit);
                }
            }
            covered += usize::from(any);
        }
    }
    let pct = 100.0 * covered as f64 / scripted.max(1) as f64;
    println!("scripted references: {scripted}; covered by a recognizer: {covered} ({pct:.1}%)");
    let mut ranked: Vec<_> = families.into_iter().collect();
    ranked.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then_with(|| a.0.cmp(&b.0)));
    println!("top script families (references, recognized):");
    for (name, (count, hit)) in ranked.iter().take(40) {
        println!("  {count:>6}  {}  {name}", if *hit == Some(true) { "yes" } else { " no" });
    }
    Ok(())
}
