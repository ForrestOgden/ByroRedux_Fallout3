# Phase 5 — Metal 3D geometry milestone

A procedural rotating cube tests native iOS Metal vertex buffers, matrix transformations, perspective projection, depth32Float, and 60 FPS render submission. No Bethesda files are embedded or rendered. This is not Fallout New Vegas gameplay.

## Acceptance

1. GitHub Actions compiles the new Swift/Metal sources and uploads an unsigned iPhone IPA.
2. Owner signs/installs it and verifies the cube rotates smoothly and back faces are correctly occluded.
3. Only after physical-device confirmation begin NIF geometry conversion and BSA asset access, with original game assets kept local.
