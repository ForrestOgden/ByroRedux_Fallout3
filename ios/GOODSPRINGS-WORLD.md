# Native Goodsprings: real New Vegas cell loading and movement

The user requested that all effort shift from asset showcases toward a standalone native game. This milestone is the first authentic world loading and first-person navigation step, **not a claim that the full game works**.

## Genuine data / retained provenance

- Windows offline exporter reads FalloutNV.esm and resolves GSProspectorSaloonInterior CELL and its authored REFR references, base MODL paths, mesh NIFs and diffuse DDS files from the owner's original Steam BSAs.
- No fictitious world positions, placeholder houses, prearranged gallery copies, or shipped licensed assets.
- REFR transforms are converted from Bethesda Z-up to Y-up with ByroRedux coordinate helpers. The real game's COCMarkerHeading is used as origin when present; fallback spawn methods are explicitly reported.
- Unique mesh sources are stored once, with independent REFR instances. The first world scene caps output at 160 unique models, 480 placements, and 192 MiB compressed source content. Counts and omitted models are logged. These limits mean the output is NOT a full interior guarantee.
- Native iOS Rust NIF import now has a separate unnormalized geometry entrypoint; it does not break the verified older normalized viewer. Native Swift imports a bounded FNVWLD01 package, builds Metal buffers and textures, applies authentic placements and renders them under a first-person Y-up camera.
- A virtual movement stick and drag-to-look move the camera at 2.5m/s. No USB or streaming while it runs on iPhone after installation.

## Critical missing parts

Movement is free walking with a fixed camera eye height. It can pass through walls. Physics/collision, NPCs, true interactable doors, dialogue, quests, combat, inventory, audio, weather, terrain and full exterior Goodsprings streaming, saves, DLC and performance validation remain open. Do not call this a full game or fully playable yet.

## Windows one-time preparation after successful CI

Download the Windows executable from the successful GitHub Windows extractor workflow on this branch. In PowerShell opened in that executable's folder run:

~~~powershell
.\falloutnv-ios-nif-extractor.exe --export-cell "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\FalloutNV.esm" "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Meshes.bsa" "C:\Program Files (x86)\Steam\steamapps\common\Fallout New Vegas\Data\Fallout - Textures.bsa" "$env:USERPROFILE\Desktop\GoodspringsSaloon.fnvworld" "GSProspectorSaloonInterior"
~~~

Dot-backslash runs a local executable; --export-cell tells it to load the real ESM cell instead of choosing a model file. The three paths are the installed ESM and mesh/texture archives. The environment variable USERPROFILE resolves the Windows user home. The last argument is the original Goodsprings Prospector Saloon interior CELL EditorID.

The produced GoodspringsSaloon.fnvworld file is kept PRIVATE. Transfer it to iPhone Files and open the new app's Load Goodsprings .fnvworld button. No PC connection is needed during play. The unsigned app needs iLoader or another legal signing/distribution path, with periodic renewal for free Apple Account signing.

## Next engineering priorities (not yet delivered)

1. Full spatially streamed geometry and original lighting/materials.
2. Havok NIF collider import and real gravity/wall/floor player movement.
3. Goodsprings exterior LAND/WRLD streaming and doors between cells.
4. Gameplay controller/HUD, native gamepad input and interactions.
5. Port existing ByroRedux ECS, audio, script, NPC, dialogue, saves.
6. Full quest/campaign/base-game and DLC compatibility, optimizations.

The upstream Vulkan desktop application has relevant reusable systems but cannot simply be dropped into a Metal iPhone application. A consistent native runtime remains substantial work.

## Spawn orientation / world wire protocol

The Windows generator now writes **FNVWLD01 version 2**, including the first-person starting yaw computed from the authored `COCMarkerHeading` REFR rotation. The iPhone loader enforces version 2 and the renderer initializes the native camera to the game's authored facing rather than fixed north. This intentionally requires the matching Windows exporter and IPA from the same build. The previous version 1 world format was never released for a user device test; do not mix these versions.


## Phase 13 — source-material-preserving interior (2026-10-16)

The physical iPhone recording showed recognizable Prospector Saloon geometry with large intrusive white polygons and incorrect gray/textured wall surfaces. Root cause of each polygon is not proven from video alone; the earlier renderer assigned one DDS to an entire NIF, irrespective of its constituent shapes.

- Windows exporter now writes **FNVWLD01 v3**. Per source NIF it writes the full NIF and one optional authored diffuse DDS **for each ByroRedux ImportedMesh ordinal**, including empty entries so indices never shift. It supports a bounded 256 material slots per model, looks in owned Textures and Textures2 archives, and does not guess names.
- Rust ARM64 adds `fnv_ios_load_nif_mesh_world`, returning the existing 48-byte vertices plus explicit, contiguous per-shape triangle ranges. This is a new FFI entrypoint; the single-mesh viewer ABI and workflows remain intact.
- Swift validates batch boundaries and requires v3 material count to match the same native parser's submesh count. Metal now binds each batch's own DDS or uses local untextured diagnostic shading if missing/unsupported.
- The bridge rejects degenerate triangles and triangles with edges exceeding 8192 original world units; emits diagnostics of skipped triangles. This is an intentionally conservative safety check, **not** proof every white spike is fixed.
- Device accepts existing v2 `.fnvworld` packages for compatibility, but they retain the old single-texture behavior. **To test the v3 improvements, rebuild GoodspringsSaloon.fnvworld with the matching new Windows executable**, transfer it to the iPhone and select **Load different location**; the app caches the successful replacement.
- Full original shader semantics (normal/specular maps, alpha handling, transparency, vertex materials and lighting), per-texture mipmaps and draw-call optimization are not yet finished. Collision, gravity, NPCs, exteriors and quest scripting also remain unimplemented.

Acceptance gates: native Rust unit tests; matching Windows optimized exporter and unsigned iPhone IPA CI; source-only assets; **new owner device recording on v3** to establish whether the white geometry artifacts are actually resolved. Do not represent CI build success as a physically verified visual fix.
