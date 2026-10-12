# SAVE-D2-2026-10-12-01: the v38 reshaping of `DialogueSpokenInfoForms` to a `(speaker, INFO)` pair has no save round-trip test, although it is a saved resource whose shape just changed

Labels: low,save-load,dialogue,test-gap,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5688

Filed from `docs/audits/AUDIT_SAVE_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Format & Schema Discipline
- **Data-Loss Class**: none (test-gap; the shape is very likely correct)
- **Location**:
  - `crates/scripting/src/dialogue.rs:54` (`pub struct DialogueSpokenInfoForms(pub std::collections::HashSet<(u32, u32)>)`).
  - `byroredux/src/save_io.rs:543-548` (registration).
  - Searched with `grep -rln DialogueSpokenInfoForms`. The only test-tree hits are the shape-baseline comments in `serde_default_guard_tests.rs`.
- **Status**: NEW. It sits beside the open #5459, which covers only the missing v32→v33 ledger entry.
- **Description**:
  1. Most saved gameplay resources have a `*_survives_save_load*` test (`QuestStageState`, `StoryEventAliasFill`, `StoryManagerNodeState`, the lock and enable ledgers, `Perks`, and so on).
  2. `DialogueSpokenInfoForms` has none. It was registered at v33 (#5367) and reshaped at v38 (#5300), and no test in the repo serializes or restores it.
  3. The only guarantee is the type-level one: `HashSet<(u32, u32)>` serializes as an array of 2-arrays under `serde_json` and decodes back. That is correct by inspection, and the typed preflight would catch a decode failure before teardown.
  4. What nothing pins is the contract: a Say-Once line spoken by speaker A stays disqualified for A, and remains available to speaker B, after an F5/F9 cycle.
- **Evidence**: the grep above. The v38 commit touched `snapshot.rs` and `serde_default_guard_tests.rs` (bump and baseline) and no round-trip test.
- **Impact**: a future change to the pair key (for example mapping `0` speakers, or switching to a map keyed by tuple, which `serde_json` rejects) would not be caught by the guards. The shape baseline and `FORMAT_MAJOR` pin the shape, not the semantics.
- **Related**: #5300, #5367, #5459.
- **Suggested Fix**: add `dialogue_spoken_info_forms_survive_save_load` to `round_trip_tests.rs`. Save a world with two `(speaker, info)` pairs, restore it, and assert the set and the per-speaker `select_info` result.

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix

