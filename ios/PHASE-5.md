# Phase 5 — Metal 3D geometry milestone

A procedural rotating cube tests native iOS Metal vertex buffers, matrix transformations, perspective projection, depth32Float, and 60 FPS render submission. No Bethesda files are embedded or rendered. This is not Fallout New Vegas gameplay.

## Acceptance

1. GitHub Actions compiles the new Swift/Metal sources and uploads an unsigned iPhone IPA.
2. Owner signs/installs it and verifies the cube rotates smoothly and back faces are correctly occluded.
3. Only after physical-device confirmation begin NIF geometry conversion and BSA asset access, with original game assets kept local.

## Device finding and fix — 2026-10-16

The first physical-device screenshots showed a scrambled triangle star instead of a cube, despite a successful Xcode/Metal build. Root cause: Swift `SIMD3<Float>` occupies 16 aligned bytes; two members yielded 32 bytes per vertex, whereas Metal `packed_float3` fields yielded 24 bytes. The GPU read successive vertices with the wrong stride and offsets.

**Fix:** two explicitly aligned `SIMD4<Float>` lanes in Swift and two native `float4` lanes in Metal; both sides use 32-byte stride. Swift verifies the vertex stride and color offset at runtime and uploads the array using explicit byte-buffer access. The on-screen label includes `stride fix v2` so a cached old app is distinguishable. Physical-device revalidation is required. Do not mark 3D depth verification passed from the older screenshots.
