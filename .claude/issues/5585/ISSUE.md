# TD1-2026-10-11-02: `byroredux/src/save_io.rs` crossed 2000 production LOC (1964 → 2011) — and four test modules self-scan its text

URL: https://github.com/matiaszanolli/ByroRedux/issues/5585
Labels: low, save-load, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 1
- **Location**: `byroredux/src/save_io.rs` (2067 lines, 2011 production; tests live in `save_io/*_tests.rs`)
- **Status**: NEW (was 1964 on 10-08 list)
- **Age**: `03e51dfad` #5394 (+6), `557ead21d` #5435 (+21), `620970028` (+20), `99cd9fd6c` #5490 (±0), all 10-09..10-11.
- **Effort**: medium (the include_str! pins make it more careful than env_translate)
- **Description**: the file is the whole save/load app layer: five responsibilities in contiguous runs.
  1. Registry + constants: `MUTABLE_DELTA_COLUMNS` (L84), `PRE_RELOAD_RESOURCES` (L174), `build_save_registry` (L363–626, 259 LOC, a flat
     registration table at nesting 1) ≈ 560
  2. Save-dir discovery + `SaveState` / `PendingSaveLoadSlot` resources (L272–360) ≈ 90
  3. Player pose + player-save-action queue (L196–262, 652–800: `PlayerPose`, `PendingPlayerSaveActions`, `capture_/apply_player_pose`) ≈ 220
  4. Console commands `SaveCommand` (execute = 201 LOC) / `SaveInfoCommand` / `LoadCommand`, quicksave/quickload helpers, save validation
     (`validate_form_ids`, `validate_cinematic_entity_refs`) (L804–1378) ≈ 570
  5. Live load-apply: `reload_interior_session`, `reload_exterior_session` (133), `execute_pending_save_loads` (**306 LOC**, L1744–2050),
     story-cursor / gear-handoff scratch resets (L1379–2050) ≈ 670
- **Evidence**: `prod_loc` → 2011; `scan.py` fn lengths: `execute_pending_save_loads` 306, `build_save_registry` 259, `SaveCommand::execute` 201.
- **Impact**: moderate — it is the P5 hardening hot file (5 fix commits in 3 days), and `execute_pending_save_loads` is a 306-line function that
  is the single most pinned function in the app layer.
- **Related**: **include_str! self-scan hazards — a naive split breaks 6 tests**:
  - `save_io/live_reload_tests.rs:416,602` find `"pub fn execute_pending_save_loads("` in `include_str!("../save_io.rs")` and then scan **from there to
    EOF**, searching for `restore_resources_subset(… PRE_RELOAD_RESOURCES)`, `reload_exterior_session(`, `reset_story_location_cursor(world)`.
  - `save_io/serde_default_guard_tests.rs:87` derives `registered_type_names` from the registry text in `save_io.rs`; also `:932` asserts a scanned
    path `ends_with("save_io.rs")`.
  - `save_io/round_trip_tests.rs:200` requires `save_io.contains(reconciler)` for each removed-at-runtime delta column.
  - `cell_loader/unload.rs:1478,1490` `source_of("save_io.rs")` + `fn_window(.., "fn reload_interior_session(", "\nfn reload_exterior_session(")`.
  After a split these need a `concat!(include_str!(...))` SOURCES constant (the `fragment.rs:SOURCES` pattern, #3854) or must be pointed at the
  new file that owns the scanned item, otherwise they go vacuous or red.
- **Suggested Fix**: `save_io/{mod.rs, registry.rs (1+2), player.rs (3), commands.rs (4), live_load.rs (5)}`; keep `execute_pending_save_loads`,
  `reload_*_session` and `PRE_RELOAD_RESOURCES` together in `live_load.rs` so the "scan from fn to EOF" tests keep their window.
  Do the SOURCES-constant repoint in the same commit. Separately, extract `execute_pending_save_loads` stages (restore → purge → reload → apply
  deltas → post-reload resets) into named helpers — but only with the existing pin tests moved first.
