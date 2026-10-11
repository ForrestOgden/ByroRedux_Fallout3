# #5543: PHYS-D2-2026-10-11-01: The quarantine cooldown's "arm one more step" wake is cleared by `step`'s post-loop `pending_wake = false`, so in a quiet scene a re-enabled body never re-enters collisions or queries

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5543
- **Labels**: medium,physics,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: MEDIUM
- **Dimension**: Step & Sync
- **Location**: `crates/physics/src/world/mod.rs:998-1000` (the clear), `:1104-1118` (cooldown re-enable + `wake()`), `:1141-1146` (quarantine + `wake()`); guard `:2627-2676`
- **Status**: NEW (introduced by `52de1bc0a`)
- **Description**:
  - Both cooldown wakes are armed inside `run_substep`. Back in `step`, any frame that ran a substep does
    `if steps > 0 { self.pending_wake = false; }`, which clears the wake the substep itself just armed.
  - The quarantine-detection wake (`:1145`) only *looks* honoured. The next frame steps because the disabled body is
    still in rapier's (stale) active set: rapier processes the disable next step (`quarantine.rs:136-141`), so
    `has_awake_dynamics()` is true.
  - The re-enable wake (`:1117`) has no such rescue.
    - After that step the body is out of the active set.
    - `set_enabled(true)` only flags `RigidBodyChanges::all()` (`rigid_body.rs:260-274`).
    - Its colliders stay `DisabledByParent` until a `pipeline.step` runs the user-change pass (`user_changes.rs:120-136`).
  - So in a quiescent scene (nothing awake, nothing moving) the parked body has no collider in the broad phase or the
    narrow phase. It stays that way until some unrelated motion arms a step.
  - Commit `52de1bc0a` and the field doc both promise the opposite: "arming one more step so the re-enable is processed
    even if the scene is otherwise asleep … sits out of collisions and queries for that one step".
- **Evidence**: Probe (rapier-quarantined NaN-force ball, nothing else in the world):
  ```
  step1 substeps=1 quarantined=[h] enabled=false pending_wake=false   ← :1145 wake already cleared
  step2 substeps=1 enabled=true col_enabled=false pending_wake=false active=0   ← :1117 wake cleared
  after 120 frames: substeps=0 col_enabled=false ray_hit=None          ← straight-down ray misses the body
  after unrelated wake: substeps=1 col_enabled=true ray_hit=Some(... distance: 18.0)
  ```
  - The existing guard says "The cooldown armed a step even though nothing else is awake" (`:2654`). That is false
    attribution: its second step comes from the stale active set.
  - The guard asserts `body.is_enabled()`, which is the flag, not the collider. It stays green with both `wake()`
    lines deleted.
- **Impact**:
  - A corpse limb or clutter item recovered from a rapier quarantine stays invisible to `cast_ray`,
    `cast_ray_corridor`, `capsule_overlaps_solid` and the KCC sweep, and generates no contacts, until something
    else steps.
  - Concretely, a ragdoll that explodes and is fully parked while the player stands still cannot be targeted by the
    activation or loot ray, or hit by melee, until the player moves. The player's kinematic push wakes only on a
    move larger than 0.01 BU (`sync.rs:104-109`).
  - The bug self-heals on the next real step, hence MEDIUM rather than HIGH.
- **Trigger Conditions**: rapier quarantines a body (a non-finite solve, e.g. a ragdoll explosion), and afterwards no
  dynamic body is awake and nothing calls `wake()` (player idle, no animated keyframed bodies, no buoyancy change).
- **Related**: #5488, the `52de1bc0a` follow-up, #2856 (the same "wake consumed before work happens" class, fixed for
  external wakes only).
- **Suggested Fix**:
  - Re-arm after the loop rather than inside the substep. For example, set
    `self.pending_wake = !self.quarantine_cooldown.is_empty() || reenabled_this_frame` where the post-loop clear
    happens, or keep a separate `wake_requested_by_substep` flag that survives the clear.
  - Extend the guard: after the re-enable step, a third `step` must return 1, the collider must be `is_enabled()`, and
    a ray must hit the body.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **LOCK_ORDER**: If a RwLock scope or access declaration changes, TypeId-sorted acquisition is preserved and `sys.accesses` stays complete
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
