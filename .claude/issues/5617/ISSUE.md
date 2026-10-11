# TD7-2026-10-11-06: ReSTIR shadow-ratio history cap/floor are bare shader literals, repeated and pinned only by source text

URL: https://github.com/matiaszanolli/ByroRedux/issues/5617
Labels: low, renderer, shaders, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md


- **Severity**: LOW
- **Dimension**: 7
- **Location**:
  - `crates/renderer/shaders/triangle.frag:3596,3929-3931`
  - pin: `crates/renderer/src/vulkan/restir.rs:~322-331`
  - the mode encoding at `triangle.frag:3929,4185`
- **Status**: NEW
- **Age**: 2a7223121 (2026-10-09, 64 → 256) and 48bed0504 (2026-10-11, 256 → 1024 and 0.008 → 0.002)
- **Effort**: small
- **Description**:
  - The parked-tail cap `1024.0` is typed twice (`clamp(rpHistLen, 0.0, 1024.0)` at `:3596` and `sceneStatic ? 1024.0 : 16.0` at `:3930`).
  - The floor `0.002` / `0.1` and the moving cap `16.0` are typed once each.
  - The history *mode* is a float in `dofParams.w`, decoded by threshold literals `> 0.5` and `> 1.5` in the shader. Meanwhile `restir_history_mode()` (`assemble_camera_and_lights.rs:95`) encodes 0..3 on the Rust side.
  - A comment ties `1024` to packHalf2x16's integer-exact range (2048), but that ceiling is prose only.
  - None of these sit in `shader_constants_data.rs`.
- **Evidence**:
  - The in-shader comment records that a hard-coded `64` guard once "silently defeated `historyCap`'s raise and pinned the parked tail at ~1/65", so this duplication has already produced a wrong result once. The 2a7223121 → 48bed0504 sequence retyped all three literals again.
  - `restir.rs:322-330` pins them with `src.contains("sceneStatic ? 1024.0 : 16.0") && src.contains("clamp(rpHistLen, 0.0, 1024.0)")`. That is a source-shape test of the same literal text, not a value check.
  - Nothing asserts `cap <= 2048` or `clamp bound >= cap`.
- **Impact**: The failure mode is subtle shadow-noise or ghosting regressions. A future retune above 2048 would lose integer exactness in `histLen`, and the history would stall. No crash.
- **Related**: #5369 (open, partial).
- **Suggested Fix**:
  - Add `RESTIR_SHADOW_HISTORY_CAP_PARKED: f32 = 1024.0`, `_MOVING: f32 = 16.0`, `ALPHA_FLOOR_PARKED = 0.002`, `ALPHA_FLOOR_MOVING = 0.1`, and `HISTORY_MODE_SCENE_STATIC_MIN = 2` to `shader_constants_data.rs`.
  - Use them in `triangle.frag` and in `restir_history_mode()`.
  - Add a `const _` assertion that the cap is `<= 2048` (half-exact).
  - Replace the `contains(...)` pin with a header-presence check.
