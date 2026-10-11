# #5555: PHYS-D2-2026-10-11-05: `clamp_explosive_velocities` walks the stale-tolerant dynamic index with no body-type check, so a scripted keyframed body that teleports is counted, logged and "parked" as an exploding dynamic

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5555
- **Labels**: low,physics,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Step & Sync
- **Location**: `crates/physics/src/world/recovery.rs:353-400` (no `body_type()` filter) vs the snapshot's filter `world/mod.rs:1067`; index doc `world/mod.rs:309-322`; `set_motion_type` keeps flipped-away handles `:813-820`
- **Status**: NEW (pre-dates the delta, since #5161/#5246; not in prior reports)
- **Description**:
  - The `dynamic_bodies` doc promises that staleness "costs a get() miss, never a wrong snapshot" because consumers
    re-check the type. The snapshot does (`:1067`). The clamp, a second consumer added by #5161, does not.
  - A body flipped Dynamic→Keyframed by Papyrus `SetMotionType` (`systems/cinematic.rs:265`) stays in the index.
  - On any tick where its kinematic velocity exceeds 20 000 BU/s (a `MoveTo`/`SetPosition` of more than about
    333 BU), the clamp does the following:
    - It counts a velocity clamp. `set_linvel` is a no-op on `KinematicPositionBased` (`rigid_body.rs:1022-1033`).
    - It logs `warn` "clamped explosive velocity".
    - On the second offence it logs `error` "parked … after repeated solver-explosion velocities" and calls `sleep()`,
      which zeroes the kinematic's velocity directly.
    - It adds a permanent `explosion_offences` entry.
- **Evidence**: Probe (dynamic → `set_motion_type(Keyframed)` → three 1 000 BU teleports):
  ```
  teleport 1: type=KinematicPositionBased linvel=28000 clamps_total=1 sleeping=false
  teleport 2: ... linvel=0 clamps_total=2 sleeping=true
  teleport 3: ... linvel=0 clamps_total=3 sleeping=true
  ```
- **Impact**:
  - False explosion evidence in the log and in `phys.stats` `velocity_clamps_total`. These are the counters #5161
    triage reads.
  - A per-body map entry that lives until the body is removed.
  - Rapier re-wakes the kinematic on its next changed target (`rigid_body.rs:1232-1239`), so motion itself is
    unaffected. A third offence detaches only when the body is in a multibody, which a scripted object is not.
- **Trigger Conditions**: a scripted `SetMotionType(Motion_Keyframed)` on a formerly dynamic reference, followed by
  teleports larger than about 333 BU per tick.
- **Related**: #5161, #5246, #4682
- **Suggested Fix**: Skip non-`Dynamic` entries in the clamp's collection loop, mirroring `:1067`, or compact the index
  on `set_motion_type` away from dynamic.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
