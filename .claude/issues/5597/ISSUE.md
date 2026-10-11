# TD2-2026-10-11-14: Navmesh test fixtures `zup_from_yup` x7 and `two_triangle_quad_navm` x6 despite a shared `test_tiles` module

URL: https://github.com/matiaszanolli/ByroRedux/issues/5597
Labels: low, ai, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW (test-only)
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/systems/{follow.rs:603,travel.rs:624,patrol.rs:412,wander.rs:673,guard.rs:542,escort.rs:731,navmesh_path.rs:828}` (`zup_from_yup`), `follow.rs:609`, `travel.rs:633`, `patrol.rs:418`, `wander.rs:679`, `guard.rs:550`, `escort.rs:737` (`two_triangle_quad_navm`); shared home `systems/navmesh_path.rs:816` (`pub(crate) mod test_tiles`, already holds `two_triangle_quad` at 10 units and `adjacent_quad`)
- **Status**: NEW
- **Age**: 2026-08-23 (`77eda8003`) onward.
- **Effort**: trivial
- **Description**: seven copies of the inverse of `zup_to_yup_pos` (`[x, -z, y]`) - the Z-up knowledge that lives in `crates/core/src/math/coord.rs` - and six copies of the same 2-triangle quad at 1000 units. The comments (`guard.rs:540-549`) cite "each module owns its own test fixtures per this codebase's established convention", while `test_tiles` exists precisely because the EX-16 harness needed cross-module fixtures.
- **Evidence**: `grep -rn 'fn zup_from_yup\|fn two_triangle_quad_navm' byroredux/src/systems` -> 13 lines.
- **Impact**: low; if the Z-up convention for navmesh vertices changes, seven fixtures change in lockstep or one silently lies.
- **Related**: TD2-2026-10-11-02.
- **Suggested Fix**: add `yup_to_zup_pos([f32;3])` beside `zup_to_yup_pos` in `crates/core/src/math/coord.rs` (a round-trip test lives there) and `pub(crate) fn two_triangle_quad_scaled(size: f32) -> NavmRecord` to `navmesh_path::test_tiles`; delete the 13 copies.
