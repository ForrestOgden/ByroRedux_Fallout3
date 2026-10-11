# #5542: PHYS-D1-2026-10-11-02: Skyrim's CRC32 `SkyrimHavokMaterial` is truncated `as u8`, so 8 material pairs collide and the byte cannot be joined to MATT

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5542
- **Labels**: medium,physics,nifal,audio,bug,game:skyrim
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: MEDIUM
- **Dimension**: Shape Translation (surface-material plumbing)
- **Location**: `crates/nif/src/import/collision/mod.rs:455-456`, `:465`. Contract doc at `crates/core/src/ecs/components/collision.rs:118-126`. Consumers: `byroredux/src/components.rs:1828-1831`, `:1876-1879`.
- **Status**: NEW
- **Description**: The core field doc says the per-game enums "share the STONE-is-0 start and fit a byte" (collision.rs:119-121). That holds for Oblivion (0-31) and FO3/FNV (0-127). It is false for Skyrim. `SkyrimHavokMaterial` is "CRC32 of the lowercase of the Creation Kit Material Name" (nif.xml:576-577), and 0 there is `SKY_HAV_MAT_NONE` while `SKY_HAV_MAT_STONE` is 3741512247. The import keeps only the low byte (`as u8`, mod.rs:456 and :465) of every Skyrim primitive's `material` and every `BhkCompressedMeshShapeData::chunk_materials[i][0]` (8,188 SSE compressed-mesh blocks).
- **Evidence**: Low byte of the 77 nif.xml Skyrim values. Eight pairs collide:
  - HEAVY_METAL / CLOTH → 163
  - BROKEN_STONE / HEAVY_WOOD → 71
  - GRAVEL / MATERIAL_BOTTLE_SMALL → 88
  - BARREL / MATERIAL_BONE → 20
  - CERAMIC_MEDIUM / STAIRS_BROKEN_STONE → 91
  - MATERIAL_CHAIN / SKIN_METAL_SMALL → 102
  - SKIN_SMALL / COIN → 62
  - AXE_1HAND / SKIN_METAL_LARGE → 203

  `SKY_HAV_MAT_STONE` lands on byte 55 and `SKY_HAV_MAT_DIRT` on 170. These are arbitrary values that overlap the Oblivion/FO3 enum ranges the `u8`-keyed maps are documented against.
- **Trigger Conditions**: any Skyrim (LE/SE) body with authored Havok material, which is effectively all Skyrim collision.
- **Impact**: The authored identity is lost at import. A Skyrim consumer resolves surfaces through MATT records keyed by the u32 Havok ID, and a byte cannot be joined back to MATT. The byte also merges metal with cloth and gravel with bottles. `RigidBodyData::surface_material: u8` is a saved column (FORMAT_MAJOR v35 → v36 was already spent on 886edcded), so widening it later costs another format bump. Nothing is audible today (maps empty).
- **Related**: PHYS-D1-2026-10-11-03. Independently found by Dim 3 as its D3-05, from the PHYSAL-doctrine side: the raw per-game Havok material crosses the parse→canonical boundary untranslated, a per-game seam `docs/engine/physal.md` §3 forbids. Merged here.
- **Suggested Fix**: Carry the material as `u32` end to end: `RigidBodyData::surface_material: u32`, `part_surface_materials: Box<[u32]>`, `user_data = u128::from(u32)`, readers returning `u32`. Alternatively, translate to a canonical engine surface enum at the NIFAL boundary, per game. Either way, correct the collision.rs:118-126 doc.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **CANONICAL-BOUNDARY**: Per-game Havok data is translated at the NIF parse→canonical boundary (`crates/nif/src/import/collision/`), never branched on in `crates/physics/` or the solver (PHYSAL `docs/engine/physal.md` §3)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
