# #5557: PHYS-D2-2026-10-11-07: Two new physics docs describe behaviour the code does not have — the force-event buffer "cleared at every substep start", and `build_ragdoll` "packing" the surface material

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5557
- **Labels**: low,physics,documentation,doc-rot
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Step & Sync
- **Location**: `crates/physics/src/world/mod.rs:277-282` (field doc) vs `:956-960` + `:1095-1103`; `crates/physics/src/world/mod.rs:597-605` vs `crates/physics/src/ragdoll.rs:426-458`
- **Status**: NEW (introduced by `1291f370a` / `886edcded`)
- **Description**:
  - **Clear point.** The `pending_force_events` field doc says "Cleared at every substep start". The code clears once
    at frame entry and *appends* across substeps, and the inline comment (`:956-959`) and
    `take_contact_force_events`' doc say so correctly. If the field doc were believed and "fixed" to match, multi-substep
    frames would lose all but the last substep's crossings.
  - **Ragdoll material.** `collider_surface_material`'s doc says the material is "packed into a collider's user-data at
    registration (`register_newcomers` / `build_ragdoll`)". `build_ragdoll` passes `0` to `collision_shape_to_parts`,
    ignores the returned part material (`_surface_material`), and sets no `user_data`. Ragdoll colliders read STONE by
    default, not by packing.
- **Evidence**: The cited lines.
- **Impact**: Misleading comments around the engine's first contact-event path. No runtime effect.
- **Trigger Conditions**: n/a
- **Related**: D2-04
- **Suggested Fix**: Reword the field doc to "cleared at `step` entry; substeps append". Reword the accessor doc to
  "`register_newcomers` packs it; `build_ragdoll` colliders carry the default 0".

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
