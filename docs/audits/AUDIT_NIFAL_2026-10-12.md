**HEAD**: 5d31fd637 · **Baseline**: docs/audits/AUDIT_NIFAL_2026-10-11.md (HEAD c6e48cbc6) · **Audited**: Dims 1, 3, 6, 7, 8, 9 (delta-scoped; the 10-11 baseline's own findings re-verified as fixed in code) · **Unchanged since baseline (skimmed)**: Dim 2 (mesh/coord/transform/units/rotation), Dim 4 (`import/types.rs`), Dim 5 (`systems/particle.rs`) — first steps run

# NIFAL Audit — 2026-10-12

One auditor, all nine dimensions, solo. Scratch notes: `/tmp/audit/nifal/run_2026-10-12/dim_{1..9}.md`.
Phase 1 guards: `cargo test -p byroredux-nif` green (1,415 lib tests plus the integration binaries; the `translation_completeness` integration test ran separately, below).
`cargo test -p byroredux --bin byroredux` filters `material_translate` (77), `canonical_animation_completeness_harness`, `every_hkx_sample_field`,
`every_exterior_spawner`, `every_source_derived` all green.

Delta since the baseline (`c6e48cbc6..HEAD`) touching NIFAL paths, by dimension:

| Dim | Commits |
|---|---|
| 1 / 8 | 0d698d16c (CDB alpha test requires an authored `HasOpacity = true`, #5661 — issue still open), c961653f4 (`resolve_through_cdb`, #5284), dbfae5a34 / d84e7f163 / 661be4eaa / 9fc8ee021 (docs only) |
| 3 | c1a99c25e (`AttachLight` placement: new `import/attach_light.rs` + `mesh_instance.rs`), 656e2bdc3 (#5524 skin decline tests + doc move), cce3d59b5 (`bytes::le_*_at` readers in `tangent.rs` / `sse_recon.rs` / `decode.rs`) |
| 6 | daddf52f5 (#5550), 06755dc5a (#5573), 993d52f3a (#5574) |
| 7 | 1ab2dd74d (#5571), 6fef6bd57 (#5572), 356a74b0f (#5575), 9cfa39fe8 (#5576), a076ca7fb (#5577), 9707cb367 (#5662, HKX) |

## Executive Summary

**New findings: 3 LOW, all doc-rot (no CRITICAL, HIGH or MEDIUM).** One finding is carried forward unchanged: **Existing #5502** (the Starfield
texture-set-slot half, HIGH by rubric, still unfixed — `collect_slots` at `crates/sfmaterial/src/index.rs:349` still reads only the named texture-set object).

Every finding from the 2026-10-11 report was published as #5571–#5579 and its fix is present in code. Each was re-read here:

- **D7-01 → #5571**: `NiPathInterpolator` now composes `path(percent(t))` using the core interpolators, resamples under the B-spline bounds, leaves rotation and scale empty, and skips a path with no percent data.
- **D7-02 → #5572**: `BsplineWindow::sample` maps `u` over the interpolator's own span. A disjoint window puts duplicate-time keys at a clamped `u`; `find_key_pair` has a `dt > 0` guard, so that is harmless.
- **D7-03 → #5575**: `usable_sequence_span` accepts only a finite, forward span.
- **D7-04 → #5576**: sampled B-spline colour keys pass the sanity gate.
- **D7-05 → #5577**: `degraded()` and `log_degraded()` are called at both entry points, `import_kf` and `import_embedded_animations`.
- **D6-01 → #5573**: the guard now slices `resolve_shape_inner` only. I re-derived the slice; it contains no other column-0 `fn` and exactly 16 `Bhk*Shape` arms.
- **D6-02 → #5574**: the raw per-game Havok id is documented as parked in `nifal.md`.
- **D8-05 → #5578** and **D8-06 → #5579** are fixed.

Also closed since the baseline and verified:

- **#5550** (one walk produces the shape and its per-leaf materials).
- **#5541 / #5549** (`body_surface_material` is the first *authored* leaf; `0` is authored STONE below `SKYRIM_LE`).

Independent review of the largest new code, with no defect found:

- **`attach_light.rs` / `spawn_mesh_instance`.**
  - The composed root→node chain starts from a default parent, exactly like `walk_node_flat`, so the root transform is included. `length_scale` is applied once and Z-up→Y-up once, in `zup_point_to_yup`.
  - The cycle and depth guards work. The light entity is parented through `add_child(placement_root)`.
  - `base_translation` is now the entity's local translation. That also repairs the old fallback path, which seeded the flicker restore point with the world `ref_pos` on a child entity.
- **`resolve_shape_into` material alignment.** `collision_shape_to_parts` emits exactly one part per non-`Compound` leaf, so the aligned sequence holds in release as well as under the `debug_assert!`. A `None` return truncates `materials`, so a skipped child skips its material entry too.
- **`resolve_through_cdb`.** Behaviour-equivalent to the four copies it replaced (the `.mat` arm records provenance, the sidecar arms do not, both trace). Its doc comment is misplaced; that is REN-D6-2026-10-12-01, not re-reported.
- **0d698d16c / #5661.** The `HasOpacity == Some(true)` gate is internally consistent (the pin is `alpha_test_requires_an_authored_has_opacity_true`). The "absent = `false`" default is census-derived and the commit says so; it is not independently sourced. Nothing else reads `has_opacity`.

Phase 1 guard run for Dim 9: `cargo test --release -p byroredux-nif --test translation_completeness -- --ignored` passed 2/2 in 64.8 s on all 8 games. Every floor and ceiling held. The harness measures the pre-merge `ImportedMaterial` tier, so it cannot see #5502's texture-set half.

| Category | This delta |
|---|---|
| Material (D1) | Converged. `material.rs`, `material_translate.rs` and `helpers.rs` unchanged. 77 `material_translate` tests and the spawner / kitchen-sink pins green; `if game` in shaders empty. No findings |
| Geometry/Transform (D2) | Only the `bytes::le_*_at` reader refactor (`unwrap_or(0.0)` is unreachable behind the `blob.len() == expected` check). No findings |
| Skinning / Lights (D3) | `AttachLight` placement reviewed clean. **1 LOW** (stale doc on the flicker base translation) |
| Nodes (D4) | First-step grep over the 7 parked fields is empty (0 canonical consumers). No findings |
| Particles (D5) | `systems/particle.rs` untouched; exactly 2 `apply_emitter_overlays` callers (`scene/nif_loader.rs:1738`, `cell_loader/spawn.rs:1276`). No findings |
| Collision (D6) | 16 resolve arms; `CollisionAuthoringSummary` unchanged; one NIF-derived `RigidBodyData` producer. **1 LOW** (stale names and wording left by #5550 / #5549) |
| Animation (D7) | NIF and HKX harnesses green; every 10-11 finding fixed. No new findings |
| Shader flags / roles / merge (D8) | Only the CDB alpha-test gate and the `resolve_through_cdb` dedup. Existing #5502 carried. No new findings |
| Completeness (D9) | Harness 2/2, floors and ceilings held. **1 LOW** (misattached doc comment in `precombined.rs`) |

Per-invariant: single-boundary 0, no-fabrication 0, no-leak 0, no-render-time-fallback 0 new violations. The three findings are doc-rot and carry no tier.

## Per-Category Tier Matrix

| Category | single-boundary | no-fabrication | no-leak | no-render-time-fallback | Boundary fn |
|---|---|---|---|---|---|
| Material | PASS | PASS (CDB `HasOpacity` default is measured, flagged by its commit) | PASS | PASS | `translate_material_with_provenance` / `translate_texture_only_material*` |
| Geometry/Transform | PASS | PASS | PASS | PASS | `import/units.rs`, `import/transform.rs`, `rotation.rs` |
| Skinning / Lights | PASS (`find_attach_light_offset` is the one resolver) | PASS | PASS | PASS | `mesh/skin.rs`, `walk/lights.rs`, `attach_light.rs` |
| Nodes | N-A (by design) | PASS | PASS | PASS | spawn sites; 7 fields parked |
| Particles | PASS | PASS | PASS | PASS | `systems/particle.rs::apply_emitter_overlays` |
| Collision | PASS (one walk now produces shape and materials) | PASS | PASS (raw per-game id documented as parked, #5574) | PASS | `import/collision/` (`resolve_shape_with_materials`) |
| Animation | PASS | PASS (path now composed, not copied) | PASS | PASS | `convert_nif_clip` + `convert_hkx_clip` |
| Shader flags / roles / merge | PARTIAL: #5525 and #4246 still open (the #5284 copies are merged) | PASS | PASS | PASS | `merge.rs::{apply_cdb_material, resolve_through_cdb}` |

## Findings

### NIFAL-D6-2026-10-12-01: #5550 / #5549 left stale names and "first non-zero" wording in the surface-material docs
- **Severity**: LOW
- **Dimension**: Collision
- **Tier Violated**: n/a (doc-rot)
- **Game Affected**: all with Havok collision
- **Location**:
  - `crates/core/src/ecs/components/collision.rs:126` ("first non-zero material") and `:143` (`shape_leaf_materials`)
  - `crates/physics/src/convert.rs:1480` and `crates/physics/src/ragdoll.rs:423` (`shape_leaf_materials`)
  - `docs/engine/physics.md:436` (`first_shape_material`)
  - `docs/engine/nifal.md:545` ("body-level first non-zero")
- **Status**: NEW
- **Description**: daddf52f5 (#5550) deleted the `shape_leaf_materials` and `first_shape_material` walkers and moved both reductions into `resolve_shape_into` / `body_surface_material`. The fix for #5549 changed the body-level rule from "first non-zero leaf" to "first *authored* leaf", where `0` is authored STONE on Oblivion / FO3 / FNV and `NONE` on Skyrim. The canonical component's own doc comment and the nifal.md "Havok surface material" paragraph (written by #5574 / 993d52f3a) still carry the old wording, and four places still name functions that no longer exist.
- **Evidence**: `grep -rn 'shape_leaf_materials\|first_shape_material'` finds no `fn` of either name, only these doc and comment mentions plus a test name (`shape.rs:1090`). `crates/nif/src/import/collision/shape.rs::body_surface_material` documents the new rule: "the first AUTHORED leaf … a `[STONE, WOOD]` body is STONE, not WOOD (#5549)".
- **Impact**: A future consumer author reading the `RigidBodyData::surface_material` doc would expect the first non-zero leaf and mis-handle a STONE-first body on Oblivion / FO3 / FNV. The canonical type is the contract, so its doc should state the real rule.
- **Related**: #5549, #5550, #5574
- **Suggested Fix**: Reword `collision.rs:126` and `nifal.md:545` to "first authored leaf (`0` is authored STONE below Skyrim)", and replace the deleted function names with `resolve_shape_with_materials` / `body_surface_material`.

### NIFAL-D3-2026-10-12-01: `attach_light_flicker_if_needed`'s doc still says `base_translation` is "captured from `ref_pos`"
- **Severity**: LOW
- **Dimension**: Skinning/Lights
- **Tier Violated**: n/a (doc-rot)
- **Game Affected**: all with flickering LIGH-with-model placements
- **Location**: `byroredux/src/cell_loader/references/attach.rs:866-868`
- **Status**: NEW (left by c1a99c25e)
- **Description**: c1a99c25e changed the `mesh_instance.rs` call to pass the light entity's local `Transform` translation (the `AttachLight` child or the first mesh entity). The `synth_child.rs` callers still pass `ref_pos`, which is correct there because that entity is an unparented root with `Transform::new(ref_pos, …)`. The function's doc still says the value is "captured from `ref_pos`", while the field's doc (`LightFlicker::base_translation`, `light.rs:185`) correctly says "un-jittered local translation".
- **Evidence**: `attach.rs:867` vs `mesh_instance.rs:1619-1629` (`light_local = world.get::<Transform>(light_entity)…`).
- **Impact**: None today. A new caller that follows the doc would seed a child entity with a world position, which was the latent bug on `mesh_instance.rs`'s old fallback path.
- **Related**: c1a99c25e
- **Suggested Fix**: Reword to "the entity's local translation (`ref_pos` only where the entity is an unparented root)".

### NIFAL-D9-2026-10-12-01: `geometry_only_cached`'s doc comment is glued onto the new `PrecombineCacheLookup` enum
- **Severity**: LOW
- **Dimension**: Completeness / cross-cutting (the `CachedNifImport` producers)
- **Tier Violated**: n/a (doc-rot)
- **Game Affected**: FO4 (precombine path)
- **Location**: `byroredux/src/cell_loader/precombined.rs:1055-1058` (the orphaned doc), `:1059-1083` (enum + `lookup_precombine_cache`), `:1085` (`geometry_only_cached`)
- **Status**: NEW (2ca9bc313, #5522). Same class as REN-D6-2026-10-12-01 (`merge.rs`).
- **Description**: #5522 inserted `PrecombineCacheLookup` and `lookup_precombine_cache` directly under the doc block "Wrap precombine-decoded meshes in a geometry-only `CachedNifImport` …", which belonged to `geometry_only_cached`. rustdoc now renders that text, followed by the enum's own, as the enum's documentation. `geometry_only_cached`, a `CachedNifImport` producer, is left without its doc.
- **Evidence**: `precombined.rs:1055-1058` runs straight into `/// What the import registry already knows about one _oc.nif path.` with no blank doc separator.
- **Impact**: Documentation only.
- **Related**: REN-D6-2026-10-12-01
- **Suggested Fix**: Move the two-line "Wrap precombine-decoded meshes …" block back above `pub(super) fn geometry_only_cached`.

### Carried: NIFAL-D8-2026-10-11-04 — Existing #5502 (texture-set slots and flat-colour replacements not `Parent`-inherited)
- **Severity**: HIGH by rubric (the issue is triaged MEDIUM); unchanged and not re-counted as new.
- **Status**: Existing: #5502 (OPEN). Re-checked: `crates/sfmaterial/src/index.rs:349` (`collect_slots`) still reads only the named texture-set object, while `collect_settings` (`:378`) walks the `Parent` chain. The 2026-10-11 measurement still stands: 2,051 of 10,158 (20.2 %) vanilla-referenced Starfield materials resolve with no texture.
- Not re-measured here, since the lookup code is unchanged. The 0d698d16c gate (#5661) changes one downstream effect this finding described: by the commit's own figure, 1,265 of the 10,152 NIF-referenced materials are no longer alpha-tested against base-colour alpha, so fewer of the texture-less ones hit the alpha-test path.

## Documented-limitation ledger (parked-not-leak — do not re-report)

- **Node/mesh passthroughs**: `bs_value_node`, `bs_ordered_node`, `tree_bones`, `range_kind`, `lod_group`, `bs_lod_cutoffs`, `bs_sub_index` — raw-tier, 0 canonical consumers (first-step grep re-run, empty). Also `NiTextureEffect`, `NiSwitchNode` identity, `BSFurnitureMarker`, `BSBound` (`nifal.md` §2 "Passthroughs").
- **Collision**: the FO4+ `BhkNPCollisionObject` blob does not resolve to a shape; `BhkPCollisionObject` phantoms need a trigger-volume path; `BhkPlaneShape → None` (counted in `plane_shapes`); decoded-but-not-imported constraint kinds; `BhkMeshShape` carries no material. The raw per-game Havok surface id on `RigidBodyData` is parked by name (#5574, `nifal.md` "Havok surface material").
- **Particles**: `initial_color` deliberately not applied; size-over-life bell cannot map to linear start/end; legacy `NiParticleSystemController` keeps the preset rate.
- **Animation**: per-light ambient colour channels and morph-weight channels (no canonical consumer). `NiPathInterpolator` follow-axis, banking, constant-velocity and closed-curve wrap are parked by name in `nifal.md` §Animation (#5571).
- **Lights**: affected-node scoping for every `NiLight` kind (#5282); `NiAmbientLight` (0 live vanilla population). The ESM-fallback light now sits at the exact-name `AttachLight` node (c1a99c25e) and keeps the first mesh entity when none is authored.
- **Materials/CDB**: Starfield `EmissiveSettingsComponent` parked (#5503); `LayeredEmissivityComponent` uncaptured; additive-family blend modes not forwarded (#5509); single-channel kinds parked by name; loose `.mat` parser lacks inheritance and `BlendingMode` (#5525); resolver trigger table (#5437, #4246); shader-type discriminator never reaches `Material` (#4256); the CDB `HasOpacity` default of `false` is census-derived, not sourced (#5661, open).

## Not verified

- The `HasOpacity` "class default `false`" in 0d698d16c was not re-measured. It needs the 250 MB CDB parse through the `#[ignore]` real-data test, which this run's rules exclude.
- Observation, not a finding: `resolved_shape_structs_in` ends the `resolve_shape_inner` body at the next `"\nfn "`, so a future `pub(super) fn` placed directly below it would be absorbed into the scan. Today the next item is the private `fn per_axis_scale`, so the slice is tight (checked).
- Whether a loaded save overwrites the freshly imported `part_surface_materials` is `serde(skip)`ed and re-derived at import. `/audit-save` and `/audit-physics` own the save-load behaviour.
- Source-scan guards (#4411) remain structural: a green guard is not proof a field is asserted.

Next step: `/audit-publish docs/audits/AUDIT_NIFAL_2026-10-12.md` (domain labels `nifal` + `doc-rot`; the carried #5502 item needs no new issue).
