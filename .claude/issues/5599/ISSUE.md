# TD3-2026-10-11-03: ecs.md and architecture.md still say the engine has "12 parallel-stage systems"; it has 9

URL: https://github.com/matiaszanolli/ByroRedux/issues/5599
Labels: low, ecs, tech-debt, documentation, doc-rot
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments (authoritative engine doc)
- **Location**: `docs/engine/ecs.md:503`, `docs/engine/architecture.md:135`
- **Status**: NEW
- **Age**: number is the M27 Phase-1 figure (`docs/engine/m27-phase1-survey.md:9` "12 parallel-stage systems (3 declared today, 9 undeclared)"); first falsified by #3111 which serialised the weather writer.
- **Effort**: trivial
- **Evidence**: `grep -rn "scheduler.add_to_with_access(" byroredux/src | wc -l` = 9; `boot/schedule/mod.rs:590` "The nine `add_to_with_access` registrations"; guard `the_parallel_system_table_covers_every_parallel_registration` pins the table to the registration count; `scheduler_access_tests.rs` floors `parallel_system_count() >= 9` ("floor 9 after #3111").
- **Impact**: the lock-order/concurrency audits cite ecs.md as authoritative for the analyzable population; 12 overstates it by a third.
- **Suggested Fix**: say "the engine's nine parallel-stage systems" (or drop the number and point at `PARALLEL_SYSTEMS` in `boot/schedule/mod.rs`).
