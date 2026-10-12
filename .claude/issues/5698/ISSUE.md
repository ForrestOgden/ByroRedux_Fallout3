# #5698 — PHYS-D2-2026-10-12-01: The dry-link damping pass resets every awake link's user force, wiping placed-current forces; its skip set is stale on every early-return path of the water scan

**Labels**: `low`, `physics`, `water`, `bug`

**Source**: `docs/audits/AUDIT_PHYSICS_2026-10-12.md` (PHYS-D2-01) · audited at HEAD 5d31fd637

- **Severity**: LOW
- **Dimension**: Step & Sync / Water
- **Location**: `crates/physics/src/world/mod.rs` `apply_multibody_link_damping` (`reset_forces`) and its call in `step`; `crates/physics/src/water.rs` (the only `water_force_targets.clear()`, the only per-body `note_water_force_target` inside the surface arm, and the current arm that flags a body only when it wakes it)
- **Status**: NEW
- **Trigger Conditions**: a ragdoll that is awake and inside a placed `WaterCurrentVolume` that overlaps no `WaterPlane` in XZ (the marker-only case #3114 and #5129 explicitly support for dynamic bodies), or that has just left water whose scan then early-returned.

## Description
Each stepping frame, `apply_multibody_link_damping` does `reset_forces(handle)` on every awake dynamic multibody link not in `water_force_targets`, then adds the damping force. Any other user force on such a link is discarded before the substeps run. The water scan protects its own forces by adding the body to `water_force_targets`, but only in the surface arm, or in the current arm when the current wakes a sleeping body. A link inside a marker-only current: the scan resets, adds the marker's current force, never flags the body; the pass then resets it away. The placed current does nothing to an awake ragdoll link. The set is also cleared only by a full scan: `surfaces.is_empty()`, the quiesced fast path and `targets.is_empty()` all return first, so after a plane streams out, ex-wet links stay "owned by the water pass" and lose dry damping until another full scan runs. `clear_stale_water_contacts` restores damping and force but not this set.

## Evidence
Probe: a ragdoll link with `angular_damping > 0`, `add_force(h, (0, 5000, 0))` then `step`: no motion at all (y stays at 1000). With the damping set to 0 the same force moves it (pass skipped by `lin <= 0 && ang <= 0`).

## Impact
Small. Placed currents are an FO3/FNV hazard detail, ragdolls in them are rarer still, and the effect is "no push", not instability. It also quietly breaks `PhysicsWorld::add_force`'s documented "accumulates until reset_forces" contract for links.

## Related
#3114, #3268, #5129, PHYS-D3-2026-10-12-01.

## Suggested Fix
Flag the body in the current arm too (any frame it receives a current force). Better, have the dry pass add its damping into a force it computes itself rather than resetting. Clear `water_force_targets` at the top of the scan, before the early returns, like `water_lift`.

## Completeness Checks
- [ ] **SIBLING**: Every early-return path of the water scan leaves `water_force_targets` consistent
- [ ] **LOCK_ORDER**: n/a unless the fix changes lock scopes in `PhysicsWorld`
- [ ] **TESTS**: A regression test pins a placed-current force on an awake link with nonzero damping
