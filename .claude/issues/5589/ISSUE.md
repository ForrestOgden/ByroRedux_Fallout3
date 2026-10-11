# TD2-2026-10-11-06: `InteriorCellApplyJob::begin` is a ~54% line-for-line twin of `load_cell_with_masters`, and fixes have to land in both

URL: https://github.com/matiaszanolli/ByroRedux/issues/5589
Labels: low, esm-plugin, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW. Same call set in both (see Evidence), no divergence found yet.
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/cell_loader/load.rs:410-806` (`load_cell_with_masters`) vs `load.rs:840-1157` (`InteriorCellApplyJob::begin` / `finish`)
- **Status**: NEW
- **Age**: job added `6cdf91ec9` 2026-09-03 ("Fix #3671").
- **Effort**: medium
- **Description**: both do plugin-path chaining, `parse_record_indexes_in_load_order`, the cell lookup with the three-tier "cells containing substring / prefix / first 20" error (`load.rs:493` and `:894`, identical text), lighting/sky/water/precombined, `RegionAmbientRes` + `dispatch_region_ambient_music`, `ensure_globals_resource`, resolver install, `CurrentCellContext`, phase-timing log. The module doc states the intent: the sync path "remains synchronous" for a job that can be retained between frames. `FrameTimeBudget::unlimited()` (`work_budget.rs:31`) already exists to run the budgeted path to completion.
- **Evidence**: normalized-line comparison of the two bodies: 125 of the sync body's 231 code lines (>12 chars) occur in the job body. Function-call set difference: sync-only `load_references`, `stamp_cell_root`; job-only `load_references_budgeted`, `advance`, `finish`, `stamp_cell_root_range`, `unload_cell` (mechanics only). `git show -U0 777351ec1` (#5484), `afd6a73f7` (#4416) and `387bb9d9c` (#4909) each carry hunks in BOTH functions.
- **Impact**: every interior-load fix is a two-place edit; callers `debug_load.rs:408` and `save_io.rs:1444` still depend on the sync copy.
- **Related**: #3671, #5484, #4416.
- **Suggested Fix**: make `load_cell_with_masters` a thin wrapper over `InteriorCellApplyJob::begin` + `advance(.., &mut FrameTimeBudget::unlimited())` returning the `InteriorCellApplyProgress::Complete` payload; extract the lookup-with-suggestions error into `fn find_interior_cell<'a>(index, key) -> anyhow::Result<&'a CellData>` in the same file. Verify the `load_references` vs `load_references_budgeted` difference is only the slicing.
