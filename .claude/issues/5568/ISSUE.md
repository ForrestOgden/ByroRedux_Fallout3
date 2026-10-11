# REN-D2-2026-10-11-04: glam-deprecated names and a stale noise-seed description remain in comments and docs

GitHub: https://github.com/matiaszanolli/ByroRedux/issues/5568
Labels: low,renderer,documentation,doc-rot
Source: docs/audits/AUDIT_RENDERER_2026-10-11.md (filed snapshot; GitHub is authoritative for state)

- **Severity**: LOW (doc-rot)
- **Dimension**: Camera-Relative Precision / Soft Shadows
- **Location**: `crates/core/src/ecs/components/camera.rs` (doc comments at the `perspective_rh` mentions and the inline "glam's perspective_rh already maps Z…" comment); `docs/engine/coordinate-system.md` (shows `Mat4::perspective_rh`); `docs/engine/shadow-pipeline-tradeoffs.md` §4 (`cameraPos.w` described as the seed for `interleavedGradientNoise` in shadow-ray jitter).
- **Status**: NEW (the `frame_params.rs` "look_at_rh call" comments the agent cited did not reproduce in a grep of `byroredux/src/render/`; those two are unverified)
- **Description**: glam 0.33 deprecated `Mat4::look_at_rh` / `perspective_rh`; the producers are now `camera::rh::view::look_at_mat4` and `camera::rh::proj::directx::perspective`. Since `4d991bad7` the finalize shadow-ray disk samples use `ditheredSequence2` (R2 in time, frame-free `ignMask` rotation per pixel), not `interleavedGradientNoise` seeded from `cameraPos.w`.
- **Impact**: doc rot only.
- **Related**: `c6b8d4584`, `4d991bad7`, `f4c2b9931` (the dependency-refresh comment sweep that missed these).
- **Suggested Fix**: Rename the references to the live producers and rewrite §4 of `shadow-pipeline-tradeoffs.md` around `ditheredSequence2`.
