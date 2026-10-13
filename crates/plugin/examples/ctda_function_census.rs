//! Census every `CTDA` in a master by function index — how often each
//! condition function is authored, by record type and Run-On — so the
//! condition catalog grows by measured frequency.
//!
//! Usage:
//!   cargo run --release -p byroredux-plugin --example ctda_function_census -- <ESM> [top N]

use std::collections::BTreeMap;

use byroredux_plugin::esm::reader::EsmReader;

/// Per-function census: total CTDAs, then splits by record type and Run-On.
type FunctionCensus = (usize, BTreeMap<String, usize>, BTreeMap<u32, usize>);

fn walk(
    reader: &mut EsmReader<'_>,
    end: usize,
    counts: &mut BTreeMap<u32, FunctionCensus>,
) -> anyhow::Result<()> {
    while reader.position() < end && reader.remaining() > 0 {
        if reader.is_group() {
            let group = reader.read_group_header()?;
            let inner_end = reader.group_content_end(&group);
            walk(reader, inner_end, counts)?;
            continue;
        }
        let header = reader.read_record_header()?;
        let subs = reader.read_sub_records(&header)?;
        let kind = String::from_utf8_lossy(&header.record_type).to_string();
        for sub in subs.iter().filter(|s| &s.sub_type == b"CTDA" && s.data.len() >= 24) {
            let function = u32::from_le_bytes(sub.data[8..12].try_into().unwrap());
            let run_on = u32::from_le_bytes(sub.data[20..24].try_into().unwrap());
            let entry = counts.entry(function).or_default();
            entry.0 += 1;
            *entry.1.entry(kind.clone()).or_default() += 1;
            *entry.2.entry(run_on).or_default() += 1;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: ctda_function_census <ESM> [top N]");
    let top: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(40);
    let bytes = std::fs::read(&path)?;
    let mut reader = EsmReader::new(&bytes);
    let _header = reader.read_file_header()?;
    let mut counts = BTreeMap::new();
    walk(&mut reader, bytes.len(), &mut counts)?;
    let total: usize = counts.values().map(|c| c.0).sum();
    let mut ranked: Vec<_> = counts.into_iter().collect();
    ranked.sort_by_key(|(_, c)| std::cmp::Reverse(c.0));
    println!("=== {path}: {total} CTDAs, {} functions", ranked.len());
    for (function, (count, kinds, run_ons)) in ranked.into_iter().take(top) {
        let mut kinds: Vec<_> = kinds.into_iter().collect();
        kinds.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        kinds.truncate(4);
        println!("  {function:>4} {count:>6}  run_on {run_ons:?}  {kinds:?}");
    }
    Ok(())
}
