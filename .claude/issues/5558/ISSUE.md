# #5558: PHYS-D2-2026-10-11-08: Since the #5311 split, the "`update_query_pipeline` never runs per frame" guard scans only `step`, not `run_substep` (the per-substep body) or `recovery.rs`

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5558
- **Labels**: low,physics,test-gap,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Step & Sync (test gap)
- **Location**: `crates/physics/src/world/mod.rs:2955-2968` (scan window = `pub fn step` … first `\n    }\n`); unscanned `run_substep` `:1050-1181`, `recovery.rs:220-232`
- **Status**: NEW
- **Description**:
  - `step_cost_rationale_is_scoped_to_history_and_names_the_real_cost_centre` asserts
    `!step_body.contains("update_query_pipeline")`. Since #5311 extracted the loop body, `step` only calls
    `run_substep`.
  - The most likely regression site is now outside the scanned window: a per-substep whole-set refresh inside
    `run_substep`, or `refresh_query_geometry_after_restore` swapping its targeted `set_query_leaf` loop for
    `update_query_pipeline()` (a plausible "simplification").
  - Today neither calls it (grep: only the definition and test sites).
- **Evidence**: The scan window construction at `:2955-2957`.
- **Impact**: The O(all colliders) refresh, 9.6 ms/frame on a 95 k world in the #4685 measurement (rapier 0.22), could
  return per substep with the guard green.
- **Trigger Conditions**: a future edit to `run_substep` or `recovery.rs`.
- **Related**: #4685, #5311, #5453
- **Suggested Fix**: Scan `step` + `run_substep` + `refresh_query_geometry_after_restore` (or the whole production text
  of `world/mod.rs` and `world/recovery.rs` minus the definition and `register_newcomers_and_refresh_queries`'s
  documented bootstrap).

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
