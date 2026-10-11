# TD7-2026-10-11-08: `post_passes.rs` re-derives the cluster grid and index buffer sizes with a bare `8` although `ClusterCullPipeline` exposes both

URL: https://github.com/matiaszanolli/ByroRedux/issues/5619
Labels: low, renderer, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md


- **Severity**: LOW
- **Dimension**: 7
- **Location**: `crates/renderer/src/vulkan/context/post_passes.rs:696-709` (`CLUSTER_ENTRY_SIZE = 8`)
- **Status**: NEW
- **Age**: 94ded998b3, 2026-07-28
- **Effort**: trivial
- **Description**: The block computes `grid_size` / `index_size` for the volumetrics lighting descriptors with a function-local `const CLUSTER_ENTRY_SIZE: vk::DeviceSize = 8`. The comment says it "mirrors the formulas in `ClusterCullPipeline::new`". `ClusterCullPipeline` already exposes `grid_buffer_size()` (`compute.rs:423`, `size_of::<ClusterEntry>() * TOTAL_CLUSTERS`) and `index_buffer_size()` (`:428`), and `context/init.rs:584-586` calls them. The same function already holds the pipeline (`self.cluster_cull`, `:652`).
- **Evidence**: `size_of::<ClusterEntry>()` is the struct size; the bare `8` is its hand-computed value (`{offset:u32, count:u32}`).
- **Impact**: If `ClusterEntry` grows (the likely edit is a third field in `bindings.glsl:340`), the volumetrics descriptor `range` is under-sized. Robust-buffer reads return zeros rather than faulting, so the symptom is wrong froxel lighting, not an error. Debug validation does not catch it either.
- **Suggested Fix**: Use `cc.grid_buffer_size()` and `cc.index_buffer_size()` at `post_passes.rs:703-709` (inside the `vol_lights` closure that already holds `cc`), and delete the local const.
