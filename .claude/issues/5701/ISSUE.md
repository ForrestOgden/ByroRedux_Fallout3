# #5701 — PHYS-D3-2026-10-12-02: `physal.md` §4 still describes soft limits and a raised ragdoll solver budget as future work; `physics.md` omits the dry-link damping pass

**Labels**: `low`, `physics`, `documentation`, `doc-rot`

**Source**: `docs/audits/AUDIT_PHYSICS_2026-10-12.md` (PHYS-D3-02) · audited at HEAD 5d31fd637

- **Severity**: LOW (doc rot)
- **Dimension**: Ragdoll & Constraint Seam
- **Location**: `docs/engine/physal.md:250-253` (solver iteration count and joint-limit softness "staged as the limits get refined") and `:236-237` ("limit-fidelity refinement ... are follow-ups"); `docs/engine/physics.md:113` (damping described as set on the body at registration)
- **Status**: NEW

## Description
#5538 shipped soft limits (`RAGDOLL_JOINT_SOFTNESS`, 30 Hz, zeta 1) and #5536 fixed limit fidelity. #5546 deliberately set the ragdoll solver budget to the world default (`RAGDOLL_EXTRA_SOLVER_ITERATIONS = 0`), so "ragdolls want more than clutter" is now the opposite of the code. `physics.md` Phase 1 says `linear_damping` / `angular_damping` are applied from `RigidBodyData`, with no mention that rapier ignores them on links and `step` re-applies them every stepping frame through `apply_multibody_link_damping` (which also wipes other user forces, see PHYS-D2-2026-10-12-01). The doc-side rot is not captured by NIFAL-D6-2026-10-12-01, which covers naming only.

## Impact
A contributor reading section 4 would re-raise the iteration count (a measured 2x whole-world cost) or re-implement soft limits.

## Related
#5538, #5546, #5540.

## Suggested Fix
Rewrite the two bullets as "done" with the #5538 / #5546 rationale, and add a sentence on the dry-link pass to `physics.md`.

## Completeness Checks
- [ ] **SIBLING**: grep `docs/engine` for other "follow-up" / "staged" ragdoll-solver claims
