# #5536: PHYS-D3-2026-10-11-01: Every LimitedHinge limit is enforced mirrored relative to Havok — Skyrim knees and elbows, FNV/FO3 elbows and (once D3-02 is fixed) Oblivion knees and elbows can only bend backwards

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5536
- **Labels**: high,physics,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: HIGH
- **Dimension**: Ragdoll & Constraint Seam
- **Location**: `crates/physics/src/ragdoll.rs:705-730` (LimitedHinge flip and limits), `:667-693` (Ragdoll twist, same derivation), `:745-775` (Prismatic, same derivation), `:623-624` (`flip`)
- **Status**: NEW. Pre-existing: unchanged by `ac81a09f9`, and rapier's coordinate convention is identical in 0.22 and 0.36.
- **Trigger Conditions**: any ragdoll activation with a `bhkLimitedHingeConstraint` (or the hinge inside a malleable or breakable wrapper) that has asymmetric limits, i.e. every vanilla humanoid elbow, knee, ankle and wrist on Oblivion, FO3, FNV and Skyrim.
- **Description**: The convention is fixed by two sources.
  - **Havok** measures the hinge angle of A (attached) relative to B (reference), right-handed about B's axle from B's perp. The 2007 SDK's own limit visualiser draws the `[min,max]` arc about **B's** axle starting at **B's** perp (`havok-20070919/…/Drawer/hkpLimitedHingeDrawer.cpp:28-41,65`). `hkDisplaySemiCircle::generatePoints` rotates with `setAxisAngle(normal, θ)` (`hkDisplaySemiCircle.cpp:41-55`), and A's perp is the moving indicator (`:84`). The ragdoll drawer uses the same B-reference (`hkpRagdollDrawer.cpp:41,58`).
  - **Rapier** gives `child = parent·frame1·R(θ)·frame2⁻¹`, so θ is the child's rotation relative to the parent.
  - **Non-flip edge** (A is the parent): θ = −θ_Havok, yet the limits pass through unchanged (`:705-708`).
  - **Flip edge** (A is the child): θ = +θ_Havok, yet the limits are negated and swapped to `[-max, -min]` (`:709-719`).
  - Both branches therefore enforce the mirror of the authored range.
- **Evidence**: Probe, bind-pose seeded coordinate versus the enforced range, together with which way +θ moves the child relative to the skeleton's forward (taken from the feet):

  | Joint | +θ moves child | Engine range | Effect |
  |---|---|---|---|
  | SSE calf | forward (= hyperextension) | `[0, 1.920]` | knee bends **backward** up to 110°, flexion forbidden |
  | SSE forearm | forward (= flexion) | `[-1.920, 0]` | elbow can only hyperextend |
  | FNV/FO3 forearm | backward (= hyperextension) | `[0, 2.269]` | elbow bends **backward** up to 130° |
  | Oblivion calf and forearm (perp derived, D3-02) | — | `[-2.094, 0]` | only hyperextension |

  Under the Havok convention every one of these becomes a flexion-only range. Bind-pose census:
  - FNV/FO3: all 8 hinges are inside the authored range under Havok's convention; 4 violate under the engine's (knees by 0.348 rad, elbows by 0.076).
  - SSE: all 8 are inside under Havok's convention; 6 violate under the engine's (feet by 0.435, knees by 0.054, elbows by 0.019).
  - One counter-case: Oblivion knees seed at −0.104, 6° past 0, under Havok's convention.
- **Impact**: Every ragdoll's elbows and knees fold the wrong way, a visible defect on every corpse. The bind or animated pose also starts outside the mirrored range, which feeds the first-solve velocity burst in D3-03. No test can see it: the only limit test compares against `build_joint` itself with symmetric limits and `flip=false`.
- **Related**: D3-02, D3-03, #2448 (closed: zero-reference, not sign), #1982.
- **Suggested Fix**: Invert the flip logic so the authored range applies to θ_Havok: negate and swap on the **non-flip** branch, pass through unchanged on the flip branch. Apply the same to Ragdoll twist and Prismatic travel. Pin it with a test that seeds a hinge child forward/backward of an asymmetric `[0, max]` range on both flip orientations, plus the probe's bind-pose census over the FNV and SSE skeletons.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
