# SAVE-D5-2026-10-12-01: `docs/engine/save-load-roundtrip.md` §6 (the authoritative load-apply trace) lists 8 steps and "two reconcilers"; the code has since grown about ten further steps, and every line cite is wrong

Labels: low,save-load,documentation,doc-rot
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5690

Filed from `docs/audits/AUDIT_SAVE_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Live Load-Apply & Frame Boundary
- **Data-Loss Class**: none (doc-rot)
- **Location**: `docs/engine/save-load-roundtrip.md:143-250`. Last edited 2026-09-26 (`2f8538334`).
- **Status**: NEW. The earlier doc-rot issues #3028, #3794 and #4145 are closed and fixed other drift. This is new drift.
- **Description**:
  1. Cites that are stale:
     - `execute_pending_save_loads` is cited at `save_io.rs:1203`; it is at `:1747`.
     - `step_save_loads` is cited at `app_step.rs:642`.
     - `apply_deltas` is cited at `driver.rs:318`; it is at `:380`.
     - `build_form_id_remap` is cited at `driver.rs:226`; it is at `:262`.
     - `capture_player_pose` and `apply_player_pose` are cited at `save_io.rs:447` / `:493`.
  2. The numbered sequence omits steps the code now runs:
     - `PRE_RELOAD_RESOURCES` is described as "the first call", but the list of resources and the `ItemInstancePool` exclusion (#4135) are not stated.
     - `purge_cinematic_retention_state` (#5056, #5379, #5490).
     - `without_parked_state` around the reload.
     - `reset_story_location_cursor` and `discard_pending_story_events` (#5435).
     - `restore_resident` (#4695).
     - `drop_entity_bound_continuations` (#4139).
     - `reseat_ambient_packages_after_restore` (#4815).
     - `unresolved_form_id_pairs` → `park_unresolved_snapshot_rows` (#5054).
     - `reconcile_worn_gear` (#5034) and `reset_player_factions_to_record` (#5058), both after the overlay.
     - The diagnostic `validate_world` before the pose restore.
  3. Step 7 says "two reconcilers now". The code runs four, counting `reconcile_worn_gear` and `reset_player_factions_to_record`.
  4. Step 6 calls the `MUTABLE_DELTA_COLUMNS` list "~20" and gives a partial inventory; the real list is 24.
  5. The skill's strict-sequence paragraph and `save_io.rs` are now the more accurate description.
- **Evidence**: compare the `execute_pending_save_loads` body (`save_io.rs:1747-2074`) with the doc's steps 1-8.
- **Impact**: the reference a new contributor reads first under-describes the order-critical load sequence. Several of the omitted steps are order-sensitive (#5060 is open on exactly that).
- **Related**: #5060, #3028, #3794, #4145.
- **Suggested Fix**: regenerate §6 from the current function, and add the file and line cites by function name instead of line number.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites

