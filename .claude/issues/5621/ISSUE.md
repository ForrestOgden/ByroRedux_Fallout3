# TD8-2026-10-11-01: CI clippy gate red at HEAD — 3 lints, renderer + physics lib, leaves the `byroredux` bin unlinted; `--all-targets` adds 5 more test-target lints

URL: https://github.com/matiaszanolli/ByroRedux/issues/5621
Labels: low, renderer, physics, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW (no tech-debt promotion trigger applies; note it is a red CI gate — treat as top quick win)
- **Dimension**: 8 (clippy gate)
- **Location**:
  - Gate: `crates/renderer/src/texture_registry/dynamic_rgba.rs:38,39` (`doc_lazy_continuation` x2; `e588e5e67`, Fix #5528, 2026-10-11)
  - Gate: `crates/physics/src/world/mod.rs:889-891` (`option_map_unit_fn`; `2917ab372`, Fix #5539, 2026-10-11)
  - All-targets only: `crates/physics/src/water.rs:2225-2226` (`field_reassign_with_default`; `cff1f9c42`, #5354) and `:2485-2486` (same; `0d28abfe3`, #5561)
  - All-targets only: `crates/nif/src/blocks/collision/bhk_constraint_tests.rs:560` (`needless_range_loop`; `526cb07d9`, #5537)
  - All-targets only: `crates/plugin/src/esm/records/misc/dialogue.rs:1862` (`needless_borrows_for_generic_args`; `29f8fa2ef`, #5454)
- **Status**: NEW (prior 10-08 TD8-01 / #5474 and TOOL-CI #5477 are CLOSED and their lints are gone; these are fresh)
- **Effort**: trivial
- **Description**: Today's fix commits landed lints the gate rejects. The gate run exits with `could not compile byroredux-renderer (lib) due to 2 previous errors` and `byroredux-physics (lib) due to 1 previous error`; `--keep-going` therefore skips `byroredux` (bin), which depends on both. `gh run list` shows the latest CI runs are in progress/failed (`4b94729` failure).
- **Evidence**:
  - `CARGO_TARGET_DIR=/mnt/data/tmp-audit-target/dim8 cargo clippy --workspace --keep-going -- -D warnings` on 1.96.0: errors only in the two files above.
  - `... --all-targets --keep-going`: `byroredux-physics (lib test)` 3 errors, `byroredux-renderer (lib test)` 2, `byroredux-nif (lib test)` 1, `byroredux-plugin (lib test)` 1.
  - Crates left unlinted in both runs: `byroredux` (bin + its tests) and any renderer/physics dependents' test/example targets.
- **Impact**: Red gate masks any further lint in the bin crate (the largest, most-churned crate). Same recurring class as #5477/#5474/#4765/#4090 — the bucket re-accumulates every active day.
- **Related**: #5474 (CLOSED), #5477 (CLOSED), #5115 (CLOSED), #5308 (toolchain pin)
- **Suggested Fix**: `if let Some(b) = self.bodies.get_mut(handle) { b.add_torque(torque, false); }` (physics/world/mod.rs:889); indent the two doc continuation lines or insert a blank line (dynamic_rgba.rs:38-39); `let mut x = T { field, ..Default::default() }` in water.rs tests; iterate with `.iter().enumerate()` in bhk_constraint_tests.rs:560; drop the `&` in dialogue.rs:1862. Consider the non-blocking `--all-targets` CI step suggested last time (still not present).
