# TD2-2026-10-11-11: Twelve identical local `try_or_cleanup!` macros and an 8-pass per-frame param-UBO loop in the renderer's pass constructors

URL: https://github.com/matiaszanolli/ByroRedux/issues/5594
Labels: low, renderer, vulkan, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: macros: `ssao.rs:162`, `taa.rs:251`, `svgf.rs:480`, `caustic.rs:412`, `bloom.rs:300`, `compute.rs:108`, `exposure_meter.rs:75`, `sky_cube.rs:234`, `composite.rs:422`, `groundcover_bench.rs:465`, `groundcover/construct.rs`, `volumetrics/init.rs` (all `crates/renderer/src/vulkan/`); UBO loop: `ssao.rs:209-219`, `taa.rs:313-323`, `caustic.rs:458-468`, `svgf.rs:553-563`, `composite.rs:693`, `water.rs:537`, `bloom.rs:1077`, `volumetrics/init.rs:318,758`
- **Status**: NEW
- **Age**: 2026-07 onward (each new pass copied its neighbour).
- **Effort**: small
- **Description**: `macro_rules! try_or_cleanup { ($expr:expr) => { match $expr { Ok(v) => v, Err(e) => { unsafe { partial.destroy(device, allocator) }; return Err(e.into()); } } } }` is redefined verbatim (the SAFETY comment even varies in wording only) in 12 constructors. The `let param_size = size_of::<XParams>(); for _ in 0..FRAMES { let buf = try_or_cleanup!(GpuBuffer::create_host_visible(device, allocator, param_size, UNIFORM_BUFFER)); partial.param_buffers.push(buf); }` block follows in at least five.
- **Evidence**: `grep -rn --include='*.rs' 'macro_rules! try_or_cleanup' crates/renderer/src | wc -l` -> 12; `grep -rn --include='*.rs' 'param_buffers.push' crates/renderer/src | grep -v test` -> 12 lines.
- **Impact**: low. A change to the partial-teardown contract (reverse-order destroy, the `Err` conversion) is a 12-site edit.
- **Related**: TD2-2026-10-11-10.
- **Suggested Fix**: one `macro_rules! try_or_cleanup { ($partial:ident, $device:expr, $allocator:expr, $expr:expr) => ... }` in `vulkan/mod.rs` (or `helpers.rs`, `pub(crate)`, `#[macro_use]`), and `pub(crate) fn create_per_frame_uniforms<T>(device, allocator, frames) -> Result<Vec<GpuBuffer>>` in `vulkan/buffer.rs` beside `create_host_visible`.
