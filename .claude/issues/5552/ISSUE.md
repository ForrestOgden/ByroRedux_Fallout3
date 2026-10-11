# #5552: PHYS-D1-2026-10-11-06: The GeometricMean pin tests the constant, not the two collider producers that must apply it

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5552
- **Labels**: low,physics,test-gap,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Shape Translation (contact combine)
- **Location**: `crates/physics/src/world/mod.rs:1677-1726` (guard). Producers at `crates/physics/src/sync.rs:1155-1156` and `crates/physics/src/ragdoll.rs:455-456`. Claim at `crates/physics/src/config.rs:70-75`.
- **Status**: NEW
- **Description**: config.rs:70-75 says the rule is "Applied at both collider producers … so they cannot drift apart. Pinned by `restitution_zero_body_does_not_bounce_on_restitution_floor`." That test builds both colliders by hand with `CONTACT_COEFFICIENT_COMBINE_RULE`. It fails if the constant changes to `Average`, but stays green if `register_newcomers` or `build_ragdoll` drops `.friction_combine_rule`/`.restitution_combine_rule`. Because `GeometricMean` wins on priority, losing it at one producer changes only pairs where neither side comes from the other producer. Losing it in sync.rs reverts every clutter-on-architecture and clutter-on-terrain pair to `Average` (restitution-0 clutter rebounding at 0.4 off a 0.8 surface), with no test failing.
- **Trigger Conditions**: a future edit to either builder chain (both are long builder expressions that were edited three times in this delta alone).
- **Impact**: The gap is in test coverage only. Today both producers do set the rule (verified above).
- **Related**: `04cb6333d`; plan `docs/engine/dependency-update-followups-plan.md` §1.1 "Done when".
- **Suggested Fix**: Add a test that registers a `RigidBodyData` body through `physics_sync_system`, and a two-body ragdoll through `build_ragdoll`, then asserts `collider.friction_combine_rule() == GeometricMean` (and restitution) on every inserted collider.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
