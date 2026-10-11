# #5544: PHYS-D2-2026-10-11-02: rapier 0.36's begin-touch strong-wake wakes every authored-resting dynamic newcomer on its first step, so the spawn-asleep contract no longer holds for clutter

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5544
- **Labels**: medium,physics,audio,performance,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: MEDIUM
- **Dimension**: Step & Sync
- **Location**: `crates/physics/src/sync.rs:1086-1100` (spawn-asleep), upstream `rapier3d-0.36.1/src/geometry/narrow_phase/contacts.rs:363-369` (`strong_wake_sleeping_side` on every begin-touch); `crates/physics/src/world/mod.rs:639-657` (`wake()` doc contract)
- **Status**: NEW (behaviour change from the `ac81a09f9` migration; not in the plan's "Checked and ruled out" list)
- **Description**:
  - `register_newcomers` spawns every `MotionType::Dynamic` body `sleeping(true)`. Its rationale: "placed clutter is
    authored resting, so asleep is the correct spawn state".
  - Rapier 0.36 applies "Wake rule (whole-island sleep): starts wake unconditionally" (`contacts.rs:363-368`). The
    first `pipeline.step` after registration runs the narrow phase over the newcomer's colliders, because they are
    modified colliders (`contacts.rs:83-94`). Every newcomer touching anything within skin plus prediction distance
    produces a begin-touch transition and is strong-woken.
  - On rapier 0.22 the same transition only emitted collision events (`rapier3d-0.22.0/.../narrow_phase.rs:1080-1088`).
    A brand-new collider had no graph index for the `handle_user_changes_on_colliders` wake (`:434-456`), so an
    asleep newcomer resting on fixed ground stayed asleep.
  - The fast path still serves the streaming frame itself. The first frame that steps for any other reason (in an
    exterior the walking player arms one every frame) then wakes all resting clutter of the new cells. It stays awake
    for rapier's sleep timer (~1 s), and the contact skin lifts each item about 1 BU.
  - Each item also produces a `started` contact-force crossing carrying its settle impulse.
- **Evidence**: Probe through the production path (`physics_sync_system`; static floor; 50 boxes of mass 10 streamed
  in resting on it; then one unrelated `wake()`):
  ```
  after streaming frame: awake dyn=(0, 0) pending_wake=false     ← spawn-asleep holds for the streaming frame
  frame 0:  awake(dyn,kin)=(50, 0)                               ← first real step wakes all 50
  frame 6:  started_events=50 max_force=60246                    ← 50 high-force crossings (static load 6 867)
  frame 60: awake(dyn,kin)=(0, 0)
  ```
- **Impact**:
  - **Solver cost on every streaming boundary.** Every cell crossing pays ≥1 s of solver work over all resting dynamic
    clutter in the new cells, and the static-scene fast path is off for that window. The cost is bounded per frame by
    `SUBSTEP_TIME_BUDGET`. It has not been measured in a release build.
  - **Impact audio, once IPDS/IPCT content lands.** The `ImpactConfig` floor (`min_force_magnitude` 5 000) is far
    below these crossings. Every streamed or loaded cell would then play a burst of impact sounds, one per resting
    item, at load.
  - **Doc drift.** The `wake()` doc (`world/mod.rs:639-657`) and `docs/engine/watal.md` §0.1 still describe newcomers
    as staying asleep until touched.
- **Trigger Conditions**: a cell containing dynamic (Havok `MO_SYS_DYNAMIC`/box-inertia) clutter whose collider rests
  within contact-skin + prediction distance (≈ 2×1 BU skin + 1.4 BU) of another collider at spawn, which is the
  authored norm. Every game with dynamic clutter.
- **Related**: #3969 (spawn exemption contract), #1698 (settle storm), D2-04 (impact consumer)
- **Suggested Fix**:
  - Restore the intent explicitly. Track the dynamic handles spawned asleep in a `spawn_sleep_pending` list. After the
    first `pipeline.step` that processes them, `sleep()` each one still within the drift threshold of its spawn pose.
    Manual `sleep()` pins eligibility (`rigid_body_components.rs:1451-1457`), so the island sleeps on the next update.
  - Either exclude those first-step crossings from `pending_force_events`, or gate impacts on Δv (force/mass) rather
    than raw force.
  - Add a production-path test asserting zero awake dynamics after the first post-stream step.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
