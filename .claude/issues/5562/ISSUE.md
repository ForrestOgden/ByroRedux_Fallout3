# #5562: PHYS-D5-2026-10-11-05: Two consumers still read the body origin where the WATAL contract says collider AABB centre: the buoyancy wave sample, and the splash/ripple marker height

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5562
- **Labels**: low,physics,water,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: WATAL Physics Sink (one reference point, #2887/#3490/#3973 class)
- **Location**: `crates/physics/src/water.rs:888-899` (`authored_wave_height_with_weather(.., Vec3::new(pos.x, pos.y, pos.z), ..)`, where `pos` is `body.translation()`, `:794-803`); `byroredux/src/systems/water.rs:438-442` (`position = global.translation; position.y += contact.depth`)
- **Status**: NEW (the wave sample dates from `6b9603490`; #3973 moved the three containment tests to `reference_point` but not this sample. The splash adapter dates from `5959bbb84`, before #2887 changed `depth` to AABB-centre-relative.)
- **Description**:
  - (a) The static surface height is sampled at `reference_point.xz` (`water.rs:880-887`), but the wave term added to it is sampled at the rigid-body origin. For compound bodies and ragdoll bones, which the code itself says are offset "by construction", the crest phase is taken at a different point from the column it is added to.
  - (b) The splash and ripple adapter comments "`GlobalTransform` is the body's centre", but `depth = surface_y − aabb_centre_y` (`water.rs:1010`). So `origin.y + depth` lands at `surface_y + (origin.y − centre.y)`, off the waterline by the origin↔centre offset. The XZ comes from the origin too.
- **Evidence**: the two cited lines. The doc at `water.rs:805-836` lists three consumers of `reference_point` and does not mention the wave sample.
- **Impact**: a small crest-phase error (amplitude × phase difference) in the buoyancy surface. The splash/ripple FX and sound for offset bodies are placed above or below the surface by up to the collider offset (tens of BU for offset compound parts and vertical ragdoll limbs).
- **Trigger Conditions**: a body whose collider AABB centre differs from its origin: bhk compound parts and ragdoll bones.
- **Related**: #2887, #3490, #3973
- **Suggested Fix**: (a) pass `reference_point` to `authored_wave_height_with_weather`. (b) Publish the contact's `surface_y` (or the reference XZ) in `WaterContact` and place the event there, rather than reconstructing it from `GlobalTransform`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
