# REN-D2-2026-10-11-02: CPU absolute view-projection is rebuilt from `position + forward` at world magnitude, rotating the cull frustum up to ~0.3–0.45° at |coord| > 131k

GitHub: https://github.com/matiaszanolli/ByroRedux/issues/5566
Labels: low,renderer,bug
Source: docs/audits/AUDIT_RENDERER_2026-10-11.md (filed snapshot; GitHub is authoritative for state)

- **Severity**: LOW
- **Dimension**: Camera-Relative Precision
- **Location**: `Camera::view_matrix` in `crates/core/src/ecs/components/camera.rs` → `look_at_mat4(position, position + forward, up)`; consumed by `assemble_camera` in `byroredux/src/render/camera.rs` as `vp_abs` → `FrustumPlanes::from_view_proj` and `vp_mat`.
- **Status**: NEW (pre-existing; the glam swap is bit-equivalent to the old `look_at_rh`)
- **Description**: `look_at_mat4` derives the direction as `(center - eye).normalize()`. With `center` formed in f32 at Markarth-scale coordinates (X ≈ −176 000, ulp 2⁻⁶), `forward` is quantised to ~0.0156 per component before normalisation. The GPU matrix is unaffected (`vp_rel` is built from the camera-relative eye); the CPU frustum planes and `vp_mat` use `vp_abs`.
- **Evidence**: an f32 simulation at eye = (−176000.3, 500.1, 121000.7) gives a direction error of 0.0° at yaw 0, 0.14° at 20°, 0.31° at 37°, worst case ≈ 0.45° (≈ 0.22° for 65k–131k). `contains_sphere` has no padding term. The agent's simulation is not backed by a checked-in test.
- **Impact**: cull edges are rotated against the rendered view by ≈ 6 px at 1080p / 0.3°. Small objects straddling the left/right screen edge can pop in late. Sort depth and particle clip use the same matrix (negligible).
- **Related**: #2744 (closed; same class of defect on the GPU cluster-cull side).
- **Suggested Fix**: Build `vp_abs` as `vp_rel * Mat4::from_translation(-render_origin)`, or use `look_to_mat4(position, forward, up)` so the direction never passes through an absolute-magnitude subtraction.
