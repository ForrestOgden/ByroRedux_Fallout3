# SCR-D5-2026-10-11-07: `fold_info_into_topic` is O(N x M) per DIAL override

Labels: low,esm-plugin,dialogue,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 5
- **Location**: `crates/plugin/src/esm/records/index.rs` `fold_info_into_topic`
- **Status**: NEW

**Untrusted-Input: Yes.** O(N×M) per DIAL override INFO fold (#5375). No panic or OOB. Build a `FormId → position` map

