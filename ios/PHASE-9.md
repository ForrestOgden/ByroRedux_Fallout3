# Phase 9 — Automatic New Vegas Material Pairing

**Verified starting point (2026-10-16):** Owner's physical A19 Pro screen recording shows the native ByroRedux/Metal bottle viewer loading `fnv-preview.nif` and `fnv-bottle.dds` with coherent texture sampling and orbit/zoom, no visible geometry tears or crashes. This is a successful *one-model* proof, not a playable Fallout: New Vegas port.

## Purpose

The Phase 8 viewer forces you to select `.nif` and `.dds` separately, and its Windows tool guessed a beer-bottle texture by filename. That does not scale beyond a single hand-matched demo.

Phase 9 builds **one private, local asset package** using the real `ImportedMesh::material.textures.base_color` resolved through ByroRedux's `StringPool`. The Windows extractor looks up that **exact authored DDS path** in the user's local `Fallout - Textures.bsa`. It never silently guesses or chooses a similarly named but unrelated texture. It bundles original NIF bytes and DDS bytes into one `.fnvpack` file.

The iPhone app reads that one package from Files, validates its magic, byte lengths and size caps, extracts the two assets to temporary local files, calls the existing native Rust NIF and DDS parsers, copies the resulting buffers, and deletes temporary files. The existing proven 48-byte Rust ↔ Swift ↔ Metal vertex ABI and sRGB render target remain unchanged.

## Windows manual setup

1. Obtain the successful `FalloutNV-local-NIF-extractor-Windows` artifact from [Phase 9 extractor CI](https://github.com/ForrestOgden/ByroRedux_Fallout3/actions/workflows/fnv-windows-extractor.yml) and unzip it.
2. Open PowerShell in the folder containing `falloutnv-ios-nif-extractor.exe`. With default Steam installation paths, run:

```powershell
.\falloutnv-ios-nif-extractor.exe --package "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Meshes.bsa" "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Textures.bsa" "$env:USERPROFILE\Desktop\fnv-auto.fnvpack"
```

- `--package`: inspect the selected NIF material, identify its authored diffuse DDS, and pack both together. Optional final argument filters the NIF's archive filename (e.g., `beerbottle`).
- The first archive argument is the locally installed base meshes BSA; the second is the local base textures BSA.
- The last argument is an output path. The tool refuses to overwrite existing packages. If your Steam library resides elsewhere, substitute the two BSA paths.
- If no authored diffuse DDS is found in the supplied archive, the tool stops with an error. This does **not** mean the game files are broken; the DDS may reside in another archive or be missing from the NIF. Do not replace it with a guessed DDS.

3. Transfer `fnv-auto.fnvpack` to the iPhone Files app (iCloud Drive or trusted transfer). **Never upload the original assets or the bundle to the public GitHub repository.**
4. Install the Phase 9 IPA through your existing iLoader workflow.
5. Open the app and choose **Import automatically matched FNV asset package**. Select your `.fnvpack` once. The app should import and render the mesh with its authored DDS, preserving manual NIF and DDS import options.

## Safety and constraints

- Entirely local asset extraction. Source-only GitHub CI uses synthetic unit tests, never Bethesda asset bytes.
- Simple binary package: eight-byte ASCII header `FNVPK001`, LE `u32` NIF size, LE `u32` DDS size, then NIF and DDS bytes. Reader strictly bounds NIF to 24 MiB and DDS to 48 MiB, and checks exact payload size. This is **not** an archival interchange format or a security boundary against a malicious file—ByroRedux's existing bounded NIF/DDS decoders remain responsible for parsing.
- The current Metal viewer applies **one diffuse DDS to the entire imported NIF**, even when there are multiple submeshes/materials. The tool warns through its printed count; true per-submesh material assignment is a later milestone.
- Geometry, material metadata, and actual texture path resolution are derived from the source NIF—not guessed by comparing similar filenames. No automatic world loading, audio, collision, quests, NPCs, scripting, saving, or gameplay.

## Build and device gates

- [ ] Windows extractor unit tests compile and pass with locked Cargo dependencies; executable artifact uploads.
- [ ] Native ARM64 Rust tests/cross-compile and SwiftUI/Metal Xcode build pass; unsigned IPA uploads.
- [ ] Source-only policy and workspace baseline pass.
- [ ] Owner generates a package from installed Steam BSAs (local only).
- [ ] Owner imports one package and shows it loading geometry and material on A19 Pro. If a picker does not show .fnvpack, report that exact issue before changing files.
