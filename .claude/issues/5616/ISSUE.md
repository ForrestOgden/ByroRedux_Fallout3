# TD7-2026-10-11-05: Consumable CTDA gating duplicates the "known CTDA lengths", the run-on offset and the `RunOn` enum range from `condition.rs`

URL: https://github.com/matiaszanolli/ByroRedux/issues/5616
Labels: low, esm-plugin, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md


- **Severity**: LOW
- **Dimension**: 7
- **Location**:
  - `crates/plugin/src/esm/records/items/consumable.rs:75-82`
  - `crates/plugin/src/esm/records/condition.rs:354` (the same length set)
  - `crates/plugin/src/esm/records/condition.rs:386-392`
- **Status**: NEW
- **Age**: 4c0053322, 2026-09-16
- **Effort**: trivial
- **Description**:
  - `consumable.rs:75` has `!matches!(sub.data.len(), 20 | 24 | 28 | 32)`, the same literal set as `condition.rs:354`.
  - `consumable.rs:79-82` reads the run-on field with `u32::from_le_bytes(sub.data[20..24]…) > 7`. The offset 20 and the maximum 7 (`RunOn::EventData`) are both facts owned by `condition.rs` (`RunOn::from_u32`, `data[20..24]` at `:386-392`).
  - Whether a new `RunOn` variant 8 gets accepted is decided in a different file.
- **Evidence**: See the line numbers above. `RunOn::from_u32` maps unknown values to `Subject`, which is the fallback the consumable gate is trying to avoid, and the consumable gate hard-codes the boundary. There is no `CTDA_KNOWN_LENGTHS` or `RunOn::try_from_u32`.
- **Impact**: Low. This is a lockstep gap with an easy failure mode: add a `RunOn` variant (or a 36-byte CTDA layout) in `condition.rs` and consumable conditions with that value silently stay rejected, so the consumable effect drops with no log line.
- **Suggested Fix**:
  - Add `pub const CTDA_KNOWN_LENGTHS: [usize; 4] = [20, 24, 28, 32]` and `pub fn RunOn::try_from_u32(u32) -> Option<Self>` to `condition.rs`.
  - Add a `ctda_run_on_raw(data) -> Option<u32>` accessor next to the parser.
  - Use them in `consumable.rs` and at `condition.rs:354`.
