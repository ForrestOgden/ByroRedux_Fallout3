# #5697 — PHYS-D3-2026-10-12-01: Angular damping on multibody links is a mass-scaled stand-in about 60x weaker than authored, on the false premise that rapier exposes no inertia

**Labels**: `medium`, `physics`, `water`, `bug`

**Source**: `docs/audits/AUDIT_PHYSICS_2026-10-12.md` (PHYS-D3-01) · audited at HEAD 5d31fd637

- **Severity**: MEDIUM
- **Dimension**: Ragdoll & Constraint Seam (also touches Water / Buoyancy)
- **Location**: `crates/physics/src/world/mod.rs` `apply_multibody_link_damping` (dry links); `crates/physics/src/water.rs` wet-link scan (`pending_torque`)
- **Status**: NEW (the linear half of #5540 is correct and pinned; this is its angular half)
- **Trigger Conditions**: any ragdoll (every dead NPC and creature on every game with a decoded constraint graph) with a nonzero authored or submerged angular damping.

## Description
#5540 found that rapier skips `RigidBody` damping for multibody links and moved both damping terms onto the user force/torque channel. The torque is computed as `-c * m * w`. Its comment justifies this with "Rapier 0.36 exposes no world angular inertia on a RigidBody (mprops is crate-private)". That is false: `RigidBody::mass_properties()` is `pub` (`rapier3d-0.36.1/src/dynamics/rigid_body.rs:360`) and `RigidBodyMassProps::effective_angular_inertia()` is `pub` (`rigid_body_components.rs:395`). The correct torque for exponential decay at rate `c` is `-c * I_world * w`. The code's `m * w` damps at `c * m / I` instead of `c`. A bone-sized capsule has `I / m` of about 66.

## Evidence
Probe: two-link ragdoll, one capsule (half-height 10, radius 5, mass 10) per bone, gravity 0, spin imparted by torque with damping zero, then authored `angular_damping = c` restored and 4 s simulated through the production `PhysicsWorld::step`.

| authored `c` | root `|w|` after 4 s (measured) | authored exponential expectation |
|---|---:|---:|
| 0.05 | 12.18 (from 13.5) | 11.07 |
| 0.5 | 12.68 (from 13.5) | 1.83 |
| 0.9 | 10.47 (from 13.5) | 0.37 |

`body.mass_properties().effective_angular_inertia()` read from outside the crate returns diagonal entries 665.6 / 118.8 / 665.6 for that mass-10 bone, so `I/m` is about 66. At `c = 0.9` the spin loses 22 % in 4 s where the authored value would remove 97 %.

## Impact
Ragdoll limb spin is nearly undamped, dry and wet. Authored bone angular damping and the `ragdoll_extra_angular_damping` lever are effectively inert, which is the state #5540 was filed to remove. Visible as corpses that keep rotating, and floating corpses that roll longer. Not a crash, bounded by the DOF clamp.

## Related
#5540 (closed), `ragdoll_extra_angular_damping` (`physal.md` section 4).

## Suggested Fix
Compute `torque = -c * (I_world * w)` from `body.mass_properties().effective_angular_inertia()` (check rapier's "square root" wording in that accessor's doc against the measured values, which look like plain inertia). Apply it in both the dry pass and the wet scan. Add an angular decay pin beside `dry_multibody_link_damping_decays_drift_velocity`, whose assertion only measures `linvel.y`.

## Completeness Checks
- [ ] **SIBLING**: Dry pass (`apply_multibody_link_damping`) and wet scan (`water.rs` `pending_torque`) both use the inertia-based torque
- [ ] **TESTS**: An angular-decay regression test pins the exponential rate against the authored `c`
