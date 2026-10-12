# Gameplay Audit — 2026-10-12

**HEAD**: 5d31fd637 · **Baseline**: `AUDIT_GAMEPLAY_2026-10-09.md` (`3bcf6c8e8`) · **Audited**: Dim 5 (NPC spawn → AI package → locomotion, in depth), Dim 1 (inventory / mid-life gear, `ec41e33e8`), Dim 2 (dialogue / force-greet / voice / reference persistence), Dim 4 (death producers, KILL raise, water ownership), Dim 7 (stage order, save coverage, determinism) · **Unchanged since baseline (skimmed)**: Dim 3 (zero commits in `inventory.rs::consume_item`, `restoration.rs`, `hardcore.rs`, `consumables.rs`), Dim 6 (only #5286/#5287 HUD hygiene and the #5508 `undamaged()` composition, both closed and verified)

**Scope**: `/audit-gameplay`, delta-first, run solo inside `/audit-suite --preset comprehensive`. 264 commits landed since the baseline. Of those, 72 touch the skill's dimension Paths (some overlap between dimensions). The ones read in full:
- `bcaf63aa8` (#5500) and `959b76f55` (#5527), the Eat/Sleep anchor and retry work.
- `f7ca4a60f` (#5499), selection past a declined Dialogue package.
- `b1fed839c` (#5501), seated-actor stream restore.
- `9bb8bc475` (#5495), the creature seating gate.
- `ec41e33e8` (#5031), mid-life gear imports.
- `f18224b02` (#5300), Say Once.
- `40b77828b` / `620970028` (#5432), voice cut.
- `c85d50356` (#5434), KILL on live deaths.
- `c28571563` (#5548), water damage ownership.
- `badc45309` (#5580), the shared walk write-back.
- `6316719ed` (#5452), `cached_walk_step`.
- `dae08000d` (#5492/#5504), PKDD offset 16.
- `f579785e3` (#5065), docs only.

**Evidence base**: no engine was launched.
- **Baseline suite** (the skill's Phase 1 filter set, `cargo test -p byroredux --bin byroredux` on 1.96.0): 363 passed, 0 failed, 20 ignored.
- **Dim 3/6/7 filter set** (consum, restoration, timed_, notifications, loading_screen, spawn_tests, registry_completeness, round_trip, scheduler_access, ambient_locomotion_default, locomotion_kill_switch, story_dispatch): 153 passed, 0 failed, 4 ignored.
- **ESM censuses** (raw Python over `FalloutNV.esm` and `Fallout3.esm`, scripts in `/tmp/audit/gameplay/scripts/`):
  - `census.py`: the Dialogue package winner simulation.
  - `travel_census.py`: Travel and Guard `NearReference` targets against actor placement order.
- **Determinism grep**: `rand::|Instant::now|SystemTime|thread_rng` over the skill's file list plus Dim 5 systems (production text only) is empty.

**Dedup applied**: `/tmp/audit/REPORTED_THIS_SUITE.md` re-read before finalizing.
- Already reported, not re-filed: CONC-D7-2026-10-12-01 (streaming salvage), ECS-2026-10-12-D6-01 (`footstep_system`), PERF-D1-2026-10-12-01 (`eat_sleep` per-frame `Vec`), ECS-2026-10-12-D3-01 (hand-rolled `Children` walks), #5641 / #5647 (Existing).
- Open issues re-confirmed and cross-referenced: #5642, #5645/#5646, #5460, #4232, #5408, #5462.

## Executive Summary

| Severity | NEW | Regression / incomplete fix | Existing (open) re-confirmed |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 1 | 1 | 1 (#4232) |
| LOW | 3 | 1 | 5 (#5642, #5460, #5645, #5646, #5647) |

The baseline's three MEDIUM findings (GAME-D5-2026-10-09-01/-02/-03) are closed by #5499, #5500, #5527 and #5501, and each fix is in the code. Two of them are still incomplete in ways the fixes' tests do not see:
- **#5495's creature gate never fires.** `CreatureActor` is read by both seat paths and stamped nowhere (GAME-D5-2026-10-12-01).
- **#5031's failure chain ping-pongs.** Two worn items that both fail to import retry each other forever (GAME-D1-2026-10-12-01).

Games and cells exercised statically: FNV and FO3 ESM data (Dialogue and Travel censuses). No cell was loaded.

## Invariant Matrix

| Invariant | Status | Evidence |
|---|---|---|
| Single damage path | VERIFIED | The production `Dead` inserts are `combat.rs:363`, `water.rs:76`, `character.rs:1585`, `extensions/commands.rs:543`, `reference_state.rs:299` and `:368`. The first four are the live producers (combat, NPC drowning, player drowning, the SDK AV batch); the last two are `restore` and `apply_starts_dead`. #5548 removed the controller's duplicate hazard arm; `water_damage_system` is the single owner and its guard uses `source_scan::production_text`. One gap: console `setav`/`modav` Health (GAME-D4-2026-10-12-01). |
| Death reconciled once | VERIFIED | Every live producer reaches `reconcile_dead_actor` (inline, or via `queue_dead_actor_reconciliation` into Late `reconcile_pending_dead_actors_system`) and raises KILL (#5434). `clear_ambient_behavior` still removes `EatSleepState`, `ForceGreetDirective`, `WalkStuckTimer` and `NavPath`; `reconcile_dead_actor` cuts the voice line (#5432). |
| Index stability | VERIFIED | No inventory row-removal path changed. `equipped_forms` counts only `count > 0` rows; zeroed rows stay (#5268). |
| Fail-closed consumables vs fail-open packages | VERIFIED (policy) / **DRIFTED (consequence, residual)** | `package_conditions_pass` is unchanged. #5499 skips a Dialogue package that passes only through fail-open, but not one that is fully modeled and then declined at install (GAME-D5-2026-10-12-02). |
| Stage order | VERIFIED | `restoration → interaction → flush → container_loot → combat_input → faction_hostility → npc_combat_ai → combat_damage` is unchanged. `ambient_ai_package_system → forcegreet_system → scene_package_system` is pinned by #5415. Late `water_damage_system` (`late.rs:207`) precedes `reconcile_pending_dead_actors_system` (`:222`), so a drowning death is reconciled the same frame. The KILL marker raised in Early/Late is drained by `story_manager_dispatch` the next frame by design. |
| State saved or re-derived | VERIFIED, with one false classification | #5501 restores `Seated` through the seating write path (marker index from the reservation map, sit clip, reservation re-insert). `DialogueSpokenInfoForms` is saved (`FORMAT_MAJOR` 38). The `CreatureActor` allowlist row claims it is re-stamped on every spawn, which is false (GAME-D5-2026-10-12-01). |
| Determinism | VERIFIED | The grep is empty. `DialogueRandomState` and `StoryManagerRng` seed from `SystemTime` by design (documented `NOT_SAVED_BY_DESIGN`, owned by `/audit-scripting`). |

## Findings

### MEDIUM

### GAME-D5-2026-10-12-01: `CreatureActor` is read by both seat paths but stamped by nothing, so #5495's creature seating gate is dormant and 59 FNV creature placements still take furniture
- **Severity**: MEDIUM
- **Dimension**: NPC Spawn → AI Package Selection → Locomotion (Dim 5), with Dim 7 (a false save-allowlist classification)
- **Location**: `byroredux/src/npc_spawn/resumable/runtime.rs:399-460` (`prepare_creature_state`, `let _ = world;` at `:437`); readers at `byroredux/src/systems/sandbox.rs:311-313` and `byroredux/src/systems/eat_sleep.rs:500-505`; the false classification at `byroredux/src/save_io/registry_completeness_tests.rs:569`
- **Status**: Regression of #5495 (the fix never took effect). Not covered by any open issue.
- **Trigger**: FNV/FO3 cell holding a `CREA` placement whose PKID list wins a Sandbox, Eat or Sleep package: Vault 11 mantises, REPCONN ghouls, Primm Slim, Gibson's dog; 59 FNV placements across 14 cells and 15 FO3, per the #5495 commit message.
- **Description**: #5495 added a `CreatureActor` marker so that `sandbox_seat_system_inner` skips creatures before any seat scan, and `seat_at_marker` returns after the walk. The commit and the registry row both say `prepare_creature_state` stamps it. That function never does:
  - It takes `world: &mut World`, then discards it with `let _ = world;`.
  - `grep -rn CreatureActor byroredux/src crates` finds the type definition, the debug-registry entry, the two read sites, the allowlist row and the tests. There is no `insert`, `insert_at_spawn` or bundle stamp in production.
  - The behavioural tests pass because they insert the marker by hand (`sandbox.rs:547`, which then removes it for the control arm).
  - `world.query::<CreatureActor>()` therefore reads an empty or absent storage. Nothing distinguishes a mantis from a settler on the seat paths.
- **Evidence**:
  ```rust
  // runtime.rs:437 — the only mention of `world` after the early signature
  let _ = world;
  RuntimeNpcState { appearance: NpcLootAppearance::default(), placement_root, … }
  ```
  ```rust
  // eat_sleep.rs:500 — the gate that can never be true in production
  if world.get::<byroredux_core::ecs::components::CreatureActor>(npc).is_some() { return false; }
  ```
- **Impact**: The #5495 failure persists. Creatures that win Sandbox or Eat/Sleep are snapped onto chair and bed markers, are parked on the humanoid sit clip on a rig that lacks its bones, and hold a `SeatReservations` entry no NPC can take for the life of the cell. `Seated` is a one-shot guard, so they stay frozen.
- **Related**: #5495 (closed), GAME-D5-2026-10-08-xx origin finding, `registry_completeness_tests.rs:569` (false claim), ECS-2026-10-12 noted the component as "registered in the debug registry" only.
- **Suggested Fix**: Stamp `CreatureActor` on `placement_root` in `prepare_creature_state` (the function already receives `&mut World`). Pin it with a spawn-level test that runs `prepare_creature_state` for an `is_creature` record, and add the `CreatureActor` stamp to the `npc_spawn_stamped_components_are_saved_or_intentionally_rederived` coverage so the allowlist claim is tested rather than asserted.

### GAME-D1-2026-10-12-01: #5031's "hold out the failed form" chain only excludes the latest failure, so two worn items that both fail to import retry each other every frame
- **Severity**: MEDIUM
- **Dimension**: Inventory & Equipment Model (Dim 1: mid-life gear / `GearImportLoader`)
- **Location**: `byroredux/src/npc_spawn/loot_appearance.rs:560-640` (`reconcile_worn_gear_excluding`, the `failed_form` filter at `:629`), `:953-958` (file-level miss), `:991-1000` (failed NIF import)
- **Status**: NEW (an incomplete part of the #5031 fix, `ec41e33e8`)
- **Trigger**: any wearer (NPC after a save-load `reconcile_worn_gear`, a looted NPC, or the player) with two or more equipped forms that have no `NpcEquipmentPart` root and whose meshes both fail, whether the mesh is missing from the loaded archives (a missing DLC archive, an unresolvable loose path) or the NIF fails to import.
- **Description**: `PendingGearImport` is a one-slot queue per wearer. When an import fails, the loader removes the request and chains `reconcile_worn_gear_excluding(world, wearer, Some(failed))`, which re-queues the first root-less form that is not `failed`. With forms A and B both failing:
  1. A fails. The chain excludes A, so `missing = [B]` and B is queued.
  2. B fails. The chain excludes only B. A still has no root and is not excluded, so `missing = [A]` and A is queued.
  3. A fails again, and the loop repeats, one attempt per frame, for as long as the wearer holds both items.
  
  With three or more failing forms the cycle is longer but equally unbounded. A single failing form is bounded (A fails, `missing = []`, stop). A failing form beside a succeeding one retries once per success. The two-failure case has no terminus. The unit test `a_failed_import_does_not_stall_the_forms_behind_it` checks one step and a one-form wearer, so it cannot see the cycle.
- **Evidence**:
  ```rust
  // step_imports, file-level miss
  let Some(bytes) = textures.extract_mesh(&path) else {
      world.remove::<PendingGearImport>(wearer);
      log::warn!("mid-life gear: missing {path}; item {wearer} wears no mesh for it");
      reconcile_worn_gear_excluding(world, wearer, Some(import.form_id));   // :957, excludes only THIS form
      return;
  };
  ```
  `missing` (`:626-634`) is `equipped_forms` minus rooted forms minus `failed_form`, so the previous failure re-enters it.
- **Impact**: One `log::warn!` plus a mesh extraction attempt per frame, indefinitely. For the "import failed" arm (`:967-1000`) that is a full `load_nif_bytes_with_skeleton` per frame; when the failed import created partial entities on a wearer with a `CellRoot`, each cycle stamps another hidden entity range onto the cell (released only at cell teardown).
- **Related**: #5031 (closed), #5460 (open: `reconcile_worn_gear` called twice back-to-back in the load drain), #5266.
- **Suggested Fix**: Carry the failed set on the wearer (a small `Vec<u32>` on a runtime-only component, cleared when the equipment slots change) rather than a one-form parameter, and have `reconcile_worn_gear_excluding` filter against all of it. Add a two-failing-forms test that drives `step_imports`-equivalent calls three times and asserts the queue empties.

### LOW

### GAME-D5-2026-10-12-02: #5499 skips only fail-open Dialogue packages; a fully-modeled Say-To or off-target Dialogue package still wins selection and is then declined at install
- **Severity**: LOW
- **Dimension**: Dim 5
- **Location**: `byroredux/src/npc_spawn/ai_package.rs:755-785` (`select_active_package`; the guard is `package_conditions_fully_modeled` only), `:296-319` (the Dialogue arm returns `None` for `targets_player == false` or `dialogue_type != 0`)
- **Status**: NEW (the residual of closed #5499, made reachable by closed #5492)
- **Trigger**: FNV/FO3. An NPC whose first scheduled package at the current hour is a Dialogue package that is Say-To (PKDD type 1) or targets a non-player reference, and whose conditions are all catalogued so that fail-open does not apply.
- **Description**: #5499 fixed the case where a Dialogue package passes selection through fail-open. The two other install gates are still evaluated only after selection: the player-target check and the Conversation-type check. #5492 (offset-16 decode) made the Say-To gate live for the first time, so a package it declines now wins selection and installs nothing. `from_package` returns `None`, the winner's FormID is still recorded, every later minute tick re-selects it with `changed == false`, and the actor runs none of the packages below it.
- **Evidence**: A lower-bound census (`census.py`; first scheduled package only, conditions not evaluated, so a conditioned package only counts when it would pass at runtime):
  - **FNV**: 1 NPC_, `JacobstownKeeneDialogue` (Say-To, no conditions), at every sampled hour.
  - **FO3**: 6 NPC_ bases at hours 3/12/20: 5 off-target (examples `CG02GuardGreetOverseer`, `FFEU19NPC1Dialogue` (no conditions), `MQ01BoSPostbattleDEMODialogue`, `CG03DadTalktoJonas`, `EclairZipTalk22x0`) and 1 Say-To (`CG03JonasToPlayer`). All but one carry `GetStage` (fn 58) or `GetInCell` (fn 67) conditions, so they win only while those hold.
  - **Package level** (fully-modeled Dialogue packages the install gates decline): FNV 45 (34 off-target Conversation, 1 off-target Say-To, 10 player-targeted Say-To); FO3 41 (36 off-target Conversation, 1 off-target Say-To, 4 player-targeted Say-To).
- **Impact**: Small and mostly quest-gated: while the package's window and conditions hold, the affected actor stands at its spawn point instead of running the Sandbox/Eat/Sleep/Travel below it. Jacobstown's Keene never runs an ambient procedure.
- **Related**: #5499 (closed), #5492 (closed), GAME-D5-2026-10-09-01 (the original), #5376.
- **Suggested Fix**: Extend the selection predicate to the same three gates as the install arm (a shared `dialogue_installs_ambient(package)` used by both), so a package the arm would decline is skipped by the scan.

### GAME-D5-2026-10-12-03: Travel, Escort-lead and Guard resolve their `NearReference` target once and freeze the fallback when the target is not spawned yet — the #5500 race, fixed only for Eat/Sleep
- **Severity**: LOW
- **Dimension**: Dim 5
- **Location**: `byroredux/src/systems/travel.rs:99-126` (`resolve_near_reference_target`, `resolve_destination`) and `:236-242` (the first-sight resolve whose result is stored in `TravelState`); `byroredux/src/systems/escort.rs:111-118`; `byroredux/src/systems/guard.rs:87` (`unwrap_or(home)`)
- **Status**: NEW (sibling of #5500 / GAME-D5-2026-10-09-02 item 3)
- **Trigger**: FNV/FO3 budgeted cell load (door transition under the loading cover, or boot). A Travel, Escort or Guard actor spawns before the reference its PLDT names.
- **Description**: #5500 stopped Eat/Sleep from caching the actor's current position when the anchor is unresolved. The same shape remains in the other walk-to-point movers. Travel resolves its destination on its first tick; if `resolve_entity_by_global_form_id` misses, it falls back to a hash-picked point within the radius of the actor and stores that in `TravelState`, which is never re-resolved. Guard falls back to the actor's spawn position, with the same freeze. The scheduler runs every frame between budgeted load slices (`app_events.rs:983` runs `scheduler.run`, then `:1082` `step_cell_transition`; `dt` is pinned to 0 under the loading cover, but the first-sight resolve does not need `dt`), so a spawn-time package winner can resolve before its marker exists.
- **Evidence**: Census (`travel_census.py`, FNV, hour 12, conditions ignored, so an upper bound): Travel `NearReference` winners whose target REFR is placed after the actor in the same cell 61; target in another cell 89 (the fallback is the documented behaviour for a non-resident target); Guard, same-cell-after 20.
- **Impact**: An affected Travel actor walks to a random point near itself instead of the authored marker, or a Guard holds its spawn position instead of the authored post, for the life of the cell. Many of the counted packages are quest-gated, and the race depends on how the load is sliced, hence LOW.
- **Related**: #5500 / #5527 (closed, Eat/Sleep), GAME-D5-2026-10-09-02.
- **Suggested Fix**: For a `NearReference` target that does not resolve but whose form is not known to be non-resident, leave `TravelState` unset and retry on the `ANCHOR_RETRY_SECS` clock `eat_sleep` already uses, falling back only after the load settles.

### GAME-D5-2026-10-12-04: #5500's In-Cell search radius is unbounded, so the seat snap becomes an arbitrary-distance teleport
- **Severity**: LOW
- **Dimension**: Dim 5
- **Location**: `byroredux/src/systems/eat_sleep.rs:336-346` (`EatSleepLocation::InCell(_) => f32::INFINITY`), `:431-450` (`seat_at_marker`), `byroredux/src/systems/sandbox.rs:121-135` (`pick_nearest_seat`)
- **Status**: NEW (a consequence of the #5500 fix, `bcaf63aa8`)
- **Trigger**: FNV/FO3. An Eat or Sleep package with PLDT type 1 (In Cell); all 70 FNV In-Cell Eat/Sleep packages author radius 0.
- **Description**: For In Cell the walk destination is the actor's current position, so the walk leg is a no-op and the actor is "arrived" immediately. #5500 widened the seat search from 512 BU to the whole cell. `apply_seat_assignments` then snaps the root's translation and rotation to the chosen marker. The nearest free marker can be rooms away. The actor teleports there at the first tick in its schedule window, in view of the player if the player is present, and (on FO3/FNV, where every legacy marker reads as Sit, #5390) can be a chair.
- **Evidence**: `pick_nearest_seat`'s filter is `d2 <= r2` with `r2 = INFINITY`; no walk leg exists between the pick and the snap.
- **Impact**: Visible pop-in at schedule boundaries; far from the diner/sleeper's room. Nothing is mis-persisted.
- **Related**: #5500, #5390, GAME-D5-2026-10-09-02 item 1.
- **Suggested Fix**: When the pick falls outside `ARRIVE_RADIUS`, set `EatSleepState.destination` to the chosen marker and let `cached_walk_step` walk there before seating, instead of snapping.

### GAME-D4-2026-10-12-01: Console `setav`/`modav` on Health can leave an actor at Health ≤ 0 without `Dead`, KILL or reconciliation
- **Severity**: LOW
- **Dimension**: Dim 4 (Health writers outside `combat_damage_system`, #4702's rule)
- **Location**: `byroredux/src/commands/actor_value.rs:59-110` (`edit_av`)
- **Status**: NEW
- **Trigger**: `setav`/`modav` with a Health AV and a value that drives the composed current to ≤ 0, on an NPC or the player.
- **Description**: #4702 made the SDK AV batch (`commit_actor_value_deaths`) apply the alive→dead transition, with the stated rule that every Health writer outside `combat_damage_system` owes it. The console's `edit_av` calls `set_base`, `set_override` or `mod_permanent` and returns, with no death check. The actor stays alive at ≤ 0 HP: its AI keeps running, `consume_item` refuses it (`values.current(health) <= 0.0` returns `Unavailable`, `inventory.rs:1425`), and `restoration_system` clears its timers, so it cannot be healed by consumables either. A later hit kills it.
- **Impact**: Developer console only; no shipped gameplay path. Recorded because it breaks the "single owner of the alive→dead edge" invariant.
- **Related**: #4702 (closed), `extensions/commands.rs:520-545`.
- **Suggested Fix**: Share `commit_actor_value_deaths` with `edit_av` (it already takes the committed entity list).

## Verified fixed since the baseline

| Item | Verdict |
|---|---|
| #5499 (selection continues past a fail-open Dialogue decline) | In the code (`ai_package.rs:755-785`); residual is GAME-D5-2026-10-12-02 |
| #5500 / #5527 (Eat/Sleep type 6 and In-Cell anchors, retry clock, one gather) | In the code; the walk reads are gathered once per frame; lock order `Transform → GlobalTransform → WalkSpeed` (#5535); consequence GAME-D5-2026-10-12-04 |
| #5501 (`Seated` stream restore through the write path) | In the code (`stream_snapshot.rs:405-490`); drops the restore when the clip, marker index or furniture is missing |
| #5495 (creature seating gate) | Gate present, never armed: GAME-D5-2026-10-12-01 |
| #5452 / #5449 (cached NAVM walk, KCC `blocked`, coarse seat retry) | In the code; force-greet and Eat/Sleep both ride `cached_walk_step` |
| #5580 (six movers share `write_back_walk`; Patrol runs Wander's generic body) | In the code; guard `walk_movers_share_one_pass_two_write_back` green |
| #5430 / #5392 (force-greet kill switch, `player_can_act` gate) | In the code (`forcegreet.rs:105-112`) |
| #5432 / #5435 (voice cut on replace, close and death) | `stop_line_voice` is called from `combat.rs:581`, `npc_dialogue.rs:368` and `:573` |
| #5434 (KILL on every live producer) | `combat.rs:371`, `water.rs:80`, `character.rs:1589`, `extensions/commands.rs:547` |
| #5548 (water damage single owner) | In the code; guard uses `production_text` |
| #5300 (Say Once per actor, `FORMAT_MAJOR` 38) | In the code; speaker keyed by `SceneAliasCandidate.reference_form_id` (0 for an unplaced actor, which degrades to the old global-per-INFO behaviour) |
| #5031 (batch folded as a log, live-slot recheck) | In the code; failure chain is GAME-D1-2026-10-12-01 |
| #5498 (FO4 `TPTA` per-flag templates) | Parsed (`npc.rs:732`) and consumed (`equip.rs:593`) |
| #5428 / #5450 (script-killed corpse provenance) | `LoadOrder::script_winner` has a production caller (`reference_state.rs:427`) |
| #5065 (resident `Enable()`/`Disable()`) | Closed as documentation only: a resident reference still gets no live strip or re-spawn until its cell reloads (ROADMAP M47 tracker) |

## Dimension notes

- **Dim 1**: besides #5031, the only delta is `274ba13c5` (#5342, a CHARAL capture row), #5508 and #5513. Guards `armor_toggles_never_touch_the_equipped_weapon` and `load_reconciler_*` are green.
- **Dim 2**: `reference_state`, `unload` and `references/*` changes are #5483/#5484 stamp-cursor and XCRI work, #5490/#5513 subtree release, and the #5065 docs. The `reference_state::tests` and `synth_child` guards are green. #5642 (force-greet pass 2 opens every in-radius greeter against a stale `open_partner`) remains open and unchanged (`forcegreet.rs`, the `open_partner` snapshot and its `match`).
- **Dim 3**: no change. The `#[ignore]`d Hardcore/perk test still guards nothing unless run `--ignored`.
- **Dim 4**: the `Dead` producer set matches the baseline (see the matrix). `commands/actor_value.rs` is the one Health writer outside the set.
- **Dim 5 four-list diff**: Eat, Sleep and Dialogue are present in `from_package`, `insert_at_spawn`, `insert_at_runtime` and `clear_ambient_behavior`; the debug registry carries `CreatureActor`, `EatBehavior`, `SleepBehavior`, `EatSleepState` and `EditorPlacement` (`registration.rs:219`, `:292-301`). `ForceGreetDirective` is binary-crate-local and unregistered, as before. The old per-procedure env vars stay unread (only `BYRO_NO_AI_LOCOMOTION`).
- **Dim 6**: the `app_frame.rs:254` drain still splices `drain_transition_notifications` ahead of the producers; #5286's registry-absent path is covered by `transition_drain_survives_an_absent_definition_registry`.
- **Dim 7**: new components or resources in the delta are `CreatureActor` and `ImpactConfig` only; `ImpactConfig` is audio/physics. The `registry_completeness_tests` row for `CreatureActor` states a stamp that does not exist (GAME-D5-2026-10-12-01). `DialogueSpokenInfoForms` is saved at `FORMAT_MAJOR` 38 and the round-trip suite is green.

## Known-Open Register

| Issue | State |
|---|---|
| #4232 `effective_actor_level` returns 0 | open, unchanged |
| #5642 force-greet pass 2 stale `open_partner` | open, unchanged |
| #5460 `reconcile_worn_gear` runs twice back-to-back in the load drain | open, unchanged |
| #5645 / #5646 `GetRandomPercent` | open (scripting) |
| #5647 `StoryEvent` one slot per entity | open (scripting); the KILL raise in GAME-D4 depends on it |
| #5641 `player_controller_system` Access row omits the KILL producer | open (ECS) |
| #5408 / #5462 | open (Skyrim) |
| #5464 / #5526 | open: `.fuz` voice and Oblivion voice unbuilt |
| #5065 | closed as documentation: resident `Enable()`/`Disable()` has no live consumer |
| #5367 and follow-ups (#5429, #5433, #5521, #5432) | closed |

## Scratch-file reconciliation

I checked `/tmp/audit/gameplay/dim_5.md` against this report.
- F1 is GAME-D5-2026-10-12-01.
- F2 is GAME-D1-2026-10-12-01.
- F3 is GAME-D5-2026-10-12-02.
- F4 is GAME-D5-2026-10-12-03.
- GAME-D5-2026-10-12-04 and GAME-D4-2026-10-12-01 were added from the read-through, with no separate scratch note.
- No other `dim_N.md` finding existed: Dim 3 and Dim 6 were unchanged, and the Dim 1, Dim 2 and Dim 7 reads produced none beyond the ones above.

## Summary

| Severity | NEW | Regression / incomplete | Existing (tracked) |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 1 | 1 | 1 |
| LOW | 3 | 1 | 5 |

Suggested next step: `/audit-publish docs/audits/AUDIT_GAMEPLAY_2026-10-12.md`. Suggested labels:
- **GAME-D5-2026-10-12-01**: `bug`, `medium`, `gameplay` + `ai`, `game:fnv`.
- **GAME-D1-2026-10-12-01**: `bug`, `medium`, `gameplay` + `inventory`.
- **GAME-D5-2026-10-12-02**: `bug`, `low`, `gameplay` + `ai`.
- **GAME-D5-2026-10-12-03**: `bug`, `low`, `gameplay` + `ai`.
- **GAME-D5-2026-10-12-04**: `bug`, `low`, `gameplay` + `ai`.
- **GAME-D4-2026-10-12-01**: `bug`, `low`, `gameplay` + `combat`.
