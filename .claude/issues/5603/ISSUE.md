# TD3-2026-10-11-07: `docs/contributing.md` and `docs/engine/testing.md` still recommend the bare `cargo test -p byroredux-core` that #3895 documented as silently dropping tests

URL: https://github.com/matiaszanolli/ByroRedux/issues/5603
Labels: low, tech-debt, documentation, doc-rot
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments (developer command)
- **Location**: `docs/contributing.md:111`, `docs/engine/testing.md:178,183`
- **Status**: NEW (same class as #3895 and #5107; those fixed CLAUDE.md/AGENTS.md only)
- **Effort**: trivial
- **Evidence**: CLAUDE.md:12-22 says `--features inspect` is load-bearing: a bare `cargo test -p byroredux-core` drops `animation::{player,stack}::inspect_tests` (the #486 round-trips) with no diagnostic. `grep -rn "features inspect" docs/*.md docs/engine` finds no mention outside audit reports.
- **Impact**: a contributor following the contributing guide or testing.md gets a green run that omits exactly the tests covering the `AnimationPlayer`/`AnimationStack` serialization they are editing.
- **Suggested Fix**: add `--features inspect` to those three commands (or one sentence pointing at CLAUDE.md's note).
