# PERF-D1-2026-10-12-02: The physics water scan keeps its per-wet-body latches in std SipHash maps

- **GitHub**: #5709 (https://github.com/matiaszanolli/ByroRedux/issues/5709)
- **Labels**: low,performance,physics,water,bug
- **Report**: docs/audits/AUDIT_PERFORMANCE_2026-10-12.md

**Severity**: LOW

## Location
- `crates/physics/src/water.rs:38,153` (`WaterContactScratch::rest_depth: HashMap<EntityId, f32>`)
- `crates/physics/src/water.rs:1102-1112,1241,1255,1326-1328`
- `crates/physics/src/world/mod.rs:260,479,889,909,1276` (`water_force_targets: std::collections::HashSet<RigidBodyHandle>`)

## Description
- `physics_sync_system`'s buoyancy scan runs every frame over every wet dynamic body.
- For each one it does a SipHash `remove`/`entry` on `rest_depth` (dry arms, awake arms and sleeping arms each touch it) and a SipHash `insert` into `water_force_targets` via `note_water_force_target`.
- `apply_multibody_link_damping` then does a SipHash `contains` per awake dynamic body while iterating *all* bodies.
- The scan ends with `rest_depth.retain(|e, _| targets.iter().any(…))`, O(latched × targets).
- This is a per-frame, per-entity keyspace. The hot-path hashing rule (#2923) says std `HashMap`/`HashSet` there is a fresh regression. `crates/physics/Cargo.toml` does not depend on `rustc-hash`.

## Evidence
`rest_depth: HashMap<EntityId, f32>`, `pw.note_water_force_target(t.handles.body)` per target, `rest_depth.retain(|entity, _| targets.iter().any(|t| t.entity == *entity))`.

## Impact
- ~10-20 ns per SipHash op. A pond with tens of floats and ragdoll bones is on the order of microseconds per frame, so LOW. It matters only because the project's own rule treats the pattern as a regression.
- Ragdoll-heavy wet scenes (FNV Lake Mead, Skyrim rivers with many corpses) scale linearly with bodies.

## Suggested Fix
Use `FxHashMap` / `FxHashSet` (add `rustc-hash` to the physics crate). For `rest_depth`, prune by comparing a scan generation stamp instead of `any()`. Add a source-scan guard like the `skin_slot_pool` one (`skin_slot_pool_maps_are_not_siphash`).

**Related**: #2923, #5561 and #5540 (closed; introduced both), `crates/core/src/ecs/resources/skin_slot_pool.rs`

## Completeness Checks
- [ ] **SIBLING**: Other `std::collections::HashMap/HashSet` per-frame keyspaces in `crates/physics` (see also PHYS-D2-2026-10-12-02, `apply_multibody_link_damping`)
- [ ] **TESTS**: Source-scan guard modelled on `skin_slot_pool_maps_are_not_siphash`

Fix with: `/fix-issue 5709`
