# PERF-D1-2026-10-12-01: `eat_sleep_system` rebuilds its actor list every frame, including seated sleepers, and sweeps three maps O(n·m)

- **GitHub**: #5707 (https://github.com/matiaszanolli/ByroRedux/issues/5707)
- **Labels**: low,performance,gameplay,ai,bug
- **Report**: docs/audits/AUDIT_PERFORMANCE_2026-10-12.md

**Severity**: LOW

## Location
`byroredux/src/systems/eat_sleep.rs:181-249`

## Description
- `actors` is a fresh `Vec<EatSleepActor>` (`:183`) filled from every `EatBehavior` / `SleepBehavior` row.
- Behaviour components are removed only at package change (`ai_package.rs:704`). So a seated diner or sleeper stays in the list for the whole package window, which for `SleepBehavior` is hours of game time.
- Each frame the vec is rebuilt, regrown and pruned by the `Seated` retain (`:233`). Then three `retain(|npc, _| actors.iter().any(…))` sweeps run (`:235-248`) whenever `seat_retry`, `anchor_retry` or `stuck_secs` is non-empty. Those are O(retry entries × actors).
- The maps are std `HashMap<EntityId, f32>`.
- Persistent scratch is the project's own rule for these systems, and `EatSleepScratch` already holds the maps beside it.

## Evidence
`let mut actors: Vec<EatSleepActor> = Vec::new();` at `eat_sleep.rs:183`; the `retain(…any…)` triples at `:233-248`.

## Impact
- Tens of allocations per second plus a few hundred comparisons per frame on a populated FNV saloon or Skyrim inn.
- Microseconds, so LOW. It matters only as the last per-frame allocation in a system whose siblings were already converted.
- Nothing quantitative guards it: per-frame ECS paths have no dhat coverage.

## Suggested Fix
Move `actors` into `EatSleepScratch` (`clear()` plus refill), and drop seated actors at the gather so they never enter the list (probe `Seated` while filling). Key the retry-map pruning off a generation or the gather's own set instead of `any()`.

**Related**: #5449, #5527 (closed; this is the residue they left), #5448

## Completeness Checks
- [ ] **SIBLING**: Sibling Eat/Sleep/Sandbox systems for per-frame `Vec::new()` + `retain(any)` shapes
- [ ] **TESTS**: A scratch-reuse / allocation test pins the converted gather

Fix with: `/fix-issue 5707`
