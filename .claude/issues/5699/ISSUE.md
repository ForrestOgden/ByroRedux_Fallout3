# #5699 — PHYS-D2-2026-10-12-02: `apply_multibody_link_damping` walks the whole body arena (and SipHashes each awake dynamic) on every stepping frame

**Labels**: `low`, `physics`, `performance`, `bug`

**Source**: `docs/audits/AUDIT_PHYSICS_2026-10-12.md` (PHYS-D2-02) · audited at HEAD 5d31fd637

- **Severity**: LOW
- **Dimension**: Step & Sync (cost)
- **Location**: `crates/physics/src/world/mod.rs` `apply_multibody_link_damping`
- **Status**: NEW (same SipHash class as PERF-D1-2026-10-12-02 — cited, not re-reported)

## Description
The pass iterates `self.bodies.iter()`, which visits fixed bodies too, testing `is_dynamic()`, then `is_sleeping()`, then `water_force_targets.contains` (std SipHash `HashSet`), and only then `rigid_body_link`. #4682 moved the recovery snapshot off exactly this kind of arena walk, measured at 1.77 ms per substep on a 95 k-body world. Stepping frames are the normal state in an occupied cell (any animated NPC bone pushes a kinematic target and arms `pending_wake`).

## Evidence
Probe, same loop shape, opt-level 3: 0.039 ms/call at 20,000 fixed bodies, 0.21 ms/call at 95,000. The engine's dev profile builds `byroredux-physics` itself at opt-level 0, so the in-engine number is higher.

## Impact
About 0.2 ms or more per stepping frame on a Commonwealth-class exterior, for a pass that has no work unless a ragdoll is awake. Not a hitch; a steady tax on the CPU-bound path.

## Related
#4682, PERF-D1-2026-10-12-02.

## Suggested Fix
Walk `self.islands.active_bodies()` (awake only) or the ragdoll body list, and test `rigid_body_link` before the hash lookup.

## Completeness Checks
- [ ] **SIBLING**: Other per-step full-arena walks in `crates/physics/src/world/` checked
- [ ] **TESTS**: Existing damping pins still pass after switching the iteration source
