# TD2-2026-10-11-04: `entity_global_form_id` exists as byte-identical private fns in two crates plus an inline third copy; its inverse is the public one

URL: https://github.com/matiaszanolli/ByroRedux/issues/5588
Labels: low, scripting, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/scripting/src/fragment/effects.rs:68`, `byroredux/src/systems/eat_sleep.rs:450`, inline at `byroredux/src/interaction.rs:1440-1445`; inverse at `crates/scripting/src/condition.rs:634` (`pub fn resolve_entity_by_global_form_id`)
- **Status**: NEW
- **Age**: `effects.rs` copy 2026-09-10 (`f5127c1cc`); `eat_sleep.rs` copy 2026-10-10 (`bcaf63aa8`, "Fix #5500"), whose doc says "the same keying `resolve_entity_by_global_form_id` searches (#3278's helper shape)".
- **Effort**: trivial
- **Description**: all three do `FormIdComponent` -> `FormIdPool::resolve(..).local.0`. The `effects.rs` doc itself calls it "the inverse of `resolve_entity_by_global_form_id`, keyed the same way so the two agree" - which is exactly why the pair belongs in one file.
- **Evidence**: `sed -n 68,80p crates/scripting/src/fragment/effects.rs` and `sed -n 450,458p byroredux/src/systems/eat_sleep.rs` are the same 7 lines.
- **Impact**: a change in keying (e.g. the PlayerRef sentinel that `resolve_entity_by_global_form_id` special-cases at #4694) updates one direction only.
- **Related**: #3278, #4694, #5500.
- **Suggested Fix**: add `pub fn entity_global_form_id(world, entity) -> Option<u32>` to `crates/scripting/src/condition.rs` beside `resolve_entity_by_global_form_id`, re-export from `lib.rs:62`, delete the two private copies and the `interaction.rs` inline closure. Consider making the player entity map back to `0x14` there too, mirroring the inverse.
