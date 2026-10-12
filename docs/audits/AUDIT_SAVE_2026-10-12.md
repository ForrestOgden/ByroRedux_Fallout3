# Save / Load Subsystem Audit (M45 + M45.1) — 2026-10-12

**HEAD**: 5d31fd637 · **Baseline**: `docs/audits/AUDIT_SAVE_2026-10-09.md` (HEAD `3bcf6c8e8`) · **Audited**: Dimensions 1, 2, 4, 5 (delta-first) · **Unchanged since baseline (skimmed)**: Dimension 3 (Container & Disk). Over `3bcf6c8e8..HEAD`, `crates/save/src/{registry,driver,validate}.rs` and `crates/core/src/atomic_file.rs` have no commits. `disk.rs` changed only by #5613 (the test-only `latest_slot` wrapper was deleted; its tests now call `slots_by_recency().first()`). `snapshot.rs` changed only the `FORMAT_MAJOR` constant and its ledger doc.

This run is part of `/audit-suite --preset comprehensive`. All dimensions were analysed synchronously with no sub-agents; scratch notes are in `/tmp/audit/save/dim_{1..5}.md`. Nothing was launched except `cargo test` (toolchain 1.96.0, `TMPDIR=/mnt/data/tmp`, repo `target/`). No engine, smoke script or GPU process was started.

The window holds roughly 120 commits. The save-relevant ones:
- `FORMAT_MAJOR` 35 → 38 in three bumps:
  - v36, Wave 4 `886edcded`: `RigidBodyData.surface_material`.
  - v37, #5542 `c173147da`: the same field widened `u8` → `u32`.
  - v38, #5300 `f18224b02`: `DialogueSpokenInfoForms` element is now a `(speaker ref FormID, INFO FormID)` pair.
- The prior MEDIUM SAVE-D4-2026-10-09-01 was fixed by #5490 `99cd9fd6c`.
- The prior MEDIUM SAVE-D2-2026-10-09-01 was fixed by #5508 `627149b62`.
- #5435 `557ead21d` / `620970028`: every in-process load now resets the CLOC cursor and discards pending `StoryEvent` markers.
- #5458 `0aa844dbb`: the Eat/Sleep allowlist reasons were corrected and a guard added.
- #5483 `5a68e983e`: `player_owned_entities` protection in cell unload.
- #5469 `334458b3f`: the weak `#[cfg(test)]` source-scan cut was routed through `core::source_scan`.

Dedup sources: `/tmp/audit/issues.json` (122 open), `/tmp/audit/REPORTED_THIS_SUITE.md` (re-read before finalizing; no save-subsystem entries from other audits), and closed-issue searches for `part_surface_materials`, `RigidBodyData overlay`, `DialogueSpokenInfoForms` and `save-load-roundtrip`.

## Executive Summary

| `lib.rs` / `snapshot.rs` design claim | Status |
|---|---|
| Full ECS snapshot of game state | **CODE-CONFIRMED.** The only new `Component`/`Resource` impls in the window are `ImpactConfig` and `CombatSoundRequests`. Both are allowlisted with accurate reasons. One older allowlist reason, `CreatureActor`, is false because nothing stamps the marker (Existing GAME-D5-2026-10-12-01; see Dimension 1). |
| Versioned container + CRC32 over the payload | **CODE-CONFIRMED.** The header gates are unchanged; 41 + 15 `byroredux-save` tests pass. |
| Atomic write (tmp → fsync → re-read+verify → rename → dir-fsync) | **CODE-CONFIRMED.** `atomic_file.rs` has no commits. |
| Ring never clobbers the last good save | **CODE-CONFIRMED.** |
| Validation gate refuses to persist a poisoned save | **CODE-CONFIRMED, and the 10-09 drift is closed.** #5490 hands the pending convoy's whole `Children` closure to `release_entities`, so no orphaned `Parent` is left for `validate_world` to trip on. |
| `FORMAT_MAJOR` bump is the only schema-evolution path | **CODE-CONFIRMED for this window.** Each of v36/v37/v38 refreshed `BASELINE_SHAPE_FINGERPRINT` and `BASELINE_MAJOR` in the same commit as the bump. The ledger still lacks a v32→v33 entry (#5459, open). The SM-type guard blind spot (#5403) is unchanged. |
| Load runs off-frame; additive overlay + reconcilers | **CODE-CONFIRMED, with one DRIFTED sub-claim.** The drain sequence is intact and #5435 added two steps. `RigidBodyData` is an overlaid column and the overlay replaces the whole component, so a `serde(skip)` field the cell reload had just re-derived is overwritten with its default (SAVE-D1-2026-10-12-01). |
| Reproducible CRC at equal state | The doc disclaims it. `DialogueSpokenInfoForms` is a `HashSet`, so it is one more hash-ordered resource. Note only. |

**Findings: 3 NEW. 0 CRITICAL, 0 HIGH, 0 MEDIUM, 3 LOW.**

| Severity | Count | IDs |
|---|---|---|
| CRITICAL | 0 | — |
| HIGH | 0 | — |
| MEDIUM | 0 | — |
| LOW | 3 | SAVE-D1-2026-10-12-01, SAVE-D2-2026-10-12-01, SAVE-D5-2026-10-12-01 |

**Prior-cycle findings:**
- **SAVE-D4-2026-10-09-01 (MEDIUM): FIXED and verified by #5490 `99cd9fd6c`.**
  - `purge_cinematic_retention_state(world, ctx)` calls `take_pending_readoption_victims` (`byroredux/src/cell_loader/unload.rs:159`).
  - That function drains the list, takes each root's `collect_subtree` closure, filters the #5483 player set, and passes the result to `release_entities`.
  - `release_entities` also frees GPU handles, item instances and Rapier bodies.
  - Both reload arms (`save_io.rs:1434`, `:1589`) and both debug-load arms (`debug_load.rs:399`, `:491`) pass `ctx`.
  - New tests pin the closure and the player-rig exclusion: `pending_readoption_victims_are_the_whole_subtree_closure` and `pending_readoption_victims_spare_the_player_rig`.
  - The player-parented-by-a-victim corner is not reachable. Riders are attached by transform (`vehicle_attachment_system`), not by `Parent`.
- **SAVE-D2-2026-10-09-01 (MEDIUM): FIXED and verified by #5508.**
  - `ActorValue::undamaged()` (`crates/core/src/ecs/components/actor_values.rs:80`) is now the single ceiling composition.
  - `hud.rs:836` and `inventory.rs:317` use it.
  - The SDK projection folds `set_override` into `base` (`byroredux/src/extensions/capture.rs:240`), and a test pins it.
  - A `permanent_mod +` grep finds only `CharacterRuleset::actor_value`, which is correct as written (see the 10-09 report).
- **SAVE-D1-2026-10-08-01 (#5458): FIXED.** The `EatBehavior`, `SleepBehavior` and `EatSleepState` reasons now say `Seated` is `restore_world`-only. The `seated_without_path` guard at the tail of the completeness test pins that.
- **#5435 (the `StoryLocationCursor` reason): FIXED.** `reset_story_location_cursor` (`save_io.rs:1710`) runs on every in-process load, and `live_reload_tests.rs` pins it.
- **Still open and unchanged:** #5403, #5459, #5460 (the double `reconcile_worn_gear`, `save_io.rs:1939-1940`), #5059, #5060, #5585 (`save_io.rs` crossed 2000 production LOC).

## Data-Loss Class Matrix

| Finding | Class | Dim | Severity | Status |
|---|---|---|---|---|
| SAVE-D1-2026-10-12-01: the live-load overlay of `RigidBodyData` wipes the re-derived `part_surface_materials` | silent-drop (derived, non-progress state; the consumer is dormant today) | 1 | LOW | NEW |
| SAVE-D2-2026-10-12-01: the v38 `DialogueSpokenInfoForms` pair shape has no save round-trip test | none (test-gap) | 2 | LOW | NEW |
| SAVE-D5-2026-10-12-01: `docs/engine/save-load-roundtrip.md` §6 is stale: about ten missing steps and every line cite | none (doc-rot) | 5 | LOW | NEW |
| #5403, #5459, #5460, #5059, #5060, #5585 | see the baseline reports | 1, 2, 5 | MEDIUM / LOW | Existing |

## Completeness Ledger (delta from baseline; full table in `AUDIT_SAVE_2026-09-11.md`)

| Column / type | Kind | Saved | Overlaid | Notes |
|---|---|---|---|---|
| `RigidBodyData` (v36/v37) | Component | yes | yes (`MUTABLE_DELTA_COLUMNS`) | `surface_material: u32` is saved. `part_surface_materials: Box<[u32]>` is `serde(skip)`. See SAVE-D1-2026-10-12-01. |
| `DialogueSpokenInfoForms` (v33, reshaped in v38) | Resource, `HashSet<(u32, u32)>` | yes | n/a (wholesale) | Serializes as an array of 2-arrays. The speaker half is `SceneAliasCandidate::reference_form_id`, or `0` for an actor with no placed identity. No round-trip test: SAVE-D2-2026-10-12-01. |
| `ImpactConfig` (Wave 4) | Resource | no (allowlisted) | n/a | Boot-time sound config, empty until IPDS decode. Reason accurate. |
| `CombatSoundRequests` (#5148) | Resource | no (allowlisted) | n/a | Per-session diagnostic counter. Reason accurate. |
| `CreatureActor` (#5495) | Component | no (allowlisted) | n/a | The reason says "stamped from the CREA record in `prepare_creature_state`". `prepare_creature_state` (`npc_spawn/resumable/runtime.rs:399`) never inserts it, and no other production site does. This is the save-side echo of Existing GAME-D5-2026-10-12-01. The allowlist posture ("re-derived, not saved") is correct for any future stamp site. |
| `StoryLocationCursor` | Resource | no (allowlisted) | n/a | Reason rewritten (#5435): it is reset on every in-process load. |
| `CinematicReAdoption` | Resource | no (allowlisted) | n/a | Reason rewritten (#5490): the purge releases roots plus the `Children` closure. |

Two-list drift: none. `MUTABLE_DELTA_COLUMNS` is unchanged. The registry gained no column in the window; the only registry-visible changes are the two reshaped payloads (v36/v37 `RigidBodyData`, v38 `DialogueSpokenInfoForms`).

## Findings

### LOW

#### SAVE-D1-2026-10-12-01: the live-load overlay replaces a freshly imported `RigidBodyData` wholesale, wiping its `serde(skip)` `part_surface_materials`; the "re-derived at import, a snapshot never carries it" invariant does not hold on the overlay path

- **Severity**: LOW. It is dormant today, and the severity rises to MEDIUM when the per-surface sound tables land.
- **Dimension**: Snapshot Completeness & the Two Lists
- **Data-Loss Class**: silent-drop (derived render/audio data, not player progress)
- **Location**:
  - `crates/core/src/ecs/components/collision.rs:134-150` (`part_surface_materials`, `#[cfg_attr(feature = "inspect", serde(skip))]`, with the justification comment).
  - `byroredux/src/save_io.rs:146` (`"RigidBodyData"` in `MUTABLE_DELTA_COLUMNS`).
  - `crates/save/src/registry.rs:157-196` (the apply closure ends in `world.insert_batch::<T, _>(remapped)`).
  - `crates/physics/src/sync.rs:1042-1054` (the consumer).
  - `byroredux/src/save_io/serde_default_guard_tests.rs` (the 2026-10-10 "refreshed WITHOUT a bump" note) and `round_trip_tests.rs:657-658` (the round-trip test writes an empty slice).
- **Status**: NEW. Closed #5550 (the leaf-alignment walker) and #5542 do not mention save, and no closed issue covers the overlay.
- **Description**:
  1. The Wave 4 follow-through added `part_surface_materials`, the per-collider-part material sequence the NIF import derives from the collision tree.
  2. It was marked `serde(skip)` on the claim that a snapshot never needs it because "the tree is re-parsed from content on load and re-derived at import" (the #3470 class). The fingerprint was refreshed without a bump on that basis.
  3. That reasoning holds for the `restore_world` path, where the value is the only source. It does not hold for the live-load path.
  4. `RigidBodyData` is an overlaid column, and `apply_deltas` replaces each matched entity's component with the deserialized row. The reload has just spawned the entity with a correct `part_surface_materials` from the re-imported NIF, and the overlay overwrites it with `Box::new([])`.
  5. The overlay runs before any collider exists. The only callers of `register_newcomers_and_refresh_queries` are `scene.rs` `select_and_spawn_player_mode` (boot-only), the transition `ground_character_body_at`, a console command and tests. Neither reload arm calls it, so colliders register in the first scheduler tick after the drain.
  6. `physics_sync_system` then builds the colliders from the overlaid data. `collision_shape_to_parts(.., body_data.surface_material, &body_data.part_surface_materials)` sees an empty slice and gives every part the body-level material.
- **Evidence**:
  - Overlay path: `execute_pending_save_loads` → `apply_deltas(world, &registry, &snapshot, &remap, MUTABLE_DELTA_COLUMNS)` → the apply closure → `insert_batch`.
  - `insert_batch` → `insert_bulk` → `insert`, which overwrites.
  - It is the only `serde(skip)` field on a component in `MUTABLE_DELTA_COLUMNS`. The other skip fields in the tree (`quest_stages.rs`, `story_manager.rs`, `quest_alias.rs`) are resources, which are restored wholesale and are not affected the same way.
  - No test exercises it: `rigid_body_data_survives_save_load_round_trip` writes an empty slice, and nothing asserts that a non-empty value survives or is re-derived across a live load.
  - I did not run a probe. The result follows from the code path above.
- **Impact**:
  - After any in-process save load (interior or exterior), every FormID-matched collider whose NIF authored heterogeneous per-child Havok materials loses its per-part split. Footstep and impact lookups see one material for the whole body.
  - Today the per-surface sound tables are empty ("until IPDS decoding lands"), so nothing audible changes.
  - Once the tables are populated, the first load in a session silently degrades surface selection on compound statics, and a fresh boot behaves differently from a loaded game.
  - Savegame data is not lost.
- **Related**: #5550, #5542, #5574, #3470 (the skip-field precedent), Dim 1's "Replacing columns" and overlay notes.
- **Suggested Fix**:
  - Preferred: make the `RigidBodyData` apply keep the live `part_surface_materials` when the live row exists, for example a `register_*` apply hook that merges non-saved fields. Alternatively, drop `RigidBodyData` from `MUTABLE_DELTA_COLUMNS`, since the static and dynamic bodies it describes re-import identically.
  - Add a live-load test that spawns a body with a non-empty slice, overlays an identical saved row, and asserts the slice is intact.
  - Note: `crates/physics/src/sync.rs:1172` still says "the material keeps the low byte". After #5542 the user-data carries the full `u32`; the comment is stale. This is physics-owned and not filed separately.

#### SAVE-D2-2026-10-12-01: the v38 reshaping of `DialogueSpokenInfoForms` to a `(speaker, INFO)` pair has no save round-trip test, although it is a saved resource whose shape just changed

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

#### SAVE-D5-2026-10-12-01: `docs/engine/save-load-roundtrip.md` §6 (the authoritative load-apply trace) lists 8 steps and "two reconcilers"; the code has since grown about ten further steps, and every line cite is wrong

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

## Notes (not filed)

- **`CreatureActor` is stamped by nothing.** GAME-D5-2026-10-12-01 (gameplay audit, this suite) owns the finding. The allowlist reason text will be right once the stamp site exists, so no save-side change is needed beyond that fix.
- **#5435 semantics check.** Every load now raises one CLOC with `L1 = None` on its first frame, like a fresh boot.
  - `StoryManagerNodeState`, `QuestStageState` and `StoryEventAliasFill` are restored after the discard, so a CLOC-started scene quest is gated by its saved reset window and not re-fired unconditionally.
  - Restoring order was checked against the drain: reset and discard run directly after the reload returns `Some`, before `restore_extension_state` and the wholesale `restore_resources`.
- **`DialogueSpokenInfoForms` speaker `0`.** Actors with no placed identity (leveled spawns) share the `0` key, so one such actor saying a Say-Once line disqualifies it for all of them. The save format round-trips whatever the producer wrote. Gameplay and scripting own the semantics.
- **Determinism.** `DialogueSpokenInfoForms` is a hash-ordered resource: two saves of equal state can differ in bytes/CRC. #4748 is closed as a doc fix and the `Snapshot` doc disclaims it, so this is a note only.
- **`save_io.rs` size and self-scans.** Existing #5585 (LOC) covers this. The four test modules that self-scan its text were switched to `source_scan::production_text` / `strip_test_modules` by #5469 and are not vacuous.

## Guards Verified

| Guard | State |
|---|---|
| `cargo test -p byroredux-save` | 41 + 15 passed. |
| `cargo test -p byroredux --bin byroredux save_io` | 78 passed, 4 ignored (the four real-master `consumable_tests`), up from 75 at the baseline. |
| `cargo test -p byroredux --bin byroredux -- the_save_drain cell_loader::unload purge pending_readoption player_owned` | 31 passed, including `the_save_drain_publishes_the_transition_flag_before_draining` and the new #5490/#5483 tests. |
| `every_component_or_resource_impl_is_saved_or_explicitly_allowlisted` (+ stale-row check, + `seated_without_path`) | green. New rows spot-checked: `ImpactConfig`, `CombatSoundRequests`, `CreatureActor` (reason false, see notes), `EatBehavior`/`SleepBehavior`/`EatSleepState` (fixed), `StoryLocationCursor` (fixed), `CinematicReAdoption` (fixed). |
| `serde_default_on_saved_struct_requires_format_major_bump` | green. The new `part_surface_materials` is a skip-only field and is exempt. |
| `saved_type_shape_changes_require_format_major_bump` | green. `BASELINE_MAJOR = 38 = FORMAT_MAJOR`. All three bumps refreshed the fingerprint in the same commit, with a justification comment each. The "refreshed WITHOUT a bump" entries (#5495 `CreatureActor` file sweep, `part_surface_materials` skip field) carry no registered-shape change. The second one is the SAVE-D1-2026-10-12-01 premise. |
| `delta_columns_*` (2), `npc_spawn_stamped_components_are_saved_or_intentionally_rederived` | green. `delta_columns_removed_at_runtime_have_a_load_reconciler` now scans with `strip_test_modules` (#5469). |
| `command_queue_tests::*` (refusal, ring, quiescent drain) and `live_reload_tests::*` | green. The latter includes the new #5435 reset/discard tests. |
| `crates/save` header/CRC/atomic/ring tests, `write_slot_has_no_clobber_fallback` | green. |
| Removed-from-allowlist columns still registered | confirmed in `build_save_registry`: `CharacterController`, `RigidBodyData`, `Material`, `RumbleOnActivate`, `FragmentExecutionQueue`, `ActorCinematicState`, `HorseTetherState`, `StoryEventAliasFill`, and the replacing columns `Perks`, `TimedRestorations`, `ActorControlState`, `Dead`. |

## Summary per Dimension

| Dimension | Findings | Notes |
|---|---|---|
| 1 — Snapshot Completeness & the Two Lists | 1 LOW | `RigidBodyData` overlay wipes the skip field. No two-list drift. |
| 2 — Format & Schema Discipline | 1 LOW | v36/v37/v38 are sound. The `DialogueSpokenInfoForms` pair has no round-trip test. #5459 and #5403 are still open. |
| 3 — Container & Disk Durability | 0 | Unchanged; guards re-run. |
| 4 — Save-Side Gates | 0 | SAVE-D4-2026-10-09-01 verified fixed. The gate order is untouched. |
| 5 — Live Load-Apply & Frame Boundary | 1 LOW (doc) | The drain order matches the skill's sequence, plus the two #5435 steps. #5460 and #5060 are still open. |

Suggested next step: `/audit-publish docs/audits/AUDIT_SAVE_2026-10-12.md`. Labels:
- SAVE-D1-2026-10-12-01: `save-load` + `physics` + `bug`.
- SAVE-D2-2026-10-12-01: `save-load` + `test-gap`.
- SAVE-D5-2026-10-12-01: `save-load` + `doc-rot`.
