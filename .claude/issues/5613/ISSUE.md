# TD6-2026-10-11-06: `save::disk::latest_slot` is claimed to be "used by the player-facing quickload action" but is a test-only wrapper since `slots_by_recency` replaced it

URL: https://github.com/matiaszanolli/ByroRedux/issues/5613
Labels: low, save-load, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/save/src/disk.rs:80-84`
- **Status**: NEW
- **Age**: delegating wrapper since the 2026-08-24 save audit
- **Effort**: trivial
- **Description**: The doc comment says it is "used by the player-facing quickload action". The prod quickload path calls `disk::slots_by_recency` (`byroredux/src/save_io.rs:1249`); `latest_slot` is referenced only by 4 asserts in `disk.rs`'s own tests (`:309-376`).
- **Evidence**: `grep -rn latest_slot crates byroredux --include='*.rs'` -> def + tests only.
- **Impact**: Trivial; a false "used by" claim in a freshly-audited crate.
- **Suggested Fix**: Delete it and retarget its 4 asserts at `slots_by_recency(..).first()`, or reword the doc to "test convenience".
