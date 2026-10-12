# #5535: CONC: ABBA lock-order lane red since the 7ef007898 push — four cycles across 15 byroredux tests (eat_sleep/sandbox, walk_anim, hair head-mount, cinematic)

## Summary

The `ABBA lock-order detector` CI lane (`BYRO_LOCK_ORDER_CHECK=1 cargo test --workspace --no-fail-fast --exclude byroredux-ui`) has been red on `main` since the `7ef007898` push on 2026-10-10. It is still red at `455b473a1` (run 38104575965). It is the only failing job at HEAD.

It went unnoticed for the whole push window because the `Test + Check + Clippy` job was also red until `a123b173a` (#5477).

## Failing tests (15, all in `-p byroredux`, `test result: FAILED. 2750 passed; 15 failed`)

Four distinct cycles from `crates/core/src/ecs/lock_tracker.rs:476`:

| Cycle | Tests |
|---|---|
| `Furniture` → `GlobalTransform` → `Furniture` | 6: `systems::eat_sleep::tests::{arrived_eat_actor_seats_at_sit_marker, in_cell_seat_search_spans_the_cell_not_the_default_radius, legacy_sleep_actor_seats_at_the_sit_fallback, sleep_actor_prefers_the_sleep_marker}`, `systems::sandbox::tests::{a_creature_actor_never_takes_furniture, fully_seated_world_skips_the_seat_table_rebuild}` |
| `AnimationPlayer` → `Transform` → `AnimationPlayer` | 5: `systems::walk_anim::tests::{moving_actor_swaps_to_walk_clip_and_back, scratch_reuses_allocation_across_frames, seated_actor_is_never_taken_over_and_abandons_mid_walk, skyrim_style_actor_gains_then_loses_a_player, yielding_when_another_writer_swapped_the_clip_mid_walk}` |
| `Children` → `MeshHandle` → `Children` | 3: `npc_spawn::resumable::tests::{an_intentional_shape_tilt_survives_the_head_mount, female_hair_with_cancelling_root_and_shape_rotations_keeps_actor_axes, male_hair_with_identity_root_and_shape_keeps_actor_axes}` |
| `Transform` → `ActorCinematicState` → `Transform` | 1: `systems::cinematic::tests::released_rider_exit_heading_matches_the_attached_heading` |

## Window

The lane was green at `ecd30d6d9` and red at the next pushed head, `7ef007898`. That push carried `75aff29f4` (#5157 hair head-mount), `e128218eb` (#5353), `1efdfda4a` (#5352), `9d826272b` (#5531), `a851c625d` (#5507), `9b19aa815` (#5402), `3cac5d5ca` (#5497 partial), `002fbd6ef` (#4134), `4739c411a` (#5453), `cc8a5df8f` (#5457), `e6d8f819f` (#5461 cinematic), `ec75b1439` (#5301) and `7ef007898` (#5343). The tracker panics on the thread that *closes* a cycle, so a failing test is not necessarily the one that added the reverse edge.

## Triage owed

For each cycle, decide whether it is:
- a real production acquisition-order inversion, which needs the systems to acquire in TypeId order or use `query_2_mut`; or
- a test body holding one guard across a second acquisition and supplying the back edge (the #5032 shape), which needs the guard dropped before the second query.

Run `BYRO_LOCK_ORDER_CHECK=1` under `-p byroredux` **and** `-p byroredux-scripting`. To attribute edges fast, temporarily log each new edge with a trimmed backtrace in `record_and_check` (`--test-threads=1 --nocapture`), then revert. See #313, #2675, #4603 and #4982.

Found at the Session 97 `/session-close`.


---

# #5567: REN-D7-2026-10-11-01: the ReSTIR direct-EMA lag fixtures still model the retired α = 0.025 parked tail; the live deep tail (1024 / 0.002) fades a switched-off light ~13× slower

_From `docs/audits/AUDIT_RENDERER_2026-10-11.md` (HEAD 334458b3f)._ Also reported by Dim 2 as REN-D2-2026-10-11-03 (same defect).

- **Severity**: LOW (test-gap / doc-rot). Dim 2 reported the same defect as REN-D2-2026-10-11-03 — publish once.
- **Dimension**: Denoise & resolve chain / Soft Shadows
- **Location**: `mixed_visibility_clusters_keep_a_documented_ratio_lag` and the flicker fixture above it in `crates/renderer/src/vulkan/restir.rs` (both hard-code `let alpha = 0.025f32`); the `restirUnshadowedSum` comment in `crates/renderer/shaders/triangle.frag` ("fades over the EMA window"); the doc of `light_rig_geometry_key_is_independent_of_light_order` in `crates/renderer/src/vulkan/caustic.rs` ("parked tail 256 -> 16").
- **Status**: NEW (adjacent to closed #5020 and #5369 and open #5370; consequence of `48bed0504`)
- **Description**: The shader's parked branch is `alphaFloor = sceneStatic ? 0.002 : 0.1` with cap 1024. Mode 3 stays in the deep tail through *any* intensity change by design (the rig key is intensity-blind). The #5020 limit (the EMA stores one whole-cluster ratio) is therefore governed by α = 0.002, not 0.025. The fixtures assert only `lit[19] > 0.25` after 20 frames, true for any α ≲ 0.03.
- **Evidence**: re-simulating the fixture's own model at the live constants: flicker pass-through stays ≈ 48 % (0.490 @ 0.025, 0.484 @ 0.002, so the shader's "~48 %" text is still right), but the switched-off fade half-life is 27 frames @ 0.025, 86 @ 0.008, **346 @ 0.002** (frames to a quarter: 55 / 173 / 693).
- **Impact**: visual transient only, in a narrow case: a multi-light cluster with differing visibility plus a light whose intensity animates to or through zero while staying in the list. At 60 fps that is a ~6 s half-life of a phantom 50 % contribution from the occluded neighbour; at 300 fps ~1.2 s. Penumbra convergence (the reason for the change) is unaffected. The risk is that the pinned model is 13× off, so a later change cannot be judged against it.
- **Related**: #5020, #5369, #5370 (open), #5507.
- **Suggested Fix**: Make the two fixtures read the live constants (parse the literal out of `triangle.frag`, or share one `const`), state the half-life in the `restirUnshadowedSum` comment, and refresh the `caustic.rs` doc to 1024 → 16. If the multi-second step lag is unacceptable, resetting `histLen` on a large per-frame change in the unshadowed sum is a design call, not a drive-by.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (other shader callers / sibling light types)
- [ ] **TESTS**: A regression test pins this specific fix


---

# #5568: REN-D2-2026-10-11-04: glam-deprecated names and a stale noise-seed description remain in comments and docs

_From `docs/audits/AUDIT_RENDERER_2026-10-11.md` (HEAD 334458b3f)._ 

- **Severity**: LOW (doc-rot)
- **Dimension**: Camera-Relative Precision / Soft Shadows
- **Location**: `crates/core/src/ecs/components/camera.rs` (doc comments at the `perspective_rh` mentions and the inline "glam's perspective_rh already maps Z…" comment); `docs/engine/coordinate-system.md` (shows `Mat4::perspective_rh`); `docs/engine/shadow-pipeline-tradeoffs.md` §4 (`cameraPos.w` described as the seed for `interleavedGradientNoise` in shadow-ray jitter).
- **Status**: NEW (the `frame_params.rs` "look_at_rh call" comments the agent cited did not reproduce in a grep of `byroredux/src/render/`; those two are unverified)
- **Description**: glam 0.33 deprecated `Mat4::look_at_rh` / `perspective_rh`; the producers are now `camera::rh::view::look_at_mat4` and `camera::rh::proj::directx::perspective`. Since `4d991bad7` the finalize shadow-ray disk samples use `ditheredSequence2` (R2 in time, frame-free `ignMask` rotation per pixel), not `interleavedGradientNoise` seeded from `cameraPos.w`.
- **Impact**: doc rot only.
- **Related**: `c6b8d4584`, `4d991bad7`, `f4c2b9931` (the dependency-refresh comment sweep that missed these).
- **Suggested Fix**: Rename the references to the live producers and rewrite §4 of `shadow-pipeline-tradeoffs.md` around `ditheredSequence2`.

## Completeness Checks
- [ ] **SIBLING**: Other docs/comments carrying the same claim grepped and updated


---

# #5569: REN-D5-2026-10-11-01: memory-budget.md egui row describes a retained retired image and cites egui-ash-renderer 0.11

_From `docs/audits/AUDIT_RENDERER_2026-10-11.md` (HEAD 334458b3f)._ 

- **Severity**: LOW (documentation drift)
- **Dimension**: Memory/Lifecycle
- **Location**: `docs/engine/memory-budget.md`, section "egui debug overlay (`EguiPass`) — #4986" (the "Retired image awaiting `pending_free`" row and the "egui-ash-renderer 0.11" prose); code side `crates/renderer/src/vulkan/egui_pass.rs` (`promote_partial_deltas`, `dispatch`) and `sync.rs` rider 14.
- **Status**: NEW (related to closed #5026)
- **Description**: In egui-ash-renderer 0.11 and 0.13, `Renderer::set_texture`'s full-upload path creates the new image and descriptor set, then destroys the previous image and frees its descriptor set inside the same call. `pending_free` carries only egui's own `TexturesDelta.free` ids. `sync.rs` rider 14 already states this correctly.
- **Impact**: overstates steady-state VRAM by one atlas-sized image (a few MiB); the real peak is transient (old plus new resident only during the call).
- **Related**: #5026, #4986, #4988.
- **Suggested Fix**: Replace the row with "transient: old image destroyed inside `set_texture` once the replacement exists (peak = 2 images during the call)" and change "0.11" to "0.11/0.13".

## Completeness Checks
- [ ] **SIBLING**: Other docs/comments carrying the same claim grepped and updated


---

# #5570: REN-D11-2026-10-11-01: CLAUDE.md still states the pre-#5482 presentation order

_From `docs/audits/AUDIT_RENDERER_2026-10-11.md` (HEAD 334458b3f)._ 

- **Severity**: LOW (documentation)
- **Dimension**: FSR/Presentation
- **Location**: `CLAUDE.md`, the `src/vulkan/presentation.rs` line of the workspace tree ("output-res tonemap(graded × exposureTex)").
- **Status**: NEW (sibling sweep gap of closed #5532)
- **Description**: Since #5482 `presentation.frag` computes `exposed = scene.rgb * exposure` first, grades the exposed radiance (saturation → `gradeContrast` → brightness → tint), applies the #5154 chroma compress, and only then `tonemap(compressed)`. `shader-pipeline.md` and `renderer.md` already carry the new order; `CLAUDE.md` is the doc loaded at every session start.
- **Impact**: misleads anyone reasoning about where contrast pivots relative to exposure.
- **Related**: #5482, #5532, #5219.
- **Suggested Fix**: "Presentation pass — output-res exposure → IMGS/IMAD grade → chroma compress → tonemap (ACES|AgX), underwater extinction, swapchain write". (Editing `CLAUDE.md` is the user's call; not done by this audit.)

## Completeness Checks
- [ ] **SIBLING**: Other docs/comments carrying the same claim grepped and updated


---

# #5580: TD2-2026-10-11-02: Six ambient-AI systems carry the same Pass-2 write-back (Transform + NavPath + stuck timer), and `patrol` is a ~130-line copy of `wander`

Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: MEDIUM (table row 1, borderline). The family already diverged: #5452 found force-greet and Eat/Sleep followed no navmesh waypoints while the docs claimed they did, and #4189 had to be applied by hand to six files.
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/systems/{wander,patrol,travel,follow,guard,escort}.rs` (Pass 2 blocks: `wander.rs:423-466`, `patrol.rs:233-277`, `travel.rs:342-382`, `follow.rs:302-340`, `guard.rs:288-322`, `escort.rs:426-466`); `patrol.rs:93-232` vs `wander.rs:284-422`
- **Status**: NEW
- **Age**: Guard/Patrol landed `aaed15038` 2026-07-16; the NavPath write-back `a58bb4cf6` (2026-09-19, "Fix #4189") edited all six identically (11 lines each).
- **Effort**: medium
- **Description**:
  - Each system ends with `// ── Pass 2: apply writes (each a scoped single-type lock). ──`, then a byte-identical Transform loop (translation, optional rotation; 5 of 6 verbatim, `follow.rs` differs only by wrapping the tuple), then a verbatim `NavPath` take/insert/remove loop including the 6-line `#4189` comment (`grep -c 'd.nav_path.take()'` = 1 in each of the six files).
  - `wander` and `patrol` additionally share a verbatim `WalkStuckTimer` write-back (`wander.rs:459`, `patrol.rs:268`), the Pass-1a gather (queries, `speed`, `stuck_secs`, `resolve_cached_waypoints`), and the Pass-1b `step_oscillating_wander` + state copy + `nav_path` construction. They differ only in component types (`WanderBehavior/WanderState` vs `PatrolBehavior/PatrolState`) and the radius field name. `patrol.rs`'s module doc says it avoids a copy of the wander state machine, but only the inner step is shared.
  - The newest movers (`eat_sleep`, `forcegreet`) use `navmesh_path::cached_walk_step` (added `6316719ed`, 2026-10-11), a third idiom for the same resolve-cache / step / write-back sequence.
- **Evidence**: `grep -c 'd.nav_path.take()' byroredux/src/systems/*.rs` -> follow/guard/escort/travel/patrol/wander = 1 each; `git show --stat a58bb4cf6` -> 6 files, +48/-18, one hunk shape.
- **Impact**: the next change to the cache contract (residency generation, #3256; take-not-clone, #4189) is a 6-file edit, and a miss is silent. The #5452 divergence already shows it.
- **Related**: #4189, #3256, #5452, #2033.
- **Suggested Fix**:
  - In `byroredux/src/systems/navmesh_path.rs` (home of `resolve_cached_waypoints` and `cached_walk_step`) add `pub(crate) fn write_back_walk<D>(world, decisions: &mut [D], entity, translation, rotation, nav_path)` taking accessor closures, or a small `trait WalkDecision { fn entity(&self); fn translation(&self); fn rotation(&self); fn take_nav_path(&mut self); fn stuck_secs(&self) }`, and call it from all six Pass-2 tails.
  - Collapse `patrol_system_inner` into a generic `oscillating_walk_system_inner::<B, S>` in `wander.rs` (it already owns `OscillateWalk` and `step_oscillating_wander`), with `patrol.rs` supplying the component pair via a two-method trait. Delete `PatrolDecision`/`PatrolScratch`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **LOCK_ORDER**: If a RwLock scope changes, TypeId-sorted acquisition is preserved
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix


---

# #5581: TD2-2026-10-11-05: `CellData` has no `Default`, so 16 full 33-field literals exist (14 in tests); two were added in the last 24 hours

Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: MEDIUM (table row 1: duplicated construction with a divergent-fix history - #4480 was a red bin-crate test build because six `CellData` fixtures missed a new field, fixed by editing each literal rather than consolidating).
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/plugin/src/esm/cell/mod.rs:191` (struct, 33 fields); literals: `cell/wrld.rs:453`, `cell/walkers.rs:488` (production); tests: `cell/tests/merge.rs:24`, `cell/tests/cell_for_refr.rs:59`, `byroredux/src/cell_loader/exterior.rs:602,779,1049,1705`, `lod_support.rs:371`, `lgtm_fallback_tests.rs:49`, `interior_spawn_tests.rs:73`, `pkin_expansion_tests.rs:41`, `reference_state.rs:1177`, `scene/world_setup.rs:1643`, `systems/story_events.rs:203` (`c85d50356`, 2026-10-11), `systems/eat_sleep.rs:693` (`bcaf63aa8`, 2026-10-10)
- **Status**: NEW (related closed #4480)
- **Age**: each new feature adding a `CellData` field re-edits all of them; `deleted_refs` (2026-08-23) was the last.
- **Effort**: small
- **Description**: `#[derive(Debug)]` only (`mod.rs:190`). Every field type is `Default`-able (`String`, `Option<_>`, `bool`, `Vec<_>`, `CellLighting`/`LandscapeData`/`CellOwnership` behind `Option`, `NavmRecord`/`PathGridRecord` in `Vec`), so a derive is enough. Four of the test constructors are named `empty_cell` with identical bodies up to 2 fields.
- **Evidence**: `grep -rn --include='*.rs' 'deleted_refs:' crates byroredux | grep -v 'pub deleted_refs'` -> 16 literals in 13 files.
- **Impact**: adding the next field is a 16-site edit and a bin-crate test compile break if one is missed - already happened once (#4480), and two new literals arrived in the last day despite it.
- **Related**: #4480, #3864 (same fix for `NifHeader`).
- **Suggested Fix**: `#[derive(Default)]` on `CellData` (plus its field types if any lack it), then rewrite the 14 test literals as `CellData { form_id, grid: Some(..), ..Default::default() }`; keep `wrld.rs`/`walkers.rs` exhaustive-destructure style only if the compiler-checked field coverage is wanted there. Collapse the four `empty_cell` helpers into `CellData::empty(form_id)` under `#[cfg(any(test, feature = "test-fixtures"))]` in `crates/plugin/src/esm/cell/mod.rs`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix


---

# #5582: TD7-2026-10-11-01: The CPU water-wave mirror re-types water.vert's wave tuning and render/water.rs's scroll prep; no shared constant, and the shader allowlist's "no Rust counterpart" claim is now false

Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).


- **Severity**: MEDIUM (duplicated logic with divergent bug-fix history: #3207, #4728, #5354)
- **Dimension**: 7
- **Location**:
  - `crates/physics/src/water.rs:417-478` (`authored_wave_height_with_weather`)
  - `crates/renderer/shaders/water.vert:191-235`
  - `byroredux/src/render/water.rs:238-247,284-289`
  - `crates/renderer/src/shader_constants.rs` (allowlist rows `("water.vert", "DEFAULT_SCROLL_A"/"DEFAULT_SCROLL_B")` under "Water + caustics")
- **Status**: NEW (related closed: #3207, #4728, #5354; open #5562 is about the sample position, not constants)
- **Age**: 11be73600 / a70f80d94, 2026-08-20 ("synchronize buoyancy with weather waves"). Re-touched by cff1f9c42 on 2026-10-11 (#5354).
- **Effort**: small
- **Description**: The physics sampler is documented as "CPU counterpart of `water.vert`". It re-types these values:
  - `DEFAULT_SCROLL_A = 0.0228254` and `DEFAULT_SCROLL_B = 0.0286531`.
  - Rate clamp `0.25..4.0`.
  - Wave mix `0.60 / 0.40`.
  - Phase-B rate `TAU * 0.75` (the shader writes `4.7123890`).
  - Amplitude clamp `32.0`.
  - Spatial floors `1/2048` and `1/4096`.
  - Fallback directions `[1, 0.35]` and `[-0.4, 1]`.
  - The `±32` angular-rate clamp.
  - The flowmap clamp `0.05..8.0`.
  - The weather coupling `0.65` onto scroll B.
  - The `0.5` wind amplitude boost.

  Beyond that, the whole "authored scroll prep" block is duplicated between the GPU upload path (`render/water.rs`) and the CPU sampler: flowmap clamp `0.05..8.0`, angular clamp `±32`, `rotate_scroll`, layer weights `0.65` (B) and `0.45` (C).
- **Evidence**:
  - `physics/water.rs:457-458` has `weather_scroll[0] * 0.65`, and `render/water.rs:284-285` has the same `* 0.65` (and `* 0.45` at `:288-289`).
  - `physics/water.rs:466-468` divides by `0.0228254` / `0.0286531`; `water.vert:205-206` declares the same two numbers.
  - `physics/water.rs:478` has `phase_a.sin() * 0.60 + phase_b.sin() * 0.40`, and `water.vert:224-225` has `sin(phaseA) * 0.60` / `sin(phaseB) * 0.40`.
  - `grep -rn "0.0228254\|0.0286531" crates byroredux` shows the shader, the physics file, tests and fixtures. Nothing in `shader_constants_data.rs`.
  - No Rust test reads `water.vert` for these values. `grep -rn 'water.vert' --include='*.rs'` finds only descriptor-reflection tests and `svgf.rs`'s include.
  - The allowlist comment says each exempted constant has "no Rust counterpart that could drift out of step". That was true before 2026-08-20; it is false for `DEFAULT_SCROLL_A/B` now.
- **Impact**: A retune of any one copy (for example the B-layer weather coupling, or a wave-mix change in the shader) moves the visible crest but not the buoyancy/submersion crest. This is the exact failure #3207, #4728 and #5354 each fixed by hand. The only coherence mechanism is prose comments ("in lockstep with `water.vert`").
- **Related**: #3207 (render/water.rs re-implemented `weather_wave_adjustment`), #4728 (phase sign), #5354, #5562, and `WEATHER_SCROLL_PER_BU_PER_S` / `WaterKind::weather_scroll_for` (the part that was already consolidated).
- **Suggested Fix**:
  - Add the wave-tuning block (default scrolls, rate clamp, `WAVE_MIX_A/B`, phase-B ratio `0.75`, amplitude/angular/flowmap clamps, spatial floors, fallback directions) to `crates/core/src/ecs/components/water.rs`, next to `WEATHER_SCROLL_PER_BU_PER_S`. The scroll-B and scroll-C weather weights (`0.65`, `0.45`) belong there too (WATAL territory).
  - Re-export it through `shader_constants_data.rs`, which already imports from `byroredux_core::ecs::components`, so the GLSL header carries it. Delete the two `DEFAULT_SCROLL_*` allowlist rows.
  - Have `render/water.rs` and `physics/water.rs` call one `authored_scroll_pair(material, weather_scroll, t)` helper that returns `(scroll_a, scroll_b)`, instead of repeating the clamp/rotate/couple code.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **LOCK_ORDER**: If a RwLock scope changes, TypeId-sorted acquisition is preserved
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix


---

# #5583: TD7-2026-10-11-02: `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME = 1366` is a hand-typed copy of a formula that evaluates to 1364, and the formula is re-typed again beside it

Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).


- **Severity**: MEDIUM (the lockstep failure it guards has already shipped twice: #1198, #1284)
- **Dimension**: 7
- **Location**:
  - `crates/renderer/src/vulkan/scene_buffer/constants.rs:69,104`
  - `crates/renderer/src/vulkan/context/mod.rs:81` (`SKIN_MAX_SLOTS`)
  - `byroredux/src/main.rs:1155`
  - 8 test files under `byroredux/src/render/` (`bone_palette_overflow_tests.rs:96`, `variant_pack_gating_tests.rs:15`, `parallax_alpha_gate_tests.rs:56`, `static_mesh_fx_skip_tests.rs:10`, `tint_alpha_gate_tests.rs:28`, `water_wave_params_tests.rs:72,174`, `fog_curve_propagation_tests.rs:13`, `render_origin_shared_tests.rs:17`)
- **Status**: NEW
- **Age**: a3c2836a6, 2026-05-28 (#1284)
- **Effort**: trivial
- **Description**: The slot-pool capacity is `MAX_TOTAL_BONES / MAX_BONES_PER_MESH - 1`, which is 1364 (slot 0 is the identity). It is computed three separate times in non-test code: `SKIN_MAX_SLOTS` (`context/mod.rs:81`), `main.rs:1155`, and the eight test copies. The per-frame first-sight upload cap `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME` is not derived from any of them.
- **Evidence**:
  - `constants.rs:104` has `= 1366`. Its doc says "(= `MAX_TOTAL_BONES (196608) / MAX_BONES_PER_MESH (144)`)". That quotient is 1365.33, floor 1365, and the real pool is 1364. 1366 matches neither.
  - `grep -rn '1366\|1364' --include='*.rs' crates byroredux` finds no test or assert tying the cap to the pool size.
  - History: the cap was 16, then 227, then 1366. Each bump was a hand edit to follow `MAX_TOTAL_BONES`. #1198 records the symptom of letting them diverge: "one-frame bind-pose glitch when more than 16 skinned NPCs first-sighted in a single frame".
  - `core/src/ecs/resources/skin_slot_pool.rs:180` tells the next person to "Bump MAX_TOTAL_BONES". Doing so leaves 1366 stale. Staging size is `1366 × 144 × 64 B` at `buffers.rs:691`. `upload.rs:526` silently defers the excess past the cap, so the failure is a one-frame glitch, not a crash.
- **Impact**: Today the cap is 2 slots over capacity, about 18 KB, so it is harmless. The risk is the documented lever: raise `MAX_TOTAL_BONES` or change `MAX_BONES_PER_MESH` and the upload cap stays at 1366 (under-cap, glitch returns) or over-allocates staging.
- **Related**: #1192, #1198, #1284, #1299 (all closed).
- **Suggested Fix**:
  - Define `pub const MAX_SKIN_SLOTS: usize = MAX_TOTAL_BONES / MAX_BONES_PER_MESH - 1;` in `scene_buffer/constants.rs`.
  - Set `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME = MAX_SKIN_SLOTS`.
  - Define `SKIN_MAX_SLOTS` from the same constant, and have `main.rs:1155` and the eight tests import it instead of re-deriving.
  - Add a `const _: () = assert!(MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME >= MAX_SKIN_SLOTS);`.

### LOW

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **DROP**: If Vulkan objects change, the Drop impl is still reverse-order correct
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix


---

# #5584: TD1-2026-10-11-01: `byroredux/src/env_translate.rs` crossed 2000 production LOC (1881 → 2186, +305 in six days)

Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 1
- **Location**: `byroredux/src/env_translate.rs` (5909 lines, 2186 production; `mod climate_tod_hours_tests;` sibling + 3700 lines of inline tests)
- **Status**: NEW (on the 10-08 report's > 1800 list at 1881)
- **Age**: eight commits, 2026-10-08..10-11: `67afa4b95` #5374 (+86), `302394f94` #5424 (+143), `b3e679dba` #5423 (+102), `217bcbc9f` #5388 (+196),
  `7ef007898` #5343 (+25), `2ca57c4bc` #5496 (+157), `65cadde2f` #5516/17/19 (+89 −94), `e38ff49d2` #5334 (+88).
- **Effort**: medium
- **Description**: the module doc calls it "the exterior environment translation boundary", and it has five contiguous, mostly independent blocks.
  No function is large (biggest 134 LOC), so this is a pure file-level split with no function surgery:
  1. Terrain/object LOD + distant water (L40–420: `TerrainLodLayout`, `translate_terrain_lod_textures`, `default_water_for_worldspace`,
     `resolve_distant_water`, `translate_lod_water`, `lod_source_worldspace`) ≈ 380
  2. Worldspace/climate/default-weather inheritance chain (L414–840: `mod pnam`, `inherit_up_chain`, `resolve_worldspace_climate`,
     `resolve_exterior_climate`, `oblivion_climate_rungs`, `region_climate_for_center`, `resolve_cell_climate`, `resolve_default_weather`,
     `worldspace_name_chain`) ≈ 430 — this is the block that took 5 of the 8 recent commits
  3. WATR → `WaterMaterial` (L843–1490: colours, specular, noise/rain, layer motion, kind/flow classification, `compose_flow_scrolls`,
     `resolve_water_material`) ≈ 650
  4. Sky / sun / cell-lighting / weather translation (L1489–2050: `translate_exterior_cell_lighting`, `translate_sky`, `fog_*`/`precipitation_*`,
     `weather_sky_state`, `climate_tod_hours`, `translate_weather`, `ImageSpaceSources`) ≈ 560
  5. Procedural fallbacks (L2051–2200: `FB_*` constants, `procedural_fallback_{cell_lighting,sky,weather}`) ≈ 150
- **Evidence**: `prod_loc byroredux/src/env_translate.rs` → 2186 (baseline tree: 1881); `grep -nE '^(pub\(crate\) )?fn ' …` shows the five runs above;
  largest fn `resolve_water_noise_and_rain` 134 LOC.
- **Impact**: every EXAL fix (LOD, climate, water, sky) now lands in one file that also hosts 3700 lines of tests; the water-reading half and the
  climate-inheritance half are independent concerns that edit-conflict only because they share a file.
- **Related**: EXAL doc `docs/engine/exal.md` §7; precedent splits `streaming/`, `volumetrics/{fog_clusters,combustion}.rs`. include_str! hazard:
  grep found NO `include_str!` of `env_translate.rs` itself (the `default_weather_rule_tests` module at L5555 reads `exterior.rs`/`world_setup`
  via `strip_test_modules(include_str!(...))` — those are *other* files and are unaffected). So the split is safe w.r.t. self-scan tests.
- **Suggested Fix**: `byroredux/src/env_translate/{mod.rs (docs + `pub(crate) use`), lod.rs, climate.rs, water.rs, sky_weather.rs, fallback.rs}`;
  move the in-file tests with their block (the `tests` mod is 3700 lines — split it along the same five axes, `climate_tod_hours_tests.rs` is
  already a sibling). Keep `mod pnam` private to `climate.rs`/`lod.rs` (it is used by both blocks 1 and 2 — put it in `mod.rs` as `pub(super)`).
  Use the sed-extract line-range method (feedback_safe_large_function_split) and run `cargo fmt` once at the end.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix


---

