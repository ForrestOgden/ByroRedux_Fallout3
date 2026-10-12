# SAVE-D1-2026-10-12-01: the live-load overlay replaces a freshly imported `RigidBodyData` wholesale, wiping its `serde(skip)` `part_surface_materials`; the "re-derived at import, a snapshot never carries it" invariant does not hold on the overlay path

Labels: low,save-load,physics,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5686

Filed from `docs/audits/AUDIT_SAVE_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW. It is dormant today, and the severity rises to MEDIUM when the per-surface sound tables land.
- **Dimension**: Snapshot Completeness & the Two Lists
- **Data-Loss Class**: silent-drop (derived render/audio data, not player progress)
- **Location**:
  - `crates/core/src/ecs/components/collision.rs:134-150` (`part_surface_materials`, `#[cfg_attr(feature = "inspect", serde(skip))]`, with the justification comment).
  - `byroredux/src/save_io.rs:146` (`"RigidBodyData"` in `MUTABLE_DELTA_COLUMNS`).
  - `crates/save/src/registry.rs:157-196` (the apply closure ends in `world.insert_batch::<T, _>(remapped)`).
  - `crates/physics/src/sync.rs:1042-1054` (the consumer).
  - `byroredux/src/save_io/serde_default_guard_tests.rs` (the 2026-10-10 "refreshed WITHOUT a bump" note) and `round_trip_tests.rs:657-658` (the round-trip test writes an empty slice).
- **Status**: NEW. Closed #5550 (the leaf-alignment walker) and #5542 do not mention save, and no closed issue covers the overlay.
- **Description**:
  1. The Wave 4 follow-through added `part_surface_materials`, the per-collider-part material sequence the NIF import derives from the collision tree.
  2. It was marked `serde(skip)` on the claim that a snapshot never needs it because "the tree is re-parsed from content on load and re-derived at import" (the #3470 class). The fingerprint was refreshed without a bump on that basis.
  3. That reasoning holds for the `restore_world` path, where the value is the only source. It does not hold for the live-load path.
  4. `RigidBodyData` is an overlaid column, and `apply_deltas` replaces each matched entity's component with the deserialized row. The reload has just spawned the entity with a correct `part_surface_materials` from the re-imported NIF, and the overlay overwrites it with `Box::new([])`.
  5. The overlay runs before any collider exists. The only callers of `register_newcomers_and_refresh_queries` are `scene.rs` `select_and_spawn_player_mode` (boot-only), the transition `ground_character_body_at`, a console command and tests. Neither reload arm calls it, so colliders register in the first scheduler tick after the drain.
  6. `physics_sync_system` then builds the colliders from the overlaid data. `collision_shape_to_parts(.., body_data.surface_material, &body_data.part_surface_materials)` sees an empty slice and gives every part the body-level material.
- **Evidence**:
  - Overlay path: `execute_pending_save_loads` → `apply_deltas(world, &registry, &snapshot, &remap, MUTABLE_DELTA_COLUMNS)` → the apply closure → `insert_batch`.
  - `insert_batch` → `insert_bulk` → `insert`, which overwrites.
  - It is the only `serde(skip)` field on a component in `MUTABLE_DELTA_COLUMNS`. The other skip fields in the tree (`quest_stages.rs`, `story_manager.rs`, `quest_alias.rs`) are resources, which are restored wholesale and are not affected the same way.
  - No test exercises it: `rigid_body_data_survives_save_load_round_trip` writes an empty slice, and nothing asserts that a non-empty value survives or is re-derived across a live load.
  - I did not run a probe. The result follows from the code path above.
- **Impact**:
  - After any in-process save load (interior or exterior), every FormID-matched collider whose NIF authored heterogeneous per-child Havok materials loses its per-part split. Footstep and impact lookups see one material for the whole body.
  - Today the per-surface sound tables are empty ("until IPDS decoding lands"), so nothing audible changes.
  - Once the tables are populated, the first load in a session silently degrades surface selection on compound statics, and a fresh boot behaves differently from a loaded game.
  - Savegame data is not lost.
- **Related**: #5550, #5542, #5574, #3470 (the skip-field precedent), Dim 1's "Replacing columns" and overlay notes.
- **Suggested Fix**:
  - Preferred: make the `RigidBodyData` apply keep the live `part_surface_materials` when the live row exists, for example a `register_*` apply hook that merges non-saved fields. Alternatively, drop `RigidBodyData` from `MUTABLE_DELTA_COLUMNS`, since the static and dynamic bodies it describes re-import identically.
  - Add a live-load test that spawns a body with a non-empty slice, overlays an identical saved row, and asserts the slice is intact.
  - Note: `crates/physics/src/sync.rs:1172` still says "the material keeps the low byte". After #5542 the user-data carries the full `u32`; the comment is stale. This is physics-owned and not filed separately.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix

