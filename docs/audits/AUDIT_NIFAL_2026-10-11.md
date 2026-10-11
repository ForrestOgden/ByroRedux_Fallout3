**HEAD**: c6e48cbc6 · **Baseline**: docs/audits/AUDIT_NIFAL_2026-10-09.md (HEAD 3bcf6c8e8) · **Audited**: Dims 1–9 (delta-scoped; 141 commits since baseline, of which 11 touch NIFAL paths) · **Unchanged since baseline (skimmed)**: Dim 2 (mesh/coord/transform/units/rotation), Dim 3 (skinning), Dim 4 (`import/types.rs`), Dim 5 (`systems/particle.rs`) — guards and first steps run

# NIFAL Audit — 2026-10-11

Three auditors split the dimensions: {6, 2, 3, 5}, {8, 1, 9}, {7, 4}. Scratch notes are in `/tmp/audit/nifal/dim_{1..9}.md`.
The orchestrator re-read the code for D7-01 and D7-02 (both confirmed) and spot-checked D6-02's field.
Phase 1 guard run: `cargo test -p byroredux-nif -p byroredux material_translate` is green (77 `material_translate` tests).

NIFAL-relevant delta commits: 5c14ebdf8 (CDB `Parent` inheritance, #5502), 4667b884b (park CDB emissive, #5503),
0b2ffed89 (MSWP palette row, #5497), 886edcded / 441d90c2f / c173147da / edb57dd9f (Havok surface material through import,
#5541, #5542), cf8730adc (revisit guard on the five scene-graph walkers, #5485), 0c4d4af08 (B-spline span clamp and aggregate budget, #5486).

## Executive Summary

**Findings: 10 (0 CRITICAL · 1 HIGH · 2 MEDIUM · 7 LOW).** One is Existing (D8-04, the unmeasured half of #5502). The other nine are NEW.
Dims 1, 2, 3, 4, 5 and 9 have no findings.

**Headline 1 — D8-04 (HIGH by rubric, Existing #5502).** #5502's fix (5c14ebdf8) inherits CDB *settings* down the `Parent` chain: glass,
emissive, alpha, blend and translucency. It does **not** inherit *texture-set slots* or flat-colour replacements. This audit measured
that half for the first time against the shipped CDB and the 10,380 material names the NIFs reference (10,158 resolve):
- **2,051 (20.2 %)** resolve with zero textures. **2,525** have neither a colour texture nor a flat colour.
- A scratch prototype that makes `collect_slots` walk the `Parent` chain cuts those to **217** and **385**.
- Inheriting the `LayerID/TextureSetID/MaterialID` child edges instead changes nothing, so texture-set slot inheritance is the effective half.
- `nifal.md`'s "98 % texture resolution" is the name→object key-hash hit rate (97.9 %), not the share of materials that end up with a texture.

These are Starfield-only, but by the severity rubric ("translatable block silently dropped … HIGH if it removes visible content") a fifth
of referenced materials rendering as the checker placeholder is HIGH. The issue is currently triaged MEDIUM, so I recommend splitting
the texture half out of #5502 and re-triaging it.

**Headline 2 — D7-02 (MEDIUM, regression of #5486's fix).** 0c4d4af08 bounds B-spline resampling by intersecting the interpolator span with
its sequence's span. It then maps the curve parameter `u` over the *clamped* window instead of over the interpolator's own span, so any interpolator
wider than its sequence has the whole curve time-warped into the trimmed window. Vanilla is inert (0 mismatches in 157,863 FO3/FNV/DLC
B-spline controlled blocks), so this is a mod/trimmed-sequence regression. The commit's new tests pin key counts only, never values.

**Headline 3 — D7-01 (MEDIUM, pre-existing, never measured).** `NiPathInterpolator` import (`anim/transform.rs`) treats the path keys' 0..1 parameter
time as animation time. It drops `percent_data_ref`, `follow_axis`, bank and flags, and stamps identity rotation and scale. The code comment
justifying this ("vanilla uses the simple time-keyed form") is contradicted by measurement: all 205 vanilla path interpolators have a non-null
percent ref and path keys at exactly t=[0,1] (Oblivion 82, FO3 15, FNV 52, Skyrim SE 71). Example: the salmon effect's percent curve
spans t=[0,33.3], so it plays about 33× too fast.

**Delta verifications (clean).**
- **cf8730adc (#5485)** revisit guard: global per walk, keyed by block index. Re-measured on 108,308 vanilla NIFs (FNV 14,881, FO3 10,989,
  Oblivion 8,032, Skyrim SE 32,709, FO4 41,697): **0** blocks with more than one parent and 0 self-references. No vanilla light, emitter,
  node or texture effect is dropped. This extends the commit's census to FO3. The skip is `debug!`-logged only.
- **4667b884b (#5503)** emissive park: consistent. Nothing consumes the CDB emissive values, and the test pins `emissive_color=0`,
  `emissive_mult=0` and the default `emissive_source` for Enabled true/false/None.
- **5c14ebdf8 (#5502)** settings inheritance: nearest-wins, depth cap 64 and cycle bound are correct. On the real CDB (2 s, 250 MB):
  `IsGlass=true` 4→217, emissive `Enabled` 0→619 (337 with an emissive map), `AlphaBlend` still 0. None of the 217 glass materials authors a
  coverage signal (`BlendingMode`, `HasOpacity` or `AlphaTestThreshold`), so the classifier stays inert. No glass is wrongly routed, and
  the `external_material_resolved` / `from_bgsm` split is intact. Root-chain versus layer-material ordering makes 0 differences over all 10,158.
- **0b2ffed89 (#5497)** MSWP palette row: stamped on the raw-tier `ImportedMaterial` before `translate_material_with_provenance`. No second `Material`
  producer and no invented math.
- **#5541, #5542** (closed): verified fixed in code. **#5549, #5550** (open): still present, not re-filed.

Per-category status against spec §2:

| Category | This delta |
|---|---|
| Material (D1) | Converged. `Material` and `material_translate.rs` unchanged since baseline. 4 production callers; 2 production `Material {}` literals plus the Cornell exemption; `sanitize_finite` covers all 35 f32 fields; `if game` in shaders empty. 298 `byroredux` bin guards pass. No findings |
| Geometry/Transform (D2) | No commits on mesh/coord/transform/units/rotation. 657 `import::` tests pass. #5485 guard measured lossless on vanilla. No findings |
| Skinning / Lights (D3) | Unchanged apart from the walker guard. Light dispatch and satellite-independence guards pass (the satellite guard is not vacuous). No findings |
| Nodes (D4) | The 7 parked fields still have 0 canonical consumers; `import/types.rs` unchanged. No findings |
| Particles (D5) | Exactly 2 `apply_emitter_overlays` callers (`scene/nif_loader.rs:1738`, `cell_loader/spawn.rs:1276`); `systems/particle.rs` untouched; 27 particle tests pass. No findings |
| Collision (D6) | 16 resolve arms (unchanged); `CollisionAuthoringSummary` still four `u32`; the proxy still inserts no `MeshHandle`; one NIF-derived `RigidBodyData` producer. **2 LOW** (new Havok surface-material channel) |
| Animation (D7) | NIF and HKX harnesses green (5 + 88 tests). **2 MEDIUM + 3 LOW**: path interpolator, B-spline clamp regression, span guard, colour sanity gate, budget observability |
| Shader flags / roles / merge (D8) | **1 HIGH (Existing #5502 half) + 2 LOW** (decoder test gap, skill doc-rot) |
| Completeness (D9) | Harness 2/2 green on all 8 games in 58.6 s; every floor and ceiling held; mesh counts equal the 2026-10-09 report's. The harness measures the pre-merge tier, so it cannot see D8-04 |

New findings per invariant:

| Invariant | Findings |
|---|---|
| single-boundary | 0 (D6-01 weakens a guard that polices it) |
| no-fabrication | D7-01 (invented identity rotation/scale keys; false vanilla claim), D7-02 (sampled curve ≠ authored curve), D7-03 |
| no-leak | D6-02 (raw per-game Havok material id on a canonical component), D7-04 |
| no-render-time-fallback | 0 |
| parked-not-leak / translation gap | D8-04 (texture-set slots dropped by the lookup) |
| harness-gap | D6-01, D7-05, D8-05; D8-04 and D7-01/02 are invisible to the existing harnesses |

## Per-Category Tier Matrix

| Category | single-boundary | no-fabrication | no-leak | no-render-time-fallback | Boundary fn |
|---|---|---|---|---|---|
| Material | PASS: 4 production callers (`nif_loader:1301`, `mesh_instance:1151`, `placement_lod:552`, `object_lod:379`) | PASS | PASS | PASS | `translate_material_with_provenance` / `translate_texture_only_material*` |
| Geometry/Transform | PASS | PASS | PASS | PASS | `import/units.rs`, `import/transform.rs`, `rotation.rs` |
| Skinning / Lights | PASS | PASS | PASS | PASS | `mesh/skin.rs`, `walk/lights.rs` |
| Nodes | N-A (by design) | PASS | PASS | PASS | spawn sites; 7 fields parked |
| Particles | PASS | PASS | PASS | PASS | `systems/particle.rs::apply_emitter_overlays` |
| Collision | PASS (one producer; D6-01 guard weakened) | PASS | **FAIL (LOW, dormant)**: D6-02 | PASS | `import/collision/` |
| Animation | PASS | **FAIL**: D7-01, D7-02, D7-03 | **PARTIAL**: D7-04 | PASS | `convert_nif_clip` + `convert_hkx_clip` |
| Shader flags / roles / merge | PARTIAL: #5284, #5525 still open | PASS | PASS | PASS | `merge.rs::{apply_cdb_material, apply_loose_mat}`, `sfmaterial::MaterialIndex::lookup` |

## Findings

### D8-04: Texture-set slots (MRTextureFile / TextureReplacement) are not Parent-inherited — 2,051 of 10,158 (20.2 %) vanilla-referenced Starfield materials resolve with zero textures
- **Severity**: HIGH (arguable; the existing issue is triaged MEDIUM — re-triage recommended)
- **Dimension**: Shader-flags/Effects (Starfield CDB arm of the merge boundary)
- **Tier Violated**: parked-not-leak (parsed and captured, then dropped by the lookup) + harness-gap (no census existed)
- **Game Affected**: Starfield
- **Location**: `crates/sfmaterial/src/index.rs:296-347` (`lookup`) → `:349-375` (`collect_slots`, first-wins over the named texture-set object only); `docs/engine/nifal.md:728-733`
- **Status**: Existing: #5502 (OPEN; this is its remaining half). Supplies the missing measurement. Recommend splitting it into its own issue.
- **Description**: A derived `TextureSet` object authors only deltas; the CDB stores a `MRTextureFile` slot on the ancestor that authored it. `collect_slots` reads only the named texture set, so any set whose slots come from a parent yields nothing. The doc's "98 % texture resolution" is the name→object key-hash hit rate (10,158/10,380), not the share of materials that end up with textures.
- **Evidence** (scratch probe over `lookup` on the 10,380 `sf_matpath_dump` names; public API, 250 MB RSS):
  - HEAD: hits 10,158; **no texture at all 2,051**; no colour texture 3,377; no colour texture and no flat colour 2,525.
  - Example: `BareScuffedMetal01_Panel01.mat` → Layer → layer material 261920 → TextureSet 261922 (own slots: none) → parent 223084 (colour `MetalGalvanized01_color`, normal `paintscuffed01_normal`). Hair `Flat_Top.mat` → TextureSet 72006 (none) → parent 72002 (`Afro_Hair_Shared_Brown_color`).
  - Prototype (scratch copy of `sfmaterial`, `collect_slots` walking `parents` nearest-wins per slot, bound 64): no-texture 2,051→**217**, no colour-nor-flat 2,525→**385**, no colour texture 3,377→1,343.
  - Control: inheriting the `LayerID/TextureSetID/MaterialID` child edges without slot inheritance changes nothing. Flat-colour inheritance was not measured separately.
- **Impact**: About 2,000 vanilla Starfield materials render as the checker placeholder. Downstream consumers (emissive-map bind, alpha-test threshold sampling base alpha, glass coverage) all work on an absent texture. 5c14ebdf8's inherited `AlphaTestThreshold` now fires `alpha_test` on 2,132 materials (was 1,544), many with no bound base-colour texture to test.
- **Related**: #5502, #3398, #5284, #5525; AUDIT_NIFAL_2026-10-09 D8-01.
- **Suggested Fix**: Make `collect_slots` (texture slots and `flat_colors`) walk the texture-set object's `parents` chain nearest-wins per slot with the existing `MAX_PARENT_DEPTH`. Add a real-data pin (`BareScuffedMetal01_Panel01.mat` resolves a colour texture) and correct the "98 %" wording in `nifal.md`.

### D7-01: `NiPathInterpolator` treats the 0..1 path-parameter key times as animation time and drops `percent_data_ref`, `follow_axis`, `bank_*`, `flags`; rotation/scale stamped identity
- **Severity**: MEDIUM
- **Dimension**: Animation
- **Tier Violated**: no-fabrication (identity rotation/scale keys invented) + silent drop of a translatable block's fields
- **Game Affected**: Oblivion, FO3, FNV, Skyrim
- **Location**: `crates/nif/src/anim/transform.rs:105-121` (dispatch + comment) and `:194-224` (`extract_transform_channel_path`); fields parsed at `crates/nif/src/blocks/interpolator.rs:560-600`
- **Status**: NEW (the only related issue, #605, is CLOSED. It only made the block reach the extractor; nothing covers percent data or follow-axis.)
- **Description**: `extract_transform_channel_path` converts the referenced `NiPosData` keys into translation keys verbatim and ignores `percent_data_ref` (`NiFloatData`). In Gamebryo the path keys are a *parametric* curve ("The path keys must have a time range starting at 0.0 and ending at 1.0", `NiPathInterpolator.h`), and the percent data maps animation time to that parameter. The comment at `transform.rs:107-115` says vanilla "uses the simple time-keyed form where path keys map directly to animation time"; measurement says the opposite. Rotation is a hard-coded identity key and scale a hard-coded 1.0 key, so follow-axis alignment and banking are never produced. The identity keys also *override* the node's authored local rotation/scale instead of leaving the bind value, which `static_transform_channel` does by leaving the key list empty for an inactive axis.
- **Evidence** (throwaway survey over the vanilla BSAs; the orchestrator re-read the extractor and confirmed it uses `data.keys` directly with identity rot/scale):
  - Every vanilla `NiPathInterpolator` carries a non-null `percent_data_ref`: Oblivion 82/82, FO3 15/15, FNV 52/52, Skyrim SE 71/71. 205/205 path `NiPosData` groups span exactly t=[0,1].
  - `meshes\effects\fxambwatersalmon02a.nif`: percent t=[0,33.3], so the salmon loop imports as a 1 s path loop, about 33× too fast. Also `megatonfanltrays01.nif` (pct t=[0,3.33]), `dlc1fxambbeamgladespot.nif` (t=[15,22]), Oblivion `destruction.nif` (t=[0,1.667], 121 path keys).
  - Non-trivial percent curves (more than 2 keys): Oblivion 15/82, FNV 3/52, Skyrim 27/71. `follow_axis` 0/1/2 present; `max_bank_angle != 0` on most.
  - None of `follow_axis`/`bank_dir`/`max_bank_angle`/`smoothing`/`flags`/`percent_data_ref` is read outside the parser. `nifal.md` does not list the path interpolator as parked.
- **Impact**: Butterflies, fish, hawks, ambient light-ray fans, magic-effect beams and similar paths play at the wrong speed and without facing along the path, and the node's authored rotation is replaced by identity. Visible content, not a crash.
- **Related**: #605 (closed), `docs/engine/nifal.md` §Animation.
- **Suggested Fix**: Evaluate the percent curve at animation time to get the path parameter, then resample the `NiPosData` spline at that parameter into linear keys at import (as the B-spline path does). Emit follow-axis rotation when the FOLLOW flag (0x20) is set and leave rotation/scale empty otherwise. If deferring, park it explicitly in `nifal.md` with the measured census above and stop emitting the identity keys.

### D7-02: #5486's span clamp re-parameterises the B-spline: `u` is mapped over the clamped span, time-warping any interpolator wider than its sequence
- **Severity**: MEDIUM (vanilla-inert; a regression on mod or trimmed-sequence content — may be downgraded to LOW)
- **Dimension**: Animation
- **Tier Violated**: no-fabrication (the sampled curve no longer equals the authored curve)
- **Game Affected**: FO3, FNV, and any mod KF (Oblivion and Skyrim vanilla have no NIF B-splines)
- **Location**: `crates/nif/src/anim/bspline.rs:95-125` (`reserve` returns the clamped `(start, stop)`); consumers `bspline.rs:316-337` (float), `:406-431` (transform), `crates/nif/src/anim/channel.rs:321-342` (point3 colour)
- **Status**: NEW — Regression introduced by 0c4d4af08 (the fix for #5486, closed); the allocation bound itself is fine.
- **Description**: A B-spline's parameter range `[0, n_cp-3]` spans the interpolator's own `[start_time, stop_time]`. Before 0c4d4af08, `u` was computed against the interpolator's own duration. Now `reserve` returns `start = max(interp_start, seq_start)` and `stop = min(interp_stop, seq_stop)`, and the samplers compute `u = ((t - start_time) / duration) * u_max` with the clamped values. Whenever the clamp trims, the entire curve is stretched over the trimmed window instead of sampling the visible sub-range. Interp [0,2] in a sequence [0,1] now plays the whole 2 s curve in 1 s. Disjoint spans (interp [0,10], seq [20,30]) give `start=20, stop=10, duration=0`: two keys at t=20 holding control point 0.
- **Evidence**: `channel.rs:324-342` (read by the orchestrator): `reserve(interp.start_time, interp.stop_time, 1)` returns the clamped window, then `duration = stop_time - start_time` and `u = ((t - start_time) / duration) * u_max`. The commit's four tests (`sampled_span_is_clamped_to_the_owning_sequence`, the ceiling, the budget decline, `import_kf_bounds_a_shared_huge_span_interpolator`) pin sample counts, never values. Vanilla scan: 0 of 157,863 FO3/FNV/DLC B-spline controlled blocks have a span differing from their sequence.
- **Impact**: KF/NIF content whose interpolator span is wider than its sequence animates at the wrong speed or segment. Previously correct, silently regressed.
- **Related**: #5486, `/audit-nif` Dim 6 (allocation side), D7-03.
- **Suggested Fix**: Keep the original `(interp_start, interp_stop)` as the `u` domain and restrict only which `t` values are emitted. Add a value-level test: the sampled key at the window start equals `deboor_cubic` at the matching `u`, not at `u=0`.

### D6-01: The dispatch-coverage guard is now satisfied by the surface-material walker for 7 of 16 shapes
- **Severity**: LOW
- **Dimension**: Collision
- **Tier Violated**: harness-gap
- **Game Affected**: all (FO3/FNV/Oblivion/Skyrim collision)
- **Location**: `crates/nif/src/import/collision/mod.rs` (`dispatch_coverage_tests::resolved_shape_structs_in`, scans `shape.rs` up to the first `#[cfg(test)]`); `crates/nif/src/import/collision/shape.rs:110-224` (`shape_leaf_materials_inner`)
- **Status**: NEW (side effect of 886edcded / 441d90c2f; builds on the "known-weak per #4411" caveat)
- **Description**: The guard proves "every dispatched `Bhk*Shape` has a `downcast_ref::<Bhk..Shape>` in `resolve_shape_inner`" by splitting the whole `shape.rs` production text on `downcast_ref::<`. The surface-material commits added a second dispatch chain, `shape_leaf_materials_inner`, in the same production prefix. It names packed, list, convex-list, transform, convex-sweep, MOPP and compressed-mesh via literal `downcast_ref::<..>`. Deleting the real `resolve_shape_inner` arm for any of those seven would still pass.
- **Evidence**: Distinct `downcast_ref::<Bhk*Shape>` in `resolve_shape_inner` (`shape.rs:233-508`) = 16; across the whole production prefix = 16; the seven names appear in both ranges. The other leaf shapes go through `leaf_material!($ty)`, whose `downcast_ref::<$ty>` does not match the `Bhk` prefix scan, so they are still discriminated.
- **Impact**: Guard confidence for the container/mesh shapes that carry the most collision (MOPP, list, packed, compressed) is structurally vacuous. No current bug.
- **Related**: #4411, #5550 (the walker that drifts from `resolve_shape_inner`). Folding material production into the single resolve walk fixes #5550 and this together.
- **Suggested Fix**: Make `resolved_shape_structs_in` scan only the `resolve_shape_inner` body (slice to the next column-0 `fn`) and assert the slice is non-empty, or fold the material production into `resolve_shape_inner`.

### D6-02: The raw per-game Havok material number is carried on the canonical `RigidBodyData` with no translate step, no sourced mapping, and no spec row
- **Severity**: LOW (dormant: both consumer maps ship empty)
- **Dimension**: Collision
- **Tier Violated**: no-leak
- **Game Affected**: Oblivion, FO3, FNV, Skyrim LE/SE (FO4+/Starfield bodies get the `0` default; no material is read from the `BhkNP` blob)
- **Location**: `crates/core/src/ecs/components/collision.rs:118-145` (`surface_material: u32` at :131, `part_surface_materials` at :144); `crates/nif/src/import/collision/mod.rs:444-466`; consumers `byroredux/src/components.rs:1826-1900` (`surface_sounds: BTreeMap<u32, Arc<Sound>>`), `byroredux/src/systems/audio.rs`
- **Status**: NEW. Adjacent: #5542 (CLOSED) listed "translate to a canonical engine surface enum at the NIFAL boundary, per game" as the alternative to widening; the fix widened instead, which leaves this seam untracked. #5549 (OPEN) covers only the `0` sentinel half.
- **Description**: This is the same class as `hkMotionType` → `MotionType` (collapsed at translate), but nothing collapses here. Oblivion's `0=STONE,1=CLOTH,…`, FO3/FNV's different 0–127 table and Skyrim's `SKY_HAV_MAT_*` CRC32s sit side by side in one `u32` and mean something only relative to the loaded game. `0` is STONE on Oblivion/FO3/FNV but `NONE` on Skyrim. A populated `surface_sounds` map must either be game-specific (a per-game branch downstream of the boundary) or rely on one game per process. `nifal.md` §2 "Collision" and `physal.md` do not mention the channel, so it is neither a documented translation nor a parked row.
- **Evidence**: The `collision.rs` doc says "the per-game material enum's numeric value (Oblivion/Fallout3/Skyrim HavokMaterial …)"; `grep -n "surface_material\|HavokMaterial" docs/engine/nifal.md docs/engine/physal.md` returns nothing. The map-building step is deferred in 886edcded's message.
- **Impact**: None audible today. When the first map entry lands the consumer must key on a game-dependent number or add a per-game branch. A saved column already persists the raw number, so a later switch to a canonical id costs another `FORMAT_MAJOR` bump.
- **Related**: #5542 (closed), #5549 (open), #5550 (open), #4256 (analogous "discriminator never crosses" shape).
- **Suggested Fix**: Before any map is populated, decide the canonical form (a game-agnostic surface id derived at the import boundary) and add a parked row to `nifal.md` "Collision". Otherwise record explicitly that raw-per-game is the intended canonical form and name the game key.

### D7-03: `seq_span` finiteness guard does not do what its comment says
- **Severity**: LOW
- **Dimension**: Animation
- **Tier Violated**: no-fabrication
- **Game Affected**: all with B-spline content; only reachable on non-vanilla input
- **Location**: `crates/nif/src/anim/sequence.rs:50-57`; defaults at `crates/nif/src/blocks/controller/sequence.rs:395-405`
- **Status**: NEW
- **Description**: The comment says the FLT_MAX/FLT_MIN neutral defaults "parse to non-finite differences", but the guard tests `start.is_finite() && stop.is_finite()` per value, and both defaults are finite. A sequence parsed below 10.1.0.106 therefore sets `seq_span` to (MAX, MIN), giving `duration=0` and two B-spline keys at t≈3.4e38. An authored `stop <= start` sequence likewise trims every interpolator to zero length.
- **Impact**: Wrong-keyed (not crashing) channels on malformed content; the comment misleads.
- **Related**: D7-02
- **Suggested Fix**: Gate `seq_span` on `(stop - start).is_finite() && stop > start`.

### D7-04: Sampled B-spline colour keys have no `is_key_value_sane` gate
- **Severity**: LOW
- **Dimension**: Animation
- **Tier Violated**: no-leak
- **Game Affected**: FO3 / FNV with `NiBSplineCompPoint3Interpolator` colour controllers
- **Location**: `crates/nif/src/anim/channel.rs:344-349`
- **Status**: NEW (a pre-existing asymmetry with the float and transform samplers, #3765; not introduced by 0c4d4af08)
- **Description**: `sample_color_keys_bspline_point3` pushes `[p[0], p[1], p[2]]` straight from `deboor_cubic`. The static fallback is gated, but a finite-yet-huge `half_range` can overflow to inf through the blending and reach an `AnimColorKey`.
- **Impact**: Reachable only via malformed quantisation parameters.
- **Related**: #3765, #4397
- **Suggested Fix**: Skip a sample whose three components fail `is_key_value_sane`, as the float sampler does.

### D7-05: B-spline budget exhaustion is logged once per import and uncounted
- **Severity**: LOW
- **Dimension**: Animation
- **Tier Violated**: harness-gap
- **Game Affected**: none in vanilla (worst measured file 200,151 of 1,000,000 units, 5× headroom)
- **Location**: `crates/nif/src/anim/bspline.rs:95-122` (`warned` flag), `crates/nif/src/anim/entry.rs:32-80`
- **Status**: NEW
- **Description**: One `log::warn!` per `BsplineSampling`, with no count of degraded channels. Which sequences degrade depends on `NiControllerManager.sequence_refs` order, then block order. No test pins "a hostile early sequence does not starve a later honest one".
- **Impact**: Diagnosability only.
- **Related**: #5486
- **Suggested Fix**: Count degraded channels and include the count in the single warn, or expose a `degraded()` accessor the importer logs once.

### D8-05: The `ObjectInfo.Parent` decode in `stream_db_file_index` has no default-lane test
- **Severity**: LOW
- **Dimension**: Shader-flags/Effects
- **Tier Violated**: harness-gap
- **Game Affected**: Starfield
- **Location**: `crates/sfmaterial/src/index.rs:686-692` (the only writer of `parents`); tests at `:1280-1320` hand-set `idx.parents`; real coverage only in `crates/sfmaterial/tests/real_cdb.rs::material_index_resolves_inherited_glass_and_emissive_settings` (`#[ignore]`)
- **Status**: NEW
- **Description**: The inheritance unit tests inject `idx.parents` directly. A rename or shape change of the `Parent` ID field would silently revert to "no inheritance" in CI without game data, since `id_named` returns `None` on any shape mismatch without a log.
- **Impact**: Silent loss of the whole #5502 settings fix on a decoder refactor in the default lane.
- **Related**: #5502, #5320
- **Suggested Fix**: Give the synthetic CDB fixture a non-zero `Parent` on one object and assert `lookup` returns the inherited setting. Consider an info log when an `Objects` entry has no decodable `Parent`.

### D8-06: Audit-skill and doc text still describes the pre-#5502/#5503/#5497 state
- **Severity**: LOW (doc-rot)
- **Dimension**: Shader-flags/Effects
- **Tier Violated**: n/a
- **Game Affected**: Starfield, FO4
- **Location**: `.claude/commands/audit-nifal/SKILL.md:90` and `:163` (inheritance unresolved, emissive forwarded, #5503 open); `.claude/commands/audit-starfield/SKILL.md:66`; `.claude/commands/audit-parsers/SKILL.md:87`; `.claude/commands/audit-fo4/SKILL.md:78` (MSWP CNAM row called open; closed by 0b2ffed89, field is now `color_remap_index`); `docs/engine/nifal.md:728-733` ("98 % texture resolution", see D8-04)
- **Status**: NEW (route to `/audit-sync`)
- **Description**: An auditor following the skills would re-file a parked arm as live or treat solved inheritance as open. Current state: settings inheritance resolved (217 `IsGlass`, 619 emissive `Enabled`, no coverage signal, classifier inert), emissive parked (#5503 closed), texture-set inheritance open (D8-04), MSWP palette row applied (#5497).
- **Suggested Fix**: Re-sync those lines via `/audit-sync`.

## Documented-limitation ledger (parked-not-leak — do not re-report)

- **Node/mesh passthroughs**: `bs_value_node`, `bs_ordered_node`, `tree_bones`, `range_kind`, `lod_group`, `bs_lod_cutoffs`, `bs_sub_index` — raw-tier, 0 canonical consumers. Also `NiTextureEffect` (extractor not called by `byroredux`; content-absent), `NiSwitchNode` identity, `BSFurnitureMarker`, `BSBound` (`nifal.md` §2 "Passthroughs").
- **Collision**: the FO4+ `BhkNPCollisionObject` blob does not resolve to a shape (container decoded, `hknpCompressedMeshShapeData` layout not); `BhkPCollisionObject` phantoms need a trigger-volume path; `BhkPlaneShape → None` (counted in `plane_shapes`); decoded-but-not-imported constraint kinds; `BhkMeshShape` carries no material (the parser has no field, nif.xml has none).
- **Particles**: `initial_color` deliberately not applied; size-over-life bell cannot map to linear start/end; legacy `NiParticleSystemController` keeps the preset rate.
- **Animation**: per-light ambient colour channels and morph-weight channels (no canonical consumer).
- **Lights**: affected-node scoping for every `NiLight` kind (#5282); `NiAmbientLight` (0 live vanilla population).
- **Materials/CDB**: Starfield `EmissiveSettingsComponent` parked after #5503 (no `emissive_color`/`emissive_mult` from CDB data); `LayeredEmissivityComponent` uncaptured; additive-family blend modes not forwarded (#5509); single-channel kinds (`_rough`/`_metal`/`_ao`/`_opacity`/`_transmissive`) parked by name; loose `.mat` parser lacks inheritance, `BlendingMode` and emissive (#5525); CDB resolve tail copied at four `merge.rs` sites (#5284); resolver trigger table (#5437, #4246); shader-type discriminator never reaches `Material` (#4256); declared beam-converter list (#5285).
- **Revisit guard residual**: a modded NIF with a deliberately shared child loses its second placement. This is logged at `debug!` only; vanilla has none (108,308 files). Not filed.
- **Open collision issues** (seen, not re-filed): #5549 (`0` read as "unauthored"), #5550 (the material walker drifts from `resolve_shape`).

## Not verified

- Whether a loaded save's restored `RigidBodyData` overwrites the freshly imported `part_surface_materials` (`serde(skip)`). That is `/audit-save` and `/audit-physics` territory.
- How many of the 217 inherited-glass Starfield shapes carry a NIF `NiAlphaProperty` (needs a NIF-level census).
- FO4 / FO76 / Starfield B-spline and path-interpolator populations, and FO76 / Starfield multi-parent counts.
- Flat-colour `TextureReplacement` inheritance, measured separately.
- The legacy Gamebryo 2.3 source was not mounted, so single-parent behaviour was checked against the commit's census and Gamebryo 2.6 headers only.
- Source-scan guards (#4411) remain structural: a green guard is not proof a field is asserted.
