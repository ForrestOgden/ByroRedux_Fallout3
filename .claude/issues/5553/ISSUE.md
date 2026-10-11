# #5553: PHYS-D1-2026-10-11-07: Migration-stale comments in convert.rs: parry's convex-hull "panic" premise, nalgebra type names, and the TriMesh `Err` rationale

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5553
- **Labels**: low,physics,documentation,doc-rot
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Shape Translation (doc rot)
- **Location**: `crates/physics/src/convert.rs:84-89`, `:93-95`, `:308-314`, `:410-412`, `:436-437`, `:458-460`, `:1065-1066`. Also `byroredux/src/cell_loader/spawn.rs:1404` (`Vec<(Isometry3, SharedShape)>`).
- **Status**: NEW (missed by the `f4c2b9931` sweep)
- **Description**:
  - **Panic premise.** Several comments say `convex_hull` "unwraps an internal `MissingSupportPoint` (a *panic*, not a `None`)". Parry 0.17.6 did: `from_convex_hull` called `convex_hull` → `try_convex_hull(points).unwrap()`. Parry 0.31.1's `ConvexPolyhedron::from_convex_hull` is `try_convex_hull(points).ok()…` (convex_polyhedron.rs:255-259), and `MissingSupportPoint` is now an `Err` (initial_mesh.rs:156-158). The `hull_degeneracy` classifier is still worth keeping for the collinear → capsule recovery and against the remaining internal `assert!` at triangle_facet.rs:42, but its stated reason is wrong.
  - **Isometry3 names.** The module docs still describe the return as `Vec<(Isometry3, SharedShape)>`. It is `(Pose, SharedShape, u8)`.
  - **TriMesh `Err` rationale.** convert.rs:410-412 says what parry still rejects "is a topology failure". In fact `with_flags` discards `set_flags`' `TopologyError` and errs only on empty indices (trimesh.rs:742-757), so the `Err` arm is unreachable after the filters.
- **Trigger Conditions**: none at runtime. The cost lands on the next maintainer reasoning from these comments.
- **Impact**: Stale rationale on the exact boundary the parry bump changed.
- **Related**: #3066, #2551.
- **Suggested Fix**: Restate the classifier's purpose (extent recovery plus defence against residual parry asserts). Update the type names. Note that the `Err` arm is a defensive no-op under 0.31.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
