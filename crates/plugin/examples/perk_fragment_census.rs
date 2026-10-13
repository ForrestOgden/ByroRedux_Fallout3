//! M47.2 census: the fragments perks bind (`PERK` `VMAD`, the Fallout 4 / Starfield
//! layout shared with `TERM`) and the entry each one hangs off — the entry's point,
//! its `EPFT` function type, `EPFB` entry id and `EPF3` flags.
//!
//! ```bash
//! cargo run --release -p byroredux-plugin --example perk_fragment_census -- "<Fallout 4>/Data/Fallout4.esm"
//! ```

use std::collections::BTreeMap;

use byroredux_plugin::esm::reader::EsmReader;
use byroredux_plugin::esm::records::script_instance::parse_indexed_fragments;

type OwnedSub = ([u8; 4], Vec<u8>);

fn walk(reader: &mut EsmReader, end: usize, f: &mut dyn FnMut(u32, &[OwnedSub])) {
    while reader.position() < end && reader.remaining() > 0 {
        if reader.is_group() {
            let Ok(g) = reader.read_group_header() else {
                return;
            };
            let sub_end = reader.group_content_end(&g);
            walk(reader, sub_end, f);
            continue;
        }
        let Ok(header) = reader.read_record_header() else {
            return;
        };
        if &header.record_type == b"PERK" {
            if let Ok(subs) = reader.read_sub_records(&header) {
                let owned: Vec<OwnedSub> = subs.into_iter().map(|s| (s.sub_type, s.data)).collect();
                f(header.form_id, &owned);
            }
        } else {
            reader.skip_record(&header);
        }
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).ok_or_else(|| anyhow::anyhow!("usage: perk_fragment_census ESM"))?;
    let bytes = std::fs::read(&path)?;
    let mut reader = EsmReader::new(&bytes);
    let end = bytes.len();
    let (mut perks, mut fragments, mut matched) = (0usize, 0usize, 0usize);
    let mut shapes: BTreeMap<String, usize> = BTreeMap::new();
    walk(&mut reader, end, &mut |form_id, subs| {
        let Some((_, vmad)) = subs.iter().find(|(t, _)| t == b"VMAD") else {
            return;
        };
        let frags = parse_indexed_fragments(vmad);
        if frags.is_empty() {
            return;
        }
        perks += 1;
        // Walk the entries: PRKE opens one; EPFT/EPFB/EPF3 follow.
        let mut entry_point = None;
        let mut entries: Vec<(u32, u8, u16, u16)> = Vec::new(); // (point, epft, epfb, flags)
        let (mut epft, mut epfb, mut epf3) = (0u8, None::<u16>, 0u16);
        let mut open = false;
        let flush = |entry_point: Option<u32>, epft: u8, epfb: Option<u16>, epf3: u16, entries: &mut Vec<(u32, u8, u16, u16)>| {
            if let (Some(point), Some(id)) = (entry_point, epfb) {
                entries.push((point, epft, id, epf3));
            }
        };
        for (tag, data) in subs {
            match tag {
                b"PRKE" => {
                    if open {
                        flush(entry_point, epft, epfb, epf3, &mut entries);
                    }
                    open = true;
                    (epft, epfb, epf3) = (0, None, 0);
                    // PRKE: u8 type, u8 rank, u8 priority; the entry point sits in DATA.
                    entry_point = None;
                }
                b"DATA" if open && data.len() >= 2 => entry_point = Some(u32::from(data[0])),
                b"EPFT" if !data.is_empty() => epft = data[0],
                b"EPFB" if data.len() >= 2 => epfb = Some(u16::from_le_bytes([data[0], data[1]])),
                b"EPF3" if data.len() >= 2 => epf3 = u16::from_le_bytes([data[0], data[1]]),
                _ => {}
            }
        }
        if open {
            flush(entry_point, epft, epfb, epf3, &mut entries);
        }
        for fragment in &frags {
            fragments += 1;
            if let Some((point, epft, _, flags)) = entries.iter().find(|e| u32::from(e.2) == fragment.index) {
                matched += 1;
                *shapes.entry(format!("entry point {point}, EPFT {epft}, flags {flags:#x}")).or_default() += 1;
            } else if std::env::var("SHOW").is_ok() {
                println!("{form_id:08X}: fragment {} matches no EPFB (entries {entries:?})", fragment.index);
            }
        }
    });
    println!("{perks} perks with fragments, {fragments} fragments, {matched} matching an entry's EPFB");
    for (shape, n) in shapes {
        println!("  {n:4}  {shape}");
    }
    Ok(())
}
