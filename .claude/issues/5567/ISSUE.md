# REN-D7-2026-10-11-01: the ReSTIR direct-EMA lag fixtures still model the retired α = 0.025 parked tail; the live deep tail (1024 / 0.002) fades a switched-off light ~13× slower

GitHub: https://github.com/matiaszanolli/ByroRedux/issues/5567
Labels: low,renderer,shaders,test-gap,bug
Source: docs/audits/AUDIT_RENDERER_2026-10-11.md (filed snapshot; GitHub is authoritative for state)

- **Severity**: LOW (test-gap / doc-rot). Dim 2 reported the same defect as REN-D2-2026-10-11-03 — publish once.
- **Dimension**: Denoise & resolve chain / Soft Shadows
- **Location**: `mixed_visibility_clusters_keep_a_documented_ratio_lag` and the flicker fixture above it in `crates/renderer/src/vulkan/restir.rs` (both hard-code `let alpha = 0.025f32`); the `restirUnshadowedSum` comment in `crates/renderer/shaders/triangle.frag` ("fades over the EMA window"); the doc of `light_rig_geometry_key_is_independent_of_light_order` in `crates/renderer/src/vulkan/caustic.rs` ("parked tail 256 -> 16").
- **Status**: NEW (adjacent to closed #5020 and #5369 and open #5370; consequence of `48bed0504`)
- **Description**: The shader's parked branch is `alphaFloor = sceneStatic ? 0.002 : 0.1` with cap 1024. Mode 3 stays in the deep tail through *any* intensity change by design (the rig key is intensity-blind). The #5020 limit (the EMA stores one whole-cluster ratio) is therefore governed by α = 0.002, not 0.025. The fixtures assert only `lit[19] > 0.25` after 20 frames, true for any α ≲ 0.03.
- **Evidence**: re-simulating the fixture's own model at the live constants: flicker pass-through stays ≈ 48 % (0.490 @ 0.025, 0.484 @ 0.002, so the shader's "~48 %" text is still right), but the switched-off fade half-life is 27 frames @ 0.025, 86 @ 0.008, **346 @ 0.002** (frames to a quarter: 55 / 173 / 693).
- **Impact**: visual transient only, in a narrow case: a multi-light cluster with differing visibility plus a light whose intensity animates to or through zero while staying in the list. At 60 fps that is a ~6 s half-life of a phantom 50 % contribution from the occluded neighbour; at 300 fps ~1.2 s. Penumbra convergence (the reason for the change) is unaffected. The risk is that the pinned model is 13× off, so a later change cannot be judged against it.
- **Related**: #5020, #5369, #5370 (open), #5507.
- **Suggested Fix**: Make the two fixtures read the live constants (parse the literal out of `triangle.frag`, or share one `const`), state the half-life in the `restirUnshadowedSum` comment, and refresh the `caustic.rs` doc to 1024 → 16. If the multi-second step lag is unacceptable, resetting `histLen` on a large per-frame change in the unshadowed sum is a design call, not a drive-by.
