//! How many of the editor ids listed on stdin (one per line) name a sound record
//! in an ESM — the check that a graph's `SoundPlay.<name>` notify names a
//! descriptor by editor id.
//!
//! Usage: cargo run --release -p byroredux-plugin --example sound_names -- <ESM> < names.txt

use std::io::BufRead;

use byroredux_plugin::esm::parse_esm;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("an ESM path");
    let index = parse_esm(&std::fs::read(&path)?)?;
    let by_edid: std::collections::HashMap<String, &byroredux_plugin::esm::records::SounRecord> =
        index.sounds.values().map(|s| (s.editor_id.to_ascii_lowercase(), s)).collect();
    let (mut found, mut missing, mut files) = (0, Vec::new(), 0);
    for line in std::io::stdin().lock().lines() {
        let name = line?;
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        match by_edid.get(&name.to_ascii_lowercase()) {
            Some(record) => {
                found += 1;
                files += usize::from(!record.sound_path.is_empty() || !record.variants.is_empty());
            }
            None => missing.push(name.to_owned()),
        }
    }
    println!("{} sound records; {found} found ({files} with a file); {} missing: {missing:?}", index.sounds.len(), missing.len());
    Ok(())
}
