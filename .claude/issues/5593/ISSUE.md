# TD2-2026-10-11-10: ~150 hand-written `vk::DescriptorSetLayoutBinding::default()` blocks in 22 files; no builder, though `write_*` has one

URL: https://github.com/matiaszanolli/ByroRedux/issues/5593
Labels: low, renderer, vulkan, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW (large mechanical surface, no live divergence)
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/renderer/src/vulkan/volumetrics/init.rs` (29), `scene_buffer/buffers.rs` (22), `svgf.rs` (18), `composite.rs` (11), `caustic.rs` (11), `taa.rs` (10), `bloom.rs` (9), `reflect.rs` (8), `skin_compute.rs` (6), `compute.rs` (5), others (`ssao.rs` 3, `presentation.rs` 3, `exposure_meter.rs` 3, ...). One local helper exists: `groundcover/construct.rs:405 storage_binding`.
- **Status**: NEW
- **Age**: spans the renderer's life; `svgf.rs`/`taa.rs`/`caustic.rs`/`ssao.rs` are the oldest compute passes.
- **Effort**: medium
- **Description**: each binding is a 5-line chain `.binding(n).descriptor_type(T).descriptor_count(1).stage_flags(COMPUTE)`; 112 of them are `COMPUTE`-stage with count 1. `descriptors.rs` consolidated the write side (`write_combined_image_sampler`, `write_storage_image`, `write_storage_buffer`, `write_uniform_buffer`, #1752/#2073) and the barrier side but has no layout-side builder, so every pass restates its layout and its pool sizing separately (`DescriptorPoolBuilder::from_layout_bindings` already consumes the slice).
- **Evidence**: `grep -rn --include='*.rs' 'DescriptorSetLayoutBinding::default()' crates/renderer/src | grep -v test | wc -l` -> 150+; `-A4 ... | grep -c 'ShaderStageFlags::COMPUTE'` -> 112.
- **Impact**: hand-copied layouts are the thing `reflect::validate_set_layout` (#427) exists to catch after the fact; a builder prevents the class and removes ~600 lines.
- **Related**: #427, #1752, #2073, #4110.
- **Suggested Fix**: in `crates/renderer/src/vulkan/descriptors.rs` add `pub fn layout_binding(binding: u32, ty: vk::DescriptorType, stages: vk::ShaderStageFlags) -> vk::DescriptorSetLayoutBinding<'static>` and `pub fn compute_binding(binding, ty)`; migrate pass by pass (start with `svgf.rs`, `taa.rs`, `caustic.rs`, `ssao.rs`, `bloom.rs`), replacing `groundcover/construct.rs::storage_binding` with it.
