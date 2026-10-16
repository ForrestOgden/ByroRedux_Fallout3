# Phase 10 — Independent native Metal scene objects

## Previously verified
The user imported `fnv-auto.fnvpack` on their physical iPhone (A19 Pro) and supplied a 17-second recording. It shows a rotating textured New Vegas bottle; the HUD displays `Preview source: fnv-auto.fnvpack`, a 128×128 DDS, and an authored texture matched automatically. Phase 9 single-package import therefore passed this narrow hardware test.

## What's new
This milestone permits rendering up to **four different local New Vegas asset packages simultaneously** in one native Metal view. Each asset is loaded and parsed through the existing native Rust bridge, copied into Swift-owned memory and uploaded as its own GPU vertex buffer and sRGB Metal DDS texture. The GPU submits an independent draw call for each model, with a shared depth buffer and camera.

The UI adds **Add another FNV asset package to scene**, plus scene count and a Clear button. Previously working single NIF, manual DDS, and combined package import paths remain available. All asset bytes stay on the user's phone.

Models are normalized individually by the original ByroRedux preview bridge, then **placed side-by-side synthetically in a gallery**. These are not gameplay world coordinates and are not read from `FalloutNV.esm`. This is an important multi-object/texture proof, not a reconstructed New Vegas cell.

## Manual test (no new Windows extraction needed)
1. Download the latest Phase 10 IPA from the successful [iOS ARM64 GitHub Actions workflow](https://github.com/ForrestOgden/ByroRedux_Fallout3/actions/workflows/ios-arm64-host.yml), extract it, and sign/install using the same iLoader setup.
2. Confirm the heading says **Native multi-object Metal preview — Phase 10**.
3. Tap **Import automatically matched FNV asset package** and select the existing `fnv-auto.fnvpack` from Files.
4. Tap **Add another FNV asset package to scene** and select **the same `fnv-auto.fnvpack` again**. This first proves two independently owned GPU meshes/texture resources without making the user extract another asset.
5. The screen should say **Scene objects: 2 / 4** and display two separate textured bottles side-by-side. Rotate/zoom and inspect for overlap or corrupt textures.
6. Optionally, create a second package on Windows using the earlier `--package` command with a different output filename and optional model substring filter. The Windows packer can only match an authored diffuse path in the supplied BSAs; selection might fail for some models. This is not required for initial Phase 10 acceptance.
7. Record a 10-second iPhone screen recording showing two models simultaneously with the on-screen count.

## Explicit limitations
- This is **not actual game scene placement**. Object positions are chosen for a gallery. No ESM cell references, AI, physics, animations, world navigation, controls, combat or quest logic.
- Each `.fnvpack` still has one diffuse DDS; different submeshes within one NIF can author more than one material, not yet supported.
- Up to 4 objects (1 primary + 3 extras). Each NIF `<=24 MiB`, each DDS `<=48 MiB`, decoded texture dimensions `<=2048`, each imported NIF `<=180,000` vertices. Real memory and frame-rate profiling has not been done.
- Rust ↔ Swift ↔ Metal vertex ABI remains **48 bytes**. The original working rendering path is used for each object, with separate buffer/texture bindings and draw calls.
- No Bethesda assets are bundled into code, CI, or GitHub artifacts.

## Exit gates
- [ ] iPhone source-only/Rust bridge/ARM64/Xcode CI passes and uploads the unsigned IPA
- [ ] Physical device two-asset render test passes
- [ ] Independent second model/texture pair tested in a subsequent iteration
