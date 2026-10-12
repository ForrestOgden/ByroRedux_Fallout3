# PERF-D7-2026-10-12-01: Distant-water rebuilds follow residency, so one crossing now costs several fence-waited uploads on the main thread

- **GitHub**: #5705 (https://github.com/matiaszanolli/ByroRedux/issues/5705)
- **Labels**: low,performance,terrain-exterior,water,bug
- **Report**: docs/audits/AUDIT_PERFORMANCE_2026-10-12.md

**Severity**: LOW

## Location
- `byroredux/src/streaming/mod.rs:633-663` (`refresh_lod_water`, `LOD_WATER_REBUILD_INTERVAL` = 250 ms at `:624`)
- `byroredux/src/cell_loader/water.rs:1203-1296` (`rebuild_lod_water_mesh`)
- caller `byroredux/src/app_step.rs:271-279`; flag set at `streaming_helpers.rs:999,1096` and `app_step.rs:176`

## Description
- Before #5334 the distant-water mesh was rebuilt once per grid crossing, with one fence wait (#5289).
- Now `refresh_lod_water` ORs `loaded_residency_changed` into `lod_water_dirty` every step, and every applied or unloaded cell sets that flag.
- The only throttle is a 250 ms minimum spacing, which the commit says "coalesces" the per-cell rebuilds.
- Each rebuild re-selects the ring (O(worldspace wet cells), via `plane.projection`). It then calls `upload_scene_meshes_batched` on `ctx.graphics_queue` with `transfer_fence`, and the main thread blocks until the fence signals.
- The new mesh takes a permanent `meshes` slot (slots are append-only) and the old one leaves a hole (`geometry_has_holes`, `geometry_dirty`).
- A crossing applies its ring one cell at a time over roughly 1-3 s (`STREAMING_APPLY_BUDGET` 16 ms per frame). That gives several rebuilds, each with a blocking submit, per crossing.
- Worldspace entry and boot are unbudgeted by design, but they follow the same 250 ms cadence for the whole initial apply.

## Evidence
```rust
self.lod_water_dirty |= residency_changed;
...
if plane.last_rebuilt.is_some_and(|at| at.elapsed() < Self::LOD_WATER_REBUILD_INTERVAL) { return; }
self.lod_water_dirty = false;
crate::cell_loader::rebuild_lod_water_mesh(world, ctx, plane, game, player_grid, |g| loaded.contains_key(&g));
```
`rebuild_lod_water_mesh` builds a `GpuUploadCtx { queue: &ctx.graphics_queue, … }` and passes `transfer_fence` to `upload_scene_meshes_batched`.

## Impact
- Main-thread stall per rebuild equals the time for the GPU to retire the transfer submit behind queued frame work. That is bounded by about one GPU frame, which the 09-26b record puts at tens of ms in heavy RT scenes.
- It lands on the frames that already carry the apply slice, which `FrameTimeBudget` cannot preempt.
- Derived, not measured. A `water.dump` plus the `cpu_ms:` `atw_post` bucket on a Commonwealth or Tamriel crossing would show it.

## Suggested Fix
Rebuild on residency *settle* (the pending-apply set empty, or no change for N frames) with the 250 ms floor kept as a ceiling, so a crossing pays one rebuild. Alternatively, move the upload to the stream pool's transfer path. Measure before changing: record `UnloadPhaseTimings` plus a rebuild-count telemetry counter.

**Related**: #5289 (closed), #5334 (closed), #5339 (closed), `streaming/mod.rs:610` (the doc comment already names the multiplication risk)

## Completeness Checks
- [ ] **SIBLING**: Other residency-driven rebuilds that take a fence-waited upload on the main thread
- [ ] **TESTS**: A rebuild-count counter/test pins that a multi-cell crossing coalesces to one rebuild

Fix with: `/fix-issue 5705`
