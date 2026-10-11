# #5556: PHYS-D2-2026-10-11-06: `footstep_system`'s access declaration omits the new `PhysicsWorld` read, and the surface-sound selection reads `FootstepConfig` while holding the `AudioWorld` write

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5556
- **Labels**: low,physics,audio,concurrency,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Step & Sync (access declarations; rule owned by `/audit-concurrency`)
- **Location**: `byroredux/src/boot/schedule/late.rs:104-116`; `byroredux/src/systems/audio.rs:264-270` (`try_resource::<PhysicsWorld>`), `:274-293` + `:317-330` (`select_footstep_sound` → `try_resource::<FootstepConfig>` inside the `AudioWorld` write scope)
- **Status**: NEW (introduced by `886edcded`, which did not touch `late.rs`)
- **Description**:
  - **Undeclared read.** `886edcded` added a `PhysicsWorld` read to `footstep_system` (the per-trigger
    `surface_material_under`), but the `add_exclusive_with_access` declaration still lists only `FootstepConfig`,
    `PlayerEntity`, `FootstepScratch`, `AudioWorld`, `GlobalTransform`, `CharacterController`, `WaterContact` and
    `FootstepEmitter`. This is the "declaration forgotten" class the physics guards exist for.
    - `impact_audio_system` is declared correctly (`late.rs:123-130`).
    - No guard covers `footstep_system`'s read surface. `footstep_runs_after_camera_follow_in_late` checks ordering only.
  - **Nested resource lock.** The play loop calls `select_footstep_sound(world, …)` while `audio_world` (a resource
    write) is live, which acquires `FootstepConfig` under it. That violates the "one resource lock at a time"
    discipline the function's own comments state. `impact_audio_system` avoids it by snapshotting the map first.
    Because of D2-03, `surface` is always `Some`, so the nested acquisition runs on every footstep.
- **Evidence**: `late.rs:104-116` shows no `PhysicsWorld`. In `audio.rs:274-293`, the `audio_world` guard is held
  across `select_footstep_sound`.
- **Impact**: No live race, since `footstep_system` is a Late exclusive. But `sys.accesses` under-reports the
  system. A future move to the parallel batch would race `PhysicsWorld` writers undetected, and the
  `AudioWorld → FootstepConfig` edge is new detector surface.
- **Trigger Conditions**: always (declaration); every footstep in character mode (nested lock).
- **Related**: #5305, #5535 (lock-order lane; owned by `/audit-concurrency`), D2-03
- **Suggested Fix**:
  - Add `.reads_resource::<byroredux_physics::PhysicsWorld>()` to the declaration.
  - Clone `surface_sounds` out with the default at the top, as `impact_audio_system` does, and select from the snapshot.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **LOCK_ORDER**: If a RwLock scope or access declaration changes, TypeId-sorted acquisition is preserved and `sys.accesses` stays complete
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
