# TD2-2026-10-11-05: `CellData` has no `Default`, so 16 full 33-field literals exist (14 in tests); two were added in the last 24 hours

URL: https://github.com/matiaszanolli/ByroRedux/issues/5581
Labels: medium, esm-plugin, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: MEDIUM (table row 1: duplicated construction with a divergent-fix history - #4480 was a red bin-crate test build because six `CellData` fixtures missed a new field, fixed by editing each literal rather than consolidating).
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/plugin/src/esm/cell/mod.rs:191` (struct, 33 fields); literals: `cell/wrld.rs:453`, `cell/walkers.rs:488` (production); tests: `cell/tests/merge.rs:24`, `cell/tests/cell_for_refr.rs:59`, `byroredux/src/cell_loader/exterior.rs:602,779,1049,1705`, `lod_support.rs:371`, `lgtm_fallback_tests.rs:49`, `interior_spawn_tests.rs:73`, `pkin_expansion_tests.rs:41`, `reference_state.rs:1177`, `scene/world_setup.rs:1643`, `systems/story_events.rs:203` (`c85d50356`, 2026-10-11), `systems/eat_sleep.rs:693` (`bcaf63aa8`, 2026-10-10)
- **Status**: NEW (related closed #4480)
- **Age**: each new feature adding a `CellData` field re-edits all of them; `deleted_refs` (2026-08-23) was the last.
- **Effort**: small
- **Description**: `#[derive(Debug)]` only (`mod.rs:190`). Every field type is `Default`-able (`String`, `Option<_>`, `bool`, `Vec<_>`, `CellLighting`/`LandscapeData`/`CellOwnership` behind `Option`, `NavmRecord`/`PathGridRecord` in `Vec`), so a derive is enough. Four of the test constructors are named `empty_cell` with identical bodies up to 2 fields.
- **Evidence**: `grep -rn --include='*.rs' 'deleted_refs:' crates byroredux | grep -v 'pub deleted_refs'` -> 16 literals in 13 files.
- **Impact**: adding the next field is a 16-site edit and a bin-crate test compile break if one is missed - already happened once (#4480), and two new literals arrived in the last day despite it.
- **Related**: #4480, #3864 (same fix for `NifHeader`).
- **Suggested Fix**: `#[derive(Default)]` on `CellData` (plus its field types if any lack it), then rewrite the 14 test literals as `CellData { form_id, grid: Some(..), ..Default::default() }`; keep `wrld.rs`/`walkers.rs` exhaustive-destructure style only if the compiler-checked field coverage is wanted there. Collapse the four `empty_cell` helpers into `CellData::empty(form_id)` under `#[cfg(any(test, feature = "test-fixtures"))]` in `crates/plugin/src/esm/cell/mod.rs`.
