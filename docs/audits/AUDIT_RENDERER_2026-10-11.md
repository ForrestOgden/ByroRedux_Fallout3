**HEAD**: `334458b3f` · **Baseline**: `AUDIT_RENDERER_2026-10-09.md` (HEAD `3bcf6c8e8`) · **Audited**: Dims 2, 3, 4, 5, 6, 7, 10, 11, 12 (delta `3bcf6c8e8..334458b3f`) plus guard runs and skims of Dims 1, 8, 9 · **Unchanged since baseline (skimmed)**: Dim 1 (nothing under `acceleration/`), Dim 8 (no renderer-path commits; the `a851c625d` rig key and a smoothstep swap only), Dim 9 (test-scan routing only), Dim 10 (own paths untouched; its two findings are the Dim 2 ones cross-listed)

# Renderer Audit — 2026-10-11 (delta, 116 commits)

The window is dominated by dependency bumps (glam 0.29→0.33 `c6b8d4584`, egui 0.36 / egui-ash-renderer 0.13 `9767acd0a`), the ReSTIR/shadow-ray work of #5369 (`4d991bad7` dither, `48bed0504` parked tail 1024 / 0.002, `a851c625d` order-independent rig key), and fixes for the previous report's findings (#5455, #5457, #5402, #5506, #5507, #5528, #5532, #5503).

**Method**
- Nine dimensions ran as read-only agents in three waves (max 3 concurrent). Each read the diffs of its delta commits and ran its guards.
- No engine, GPU process or validation run was launched.
- The one finding above LOW-adjacent worth acting on (REN-D2-2026-10-11-01) was re-verified by the orchestrator against `traceShadowTransmittanceSkippingInstance` and its caller `traceLightTransmittanceSkippingInstance`.

**Guards run**
- renderer lib, whole run: **1403 passed / 0 failed / 1 ignored** (baseline 1394 / 0 / 1; the ignore is the GPU-only sky-filter test).
- `shader_contract`: 128 / 0. `acceleration::`: 145 / 0.
- bin crate (rustc 1.96) filtered runs: lights / light_anim / camera / frustum 107; `material_translate` 77; `cdb` 28; `static_blas_recovery` + `bone_palette_overflow` + `skin_dispatch_ran_rollback` 6; `bench_gpu_key` / `cornell` / `mat_set_tests` all pass. `renderer_config_defaults_to_fsr_quality` passes.
- core `--features inspect` (`resolve_pbr_is_idempotent`, `glass_behavior_preserves_authored_map_overlay`): pass. `skin_slot_pool`: 28 / 0.
- `byroredux-fsr3-sys`: 8 / 0. `byroredux-debug-ui`: 19 / 0.
- `cross_game_translation_completeness` (`--ignored`): 1 / 1 in 2.4 s, all 8 games probed, none skipped.
- Not run (needs a free GPU, `#[ignore]`): `cornell_parked_penumbra_converges_at_one_shadow_ray`. It gates only history mode 2.
- `scripts/check-shader-artifacts.sh`: clean (36 shaders plus the early variant).

## Executive Summary

| Severity | NEW | Findings |
|---|---|---|
| CRITICAL | 0 | — |
| HIGH | 0 | — |
| MEDIUM | 0 | — |
| LOW | 6 | D2-01 (regression of #5455: lone glass pane never attenuates the transmission half), D2-02 (CPU absolute view-projection loses direction precision at |coord| > 131k), D7-01 (ratio-lag fixtures model α = 0.025, shipped 0.002; also cited as D2-03), D2-04 (glam-deprecated names and `interleavedGradientNoise` in comments/docs), D5-01 (memory-budget egui row stale), D11-01 (CLAUDE.md pre-#5482 presentation order) |

**Baseline closure**

| Baseline item | State |
|---|---|
| #5455 (transmission foreign-hit mask) | Fix `6d64634aa` present and pinned, **but incomplete**: it only reaches the foreign-opaque-hit leg (D2-01) |
| #5457 (stale SAFETY text) | **Fixed** (`cc8a5df8f`; text matches the `as_chunks` decode) |
| #5402 (L8 / L8A8 / L16 DDS) | **Fixed** (`9b19aa815`; payload-length check, bounded mips, saturating size math; hostile input cannot panic or overrun) |
| #5506 (contrast toe crossing zero) | **Fixed** (`38f711a93`; GLSL and Rust `grade_contrast` algebraically identical; a 2M-point sweep over contrast 0–8 is always positive, monotone to ≈ 2.25, sub-1e-5 dips above ≈ 2.4 — below the authored maximum) |
| #5507 (rig key order dependence) | **Fixed** (`a851c625d`; commutative wrapping-add, six-permutation test) |
| #5532 (camera-UBO mirrors / presentation docs) | **Fixed** for the five mirrors, `shader-pipeline.md`, `renderer.md`; `CLAUDE.md` was missed (D11-01) |
| #5528 (staging-arena growth retry) | **Fixed** (`e588e5e67`; 120 growth-needing frames of back-off, immediate retry for a smaller request, cleared on success; warn flag untouched) |
| #5503 (Starfield emissive raw forward) | **Fixed** (`4667b884b`; arm parked, `cdb_emissive_settings_are_parked_not_forwarded_raw` non-vacuous) |

## RT Pipeline Assessment

**Acceleration structures (Dim 1).** Unchanged. The glam bump does not reach the TLAS transform conversion (`[f32;16]` from `to_cols_array()`); there is no `transmute` / `bytemuck` / `from_raw_parts` over a `Mat4` or `Quat` in the AS, `context/` or `render/` paths. Scratch sizing `max(build, update)`, deferred destroy and the `instance_custom_index` contract hold.

**Ray queries and shadow rays (Dim 2/10).**
- The dither (`4d991bad7`) is clean: a pure function of (pixel, frame counter, ray index, light index); the R2 constants are correct; the frame counter is exact in f32; the zero-budget path clamps to ≥ 1 ray; point, spot and directional lights share the sampler; no NaN path.
- The deeper parked tail (`48bed0504`) is internally consistent: the `histPrev` clamp matches the 1024 cap, half-float packing is exact to 2048, and the accumulator is full f32 so α = 0.002 does not stall on quantisation. α reaches its floor at histLen 500, so the cap above 500 never changes α.
- The transmission self-skip walk has a real gap (D2-01).

**Camera/glam (Dim 2).** `look_at_mat4` and `directx::perspective` are bit-equivalent to the old `look_at_rh` / `perspective_rh` (z row differs by ≤ 1 ulp); the engine's Y flip and the reversed-Z plane swap still route through the same function; argument order matches at every call site. `glam_assert!` is compiled out (workspace enables only `mint`), so the failure mode (centre == eye) is NaN as before.

**Denoiser (Dim 7).** No SVGF coupling from `48bed0504`: SVGF keys on the intensity-aware caustic key, ReSTIR on the intensity-blind rig key, and both share the same occluder gates. The only gap is a test/doc one (D7-01).

## GPU-Struct & Memory Assessment

- **Layout (Dim 3):** clean. The #5532 sweep landed in all five `dofParams` mirrors, the `GpuCamera` rustdoc, `restir_history_mode` and the `shader-pipeline.md` row; the pin test now also requires the mode-3 substring. It is still a literal match, so a mode 4 would pass silently until the test and the five comments are edited together. `gpu_material_size_claims` is live and every size quoted in `.claude/commands` is correct. The reservoir stride is unchanged.
- **Memory (Dim 5):** clean. No new GPU resource owners since 2026-10-09, so no ledger row is missing. Only the egui ledger prose is stale (D5-01).
- **egui 0.36 / 0.13 (Dims 4, 12):** all five paths that drop an egui frame unrendered (no overlay pass, mid-frame quit, teardown, skipped-frame merge, early return from `dispatch`) now discard the pending texture deltas through `drop_without_applying_deltas`. Texture retirement is safe only because `sync_and_acquire_frame` waits on all in-flight fences, and its guard passes. No test pins the discard sites themselves (the failure mode is a debug-build-only panic).
- **Material consumption (Dim 6):** clean. Every `parallaxMapIndex` reader masks bit 31 (both POM marchers); `perturbNormalGrad` keeps its zero-length guards (#5199).

## Findings

### REN-D2-2026-10-11-01: #5455's fix only reaches the foreign-opaque-hit leg — a glass pane alone still never attenuates the transmission half
- **Severity**: LOW (visual only; same severity #5455 was filed at)
- **Dimension**: Soft Shadows (Ray Queries). Cross-listed by Dim 10 as REN-D10-2026-10-11-01 — publish once.
- **Location**: `traceShadowTransmittanceSkippingInstance` in `crates/renderer/shaders/include/shadow_transport.glsl` (the `NoneIntersection` early return and the two other `return vec3(1.0)` exits); caller `traceLightTransmittanceSkippingInstance` in `lighting.glsl`; pin `transmission_skip_walk_hands_the_full_mask_to_the_foreign_leg` in `scene_buffer/shader_contract_tests.rs`.
- **Status**: Regression of #5455 (closed; its repro scenario is not actually fixed)
- **Description**: `6d64634aa` hands the full `visibilityMask` to the foreign-hit handoff `traceShadowTransmittance(at, …, visibilityMask)`. The skip walk that precedes it queries `skipRQ` with `mask = visibilityMask & VISIBILITY_MASK_ALL_OPAQUE` (15u), and glass instances carry `VISIBILITY_LAYER_GLASS` (16u, outside `ALL_OPAQUE`). The walk therefore cannot see a pane. The glass-aware handoff runs only when the walk finds a foreign *opaque* hit. If the segment light → receiver contains the receiver's own body and a window pane but no other opaque blocker, the walk reports `gl_RayQueryCommittedIntersectionNoneEXT` and returns `vec3(1.0)`. The same holds for the budget-exhausted exit and the `advanceShadowRayPastHit` false exit.
- **Evidence** (re-verified by the orchestrator): `uint mask = visibilityMask & VISIBILITY_MASK_ALL_OPAQUE; // skip walk only` → `rayQueryInitializeEXT(skipRQ, topLevelAS, gl_RayFlagsOpaqueEXT, mask, …)` → `if (… == gl_RayQueryCommittedIntersectionNoneEXT) { return vec3(1.0); }`. `VISIBILITY_LAYER_GLASS 16u` and `VISIBILITY_MASK_ALL_OPAQUE 15u` in `include/shader_constants.glsl`. #5455's own repro (a back-lit surface behind a window pane) is the no-foreign-opaque case, so it is unchanged. The new source pin checks the handoff argument text only.
- **Impact**: Skyrim back-light, FO4 translucency and foliage transmission lobes ignore windows and glass partitions between the light and the receiver, while the reflection half of the same pixel is attenuated by the pane. The two halves disagree.
- **Related**: #5455, #5249, #5192, #4946.
- **Suggested Fix**: On the lit-returning exits, when `visibilityMask & VISIBILITY_LAYER_GLASS` is set, finish with `traceShadowTransmittance(at, direction, remaining, emitterRadius, visibilityMask & VISIBILITY_LAYER_GLASS)`. Extend the pin so the lit exits are not bare `return vec3(1.0)` when the mask carries the glass bit. The GPU result needs a Cornell glass-pane oracle (**needs RenderDoc / GPU**).

### REN-D2-2026-10-11-02: CPU absolute view-projection is rebuilt from `position + forward` at world magnitude, rotating the cull frustum up to ~0.3–0.45° at |coord| > 131k
- **Severity**: LOW
- **Dimension**: Camera-Relative Precision
- **Location**: `Camera::view_matrix` in `crates/core/src/ecs/components/camera.rs` → `look_at_mat4(position, position + forward, up)`; consumed by `assemble_camera` in `byroredux/src/render/camera.rs` as `vp_abs` → `FrustumPlanes::from_view_proj` and `vp_mat`.
- **Status**: NEW (pre-existing; the glam swap is bit-equivalent to the old `look_at_rh`)
- **Description**: `look_at_mat4` derives the direction as `(center - eye).normalize()`. With `center` formed in f32 at Markarth-scale coordinates (X ≈ −176 000, ulp 2⁻⁶), `forward` is quantised to ~0.0156 per component before normalisation. The GPU matrix is unaffected (`vp_rel` is built from the camera-relative eye); the CPU frustum planes and `vp_mat` use `vp_abs`.
- **Evidence**: an f32 simulation at eye = (−176000.3, 500.1, 121000.7) gives a direction error of 0.0° at yaw 0, 0.14° at 20°, 0.31° at 37°, worst case ≈ 0.45° (≈ 0.22° for 65k–131k). `contains_sphere` has no padding term. The agent's simulation is not backed by a checked-in test.
- **Impact**: cull edges are rotated against the rendered view by ≈ 6 px at 1080p / 0.3°. Small objects straddling the left/right screen edge can pop in late. Sort depth and particle clip use the same matrix (negligible).
- **Related**: #2744 (closed; same class of defect on the GPU cluster-cull side).
- **Suggested Fix**: Build `vp_abs` as `vp_rel * Mat4::from_translation(-render_origin)`, or use `look_to_mat4(position, forward, up)` so the direction never passes through an absolute-magnitude subtraction.

### REN-D7-2026-10-11-01: the ReSTIR direct-EMA lag fixtures still model the retired α = 0.025 parked tail; the live deep tail (1024 / 0.002) fades a switched-off light ~13× slower
- **Severity**: LOW (test-gap / doc-rot). Dim 2 reported the same defect as REN-D2-2026-10-11-03 — publish once.
- **Dimension**: Denoise & resolve chain / Soft Shadows
- **Location**: `mixed_visibility_clusters_keep_a_documented_ratio_lag` and the flicker fixture above it in `crates/renderer/src/vulkan/restir.rs` (both hard-code `let alpha = 0.025f32`); the `restirUnshadowedSum` comment in `crates/renderer/shaders/triangle.frag` ("fades over the EMA window"); the doc of `light_rig_geometry_key_is_independent_of_light_order` in `crates/renderer/src/vulkan/caustic.rs` ("parked tail 256 -> 16").
- **Status**: NEW (adjacent to closed #5020 and #5369 and open #5370; consequence of `48bed0504`)
- **Description**: The shader's parked branch is `alphaFloor = sceneStatic ? 0.002 : 0.1` with cap 1024. Mode 3 stays in the deep tail through *any* intensity change by design (the rig key is intensity-blind). The #5020 limit (the EMA stores one whole-cluster ratio) is therefore governed by α = 0.002, not 0.025. The fixtures assert only `lit[19] > 0.25` after 20 frames, true for any α ≲ 0.03.
- **Evidence**: re-simulating the fixture's own model at the live constants: flicker pass-through stays ≈ 48 % (0.490 @ 0.025, 0.484 @ 0.002, so the shader's "~48 %" text is still right), but the switched-off fade half-life is 27 frames @ 0.025, 86 @ 0.008, **346 @ 0.002** (frames to a quarter: 55 / 173 / 693).
- **Impact**: visual transient only, in a narrow case: a multi-light cluster with differing visibility plus a light whose intensity animates to or through zero while staying in the list. At 60 fps that is a ~6 s half-life of a phantom 50 % contribution from the occluded neighbour; at 300 fps ~1.2 s. Penumbra convergence (the reason for the change) is unaffected. The risk is that the pinned model is 13× off, so a later change cannot be judged against it.
- **Related**: #5020, #5369, #5370 (open), #5507.
- **Suggested Fix**: Make the two fixtures read the live constants (parse the literal out of `triangle.frag`, or share one `const`), state the half-life in the `restirUnshadowedSum` comment, and refresh the `caustic.rs` doc to 1024 → 16. If the multi-second step lag is unacceptable, resetting `histLen` on a large per-frame change in the unshadowed sum is a design call, not a drive-by.

### REN-D2-2026-10-11-04: glam-deprecated names and a stale noise-seed description remain in comments and docs
- **Severity**: LOW (doc-rot)
- **Dimension**: Camera-Relative Precision / Soft Shadows
- **Location**: `crates/core/src/ecs/components/camera.rs` (doc comments at the `perspective_rh` mentions and the inline "glam's perspective_rh already maps Z…" comment); `docs/engine/coordinate-system.md` (shows `Mat4::perspective_rh`); `docs/engine/shadow-pipeline-tradeoffs.md` §4 (`cameraPos.w` described as the seed for `interleavedGradientNoise` in shadow-ray jitter).
- **Status**: NEW (the `frame_params.rs` "look_at_rh call" comments the agent cited did not reproduce in a grep of `byroredux/src/render/`; those two are unverified)
- **Description**: glam 0.33 deprecated `Mat4::look_at_rh` / `perspective_rh`; the producers are now `camera::rh::view::look_at_mat4` and `camera::rh::proj::directx::perspective`. Since `4d991bad7` the finalize shadow-ray disk samples use `ditheredSequence2` (R2 in time, frame-free `ignMask` rotation per pixel), not `interleavedGradientNoise` seeded from `cameraPos.w`.
- **Impact**: doc rot only.
- **Related**: `c6b8d4584`, `4d991bad7`, `f4c2b9931` (the dependency-refresh comment sweep that missed these).
- **Suggested Fix**: Rename the references to the live producers and rewrite §4 of `shadow-pipeline-tradeoffs.md` around `ditheredSequence2`.

### REN-D5-2026-10-11-01: memory-budget.md egui row describes a retained retired image and cites egui-ash-renderer 0.11
- **Severity**: LOW (documentation drift)
- **Dimension**: Memory/Lifecycle
- **Location**: `docs/engine/memory-budget.md`, section "egui debug overlay (`EguiPass`) — #4986" (the "Retired image awaiting `pending_free`" row and the "egui-ash-renderer 0.11" prose); code side `crates/renderer/src/vulkan/egui_pass.rs` (`promote_partial_deltas`, `dispatch`) and `sync.rs` rider 14.
- **Status**: NEW (related to closed #5026)
- **Description**: In egui-ash-renderer 0.11 and 0.13, `Renderer::set_texture`'s full-upload path creates the new image and descriptor set, then destroys the previous image and frees its descriptor set inside the same call. `pending_free` carries only egui's own `TexturesDelta.free` ids. `sync.rs` rider 14 already states this correctly.
- **Impact**: overstates steady-state VRAM by one atlas-sized image (a few MiB); the real peak is transient (old plus new resident only during the call).
- **Related**: #5026, #4986, #4988.
- **Suggested Fix**: Replace the row with "transient: old image destroyed inside `set_texture` once the replacement exists (peak = 2 images during the call)" and change "0.11" to "0.11/0.13".

### REN-D11-2026-10-11-01: CLAUDE.md still states the pre-#5482 presentation order
- **Severity**: LOW (documentation)
- **Dimension**: FSR/Presentation
- **Location**: `CLAUDE.md`, the `src/vulkan/presentation.rs` line of the workspace tree ("output-res tonemap(graded × exposureTex)").
- **Status**: NEW (sibling sweep gap of closed #5532)
- **Description**: Since #5482 `presentation.frag` computes `exposed = scene.rgb * exposure` first, grades the exposed radiance (saturation → `gradeContrast` → brightness → tint), applies the #5154 chroma compress, and only then `tonemap(compressed)`. `shader-pipeline.md` and `renderer.md` already carry the new order; `CLAUDE.md` is the doc loaded at every session start.
- **Impact**: misleads anyone reasoning about where contrast pivots relative to exposure.
- **Related**: #5482, #5532, #5219.
- **Suggested Fix**: "Presentation pass — output-res exposure → IMGS/IMAD grade → chroma compress → tonemap (ACES|AgX), underwater extinction, swapchain write". (Editing `CLAUDE.md` is the user's call; not done by this audit.)

## Prioritized Fix Order

1. **Correctness:** D2-01 (finish the glass leg of #5455; the shader change is small, the verification needs a Cornell glass-pane probe). Then D2-02 (one-line camera-relative construction of `vp_abs`).
2. **Guard honesty:** D7-01 (tie the lag fixtures to the shader constants so the next tail change is judged against the real model).
3. **Docs:** D11-01, D5-01, D2-04 (batch into one doc-rot issue).

## Needs-RenderDoc

- **D2-01:** a Cornell glass-pane + back-lit translucent probe to confirm the visual effect (no GPU process was launched).
- **Dither screen-locking:** the per-pixel static mask with an identical R2 increment per pixel makes the noise pattern screen-locked while the camera moves (moving-mode α = 0.1, cap 16). Whether it reads as dither or a dirty-window pattern under TAA/FSR is a visual call on a captured sequence; no code defect found.
- **egui 0.13 against the overlay render pass:** a `BYRO_VALIDATION=1` run (not performed — the user may have an instance up).
- **TLAS UPDATE selection:** UPDATE is chosen when entity membership matches even if an instance custom index shifts. The agent did not check this against a spec VUID; a `BYRO_VALIDATION=1` run on a parked interior with a walking NPC would settle it.
- **Carried, still unexecuted:** the #5187, #5188 and #5194 fault-injection runs; `cornell_parked_penumbra_converges_at_one_shadow_ray --ignored` (covers mode 2 only; mode 3, the flicker mode, has no gate).

## Stale skill premises (for the next `/audit-renderer` sync)

- **Dim 2, noise bullet:** "ReSTIR and GI streams use the integer PCG `hash2_pixel_frame`; IGN survives only in glass roughness scatter and the legacy arm". Since `4d991bad7` the finalize shadow-ray disk samples (and the legacy arm's shadow pair) use `ditheredSequence2`. Add the pin `shadow_ray_disk_samples_are_dithered_low_discrepancy`. Dim 10's "Sun soft shadow" bullet needs the same edit.
- **Dim 2, ReSTIR-DI bullet:** "a 256 / 0.008 parked tail" → **1024 / 0.002** on modes 2 and 3 (`histPrev` clamp 1024; α reaches its floor at histLen 500). The literals now ride `clamp(rpHistLen, 0.0, 1024.0)`, `sceneStatic ? 1024.0 : 16.0` and `sceneStatic ? 0.002 : 0.1` in `temporal_reuse_validates_surface_depth_and_does_not_seed_neighbor_radiance`.
- **Dim 2 / Dim 10, `traceShadowTransmittanceSkippingInstance`:** "defers to the alpha/glass-aware trace on the first foreign hit" is true only for a foreign *opaque* hit. Do not list #5455 as fully fixed (D2-01). Guard additions: `transmission_skip_walk_hands_the_full_mask_to_the_foreign_leg`, `mixed_visibility_clusters_keep_a_documented_ratio_lag` (note its stale α mirror), `cornell_parked_penumbra_converges_at_one_shadow_ray` (ignored).
- **Dim 2, camera path:** the producers are now `camera::rh::view::look_at_mat4` and `camera::rh::proj::directx::perspective`, not `look_at_rh` / `perspective_rh`.
- **Dim 3, lane-semantics bullet:** "a prefix match that cannot see a new mode; the GLSL mirror comments still omit mode 3, #5532" — both halves stale. Replace with "literal-substring match per mirror (modes 0–3); a mode 4 needs the test and the five comments edited together".
- **Dim 5, textures bullet:** "#5528, open" → fixed (`e588e5e67`). Add the guards `growth_retries_back_off_during_a_failure_episode` and `growth_gate_sits_before_the_allocation` (`dynamic_rgba.rs`).
- **Dim 6, bullet 2:** the CDB arm no longer forwards `EmissiveSettingsComponent` (#5503 parked it; physical luminance has no measured mapping, `nifal.md` §4 Starfield note). The regression to guard is a re-introduced raw `luminous_emittance` → `emissive_mult` forward, pinned by `cdb_emissive_settings_are_parked_not_forwarded_raw`. The same stale claim sits in `.claude/commands/audit-nifal/SKILL.md` (line 163, which also lists #5503 as known-open) and `.claude/commands/audit-starfield/SKILL.md` (lines 66 and 111). Open #5525 (loose `.mat` parser drops the component) is moot while the arm is parked.
- **Dim 8:** add one sentence that `light_rig_geometry_key` (commutative, intensity-blind, #5507) lives in `caustic.rs` beside the every-float, order-sensitive caustic key, and that the asymmetry is deliberate.
- **Dim 11, Order bullet:** "Known open: #5506 … #5532" — both fixed. Replace with: the toe weights on the stretched value (`stretched / TOE`), is positive for all contrast ≥ 0 and monotone to ≈ 2.25, pinned by `contrast_keeps_shadows_positive_and_monotone_across_the_census_range`. Docs carry the new order everywhere except `CLAUDE.md` (D11-01).
- **Dim 12, egui bullet:** name the five discard sites and `drop_without_applying_deltas`; note no test pins the three non-render discard sites.
- **Dim 1 (cosmetic):** the bin guard `static_blas_recovery_runs_between_frames_not_in_the_render_driver` lives in the module `app_step::bench_subject_distance_tests`. The filter text is still correct.

## Guard posture

| Dim | Guards confirmed green | Blind spot |
|---|---|---|
| 1 | 145 `acceleration::` tests + bin static-BLAS recovery | TLAS UPDATE vs shifted custom index is validation-only |
| 2/10 | `shader_contract` 128, restir/light_history/rig-key sweep 52, bin lights/camera 107 | skip-walk glass handoff is a source-shape pin (D2-01); lag fixtures use a stale α (D7-01) |
| 3 | sizes, offsets, mirrors, `gpu_material_size_claims` | mirror pin is a per-mode substring match |
| 4/5/12 | 1403-test lib run, `frames_in_flight_contract_names_every_dependent_resource`, DDS luminance fixtures, #5528 back-off tests | no test pins the egui discard sites |
| 6 | 77 `material_translate`, 28 `cdb`, parallax / tangent / perturb filters, vanilla-data completeness | — |
| 7 | `temporal_pass_sanitizes_non_finite_samples_on_every_history_path` | mode-3 responsiveness has no gate |
| 8/9 | volumetrics/caustic latch tests, palette dirty-plan, refit | — |
| 11 | `contrast_keeps_shadows_positive_and_monotone_across_the_census_range`, GLSL source pin, `renderer_config_defaults_to_fsr_quality` | sub-1e-5 contrast dips above ≈ 2.4 (not reported) |

## Process notes

**Dedup.** Open issues came from `/tmp/audit/renderer/issues.json` (162 open). All-state searches: "glass transmission shadow", "ratio lag parked", "look_at precision frustum", "memory-budget egui", "CLAUDE.md presentation order", "5455". Matches: #5455 (closed), #5249 (closed), #5020 (closed), #5370 (open), #5026 (closed), #5482 / #5532 (closed). "look_at precision frustum" matched nothing.

**Dropped candidates.**
- A latent leak in the swapchain-format-change rebuild: an egui mirror whose texture was pending free gets re-uploaded and never freed. Unreachable today because the engine creates no egui user textures. Noted in `dim_5.md`.
- `contrast` dips of at most ≈ 5e-6 above ≈ 2.4 — beyond the authored maximum and below any visible threshold.
- Per-light rotation salt in the dither (`salt = i`) and `n` re-indexing when the adaptive budget changes: neither biases the estimator.
- Not audited: #5497 (`0b2ffed89`, MSWP Color Remapping Index) lies outside every dimension's Paths.

To publish: `/audit-publish docs/audits/AUDIT_RENDERER_2026-10-11.md`. Suggested labels: `renderer` and `shaders` for D2-01 and D7-01 (plus `test-gap` for D7-01), `renderer` for D2-02, and `doc-rot` for D2-04, D5-01 and D11-01 (batch into one issue). Do not file D2-03 or the Dim 10 cross-lists separately.
