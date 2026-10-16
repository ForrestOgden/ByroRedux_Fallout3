# Phase 11 — Discover another genuine New Vegas asset

## Phase 10 hardware result
The owner supplied a 13.57-second recording of the A19 Pro displaying two textured, independently drawn New Vegas bottles in one Metal viewport, rotating and zooming without visible corruption. This passes the Phase 10 two-instance diagnostic. Both instances used the same package, so heterogeneous meshes remain unverified.

## Source-only improvement
The existing Windows extractor now has a read-only --discover-pairs mode. It searches a bounded window of at most 400 NIFs in the locally owned Fallout - Meshes.bsa, examines the source material's authored diffuse DDS reference, matches that path in Fallout - Textures.bsa, and verifies that the DDS can be decoded with the exact production Rust decoder shared with the iOS bridge. It prints up to 12 complete NIF paths and diffuse paths.

This is not an exhaustive search or proof that a complex NIF will render exactly like the original game. In particular, a NIF with several distinct diffuse materials is still presented by our single-texture-per-package viewer.

## Manual steps — only after the extractor's Windows CI succeeds
1. Download the Phase 11 Windows extractor artifact from:
   https://github.com/ForrestOgden/ByroRedux_Fallout3/actions/workflows/fnv-windows-extractor.yml
2. Extract the ZIP. In the folder containing falloutnv-ios-nif-extractor.exe, open PowerShell and run:

~~~powershell
.\falloutnv-ios-nif-extractor.exe --discover-pairs "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Meshes.bsa" "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Textures.bsa"
~~~

The flag --discover-pairs reads NIF material paths and checks DDS compatibility locally. No asset is extracted. The default search substring is clutter. Add a final word like food, furniture or weapon to change the search; the scanner only checks up to 400 matching NIF entries each time.

3. Pick a non-bottle NIF from the printed suggestions. Supply the COMPLETE NIF path printed after NIF: in the final quoted argument below. Also choose a new output filename, to protect the previously working fnv-auto.fnvpack.

~~~powershell
.\falloutnv-ios-nif-extractor.exe --package "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Meshes.bsa" "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Textures.bsa" "$env:USERPROFILE\Desktop\fnv-second.fnvpack" "REPLACE_WITH_COMPLETE_NIF_PATH"
~~~

The --package flag constructs one locally owned mesh-and-texture package, without uploading the source assets. The $env:USERPROFILE variable expands to the Windows user's home folder.

4. Transfer the new fnv-second.fnvpack to the iPhone Files app using the same local process as before. Do not post it to public GitHub.
5. KEEP THE EXISTING PHASE 10 IPA installed. Import fnv-auto.fnvpack as the main model, then use Add another FNV asset package to scene for fnv-second.fnvpack.
6. Record two different textured objects together, including the app's object count and filename labels.

## Limitations and exit criteria
- No new iPhone IPA is needed for this Windows-only helper.
- Discovery is capped at 400 examined NIFs and 12 candidates. It can report zero compatible results even where other game models are usable.
- The scanner checks DDS decompression but cannot guarantee geometry/material fidelity for arbitrary NIFs.
- The display is a test gallery, not original ESM cell positions, world streaming, AI, combat or gameplay.
- The Phase 11 manual acceptance gate is two genuinely different textured models in one iPhone Metal scene.
- GitHub only contains source and synthetic test data, never game assets.
