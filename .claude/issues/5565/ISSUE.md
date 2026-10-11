# REN-D2-2026-10-11-01: #5455's fix only reaches the foreign-opaque-hit leg — a glass pane alone still never attenuates the transmission half

GitHub: https://github.com/matiaszanolli/ByroRedux/issues/5565
Labels: low,renderer,shaders,bug
Source: docs/audits/AUDIT_RENDERER_2026-10-11.md (filed snapshot; GitHub is authoritative for state)

- **Severity**: LOW (visual only; same severity #5455 was filed at)
- **Dimension**: Soft Shadows (Ray Queries). Cross-listed by Dim 10 as REN-D10-2026-10-11-01 — publish once.
- **Location**: `traceShadowTransmittanceSkippingInstance` in `crates/renderer/shaders/include/shadow_transport.glsl` (the `NoneIntersection` early return and the two other `return vec3(1.0)` exits); caller `traceLightTransmittanceSkippingInstance` in `lighting.glsl`; pin `transmission_skip_walk_hands_the_full_mask_to_the_foreign_leg` in `scene_buffer/shader_contract_tests.rs`.
- **Status**: Regression of #5455 (closed; its repro scenario is not actually fixed)
- **Description**: `6d64634aa` hands the full `visibilityMask` to the foreign-hit handoff `traceShadowTransmittance(at, …, visibilityMask)`. The skip walk that precedes it queries `skipRQ` with `mask = visibilityMask & VISIBILITY_MASK_ALL_OPAQUE` (15u), and glass instances carry `VISIBILITY_LAYER_GLASS` (16u, outside `ALL_OPAQUE`). The walk therefore cannot see a pane. The glass-aware handoff runs only when the walk finds a foreign *opaque* hit. If the segment light → receiver contains the receiver's own body and a window pane but no other opaque blocker, the walk reports `gl_RayQueryCommittedIntersectionNoneEXT` and returns `vec3(1.0)`. The same holds for the budget-exhausted exit and the `advanceShadowRayPastHit` false exit.
- **Evidence** (re-verified by the orchestrator): `uint mask = visibilityMask & VISIBILITY_MASK_ALL_OPAQUE; // skip walk only` → `rayQueryInitializeEXT(skipRQ, topLevelAS, gl_RayFlagsOpaqueEXT, mask, …)` → `if (… == gl_RayQueryCommittedIntersectionNoneEXT) { return vec3(1.0); }`. `VISIBILITY_LAYER_GLASS 16u` and `VISIBILITY_MASK_ALL_OPAQUE 15u` in `include/shader_constants.glsl`. #5455's own repro (a back-lit surface behind a window pane) is the no-foreign-opaque case, so it is unchanged. The new source pin checks the handoff argument text only.
- **Impact**: Skyrim back-light, FO4 translucency and foliage transmission lobes ignore windows and glass partitions between the light and the receiver, while the reflection half of the same pixel is attenuated by the pane. The two halves disagree.
- **Related**: #5455, #5249, #5192, #4946.
- **Suggested Fix**: On the lit-returning exits, when `visibilityMask & VISIBILITY_LAYER_GLASS` is set, finish with `traceShadowTransmittance(at, direction, remaining, emitterRadius, visibilityMask & VISIBILITY_LAYER_GLASS)`. Extend the pin so the lit exits are not bare `return vec3(1.0)` when the mask carries the glass bit. The GPU result needs a Cornell glass-pane oracle (**needs RenderDoc / GPU**).
