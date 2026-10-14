# Fallout 3 → Fallout: New Vegas native iOS pivot

**Date:** 2026-10-14

## Rationale
The owner already has Fallout: New Vegas installed through Steam, but the available Fallout 3 ISO requires activation. New Vegas is the supported PC content target from this commit forward. Existing ByroRedux engine architecture, original upstream commit history, prior Rust build proof, and asset-free smoke-test methodology remain useful.

## Migration rules
- Keep upstream engine files and Git history unchanged.
- Do not re-use or rename an old Fallout 3 IPA as a New Vegas binary.
- Replace archive validation with `FalloutNV.esm` and original `Fallout - Meshes.bsa` / `Fallout - Textures.bsa`.
- Recognize standard New Vegas DLC files but do not require DLC to pass base-game validation.
- Rebuild the Rust staticlib and Swift iPhone app with **New Vegas** identifiers under a new `fnv/native-ios-transition` branch. A successful CI build is not device or gameplay verification.
- Retire the superseded Fallout 3 pull requests after the replacement New Vegas pull request is established.
- GitHub repository display name still says `ByroRedux_Fallout3` until the owner renames it in GitHub Settings → General → Repository name. The connected GitHub integration cannot change that setting.

## First New Vegas-specific runtime target
ByroRedux's documented reference scene is **GSProspectorSaloonInterior** in Goodsprings. Original New Vegas assets live under the Steam game's Data folder. Graphics porting remains unproven; renderer still requires Vulkan ray queries and cannot simply be compiled to iOS.

## Integrity rules
Do not commit or upload Bethesda game data, DLC, executables, installers, Steam credentials, activation keys, or signing certificates. All game data tests are local; the public repository contains only tooling and source.
