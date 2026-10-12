# TOOL-D1-2026-10-12-01: Two #5147/egui-0.36 helpers were inserted between a doc comment and its function, so `write_screenshot` and `DebugUiState::set_visible` lost their docs

Labels: low,tech-debt,documentation,doc-rot
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5692

Filed from `docs/audits/AUDIT_TOOLING_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Debug Trust Boundary (and the debug-ui panel host)
- **Exposure**: developers only (rustdoc and reader confusion)
- **Location**: `crates/debug-server/src/system.rs:223-243` (`write_screenshot`, `72a63616b`); `crates/debug-ui/src/lib.rs:229-248` (`set_visible`, `500fcb8ca` "egui: discard dropped frames")
- **Status**: NEW (same class as REN-D6-2026-10-12-01 and NIFAL-D9-2026-10-12-01, different files)
- **Description**: Both commits added a new function directly under an existing `///` block.
  - `system.rs`: the original docs "Save debug screenshots under the working directory's `screenshots/` folder. Only a single relative filename is accepted ..." now sit on top of the #5147 text and attach to `unique_screenshot_name`, which does not confine anything. `write_screenshot`, the function that actually enforces the filename-only rule and the `starts_with(&cwd)` check, has no doc.
  - `lib.rs`: "Show or hide the overlay explicitly (scripted captures)." now documents `discard_last_output`. The public `set_visible` has none.
- **Evidence**: `sed -n 223,230p crates/debug-server/src/system.rs` shows five consecutive `///` lines followed by `fn unique_screenshot_name`; `sed -n 229,247p crates/debug-ui/src/lib.rs` shows the `set_visible` doc followed directly by "Drop a stashed `FullOutput` ..." and `fn discard_last_output`.
- **Impact**: rustdoc for `set_visible` is empty and `discard_last_output` (private) carries a public-API sentence; the security-relevant confinement contract of `write_screenshot` is no longer attached to it. No behaviour change.
- **Related**: #5147, #5165
- **Suggested Fix**: Move each original doc block back above its function and give the helper only its own comment.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites

