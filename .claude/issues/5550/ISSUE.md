# #5550: PHYS-D1-2026-10-11-04: `part_surface_materials` is built by a second hand-maintained walker that drifts from `resolve_shape`'s leaf order for multi-spheres and None-resolving children

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5550
- **Labels**: low,physics,nifal,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Shape Translation (surface-material plumbing)
- **Location**:
  - `crates/nif/src/import/collision/shape.rs:141-147` (`leaf_material!(BhkMultiSphereShape)` pushes one entry) vs `:257-283` (resolves to k `Ball` parts).
  - `:164-177` (List/ConvexList materials recurse every child) vs `:364-405` (resolve drops children that return None).
  - `:214` (unmatched block pushes one entry) vs `:249-251` and `:470-485` (resolve returns None).
  - `:95-97` (cycle pushes an entry) vs `:48-54` (resolve drops it).
- **Status**: NEW
- **Description**: `RigidBodyData::part_surface_materials` must be index-aligned with `collision_shape_to_parts`' part order. The converter consumes it with a running cursor (convert.rs:170-189), and that cursor is correct only if the producer's sequence matches part order. The producer is a separate traversal that re-implements `resolve_shape`'s dispatch, and it disagrees in four branches:
  1. A `bhkMultiSphereShape` with k ≥ 2 finite spheres becomes k Ball parts but contributes one material entry.
  2. A List/ConvexList child that resolves to None contributes an entry but no part. That covers plane, phantom, unsupported, non-finite primitive, empty `finish_trimesh`, failed transform, and missing packed/compressed data.
  3. A cycled child contributes an entry but no part.
  4. A multi-sphere with 0 finite spheres contributes an entry but no part.

  After the first drift every later part takes its neighbour's material. Parts past the sequence end take the body default. For example, List[MultiSphere(2, WOOD), Box(METAL)] yields parts [Ball, Ball, Cuboid] against materials [WOOD, METAL], so it registers WOOD, METAL, WOOD (the default). The metal box reports wood.
- **Trigger Conditions**: a compound body containing one of the drift branches followed by at least one sibling with a different material. Vanilla exposure is tiny: 1 Oblivion multi-sphere and 1 SSE plane in the census. Otherwise only corrupt or modded content triggers it.
- **Impact**: The wrong per-part surface byte is assigned silently. No test can see it: the converter test hand-writes the slice, and the nif test fixture has only List/Sphere/Box. Low vanilla incidence keeps this LOW.
- **Related**: -01 (another way the two walkers disagree with the parser).
- **Suggested Fix**: Produce the material alongside the shape in the same walk. Have `resolve_shape` return each leaf's material, either in the `CollisionShape` leaf or as a parallel `Vec` pushed at the exact sites that emit leaves, so alignment holds by construction. At minimum, `debug_assert_eq!` the leaf count against a leaf count of the resolved `CollisionShape` in `extract_from_classic`, and add a multi-sphere-plus-sibling fixture.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **CANONICAL-BOUNDARY**: Per-game Havok data is translated at the NIF parse→canonical boundary (`crates/nif/src/import/collision/`), never branched on in `crates/physics/` or the solver (PHYSAL `docs/engine/physal.md` §3)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
