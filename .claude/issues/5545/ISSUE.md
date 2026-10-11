# #5545: PHYS-D2-2026-10-11-03: The surface-aware footstep probe starts inside the player's own capsule and never excludes it, so every player footstep reads material 0

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5545
- **Labels**: medium,physics,audio,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: MEDIUM
- **Dimension**: Queries & Diagnostics (found under Dim 2's footstep-surface scope)
- **Location**: `crates/physics/src/world/queries.rs:220-230` (`surface_material_under` — `solid_probe_filter()` with no exclusion parameter); caller `byroredux/src/systems/audio.rs:262-270`; emitter on the body `byroredux/src/scene.rs:1162-1180, 1245`
- **Status**: NEW (introduced by `886edcded`; the same self-hit class as closed #2859)
- **Description**:
  - In character mode the `FootstepEmitter` is on the player body (`scene.rs:1245`, #5146). The trigger position is
    that body's `GlobalTransform.translation`, which is the capsule centre.
  - The player capsule is `CharacterKinematic`, i.e. rapier `KinematicPositionBased`, with `surface_material: 0`
    (`scene.rs:1171-1180`). It is not in `ACTOR_BONE_GROUP`.
  - `surface_material_under` casts a solid ray with `solid_probe_filter()`, which excludes dynamics, sensors and actor
    bones but **not kinematics**. It has no `excluded_body` parameter at all.
  - A solid ray that starts inside the capsule returns `toi = 0` on the capsule itself. That is exactly the #2859
    failure `cast_ray_down`'s doc warns about (`queries.rs:178-186`: "`excluded_body` must be passed whenever the
    origin can lie inside a body — the player capsule").
- **Evidence**: Probe (production registration; floor `surface_material: 7`; player capsule half-height 46, radius 18,
  centre 64 above the floor):
  ```
  from the body centre (footstep_system's origin): Some(0); same height 100 BU aside: Some(7)
  ```
  The only test, `surface_material_under_reads_the_hit_colliders_material` (`world/mod.rs:3260`), casts from open air
  with no player body.
- **Impact**:
  - The surface lookup is live every footstep, but its answer is always STONE for the player, which is the only
    emitter that fires (#5348).
  - Once `FootstepConfig.surface_sounds` is populated, the player's steps play the stone sound (or the default) on
    every surface: the per-material footstep feature is dead on arrival.
  - It also wastes the per-trigger ray.
- **Trigger Conditions**: character mode (any game), any footstep.
- **Related**: #2859 (closed: the same self-hit in the fog-height probe), #5146, #5348
- **Suggested Fix**:
  - Give `surface_material_under` an `excluded_body: Option<RigidBodyHandle>` like `cast_ray_down`, with the same
    mandatory-decision rationale.
  - In `footstep_system`, resolve the body's `RapierHandles.body` (in the pre-resolved `body_gate` block, to keep the
    #5305 lock discipline) and pass it.
  - Add a test with a capsule around the origin.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
