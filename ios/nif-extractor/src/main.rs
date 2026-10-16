//! Extract exactly one NIF from a locally owned Fallout New Vegas Meshes BSA.
//! Game data is never uploaded or checked into the public repository.
use byroredux_bsa::BsaArchive;
use std::env;
use std::fs;
use std::path::Path;

fn choose<'a>(paths: &'a [&'a str]) -> Option<&'a str> {
    let mut paths: Vec<&str> = paths.iter().copied().filter(|p| p.ends_with(".nif")).collect();
    paths.sort_unstable();
    for needle in ["beerbottle01.nif", "beerbottle", "meshes\\clutter\\food\\", "meshes\\clutter\\"] {
        if let Some(&name) = paths.iter().find(|s| s.contains(needle)) { return Some(name); }
    }
    paths.first().copied()
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!("Usage: {} <Fallout - Meshes.bsa> <output.nif>\n\
             Example: .\\fnv-ios-nif-extractor.exe \"C:\\Program Files (x86)\\Steam\\steamapps\\common\\Fallout New Vegas\\Data\\Fallout - Meshes.bsa\" \"$env:USERPROFILE\\Desktop\\fnv-preview.nif\"",
            args.first().map(String::as_str).unwrap_or("fnv-ios-nif-extractor")));
    }
    let output = Path::new(&args[2]);
    if output.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("nif")) != Some(true) {
        return Err("Output must end in .nif".to_string());
    }
    if output.exists() { return Err("Output already exists; use a different filename to avoid overwriting data".to_string()); }
    let archive = BsaArchive::open(&args[1]).map_err(|e| format!("Cannot open BSA: {e}"))?;
    if archive.version() != 104 { return Err(format!("Expected FNV BSA version 104, got {}", archive.version())); }
    let paths = archive.list_files();
    let path = choose(&paths).ok_or("No NIF mesh in this archive")?;
    let bytes = archive.extract(path).map_err(|e| format!("Failed extracting {path}: {e}"))?;
    if bytes.is_empty() || bytes.len() > 24 * 1024 * 1024 {
        return Err("Selected NIF outside safe 1-byte–24 MiB size bounds".to_string());
    }
    fs::write(output, bytes).map_err(|e| format!("Could not write output file: {e}"))?;
    println!("Extracted genuine FNV asset: {path}\nSaved locally to: {}", output.display());
    println!("Transfer only this .nif to the iPhone Files app, not the full BSA archive.");
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
    fn rejects_non_nifs() {
        assert!(choose(&["meshes\\a.dds", "textures\\bar.dds"]).is_none());
    }
}
