# #5700 — PHYS-D2-2026-10-12-03: The spawn re-sleep filters the impact buffer with `Vec::contains` (quadratic) and also drops genuine impacts of newcomers that moved

**Labels**: `low`, `physics`, `performance`, `bug`

**Source**: `docs/audits/AUDIT_PHYSICS_2026-10-12.md` (PHYS-D2-03) · audited at HEAD 5d31fd637

- **Severity**: LOW
- **Dimension**: Step & Sync
- **Location**: `crates/physics/src/world/mod.rs` spawn re-sleep block in `step` (`settled_colliders.extend`, the drift test, and the `pending_force_events.retain`)
- **Status**: NEW
- **Trigger Conditions**: the first stepping frame after a large newcomer batch (grid load, teleport, save load) with thousands of dynamic clutter bodies.

## Description
(a) `settled_colliders` is a `Vec` and each buffered event does two `contains` scans, so the filter is O(events x colliders). The collider force-event threshold is rapier's default 0.0, so every resting item's first contact emits a `started` event, which makes events about equal to newcomers on that frame. (b) `settled_colliders.extend(...)` runs before the drift test, so the filter also removes the events of newcomers that did drift more than `SPAWN_SLEEP_DRIFT_BU` (a real push, or an unsupported item falling), while the doc says those "keep rapier's verdict". Only their first stepped frame is affected.

## Evidence
Probe, `Vec<ColliderHandle>` `retain` with two `contains`: 1,000 -> 0.20 ms, 5,000 -> 4.7 ms, 15,000 -> 42.8 ms. For (b): code reading — `extend` precedes `let drifted = ...`.

## Impact
A one-off 5-40 ms spike on the first stepping frame of a very large load (per-cell batches of a few hundred are well under 1 ms), inside the streaming-hitch-sensitive window. For (b): a lost first-impact sound on a newcomer that falls or is hit on its first frame. Neither is a correctness failure.

## Related
#5544, PERF-D1-2026-10-12-03 (all dynamic colliders armed for force events).

## Suggested Fix
Build a hash set (or sort + binary search) of the settled colliders, and extend it only for bodies that were actually re-slept.

## Completeness Checks
- [ ] **SIBLING**: Other `Vec::contains` filters over per-frame collider/body batches in the physics crate
- [ ] **TESTS**: A regression test pins that a drifted newcomer's impact event survives the filter
