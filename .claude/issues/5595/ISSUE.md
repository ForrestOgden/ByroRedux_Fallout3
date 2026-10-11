# TD2-2026-10-11-12: groundcover descriptor writes hand-roll `write_storage_buffer` twice as local fns and seven times inline

URL: https://github.com/matiaszanolli/ByroRedux/issues/5595
Labels: low, renderer, vulkan, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/renderer/src/vulkan/groundcover_bench.rs:1132-1176` (local `buffer_write` plus 5 inline image writes), `groundcover/frame.rs:245-256` (local `write`), `groundcover_models.rs:747-751` (inline loop)
- **Status**: NEW
- **Age**: `groundcover_bench.rs` 2026-09-15 (`13e00a24b`, which routed its barriers through `descriptors.rs` but left the writes); `frame.rs` 2026-09-06.
- **Effort**: trivial
- **Description**: the local `buffer_write(set, binding, info)` and `write(set, binding, info)` are signature- and body-identical to `descriptors::write_storage_buffer(dst_set, binding, info)` (`descriptors.rs:89`). The five image writes in the bench are `write_combined_image_sampler` (x3) and `write_storage_image` (x3) by another name. `groundcover_bench.rs:73` imports two barrier helpers from `descriptors` but none of the writers.
- **Evidence**: `grep -rn --include='*.rs' 'WriteDescriptorSet::default()' crates | grep -v 'vulkan/descriptors.rs'` -> 9 lines in exactly these three files.
- **Impact**: low, but it is the exact class #1752/#2073 closed.
- **Related**: #1752, #2073, #2071.
- **Suggested Fix**: delete the two local closures and the nine inline builders; import `write_storage_buffer`, `write_combined_image_sampler`, `write_storage_image` from `super::descriptors`.
