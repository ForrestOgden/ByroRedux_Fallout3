# #5546: PHYS-D3-2026-10-11-04: rapier 0.36 made the ragdoll's `additional_solver_iterations(12)` global — while any ragdoll body is awake, every awake dynamic body in the world solves 16 substeps (measured about 2× step cost)

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5546
- **Labels**: medium,physics,performance,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: MEDIUM
- **Dimension**: Ragdoll & Constraint Seam
- **Location**: `crates/physics/src/ragdoll.rs:390-395` (comment: "Scope the extra work to ragdoll … islands rather than … the whole world"); rapier 0.36 `staged_island_solver/init.rs:57-67,94-111` (`multi_group = solve_groups.len() >= 2 && multibodies.iter().next().is_none()`: with any multibody present, a single group runs at `max_extra` across the whole awake island); `island_manager/manager.rs:36-38` (all awake bodies share one island); `docs/engine/dependency-update-followups-plan.md:505-509` (the "ruled out" entry checks only the ragdoll's own substep count)
- **Status**: NEW — regression from `ac81a09f9`
- **Trigger Conditions**: any awake ragdoll body (a settling corpse) while other dynamic bodies, such as clutter, are awake anywhere in the world.
- **Description**: rapier 0.22 kept islands per connected component, so the +12 stayed local. In 0.36 every awake body lives in one island. Substep groups would localise the extra iterations, but the staged solver turns groups off whenever the multibody set is non-empty. Every ragdoll is a multibody.
- **Evidence**: The `leak` probe drops an unrelated ball (y = 60, restitution 0.6) onto a fixed floor while a 2-link ragdoll falls 2000 BU away; there is no contact and no joint.
  - At HEAD, the ball's trajectory with the ragdoll at +12 is bit-identical to the ball alone at +12 (16 substeps: `…4.9209 16.2341…`). With the ragdoll at +0 it matches the ball alone at 4 substeps (`…4.6500 15.9419…`).
  - At `ac81a09f9^` (0.22), the ragdoll at +12 matches the 4-substep run (local).
  - Cost with 500 unrelated awake boxes: 0.33 ms/tick with the ragdoll at +0 and 0.64 ms/tick at +12 (two runs each).
- **Impact**: Each settling corpse roughly doubles the whole simulation's step cost for several seconds. Unrelated bodies also behave differently depending on whether a ragdoll is awake, because their contact substep count changes. The guard named for this property is vacuous (see the guard table).
- **Related**: D3-03 (soft limits may let the +12 be lowered), Dim 6 cost.
- **Suggested Fix**: Re-measure whether ragdolls still need +12 once limits are soft (D3-03). If they do, accept the cost knowingly, or keep it to the activation window by dropping to 0 once the rig is near rest. Correct the plan-doc entry and the ragdoll comment, and replace the guard's field check with an unrelated-body trajectory check like the probe.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
