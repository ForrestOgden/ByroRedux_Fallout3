# TD7-2026-10-11-09: Four compute shaders hard-code workgroup size, reduction strides and Rust dispatch counts that the generated header already provides for the other 17

URL: https://github.com/matiaszanolli/ByroRedux/issues/5620
Labels: low, renderer, shaders, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md


- **Severity**: LOW
- **Dimension**: 7
- **Location**:
  - `shaders/exposure_meter.comp:42,55-56,70,74,95` (`local_size_x = 64`, `shared[64]`, `side = 64`, `idx += 64`, reduction start `s = 32`)
  - `shaders/sky_irradiance.comp:5,8,10,21,23,36` (`local_size_x = 64`, `partialSum[64]`, `+= 64u`, `stride = 32u`, `coefficients[9]`)
  - `shaders/sky_prefilter.comp:7`
  - `shaders/groundcover_bench_bake.comp:23`
  - Rust: `vulkan/sky_cube/irradiance.rs:6,153` (`SH_BYTES = 9 * 16`, `cmd_dispatch(cmd, 9, 1, 1)`), `vulkan/sky_cube/filter.rs:208` (`edge.div_ceil(8)`), `vulkan/groundcover_bench.rs:1359-1360` (`LAND_GRID_VERTS.div_ceil(8)`)
- **Status**: NEW
- **Age**: older than the window (not changed since 2026-10-08)
- **Effort**: small
- **Description**: 17 compute shaders take `local_size_*` from generated `WORKGROUP_X/Y`, `SKIN_WORKGROUP_SIZE`, `GROUNDCOVER_*_WORKGROUP` and so on. These four use bare literals. In `exposure_meter.comp` and `sky_irradiance.comp` the workgroup width is coupled to a hand-written tree reduction (`s = 32`) and to `shared[]` array sizes. The Rust side re-types the group counts and the SH coefficient count (9). The `shader_constants.rs` provenance gate looks at SCREAMING_CASE names, so lower-case locals (`side`, `edge`, `count`) and layout qualifiers are invisible to it.
- **Evidence**:
  - `grep -rn local_size shaders/*.comp` shows the four literals.
  - Changing `exposure_meter.comp:42` to 128 without touching `shared[64]` or the reduction start would write out of bounds of the shared arrays.
  - In `irradiance.rs`, `SH_BYTES` and the dispatch group count are typed as separate 9s.
- **Impact**: Lockstep cost only. Nothing is wrong today and each file is self-consistent. The exposure meter drives the auto-exposure of the whole image, so a mismatched edit there would be visible immediately.
- **Suggested Fix**:
  - Add `EXPOSURE_METER_WORKGROUP = 64` (so `shared[EXPOSURE_METER_WORKGROUP]`, and `s = WORKGROUP/2`) and `SKY_SH_COEFFICIENTS = 9` (with `SH_BYTES = SKY_SH_COEFFICIENTS * 16`, `cmd_dispatch(cmd, SKY_SH_COEFFICIENTS, 1, 1)`) to `shader_constants_data.rs`.
  - For `sky_prefilter.comp` and `groundcover_bench_bake.comp`, use `WORKGROUP_X/Y` (value 8, the same today) and make `filter.rs:208` / `groundcover_bench.rs:1359` use `WORKGROUP_X` instead of a literal 8.
