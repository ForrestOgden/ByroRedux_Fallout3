# AUD-2026-10-12-D5-01: Clutter-impact sounds play at the midpoint of the two parent *body origins*, not the contact. Every exterior terrain tile's body origin is the world origin

Labels: medium,audio,physics,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5675

Filed from `docs/audits/AUDIT_AUDIO_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: MEDIUM (dormant today: `ImpactConfig` ships with no `default_sound` and an empty map, so `impact_audio_system` returns before it computes a position. It is wrong the moment IPDS/IPCT content lands.)
- **Dimension**: Engine Consumers
- **Location**:
  - `crates/physics/src/world/mod.rs:706-716` (`contact_event_position`)
  - `byroredux/src/systems/audio.rs:405` (the consumer)
  - `byroredux/src/cell_loader/terrain.rs:1276-1285` (terrain ghost placed at `Vec3::ZERO`)
  - `byroredux/src/cell_loader/spawn.rs:398-418` (static trimesh ghosts placed at the REFR origin)
- **Status**: NEW (introduced by `1291f370a`)
- **Description**: The impact position is `(body_a.translation + body_b.translation) / 2`. That is the midpoint of the two *rigid-body origins*, not of the contact manifold.
  - For a dynamic clutter body hitting a static one, the second origin is wherever the static's body sits.
  - Static architecture ghosts are placed at the placement origin (`spawn_trimesh_collider_ghost(world, positions, indices, pos, rot, scale, …)`), which for a large kit piece can be many metres from the point of contact.
  - Exterior terrain is worse. The LAND tile's vertices are already world-space and the ghost is spawned with `Vec3::ZERO`, so its body translation is exactly the world origin.
  - The `ContactForceEvent` carries no contact point. The point has to come from the narrow phase (`contact_pair(c1, c2)` manifold points) or from the dynamic body's own position.
- **Evidence**:
  ```rust
  // physics/src/world/mod.rs:708-715
  let pa = self.bodies.get(a)?.translation();
  let pb = self.bodies.get(b)?.translation();
  Some(Vec3::new((pa.x + pb.x) * 0.5, (pa.y + pb.y) * 0.5, (pa.z + pb.z) * 0.5))
  // cell_loader/terrain.rs:1276-1284
  spawn_trimesh_collider_ghost(world, &positions, &indices, Vec3::ZERO, Quat::IDENTITY, 1.0, None)
  ```
  The only test is `contact_force_started_event_carries_surface_materials` (`physics/src/world/mod.rs:4102`), which asserts `contact_event_position(&event).is_some()`. Nothing pins the value, and `systems/audio.rs` tests cover selection only (`impact_selects_the_surface_sound_when_authored`, `an_unmapped_material_skips_only_its_own_impact`, `impact_material_resolution_keeps_both_zero_pairs`), never the position handed to `play_oneshot`.
- **Impact**: Once impact content is authored, a clutter item landing on exterior terrain at (x, z) plays at (x/2, z/2). At 300 m from the origin that is 150 m from the item, past the 24 m attenuation range, so exterior terrain impacts are inaudible. Interior statics shift the sound by half the kit piece's origin offset. No effect on the current build.
- **Related**: #5562 (the same "body origin where a contact point belongs" family on the water side), PERF-D1-2026-10-12-03 (same system, perf aspect).
- **Suggested Fix**: Resolve the position from the narrow-phase contact pair (deepest manifold point, converted to world space), falling back to the dynamic collider's body translation when the manifold is empty. Add a test that drops a body onto an off-origin static and asserts the resolved point is within the contact's extent.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix

