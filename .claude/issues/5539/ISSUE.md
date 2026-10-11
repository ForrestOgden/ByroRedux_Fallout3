# #5539: PHYS-D4-2026-10-11-01: rapier 0.36's snap-on-every-horizontal-move sinks walking NPCs into convex floors, and through thin ones; the KCC then reports `grounded`, so the clamped fallback ray never corrects it

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5539
- **Labels**: high,physics,ai,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: HIGH
- **Dimension**: Character Controller, Grounding & the NPC KCC (M42.10)
- **Location**: `byroredux/src/systems/locomotion.rs:183-225` (NPC step: purely horizontal `desired`, snap 64 BU,
  fallback gated on `!result.grounded`); `crates/physics/src/world/queries.rs:667-671` (snap passed through);
  `locomotion.rs:72-76` (`LOCOMOTION_NPC_SNAP_TO_GROUND`)
- **Status**: NEW (regression introduced by `ac81a09f9`; the player-side analogue is closed #2857, HIGH)
- **Description**: The NPC step asks the KCC for a purely horizontal translation (`locomotion.rs:183-188`, `y = 0`)
  from a capsule whose bottom sits exactly at the feet (`position = feet + hh + r`, `locomotion.rs:178,192`) —
  i.e. *inside* the 4 BU offset band of the floor. Under rapier 0.22 snap-to-ground only fired for
  `translation·up < -1e-5` (0.22 `character_controller.rs:371`), so a horizontal NPC step never snapped. Since 0.35
  it fires on `<= 0.0` (0.36 `:466`) — every horizontal NPC step from a grounded start now runs the snap cast, a
  `-up` shape cast with `target_distance = offset` from a configuration already inside the target band. Against a
  *convex* (support-map) floor that cast intermittently returns a large bogus time of impact — the exact parry
  failure #2857 documented for the player ("in that configuration parry's cast against a CONVEX primitive frequently
  reports no interference", `character.rs:386-399`) — and snap applies it (`result.translation -= up * toi`,
  `grounded = true`, 0.36 :481-482). Because 0.36 now also reports `grounded`, the `!result.grounded` fallback
  (`locomotion.rs:205-225`, the only thing that re-grounded NPCs under 0.22 — it fired on 335–409 of 600 frames
  there) never runs, so the sink persists until a later sweep pops the capsule back up. The player is immune only
  because #2857 gave *it* a probe-bounded vertical request (`character.rs:418-477`); the NPC step never got one,
  and the #2857 guard tests only the player policy.
- **Evidence** (probes; same harness both versions; NPC params = production: hh 32, r 20, offset 4, snap 64, step
  32/8, 100 BU/s, 60 Hz; cuboid floor slabs; production airborne fallback replicated):
  - Sweep of 42 floor heights (#2857's set: 0, 137, −60 + 7.3·i) × slab thickness {20, 4} × 4 headings, 240 frames:
    **0.36: NPC sank > 1 BU in 82/336, went below the slab's bottom face in 63/336**; 0.22: 54/336 and 10/336.
    0.36 with `snap_to_ground = 0`: **0/336 and 0/336**. Player replica (production grounded branch) on the same
    floors: 0/336.
  - Sunk on 3 626 of 80 640 NPC-frames; worst configuration sunk 193/240 frames; 13 configurations still sunk at
    the end of the 4 s window.
  - Trace (20 BU slab, diagonal): `f58: 3.821 → -12.433 (kcc dy -16.254, grounded true, fallback false)`, sinking
    to −20.18 (the slab's bottom face is −20) until `f89: kcc dy +24.180`. 8 BU slab: feet reach −53.4; 4 BU slab:
    −35.4. Trimesh floors: no sink (feet stay within [0, 4]).
- **Impact**: NPCs walking on box / convex-hull walkable collision (`BhkBoxShape`, `BhkConvexVerticesShape` parts,
  the FO4+/Starfield `PackedAabbProxy` cuboids on Clutter/Actor placements, `cell_loader/spawn.rs:58-72`) dip
  knee- to waist-deep for ~0.7 s at a time and pop back up; through thin convex floors (planks, catwalks) they drop
  below the floor entirely. Visible on every game that uses convex walkable shapes; direction- and absolute-height
  dependent (float precision of the resting gap, as in #2857), so it reads as intermittent. Content prevalence of
  convex walkable floors was not measured.
- **Trigger Conditions**: any NPC whose Wander/Patrol/Travel/Follow/Escort/Guard/combat/force-greet/Eat-Sleep step
  runs `step_toward(_detailed)` with a `PhysicsWorld` on a convex floor collider, grounded at the start of the step.
- **Related**: #2857 (closed, player analogue, HIGH), #3799, `738ab7391` (re-checked grounding but not snap on
  horizontal moves), #5452 (same mover path, different defect)
- **Suggested Fix**: Give the NPC step the player's #2857 treatment — measure the gap with
  `cast_capsule_down_surface_and_normal` and request exactly that vertical alongside the horizontal move — or pass
  `snap_to_ground: 0` for NPCs and let the existing clamped fallback ray handle downslopes (0/336 sinks measured).
  Add an NPC variant of `grounded_capsule_does_not_sink_through_convex_floors_across_absolute_y` driving the real
  `step_toward_detailed` parameters.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
