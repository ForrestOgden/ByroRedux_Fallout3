# TD7-2026-10-11-01: The CPU water-wave mirror re-types water.vert's wave tuning and render/water.rs's scroll prep; no shared constant, and the shader allowlist's "no Rust counterpart" claim is now false

URL: https://github.com/matiaszanolli/ByroRedux/issues/5582
Labels: medium, water, physics, shaders, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md


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
