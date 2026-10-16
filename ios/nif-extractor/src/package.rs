//! Source-only local FNV mesh + authored diffuse-material packer.
//!
//! The tool takes two original PC BSAs and creates a private, standalone
//! FNVPK001 package. It never uploads content or modifies the BSAs.
//! The iOS app imports the package locally through Files.

use byroredux_bsa::BsaArchive;
use byroredux_core::string::StringPool;
use byroredux_nif::import::import_nif_scene;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const MAGIC: &[u8; 8] = b"FNVPK001";
const MAX_NIF: usize = 24 * 1024 * 1024;
const MAX_DDS: usize = 48 * 1024 * 1024;

/// Validate and normalize an *archive-relative* texture path. A NIF may
/// include "Textures\\" while a BSA file index may omit that archive root.
fn texture_key(path: &str) -> Option<String> {
    if path.contains('\0') { return None; }
    let mut text = path.trim().replace('/', "\\").to_ascii_lowercase();
    if text.starts_with('\\') || text.contains(':') { return None; }
    if let Some(rest) = text.strip_prefix("data\\") {
        text = rest.to_string();
    }
    if let Some(rest) = text.strip_prefix("textures\\") {
        text = rest.to_string();
    }
    let sections: Vec<_> = text.split('\\').collect();
    if sections.is_empty() || sections.iter().any(|s| s.is_empty() || *s == "." || *s == "..") {
        return None;
    }
    if !text.ends_with(".dds") { return None; }
    Some(text)
}

fn match_texture<'a>(files: &'a [&'a str], authored: &str) -> Option<&'a str> {
    let key = texture_key(authored)?;
    files.iter().copied().find(|candidate| texture_key(candidate).as_deref() == Some(key.as_str()))
}

fn preferred_nif<'a>(paths: &'a [&'a str], filter: Option<&str>) -> Option<&'a str> {
    let mut candidates: Vec<_> = paths.iter().copied().filter(|s| s.ends_with(".nif")).collect();
    candidates.sort_unstable();
    if let Some(search) = filter {
        let needle = search.to_ascii_lowercase();
        return candidates.into_iter().find(|p| p.contains(&needle));
    }
    for needle in ["beerbottle01.nif", "beerbottle", "meshes\\clutter\\food\\", "meshes\\clutter\\"] {
        if let Some(&p) = candidates.iter().find(|p| p.contains(needle)) {
            return Some(p);
        }
    }
    candidates.first().copied()
}

/// Strictly read one entry with an upper size bound before decompression.
fn extract_limited(archive: &BsaArchive, path: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
    let length = archive.declared_size(path).map_err(|e| format!("Cannot read entry size {path}: {e}"))?;
    if length == 0 || length > max_bytes {
        return Err(format!("Archive entry {path} exceeds the limit of {max_bytes} bytes"));
    }
    let bytes = archive.extract(path).map_err(|e| format!("Cannot decompress {path}: {e}"))?;
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err(format!("Decompressed file {path} exceeds size limits"));
    }
    Ok(bytes)
}

/// Return the NIF's actual first diffuse map, not a filename guess. Full
/// multi-material asset resolution will be required for full game scenes.
fn authored_diffuse(nif: &[u8]) -> Result<(String, usize), String> {
    let scene = byroredux_nif::parse_nif(nif).map_err(|e| format!("Cannot parse NIF: {e}"))?;
    if scene.truncated { return Err("NIF is truncated".into()); }
    let mut pool = StringPool::new();
    let imported = import_nif_scene(&scene, &mut pool);
    let mesh_count = imported.meshes.len();
    let path = imported.meshes.iter().find_map(|mesh| {
        mesh.material.textures.base_color.and_then(|sym| pool.resolve(sym))
    }).ok_or("No authored diffuse texture path in selected NIF")?;
    Ok((path.to_string(), mesh_count))
}

pub fn build_package(
    meshes_bsa: &str,
    textures_bsa: &str,
    destination: &str,
    filter: Option<&str>,
) -> Result<(), String> {
    let output = Path::new(destination);
    let ext_ok = output.extension().and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("fnvpack"));
    if !ext_ok { return Err("Output filename must end in .fnvpack".into()); }
    if output.exists() { return Err("Output already exists: refusing to overwrite it".into()); }
    let meshes = BsaArchive::open(meshes_bsa).map_err(|e| format!("Cannot open meshes BSA: {e}"))?;
    let textures = BsaArchive::open(textures_bsa).map_err(|e| format!("Cannot open textures BSA: {e}"))?;
    if meshes.version() != 104 || textures.version() != 104 {
        return Err("This extractor expects Fallout New Vegas BSA version 104".into());
    }
    let mesh_paths = meshes.list_files();
    let nif_path = preferred_nif(&mesh_paths, filter)
        .ok_or("No matching NIF in mesh archive")?;
    let nif = extract_limited(&meshes, nif_path, MAX_NIF)?;
    if !nif.starts_with(b"Gamebryo File Format") && !nif.starts_with(b"NetImmerse File Format") {
        return Err("Extracted mesh is not a recognized NIF header".into());
    }
    let (authored, mesh_count) = authored_diffuse(&nif)?;
    let texture_paths = textures.list_files();
    let dds_path = match_texture(&texture_paths, &authored)
        .ok_or_else(|| format!("Authored diffuse {authored:?} is not present in this Textures BSA. No guessed replacement will be used."))?;
    let dds = extract_limited(&textures, dds_path, MAX_DDS)?;
    if !dds.starts_with(b"DDS ") { return Err("Selected material texture lacks DDS magic".into()); }
    let nif_len = u32::try_from(nif.len()).map_err(|_| "NIF too large for package")?;
    let dds_len = u32::try_from(dds.len()).map_err(|_| "DDS too large for package")?;

    // Atomic-like create_new policy prevents overwriting a user's existing
    // hand-extracted assets. Delete only a partial output of this operation.
    // Acquire ownership first. If create_new fails (a concurrent process
    // created the path), never remove the other process's file.
    let mut f = OpenOptions::new().write(true).create_new(true).open(output)
        .map_err(|e| format!("Cannot create package: {e}"))?;
    let write_result = f.write_all(MAGIC)
        .and_then(|_| f.write_all(&nif_len.to_le_bytes()))
        .and_then(|_| f.write_all(&dds_len.to_le_bytes()))
        .and_then(|_| f.write_all(&nif))
        .and_then(|_| f.write_all(&dds))
        .and_then(|_| f.flush());
    drop(f);
    if let Err(error) = write_result {
        // Only this process's successfully opened output can be removed.
        let _ = fs::remove_file(output);
        return Err(format!("Failed to write package: {error}"));
    }
    println!("NIF: {nif_path} ({mesh_count} imported submeshes)");
    println!("Authored diffuse reference: {authored}");
    println!("Matched archive DDS: {dds_path}");
    println!("Local package created: {}", output.display());
    println!("Copy ONLY this package to your iPhone Files app, not GitHub.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn texture_paths_resolve_with_or_without_bsa_root() {
        let paths = ["clutter\\food\\beerbottle01.dds", "textures\\clutter\\other.dds"];
        assert_eq!(match_texture(&paths, "Textures/Clutter/Food/BeerBottle01.DDS"), Some(paths[0]));
        assert_eq!(match_texture(&paths, "data\\textures\\clutter\\other.dds"), Some(paths[1]));
    }
    #[test]
    fn paths_reject_traversal_and_absolute_references() {
        for path in ["../private.dds", "\\textures\\foo.dds", "C:\\data\\textures\\foo.dds", "foo\\\\bar.dds", "foo\\..\\bar.dds", "foo.nif"] {
            assert!(texture_key(path).is_none(), "Should reject {path}");
        }
    }
    #[test]
    fn avoids_guessed_materials() {
        let paths = ["textures\\clutter\\glass01.dds", "textures\\clutter\\glass02.dds"];
        assert_eq!(match_texture(&paths, "textures\\clutter\\beerbottle01.dds"), None);
    }
    #[test]
    fn picks_specific_mesh_when_filtered() {
        let paths = ["meshes\\clutter\\a.nif", "meshes\\clutter\\beerbottle01.nif"];
        assert_eq!(preferred_nif(&paths, None), Some(paths[1]));
        assert_eq!(preferred_nif(&paths, Some("a.nif")), Some(paths[0]));
        assert_eq!(preferred_nif(&paths, Some("not-here")), None);
    }
}
