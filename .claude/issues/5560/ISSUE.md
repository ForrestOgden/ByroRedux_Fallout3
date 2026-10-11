# #5560: PHYS-D4-2026-10-11-03: On a flat floor the KCC still climbs a phantom slope and drops `grounded` on 1–13% of walking frames — the #3799 OR stays load-bearing while walking, which 738ab7391's rationale and its idle-only pin understate

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5560
- **Labels**: low,physics,documentation,doc-rot,test-gap
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Character Controller, Grounding & the NPC KCC (M42.10)
- **Location**: `byroredux/src/systems/character.rs:1612-1631` (rationale), `:1686` (the OR);
  `crates/physics/src/world/mod.rs:3208-3252` (`kcc_reports_grounded_for_a_capsule_at_rest`)
- **Status**: NEW (behaviour pre-exists and was worse on 0.22; the documentation claim is new in `738ab7391`)
- **Description**: 738ab7391 rewrote the OR's rationale to "rapier 0.36 closed that hole … the OR below is no longer
  a workaround for a rapier bug. It stays as a defence". True for the idle input it pins, not for walking: when a
  grounded capsule slides horizontally at exactly the offset gap, parry 0.31's support-map cast returns a
  `Converged` hit at a small-but-above-threshold TOI (the contact-normal fallback only covers `toi < 1e-4`,
  `shape_cast_support_map_support_map.rs:34-35`) with a GJK normal tilted 4–21° against the motion; `handle_slopes`
  treats it as a climbable slope (below 50°) and lifts the capsule by up to ≈ ½ the step, ending outside the
  narrowed 4.05 BU grounded band (0.36 :492), so the KCC answers `grounded = false`. The engine probe's walkable
  hit (`character.rs:453-454`) is what keeps `is_grounded` — and therefore jump — on those frames.
- **Evidence**: Probe P12 (captured collision events, cuboid floor, +X walk): `hit toi 0.019739 normal1
  (-0.0786, 0.9969, 0)`, `toi 0.008156 normal1 (-0.2344, 0.9330, -0.2730)`. Probe P6 (production correction +
  walk, 600 frames): KCC grounded lost 8/600 (diag walk), 4/600 (diag sprint), 16/600 (diag 144 Hz), 80/600 (+X
  walk); lifts up to 1.67 / 3.35 / 0.70 / 2.07 BU. Production replica P8: feet gap stays in [3.86, 7.72] (body bob,
  pulled back next frame by the probe; the three sprint runs that walked off the 4 000 BU test slab excluded) — vs
  [−13.7, +16.8] and KCC-grounded lost 70–452/600 for the same replica on 0.22, so the migration improved this; it did not close it. Idle (P4): 0/120 lost at every tested offset.
- **Impact**: none today (the OR covers it); a ≤ 0.5×step vertical body bob (camera eases a fraction of it up,
  `character.rs:746-752`). Removing or weakening the OR on the strength of the rationale would drop jump input on
  up to 13% of walking frames; `kcc_reports_grounded_for_a_capsule_at_rest` would stay green.
- **Trigger Conditions**: grounded player walking or sprinting on a flat convex or trimesh floor.
- **Related**: #3799, #3971, PHYS-D4-2026-10-11-01
- **Suggested Fix**: Reword the rationale to say the OR is still load-bearing on walking frames (cite the phantom
  slope), and add a walking variant of the 0.36 pin that drives the production correction + horizontal input and
  asserts `kcc_grounded || probe_found_support` on every frame while documenting the KCC-only loss rate.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
