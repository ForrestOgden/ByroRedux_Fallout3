**HEAD**: `5d31fd637` · **Baseline**: `AUDIT_RENDERER_2026-10-11.md` (HEAD `334458b3f`) · **Audited**: Dims 2, 3, 5, 6, 8, 9, 10, 11 (delta `334458b3f..5d31fd637`, 148 commits, 11 of them under `crates/renderer`) plus guard runs · **Unchanged since baseline (skimmed)**: Dim 1 (nothing under `acceleration/`, `resources.rs` or `begin_frame_recording.rs`), Dim 4 (no barrier/layout/pass-order edit; only #5619's descriptor-range sizes and comment text), Dim 7 (no `svgf`/`composite`/`taa`/`bloom` commit; only the #5567 fixture/comment fix), Dim 12 (an `egui_pass.rs` comment only)

# Renderer Audit — 2026-10-12 (delta, suite run)

The window is mostly the fixes for yesterday's report plus single-source constant refactors. Every finding from `AUDIT_RENDERER_2026-10-11.md` is now fixed in code, and the one behavioural fix (#5565) holds on a full read. The new findings are three LOW doc-rot items. No renderer regression.

**Method**
- Solo and read-only. No engine or GPU process was launched, per `SUITE_RULES.md`.
- Per dimension: `git log` / `git diff 334458b3f..HEAD` over its Paths, the delta read in full, then the owning guards.
- Dedup against `/tmp/audit/issues.json` (122 open) and against closed issues by keyword. `/tmp/audit/REPORTED_THIS_SUITE.md` was empty at both reads.

**Guards run** (1.96 toolchain, repo `target/`)
- renderer lib, whole run: **1407 passed / 0 failed / 1 ignored** (baseline 1403 / 0 / 1; the ignore is the GPU-only sky-filter test). The run includes `shader_contract`, `acceleration::`, `restir`, `light_history`, the size pins, `exposure_meter`, and the water, DDS and `dynamic_rgba` tests.
- bin `byroredux` whole run: **2829 passed / 0 failed / 58 ignored**. This covers `material_translate`, `cdb`, `render::*`, the `static_blas_recovery` and `skin_dispatch_ran_rollback` guards, `cell_loader::unload` and the `purge_wiring` pins.
- core `--features inspect`, lib filters: `camera` 31, `water` 10, `skin_slot_pool` 28, `resolve_pbr_is_idempotent` 1, `glass_behavior_preserves_authored_map_overlay` 1; all pass.
- physics `water`: 39 / 0. `byroredux-fsr3-sys`: 8 / 0.
- `cross_game_translation_completeness` (`--ignored`, ~3 s): 1 / 1.
- `scripts/check-shader-artifacts.sh`: clean (36 shaders plus the early variant match glslang). `_audit-validate.sh`: all path references valid.
- Not run (needs a free GPU, `#[ignore]`): `cornell_parked_penumbra_converges_at_one_shadow_ray`.

## Executive Summary

| Severity | NEW | Findings |
|---|---|---|
| CRITICAL | 0 | — |
| HIGH | 0 | — |
| MEDIUM | 0 | — |
| LOW | 3 | REN-D2-2026-10-12-01 (the #5249/#5455 docblock now sits on the wrong function), REN-D3-2026-10-12-01 (two ledgers still say 1 366 upload slots; live is 1 364), REN-D6-2026-10-12-01 (the #3903 doc block is glued onto `resolve_through_cdb`) |

**Baseline closure** (each verified against current code, not against the commit message)

| Baseline item | State |
|---|---|
| REN-D2-2026-10-11-01 / #5455: lone glass pane never attenuates the transmission half | **Fixed** by `e3dfd3a31` (#5565). All three lit-returning exits of `traceShadowTransmittanceSkippingInstance` now call `traceShadowGlassLegRemaining`, which hands `traceShadowTransmittance` only `visibilityMask & VISIBILITY_LAYER_GLASS`. With only the glass bit the shared transport's opaque loop sees an empty mask and breaks, and its glass loop runs. The pin `transmission_skip_walk_lit_exits_run_the_glass_leg` slices the function by name, counts exactly three helper returns, bans a bare `return vec3(1.0);` and checks the glass-only argument. GPU confirmation is still outstanding (see Needs-RenderDoc). |
| REN-D2-2026-10-11-02: CPU `vp_abs` rotated up to ~0.45° at \|coord\| > 131k | **Fixed** by `e4433baf8` (#5566). `Camera::view_matrix` builds from the look direction with `look_to_mat4(position, forward, up)`. `assemble_camera`'s `vp_rel` is unchanged. `view_matrix_rotation_is_exact_at_worldspace_magnitude` pins it. |
| REN-D7-2026-10-11-01: ratio-lag fixtures model α = 0.025 | **Fixed** by `aaad8c2e8` (#5567). The fixtures read `parked_steady_state_alpha()` from the generated constants, pin the 330–360-frame half-life against the analytic ceil(ln½ / ln(1−α)), and require the shader comment to say "~346-frame half-life". |
| REN-D2-2026-10-11-04: glam-deprecated names / `interleavedGradientNoise` in docs | **Fixed** by `68790cefc` (#5568). `camera.rs`, `coordinate-system.md` and `shadow-pipeline-tradeoffs.md` §4 were re-read against the shaders; the descriptions match (`ditheredSequence2` at the finalize and non-ReSTIR direct arms, `hash2_pixel_frame` for reservoirs and GI, IGN only for glass scatter and the compiled-out WRS arm). |
| REN-D5-2026-10-11-01: memory-budget egui row | **Fixed** by `3f6c3f7a4` (#5569). |
| REN-D11-2026-10-11-01: CLAUDE.md pre-#5482 presentation order | **Fixed** by `51311e2ab` (#5570). Both `presentation` lines now state exposure → grade → chroma compress → tonemap. |

## RT Pipeline Assessment

**Acceleration structures (Dim 1).** Unchanged. `build_tlas`, `decide_use_update`, scratch sizing and deferred destroy are untouched, and `acceleration::` is green. The `instance_custom_index` contract is not touched by anything in the window.

**Ray queries and shadow rays (Dim 2).**
- #5565's helper is sound. `at` and `remaining` are the live walk state at each exit, `remaining <= 0` and a missing glass bit both return lit, and the glass loop's `advanceShadowRayPastHit` handles the origin offset.
- One residual: the self-skip walk never looks for glass between the original origin and the first own-body hit. The origin sits on the receiver surface, so that span is the body's own thickness and cannot hold a pane. I did not file it.
- The extra glass-only query on the unobstructed exits costs one query per transmission-lobe pixel. `traceLightTransmittanceSkippingInstance` has a single call site (the ReSTIR finalize in `triangle.frag`), gated on a non-zero transmission lobe and `shadowFade > 0.01`. That is not a perf finding.
- #5617 named the ReSTIR EMA constants without changing behaviour. The thresholds match the old literals: `HISTORY_MODE_SCENE_STATIC - 0.5` = 1.5 and `HISTORY_MODE_PARKED - 0.5` = 0.5. `restir_history_mode` returns the same four values. The `const _` asserts pin the half-float exactness ceiling (1024 ≤ 2048), α-floor ordering and mode ordering. `restir_direct_light_ema_constants_come_from_the_generated_header` checks the emitted `#define` lines and bans the bare literals.

**Denoiser (Dim 7).** Unchanged; no SVGF, TAA, bloom or composite code moved.

## GPU-Struct & Memory Assessment

- **Layout (Dim 3):** no GPU struct changed. Size, offset and mirror pins are green. The one new generated-header block (#5617, #5620, #5582) is emitted from `SHADER_DEFINES` and the committed `shader_constants.glsl` matches it.
- **Single-sourced constants (#5620, #5617, #5582):** no residual retyped workgroup width or tile width in the renderer. `local_size_x = N` literals are gone from every `.comp`, and a grep for `div_ceil(8|16|32|64)` finds only a doc comment. The two single-workgroup reductions derive their shared-array sizes, sample stride, first reduction step and dispatch count from `EXPOSURE_METER_WORKGROUP` and `SKY_IRRADIANCE_WORKGROUP`. A power-of-two `const` assert guards both. The water-wave numbers in `water.vert` were checked one by one against the old literals (1/2048, 1/4096, 0.75·2π = 4.712389, 0.60/0.40, 0.25..4.0, 32.0). The CPU crest sampler in `physics/water.rs` and `render/water.rs`'s `wave_scrolls` read the same constants (flow-map clamp 0.05..8, angular clamp ±32, weather weights 1/0.65/0.45).
- **Skin slots (#5583):** `MAX_SKIN_SLOTS = MAX_TOTAL_BONES / MAX_BONES_PER_MESH - 1` = 1364 feeds `SkinSlotPool::new`, `SKIN_MAX_SLOTS` and the upload cap. The highest palette index is 1364·144 + 143 = 196 559, below 196 608. The upload cap drops from 1366 to 1364, shrinking staging by about 18 KB. The ledger rows were not updated (REN-D3-2026-10-12-01).
- **Memory (Dim 5):** no new GPU resource owner. `read_u32` in `dds.rs` became fallible (#5596), and `parse_dds` still length-checks first, so hostile input cannot panic or overrun.
- **Material consumption (Dim 6):** `resolve_through_cdb` is behaviour-equivalent to the four hand copies. The `.mat` arm still records texture provenance and the sidecar arms still do not. `0d698d16c` (#5661) now requires `HasOpacity == Some(true)` for the CDB alpha test. Its default of `false` is derived from a CDB census rather than sourced, which the commit says openly. It is a NIFAL/Starfield call; issue #5661 is still open in the snapshot.
- **Lighting (Dim 10):** `c1a99c25e` places ESM-fallback lights at the NIF's exact-name `AttachLight` node, composed through the full chain with the root transform and length scale. The light entity is a child of `placement_root`, so it picks up `CellRoot` through the `insert_batch` id range and unloads with the cell. `attach_light_flicker_if_needed` now receives the entity's local translation instead of the world `ref_pos`. I found no defect.

## Findings

### REN-D2-2026-10-12-01: the #5249/#5455 docblock now documents the wrong function after #5565 inserted a helper above its subject
- **Severity**: LOW (doc-rot)
- **Dimension**: Soft Shadows (Ray Queries)
- **Location**: `crates/renderer/shaders/include/shadow_transport.glsl`, the comment block that opens "#5249 — visibility of a light segment that ignores the receiver's OWN instance" and the function `traceShadowTransmittanceSkippingInstance` below it.
- **Status**: NEW (introduced by `e3dfd3a31`, #5565)
- **Description**: The long "#5249 … #5455 … #5565" block was written for `traceShadowTransmittanceSkippingInstance`. #5565 appended its paragraph to the block and then inserted the new `traceShadowGlassLegRemaining` between the block and the function. The block now attaches to the helper. The function it describes (self-skip walk, foreign-hit handoff, the exhausted-budget "reports lit" rule) has no leading comment.
- **Evidence**: file order is the comment block, then `vec3 traceShadowGlassLegRemaining(`, then `vec3 traceShadowTransmittanceSkippingInstance(`. The pin test slices both functions by name, so it is unaffected.
- **Impact**: the contract that the #5455 and #5565 fixes rely on is no longer next to the code it constrains, which is where the next editor will look.
- **Related**: #5565, #5455, #5249, #5029 (same splice-into-doc class).
- **Suggested Fix**: move the helper above the block, or give the helper its own two-line comment and leave the block on the walk. `.glsl` comments are invisible to `cargo test`, but the SPIR-V does not change.

### REN-D3-2026-10-12-01: shader-pipeline.md and memory-budget.md still quote the 1 366-slot bind-inverse upload cap; #5583 made it 1 364
- **Severity**: LOW (doc-rot)
- **Dimension**: GPU-Struct Layout / Memory/Lifecycle
- **Location**: `docs/engine/shader-pipeline.md` (constants table row `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME`, "1 366"); `docs/engine/memory-budget.md` (footnote ¹ to the bone-buffer table, "`1 366 × MAX_BONES_PER_MESH(144) × 64 B ≈ 12.6 MB`").
- **Status**: NEW (introduced by `b7d40dc81`, #5583)
- **Description**: #5583 defined `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME = MAX_SKIN_SLOTS` = 1364. The Rust docs and `main.rs` comments were updated; the two reference docs were not. `shader-pipeline.md` still tells a reader the cap differs from the 1 364 allocatable slots listed one row above it, which was true before #5583 and is the exact mismatch #5583 removed.
- **Evidence**: `MAX_SKIN_SLOTS = 196608 / 144 - 1` = 1364 (`scene_buffer/constants.rs`); `docs/engine/shader-pipeline.md` row directly below `MAX_TOTAL_BONES` quotes 1 366. `1364 × 144 × 64 B` = 12.57 MB, so the "≈ 12.6 MB" figure survives, only the multiplier is stale.
- **Impact**: documentation only. The skill's `gpu_material_size_claims` guard does not cover these two docs.
- **Related**: #5583, #1284, #5603 (docs catch-up family).
- **Suggested Fix**: write "1 364 (= `MAX_SKIN_SLOTS`)" in both places and say the cap is the slot pool's capacity by construction.

### REN-D6-2026-10-12-01: `resolve_through_cdb` was inserted between `record_external_texture_sources` and its doc comment
- **Severity**: LOW (doc-rot)
- **Dimension**: NIFAL Material
- **Location**: `byroredux/src/asset_provider/material/merge.rs`, the doc block ending "where the old loop's separate index walk lived." and the `/// #5284 — the CDB resolve tail, once.` block, `fn resolve_through_cdb`, then `fn record_external_texture_sources`.
- **Status**: NEW (introduced by `c961653f4`, #5284)
- **Description**: The existing doc comment for `record_external_texture_sources` (the #3903 `zip_map_ref` rationale) ran straight into the new `/// #5284 …` lines, so rustdoc glues both into one comment on `resolve_through_cdb`. `record_external_texture_sources` is left with no doc comment, and `resolve_through_cdb`'s docs open with an unrelated paragraph about texture provenance roles.
- **Evidence**: no blank or non-doc line separates the two `///` blocks; the next item after them is `fn resolve_through_cdb(`; `fn record_external_texture_sources(` follows the closing brace with no `///` above it.
- **Impact**: rustdoc and anyone reading `merge.rs` see the wrong description on both functions. Code behaviour is unaffected.
- **Related**: #5284, #3903, #5029 (the same class of splice).
- **Suggested Fix**: move the #3903 block down to sit directly above `record_external_texture_sources`.

## Prioritized Fix Order

1. **Correctness:** nothing open. #5565 and #5566 are verified fixed in code.
2. **Guard honesty:** nothing new. The #5565 pin is a source-shape pin, so GPU confirmation is the remaining gap (below).
3. **Docs (batch into one doc-rot issue):** REN-D3-2026-10-12-01, REN-D2-2026-10-12-01, REN-D6-2026-10-12-01.

## Needs-RenderDoc

- **#5565 glass leg, GPU check:** the fix is verified by reading and by the source-shape pin only. A Cornell glass-pane plus back-lit translucent probe would show the transmission half now dimming behind a pane. No GPU process was launched.
- **TLAS UPDATE selection (carried):** `decide_use_update` picks UPDATE when the BLAS-address layout and the entity-id sequence both match; its own doc says SSBO indices may change freely with raster order. Whether UPDATE may carry a changed `instanceCustomIndex` was not checked against a spec VUID. A `BYRO_VALIDATION=1` run on a parked interior with a walking NPC settles it.
- **Dither screen-locking (carried):** the per-pixel mask with an identical R2 step per pixel is screen-locked while the camera moves (moving α = 0.1, cap 16). It is a visual call on a captured sequence.
- **egui 0.13 against the overlay pass (carried):** a `BYRO_VALIDATION=1` run.
- **Carried, still unexecuted:** the #5187, #5188 and #5194 fault-injection runs, and `cornell_parked_penumbra_converges_at_one_shadow_ray --ignored` (covers history mode 2 only; mode 3 has no GPU gate).

## Stale skill premises (for the next `/audit-renderer` sync)

- **Dim 2, `traceShadowTransmittanceSkippingInstance` bullet:** the skill is current on #5455/#5565. Add that the glass handoff is now `traceShadowGlassLegRemaining` on all three lit exits, pinned by `transmission_skip_walk_lit_exits_run_the_glass_leg`; and that the docblock above it is displaced (REN-D2-2026-10-12-01).
- **Dim 9 / Dim 3 capacity bullet:** add `scene_buffer::MAX_SKIN_SLOTS` (1364) as the single source for `SkinSlotPool::new`, `SKIN_MAX_SLOTS` and `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME` (#5583), and the two reference docs that still say 1 366 (REN-D3-2026-10-12-01).
- **Dim 10, light canonical translation:** add the `AttachLight` placement (`find_attach_light_offset`, exact name, composed chain; `spawn_mesh_instance` parents the ESM-fallback light to `placement_root`) and that flicker `base_translation` is the entity's local translation (`c1a99c25e`).
- **Dim 6, bullet 2:** `apply_cdb_material` gates the alpha test on `HasOpacity == Some(true)` (#5661, `0d698d16c`; the default is census-measured, not sourced). Add `resolve_through_cdb` as the one CDB tail with a one-call-site pin (#5284).
- **Dim 11 / Dim 12 constants bullet:** `EXPOSURE_METER_SAMPLE_GRID` (64) is a generated constant beside `EXPOSURE_METER_WORKGROUP`; `WORKGROUP_X`/`WORKGROUP_Y` also drive the sky-filter and ground-cover bench-bake dispatch (`single_workgroup_reductions_derive_from_the_generated_constants`).
- **Dim 8:** the CPU wave-scroll prep is `WaterMaterial::wave_scrolls` and the per-kind damping is `WaterKind::weather_scroll_for` (#5582, #5354); the skill's #5582 sentence is correct but does not name them.

## Guard posture

| Dim | Guards confirmed green | Blind spot |
|---|---|---|
| 1 | `acceleration::` inside the 1407-test lib run, bin static-BLAS recovery | TLAS UPDATE vs shifted custom index is validation-only |
| 2/10 | `shader_contract`, `restir`, `light_history`, bin lights/camera | the glass leg is a source-shape pin; no GPU oracle |
| 3 | size/offset/mirror pins, `gpu_material_size_claims` | reference docs for the upload cap are not scanned |
| 4/5/12 | whole lib run, `frames_in_flight_contract_names_every_dependent_resource`, DDS/`dynamic_rgba` tests | no test pins the egui discard sites (carried) |
| 6 | whole bin run, core `inspect` filters, completeness `--ignored` | — |
| 7 | whole lib run | mode-3 responsiveness has no GPU gate |
| 8/9 | water tests (renderer, core, physics), `skin_slot_pool` | — |
| 11 | `exposure_meter`, `presentation`, `fsr3-sys` | FP32 FSR permutation untested (carried) |

## Process notes

**Dedup.** Open issues from `/tmp/audit/issues.json` (122 open). Keyword searches over open titles: renderer terms, `doc comment`, `1366`, `merge.rs`, `shadow_transport`. Closed-issue search: `MAX_PENDING_BIND_INVERSE` (no match), `record_external_texture_sources` doc (no match). No overlap with open issues or with `/tmp/audit/REPORTED_THIS_SUITE.md`. Related open: #5661 (the `HasOpacity` default, now addressed in `0d698d16c` but still open in the snapshot), #5365, #5594.

**Dropped candidates.**
- The extra per-pixel glass query on the walk's lit exits (see above): bounded to transmission-lobe pixels, not a defect.
- Glass between the ray origin and the first own-body hit: the span is the receiver's own thickness.
- The `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME >= MAX_SKIN_SLOTS` const-assert is tautological since the first is defined as the second. The comment states the intent and nothing can drift, so it is not worth a finding.
- Not audited: `cell_loader/water.rs` (distant water, #5334), `env_translate/` (split #5584) and the NIF collision rewrite; they belong to `/audit-exterior` and `/audit-physics`.

To publish: `/audit-publish docs/audits/AUDIT_RENDERER_2026-10-12.md`. Suggested labels: `renderer`, `doc-rot` for all three (batch into one issue); add `shaders` for REN-D2-2026-10-12-01.
