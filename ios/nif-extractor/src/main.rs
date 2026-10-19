//! Extract exactly one NIF from a locally owned Fallout New Vegas Meshes BSA.
//! Game data is never uploaded or checked into the public repository.
use byroredux_bsa::BsaArchive;
use std::env;
use std::fs;
use std::path::Path;
mod package;
mod world;
const MAX_EXTRACTED_BYTES: usize = 48 * 1024 * 1024;


fn choose<'a>(paths: &'a [&'a str]) -> Option<&'a str> {
    let mut paths: Vec<&str> = paths.iter().copied().filter(|p| p.ends_with(".nif")).collect();
    paths.sort_unstable();
    for needle in ["beerbottle01.nif", "beerbottle", "meshes\\clutter\\food\\", "meshes\\clutter\\"] {
        if let Some(&name) = paths.iter().find(|s| s.contains(needle)) { return Some(name); }
    }
    paths.first().copied()
}

fn choose_texture<'a>(paths: &'a [&'a str], filter: Option<&str>) -> Option<&'a str> {
    let files: Vec<&str> = paths.iter().copied().filter(|p| {
        if !p.ends_with(".dds") { return false; }
        // Simple common suffix exclusions prevent selecting a normal/spec map
        // when the goal is an albedo (diffuse) sample.
        !["_n.dds", "_s.dds", "_m.dds", "_g.dds", "_e.dds", "_msn.dds"]
            .iter().any(|suffix| p.ends_with(suffix))
    }).collect();
    let mut files = files;
    files.sort_unstable();
    if let Some(filter) = filter {
        let filter = filter.to_ascii_lowercase();
        return files.into_iter().find(|p| p.contains(&filter));
    }
    for name in ["beerbottle01", "beerbottle", "bottle"] {
        if let Some(&match_file) = files.iter().find(|p| p.contains(name)) {
            return Some(match_file);
        }
    }
    None
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let kind = args.get(1).map(String::as_str);
    let texture_mode = kind == Some("--texture");
    // Machine-checkable handshake. An older executable does not support
    // this flag, so the v3 one-click builder refuses to run against it.
    if kind == Some("--require-world-v3") {
        if args.len()!=2 || world::VERSION!=3 {
            return Err("This exporter does not support FNVWLD01 v3 worlds.".into());
        }
        println!("FNVWLD01 v{}",world::VERSION);
        return Ok(());
    }
    if kind == Some("--export-cell") {
        if args.len() < 6 || args.len() > 7 {
            return Err("Usage: fnv-ios-nif-extractor.exe --export-cell <FalloutNV.esm> <Meshes.bsa> <Textures.bsa> <out.fnvworld> [CELL_EDITOR_ID]".into());
        }
        return world::export_cell(&args[2], &args[3], &args[4], &args[5],
                                  args.get(6).map(String::as_str).unwrap_or("GSProspectorSaloonInterior"));
    }
    if kind == Some("--discover-pairs") {
        if args.len() < 4 || args.len() > 5 {
            return Err("Usage: fnv-ios-nif-extractor.exe --discover-pairs <Meshes.bsa> <Textures.bsa> [filename-substring]".into());
        }
        return package::discover_pairs(&args[2], &args[3], args.get(4).map(String::as_str));
    }
    if kind == Some("--package") {
        if args.len() < 5 || args.len() > 6 {
            return Err("Usage: fnv-ios-nif-extractor.exe --package <Meshes.bsa> <Textures.bsa> <output.fnvpack> [nif-name-filter]".into());
        }
        return package::build_package(&args[2], &args[3], &args[4], args.get(5).map(String::as_str));
    }
    let list_mode = kind == Some("--list-textures");
    let usage = "Usage:
  fnv-ios-nif-extractor.exe <Meshes.bsa> <out.nif>
  fnv-ios-nif-extractor.exe --texture <Textures.bsa> <out.dds> [search]
  fnv-ios-nif-extractor.exe --list-textures <Textures.bsa> [search]

--texture: extracts a diffuse texture (prefer beer bottle) with an optional case-insensitive substring filter.
--package: inspects an actual NIF material and bundles the exact referenced diffuse texture (no filename guessing).
--discover-pairs: examines up to 400 matching NIFs locally, showing up to 12 authored NIF+DDS candidates.
--export-cell: converts ESM-authored cell references and owned BSAs into a bounded native world package for offline iPhone navigation.
--list-textures: prints up to 40 matching texture paths, without extracting data.";

    if list_mode {
        if args.len() < 3 || args.len() > 4 { return Err(usage.into()); }
        let archive = BsaArchive::open(&args[2]).map_err(|e| format!("Cannot open BSA: {e}"))?;
        if archive.version() != 104 { return Err("Expected FNV BSA version 104".into()); }
        let filter = args.get(3).map(String::as_str).unwrap_or("bottle").to_ascii_lowercase();
        let mut matches: Vec<_> = archive.list_files().into_iter()
            .filter(|p| p.ends_with(".dds") && p.contains(&filter)).collect();
        matches.sort_unstable();
        for path in matches.iter().take(40) { println!("{path}"); }
        println!("Matches: {} (showing up to 40)", matches.len());
        return Ok(());
    }
    let index = if texture_mode { 2 } else { 1 };
    if args.len() < index + 2 || args.len() > index + 2 + usize::from(texture_mode) {
        return Err(usage.into());
    }
    let output = Path::new(&args[index + 1]);
    let extension = if texture_mode { "dds" } else { "nif" };
    if output.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case(extension)) != Some(true) {
        return Err(format!("Output must end in .{extension}"));
    }
    if output.exists() {
        return Err("Output already exists: choose a different file name (no overwrite)".into());
    }
    let archive = BsaArchive::open(&args[index]).map_err(|e| format!("Cannot open BSA: {e}"))?;
    if archive.version() != 104 { return Err(format!("Expected FNV BSA version 104, got {}", archive.version())); }
    let paths = archive.list_files();
    let selected = if texture_mode {
        choose_texture(&paths, args.get(index + 2).map(String::as_str))
    } else {
        choose(&paths)
    }.ok_or_else(|| {
        if texture_mode { "No matching diffuse DDS found; try --list-textures with a search term".to_string() }
        else { "No NIF mesh in this archive".to_string() }
    })?;

    // BSA reader's declared-size gate prevents inflating unexpectedly large files.
    let declared = archive.declared_size(selected).map_err(|e| format!("Invalid record: {e}"))?;
    if declared == 0 || declared > MAX_EXTRACTED_BYTES {
        return Err(format!("Selected file outside safe size cap: {declared} bytes"));
    }
    let bytes = archive.extract(selected).map_err(|e| format!("Failed extracting {selected}: {e}"))?;
    if bytes.is_empty() || bytes.len() > MAX_EXTRACTED_BYTES {
        return Err("Extracted file size outside 1-byte–48 MiB".into());
    }
    if texture_mode && !bytes.starts_with(b"DDS ") {
        return Err("Selected DDS has invalid magic".into());
    }
    // A deliberately narrow extractor; never edits original game archives.
    fs::write(output, bytes).map_err(|e| format!("Could not write output: {e}"))?;
    println!("Extracted genuine FNV asset: {selected}");
    println!("Saved locally to: {}", output.display());
    println!("Transfer only this file to the iPhone Files app, not entire archives.");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("FNV extractor: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefers_named_bottle_over_random_nif() {
        let paths = ["meshes\\clutter\\foo.nif", "meshes\\clutter\\food\\beerbottle01.nif"];
        assert_eq!(choose(&paths), Some(paths[1]));
    }
    #[test]
    fn chooses_diffuse_bottle_texture_and_excludes_normals() {
        let paths = [
            "textures\\clutter\\bottle_n.dds",
            "textures\\clutter\\beerbottle01.dds",
            "textures\\misc\\other.dds",
        ];
        assert_eq!(choose_texture(&paths, None), Some(paths[1]));
        assert_eq!(choose_texture(&paths, Some("other")), Some(paths[2]));
        assert_eq!(choose_texture(&paths, Some("absent")), None);
    }
    #[test]
    fn builder_world_format_is_v3() {
        assert_eq!(world::VERSION,3);
    }
    #[test]
    fn rejects_non_nifs() {
        assert!(choose(&["meshes\\a.dds", "textures\\bar.dds"]).is_none());
    }
}
