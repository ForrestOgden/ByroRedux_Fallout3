# #5564: PHYS-D6-2026-10-11-02: `phys.stats`' contact trio mixes populations: `pairs` is the whole narrow-phase graph (sleeping and non-touching pairs included), while `contacts`/`constraints` are the awake island only, and `constraints` never counts the m…

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5564
- **Labels**: low,physics,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Queries, Diagnostics & Cost
- **Location**: `crates/physics/src/world/mod.rs:570-584` (`contact_counts` and its doc); `byroredux/src/commands/physics.rs:216-219, 236-239`
- **Status**: NEW (introduced by `e25d89d17`)
- **Description**: each count comes from a different source in rapier 0.36:
  - `ncontact_pairs = narrow_phase.contact_graph().graph.edges.len()` (`solve.rs:158`). That is every broad-phase pair the narrow phase holds: touching or merely AABB-overlapping, awake or in sleeping islands.
  - `ncontacts` and `nconstraints` are summed only inside `init_and_solve` for `islands.awake_island` (`solve.rs:378-387`, `init.rs:600-610`).
    - `nconstraints = graph.len() + joint_indices.len()`, which is active contact manifolds plus impulse joints (`solver_contact_graph.rs:204-211`).
    - Multibody joints are solved as generic constraints and never enter `joint_indices`. Every engine ragdoll is a multibody articulation (`ragdoll.rs:517`, `pw.multibody_joints.insert`).
  - The doc's "zeroes only before the very first step" is also false. Any step with no awake island zeroes `contacts` and `constraints`, and the static fast path then freezes that value.
  - The commit and doc pitch the trio as "the live solve's shape (a runaway count is what the recovery counters below it usually precede)". In fact the first number is dominated by sleeping resting clutter, and ragdoll joints never appear in it.
- **Evidence**: from the probe:
  - `trio_populations`: 10 boxes each resting on its own floor, then one woken with the other nine asleep, gives `counts(pairs,contacts,constraints)=(10, 4, 1)`. After re-quiescing it reads `(10, 0, 0)`.
  - `multibody_joint_not_in_constraints`: one multibody spherical joint plus one impulse spherical joint, no contacts, gives `(0, 0, 1)`.
- **Impact**:
  - **Diagnostic misreading only.** On a loaded exterior `pairs` tracks the count of resting clutter (thousands), not solver load.
  - A ragdoll whose articulation is exploding shows no constraint growth. An operator comparing `pairs` with `contacts` compares two different populations.
  - There is no runtime cost (see Verified: Cost).
- **Trigger Conditions**: any `phys.stats` read in a cell with sleeping dynamics, or with an active ragdoll.
- **Related**: D3-04 (rapier 0.36 extra-iteration behaviour around multibodies); D6-01 (the other `phys.stats` label rapier 0.36 changed).
- **Suggested Fix**:
  - Label the counts by scope, e.g. `narrow-phase pairs (all)=… awake-island contacts=… contact manifolds+impulse joints=…`.
  - Add the live multibody joint count (`articulation_joints.len()` after its sweep) next to them.
  - Fix the "zeroes only before the very first step" sentence.

---

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
