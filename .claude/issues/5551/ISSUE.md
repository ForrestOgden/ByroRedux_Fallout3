# #5551: PHYS-D1-2026-10-11-05: The release test target of `byroredux-physics` no longer compiles; both release-only shape-boundary guards are unrunnable, and the #2862 compound backstop has no runnable test in any lane

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5551
- **Labels**: low,physics,test-gap,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Shape Translation (guards)
- **Location**: `crates/physics/src/convert.rs:748-777` (the error at `:769-773`). Collateral: `:661-672`.
- **Status**: NEW. Introduced by `ac81a09f9`. Related to #4686 (CLOSED, which added a debug twin for the #2543 clamp only).
- **Description**: `non_finite_compound_child_transform_neutralizes_instead_of_reaching_rapier` is `#[cfg(not(debug_assertions))]`, so the migration's debug-lane build never compiled it. It still uses the nalgebra API: `iso.rotation.into_inner().coords.iter()`. Under rapier 0.36, `Pose::rotation` is `glam::Quat` (glamx `rot3.rs:9`), which has no `into_inner`. The whole release test binary fails, which also kills `non_finite_cuboid_extent_clamps_instead_of_reaching_rapier`.
- **Evidence**: `cargo +1.96 test --release -p byroredux-physics --no-run` output:
  ```
  error[E0599]: no method named `into_inner` found for struct `glam::Quat` in the current scope
     --> crates/physics/src/convert.rs:770:18
  error: could not compile `byroredux-physics` (lib test) due to 1 previous error
  ```
  No CI lane runs physics in release. `real-data-gates.yml:207` loops only bsa/bgsm/sfmaterial/hkx/facegen/menuxml, which is how this stayed latent.
- **Trigger Conditions**: any `cargo test --release -p byroredux-physics` (or a workspace-wide `--release` test).
- **Impact**: The #2862 release backstop (convert.rs:235-236, which neutralises a non-finite child pose to identity) has no executable coverage anywhere. The debug lane only checks that the `debug_assert!` fires (convert.rs:726-738). The #2543 cuboid release test is collateral; it is partly compensated by the #4686 debug twin. The code itself is correct by inspection.
- **Related**: #2862, #2543, #4686.
- **Suggested Fix**: Replace the assertion with `iso.rotation.is_finite()` (glam). Add a default-lane twin that calls a factored `sanitize_child_pose(t, r)` helper directly, as #4686 did for `clamp_shape_extent`, so the backstop is covered without depending on the release profile.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
