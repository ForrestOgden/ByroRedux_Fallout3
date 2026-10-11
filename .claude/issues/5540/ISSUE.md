# #5540: PHYS-D5-2026-10-11-01: Submerged damping is inert on every jointed ragdoll body (multibody links ignore `RigidBody` damping), so a floating corpse's bob grows without bound instead of settling

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5540
- **Labels**: high,physics,water,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: HIGH
- **Dimension**: WATAL Physics Sink (seam: Dim 3 ragdoll — same root cause)
- **Location**: `crates/physics/src/water.rs:951-952` (submerged `set_linear_damping` / `set_angular_damping`), `:1024-1025,1037-1038` (restore), `:524-525` (`clear_stale_water_contacts` restore); `crates/physics/src/ragdoll.rs:385-399` (bone damping at build); upstream `rapier3d-0.36.1/src/dynamics/solver/staged_island_solver/worker.rs:956-974`, `rapier3d-0.22.0/src/dynamics/solver/velocity_solver.rs:295-312`
- **Status**: NEW. Present since #3492 (2026-08-27) made ragdolls reach the sink; not a migration regression, because both rapier versions skip rigid-body damping for multibody links.
- **Description**:
  - The buoyancy pass makes a float settle with submerged viscous damping: `b.set_linear_damping(1.5)` / `set_angular_damping(1.5)` (`water.rs:951-952`). Its own docs say this lets the body "settle (and can then sleep) rather than bobbing forever".
  - `build_ragdoll` joins every bone body with multibody joints (`Ragdoll.joints: Vec<MultibodyJointHandle>`). In rapier, every body of an articulation, root included, is a multibody link.
  - Rapier applies `RigidBody` damping only on the rigid-body writeback path. That path `continue`s for `multibodies.rigid_body_link(handle).is_some()` (`worker.rs:956-962`), and the damping call is at `:974`. Links take their velocities from the multibody's generalized solve, whose damping is the separate per-DOF vector (`multibody.rs:621`): 0.1 on angular DOFs, 0 on linear (`multibody_joint.rs:273-285`). 0.22 has the same split (`velocity_solver.rs:295-312`).
  - So for a ragdoll the submerged damping, the authored bone damping and the restore are all stored on the body but never applied. The vertical heave of a floating corpse has no damping at all.
  - Undamped, the per-frame zero-order-hold buoyancy (PHYS-D5-2026-10-11-02) injects energy every cycle, and ragdolls hold the force over 16 internal small steps per substep. The oscillation grows.
- **Evidence**: probe `/tmp/claude-1000/-mnt-data-src-gamebyro-redux/95e852db-3454-4021-b6be-d5acf8770538/scratchpad/probe/rag.rs (siblings zoh.rs, wave.rs, aabb.rs)`, linked against the repo's built `byroredux_physics` rlib. It runs the production `build_ragdoll` + `physics_sync_system`: two radius-8 balls, mass 10, calm water at y=0, 40 s simulated.
  ```
  jointed=false fps=60: multibody_link=false body.linear_damping=1.5 frac=0.64 last-5s p2p=0.00     asleep 299/299
  jointed=true  fps=60: multibody_link=true  ... last-5s y -1437.75..135.84 p2p=1573.59   asleep 0/299
  jointed=false fps=30: ... p2p=0.00 asleep 149/149
  jointed=true  fps=30: ... y -25004.24 (left the column through its floor, fell to the kill plane)
  ```
  The jointed trace at 60 fps (damping reads 1.5 while wet and is applied as 0):
  `t=1 s` peak depth 19 → `t=4.5` 35 → `t=9` 58 → `t=14.5` 87 BU, with launches out of the water to y=+55 between dives. The unjointed pair with identical shapes settles in ~2.5 s.
- **Impact**:
  - Any actor ragdolled in water: FNV Lake Mead, Skyrim rivers, slaughterfish and mudcrab kills.
  - The corpse hops in and out of the surface with growing amplitude until it strikes the lakebed or exits the volume. It never sleeps, so its island keeps the step awake forever.
  - Every exit and re-entry is a `WaterContact` dry→wet edge, producing a fresh splash event and splash sound (`systems/water.rs:411-541`).
  - Same root cause on the ragdoll side: the authored bhk bone damping and the `ContactConfig::ragdoll_extra_angular_damping` "less floppy than Havok" lever (`config.rs:114-117`) are inert on every jointed rig. Dim 3 lists the lever as verified ("added once per body"); it is added and then never applied. The pin test `recorded_damping_is_the_effective_value_build_ragdoll_applied` passes only because its fixture is a single unjointed body.
- **Trigger Conditions**: any ragdoll with at least one constraint, which is every vanilla rig, whose bodies reach a `WaterVolume`. It happens at any frame rate; lower rates diverge faster.
- **Related**: PHYS-D5-2026-10-11-02 (the energy source); #3492 (made ragdolls reach the sink); PHYS-D3-2026-10-11-04 (16 internal iterations).
- **Suggested Fix**: for multibody links, apply water drag as a force instead of rigid-body damping, `F = -m·c·v` added with the buoyancy force. Alternatively, write the submerged damping into the articulation's `Multibody::damping_mut()` for the free-root linear and angular DOFs, and restore it on exit. Apply the same treatment to `build_ragdoll`'s authored damping and the extra-angular lever. Pin it with a two-body jointed fixture.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **LOCK_ORDER**: If a RwLock scope or access declaration changes, TypeId-sorted acquisition is preserved and `sys.accesses` stays complete
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
