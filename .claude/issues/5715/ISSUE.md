# CONC-D7-2026-10-12-01: Stale-payload salvage runs every parsed import of every stale payload synchronously, outside the frame budget's yield check

- **GitHub**: #5715 (https://github.com/matiaszanolli/ByroRedux/issues/5715)
- **Labels**: medium,concurrency,performance,terrain-exterior,bug
- **Report**: docs/audits/AUDIT_CONCURRENCY_2026-10-12.md

**Severity**: MEDIUM. This is a main-thread hitch with no upper bound in the streaming apply path. It is not a correctness bug or a deadlock. The size of the hitch is unmeasured: a play session's `worker_stale_salvaged_imports` telemetry would size it, and the severity downgrades to LOW if that stays near zero.

## Location
- `byroredux/src/streaming_helpers.rs:886-900` (the stale arm of the payload receive loop in `advance_streaming_apply`); the helper is at `:721-740` (`salvage_stale_imports`)
- `byroredux/src/streaming_helpers.rs:1052` (`consume_streaming_payload`, `:1029`)
- `byroredux/src/streaming_helpers.rs:742-766` (`cancel_streaming_apply_job`, whose `FinishImports` arm now salvages), reached from `cancel_active_streaming_apply`, which `drain_streaming_state` calls at `:526`
- What each salvaged entry costs: `byroredux/src/cell_loader/partial.rs` (`finish_partial_import`)

## Description
**Trigger conditions**
- (a) A payload is classified `StaleNewerPending` or `StaleNoPending` while the player keeps moving during an exterior fill. A fast-moving player, a cold fill or a fast-travel backlog produces several in a row.
- (b) A worldspace drain (door walk, exterior-to-exterior transition, save or debug load) cancels an `active_apply` that is mid-`FinishImports`.

#5512 correctly makes the worker's skip filters true: a key the worker skipped is now resident in `NifImportRegistry`. It does that by calling `salvage_stale_imports` at three drop sites, and each call runs `finish_streaming_import` for every entry of the payload in a plain `for` loop. Three properties follow:
1. **No yield inside the loop.** The normal path takes one import per outer iteration and tests `budget.should_yield()` between units (`FinishImports(mut imports)`, `:922`). The salvage runs the whole `payload.parsed` vector first, and `complete_unit()` runs only afterwards, so the budget cannot stop it.
2. **Several stale payloads run back to back.** The surrounding receive `loop` has no `should_yield` check on the stale arm. It keeps pulling and salvaging until it finds an `Apply` payload or the channel is empty.
3. **Each entry is not cheap.** `finish_partial_import` re-interns the worker's strings, runs `merge_external_materials` (BGSM/BGEM reads from the archive pool plus the dead-path probe through `tex_provider`), converts an embedded `NiControllerSequence` clip, and builds the `CachedNifImport`. That is the work the normal path's per-unit budget exists to slice.

The drain path (b) pays the same cost for no benefit it can use: `cancel_streaming_apply_job` salvages the active job's remaining `FinishImports` iterator inside a transition that is already tearing the world down.

## Evidence
```rust
// streaming_helpers.rs:886 — inside `loop { let Ok(payload) = state.payload_rx.try_recv() … }`
let salvaged = salvage_stale_imports(world, &mut state.mat_provider, &|p| state.tex_provider.has_texture(p),
                                     &mut state.telemetry, payload.parsed);
for _ in 0..salvaged { budget.complete_unit(); }   // charged AFTER the work
```
Verification path: `cargo test`-level for the structure (a stale payload carrying N entries finishes all N before the next `should_yield` check). The wall-clock cost needs a streaming run: read `worker_stale_salvaged_imports` (already in the telemetry line, #5512) beside `apply_max_ms` on a fast-travel or sprint-through-boundary session.

## Impact
A frame hitch whose length scales with (stale payloads in the channel) x (new unique models per payload) x (per-import merge cost). That is the same class of main-thread stall the worker exists to prevent. It lands on exactly the cold-fill and fast-travel frames that #5512 targeted. The old behaviour was a single main-thread parse fallback on the next request that needed a skipped key, which is cheaper per event but unbounded in a different way; the salvage trades a rare small hitch for a rarer large one.

## Suggested Fix
Salvage through the same budgeted machinery as a current apply. For example, push the stale payload's `parsed` iterator onto a small `VecDeque` of pending salvage jobs on `WorldStreamingState`, and advance one import per `advance_streaming_apply` iteration under `should_yield()`, calling `budget.complete_unit()` per import. For the drain path, skip the salvage entirely, or let the registry keep nothing from a cancelled job: the state, and with it the worker's memo, is being discarded, so the residency premise no longer applies.

**Related**: #5512 (closed; the fix this follows from), #3670 (the batch memo), #5491, CONC-D7-2026-10-09-01. PERF-D7-2026-10-12-01 is the LOD-water rebuild, a different path.

## Completeness Checks
- [ ] **SIBLING**: All three `salvage_stale_imports` call sites (stale arm, `consume_streaming_payload`, `cancel_streaming_apply_job`) are covered
- [ ] **TESTS**: A stale payload carrying N entries yields to `should_yield` between imports; the drain path does no salvage

Fix with: `/fix-issue 5715`
