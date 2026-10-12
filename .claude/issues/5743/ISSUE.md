# TD7-2026-10-12-01: The footstep surface probe's 128-unit reach is a bare literal

- **GitHub**: #5743 (https://github.com/matiaszanolli/ByroRedux/issues/5743)
- **Labels**: low,audio,physics,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD7-2026-10-12-01`)

**Audit finding** `TD7-2026-10-12-01` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `byroredux/src/systems/audio.rs` (`pw.surface_material_under(*pos, 128.0, own_body)`); the same literal is hand-typed in 3 `crates/physics/src/world/mod.rs` tests

## Description
The probe starts at the walker's capsule centre and reaches 128 BU; against the `capsule_y(46.0, 18.0)` the physics tests use (centre 64 BU above the feet) that is roughly twice the centre height, but nothing in the code says so. The neighbouring systems name their equal values (`FOLLOW_DEFAULT_DISTANCE`, `ESCORT_COLLECT_DISTANCE`, `FORCE_GREET_RADIUS`, all `128.0`) while this one does not.

## Evidence
`git grep -n 'surface_material_under(' -- byroredux crates` shows the single production caller (`systems/audio.rs`, still `128.0` at HEAD).

## Impact
A taller capsule (creature, crouch) or a different kinematic origin silently makes footsteps read `None` and take the default sound; there is no test tying the reach to the capsule.

## Suggested Fix
`const FOOTSTEP_SURFACE_PROBE_REACH: f32` next to the capsule constants in `crates/physics`, with a test asserting it exceeds the player capsule's centre height.

## Related
#5545, #5556, AUD-2026-10-12-D5-01.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
