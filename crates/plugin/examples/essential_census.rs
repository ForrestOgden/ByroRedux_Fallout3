//! M47.2 census: how many actor bases are Essential / Protected once the
//! template's "Use Base Data" is resolved (xEdit: ACBS bit 1 Essential;
//! bit 11 Protected from Skyrim on, "No Blood Spray" on FO3/FNV), and the
//! status of any base FormIDs given after `--`.
//!
//! `ESSENTIAL_LIST=1` also lists every essential base.
//!
//! Usage:
//!   cargo run -p byroredux-plugin --example essential_census -- <master.esm> [-- <base hex>...]

use byroredux_plugin::equip::ResolvedNpc;
use byroredux_plugin::esm::reader::GameKind;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (paths, bases) = match args.iter().position(|arg| arg == "--") {
        Some(split) => (&args[..split], &args[split + 1..]),
        None => (&args[..], &[][..]),
    };
    for path in paths {
        let index = byroredux_plugin::esm::parse_esm(&std::fs::read(path)?)?;
        let protected_bit = !matches!(index.game, GameKind::Oblivion | GameKind::Fallout3NV);
        let flags = |base: u32| {
            let npc = index.npcs.get(&base).or_else(|| index.creatures.get(&base))?;
            Some(ResolvedNpc::resolve(npc, &index).base_data.acbs_flags)
        };
        let (mut essential, mut protected) = (0, 0);
        for &base in index.npcs.keys().chain(index.creatures.keys()) {
            let flags = flags(base).unwrap_or(0);
            essential += usize::from(flags & 0x2 != 0);
            if flags & 0x2 != 0 && std::env::var_os("ESSENTIAL_LIST").is_some() {
                let record = index.npcs.get(&base).or(index.creatures.get(&base)).unwrap();
                println!("  essential {base:08X} {}", record.editor_id);
            }
            protected += usize::from(protected_bit && flags & 0x800 != 0);
        }
        println!(
            "{path}: {} actor bases, {essential} essential, {protected} protected",
            index.npcs.len() + index.creatures.len()
        );
        for base in bases {
            let form = u32::from_str_radix(base.trim_start_matches("0x"), 16)?;
            match flags(form) {
                Some(flags) => println!(
                    "  {form:08X} {}: essential={} protected={}",
                    index.npcs.get(&form).or(index.creatures.get(&form)).unwrap().editor_id,
                    flags & 0x2 != 0,
                    protected_bit && flags & 0x800 != 0
                ),
                None => println!("  {form:08X}: not an actor base"),
            }
        }
    }
    Ok(())
}
