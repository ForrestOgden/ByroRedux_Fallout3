# TD6-2026-10-11-03: `PerkList` is an orphan component (zero production readers or writers) whose module doc and the `condition.rs` function table still say `HasPerk` reads it

URL: https://github.com/matiaszanolli/ByroRedux/issues/5610
Labels: low, character, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/core/src/ecs/components/perk_list.rs:1-5` (module doc), `crates/scripting/src/condition.rs:44` (table row `449 | HasPerk | PerkList`), `crates/core/src/ecs/components/mod.rs:95` (re-export)
- **Status**: NEW (the code half was settled by #2940/#3158 - `HasPerk` moved to `Perks` - but the component and two doc sites were left behind)
- **Age**: `97fbc66f3` (2026-06-23); superseded by `Perks` read at `condition.rs:972-981`
- **Effort**: trivial
- **Description**: `perk_list.rs` says it "is the ECS surface the perk system ... writes to and the M47.1 `HasPerk` condition function reads from (#1667)". The live evaluator (`condition.rs:972`) reads `byroredux_core::character::Perks` and its own comment says "the legacy `PerkList` projection is not populated by production code". The save-registry test (`registry_completeness_tests.rs:561`) also records "zero production write sites ... only #[cfg(test)]".
- **Evidence**: `grep -rnw PerkList crates byroredux tools --include='*.rs'` -> definition, re-export, 2 doc mentions in `character/components.rs`, its own tests, and `condition.rs:44/977` doc/comment. No `world.insert(_, PerkList..)`, no `query::<PerkList>`; my constructor-token scan flags it as the only component with zero references.
- **Impact**: A contributor wiring perk grants will "attach `PerkList`" per its doc and silently get a no-op, since `HasPerk` reads `Perks`.
- **Related**: #2940, #3158, #1667 (all closed)
- **Suggested Fix**: Delete `perk_list.rs` + re-export + the two `character/components.rs` doc mentions and change `condition.rs:44` to `Perks`; or, if kept as a projection, demote its module doc to "legacy, unpopulated".
