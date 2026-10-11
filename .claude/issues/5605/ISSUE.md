# TD4-2026-10-11-01: Six backticked test/function names in four skills were renamed or deleted by today's fix wave. The validator prints them only as advisories

URL: https://github.com/matiaszanolli/ByroRedux/issues/5605
Labels: low, tech-debt, documentation, doc-rot
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 4, Audit-Finding Rot
- **Location** (skill:line, stale name, current name, commit):
  - `audit-starfield/SKILL.md:59`: `cdb_emissive_settings_weight_the_emissive_role`; now `cdb_emissive_settings_are_parked_not_forwarded_raw` (`byroredux/src/asset_provider/material/merge.rs:2074`); renamed by `4667b884b` (#5503).
  - `audit-exterior/SKILL.md:76`: `distant_water_hole_is_exactly_the_streaming_boundary`; now `distant_water_holes_only_actual_residency` (`cell_loader/water.rs:2019`); `e38ff49d2` (#5334) also deleted `distant_water_hole_radius`.
  - `audit-exterior/SKILL.md:71`: `recenter_lod_water`; now `refresh_lod_water` (`streaming/mod.rs:633`); `e38ff49d2` / `310723850` (#5289).
  - `audit-exterior/SKILL.md:33`: `inherit_all_up_chain`. `65cadde2f` (#5516) deleted this function, and the sentence calling it a flag-less twin "that the tests pin (open #5516)" is now wrong on both counts. The surviving guard is `oblivion_child_gets_the_inherit_all_stamp_and_eras_stay_real` (`crates/plugin/src/esm/cell/tests/wrld.rs:1043`).
  - `audit-oblivion/SKILL.md:100`: `oblivion_child_inherits_parents_water_without_pnam`; now `oblivion_child_inherits_parents_water_through_the_parse_stamp` (`env_translate.rs:2891`); `65cadde2f`.
  - `audit-scripting/SKILL.md:417`: `purge_despawns_pending_readoption_entities`; now `pending_readoption_victims_are_the_whole_subtree_closure` and `pending_readoption_victims_spare_the_player_rig` (`cell_loader/unload.rs:1242,1292`); `99cd9fd6c` (#5490). That commit also changed `purge_cinematic_retention_state(world)` to `(world, ctx)`.
- **Status**: NEW
- **Age**: all six renames are from 2026-10-11, 18:20 to 18:56 -03. The skills were last re-synced at 18:03 (`d84e7f163`, #5579). 15 commits landed after that re-sync, 13 of them fix commits.
- **Effort**: trivial
- **Description**: These skills cite guard names as "the regression pin". An auditor who greps for the name finds nothing, and cannot tell a deleted guard from a renamed one.
- **Evidence**:
  - `_audit-validate.sh` ADVISORY (audit skills) lists exactly these symbols: `cdb_emissive_settings_weight_the_emissive_role`, `distant_water_hole_is_exactly_the_streaming_boundary`, `inherit_all_up_chain`, `oblivion_child_inherits_parents_water_without_pnam`, `purge_despawns_pending_readoption_entities`, `recenter_lod_water`.
  - `git log -S<name> --format='%h %cs %s'` shows each name last present in the commit above.
  - The other two skill advisories are not rot: `FIXTURE_DATA_ENV` and `FIXTURE_GATES` live in `docs/smoke-tests/fixtures/*.env`, which the symbol scan does not read.
- **Impact**: The guard pointers rot within hours of a fix wave. No misled audit is demonstrated yet, so this stays LOW. It does leave the exterior, oblivion, scripting and starfield skills pointing at removed guards.
- **Related**: #5579 (the re-sync that is now behind); TD4-2026-10-11-02 (same sites). Prior TD4-2026-10-08-01 / #5473 (same class, fixed for pathspecs only).
- **Suggested Fix**:
  - Rename the six pointers to the successor names above.
  - Rewrite `audit-exterior:33`: drop the "inherit_all_up_chain twin … open #5516" clause, and say the Oblivion inherit-all is the parse-time PNAM stamp with one walk (`inherit_up_chain`).
  - Re-run `/audit-sync` after the current fix wave settles.
