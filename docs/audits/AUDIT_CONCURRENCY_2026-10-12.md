**HEAD**: `5d31fd637` · **Baseline**: `docs/audits/AUDIT_CONCURRENCY_2026-10-09.md` (@ `3bcf6c8e8`, 264 commits ago) · **Audited**: Dim 7 in full (the #5491 shutdown fix, the #5512 stale-payload salvage, the new `sf-cdb-index` thread, the shared archive registry); Dim 3 and Dim 4 for their deltas (49 and 9 commits, the lock-order lanes re-run); Dim 5 for its delta (rapier 0.22→0.36, the #5490 purge fix, the new contact-force channel, the PhysicsWorld-sink sweep, the non-canonical-despawn sweep) · **Unchanged since baseline (skimmed)**: Dim 1, Dim 2 and Dim 6. Their only commits are source-scan routing, doc comments, byte-identical shader-constant refactors (#5620), shader math, the egui 0.36 discard API and the `MAX_SKIN_SLOTS` constant; no queue submit, fence, barrier, AS build or destroy was added anywhere in the repo diff. Guards run for each.

# Concurrency & Synchronization Audit — 2026-10-12

**Command**: `/audit-concurrency` (all dimensions, depth `deep`), run inside `/audit-suite --preset comprehensive`.
**Severity scale**: `.claude/commands/_audit-severity.md`.

## Method

- **Delta scope.** I ran `git log 3bcf6c8e8..HEAD -- <Paths>` per dimension and analysed all seven dimensions myself in this session, with no sub-agents. Per-dimension notes are in `/tmp/audit/concurrency/dim_{1..7}.md`. The report was reconciled against each of them.
- **No engine launch** (suite rule). The lock-order lane that drives a real Vulkan device (CI `vulkan-validation`) needs an engine run, so I read its status from CI with `gh` (read-only). The test-only lanes ran locally.
- **Dedup.** Sources checked: open issues (`/tmp/audit/issues.json`, 122), closed-issue searches (`salvage stale imports`, `5512`), the baseline report, and `/tmp/audit/REPORTED_THIS_SUITE.md` (renderer, performance, ecs, safety). No overlap with any of them for the one NEW finding.
- **Sweeps added from the baseline's skill-drift list.**
  - Non-canonical despawn sweep, `grep -rn 'despawn_batch(\|\.despawn(' byroredux/src/cell_loader byroredux/src/systems`: 12 production hits. Each sits inside `release_entities`, a LOD/water unload block that releases GPU state first, or a `reference_state` path that calls `release_victim_rapier_bodies` before the despawn. Clean.
  - PhysicsWorld-sink sweep over every `PhysicsWorld` acquisition in `byroredux/src/`: every guard is scoped with nothing acquired under it. New sites checked: `footstep_system`, `impact_audio_system`, `cached_walk_step`.

### Guard runs at HEAD (local, rustc 1.96.0, `TMPDIR=/mnt/data/tmp`)

| Command | Result |
|---|---|
| `BYRO_LOCK_ORDER_CHECK=1 cargo test -p byroredux --bin byroredux --no-fail-fast` | **2829 passed, 0 failed**, 58 ignored (2743 at baseline) |
| `BYRO_LOCK_ORDER_CHECK=1 cargo test -p byroredux-scripting --no-fail-fast` | **562 passed, 0 failed**, 5 ignored (532 at baseline) |
| `BYRO_LOCK_ORDER_CHECK=1 cargo test -p byroredux-physics --no-fail-fast` | **244 passed, 0 failed** (208 at baseline) |
| scheduler-proof tests inside the bin run | all ok, none ignored (`build_scheduler_reports_zero_access_conflicts`, `scheduler_access_invariants_hold_on_the_real_schedule`, `the_parallel_system_table_covers_every_parallel_registration`, `footstep_declares_its_physics_reads`, the ordering pins) |
| `cargo test -p byroredux-renderer --lib -- <Dim 1/2 guard filters>` | 11 passed, 0 failed |
| `cargo test -p byroredux-bsa --lib -- concurrent threaded parallel` | 3 passed |

`PARALLEL_SYSTEMS` rows (9) equal the `add_to_with_access(` registrations (9).

CI evidence (read with `gh`; run 38274857531 at `f579785e3`, plus 38254842761 and 38250857080):
- **ABBA lock-order detector**: green on all three.
- **Vulkan validation layers (lavapipe)**: green on all three.
- **Test + Check + Clippy**: red, on the `cargo clippy` step only. The errors are one `map(f)` on an `Option` returning `()` in `byroredux-physics` (`crates/physics/src/world/mod.rs`) and two `doc list item without indentation` errors in `byroredux-renderer`. This is not a lock lane and does not hide one. The ecs audit of this suite notes the same step; compare #5621.

## Summary

| Severity | NEW | Regression | Existing |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 1 | 0 | 0 |
| LOW | 1 | 0 | 0 |

(The counts above are this report's own findings. Overlapping Existing issues are listed separately below and are not re-reported.)

**Headline.**
- **All three baseline findings are fixed and confirmed in code:**
  - CONC-D7-2026-10-09-01 → #5491 (`b6aeecf56`): `shutdown` sets a shared `cancel` flag before it drops `request_tx`, and `recv_next_live_request` discards any request received after it. Test: `cancel_flag_stops_a_worker_with_a_queued_backlog`.
  - CONC-D5-2026-10-09-01 → #5490 (`99cd9fd6c`): `purge_cinematic_retention_state` now takes the `VulkanContext`, collects each pending root's `Children` closure with `collect_subtree` (minus the #5483 player set), and hands it to `release_entities`. Test: `pending_readoption_victims_are_the_whole_subtree_closure`.
  - CONC-D7-2026-10-09-02 → #5512 (`b5ae3e5cb`): stale payloads now finish their parsed imports into the registry. That fix is the source of the one new MEDIUM below. Test: `stale_payload_imports_are_salvaged_into_the_registry`.
- **The lock graph is clean at HEAD.** The #5535 inversions (`eat_sleep`, hair head-mount, the #5501 seated restore) stay fixed, and the detector is green on the bin, scripting and physics graphs. I found no new lock-order hazard.
- **One new thread, one new channel, one new process-wide registry**; all three are sound:
  - `sf-cdb-index` (#5291, `cdb.rs`): a detached builder thread behind a single-flight `OnceLock` slot and a `PREFETCH_GATE` mutex. There is no lock cycle. Its residual flaws are already filed (#5665, #5666).
  - `PhysicsWorld::contact_force_rx` (an `mpsc` receiver inside a `Mutex`): there is exactly one `pipeline.step` site, and it drains the channel after each substep.
  - `Archive::open_shared` (#5288): the registry lock is held for map access only, never across the open.

| ID | Sev | Dim | Status | Title |
|---|---|---|---|---|
| CONC-D7-2026-10-12-01 | MEDIUM | D7 | NEW (side effect of the #5512 fix) | Stale-payload salvage finishes every parsed import of every stale payload in one unsliced loop: no `should_yield` check, work charged to the budget only after it is done; the door/save drain pays it too |
| CONC-D5-2026-10-12-01 | LOW | D5 | NEW | `PhysicsWorld::contact_force_rx`'s comment says rapier's narrow phase calls the event handler from rayon workers; the build has no `parallel` feature |

---

## Findings

### CONC-D7-2026-10-12-01: Stale-payload salvage runs every parsed import of every stale payload synchronously, outside the frame budget's yield check
- **Severity**: MEDIUM. This is a main-thread hitch with no upper bound in the streaming apply path. It is not a correctness bug or a deadlock. The size of the hitch is unmeasured: a play session's `worker_stale_salvaged_imports` telemetry would size it, and I downgrade to LOW if that stays near zero.
- **Dimension**: Worker Threads (worker ↔ main handoff)
- **Location**:
  - `byroredux/src/streaming_helpers.rs:886-900` (the stale arm of the payload receive loop in `advance_streaming_apply`); the helper is at `:721-740` (`salvage_stale_imports`)
  - `byroredux/src/streaming_helpers.rs:1052` (`consume_streaming_payload`, `:1029`)
  - `byroredux/src/streaming_helpers.rs:742-766` (`cancel_streaming_apply_job`, whose `FinishImports` arm now salvages), reached from `cancel_active_streaming_apply`, which `drain_streaming_state` calls at `:526`
  - What each salvaged entry costs: `byroredux/src/cell_loader/partial.rs:20-` (`finish_partial_import`)
- **Status**: NEW. The matching closed issue is #5512, whose fix introduced it. The performance audit's PERF-D7-2026-10-12-01 is the LOD-water rebuild, a different path.
- **Trigger Conditions**:
  - (a) A payload is classified `StaleNewerPending` or `StaleNoPending` while the player keeps moving during an exterior fill. A fast-moving player, a cold fill or a fast-travel backlog produces several in a row.
  - (b) A worldspace drain (door walk, exterior→exterior transition, save or debug load) cancels an `active_apply` that is mid-`FinishImports`.
- **Verification Path**: `cargo test`-level for the structure (a stale payload carrying N entries finishes all N before the next `should_yield` check). The wall-clock cost needs a streaming run: read `worker_stale_salvaged_imports` (already in the telemetry line, #5512) beside `apply_max_ms` on a fast-travel or sprint-through-boundary session.
- **Description**: #5512 correctly makes the worker's skip filters true: a key the worker skipped is now resident in `NifImportRegistry`. It does that by calling `salvage_stale_imports` at three drop sites, and each call runs `finish_streaming_import` for every entry of the payload in a plain `for` loop:

  ```rust
  // streaming_helpers.rs:886 — inside `loop { let Ok(payload) = state.payload_rx.try_recv() … }`
  let salvaged = salvage_stale_imports(world, &mut state.mat_provider, &|p| state.tex_provider.has_texture(p),
                                       &mut state.telemetry, payload.parsed);
  for _ in 0..salvaged { budget.complete_unit(); }   // charged AFTER the work
  ```

  Three properties follow:
  1. **No yield inside the loop.** The normal path takes one import per outer iteration and tests `budget.should_yield()` between units (`FinishImports(mut imports)`, `:922`). The salvage runs the whole `payload.parsed` vector first, and `complete_unit()` runs only afterwards, so the budget cannot stop it.
  2. **Several stale payloads run back to back.** The surrounding receive `loop` has no `should_yield` check on the stale arm. It keeps pulling and salvaging until it finds an `Apply` payload or the channel is empty.
  3. **Each entry is not cheap.** `finish_partial_import` re-interns the worker's strings, runs `merge_external_materials` (BGSM/BGEM reads from the archive pool plus the dead-path probe through `tex_provider`), converts an embedded `NiControllerSequence` clip, and builds the `CachedNifImport`. That is the work the normal path's per-unit budget exists to slice.

  The drain path (b) pays the same cost for no benefit it can use: `cancel_streaming_apply_job` salvages the active job's remaining `FinishImports` iterator inside a transition that is already tearing the world down.
- **Impact**: A frame hitch whose length scales with (stale payloads in the channel) × (new unique models per payload) × (per-import merge cost). That is the same class of main-thread stall the worker exists to prevent. It lands on exactly the cold-fill and fast-travel frames that #5512 targeted. The old behaviour was a single main-thread parse fallback on the next request that needed a skipped key, which is cheaper per event but unbounded in a different way; the salvage trades a rare small hitch for a rarer large one.
- **Related**: #5512 (the fix this follows from), #3670 (the batch memo), #5491, CONC-D7-2026-10-09-01.
- **Suggested Fix**: Salvage through the same budgeted machinery as a current apply. For example, push the stale payload's `parsed` iterator onto a small `VecDeque` of pending salvage jobs on `WorldStreamingState`, and advance one import per `advance_streaming_apply` iteration under `should_yield()`, calling `budget.complete_unit()` per import. For the drain path, skip the salvage entirely, or let the registry keep nothing from a cancelled job: the state, and with it the worker's memo, is being discarded, so the residency premise no longer applies.

### CONC-D5-2026-10-12-01: `contact_force_rx`'s comment names a rayon-worker invocation the build cannot produce
- **Severity**: LOW (comment accuracy)
- **Dimension**: RwLock Patterns / physics step (Dim 5)
- **Location**: `crates/physics/src/world/mod.rs:291-294`
- **Status**: NEW
- **Description**: The field is `std::sync::Mutex<mpsc::Receiver<ContactForceEvent>>`, with this note: *"the narrow phase invokes the event handler from rayon workers, so `PhysicsWorld` must stay `Sync`"*. `cargo tree -p byroredux-physics -e features -i rapier3d` shows rapier 0.36.1 built with `alloc`, `std`, `dim3`, `f32`, `default` and no `parallel`. Rapier's narrow phase therefore does not fan out to rayon in this build, and the handler runs on the calling thread inside `physics_sync_system`.
- **Evidence**:
  ```
  rapier3d v0.36.1
  ├── rapier3d feature "alloc" / "std" / "default" / "dim3" / "f32"      (no "parallel")
  ```
- **Impact**: None on behaviour. The `Mutex` is still required, because `Receiver` is `!Sync` and `PhysicsWorld` is an ECS resource. A reader who trusts the comment concludes the physics step is multi-threaded and may add synchronization that is not needed, or skip a check that would matter if `parallel` is ever enabled.
- **Related**: #5542, #5549, #5554 (the impact-audio chain on the same channel).
- **Suggested Fix**: Reword the note to "`Receiver` is `!Sync` and `PhysicsWorld` is a `Sync` resource; with rapier's `parallel` feature the handler would also be called from rayon workers." Leave the `Mutex`.

---

## Existing issues that overlap this audit (not re-reported)

- **#5641** (open, SCR-D5-2026-10-11-04): `player_controller_system`'s Access row omits the KILL producer's acquisitions. I confirmed it independently from the concurrency side, and I did not file it again.
  - `character.rs:1589` (`apply_player_drowning_damage`) calls `story_events::raise_kill_story_event`, which reads `LoadedCellIndex`, `CurrentCellContext` and `CurrentExteriorContext` and writes `StoryEvent`.
  - `player_controller_system` is a *parallel* Stage::Early system (`boot/schedule/early.rs`). Its Access row declares none of the four, and `PARALLEL_SYSTEMS` has no `(STORY_EVENTS_SRC, "raise_kill_story_event")` hop. The scan is blind to the hop because it follows calls only inside a listed file.
  - No other Early parallel system touches those types today (the only Early partner is `timer_tick_system`), so the exposure is latent: `known_conflict_count() == 0` is unsound for those four types until the row is fixed.
  - A second producer of the same call, `water_damage_system`, is an exclusive and so never paired; its row is advisory only (`sys.accesses`).
- **#5666** (open): the `sf-cdb-index` prefetch takes `PREFETCH_GATE` before creating its slot, so a lookup for a still-queued CDB builds on the main thread. Confirmed: lookups walk sources last-registered-first, so DLC indexes are reached before the base. Not re-reported.
- **#5665** (open): the single-flight slot does not memoise a *panicking* build and the worker has no `catch_unwind`. Not re-reported.
- **#5663** (open): `open_shared`'s `(length, mtime)` stamp misses a same-length, mtime-preserving rewrite. Not re-reported.
- **#5647** (open): `StoryEvent` is a one-slot-per-entity marker with several producers. Not re-reported; the new `water_damage_system` and extension-batch KILL producers share the exposure.
- **#5365** (open): the all-slots fence wait; unchanged, not re-filed.
- **#5442** (open): the `PartialNifImport: Send` compile-time assertion; still absent, not re-reported.
- **#5069**: closed by `9355fcb5c` (the `equipment_appearance` row now declares its mid-life import surface).

## Dimension notes

- **D1 — Queue & AS sync (unchanged).** Commits on the Paths: `334458b3f`, `f4c2b9931`, `3efd17e5b` (source-scan routing, comment sweeps, whitespace). No `queue_submit`, `queue_present`, fence wait or barrier was added anywhere in the repo diff (grepped). The only sync-adjacent new call is the LOD-water rebuild's `upload_scene_meshes_batched` (#5289), the existing batched one-fence path that `mesh_instance.rs` already uses. Guards ok: `frames_in_flight_contract_names_every_dependent_resource`, `render_finished_is_sized_and_indexed_per_swapchain_image`, `queue_guard_released_before_one_time_fence_wait`, `one_time_commands_free_cmd_buffer_on_every_error_path`, `wait_failure_arm_disposes_only_on_device_loss`, `rgba_updates_use_the_frame_command_buffer_and_keep_descriptors`, `only_the_submitted_recording_consumes_its_pixels`, `fresh_build_records_peak_unconditionally_of_scratch_regrow`.
- **D2 — Compute chains (unchanged).** `fff5ed9a9` (#5620) moves workgroup widths, SH count and dispatch grids into the generated header; its message states all 36 shaders recompile byte-identical to the committed SPIR-V. `6435ee68a` (#5619) takes the volumetrics lighting descriptors' cluster buffer sizes from `ClusterCullPipeline`. The rest is ReSTIR test fixtures and rig-key ordering (CPU). Guards ok: `skin_publish_barrier_consumer_tests`, `taa_resolves_the_post_bloom_scene_tap`, `every_skipped_frame_drops_the_temporal_history_not_just_the_first`.
- **D3 — Lock ordering.** New code read:
  - `eat_sleep` (#5527/#5535): one gather block `EatSleepState` → `Transform` → `GlobalTransform` → `WalkSpeed`, all dropped before the writes; the `Transform` write comes after the KCC step.
  - `cached_walk_step` (#5452): `NavmeshTile` and `NavPath` reads in one scope, then a `PhysicsWorld` read in its own scope, then a `NavPath` write.
  - `write_back_walk` (#5580): separate single-type write scopes.
  - `footstep_system` (#5556): the `FootstepConfig` table is cloned before the `AudioWorld` write guard.
  - `impact_audio_system` (new, exclusive): snapshots `ImpactConfig`, takes the `PhysicsWorld` write for the drain, drops it, then takes `AudioWorld`.
  - `stop_line_voice`, `raise_kill_story_event` (resolves the location before the `StoryEvent` write, the #5372 shape) and the story-manager walk (#5628 scoped the `StoryManagerRng` guard to the draw) were checked and are consistent.
  - The extension commit path (`commit_actor_value_deaths`) drops the `Dead` and `ActorValues` guards before it queues reconciliation and raises KILL.
  - Lanes: see the table; the CI ABBA and lavapipe jobs are green.
- **D4 — Scheduler proof.** All proof tests are live and none are ignored. The one gap is #5641 above. New single-writer/multi-reader contracts checked: `PendingDeathReconciliations` (producers Early, Late, extension commit; consumer is a Late exclusive after `water_damage_system`, the order documented in `late.rs`), `WaterContact` (writers Early/Physics, readers Late), `PhysicsWorld::pending_force_events` (writer Physics stage, consumer Late exclusive), `StoryEvent` KILL (raised in Late, drained by the next frame's Update dispatch; `event_cleanup` does not own it). All have writer stage ≤ reader stage.
- **D5 — Physics / teardown.** `sync.rs` kept its phase shape; the numbered phases and `register_newcomers_parallel_section_holds_no_world_guard` hold. The rapier 0.36 bump, the Havok contact-combine rule, per-part surface materials and the re-sleep of spawned-asleep clutter (#5544) are single-threaded changes inside `PhysicsWorld`. The #5490 purge fix and the `collect_subtree` helper (#5513: a per-node `world.get::<Children>`, a visited set and a traversal budget; no guard held between nodes) are verified above.
- **D6 — Resource lifecycle (unchanged).** Commits: egui 0.36 (`drop_without_applying_deltas` at three discard sites, `set_texture` / `free_texture` per id; the old atlas image is destroyed inside `set_texture`, covered by the all-slots fence wait, sync.rs rider 14), `MAX_SKIN_SLOTS` (#5583) and shader math. Observation, not a finding: `EguiPass::dispatch` takes `textures_free` up front, so an early `?` on an upload error drops that frame's frees (a leak on the error path only; the same shape existed before).
- **D7 — Workers.** Beyond the finding:
  - First-step grep over `thread::spawn|Builder|rayon::|mpsc::`: the only new production hit is `asset_provider/material/cdb.rs` (the `sf-cdb-index` thread).
  - `shutdown` / `recv_next_live_request` handshake re-read: the `cancel` store is `Release`, the worker's load is `Acquire`, and a cancelled request is discarded unparsed with the batch memo cleared.
  - `recv_next_batch_request`'s empty-queue clear of `batch_keys` is unchanged.
  - Debug server: only the default-name uniqueness change (#5147) and a loopback pin test (#5149). Ruffle `Player`: callback registry consult only. No new thread in either.
  - `build_render_data` and `cell_loader/load_order.rs`: no commits.

## Skill drift (fold into the next `/audit-concurrency` sync)

- **Dim 4** should name the `raise_kill_story_event` hop out of `character.rs` once #5641 lands (a `PARALLEL_SYSTEMS` row beside `queue_dead_actor_reconciliation`).
- **Dim 5 first step** still lacks the non-canonical despawn sweep (`grep -rn 'despawn_batch(\|\.despawn(' byroredux/src/cell_loader byroredux/src/systems`) the baseline asked for; this run's result is clean.
- **Dim 5 first step** should also keep the PhysicsWorld-sink sweep as a standing line (`grep -rn 'PhysicsWorld' byroredux/src --include='*.rs'`, require every guard scoped with nothing acquired under it).
- **Dim 7** should mention `salvage_stale_imports` and the three sites where a stale payload's imports are finished, once CONC-D7-2026-10-12-01 is decided.
- **Dim 3** should note that "Test + Check + Clippy" is red on the clippy step only (physics `map(f)` unit, renderer doc-list indentation) while the ABBA lane is green. Read which step failed before treating CI red as a lock regression.

---

Publish with: `/audit-publish docs/audits/AUDIT_CONCURRENCY_2026-10-12.md`. Domain labels:
- CONC-D7-2026-10-12-01: **concurrency** + `performance` (+ `terrain-exterior`).
- CONC-D5-2026-10-12-01: **concurrency** + `physics` + `doc-rot`.
