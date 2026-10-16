# Native Fallout: New Vegas mesh import (first real data milestone)

This milestone uses **ByroRedux's own `byroredux-nif::parse_nif` and `import_nif_scene` on iPhone ARM64**. It is not a texture/lighting/world/physics implementation.

## Building
- The iOS bridge links `byroredux-nif`, `byroredux-core`, `byroredux-bsa`, and `glam`.
- `fnv_ios_load_nif_mesh` parses a **local** user-selected NIF (max 24 MiB), walks its parent node hierarchy, combines rigid transforms, expands triangles, normalizes bounds, and exposes a buffer of `FNVVertex` (32 bytes).
- `fnv_ios_release_mesh` frees the returned buffer. Swift copies it first, so the Metal GPU never reads freed Rust memory.
- The Metal preview retains a fallback spinning cube and draws user-imported NIF triangles after a successful file selection. No Bethesda content is bundled, uploaded, or logged.

## Preparing one legally owned PC New Vegas NIF on Windows
1. Download `FalloutNV-local-NIF-extractor-Windows` from the [successful Windows extractor CI](https://github.com/ForrestOgden/ByroRedux_Fallout3/actions/workflows/fnv-windows-extractor.yml), unzip it.
2. In PowerShell, from the folder containing `falloutnv-ios-nif-extractor.exe`, run:
   ```powershell
   .\falloutnv-ios-nif-extractor.exe "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Meshes.bsa" "$env:USERPROFILE\Desktop\fnv-preview.nif"
   ```
   Replace the example Steam library path only if installed elsewhere.
3. The extractor uses ByroRedux's BSA v104 reader and chooses a real NIF, preferring a beer bottle asset when present. It creates one `.nif` on Desktop, **never modifies the BSA**, and refuses to overwrite an existing output.
4. Transfer just `fnv-preview.nif` to the iPhone Files app using iCloud Drive, a supported file transfer, or AirDrop.
5. Sign/install the new **unsigned** iPhone diagnostic IPA using your existing iLoader workflow. Open the app and select **Import Fallout New Vegas .nif from Files**. Screenshots should show the selected filename and triangle count.

The user must **not** upload original game files to this public GitHub fork. Only share screenshots and diagnostic messages.

## Validation gates
- [ ] Cross-platform Rust bridge tests pass.
- [ ] Rust static library cross-compiles to `aarch64-apple-ios`.
- [ ] iPhone Swift/Metal application builds and GitHub Actions uploads unsigned IPA.
- [ ] Windows extractor tests pass and its executable is uploaded.
- [ ] Actual user NIF extracted and selected on physical iPhone.
- [ ] Nonempty real NIF geometry renders with valid depth/winding and no crash.

**Known gaps:** rigid static-mesh preview only. Skinned meshes, actual source textures/UVs, alpha blending, world placement, collision, animation, and gameplay remain unimplemented. The NIF importer's simplified diagnostic coloration is not equivalent to New Vegas's materials.
