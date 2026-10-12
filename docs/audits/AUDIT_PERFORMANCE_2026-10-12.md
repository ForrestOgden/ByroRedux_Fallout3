**HEAD**: `5d31fd637` · **Baseline**: `docs/audits/AUDIT_PERFORMANCE_2026-10-09.md` (HEAD `3bcf6c8e8`) · **Audited**: Dim 1 and Dim 7 (delta read of the changed code, plus verification that every baseline finding landed), Dim 3 (delta read plus guard runs), Dims 2, 4, 5, 6, 8 (delta read and guard runs). The delta is 264 commits in `3bcf6c8e8..HEAD`. · **Unchanged since baseline (skimmed)**: Dim 2 (`byroredux/src/render/` has only water, groundcover and camera-helper commits; `draw.rs` has none), Dim 4 (`scene_buffer/` has constants and docs only), Dim 6 (zero functional commits), Dim 8 (`gpu_timers.rs`, `context/telemetry.rs` and `render/camera.rs` have no functional change)

# Performance Audit — 2026-10-12

**Command**: `/audit-performance`, `--depth deep`. This is one leg of `/audit-suite --preset comprehensive`, run solo (no sub-agents).

**Mode**: static analysis plus unit-test, dhat, shader-parity and harness-provenance runs. Per the suite rules no engine, GPU process or benchmark was launched. **Every cost statement below is either derived from code or quoted from a dated earlier measurement. None was measured in this run.**

## Executive Summary

| Severity | NEW | Already tracked (verified still present) |
|---|---:|---:|
| CRITICAL | 0 | 0 |
| HIGH | 0 | 0 |
| MEDIUM | 0 | 0 |
| LOW | 4 | 1 (#5365) |

**No guard has eroded.** Every guard test that was run passed (see "Guard verification").

**Every finding in the baseline report landed.** All of #5401, #5447–#5451, #5275's residual (#5528), #5339, #5288, #5289 and #5291 are CLOSED, and the fix was read in the code, not only trusted from the commit title:

- **#5447** (`7bba019e1`): `story_change_location_system` returns before any key work when the `SmTree` has no nodes. The key is a hash of typed fields, with no `format!`.
- **#5448** (`9cc748ef2`): `BYRO_M42_DEBUG` is read once through a `OnceLock` (`ai_package.rs:789`).
- **#5449 / #5527** (`35a009431`, `959b76f55`): `eat_sleep` keeps `SEAT_RETRY_SECS` and `ANCHOR_RETRY_SECS` clocks, a `FrameSeats` shared marker gather, and one pass per storage for the walk leg.
- **#5450** (`eaac75911`): the script-killed corpse scan returns early unless the game is FO3/FNV (`reference_state.rs`).
- **#5528** (`e588e5e67`): `GrowthBackoff` in `dynamic_rgba.rs`, with the gate before the allocation.
- **#5529** (`2fd7ee270`): the climate override is evaluated once per grid cell (`claim_climate_evaluation`).
- **#5288 / #5291**: `Archive::open_shared` and the background `sf-cdb-index` thread. The residual race on `PREFETCH_GATE` is open as #5666 (see "Related, not filed").

The only open item from the baseline is **#5365** (the top-of-frame all-slots fence wait). It is a bench-gated throughput question and no commit touched it.

**New this run: four LOW findings.** None is a regression of a landed guard.

- **PERF-D7-2026-10-12-01**: since #5334, a distant-water mesh rebuild runs per applied cell instead of once per crossing. Each rebuild takes a blocking graphics-queue fence wait on the main thread. This partly undoes what #5289 removed.
- **PERF-D1-2026-10-12-01**: `eat_sleep_system` builds a fresh actor `Vec` every frame, including already-seated sleepers. It also runs three O(n·m) `retain` sweeps over std `HashMap`s.
- **PERF-D1-2026-10-12-02**: the physics water scan (#5561, #5540) keeps its per-wet-body latches in std SipHash `HashMap`/`HashSet`. That is the keyspace the hot-path hashing rule forbids.
- **PERF-D1-2026-10-12-03**: `impact_audio_system` clones its per-material `BTreeMap` on every frame, before its empty-check. Separately, `CONTACT_FORCE_EVENTS` is armed on every dynamic collider while the only consumer is a shipped no-op.

**Observed vs ROADMAP.** Nothing was measured, so this report claims no bench delta.

- The Bench-of-record is still `a37fcba3c`, now **800 commits** behind HEAD (ROADMAP's own close line says 616 at its last refresh, R6a-stale-27).
- `scripts/check-bench-harness-provenance.sh a37fcba3c` still reports **DIVERGED** with the same two commits (`a4ede5fa0`, `8fdd9ae56`). Each carries its recorded disposition (#5451, closed).
- No HEAD frame-time claim is current until the record is re-run with a same-machine control.
- Frame-path changes since the last record that the refresh must cover, beyond the ones the baseline listed:
  - #5565 (`e3dfd3a31`) and #5455 (`6d64634aa`): the transmission shadow walk now finishes every lit exit with a glass-only trace.
  - `4d991bad7`: the shadow-ray disk samples use a dithered R2 sequence.
  - `48bed0504`: the parked direct-light EMA is deepened to cap 1024 / floor 0.002.
  - #5507 (`a851c625d`): the ReSTIR rig key is order-independent.
  - #5506 (`38f711a93`): the contrast-toe fix in `presentation.frag`.

## Hot Path Analysis

### Per-pass GPU cost

There are no new timings. The newest in-repo per-pass numbers are `AUDIT_PERFORMANCE_2026-09-26b.md`: HEAD `b7491072f`, MedTekResearch01, renderer-stepped/pan, Native AA 1280×720, **RT quality tier pinned to 0**. Only the deltas since the baseline are listed. The bracket inventory is unchanged: 28 `_ms` fields (23 named + 5 main-render sub-phases) in `QUERIES_PER_FRAME` = 56 slots (`gpu_timers.rs:131`). No pass was added or removed.

| Bracket (`GpuTimerSnapshot`) | Code change since the 10-09 baseline | Cost direction (derived, unmeasured) |
|---|---|---|
| `main_render_ms` (inclusive) | **#5565 / #5455**: `traceShadowTransmittanceSkippingInstance` now ends every lit exit (`no more hits`, `advance failed`, hop budget exhausted) in `traceShadowGlassLegRemaining`. That is one extra glass-only ray-query leg, taken only when the light's mask carries `VISIBILITY_LAYER_GLASS` and `remaining > 0`. | Up, bounded. It touches only the transmission-lobe population that #5401 measured. That A/B moved `gpu_main_render` by at most ~3 % with the whole trace gated off, so the new leg is a fraction of that ceiling. Not re-measured. |
| `main_render_ms` | `4d991bad7`: `ditheredSequence2` replaces a PCG hash and two IGN taps per shadow ray. The call is a handful of integer multiplies plus two `ignMask` evaluations. | Flat. |
| `main_render_ms` | `48bed0504` / #5617: parked cap and floor are named constants (cap 1024, floor 0.002). History length stays integer-exact in the half-float pack (≤ 2048). | Flat in GPU time. Longer parked convergence tail only. |
| `presentation_ms` | #5506: the toe weight reads `stretched` instead of `toe`. Same operation count. | Flat. |
| `water_*` (inside `main_render_ms`) | #5582: `water.vert` wave constants move to the generated header. Same arithmetic. | Flat. |
| `exposure_meter_ms`, sky SH | #5620: workgroup width and sample grid become named constants. Same value (64). | Flat. |
| every other bracket | none | none |

Notes:

- The adaptive ray budget still governs lighting cost. A/B any `main_render_ms` delta with `--rt-test-ray-quality-tier N`.
- A frame-time movement after `e3dfd3a31` on SOFT/BACK-lit or translucent-heavy scenes is expected to be small. Compare same-pose captures.

### Per-frame CPU (derived from code)

| Item | Cost shape | Change since baseline |
|---|---|---|
| `eat_sleep_system` (`systems/eat_sleep.rs:181`) | Fresh `Vec<EatSleepActor>` (~100 B per row) every frame. A seated sleeper keeps `SleepBehavior` all night and is pushed, then dropped by the `Seated` retain. Three `retain(any())` sweeps are O(actors²) while the retry maps are non-empty. Per-actor storage reads are now one gather pass (#5527). | Down overall (#5449 / #5527). Residual in **D1-01**. |
| `cached_walk_step` (`navmesh_path.rs:742`, used by force-greet and Eat/Sleep walks since #5452) | Per walking actor per tick: one `navmesh_residency_generation`, two storage reads, one `VecDeque` clone on a cache hit (`resolve_cached_waypoints:707`), one `PhysicsWorld` read, one `NavPath` write. | New callers, same shape as Travel/Follow. The cache-hit clone is the residue of #3269, see "Checked and not filed". |
| `write_back_walk` (`navmesh_path.rs:820`, #5580) | The six walkers now share the Transform write and the take-not-clone `NavPath` write (#4189). The scratch is persistent. | Neutral. Unifies the guard. |
| `footstep_system` (`systems/audio.rs`) | On a stride trigger only: one `RapierHandles` read at gate resolve, one `surface_material_under` ray per trigger, one `BTreeMap` clone of `Arc<Sound>`s. | New, event-driven. |
| `impact_audio_system` (`systems/audio.rs`) | **Every frame**: reads `ImpactConfig`, clones `surface_sounds` (a `BTreeMap`) and `default_sound`, then early-returns when there is nothing to play. | New. The map is empty today. **D1-03**. |
| `physics_sync_system` water scan (`crates/physics/src/water.rs`) | Per wet dynamic body per frame: a `rest_depth` SipHash `remove` or `entry`, a `water_force_targets` SipHash `insert`, and an AABB submerge sample for the lift. A trailing `retain(any())` runs while any sleeping float is latched. | New since #5561 / #5540. **D1-02**. |
| `refresh_lod_water` (`streaming/mod.rs:633`) | Two comparisons when idle. During a crossing's apply window: a ring re-selection over the worldspace projection plus a fence-waited upload at most every 250 ms. | New since #5334. **D7-01**. |
| `unload_cell_inner` (`unload.rs:386`) | #5483 adds, per cell victim, one SipHash `contains` against the player-owned set, plus one body-subtree walk per `unload_cell(s)` call. | New, per crossing. Not filed (load-time class). |
| `retry_cinematic_readoption` (`cinematic.rs:602`) | Early-outs on an empty queue. While entities are pending it runs `collect_subtree(player)`, one `Children` lock round-trip per body node plus a std `HashSet`. | Slightly dearer shape since #5513 (the shared collector). Pending is transient. Not filed. |
| Light attach (`spawn/mesh_instance.rs`, `attach_light_offset`) | One extra entity (`Transform` + `GlobalTransform` + `Parent`) per light-bearing mesh with an attach offset. | Spawn-time only. |

## Findings

### Eroded guards

None.

### New issues

### PERF-D7-2026-10-12-01: Distant-water rebuilds follow residency, so one crossing now costs several fence-waited uploads on the main thread
- **Severity**: LOW
- **Dimension**: Streaming & Cells
- **Location**:
  - `byroredux/src/streaming/mod.rs:633-663` (`refresh_lod_water`, `LOD_WATER_REBUILD_INTERVAL` = 250 ms at `:624`)
  - `byroredux/src/cell_loader/water.rs:1203-1296` (`rebuild_lod_water_mesh`)
  - caller `byroredux/src/app_step.rs:271-279`; flag set at `streaming_helpers.rs:999,1096` and `app_step.rs:176`
- **Status**: NEW. Related: #5289 (closed) removed the per-crossing blocking upload; #5334 (closed) moved the trigger from "grid crossing" to "residency change".
- **Description**:
  - Before #5334 the distant-water mesh was rebuilt once per grid crossing, with one fence wait (#5289).
  - Now `refresh_lod_water` ORs `loaded_residency_changed` into `lod_water_dirty` every step, and every applied or unloaded cell sets that flag.
  - The only throttle is a 250 ms minimum spacing, which the commit says "coalesces" the per-cell rebuilds.
  - Each rebuild re-selects the ring (O(worldspace wet cells), via `plane.projection`). It then calls `upload_scene_meshes_batched` on `ctx.graphics_queue` with `transfer_fence`, and the main thread blocks until the fence signals.
  - The new mesh takes a permanent `meshes` slot (slots are append-only) and the old one leaves a hole (`geometry_has_holes`, `geometry_dirty`).
  - A crossing applies its ring one cell at a time over roughly 1–3 s (`STREAMING_APPLY_BUDGET` 16 ms per frame). That gives several rebuilds, each with a blocking submit, per crossing.
  - Worldspace entry and boot are unbudgeted by design, but they follow the same 250 ms cadence for the whole initial apply.
- **Evidence**:
  ```rust
  self.lod_water_dirty |= residency_changed;
  ...
  if plane.last_rebuilt.is_some_and(|at| at.elapsed() < Self::LOD_WATER_REBUILD_INTERVAL) { return; }
  self.lod_water_dirty = false;
  crate::cell_loader::rebuild_lod_water_mesh(world, ctx, plane, game, player_grid, |g| loaded.contains_key(&g));
  ```
  `rebuild_lod_water_mesh` builds a `GpuUploadCtx { queue: &ctx.graphics_queue, … }` and passes `transfer_fence` to `upload_scene_meshes_batched`.
- **Impact**:
  - Main-thread stall per rebuild equals the time for the GPU to retire the transfer submit behind queued frame work. That is bounded by about one GPU frame, which the 09-26b record puts at tens of ms in heavy RT scenes.
  - It lands on the frames that already carry the apply slice, which `FrameTimeBudget` cannot preempt.
  - Derived. Not measured. A `water.dump` plus the `cpu_ms:` `atw_post` bucket on a Commonwealth or Tamriel crossing would show it.
- **Related**: #5289, #5334, #5339 (closed, retry-per-frame fixed), `streaming/mod.rs:610` (the doc comment already names the multiplication risk).
- **Suggested Fix**: Rebuild on residency *settle* (the pending-apply set empty, or no change for N frames) with the 250 ms floor kept as a ceiling, so a crossing pays one rebuild. Alternatively, move the upload to the stream pool's transfer path. Measure before changing: record `UnloadPhaseTimings` plus a rebuild-count telemetry counter.

### PERF-D1-2026-10-12-01: `eat_sleep_system` rebuilds its actor list every frame, including seated sleepers, and sweeps three maps O(n·m)
- **Severity**: LOW
- **Dimension**: CPU Hot Paths
- **Location**: `byroredux/src/systems/eat_sleep.rs:181-249`
- **Status**: NEW. Related: #5449 and #5527 (closed) fixed the furniture gather and per-actor probes. This is the residue they left.
- **Description**:
  - `actors` is a fresh `Vec<EatSleepActor>` (`:183`) filled from every `EatBehavior` / `SleepBehavior` row.
  - Behaviour components are removed only at package change (`ai_package.rs:704`). So a seated diner or sleeper stays in the list for the whole package window, which for `SleepBehavior` is hours of game time.
  - Each frame the vec is rebuilt, regrown and pruned by the `Seated` retain (`:233`). Then three `retain(|npc, _| actors.iter().any(…))` sweeps run (`:235-248`) whenever `seat_retry`, `anchor_retry` or `stuck_secs` is non-empty. Those are O(retry entries × actors).
  - The maps are std `HashMap<EntityId, f32>`.
  - Persistent scratch is the project's own rule for these systems, and `EatSleepScratch` already holds the maps beside it.
- **Evidence**: `let mut actors: Vec<EatSleepActor> = Vec::new();` at `eat_sleep.rs:183`; the `retain(…any…)` triples at `:233-248`.
- **Impact**:
  - Tens of allocations per second plus a few hundred comparisons per frame on a populated FNV saloon or Skyrim inn.
  - Microseconds, so LOW. It matters only as the last per-frame allocation in a system whose siblings were already converted.
  - Nothing quantitative guards it: per-frame ECS paths have no dhat coverage.
- **Related**: #5449, #5527, #5448.
- **Suggested Fix**: Move `actors` into `EatSleepScratch` (`clear()` plus refill), and drop seated actors at the gather so they never enter the list (probe `Seated` while filling). Key the retry-map pruning off a generation or the gather's own set instead of `any()`.

### PERF-D1-2026-10-12-02: The physics water scan keeps its per-wet-body latches in std SipHash maps
- **Severity**: LOW
- **Dimension**: CPU Hot Paths
- **Location**:
  - `crates/physics/src/water.rs:38,153` (`WaterContactScratch::rest_depth: HashMap<EntityId, f32>`)
  - `crates/physics/src/water.rs:1102-1112,1241,1255,1326-1328`
  - `crates/physics/src/world/mod.rs:260,479,889,909,1276` (`water_force_targets: std::collections::HashSet<RigidBodyHandle>`)
- **Status**: NEW. Related: #5561 and #5540 (closed) introduced both.
- **Description**:
  - `physics_sync_system`'s buoyancy scan runs every frame over every wet dynamic body.
  - For each one it does a SipHash `remove`/`entry` on `rest_depth` (dry arms, awake arms and sleeping arms each touch it) and a SipHash `insert` into `water_force_targets` via `note_water_force_target`.
  - `apply_multibody_link_damping` then does a SipHash `contains` per awake dynamic body while iterating *all* bodies.
  - The scan ends with `rest_depth.retain(|e, _| targets.iter().any(…))`, O(latched × targets).
  - This is a per-frame, per-entity keyspace. The hot-path hashing rule (#2923) says std `HashMap`/`HashSet` there is a fresh regression. `crates/physics/Cargo.toml` does not depend on `rustc-hash`.
- **Evidence**: `rest_depth: HashMap<EntityId, f32>`, `pw.note_water_force_target(t.handles.body)` per target, `rest_depth.retain(|entity, _| targets.iter().any(|t| t.entity == *entity))`.
- **Impact**:
  - ~10–20 ns per SipHash op. A pond with tens of floats and ragdoll bones is on the order of microseconds per frame, so LOW. It matters only because the project's own rule treats the pattern as a regression.
  - Ragdoll-heavy wet scenes (FNV Lake Mead, Skyrim rivers with many corpses) scale linearly with bodies.
- **Related**: #2923, `crates/core/src/ecs/resources/skin_slot_pool.rs` (`skin_slot_pool_maps_are_not_siphash` is the model guard).
- **Suggested Fix**: Use `FxHashMap` / `FxHashSet` (add `rustc-hash` to the physics crate). For `rest_depth`, prune by comparing a scan generation stamp instead of `any()`. Add a source-scan guard like the `skin_slot_pool` one.

### PERF-D1-2026-10-12-03: `impact_audio_system` clones its sound table every frame, and impact events are armed with no consumer
- **Severity**: LOW
- **Dimension**: CPU Hot Paths
- **Location**:
  - `byroredux/src/systems/audio.rs` (`impact_audio_system`, the `ImpactConfig` snapshot before the `default.is_none() && surface_sounds.is_empty()` early return)
  - `crates/physics/src/sync.rs:1155-1161` (`ActiveEvents::CONTACT_FORCE_EVENTS` for every dynamic collider)
  - `crates/physics/src/world/mod.rs:1369-1377` (per-substep channel drain)
- **Status**: NEW. Wave-4 clutter-impact feature, no issue.
- **Description**:
  - **(a)** The system clones `config.surface_sounds` (a `BTreeMap<u32, Arc<Sound>>`) and `default_sound` every frame to satisfy a lock-order rule, then checks emptiness *after* the clone. Today the map is empty (IPDS/IPCT decode has not landed), so the clone is free. Once content populates it, the clone allocates O(entries) nodes every frame for a system that returns at once when no impact fired.
  - **(b)** `register_newcomers` arms `CONTACT_FORCE_EVENTS` on every dynamic collider with rapier's 0.0 threshold. Every awake resting contact pair then pushes an event each substep, and `run_substep` drains and filters the channel each substep behind a `Mutex`. All of it is discarded, because the consumer returns early while `ImpactConfig` has no default sound and no entries (the shipped state).
- **Evidence**: `world.try_resource::<ImpactConfig>().map(|config| (config.surface_sounds.clone(), config.default_sound.clone(), …))` precedes the `if default.is_none() && surface_sounds.is_empty() { return; }` check. `ActiveEvents::CONTACT_FORCE_EVENTS` is set whenever `motion_type == Dynamic`.
- **Impact**:
  - Today only (b), and it is small: rapier already computes the contact forces, and the channel send is cheap.
  - Latent (a) once a table ships.
  - Not measured. A `phys.stats` plus `cpu_ms:` A/B with `ImpactConfig` unregistered would size (b).
- **Related**: #5549, #5554, #5556 (closed, same function), #5348 / #5347 (open, footstep emitter docs).
- **Suggested Fix**:
  - (a) Hold the table as an `Arc<BTreeMap<…>>` (clone is a refcount bump), or check a cheap `has_content` flag first.
  - (b) Arm `CONTACT_FORCE_EVENTS` only while `ImpactConfig` carries content, or gate the drain on the same flag.

## Related, not filed (existing)

| Issue | What | Status at HEAD |
|---|---|---|
| #5365 | Top-of-frame all-slots fence wait | OPEN. `app_frame.rs` has only the egui commits since the baseline. Not re-measured. |
| #5666 | `PREFETCH_GATE` taken before the slot exists, so a lookup for a still-queued CDB builds on the main thread | OPEN, Existing. This is the residual of #5291 (`cdb.rs:136,202`: the slot is created inside `cdb_material_index`, which the prefetch thread calls only after taking the gate). |
| #5663 / #5664 | `open_shared` stamp misses a same-length same-mtime rewrite; reused handles skip the #3637 shadow count | OPEN, Existing. Correctness, not cost. |
| #5644 / #5643 | `fold_info_into_topic` is O(N×M); the Story Manager tree build is O(g³) | OPEN, Existing (SCR-D5-2026-10-11-07 / -06). Boot-time cost. The baseline's "checked, not filed" DIAL fold is now tracked. |
| #5348 | Footstep emitter suppression under `PlayerEntity` | OPEN, Existing. Not performance. |

**Suite overlap** (re-read of `/tmp/audit/REPORTED_THIS_SUITE.md` before finalizing): the renderer audit's three LOW doc findings (REN-D2-2026-10-12-01 stale `traceShadowGlassLegRemaining` docblock, REN-D3-2026-10-12-01 bind-inverse cap 1366→1364 in `shader-pipeline.md` / `memory-budget.md`, REN-D6-2026-10-12-01 misplaced doc comment in `merge.rs`) are not performance findings and are not repeated here. Existing: renderer report REN-D3-2026-10-12-01 covers the doc side of #5583. None of the four findings above overlap a reported item.

## Guard verification

| Guard | Result |
|---|---|
| Renderer lib, filters `acceleration static_blas_recovery scene_buffer skin_compute dispatch_skin gpu_timers material_tests hash_ bone_world has_any_view_of_path early_fragment light_history group_state tlas_scratch restir_history caustic light_rig growth_ staging` | **487 passed**. Includes `light_rig_geometry_key_is_independent_of_light_order` (#5507) and the `restir_direct_light_ema_constants_come_from_the_generated_header` pin. |
| Renderer lib, filters `skinned_blas pose_dirty plan_palette a_reused_plan blas_budget dynamic_rgba shader_contract tonemap exposure` | **190 passed**. Includes `growth_retries_back_off_during_a_failure_episode` and `growth_gate_sits_before_the_allocation` (#5528). |
| Core (`--features inspect`), filters `drain_dirty skin_slot_pool` | **29 passed**. Includes `drain_dirty_into_preserves_storage_capacity` and `skin_slot_pool_maps_are_not_siphash`. |
| Bin (toolchain 1.96.0), filters `draw_sort_key static_blas_recovery skin_dispatch_ran bench_gpu_keys atw_bracket sort_key eat_sleep forcegreet story_events detach_victims climate cinematic walk_movers` | **134 passed, 2 ignored** (the manual benches). Includes `walk_movers_share_one_pass_two_write_back`, `detach_victims_takes_one_parent_guard_for_the_whole_sweep` and the #5527 / #5449 Eat/Sleep tests. |
| Physics lib, filters `water contact_force surface_material` | **43 passed** (waterline, KCC-damped crest, contact-force `started` drain, `surface_material_under`). |
| NIF dhat (`--features dhat-heap`, `heap_allocation_bounds`) | 7 passed. |
| `scripts/check-shader-artifacts.sh` | 36 shaders + the opaque early-test variant are byte-identical (glslang 11:16.4.0). |
| `scripts/check-bench-harness-provenance.sh a37fcba3c` | **DIVERGED**, 2 commits, each with its recorded disposition (#5451, closed). |

Symbol and constant checks (grep and read). All held:

- **Dim 1**: `make_eat_sleep_system` persistent scratch, `ANCHOR_RETRY_SECS`, `FrameSeats`; `story_change_location_system` empty-tree early-out; `ambient_ai_package_system`'s `OnceLock` read; billboard, animation, bounds and `SkinSlotPool` files have zero commits.
- **Dim 2**: `DRAW_SORT_PARALLEL_THRESHOLD` 3000 (`render/mod.rs:933`).
- **Dim 3**: `acceleration/`, `context/resources.rs`, `mesh/` and `deferred_destroy.rs` have zero functional commits (`blas_static.rs` is a 2-line source-scan edit). `Archive::open_shared` plus its length/mtime stamp and the `SHARED_ARCHIVE_MAX` = 1024 cap; `CdbIndexSlot` single-flight and the `sf-cdb-index` thread.
- **Dim 4**: `MAX_INSTANCES` 0x40000, `INITIAL_INSTANCE_CAPACITY` 0x10000, `MAX_SKIN_SLOTS` now the single source of the upload cap (cap 1366→1364, staging −18 KB, `scene_buffer/constants.rs:83,114`).
- **Dim 5**: `ENABLE_LEGACY_WRS` 0; `QUERIES_PER_FRAME` 56; `triangle.frag` / `triangle_early.frag` rebuilt-byte-identical.
- **Dim 6**: `plan_palette_dispatch`, `skinned_blas_stays_fx_hashed`, `pose_dirty_crosses_the_crate_boundary_without_siphash` and the refit threshold constants are unchanged (their files have no functional commits).
- **Dim 8**: `render/camera.rs` is only the `look_at_mat4` swap, so `assemble_camera` still adds one view build. `Camera::view_matrix` now uses `look_to_mat4` (#5566, correctness at world magnitude; one extra `normalize`). `metrics_sample_system` narrowed its sysinfo refresh to memory (still throttled).

### Checked and not filed (quantify first)

- **`resolve_cached_waypoints` cache-hit clone** (`navmesh_path.rs:707`): one `VecDeque<Vec3>` clone per walking actor per tick. #3269 halved it and #4189 removed the write-back clone. The new `cached_walk_step` callers (force-greet, Eat/Sleep) reuse it. A `take` from the `NavPath` would remove it. Per-actor cost is a small heap copy.
- **#5483 protected-set retain** (`unload.rs:410-425`): `victims.retain(|e| !protected.contains(e))` is one SipHash lookup per cell victim per unload. At roughly 3–6 k victims per cell and 7 cells per crossing, about 20–40 k lookups, so on the order of 0.5 ms on the main thread. `player_owned_entities` also walks the player-body subtree once per `unload_cell(s)` call. This is load-time class (not the per-frame rule), and it is booked under `UnloadPhaseTimings.ownership_index`. Check that bucket before filing. Victims are already sorted, so a binary search over the small protected set would be linear.
- **`retry_cinematic_readoption` player-subtree walk** (`cinematic.rs:640`): runs only while convoy entities are pending. The shared `collect_subtree` (`hierarchy.rs:50`) uses a std `HashSet` and one `world.get::<Children>` per node. Transient, same posture as the baseline.
- **NIF walker revisit guard (#5485)**: a `Vec<bool>` of `scene.blocks.len()` per walker per import (five of them). Bytes, not allocations of note. #5486's `BsplineSampling` budget (1 M keys) bounds hostile input and leaves vanilla clips unchanged.
- **`prune_same_level_overlaps` tie-break** (`lod_bands.rs`): the new `coverage` closure does O(quads × level²) `cell_exists` probes, but only for residue classes tied on count. The vanilla lattices are single-class. Recomputed per `reconcile_lod_rings` call as before.
- **Ground-cover union accept rate** (`render/groundcover.rs`): chunk-build time, not per frame. The accept-rate change may raise accepted candidates; compare `groundcover_scatter_ms` on a same-pose capture before the next refresh.
- **Light-attach entity** (`mesh_instance.rs`, `attach_light_offset`): one extra three-component entity per light-bearing placement, spawn time only.

## Prioritized Fix Order

1. **Bench-of-record refresh (R6a-stale-27)**: now 800 commits and the top measurement item. Nothing in Dim 5 can be confirmed or cleared until it is done. The refresh must cover the #5565 / #5455 glass leg, the dithered shadow sampling and the deeper parked tail.
2. **PERF-D1-2026-10-12-03 (b)** and **PERF-D1-2026-10-12-01**: quick wins, both gate-or-scratch changes with no behaviour risk.
3. **PERF-D1-2026-10-12-02**: add `rustc-hash` to the physics crate and convert the two maps. Add the `skin_slot_pool`-style guard.
4. **PERF-D7-2026-10-12-01**: measure first (count rebuilds and sum fence-wait ms across one crossing), then settle-gate the rebuild.
5. Open **#5365** after the refresh gives a same-pose control.

## Stale skill premises

- **Dim 1 Paths** omit three files that carry per-frame cost since the baseline: `byroredux/src/systems/{audio,water}.rs` (`footstep_system`, `impact_audio_system`, `water_damage_system`), `byroredux/src/systems/navmesh_path.rs` (`cached_walk_step`, `write_back_walk`), and the physics water scan (`crates/physics/src/water.rs`, `crates/physics/src/world/mod.rs`). All three D1 findings live there. Add a `crates/physics/` line to the Dim 1 hashing rule: `PhysicsWorld` / `WaterContactScratch` are per-frame, per-entity keyspaces.
- **Dim 1 Guard** lists #5449 and #5527 as fixed. Add `eat_sleep` `actors` Vec / retain sweeps (D1-01) as the known residue, and add `physics_sync_system` water scan as a per-wet-body hash site.
- **Dim 7 Paths** omit `byroredux/src/streaming/mod.rs` `refresh_lod_water` (D7-01), `byroredux/src/cell_loader/water.rs` `rebuild_lod_water_mesh` (the baseline noted `water.rs` as missing, still absent), and `byroredux/src/cell_loader/stream_snapshot.rs`. The Dim 7 checklist's #5289 bullet should say the rebuild now also fires on residency change (#5334), at a 250 ms floor.
- **Dim 3 Checklist** names #5528 (`e588e5e67`) correctly. The open PAR issues #5663 / #5664 / #5666 touch `open_shared` and the `PREFETCH_GATE` order. Add #5666 as a known-open residual under the #5291 sentence rather than stating #5291 as fully closed.
- **Bench-of-record staleness**: the skill and ROADMAP quote a stale commit count (ROADMAP: 616 at its last close; HEAD is 800). Re-derive with `git rev-list --count a37fcba3c..HEAD` rather than carrying a figure.
- **Dim 5**: the transmission-walk bullet says "plus the glass leg of the remaining segment since #5455/#5565" correctly. It should add that the new leg runs at *every* lit exit of the skip walk, so a worst-case A/B (large SOFT / BACK-lit share) is the open ask from #5401.

Next step: `/audit-publish docs/audits/AUDIT_PERFORMANCE_2026-10-12.md`
