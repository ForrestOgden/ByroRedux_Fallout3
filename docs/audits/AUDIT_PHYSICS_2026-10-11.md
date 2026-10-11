**HEAD**: `65cadde2f` (physics-scope files identical to `455b473a1` / `c43c25624`) · **Baseline**: `docs/audits/AUDIT_PHYSICS_2026-10-09.md` (HEAD `3bcf6c8e8`) · **Audited**: all six dimensions (Dim 1 Shape Translation, Dim 2 Step & Sync, Dim 3 Ragdoll & Constraint Seam, Dim 4 Character & NPC Controller, Dim 5 Water / Buoyancy, Dim 6 Queries & Diagnostics) · **Unchanged since baseline (skimmed)**: none. Every dimension had commits, because the rapier 0.22 → 0.36 migration touched all of them.

# PHYSAL / Physics Audit — 2026-10-11

**Run**: `/audit-physics`, standalone, with one sub-agent per dimension (at most three at a time). They shared a rules file covering dedup, build constraints and output format. The orchestrator merged their output into this report and re-checked the highest-impact claims itself.

**Delta audited**: `3bcf6c8e8..65cadde2f`. The physics-relevant commits are the 2026-10-10 dependency refresh and its follow-ups:
- `ac81a09f9`: rapier 0.22 → 0.36, parry 0.31, glam 0.33.
- `04cb6333d`: GeometricMean contact combine.
- `738ab7391`: pins the KCC at-rest grounded verdict.
- `52de1bc0a`: re-enables quarantined bodies one step later.
- `1efdfda4a`: #5352.
- `e128218eb`: #5353.
- `9d826272b`: #5531.
- `002fbd6ef`: #4134.
- `4739c411a`: #5453.
- `e25d89d17`: contact counts in `phys.stats`.
- `886edcded` + `441d90c2f` + `1291f370a`: Havok surface material → collider user-data → footsteps and the engine's first `ContactForceEvent` handler (impact sounds).
- `f4c2b9931`: comment sweep.
- `26a1277bc`: #5355/#5356/#5272.
- `dd3a506e9`: #5357.

**Method**:
- Every API-semantics claim was checked against upstream source in the cargo registry (`rapier3d-0.22.0` against `rapier3d-0.36.1`, `parry3d-0.31.1`) and against the Havok SDK copies under `/mnt/data/src/reference/`. None was assumed from changelogs.
- Behavioural claims were measured with throwaway probe binaries linked against the repo's own built rlibs. A rapier-0.22 twin, built from `ac81a09f9^`, served as the A/B baseline for every "regression" verdict.
- No repo file was edited and no engine was launched.

**Lane counts at HEAD** (pinned toolchain 1.96.0):
- `-p byroredux-physics`: 220 passed.
- Bin filters: `ragdoll` 30 (+7 ignored), `character` 60, `locomotion` 6, `water` 140 (+4 ignored).
- 0 failures in the default lanes.
- **The FO3 real-data ragdoll gate is red.** `BYROREDUX_FO3_DATA=… cargo test -p byroredux --bin byroredux -- --ignored ragdoll::installed_tests` gives **2 passed, 4 failed**: `on_flat_floor`, `in_saloon_geometry`, `among_saloon_followers` and `after_physics_world_reuse`, all panicking at `ragdoll_installed_tests.rs:229` (body drift > 512 BU at tick 429–431). The orchestrator re-ran this independently (see PHYS-D3-2026-10-11-03).
- **The release test target of `byroredux-physics` does not compile** (PHYS-D1-2026-10-11-05).

---

## Executive Summary

| Severity | NEW | Regression | Existing (matched, cited) |
|---|---:|---:|---:|
| CRITICAL | 0 | 0 | 0 |
| HIGH | **5** | 0 | 0 |
| MEDIUM | **8** | 0 | 3 (#5354, #4772, #5015) |
| LOW | **16** | 0 | 2 (#5530, #5452) |

The 29 NEW findings come from 30 raw dimension findings after one cross-dimension merge. PHYS-D3-2026-10-11-05 (Skyrim CRC32 material truncation) merged into PHYS-D1-2026-10-11-02. PHYS-D2-2026-10-11-04 had a both-0 half that duplicates PHYS-D1-2026-10-11-03; it is counted once, under D1-03.

Four of the five HIGHs are ragdoll or articulation correctness. They group into one story:

1. **The joint convention is wrong.** Every LimitedHinge, ragdoll twist and prismatic limit is enforced as the mirror of the authored Havok range, so knees and elbows can only bend backward (D3-01).
2. **Oblivion's hinge reference is wrong.** Its zero-reference perpendicular is synthesised instead of derived, so every Oblivion rig explodes and freezes on its first solve (D3-02).
3. **Rapier's default rigid joint limits amplify both.** They correct a seed that starts outside its limits within one 1/960 s substep. Together with authored restitution, the #5246 offence ladder detaches every joint. This is why the FO3 real-data gate is red, and it is the likely root cause of open #4772 (D3-03).
4. **Damping does nothing on articulations.** Rapier skips rigid-body damping for multibody links in both versions, so submerged damping (and authored bone damping) is inert and floating corpses diverge (D5-01).

The fifth HIGH is a rapier 0.35+ behaviour change: snap-to-ground runs on every horizontal move. NPC steps (not the player, which #2857 protects) sink into and through convex floors (D4-01).

The rapier 0.36 migration also changed four behaviours the follow-ups plan did not record:
- Begin-touch strong-wake wakes spawned-asleep clutter (D2-02).
- Extra solver iterations become world-wide while any multibody exists (D3-04).
- Zero-motion depenetration is now real code (D4-02).
- Idle kinematics sleep (D6-01).

Two of the plan's "Checked and ruled out" entries (lines 505-509 and 528) are therefore incomplete or wrong.

The surface-material and contact-event track (Wave 4) ships with its sound maps empty, so it plays nothing today. It has several defects that will surface the day IPDS/IPCT content lands:
- FO3/FNV packed meshes all read STONE (D1-01).
- Skyrim materials are cut to a byte (D1-02).
- The player's footstep probe hits its own capsule (D2-03).
- The impact drain drops 0-pairs and `break`s early (D1-03, D2-04).
- Streamed clutter would clatter on first wake (D2-02).

### PHYSAL doctrine verdict: **HOLDS on the solver side, one parse-boundary breach**

- Grepping `crates/physics/src` and `byroredux/src/ragdoll.rs` for `GameKind|bsver|NifVersion|is_skyrim|havok_scale|GameVariant` finds nothing (Dim 3).
- The new surface-material path adds no solver-side branch. It does, however, carry each game's raw Havok material value across the parse→canonical boundary untranslated (Oblivion/FO3 enum bytes; Skyrim CRC32 truncated). That is a per-game seam with no translation, which `docs/engine/physal.md` §3 forbids (PHYS-D1-2026-10-11-02). The fix belongs at the parse boundary, not in the solver.
- The constraint-CInfo seam has a defect inside it: Oblivion `Perp B1` is not derived (D3-02).

**Collision data traced**:
- FO3 real data: the installed-test fixture and the male skeleton, through `build_ragdoll` + `PhysicsWorld::step`.
- FNV, Oblivion and Skyrim SE male skeletons: same probe.
- Vanilla mesh-BSA block census for Oblivion, FO3, FNV and SSE (`nif_stats --tsv`, Dim 1).
- Skyrim water NIF collision (`water_mesh_probe`, Dim 5).
- FO4, FO76 and Starfield were not traced: articulation is blocked on the opaque `BhkSystemBinary`, which is reported correctly as "authored but opaque".

---

## Solver Invariant Matrix

| Invariant | State | Evidence |
|---|---|---|
| Fixed step + accumulator clamp + NaN/negative dt | **Verified** | `world/mod.rs:895-899`. `f32::max` drops NaN; `+inf` clamps to `MAX_SUBSTEPS*PHYSICS_DT`; the budget bail-out (`:1176-1179`) is the only wall-clock read. |
| Determinism (no HashMap order feeding the solver) | **Verified** | Per-body maps are point lookups. The quarantine cooldown and contact drain are Vecs in rapier report order (Dim 2). |
| Static-scene fast path | **Verified** | The rapier 0.36 `active_bodies()` holds kinematics too, but `has_awake_dynamics` filters to dynamics (`:625-629`). The docs claiming kinematics "never leave it" are false (D6-01; diagnostics only). |
| Wake discipline: spawn exemption (#3969) | **Drifted** | The code still spawns asleep with no `wake()`, but rapier 0.36 begin-touch strong-wake wakes every resting newcomer on its first real step (D2-02). |
| Wake discipline: wakes armed inside a substep | **Drifted** | The quarantine-cooldown wake is cleared by `step`'s post-loop `pending_wake = false`. A re-enabled body stays collider-less in a quiet scene (D2-01). |
| Query BVH freshness | **Verified** | Queued leaves are inserted on no-step frames, and `set_aabb` on a missing leaf inserts it (parry `bvh_insert.rs:524-536`). Under 0.36 a stale leaf can only cause a miss, because the exact test uses the live `co.position()` (Dim 6). `update_query_pipeline` never runs per frame, but the guard scans only `step` (D2-08). |
| Lock order / access declarations | **Verified for physics systems; drift in a consumer** | `physics_sync_system` collect→write discipline holds (`sync.rs:990,1057`). `footstep_system` omits its new `PhysicsWorld` read and nests `FootstepConfig` under `AudioWorld` (D2-06). The bin-test ABBA cycles are #5535 (`/audit-concurrency`). |
| Phase order (collect/register → push kinematic → buoyancy → step → pull) | **Verified** | `sync.rs:131-168`. Pull writes `Transform` (`:1448`). |
| Explosion recovery + the #5161/#5246 ladder | **Verified, one gap** | The snapshot precedes every step, the delta bound holds, the restore propagates via `set_aabb`, the clamp runs on recovery substeps (#5352), free-root DOFs take body caps (#5353), rigs are attributed (#5531), maps are pruned (#5355/#5272). The in-step cap is exactly 28 000 BU/s. Gap: the clamp has no body-type check (D2-05). |
| Rapier quarantine round-trip | **Partially drifted** | No same-frame re-enable; stale handles are generation-checked. In a quiet scene the re-enable is never processed (D2-01). |
| Ragdoll solver budget locality | **Drifted (regression)** | `additional_solver_iterations(12)` is world-wide while any multibody exists in 0.36 (D3-04). |
| Ragdoll joint-limit convention | **Wrong (pre-existing)** | Mirrored relative to Havok on both flip branches (D3-01). Oblivion `Perp B1` is not derived (D3-02). Default stiff limits explode out-of-range seeds (D3-03). |
| Damping on articulations | **Wrong (pre-existing)** | Rapier skips `RigidBody` damping for multibody links (`worker.rs:956-962`, orchestrator-verified). Submerged, authored and extra-angular damping are all inert on jointed rigs (D5-01). |
| Teardown | **Verified** | `RigidBodySet::remove` drops the body, colliders and both joint kinds (`rigid_body_set.rs:203-250`). `remove_ragdoll` goes through `remove_body`. The release tests assert counts. |
| Contact events (new) | **Bounded; consumer defects** | The dropped-receiver `send` is discarded; the drain is bounded per step; stale handles are safe. The consumer drops 0-pairs, `break`s at the first unmapped material, and plays crossings from rolled-back substeps (D1-03, D2-04). |
| Buoyancy force persistence across substeps | **Verified (no migration change); hold is lossy** | `user_force` persists in both versions and is not scaled by N. Holding it per frame adds energy, so thin floats never settle (D5-02). |

---
## Findings — HIGH

### PHYS-D3-2026-10-11-01: Every LimitedHinge limit is enforced mirrored relative to Havok — Skyrim knees and elbows, FNV/FO3 elbows and (once D3-02 is fixed) Oblivion knees and elbows can only bend backwards
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

### PHYS-D3-2026-10-11-02: Oblivion LimitedHinge's zero-reference `Perp Axis In B1` is never derived from the parsed `Perp B2 × Axis B`; the synthesized perpendicular puts bind poses 1.5–1.8 rad outside the limits and the Oblivion rig is restored and parked frozen on its first step
- **Severity**: HIGH
- **Dimension**: Ragdoll & Constraint Seam (parse→canonical seam)
- **Location**: `crates/nif/src/blocks/collision/constraints.rs:378-404` (`parse_oblivion` zeroes `perp_axis_in_b1` and keeps `perp_axis_in_b2`); `crates/nif/src/import/collision/ragdoll.rs:400` (`perp_b` comes from the zero b1); `crates/physics/src/ragdoll.rs:850-859` (`frame_rot` substitutes `any_perp`); `docs/engine/physal.md:171-174` ("behavior on that era is unchanged")
- **Status**: NEW
- **Trigger Conditions**: any Oblivion (NIF ≤ 20.0.0.5) ragdoll activation. The male skeleton has 7 LimitedHinges.
- **Description**:
  - nif.xml defines `Perp Axis In A2` as `Axis A × Perp Axis In A1`, and `Perp Axis In B2` as the same vector in B's space. So `Perp B1 = Perp B2 × Axis B` exactly, the same identity physal.md already uses to derive the Oblivion hinge's `Axis A`.
  - The parser reads `Perp Axis In B2` but leaves B1 zeroed. `frame_rot` then falls back to `any_perp(axis_b)`, an arbitrary zero-reference unrelated to A's authored one. That is #2448's defect, still live on Oblivion.
- **Evidence**:
  - **Identity holds on real data.** On FNV, which authors B1, the derived `B2 × Axis B` equals the authored B1 to about 1e-7 (e.g. authored `(0.1608982, -2.4e-7, 0.98697096)`, derived `(0.16089816, -2.4e-7, 0.98697084)`).
  - **Oblivion at HEAD.** Bind-pose hinge coordinates violate by 0.075, 1.720 ×2, 1.550 ×2 and 1.823 ×2 rad. Tick 0 shows `recov=(1, 18, 0)`, `joints=0`, `asleep=18`: all 18 bodies are rolled back, the articulation is detached and the rig is parked.
  - **With Perp B1 derived.** All seven hinges seed at about 0 (−0.104 to 0.000), with 0 recoveries, 0 clamps and 17 joints intact; the rig settles.
- **Impact**: Every Oblivion ragdoll explodes on its first solve and freezes rigid in its activation pose, with the joints detached. The physal.md claim that the zeroed value leaves "behavior on that era unchanged" is false.
- **Related**: D3-01 (once derived, Oblivion knees and elbows show the mirror), #2448, #4772.
- **Suggested Fix**: In `LimitedHingeCInfo::parse_oblivion`, set `perp_axis_in_b1 = perp_axis_in_b2 × axis_b`, the nif.xml identity, mirroring the hinge `Axis A` derivation. Correct physal.md:171-174 and add a byte-level parser test.

### PHYS-D3-2026-10-11-03: The FO3 real-data ragdoll gate is red (4/6): a bind-pose FO3 rig is torn apart by the explosion ladder within 30 ticks and its capsules roll more than 512 BU away. It was already red at yaw 0 before the rapier migration, and 0.36 fails both yaws
- **Severity**: HIGH
- **Dimension**: Ragdoll & Constraint Seam
- **Location**: `byroredux/src/ragdoll_installed_tests.rs:215-233` (assertion); `crates/physics/src/ragdoll.rs:654-778` (`build_joint`: no joint softness set, so rapier's `SpringCoefficients::joint_defaults()` applies, 1e6 Hz with ζ = 1, `integration_parameters.rs:78-83`, `generic_joint.rs:307,327`); `:395` (16 substeps); `world/recovery.rs` (third offence → `remove_multibody_articulations`)
- **Status**: NEW (Related: #4772 — this supplies the root cause #4772 asks for; consider publishing as a comment there)
- **Trigger Conditions**: An FO3/FNV humanoid ragdoll activated from bind pose. That covers restored corpses, the copied-save restore and Starts Dead actors on any floor.
- **Description**: The failure is a chain.
  1. The bind pose violates the enforced limits at activation: the knees by 0.348 rad and the elbows by 0.076 under the mirrored sign (D3-01), plus the neck's swing by 0.230 and its twist by 0.011 under the cone-on-both-axes approximation.
  2. Rapier's default joint-limit stiffness corrects a violation in **one substep**: `erp_inv_dt ≈ 1/dt_sub` (`unit_multibody_joint.rs:29-32`), with `dt_sub = 1/960 s` at 16 substeps. The result at tick 0 is 617 BU/s and 100 rad/s, and the clamp fires.
  3. The authored restitution of 0.8–0.9 bounces the rig off the floor, which earns further offences.
  4. The third offence detaches all 17 joints. The free capsules then drift and roll, crossing 512 BU at about tick 431.
- **Evidence**: Probe matrix on the FO3 skeleton and flat floor:

  | Variant | Result |
  |---|---|
  | HEAD | `clamps=4 detaches=1 joints=0` by tick 30; worst body 511.6 BU at tick 430, 515.2 at tick 435 (test: tick 431) |
  | Restitution 0 | joints intact, settles |
  | Hinge sign fixed only | still detaches (neck) |
  | Per-joint softness 30 Hz or 120 Hz (`GenericJoint::softness`, a per-joint field rapier 0.36 adds) | 0 clamps, 17 joints, all 18 bodies asleep by tick 570, both yaws; Oblivion (and SSE) settle too |
  | Pre-migration (`ac81a09f9^`, rapier 0.22), yaw 0 | detaches by tick 60 and crosses 512 BU at about tick 485–510, so `on_flat_floor` was already red |
  | Pre-migration, yaw −π/2 | settled |
  | HEAD, yaw −π/2 | detaches by tick 60 |

  The gate went unobserved: it is `#[ignore]`d, four physics audits (09-21, 09-29, 10-05, 10-09) recorded "not run". This run's first attempt used a filter (`ragdoll_installed`) that matches 0 tests and exits green; the module path is `ragdoll::installed_tests`.
- **Impact**: Every FO3/FNV restored or Starts Dead humanoid corpse comes apart within about half a second and its limbs scatter. The one real-data articulation gate is red on HEAD.
- **Related**: D3-01, D3-02, #4772, #5246 (offence ladder), known-open "mass-inverted rigs / authored restitution 0.8" register entry.
- **Suggested Fix**:
  - Give ragdoll joints Havok-like soft limits through rapier 0.36's per-joint `softness` (e.g. `GenericJointBuilder::softness(SpringCoefficients::new(30–120, 1.0))`; measure). This replaces 0.22's global `joint_natural_frequency`, so the limits pull a violating seed in over frames instead of in one substep.
  - Land D3-01 and D3-02.
  - Record the exact gate command in the skill (`-- --ignored ragdoll::installed_tests` with `BYROREDUX_FO3_DATA`) so the gate is actually run.

### PHYS-D4-2026-10-11-01: rapier 0.36's snap-on-every-horizontal-move sinks walking NPCs into convex floors, and through thin ones; the KCC then reports `grounded`, so the clamped fallback ray never corrects it
- **Severity**: HIGH
- **Dimension**: Character Controller, Grounding & the NPC KCC (M42.10)
- **Location**: `byroredux/src/systems/locomotion.rs:183-225` (NPC step: purely horizontal `desired`, snap 64 BU,
  fallback gated on `!result.grounded`); `crates/physics/src/world/queries.rs:667-671` (snap passed through);
  `locomotion.rs:72-76` (`LOCOMOTION_NPC_SNAP_TO_GROUND`)
- **Status**: NEW (regression introduced by `ac81a09f9`; the player-side analogue is closed #2857, HIGH)
- **Description**: The NPC step asks the KCC for a purely horizontal translation (`locomotion.rs:183-188`, `y = 0`)
  from a capsule whose bottom sits exactly at the feet (`position = feet + hh + r`, `locomotion.rs:178,192`) —
  i.e. *inside* the 4 BU offset band of the floor. Under rapier 0.22 snap-to-ground only fired for
  `translation·up < -1e-5` (0.22 `character_controller.rs:371`), so a horizontal NPC step never snapped. Since 0.35
  it fires on `<= 0.0` (0.36 `:466`) — every horizontal NPC step from a grounded start now runs the snap cast, a
  `-up` shape cast with `target_distance = offset` from a configuration already inside the target band. Against a
  *convex* (support-map) floor that cast intermittently returns a large bogus time of impact — the exact parry
  failure #2857 documented for the player ("in that configuration parry's cast against a CONVEX primitive frequently
  reports no interference", `character.rs:386-399`) — and snap applies it (`result.translation -= up * toi`,
  `grounded = true`, 0.36 :481-482). Because 0.36 now also reports `grounded`, the `!result.grounded` fallback
  (`locomotion.rs:205-225`, the only thing that re-grounded NPCs under 0.22 — it fired on 335–409 of 600 frames
  there) never runs, so the sink persists until a later sweep pops the capsule back up. The player is immune only
  because #2857 gave *it* a probe-bounded vertical request (`character.rs:418-477`); the NPC step never got one,
  and the #2857 guard tests only the player policy.
- **Evidence** (probes; same harness both versions; NPC params = production: hh 32, r 20, offset 4, snap 64, step
  32/8, 100 BU/s, 60 Hz; cuboid floor slabs; production airborne fallback replicated):
  - Sweep of 42 floor heights (#2857's set: 0, 137, −60 + 7.3·i) × slab thickness {20, 4} × 4 headings, 240 frames:
    **0.36: NPC sank > 1 BU in 82/336, went below the slab's bottom face in 63/336**; 0.22: 54/336 and 10/336.
    0.36 with `snap_to_ground = 0`: **0/336 and 0/336**. Player replica (production grounded branch) on the same
    floors: 0/336.
  - Sunk on 3 626 of 80 640 NPC-frames; worst configuration sunk 193/240 frames; 13 configurations still sunk at
    the end of the 4 s window.
  - Trace (20 BU slab, diagonal): `f58: 3.821 → -12.433 (kcc dy -16.254, grounded true, fallback false)`, sinking
    to −20.18 (the slab's bottom face is −20) until `f89: kcc dy +24.180`. 8 BU slab: feet reach −53.4; 4 BU slab:
    −35.4. Trimesh floors: no sink (feet stay within [0, 4]).
- **Impact**: NPCs walking on box / convex-hull walkable collision (`BhkBoxShape`, `BhkConvexVerticesShape` parts,
  the FO4+/Starfield `PackedAabbProxy` cuboids on Clutter/Actor placements, `cell_loader/spawn.rs:58-72`) dip
  knee- to waist-deep for ~0.7 s at a time and pop back up; through thin convex floors (planks, catwalks) they drop
  below the floor entirely. Visible on every game that uses convex walkable shapes; direction- and absolute-height
  dependent (float precision of the resting gap, as in #2857), so it reads as intermittent. Content prevalence of
  convex walkable floors was not measured.
- **Trigger Conditions**: any NPC whose Wander/Patrol/Travel/Follow/Escort/Guard/combat/force-greet/Eat-Sleep step
  runs `step_toward(_detailed)` with a `PhysicsWorld` on a convex floor collider, grounded at the start of the step.
- **Related**: #2857 (closed, player analogue, HIGH), #3799, `738ab7391` (re-checked grounding but not snap on
  horizontal moves), #5452 (same mover path, different defect)
- **Suggested Fix**: Give the NPC step the player's #2857 treatment — measure the gap with
  `cast_capsule_down_surface_and_normal` and request exactly that vertical alongside the horizontal move — or pass
  `snap_to_ground: 0` for NPCs and let the existing clamped fallback ray handle downslopes (0/336 sinks measured).
  Add an NPC variant of `grounded_capsule_does_not_sink_through_convex_floors_across_absolute_y` driving the real
  `step_toward_detailed` parameters.

### PHYS-D5-2026-10-11-01: Submerged damping is inert on every jointed ragdoll body (multibody links ignore `RigidBody` damping), so a floating corpse's bob grows without bound instead of settling
- **Severity**: HIGH
- **Dimension**: WATAL Physics Sink (seam: Dim 3 ragdoll — same root cause)
- **Location**: `crates/physics/src/water.rs:951-952` (submerged `set_linear_damping` / `set_angular_damping`), `:1024-1025,1037-1038` (restore), `:524-525` (`clear_stale_water_contacts` restore); `crates/physics/src/ragdoll.rs:385-399` (bone damping at build); upstream `rapier3d-0.36.1/src/dynamics/solver/staged_island_solver/worker.rs:956-974`, `rapier3d-0.22.0/src/dynamics/solver/velocity_solver.rs:295-312`
- **Status**: NEW. Present since #3492 (2026-08-27) made ragdolls reach the sink; not a migration regression, because both rapier versions skip rigid-body damping for multibody links.
- **Description**:
  - The buoyancy pass makes a float settle with submerged viscous damping: `b.set_linear_damping(1.5)` / `set_angular_damping(1.5)` (`water.rs:951-952`). Its own docs say this lets the body "settle (and can then sleep) rather than bobbing forever".
  - `build_ragdoll` joins every bone body with multibody joints (`Ragdoll.joints: Vec<MultibodyJointHandle>`). In rapier, every body of an articulation, root included, is a multibody link.
  - Rapier applies `RigidBody` damping only on the rigid-body writeback path. That path `continue`s for `multibodies.rigid_body_link(handle).is_some()` (`worker.rs:956-962`), and the damping call is at `:974`. Links take their velocities from the multibody's generalized solve, whose damping is the separate per-DOF vector (`multibody.rs:621`): 0.1 on angular DOFs, 0 on linear (`multibody_joint.rs:273-285`). 0.22 has the same split (`velocity_solver.rs:295-312`).
  - So for a ragdoll the submerged damping, the authored bone damping and the restore are all stored on the body but never applied. The vertical heave of a floating corpse has no damping at all.
  - Undamped, the per-frame zero-order-hold buoyancy (PHYS-D5-2026-10-11-02) injects energy every cycle, and ragdolls hold the force over 16 internal small steps per substep. The oscillation grows.
- **Evidence**: probe `/tmp/claude-1000/-mnt-data-src-gamebyro-redux/95e852db-3454-4021-b6be-d5acf8770538/scratchpad/probe/rag.rs (siblings zoh.rs, wave.rs, aabb.rs)`, linked against the repo's built `byroredux_physics` rlib. It runs the production `build_ragdoll` + `physics_sync_system`: two radius-8 balls, mass 10, calm water at y=0, 40 s simulated.
  ```
  jointed=false fps=60: multibody_link=false body.linear_damping=1.5 frac=0.64 last-5s p2p=0.00     asleep 299/299
  jointed=true  fps=60: multibody_link=true  ... last-5s y -1437.75..135.84 p2p=1573.59   asleep 0/299
  jointed=false fps=30: ... p2p=0.00 asleep 149/149
  jointed=true  fps=30: ... y -25004.24 (left the column through its floor, fell to the kill plane)
  ```
  The jointed trace at 60 fps (damping reads 1.5 while wet and is applied as 0):
  `t=1 s` peak depth 19 → `t=4.5` 35 → `t=9` 58 → `t=14.5` 87 BU, with launches out of the water to y=+55 between dives. The unjointed pair with identical shapes settles in ~2.5 s.
- **Impact**:
  - Any actor ragdolled in water: FNV Lake Mead, Skyrim rivers, slaughterfish and mudcrab kills.
  - The corpse hops in and out of the surface with growing amplitude until it strikes the lakebed or exits the volume. It never sleeps, so its island keeps the step awake forever.
  - Every exit and re-entry is a `WaterContact` dry→wet edge, producing a fresh splash event and splash sound (`systems/water.rs:411-541`).
  - Same root cause on the ragdoll side: the authored bhk bone damping and the `ContactConfig::ragdoll_extra_angular_damping` "less floppy than Havok" lever (`config.rs:114-117`) are inert on every jointed rig. Dim 3 lists the lever as verified ("added once per body"); it is added and then never applied. The pin test `recorded_damping_is_the_effective_value_build_ragdoll_applied` passes only because its fixture is a single unjointed body.
- **Trigger Conditions**: any ragdoll with at least one constraint, which is every vanilla rig, whose bodies reach a `WaterVolume`. It happens at any frame rate; lower rates diverge faster.
- **Related**: PHYS-D5-2026-10-11-02 (the energy source); #3492 (made ragdolls reach the sink); PHYS-D3-2026-10-11-04 (16 internal iterations).
- **Suggested Fix**: for multibody links, apply water drag as a force instead of rigid-body damping, `F = -m·c·v` added with the buoyancy force. Alternatively, write the submerged damping into the articulation's `Multibody::damping_mut()` for the free-root linear and angular DOFs, and restore it on exit. Apply the same treatment to `build_ragdoll`'s authored damping and the extra-angular lever. Pin it with a two-body jointed fixture.

## Findings — MEDIUM

### PHYS-D1-2026-10-11-01: FO3/FNV packed-tri-strip surface materials never reach the material walker, so every FO3/FNV packed collider reports STONE (0)
- **Severity**: MEDIUM
- **Dimension**: Shape Translation (surface-material plumbing)
- **Location**: `crates/nif/src/import/collision/shape.rs:151-160` (consumer). Parser: `crates/nif/src/blocks/collision/shape_mesh.rs:79-100`, `:122-135`, `:235-251`.
- **Status**: NEW. This is the consumer half that #2550 (CLOSED, capture-only) explicitly deferred.
- **Description**: `shape_leaf_materials_inner` (and therefore `first_shape_material`) takes a packed mesh's material from `BhkPackedNiTriStripsShape::sub_shapes`. nif.xml places that table on the shape only `until="20.0.0.5"` (nif.xml:3195-3196). From 20.2.0.7 it lives on `hkPackedNiTriStripsData` (nif.xml:3968-3969). The parser follows nif.xml exactly. For FO3/FNV, `BhkPackedNiTriStripsShape::parse` leaves `sub_shapes` empty (`// FO3+: sub-shapes are in hkPackedNiTriStripsData` → `Vec::new()`, shape_mesh.rs:98-100). It stores the authored table on `HkPackedNiTriStripsData::sub_parts`, whose doc says it was captured precisely for "the eventual footstep/impact consumer" (shape_mesh.rs:129-135). The new walker never reads `sub_parts`. `.find(|part| part.material != 0)` on the empty `sub_shapes` falls to `inherited`. The MOPP wrapper has no material and the rigid body carries none, so the result is 0.
- **Evidence**:
  ```rust
  // shape.rs:151-159
  if let Some(s) = block.as_any().downcast_ref::<BhkPackedNiTriStripsShape>() {
      let material = s.sub_shapes.iter()            // always empty on 20.2.0.7
          .find(|part| part.material != 0).map(|part| part.material)
          .unwrap_or(inherited);                    // → 0 under a MOPP root
  ```
  `grep sub_parts` shows only test fixtures outside shape_mesh.rs. No production reader exists.
- **Trigger Conditions**: any FO3/FNV static or clutter body whose collision is `bhkMoppBvTreeShape → bhkPackedNiTriStripsShape`. That is the dominant FO3/FNV static collision type: 5,634 FO3 and 7,468 FNV packed blocks in vanilla mesh BSAs. #2550 measured 3,232 FO3 packed meshes carrying more than one authored material.
- **Impact**: `RigidBodyData::surface_material` and every per-part byte are 0 on effectively all FO3/FNV architecture. `surface_material_under` therefore answers STONE for wood floors, metal catwalks, dirt and sand on the reference title. When the IPDS-backed `FootstepConfig::surface_sounds` / `ImpactConfig` maps land, FO3/FNV will play only the STONE entry. Nothing is audible today because both maps ship empty. The authored data is parsed and then dropped at the NIFAL boundary, which is the "translatable data silently dropped" class (MEDIUM floor).
- **Related**: #2550 (capture), PHYS-D1-2026-10-11-03 (0 = STONE conflation hides this: a 0 here is indistinguishable from authored stone).
- **Suggested Fix**: In the packed arm, resolve `s.data_ref` to `HkPackedNiTriStripsData` and use `sub_parts` when `sub_shapes` is empty (version-agnostic: take whichever table is populated). Add a fixture in the FO3 layout (empty shape table, populated data table) to the shape.rs tests.

### PHYS-D1-2026-10-11-02: Skyrim's CRC32 `SkyrimHavokMaterial` is truncated `as u8`, so 8 material pairs collide and the byte cannot be joined to MATT
- **Severity**: MEDIUM
- **Dimension**: Shape Translation (surface-material plumbing)
- **Location**: `crates/nif/src/import/collision/mod.rs:455-456`, `:465`. Contract doc at `crates/core/src/ecs/components/collision.rs:118-126`. Consumers: `byroredux/src/components.rs:1828-1831`, `:1876-1879`.
- **Status**: NEW
- **Description**: The core field doc says the per-game enums "share the STONE-is-0 start and fit a byte" (collision.rs:119-121). That holds for Oblivion (0-31) and FO3/FNV (0-127). It is false for Skyrim. `SkyrimHavokMaterial` is "CRC32 of the lowercase of the Creation Kit Material Name" (nif.xml:576-577), and 0 there is `SKY_HAV_MAT_NONE` while `SKY_HAV_MAT_STONE` is 3741512247. The import keeps only the low byte (`as u8`, mod.rs:456 and :465) of every Skyrim primitive's `material` and every `BhkCompressedMeshShapeData::chunk_materials[i][0]` (8,188 SSE compressed-mesh blocks).
- **Evidence**: Low byte of the 77 nif.xml Skyrim values. Eight pairs collide:
  - HEAVY_METAL / CLOTH → 163
  - BROKEN_STONE / HEAVY_WOOD → 71
  - GRAVEL / MATERIAL_BOTTLE_SMALL → 88
  - BARREL / MATERIAL_BONE → 20
  - CERAMIC_MEDIUM / STAIRS_BROKEN_STONE → 91
  - MATERIAL_CHAIN / SKIN_METAL_SMALL → 102
  - SKIN_SMALL / COIN → 62
  - AXE_1HAND / SKIN_METAL_LARGE → 203

  `SKY_HAV_MAT_STONE` lands on byte 55 and `SKY_HAV_MAT_DIRT` on 170. These are arbitrary values that overlap the Oblivion/FO3 enum ranges the `u8`-keyed maps are documented against.
- **Trigger Conditions**: any Skyrim (LE/SE) body with authored Havok material, which is effectively all Skyrim collision.
- **Impact**: The authored identity is lost at import. A Skyrim consumer resolves surfaces through MATT records keyed by the u32 Havok ID, and a byte cannot be joined back to MATT. The byte also merges metal with cloth and gravel with bottles. `RigidBodyData::surface_material: u8` is a saved column (FORMAT_MAJOR v35 → v36 was already spent on 886edcded), so widening it later costs another format bump. Nothing is audible today (maps empty).
- **Related**: PHYS-D1-2026-10-11-03. Independently found by Dim 3 as its D3-05, from the PHYSAL-doctrine side: the raw per-game Havok material crosses the parse→canonical boundary untranslated, a per-game seam `docs/engine/physal.md` §3 forbids. Merged here.
- **Suggested Fix**: Carry the material as `u32` end to end: `RigidBodyData::surface_material: u32`, `part_surface_materials: Box<[u32]>`, `user_data = u128::from(u32)`, readers returning `u32`. Alternatively, translate to a canonical engine surface enum at the NIFAL boundary, per game. Either way, correct the collision.rs:118-126 doc.

### PHYS-D2-2026-10-11-01: The quarantine cooldown's "arm one more step" wake is cleared by `step`'s post-loop `pending_wake = false`, so in a quiet scene a re-enabled body never re-enters collisions or queries
- **Severity**: MEDIUM
- **Dimension**: Step & Sync
- **Location**: `crates/physics/src/world/mod.rs:998-1000` (the clear), `:1104-1118` (cooldown re-enable + `wake()`), `:1141-1146` (quarantine + `wake()`); guard `:2627-2676`
- **Status**: NEW (introduced by `52de1bc0a`)
- **Description**:
  - Both cooldown wakes are armed inside `run_substep`. Back in `step`, any frame that ran a substep does
    `if steps > 0 { self.pending_wake = false; }`, which clears the wake the substep itself just armed.
  - The quarantine-detection wake (`:1145`) only *looks* honoured. The next frame steps because the disabled body is
    still in rapier's (stale) active set: rapier processes the disable next step (`quarantine.rs:136-141`), so
    `has_awake_dynamics()` is true.
  - The re-enable wake (`:1117`) has no such rescue.
    - After that step the body is out of the active set.
    - `set_enabled(true)` only flags `RigidBodyChanges::all()` (`rigid_body.rs:260-274`).
    - Its colliders stay `DisabledByParent` until a `pipeline.step` runs the user-change pass (`user_changes.rs:120-136`).
  - So in a quiescent scene (nothing awake, nothing moving) the parked body has no collider in the broad phase or the
    narrow phase. It stays that way until some unrelated motion arms a step.
  - Commit `52de1bc0a` and the field doc both promise the opposite: "arming one more step so the re-enable is processed
    even if the scene is otherwise asleep … sits out of collisions and queries for that one step".
- **Evidence**: Probe (rapier-quarantined NaN-force ball, nothing else in the world):
  ```
  step1 substeps=1 quarantined=[h] enabled=false pending_wake=false   ← :1145 wake already cleared
  step2 substeps=1 enabled=true col_enabled=false pending_wake=false active=0   ← :1117 wake cleared
  after 120 frames: substeps=0 col_enabled=false ray_hit=None          ← straight-down ray misses the body
  after unrelated wake: substeps=1 col_enabled=true ray_hit=Some(... distance: 18.0)
  ```
  - The existing guard says "The cooldown armed a step even though nothing else is awake" (`:2654`). That is false
    attribution: its second step comes from the stale active set.
  - The guard asserts `body.is_enabled()`, which is the flag, not the collider. It stays green with both `wake()`
    lines deleted.
- **Impact**:
  - A corpse limb or clutter item recovered from a rapier quarantine stays invisible to `cast_ray`,
    `cast_ray_corridor`, `capsule_overlaps_solid` and the KCC sweep, and generates no contacts, until something
    else steps.
  - Concretely, a ragdoll that explodes and is fully parked while the player stands still cannot be targeted by the
    activation or loot ray, or hit by melee, until the player moves. The player's kinematic push wakes only on a
    move larger than 0.01 BU (`sync.rs:104-109`).
  - The bug self-heals on the next real step, hence MEDIUM rather than HIGH.
- **Trigger Conditions**: rapier quarantines a body (a non-finite solve, e.g. a ragdoll explosion), and afterwards no
  dynamic body is awake and nothing calls `wake()` (player idle, no animated keyframed bodies, no buoyancy change).
- **Related**: #5488, the `52de1bc0a` follow-up, #2856 (the same "wake consumed before work happens" class, fixed for
  external wakes only).
- **Suggested Fix**:
  - Re-arm after the loop rather than inside the substep. For example, set
    `self.pending_wake = !self.quarantine_cooldown.is_empty() || reenabled_this_frame` where the post-loop clear
    happens, or keep a separate `wake_requested_by_substep` flag that survives the clear.
  - Extend the guard: after the re-enable step, a third `step` must return 1, the collider must be `is_enabled()`, and
    a ray must hit the body.

### PHYS-D2-2026-10-11-02: rapier 0.36's begin-touch strong-wake wakes every authored-resting dynamic newcomer on its first step, so the spawn-asleep contract no longer holds for clutter
- **Severity**: MEDIUM
- **Dimension**: Step & Sync
- **Location**: `crates/physics/src/sync.rs:1086-1100` (spawn-asleep), upstream `rapier3d-0.36.1/src/geometry/narrow_phase/contacts.rs:363-369` (`strong_wake_sleeping_side` on every begin-touch); `crates/physics/src/world/mod.rs:639-657` (`wake()` doc contract)
- **Status**: NEW (behaviour change from the `ac81a09f9` migration; not in the plan's "Checked and ruled out" list)
- **Description**:
  - `register_newcomers` spawns every `MotionType::Dynamic` body `sleeping(true)`. Its rationale: "placed clutter is
    authored resting, so asleep is the correct spawn state".
  - Rapier 0.36 applies "Wake rule (whole-island sleep): starts wake unconditionally" (`contacts.rs:363-368`). The
    first `pipeline.step` after registration runs the narrow phase over the newcomer's colliders, because they are
    modified colliders (`contacts.rs:83-94`). Every newcomer touching anything within skin plus prediction distance
    produces a begin-touch transition and is strong-woken.
  - On rapier 0.22 the same transition only emitted collision events (`rapier3d-0.22.0/.../narrow_phase.rs:1080-1088`).
    A brand-new collider had no graph index for the `handle_user_changes_on_colliders` wake (`:434-456`), so an
    asleep newcomer resting on fixed ground stayed asleep.
  - The fast path still serves the streaming frame itself. The first frame that steps for any other reason (in an
    exterior the walking player arms one every frame) then wakes all resting clutter of the new cells. It stays awake
    for rapier's sleep timer (~1 s), and the contact skin lifts each item about 1 BU.
  - Each item also produces a `started` contact-force crossing carrying its settle impulse.
- **Evidence**: Probe through the production path (`physics_sync_system`; static floor; 50 boxes of mass 10 streamed
  in resting on it; then one unrelated `wake()`):
  ```
  after streaming frame: awake dyn=(0, 0) pending_wake=false     ← spawn-asleep holds for the streaming frame
  frame 0:  awake(dyn,kin)=(50, 0)                               ← first real step wakes all 50
  frame 6:  started_events=50 max_force=60246                    ← 50 high-force crossings (static load 6 867)
  frame 60: awake(dyn,kin)=(0, 0)
  ```
- **Impact**:
  - **Solver cost on every streaming boundary.** Every cell crossing pays ≥1 s of solver work over all resting dynamic
    clutter in the new cells, and the static-scene fast path is off for that window. The cost is bounded per frame by
    `SUBSTEP_TIME_BUDGET`. It has not been measured in a release build.
  - **Impact audio, once IPDS/IPCT content lands.** The `ImpactConfig` floor (`min_force_magnitude` 5 000) is far
    below these crossings. Every streamed or loaded cell would then play a burst of impact sounds, one per resting
    item, at load.
  - **Doc drift.** The `wake()` doc (`world/mod.rs:639-657`) and `docs/engine/watal.md` §0.1 still describe newcomers
    as staying asleep until touched.
- **Trigger Conditions**: a cell containing dynamic (Havok `MO_SYS_DYNAMIC`/box-inertia) clutter whose collider rests
  within contact-skin + prediction distance (≈ 2×1 BU skin + 1.4 BU) of another collider at spawn, which is the
  authored norm. Every game with dynamic clutter.
- **Related**: #3969 (spawn exemption contract), #1698 (settle storm), D2-04 (impact consumer)
- **Suggested Fix**:
  - Restore the intent explicitly. Track the dynamic handles spawned asleep in a `spawn_sleep_pending` list. After the
    first `pipeline.step` that processes them, `sleep()` each one still within the drift threshold of its spawn pose.
    Manual `sleep()` pins eligibility (`rigid_body_components.rs:1451-1457`), so the island sleeps on the next update.
  - Either exclude those first-step crossings from `pending_force_events`, or gate impacts on Δv (force/mass) rather
    than raw force.
  - Add a production-path test asserting zero awake dynamics after the first post-stream step.

### PHYS-D2-2026-10-11-03: The surface-aware footstep probe starts inside the player's own capsule and never excludes it, so every player footstep reads material 0
- **Severity**: MEDIUM
- **Dimension**: Queries & Diagnostics (found under Dim 2's footstep-surface scope)
- **Location**: `crates/physics/src/world/queries.rs:220-230` (`surface_material_under` — `solid_probe_filter()` with no exclusion parameter); caller `byroredux/src/systems/audio.rs:262-270`; emitter on the body `byroredux/src/scene.rs:1162-1180, 1245`
- **Status**: NEW (introduced by `886edcded`; the same self-hit class as closed #2859)
- **Description**:
  - In character mode the `FootstepEmitter` is on the player body (`scene.rs:1245`, #5146). The trigger position is
    that body's `GlobalTransform.translation`, which is the capsule centre.
  - The player capsule is `CharacterKinematic`, i.e. rapier `KinematicPositionBased`, with `surface_material: 0`
    (`scene.rs:1171-1180`). It is not in `ACTOR_BONE_GROUP`.
  - `surface_material_under` casts a solid ray with `solid_probe_filter()`, which excludes dynamics, sensors and actor
    bones but **not kinematics**. It has no `excluded_body` parameter at all.
  - A solid ray that starts inside the capsule returns `toi = 0` on the capsule itself. That is exactly the #2859
    failure `cast_ray_down`'s doc warns about (`queries.rs:178-186`: "`excluded_body` must be passed whenever the
    origin can lie inside a body — the player capsule").
- **Evidence**: Probe (production registration; floor `surface_material: 7`; player capsule half-height 46, radius 18,
  centre 64 above the floor):
  ```
  from the body centre (footstep_system's origin): Some(0); same height 100 BU aside: Some(7)
  ```
  The only test, `surface_material_under_reads_the_hit_colliders_material` (`world/mod.rs:3260`), casts from open air
  with no player body.
- **Impact**:
  - The surface lookup is live every footstep, but its answer is always STONE for the player, which is the only
    emitter that fires (#5348).
  - Once `FootstepConfig.surface_sounds` is populated, the player's steps play the stone sound (or the default) on
    every surface: the per-material footstep feature is dead on arrival.
  - It also wastes the per-trigger ray.
- **Trigger Conditions**: character mode (any game), any footstep.
- **Related**: #2859 (closed: the same self-hit in the fog-height probe), #5146, #5348
- **Suggested Fix**:
  - Give `surface_material_under` an `excluded_body: Option<RigidBodyHandle>` like `cast_ray_down`, with the same
    mandatory-decision rationale.
  - In `footstep_system`, resolve the body's `RapierHandles.body` (in the pre-resolved `body_gate` block, to keep the
    #5305 lock discipline) and pass it.
  - Add a test with a capsule around the origin.

### PHYS-D3-2026-10-11-04: rapier 0.36 made the ragdoll's `additional_solver_iterations(12)` global — while any ragdoll body is awake, every awake dynamic body in the world solves 16 substeps (measured about 2× step cost)
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

### PHYS-D5-2026-10-11-02: Buoyancy is re-derived once per render frame and held constant across every substep and solver iteration, so thin floats never settle and the threshold scales with frame rate
- **Severity**: MEDIUM
- **Dimension**: WATAL Physics Sink
- **Location**: `crates/physics/src/sync.rs:152` (one `apply_buoyancy` per `physics_sync_system` call), `crates/physics/src/water.rs:968-1004` (force derived from this frame's `frac`, then `add_force`), `crates/physics/src/world/mod.rs:976-986` (up to 5 substeps with the same `user_force`)
- **Status**: NEW (pre-dates the migration; 0.22 and 0.36 have identical force persistence and 4 internal iterations)
- **Description**:
  - Lift is a spring on the submerged fraction, with stiffness 1.67·m·|g|/h and ω² = 1146/h for an AABB height of h BU.
  - It is sampled once per frame, before the step, and held over N engine substeps × K internal small steps (K=4, or 16 while a ragdoll is awake), with no re-sampling.
  - Holding a spring force constant over M = N·K integration steps adds energy each frame: the linearized per-frame growth is det = 1 + ω²(h_s)²·M(M−1)/2. The only loss is the submerged damping of 1.5/s.
  - Bodies whose h falls below a frame-rate-dependent threshold therefore enter a limit cycle bounded only by the fraction clamp, bouncing out of the water and back. The estimated thresholds (≈4.8 BU at 60 fps, ≈11 BU at 30 fps, ≈28 BU at 12 fps) match the probe.
- **Evidence**: probe `zoh.rs` (production `physics_sync_system`, calm water, ball released slightly below equilibrium, 30 s, last-3 s range):
  ```
  h=4  60fps p2p 5.7  never sleeps | 30fps p2p 20.0 | 12fps p2p 64.0
  h=6  60fps sleeps                | 30fps p2p 19.3 never sleeps
  h=10 60fps sleeps                | 30fps p2p 11.6 never sleeps | 20fps p2p 30.7
  h=20 60/30/20fps sleep           | 12fps p2p 45.8 never sleeps
  h=30 sleeps ≤20fps               | 12fps p2p 9.2
  ```
- **Impact**:
  - Thin clutter (plates, books, coins and cutlery lying flat, a few BU tall) bobs forever, even at 60 fps.
  - At 30 fps, anything under ~10 BU tall leaps in and out of the water. Each cycle emits a dry→wet splash edge, and the body's island never sleeps, which defeats the settled-water fast path.
  - The water response depends on render frame rate, against the fixed-60 Hz-tick contract (`physics.md` "Fixed tick").
- **Trigger Conditions**: a dynamic body with collider AABB height under the threshold for the current frame rate, inside a `WaterVolume`.
- **Related**: PHYS-D5-2026-10-11-01 (the same energy source drives the corpse divergence), PHYS-D3-2026-10-11-04
- **Suggested Fix**:
  - Re-derive buoyancy per fixed tick inside the substep loop, for example as a pre-substep callback, so N stops multiplying the hold.
  - Bound the remaining K-step hold by flooring the effective height in `submerged_fraction`, e.g. `h_eff = max(h, k·|g|·PHYSICS_DT²·…)`. Alternatively, add heave damping proportional to the spring stiffness.
  - Pin it with a thin-body settle test driven at 30 fps.

### PHYS-D5-2026-10-11-03: The swimming player takes harmful-water damage twice per frame, from the controller and from `water_damage_system`, since the player gained `ActorVitals` and publishes its own `WaterContact`
- **Severity**: MEDIUM
- **Dimension**: WATAL Physics Sink (two samplers / damage)
- **Location**: `byroredux/src/systems/character.rs:618-627` (controller applies `water_damage_for_contact` while swimming); `character.rs:1332-1341` (`sync_player_water_contact` publishes `damage_per_second` + `submerged_fraction` on the player); `byroredux/src/systems/water.rs:21-65` (`water_damage_system` damages every `WaterContact` row with `ActorVitals`, with no player exclusion)
- **Status**: NEW. It is an interaction between `4c9a5b368` (2026-09-09, the player publishes `WaterContact`) and the #4458 fix (closed 2026-09-19), which stamps `ActorValues` + `ActorVitals` on the player through `inventory::attach_to_player` (`inventory.rs:762-763`, `scene.rs:1219`). Before #4458, `water_damage_system`'s `vitals_q.get(entity)?` skipped the player.
- **Description**:
  - `water_damage_system`'s doc says "The kinematic player has its own controller path; this covers dynamic NPC/creature bodies". Its filter is only `WaterContact` + `ActorVitals` + not `Dead`.
  - The player now matches. In the same frame, `character_controller_system` (Stage::Early) applies `dps × fraction × dt` when `swim` is set, and `water_damage_system` (Late) applies `dps × fraction × dt` again from the published contact.
  - Swimming in hazard water therefore costs 2× the authored rate. Wading below swimlevel costs 1×, but only from the Late system, so the controller's swim-only gate is not the rule that actually holds.
  - Meanwhile the doc's intended target class is empty in production. NPC actors are `CharacterKinematic`, which the buoyancy scan skips (`water.rs:723-725`). Ragdoll bones carry contacts but no vitals. The player is the system's only live target.
- **Evidence**: the code path above. Both writers run every frame. `apply_player_drowning_damage` and `water_damage_system` both call `ActorValues::apply_damage` on the same `health` key.
- **Impact**: double hazard damage, and a wading-vs-swimming rule that contradicts itself. Hazard water is authored on Oblivion (5/23 WATR, `watal.md:732`) and on modded FO3/FNV. Vanilla FO3/FNV `DATA` damage is a sentinel 0, so those games are unaffected unless modded.
- **Trigger Conditions**: the player in a `WaterPlane` with `damage_per_second > 0` in Character mode.
- **Related**: #3119 (single death reconciler, intact), #4458
- **Suggested Fix**: pick one owner. Either `water_damage_system` skips `PlayerEntity` and the controller keeps its swim-gated damage, or the controller drops its arm and the Late system owns damage for every wet actor. Correct the "dynamic NPC/creature bodies" doc either way, and pin with a test that the player loses exactly `dps·frac·dt` per frame.

## Findings — LOW

### PHYS-D1-2026-10-11-03: Material 0 is treated as "unauthored" everywhere, but it is authored STONE on Oblivion/FO3/FNV; the impact drain drops every both-0 pair, contradicting its own comment
- **Severity**: LOW
- **Dimension**: Shape Translation (surface-material plumbing / user-data contract)
- **Location**:
  - Reduction: `crates/nif/src/import/collision/shape.rs:110-118`, `:136`, `:152-157`, `:165`, `:172`, `:179`, `:184`.
  - Impact drain: `byroredux/src/systems/audio.rs:374-383`.
- **Status**: NEW
- **Description**: On Oblivion and FO3/FNV, `*_HAV_MAT_STONE = 0` (nif.xml:410, 446). There is no "unset" value, and only Skyrim reserves 0 (`NONE`). The walker uses `!= 0` as "authored" throughout, with two consequences:
  - **(a) Wrong reduction.** A leaf authored STONE under a container with a non-zero material inherits the container's material (shape.rs:136). The body-level reduction is "first non-zero", so a body whose leaves are [STONE, WOOD] reports WOOD (shape.rs:117). A packed mesh's collapse also skips stone (shape.rs:155).
  - **(b) Silent impact drop.** The impact drain filters both colliders with `.filter(|m| *m != 0)` and then `continue`s:
    ```rust
    // The impacted side's material wins: collider 1's authored byte,
    // else collider 2's. Both STONE (0) falls through to the default
    // via the selection below.
    let Some(material) = (pw.collider_surface_material(event.collider1).filter(|m| *m != 0))
        .or_else(|| pw.collider_surface_material(event.collider2).filter(|m| *m != 0))
    else { continue; };
    ```
    The comment says both-0 pairs reach the default sound. The code skips them. `select_impact_sound(…, 0)` returning the default is pinned by `impact_selects_the_surface_sound_when_authored` (audio.rs:650-705), but the drain never calls it with 0.
- **Trigger Conditions**:
  - (a) Mixed-material Oblivion/FO3/FNV bodies where one part is authored STONE.
  - (b) Any `started` contact-force crossing where both colliders carry 0. That includes every impact between a stone-authored or unauthored clutter body and terrain, synthesized architecture, the player capsule, or ragdoll bones (all 0 by construction), and every FO3/FNV pair given -01.
- **Impact**: Dormant today, because `ImpactConfig::default_sound` and both maps ship `None`/empty (components.rs:1891-1897). Once a default impact sound is configured, the most common impact class on the three Gamebryo-era games stays silent, against the documented "falls back to the default" contract (components.rs:1872-1874).
- **Related**: -01, -02. The audio-side half overlaps whichever dimension owns the `1291f370a` contact-event drain; dedup there if also reported.
- **Suggested Fix**: Make "unauthored" explicit instead of overloading 0. For example, carry `Option<u32>`/a sentinel decided per game at the import boundary (Skyrim 0 → None; Oblivion/FO3 0 → Some(STONE)). In the drain, fall through to `select_impact_sound` with the resolved material rather than `continue`.

### PHYS-D1-2026-10-11-04: `part_surface_materials` is built by a second hand-maintained walker that drifts from `resolve_shape`'s leaf order for multi-spheres and None-resolving children
- **Severity**: LOW
- **Dimension**: Shape Translation (surface-material plumbing)
- **Location**:
  - `crates/nif/src/import/collision/shape.rs:141-147` (`leaf_material!(BhkMultiSphereShape)` pushes one entry) vs `:257-283` (resolves to k `Ball` parts).
  - `:164-177` (List/ConvexList materials recurse every child) vs `:364-405` (resolve drops children that return None).
  - `:214` (unmatched block pushes one entry) vs `:249-251` and `:470-485` (resolve returns None).
  - `:95-97` (cycle pushes an entry) vs `:48-54` (resolve drops it).
- **Status**: NEW
- **Description**: `RigidBodyData::part_surface_materials` must be index-aligned with `collision_shape_to_parts`' part order. The converter consumes it with a running cursor (convert.rs:170-189), and that cursor is correct only if the producer's sequence matches part order. The producer is a separate traversal that re-implements `resolve_shape`'s dispatch, and it disagrees in four branches:
  1. A `bhkMultiSphereShape` with k ≥ 2 finite spheres becomes k Ball parts but contributes one material entry.
  2. A List/ConvexList child that resolves to None contributes an entry but no part. That covers plane, phantom, unsupported, non-finite primitive, empty `finish_trimesh`, failed transform, and missing packed/compressed data.
  3. A cycled child contributes an entry but no part.
  4. A multi-sphere with 0 finite spheres contributes an entry but no part.

  After the first drift every later part takes its neighbour's material. Parts past the sequence end take the body default. For example, List[MultiSphere(2, WOOD), Box(METAL)] yields parts [Ball, Ball, Cuboid] against materials [WOOD, METAL], so it registers WOOD, METAL, WOOD (the default). The metal box reports wood.
- **Trigger Conditions**: a compound body containing one of the drift branches followed by at least one sibling with a different material. Vanilla exposure is tiny: 1 Oblivion multi-sphere and 1 SSE plane in the census. Otherwise only corrupt or modded content triggers it.
- **Impact**: The wrong per-part surface byte is assigned silently. No test can see it: the converter test hand-writes the slice, and the nif test fixture has only List/Sphere/Box. Low vanilla incidence keeps this LOW.
- **Related**: -01 (another way the two walkers disagree with the parser).
- **Suggested Fix**: Produce the material alongside the shape in the same walk. Have `resolve_shape` return each leaf's material, either in the `CollisionShape` leaf or as a parallel `Vec` pushed at the exact sites that emit leaves, so alignment holds by construction. At minimum, `debug_assert_eq!` the leaf count against a leaf count of the resolved `CollisionShape` in `extract_from_classic`, and add a multi-sphere-plus-sibling fixture.

### PHYS-D1-2026-10-11-05: The release test target of `byroredux-physics` no longer compiles; both release-only shape-boundary guards are unrunnable, and the #2862 compound backstop has no runnable test in any lane
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

### PHYS-D1-2026-10-11-06: The GeometricMean pin tests the constant, not the two collider producers that must apply it
- **Severity**: LOW
- **Dimension**: Shape Translation (contact combine)
- **Location**: `crates/physics/src/world/mod.rs:1677-1726` (guard). Producers at `crates/physics/src/sync.rs:1155-1156` and `crates/physics/src/ragdoll.rs:455-456`. Claim at `crates/physics/src/config.rs:70-75`.
- **Status**: NEW
- **Description**: config.rs:70-75 says the rule is "Applied at both collider producers … so they cannot drift apart. Pinned by `restitution_zero_body_does_not_bounce_on_restitution_floor`." That test builds both colliders by hand with `CONTACT_COEFFICIENT_COMBINE_RULE`. It fails if the constant changes to `Average`, but stays green if `register_newcomers` or `build_ragdoll` drops `.friction_combine_rule`/`.restitution_combine_rule`. Because `GeometricMean` wins on priority, losing it at one producer changes only pairs where neither side comes from the other producer. Losing it in sync.rs reverts every clutter-on-architecture and clutter-on-terrain pair to `Average` (restitution-0 clutter rebounding at 0.4 off a 0.8 surface), with no test failing.
- **Trigger Conditions**: a future edit to either builder chain (both are long builder expressions that were edited three times in this delta alone).
- **Impact**: The gap is in test coverage only. Today both producers do set the rule (verified above).
- **Related**: `04cb6333d`; plan `docs/engine/dependency-update-followups-plan.md` §1.1 "Done when".
- **Suggested Fix**: Add a test that registers a `RigidBodyData` body through `physics_sync_system`, and a two-body ragdoll through `build_ragdoll`, then asserts `collider.friction_combine_rule() == GeometricMean` (and restitution) on every inserted collider.

### PHYS-D1-2026-10-11-07: Migration-stale comments in convert.rs: parry's convex-hull "panic" premise, nalgebra type names, and the TriMesh `Err` rationale
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

### PHYS-D2-2026-10-11-04: (latent) `impact_audio_system` `break`s out of the frame on one unmapped material, and plays sounds for crossings from a substep the engine rolled back
- **Severity**: LOW
- **Dimension**: Step & Sync (contact-event consumer)
- **Location**: `byroredux/src/systems/audio.rs:371-389` (material resolve), `:398` (`else { break }`); `crates/physics/src/world/mod.rs:1097-1103` vs `:1147-1166` (drain precedes the restore decision)
- **Status**: NEW (introduced by `1291f370a`)
- **Merge note**: the first bullet below (both-0 pairs skipped) duplicates PHYS-D1-2026-10-11-03 and is counted and fixed there. This finding covers the `break` and the rolled-back crossings.
- **Description**:
  - **STONE/unknown pairs are skipped.** The comment says "Both STONE (0) falls through to the default via the
    selection below" (`:375-376`), and the fn doc says "else the default". The code does `.filter(|m| *m != 0)` on
    both sides and `continue`s when both are 0. `0` is STONE *and* the value of every non-NIF body (terrain trimesh,
    water planes, the player capsule, ragdoll colliders, which `build_ragdoll` registers with material 0 and no
    user-data, `ragdoll.rs:426-458`). So a stone item hitting terrain, or anything hitting a ragdoll or stone floor,
    never plays, even when `default_sound` or `surface_sounds[0]` is authored.
  - **One unmapped material ends the loop.** `select_impact_sound(...) else { break }` abandons every remaining impact
    of the frame the first time a material has no entry and there is no default.
  - **Rolled-back crossings still play.** The drain (`:1097-1103`) keeps `started` crossings from a substep whose
    bodies the engine then restores as an invalid solve. An explosion's insane contact forces clear any
    `min_force_magnitude` and become a burst of impact one-shots at the corpse.
- **Evidence**: The code at `audio.rs:377-383`. `impact_selects_the_surface_sound_when_authored` tests only
  `select_impact_sound`, not the 0-filter or the loop.
- **Impact**: None today: `ImpactConfig::default()` ships empty and defaultless (`components.rs:1892-1900`). Once
  IPDS/IPCT content lands, impacts are missing for stone, terrain and corpses, one unmapped material truncates a
  frame, and explosions clatter.
- **Trigger Conditions**: authored `ImpactConfig`; a contact pair whose two materials are both 0; or a material with no
  entry and no default; or a recovery substep.
- **Related**: D2-02 (spurious first-wake crossings), #4743 (combat sounds, different path)
- **Suggested Fix**:
  - Resolve `material` without the `!= 0` filter on the fallback side, e.g. prefer the first non-zero byte, else 0, and
    let `select_impact_sound` fall to the default.
  - Change the `break` to `continue`.
  - Discard (truncate) the substep's appended events when `restored > 0`.

### PHYS-D2-2026-10-11-05: `clamp_explosive_velocities` walks the stale-tolerant dynamic index with no body-type check, so a scripted keyframed body that teleports is counted, logged and "parked" as an exploding dynamic
- **Severity**: LOW
- **Dimension**: Step & Sync
- **Location**: `crates/physics/src/world/recovery.rs:353-400` (no `body_type()` filter) vs the snapshot's filter `world/mod.rs:1067`; index doc `world/mod.rs:309-322`; `set_motion_type` keeps flipped-away handles `:813-820`
- **Status**: NEW (pre-dates the delta, since #5161/#5246; not in prior reports)
- **Description**:
  - The `dynamic_bodies` doc promises that staleness "costs a get() miss, never a wrong snapshot" because consumers
    re-check the type. The snapshot does (`:1067`). The clamp, a second consumer added by #5161, does not.
  - A body flipped Dynamic→Keyframed by Papyrus `SetMotionType` (`systems/cinematic.rs:265`) stays in the index.
  - On any tick where its kinematic velocity exceeds 20 000 BU/s (a `MoveTo`/`SetPosition` of more than about
    333 BU), the clamp does the following:
    - It counts a velocity clamp. `set_linvel` is a no-op on `KinematicPositionBased` (`rigid_body.rs:1022-1033`).
    - It logs `warn` "clamped explosive velocity".
    - On the second offence it logs `error` "parked … after repeated solver-explosion velocities" and calls `sleep()`,
      which zeroes the kinematic's velocity directly.
    - It adds a permanent `explosion_offences` entry.
- **Evidence**: Probe (dynamic → `set_motion_type(Keyframed)` → three 1 000 BU teleports):
  ```
  teleport 1: type=KinematicPositionBased linvel=28000 clamps_total=1 sleeping=false
  teleport 2: ... linvel=0 clamps_total=2 sleeping=true
  teleport 3: ... linvel=0 clamps_total=3 sleeping=true
  ```
- **Impact**:
  - False explosion evidence in the log and in `phys.stats` `velocity_clamps_total`. These are the counters #5161
    triage reads.
  - A per-body map entry that lives until the body is removed.
  - Rapier re-wakes the kinematic on its next changed target (`rigid_body.rs:1232-1239`), so motion itself is
    unaffected. A third offence detaches only when the body is in a multibody, which a scripted object is not.
- **Trigger Conditions**: a scripted `SetMotionType(Motion_Keyframed)` on a formerly dynamic reference, followed by
  teleports larger than about 333 BU per tick.
- **Related**: #5161, #5246, #4682
- **Suggested Fix**: Skip non-`Dynamic` entries in the clamp's collection loop, mirroring `:1067`, or compact the index
  on `set_motion_type` away from dynamic.

### PHYS-D2-2026-10-11-06: `footstep_system`'s access declaration omits the new `PhysicsWorld` read, and the surface-sound selection reads `FootstepConfig` while holding the `AudioWorld` write
- **Severity**: LOW
- **Dimension**: Step & Sync (access declarations; rule owned by `/audit-concurrency`)
- **Location**: `byroredux/src/boot/schedule/late.rs:104-116`; `byroredux/src/systems/audio.rs:264-270` (`try_resource::<PhysicsWorld>`), `:274-293` + `:317-330` (`select_footstep_sound` → `try_resource::<FootstepConfig>` inside the `AudioWorld` write scope)
- **Status**: NEW (introduced by `886edcded`, which did not touch `late.rs`)
- **Description**:
  - **Undeclared read.** `886edcded` added a `PhysicsWorld` read to `footstep_system` (the per-trigger
    `surface_material_under`), but the `add_exclusive_with_access` declaration still lists only `FootstepConfig`,
    `PlayerEntity`, `FootstepScratch`, `AudioWorld`, `GlobalTransform`, `CharacterController`, `WaterContact` and
    `FootstepEmitter`. This is the "declaration forgotten" class the physics guards exist for.
    - `impact_audio_system` is declared correctly (`late.rs:123-130`).
    - No guard covers `footstep_system`'s read surface. `footstep_runs_after_camera_follow_in_late` checks ordering only.
  - **Nested resource lock.** The play loop calls `select_footstep_sound(world, …)` while `audio_world` (a resource
    write) is live, which acquires `FootstepConfig` under it. That violates the "one resource lock at a time"
    discipline the function's own comments state. `impact_audio_system` avoids it by snapshotting the map first.
    Because of D2-03, `surface` is always `Some`, so the nested acquisition runs on every footstep.
- **Evidence**: `late.rs:104-116` shows no `PhysicsWorld`. In `audio.rs:274-293`, the `audio_world` guard is held
  across `select_footstep_sound`.
- **Impact**: No live race, since `footstep_system` is a Late exclusive. But `sys.accesses` under-reports the
  system. A future move to the parallel batch would race `PhysicsWorld` writers undetected, and the
  `AudioWorld → FootstepConfig` edge is new detector surface.
- **Trigger Conditions**: always (declaration); every footstep in character mode (nested lock).
- **Related**: #5305, #5535 (lock-order lane; owned by `/audit-concurrency`), D2-03
- **Suggested Fix**:
  - Add `.reads_resource::<byroredux_physics::PhysicsWorld>()` to the declaration.
  - Clone `surface_sounds` out with the default at the top, as `impact_audio_system` does, and select from the snapshot.

### PHYS-D2-2026-10-11-07: Two new physics docs describe behaviour the code does not have — the force-event buffer "cleared at every substep start", and `build_ragdoll` "packing" the surface material
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

### PHYS-D2-2026-10-11-08: Since the #5311 split, the "`update_query_pipeline` never runs per frame" guard scans only `step`, not `run_substep` (the per-substep body) or `recovery.rs`
- **Severity**: LOW
- **Dimension**: Step & Sync (test gap)
- **Location**: `crates/physics/src/world/mod.rs:2955-2968` (scan window = `pub fn step` … first `\n    }\n`); unscanned `run_substep` `:1050-1181`, `recovery.rs:220-232`
- **Status**: NEW
- **Description**:
  - `step_cost_rationale_is_scoped_to_history_and_names_the_real_cost_centre` asserts
    `!step_body.contains("update_query_pipeline")`. Since #5311 extracted the loop body, `step` only calls
    `run_substep`.
  - The most likely regression site is now outside the scanned window: a per-substep whole-set refresh inside
    `run_substep`, or `refresh_query_geometry_after_restore` swapping its targeted `set_query_leaf` loop for
    `update_query_pipeline()` (a plausible "simplification").
  - Today neither calls it (grep: only the definition and test sites).
- **Evidence**: The scan window construction at `:2955-2957`.
- **Impact**: The O(all colliders) refresh, 9.6 ms/frame on a 95 k world in the #4685 measurement (rapier 0.22), could
  return per substep with the guard green.
- **Trigger Conditions**: a future edit to `run_substep` or `recovery.rs`.
- **Related**: #4685, #5311, #5453
- **Suggested Fix**: Scan `step` + `run_substep` + `refresh_query_geometry_after_restore` (or the whole production text
  of `world/mod.rs` and `world/recovery.rs` minus the definition and `register_newcomers_and_refresh_queries`'s
  documented bootstrap).

### PHYS-D4-2026-10-11-02: rapier 0.36's zero-motion depenetration is unaccounted for — the boot `dt = 0` tick pops embedded walking NPCs by offset + embed, an idle player is shoved by overlapping NPC bones and clutter, and two rationale comments still describe 0.22
- **Severity**: LOW
- **Dimension**: Character Controller, Grounding & the NPC KCC (M42.10)
- **Location**: `byroredux/src/app_events.rs:314` (boot `scheduler.run(&self.world, 0.0)`);
  `byroredux/src/systems/wander.rs:313-324` + `locomotion.rs:183-188` (no `dt` guard; `step = speed·0` → `desired = 0`);
  `byroredux/src/systems/character.rs:509` (player `filter_groups: None`), `:391`, `:952-954`
- **Status**: NEW
- **Description**: 0.36 `move_shape` runs `check_and_fix_penetrations` whenever `|desired| < 1e-5`
  (0.36 :315-323, :240-290): it pushes the capsule out of every non-sensor collider passing the filter until the
  `offset` gap is restored, up to `dims.y/4` per call (16 BU player, 13 BU NPC). 0.22 had only a commented-out stub
  (0.22 :182). Neither the migration nor `dependency-update-followups-plan.md` records it. Consequences:
  (1) **NPCs at boot** — the startup `scheduler.run(&world, 0.0)` (`app_events.rs:314`) runs `wander_system`, which
  starts a fresh actor in `Walking` (`wander.rs:313-324`) and steps it with `dt = 0`; no locomotion system has a
  `dt <= 0` guard (wander/travel/guard/escort/follow/patrol/forcegreet/eat_sleep/cinematic), so the step's desired is
  exactly zero and every NPC whose feet are embedded > 1e-5 BU in its floor is popped up by `offset + embed`; it then
  hovers at the 4 BU offset for the rest of its walk (the KCC holds that gap; the fallback ray never fires because
  the KCC reports grounded). (2) **Idle player** — the grounded-branch correction is below 1e-5 on 59–120 of 120 idle
  frames (probe P4), so a standing player now takes the depenetration path most frames; its filter has no
  `ACTOR_BONE_GROUP` mask (`character.rs:509`), so an NPC bone collider or a dynamic body overlapping the capsule
  shoves the player up to 16 BU/frame while standing and never while walking. (3) Stale comments:
  `character.rs:952-954` says "Rapier's penetration recovery is a stub"; `character.rs:391` cites the 0.22
  no-interference branch (`character_controller.rs:317-322`), which in 0.36 is :401-412 and now runs the grounded
  check. `f4c2b9931`'s sweep missed both.
- **Evidence**: Probe P2 (NPC, production params, zero desired): embed 0 → dy 0; 0.5 → +4.5; 2 → +6; 6 → +10; a
  60-frame walk afterwards keeps feet in [3.906, 4.002], whereas the same walk without the zero step keeps the embed
  (−0.5 / −2 / −6). Probe P1 (player at rest, `Vec3::ZERO` desired, bone box overlapping the capsule side by 3 BU):
  translation (−7.0, −0.007, 0) — pushed to the offset gap; same pose walking 3.67 BU: no push. Dynamic box overlap:
  (−6.8, +0.9, 0).
- **Impact**: cosmetic — a 4 BU (≈ 5.7 cm) hover on NPCs embedded at spawn; idle-only nudges of the player by NPC
  limbs/clutter. No crash, no gameplay state. The larger risk is a future fix reasoning from the "stub" comment.
- **Trigger Conditions**: (1) boot cell with a spawned Wander/Patrol NPC whose ACHR feet sit inside its floor;
  (2) idle player overlapped by an NPC bone collider or a dynamic body.
- **Related**: PHYS-D4-2026-10-11-01, PHYS-D4-2026-10-11-03, #3799
- **Suggested Fix**: Early-return the NPC movers (or `step_toward_detailed`) on `dt <= 0`; decide whether the idle
  player should be depenetrated from actor bones (pass `actor_move_interaction_groups()` if not) and record the
  choice; correct the two comments and add the four 0.36 semantics changes to the plan doc's ledger.

### PHYS-D4-2026-10-11-03: On a flat floor the KCC still climbs a phantom slope and drops `grounded` on 1–13% of walking frames — the #3799 OR stays load-bearing while walking, which 738ab7391's rationale and its idle-only pin understate
- **Severity**: LOW
- **Dimension**: Character Controller, Grounding & the NPC KCC (M42.10)
- **Location**: `byroredux/src/systems/character.rs:1612-1631` (rationale), `:1686` (the OR);
  `crates/physics/src/world/mod.rs:3208-3252` (`kcc_reports_grounded_for_a_capsule_at_rest`)
- **Status**: NEW (behaviour pre-exists and was worse on 0.22; the documentation claim is new in `738ab7391`)
- **Description**: 738ab7391 rewrote the OR's rationale to "rapier 0.36 closed that hole … the OR below is no longer
  a workaround for a rapier bug. It stays as a defence". True for the idle input it pins, not for walking: when a
  grounded capsule slides horizontally at exactly the offset gap, parry 0.31's support-map cast returns a
  `Converged` hit at a small-but-above-threshold TOI (the contact-normal fallback only covers `toi < 1e-4`,
  `shape_cast_support_map_support_map.rs:34-35`) with a GJK normal tilted 4–21° against the motion; `handle_slopes`
  treats it as a climbable slope (below 50°) and lifts the capsule by up to ≈ ½ the step, ending outside the
  narrowed 4.05 BU grounded band (0.36 :492), so the KCC answers `grounded = false`. The engine probe's walkable
  hit (`character.rs:453-454`) is what keeps `is_grounded` — and therefore jump — on those frames.
- **Evidence**: Probe P12 (captured collision events, cuboid floor, +X walk): `hit toi 0.019739 normal1
  (-0.0786, 0.9969, 0)`, `toi 0.008156 normal1 (-0.2344, 0.9330, -0.2730)`. Probe P6 (production correction +
  walk, 600 frames): KCC grounded lost 8/600 (diag walk), 4/600 (diag sprint), 16/600 (diag 144 Hz), 80/600 (+X
  walk); lifts up to 1.67 / 3.35 / 0.70 / 2.07 BU. Production replica P8: feet gap stays in [3.86, 7.72] (body bob,
  pulled back next frame by the probe; the three sprint runs that walked off the 4 000 BU test slab excluded) — vs
  [−13.7, +16.8] and KCC-grounded lost 70–452/600 for the same replica on 0.22, so the migration improved this; it did not close it. Idle (P4): 0/120 lost at every tested offset.
- **Impact**: none today (the OR covers it); a ≤ 0.5×step vertical body bob (camera eases a fraction of it up,
  `character.rs:746-752`). Removing or weakening the OR on the strength of the rationale would drop jump input on
  up to 13% of walking frames; `kcc_reports_grounded_for_a_capsule_at_rest` would stay green.
- **Trigger Conditions**: grounded player walking or sprinting on a flat convex or trimesh floor.
- **Related**: #3799, #3971, PHYS-D4-2026-10-11-01
- **Suggested Fix**: Reword the rationale to say the OR is still load-bearing on walking frames (cite the phantom
  slope), and add a walking variant of the 0.36 pin that drives the production correction + horizontal input and
  asserts `kcc_grounded || probe_found_support` on every frame while documenting the KCC-only loss rate.

### PHYS-D5-2026-10-11-04: A sleeping float re-wakes for waves only when the per-frame depth change exceeds 0.1 BU, so wave-following depends on frame rate and stops entirely at high refresh rates
- **Severity**: LOW
- **Dimension**: WATAL Physics Sink (wake discipline)
- **Location**: `crates/physics/src/water.rs:955-961` (the wake predicate); `prior_depth` is rewritten for every wet body every scan (`water.rs:1006-1017`)
- **Status**: NEW (logic from `6b9603490`, 2026-08-20)
- **Description**:
  - The predicate is `b.is_sleeping() && (surface_y - center_y - t.prior_depth).abs() > 0.1`.
  - `prior_depth` is last frame's depth, written even while the body sleeps, so the test is a per-frame delta of (wave rate × frame dt) and not drift from the rest pose.
  - At higher frame rates the per-frame change shrinks below 0.1 BU, so a slept float never wakes and stays rigid while the rendered surface oscillates around it.
- **Evidence**: probe `wave.rs` (radius 15 float, `TotalTime` advancing, last 10 s):
  ```
  amp=1: 30fps tracks (p2p 2.04, 5 wakes) | 60fps & 144fps rigid (asleep 100%, 0 wakes)
  amp=2: 30fps p2p 5.89 | 60fps p2p 6.98 | 144fps rigid (max dWave/frame 0.047)
  amp=4: 30/60fps p2p ~11.3 | 144fps rigid against ±3.9 BU waves
  ```
- **Impact**: a settled float sits up to 2·amplitude off the visible surface at ≥ ~70–144 fps but bobs at 30 fps. It is cosmetic; vanilla amplitudes are small (Skyrim clamps at ≤ 2).
- **Trigger Conditions**: authored `wave_amplitude` ≳ 0.5 BU, a slept float, and a high frame rate.
- **Related**: PHYS-D5-2026-10-11-02 (same per-frame-not-per-tick root); #3135 (the rescan gate that makes this predicate reachable)
- **Suggested Fix**: compare against the depth latched when the body fell asleep, i.e. don't overwrite `prior_depth` while sleeping. Alternatively, scale the threshold by `dt`.

### PHYS-D5-2026-10-11-05: Two consumers still read the body origin where the WATAL contract says collider AABB centre: the buoyancy wave sample, and the splash/ripple marker height
- **Severity**: LOW
- **Dimension**: WATAL Physics Sink (one reference point, #2887/#3490/#3973 class)
- **Location**: `crates/physics/src/water.rs:888-899` (`authored_wave_height_with_weather(.., Vec3::new(pos.x, pos.y, pos.z), ..)`, where `pos` is `body.translation()`, `:794-803`); `byroredux/src/systems/water.rs:438-442` (`position = global.translation; position.y += contact.depth`)
- **Status**: NEW (the wave sample dates from `6b9603490`; #3973 moved the three containment tests to `reference_point` but not this sample. The splash adapter dates from `5959bbb84`, before #2887 changed `depth` to AABB-centre-relative.)
- **Description**:
  - (a) The static surface height is sampled at `reference_point.xz` (`water.rs:880-887`), but the wave term added to it is sampled at the rigid-body origin. For compound bodies and ragdoll bones, which the code itself says are offset "by construction", the crest phase is taken at a different point from the column it is added to.
  - (b) The splash and ripple adapter comments "`GlobalTransform` is the body's centre", but `depth = surface_y − aabb_centre_y` (`water.rs:1010`). So `origin.y + depth` lands at `surface_y + (origin.y − centre.y)`, off the waterline by the origin↔centre offset. The XZ comes from the origin too.
- **Evidence**: the two cited lines. The doc at `water.rs:805-836` lists three consumers of `reference_point` and does not mention the wave sample.
- **Impact**: a small crest-phase error (amplitude × phase difference) in the buoyancy surface. The splash/ripple FX and sound for offset bodies are placed above or below the surface by up to the collider offset (tens of BU for offset compound parts and vertical ragdoll limbs).
- **Trigger Conditions**: a body whose collider AABB centre differs from its origin: bhk compound parts and ragdoll bones.
- **Related**: #2887, #3490, #3973
- **Suggested Fix**: (a) pass `reference_point` to `authored_wave_height_with_weather`. (b) Publish the contact's `surface_y` (or the reference XZ) in `WaterContact` and place the event there, rather than reconstructing it from `GlobalTransform`.

### PHYS-D6-2026-10-11-01: rapier 0.36 sleeps idle kinematic bodies, so `phys.stats`' "kinematic bodies=" became an active count; the #3975 docs, the fast-path rationale, the source guard that pins them, and the follow-ups plan's "ruled out" entry all assert the 0.22 behaviour
- **Severity**: LOW
- **Dimension**: Queries, Diagnostics & Cost
- **Location**:
  - `crates/physics/src/world/mod.rs:544-557` (`active_island_counts` doc), `:622-624` (`has_awake_dynamics` doc), `:947-952` (fast-path NOTE), `:2984-3020` (guard)
  - `byroredux/src/commands/physics.rs:189-192, 225`
  - `crates/physics/src/sync.rs:172-173`
  - `docs/engine/dependency-update-followups-plan.md:528-529`
- **Status**: NEW. The premise was inverted by `ac81a09f9`. Related: closed #3975, whose doc fix is still in place but is now false.
- **Description**:
  - Under rapier 0.22 kinematic bodies lived in `active_kinematic_set`. That set was only ever pruned on removal or a user type or enable change (0.22 `island_manager.rs:15, 43-90`, `user_changes.rs:81-104`). #3975 therefore documented `active_island_counts().1` as "every live kinematic body, not an awake count".
  - Rapier 0.36 gives kinematics a real sleep test, "Platforms only sleep if both velocities are exactly zero" (`rigid_body_components.rs:1488-1493`). Idle keyframed bodies hit exactly that: `push_kinematic` skips no-op targets (`sync.rs:1273-1282`), so the interpolated kinematic velocity is 0.
  - The line the follow-ups plan cites for "still correct" (`island_manager/sleep.rs:30`, "kinematic and fixed bodies don't sleep") is the doc of `wake_up`. It is not the sleep decision.
- **Evidence**: the probe's `kinematic_sleep_tick` test:
  - Setup: 5 idle kinematic balls plus one free-falling dynamic, so `step` is never fast-pathed.
  - Output:
    ```
    tick 0: kinematic active = 5
    tick 29: kinematic active = 0
    live kinematic bodies in the set: 5
    ```
  - After a one-shot `set_next_kinematic_translation`, the count returns to 1 and drops to 0 again once the body idles (`idle_kinematics_leave_the_active_set`).
  - Every site listed under Location still states the 0.22 behaviour:
    - `world/mod.rs:548-553`: "the kinematic bodies, which rapier does not put to sleep … counts kinematic bodies whether or not anything moved them".
    - `:623`: "kinematics (which never leave it)".
    - `:948-951`: "Rapier keeps kinematic bodies in its active set without a sleep test … the set is never empty in a cell with authored-keyframed clutter".
    - `commands/physics.rs:189-192`: "Rapier's kinematic active set is never drained, so this is every *live* kinematic body (asleep or not)".
    - `sync.rs:172-173`: the same claim for `BYRO_PROFILE`.
    - The guard `kinematic_count_doc_agrees_with_the_fast_paths_own_rationale` asserts `doc.contains("not an awake count")` and `rationale.contains("never empty")`, so it now enforces the false text.
- **Impact**:
  - **Diagnostics only.** No simulation change: `has_awake_dynamics` filters to dynamics, so the fast-path decision is unaffected.
  - `phys.stats` (and the `BYRO_PROFILE` line) report e.g. `kinematic bodies=0` about 0.5 s after a cell's doors, keyframed clutter and an idle player capsule settle, even though hundreds of live kinematic bodies exist. The output's own comments tell the operator the number is the live total.
  - The plan's "Checked and ruled out — don't redo" entry steers the next reader away from the fact.
- **Trigger Conditions**: any cell with keyframed bodies (doors, FO4+ packed proxies, actor bones of non-animating actors, an idle player capsule) after about 30 idle ticks; `phys.stats` or `BYRO_PROFILE=1` read at that point.
- **Related**: #3975 (closed); D2's verified note "Kinematics sleep only at exactly zero velocity".
- **Suggested Fix**:
  - Re-word the five comment sites and the `phys.stats` label to "active kinematic", meaning moved within the last ~0.5 s.
  - Flip the guard's needles to the 0.36 claim.
  - Strike or correct plan line 528. If a live total is wanted, add `bodies.iter().filter(is_kinematic).count()`; it is on-demand only.

### PHYS-D6-2026-10-11-02: `phys.stats`' contact trio mixes populations: `pairs` is the whole narrow-phase graph (sleeping and non-touching pairs included), while `contacts`/`constraints` are the awake island only, and `constraints` never counts the multibody (ragdoll) joints the trio exists to watch
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

## Existing (matched; cited, not re-counted)

| Issue | Severity | Note |
|---|---|---|
| #4772 | MEDIUM | FO3 restore: the first ragdoll solve jumps ~1e12 BU. Under rapier 0.36 the probe shows no 1e12 restore, only clamp → detach. PHYS-D3-2026-10-11-03 supplies the likely root cause: rigid default joint limits correcting an out-of-range seed (mirrored by D3-01) inside one 1/960 s substep. Consider posting D3-03's probe matrix to #4772 as a comment. |
| #5354 | MEDIUM | Flowing-water weather damping is missing from dynamic buoyancy and `player_water_state`. Still open; not re-filed (Dim 5). |
| #5015 | MEDIUM | XRGD corpse poses are not decoded. Untouched by this delta. |
| #5530 | LOW | `accept_keyframe_target` gates only `push_kinematic`. Unchanged (Dim 2). |
| #5452 | LOW | The force-greet and Eat/Sleep movers discard `blocked`. Unchanged. PHYS-D4-2026-10-11-01 is a different defect on the same mover path. |
| #5535 | — | The ABBA lock-order lane is red (bin-test cycles). Owned by `/audit-concurrency`; referenced only. |

## Known-Open Register (what this pass changed)

- **#4134** is fixed and confirmed in place: `character_capsule` goes through `clamp_shape_extent`, and the guard fails on the old floor-only clamp.
- **"Upstream instability of mass-inverted rigs / authored restitution 0.8"** (no issue) is **superseded by PHYS-D3-2026-10-11-03**. Restitution 0 lets the FO3 fixture settle, but per-joint softness alone also fixes it at authored restitution. So restitution is a contributor, not the root. The root is stiff limits plus a seed outside the (mirrored) limits.
- **The rapier 0.22 multi-SAP `sap_axis.rs` panic class** (#5161/#5488) is gone with the BVH broad phase. No residual was found.
- **Ball-and-socket / stiff-spring / chain declines**: unchanged by design. There is still no occupancy evidence, so nothing was filed.
- **NEW register entry**: the FO3 real-data ragdoll gate (`ragdoll::installed_tests`, 6 tests, `#[ignore]`) is **red at HEAD, 4/6**. It was already red at yaw 0 before the migration (probe at `ac81a09f9^`). Four prior physics audits recorded it as "not run".
- **NEW register entry**: rapier 0.36 behaviour changes absent from `docs/engine/dependency-update-followups-plan.md`:
  - begin-touch strong-wake (D2-02);
  - extra iterations made world-wide by any multibody (D3-04; the plan's line 505-509 "ruled out" entry checks only the ragdoll's own substep count);
  - snap-to-ground on every non-upward move (D4-01; the plan names it but rules no consequence);
  - zero-move depenetration (D4-02);
  - idle kinematics sleeping (D6-01; the plan's line 528 cites the `wake_up` doc, not the sleep decision).

## Cross-Dimension Deduplication

- **D1-02 = D3-05**: the Skyrim CRC32 material is truncated `as u8`. Counted once, as D1-02 (Dim 1 owns shape translation). D3-05's PHYSAL framing (an untranslated per-game seam) is folded into D1-02's Related line.
- **D1-03 ⊃ D2-04 (first bullet)**: `impact_audio_system` skips both-0 pairs. Counted once, under D1-03, which also covers the "0 = authored STONE" reduction error. D2-04 keeps the `break` and the rolled-back crossings.
- **D2-03, extended by Dim 4**:
  - The footstep self-hit is D2-03's.
  - Dim 4 adds that each part is its own collider, so the hit part's byte is read (never part 0).
  - Packed and compressed meshes collapse to their first non-zero material by design.
  - No NPC footstep emitter exists.
- **D5-01 corrects Dim 3's verified line**: "`ragdoll_extra_angular_damping` added once per body" is true as written, but the value is never applied. Rapier skips rigid-body damping for multibody links. The orchestrator confirmed this at `rapier3d-0.36.1/src/dynamics/solver/staged_island_solver/worker.rs:956-962`, a `continue` before `apply_damping` at `:974`. The pin `recorded_damping_is_the_effective_value_build_ragdoll_applied` passes only because its fixture is a single unjointed body.
- **D6-01 vs Dim 2's fast-path verification**: consistent. Dim 2 verified the fast-path *decision*, which filters to dynamics. D6-01 is the doc and label claim that kinematics never sleep.

## Orchestrator verification notes

- **FO3 gate re-run independently** with `BYROREDUX_FO3_DATA="/mnt/data/SteamLibrary/steamapps/common/Fallout 3 goty/Data/"` and `-p byroredux --bin byroredux -- --ignored ragdoll::installed_tests`. Result: 2 passed, 4 failed, all four panicking at `ragdoll_installed_tests.rs:229`.
- **D3-01 read against the code** at `crates/physics/src/ragdoll.rs:705-730`.
  - Non-flip: `frame1 = A` (parent), `frame2 = B`, limits unchanged. Rapier measures θ as the child relative to the parent, i.e. B relative to A.
  - Flip: `frame1 = B`, `frame2 = A` (child), limits negated and swapped.
  - Under Havok's A-relative-to-B convention (the 2007 SDK limit drawer, cited in the finding), both branches enforce the mirror. The bind-pose census is the strongest evidence: FNV/FO3 and SSE fit Havok's convention 8/8 but violate the engine's on 4/8 and 6/8.
- **D5-01's upstream claim verified** (see Cross-Dimension Deduplication).
- **D1-05 verified by reading**: `convert.rs:748-777` is `#[cfg(not(debug_assertions))]` and calls nalgebra's `UnitQuaternion::into_inner().coords` on a `glam::Quat`.
- **HEAD drift during the run**: seven commits (`9da80470d`..`65cadde2f`) landed on main from another session while the audit ran. `git diff --stat c43c25624..65cadde2f` over every physics-scope path is empty.

## Skill drift (for the next `/audit-sync`)

1. **Phase 1**: give the exact FO3 gate command, `BYROREDUX_FO3_DATA=<FO3 Data> cargo test -p byroredux --bin byroredux -- --ignored ragdoll::installed_tests`. The module path is `ragdoll::installed_tests`; a `ragdoll_installed` filter matches 0 tests and exits green.
2. **Dim 1**: the `huge_finite_*` guards run in the default lane, not release-only. The release-only guards cannot currently compile (D1-05).
3. **Dim 2**: "rapier 0.36's active set also holds kinematics, which never leave it" is false. Kinematics sleep at exactly zero velocity (D6-01). Also add the begin-touch strong-wake (D2-02) and the cooldown-wake clear (D2-01) to the checklist.
4. **Dim 3**:
   - The guard `ragdoll_solver_budget_is_local_and_preserves_authored_mass_and_limits` does not test locality (D3-04) or the limit sign (D3-01).
   - "Byte consumption is exact for the decoded types, so drift telemetry is live" is stale: the FO3+ motor-tail drift is by design (`constraints.rs:755-765`).
   - "`ragdoll_extra_angular_damping` … is added once per body" should say "and is inert on multibody links" (D5-01).
   - Add the hinge sign convention and the Oblivion `Perp B1` derivation to the checklist.
5. **Dim 4**: add rapier 0.35+ snap-on-horizontal-move (D4-01) and 0.36 zero-move depenetration (D4-02) to the KCC checklist. The NPC step has no #2857-style probe.
6. **Dim 5**: add "multibody links ignore `RigidBody` damping" and "buoyancy is a per-frame zero-order hold across N×K integration steps" (D5-01/D5-02).

## Probe artifacts

- Dim 3 built `target/debug/{ragdoll_probe,leak,ragdoll_probe_pre,leak_pre}` (and their deps) into the repo `target/`. These are build output only, safe to delete, and not tracked.
- The other probes were throwaway `rustc --test` binaries or crates under the session scratchpad and `/tmp/audit/physics/` (`d4probe`, `d4probe22`, `dim3_probe`), linked against the repo's built rlibs.
- No repo file was edited by any dimension. The only repo file this run writes is this report.

Next step: `/audit-publish docs/audits/AUDIT_PHYSICS_2026-10-11.md`. Use labels `physics`, plus `water` for D5-*, `audio` for D1-01..03 / D2-02..04 / D2-06, and `game:*` where a finding is Havok-data-specific: D1-01 `game:fo3`/`game:fnv`, D1-02 `game:skyrim`, D3-02 `game:oblivion`, D3-03 `game:fo3`.

---

## Appendix — per-dimension guards, verification and dedup notes

Carried from each dimension's working file, so the next delta run can see what was checked, what is vacuous, and what was disproved. Findings themselves are above.


### Dim 1 — Shape Translation

**Delta commits reviewed** (`git show` each): `ac81a09f9` (rapier 0.36 / parry 0.31 migration, convert.rs + config.rs), `04cb6333d` (GeometricMean combine), `886edcded` + `441d90c2f` (Havok surface material → `RigidBodyData::surface_material` / `part_surface_materials` → collider user-data), `002fbd6ef` (#4134 capsule clamp), `51feb00a4` (cite re-point, config.rs comment only), plus the user-data consumer half of `1291f370a` (impact drain) where it reads the material byte.

**Upstream read, not assumed**: `parry3d-0.31.1` (`shape/trimesh.rs`, `shape/shared_shape.rs`, `shape/triangle_pseudo_normals.rs`, `shape/convex_polyhedron.rs`, `transformation/convex_hull3/*`), `rapier3d-0.36.1` (`dynamics/coefficient_combine_rule.rs`, `geometry/collider.rs`), `glamx-0.3.1` (`rot3.rs`, `pose3.rs`), and the 0.17.6 / 0.22.0 copies for before/after comparison. Havok SDKs: `havok-2013/Physics2012/Dynamics/Common/hkpMaterial.inl:49-57`, `havok-20070919/Source/Physics/Dynamics/Common/hkpMaterial.inl:38-46`. nif.xml: `/mnt/data/src/reference/nifxml/nif.xml:408,444,576,2293-2299,3195-3196,3968-3969`.

**Corpus census** (prebuilt `target/debug/examples/nif_stats --tsv` over vanilla mesh BSAs, read-only; no build, no engine launch):

| block | Oblivion | FO3 | FNV | SSE Meshes0+1 |
|---|---|---|---|---|
| bhkPackedNiTriStripsShape | 0 | 5,634 | 7,468 | 0 |
| bhkCompressedMeshShape | 0 | 0 | 0 | 8,188 |
| bhkMultiSphereShape | 1 | 0 | 0 | 0 |
| bhkListShape | 185 | 66 | 147 | 405 |
| bhkConvexListShape | 0 | 55 | 0 | 0 |
| bhkPlaneShape | 0 | 0 | 0 | 1 |

#### Guards checked

| Guard | Lane | Verdict |
|---|---|---|
| `config.rs::trimesh_flag_bits_match_rapier_definitions` (config.rs:142-156) | default | **PASS, non-vacuous.** It compares the engine constants against the linked parry's `TriMeshFlags::{ORIENTED, MERGE_DUPLICATE_VERTICES, FIX_INTERNAL_EDGES}.bits()`. The 0.31.1 values are `1<<3`, `1<<4`, `(1<<7)\|MERGE` (trimesh.rs:476,481,501), which match config.rs:36-41. Parry 0.19 dropped the implied `ORIENTED`; `default_trimesh_flags_include_fix_internal_edges` (config.rs:159-174) covers that separately. |
| `convert.rs::huge_finite_{cuboid,ball,capsule,cylinder}_*` + `huge_placement_scale_clamps_to_sane_ceiling` (convert.rs:586-653) | **default** (no cfg gate) | **PASS, non-vacuous.** The dim brief calls these release-only. They are not: all five run and pass in the default lane. |
| `convert.rs::non_finite_cuboid_extent_clamps_instead_of_reaching_rapier` (convert.rs:661-672) | release-only | **UNRUNNABLE.** The release test target of `byroredux-physics` no longer compiles (PHYS-D1-2026-10-11-05). |
| `convert.rs::non_finite_extents_clamp_in_the_debug_lane_too` (convert.rs:683-707, the #4686 twin) | default | **PASS, non-vacuous.** It tests `clamp_shape_extent` directly and through Ball/Capsule/Cylinder. |
| `world/mod.rs::restitution_zero_body_does_not_bounce_on_restitution_floor` (world/mod.rs:1677-1726) | default | **PASS.** It pins the constant's behaviour, not the producers' use of it (PHYS-D1-2026-10-11-06). |
| `convert.rs::parts_carry_their_leaf_surface_materials` (convert.rs:1437-1484) | default | **PASS.** It pins the converter cursor only. The slice is hand-written, so it cannot see producer-side misalignment (PHYS-D1-2026-10-11-04). |
| nif `shape_leaf_materials_match_resolve_order_and_inherit_containers` / `first_shape_material_prefers_…` (shape.rs:973-1066) | default | **PASS.** Fixtures cover only List/Sphere/Box. There is no multi-sphere, packed-strips (FO3 layout), compressed-mesh, or None-resolving child. |
| `world::tests::character_capsule_sanitises_non_finite_and_absurd_extents` (#4134) | default | **PASS, non-vacuous** (fails on the old floor-only `.max(1e-3)`). |

Run: `cargo +1.96 test -p byroredux-physics -- trimesh_flag huge_finite non_finite clamp restitution_zero parts_carry character_capsule` gave 22 passed. `-p byroredux-nif --lib -- shape_material leaf_materials multi_sphere` gave 5 passed. `cargo test --release -p byroredux-physics --no-run` gave **E0599** (see -05).

#### Verified

- **Variant/arm parity, delta 0.** `CollisionShape` has 7 variants (Ball, Cuboid, Capsule, Cylinder, ConvexHull, TriMesh, Compound; `crates/core/src/ecs/components/collision.rs:25-46`). The match in `flatten_to_parts` (convert.rs:214-423) is exhaustive with 7 arms, and the compiler enforces it. Importer side: every bhk shape type present in the vanilla census has a `resolve_shape_inner` arm (shape.rs:229-459). `bhkPlaneShape` → None is by design and counted (#4407; shape.rs:249-251). Phantoms → None by design (shape.rs:470-478).
- **Compose order.** `parent_iso * pose_from_trs(safe_t * scale, safe_r)` (convert.rs:237) is parent-then-child under glamx `Pose3`. The non-commuting pin `rotated_compound_composes_parent_then_child` (convert.rs:858) passes under 0.31.
- **Non-uniform scale** cannot arise. `GlobalTransform::scale` is a scalar `f32` (convert.rs:119-121). Per-axis authored Havok scale is folded into vertices at import (`per_axis_scale`, shape.rs:495-501), which is a unit conversion, not placement scale.
- **Scale applied exactly once at the sink.** `collision_shape_to_parts(&n.shape, n.global.scale, …)` (sync.rs:1046-1052) and `(…, b.scale, …)` (ragdoll.rs:426-432) are the only production callers (plus test world/mod.rs:1267). `spawn_collision_shapes` keeps the shape local and puts the composed scale on `GlobalTransform` (byroredux/src/cell_loader/spawn.rs:1395-1415). The packed proxy no longer pre-bakes `ref_scale` (spawn.rs:151, 240-254; #3959). Propagation precedes `Stage::Physics`, so the seeded `1.0` on the ghost is overwritten (#3959 evidence chain).
- **Rapier/parry 0.31 API semantics re-checked against source:**
  - `SharedShape::cuboid(hx,hy,hz)` takes half extents (shared_shape.rs:290).
  - `capsule_y(half_height, r)` is the segment ±half_height without caps (shared_shape.rs:331-334), which matches the importer's `|p2-p1|/2` (shape.rs:305).
  - `cylinder(half_height, r)` (shared_shape.rs:221).
  - `capsule(a, b, r)` (shared_shape.rs:304).
  - `ColliderBuilder::position` is relative to the parent body (rapier collider.rs:1423-1430).
  - `TriMesh::with_flags` returns `Err` only on empty indices and discards `set_flags`' `TopologyError` (trimesh.rs:737-765). The convert.rs `match` handles the `Result` (convert.rs:404-421).
  - `Isometry3` → `Pose = Pose3` with `rotation: glam::Quat` (parry math/mod.rs:94, glamx rot3.rs:9).
- **Degenerate trimesh input.** Empty, non-finite and out-of-range indices are filtered before parry (convert.rs:354-391). Zero-area triangles reach parry but are tolerated: `compute_pseudo_normals` skips `tri.normal() == None` (trimesh.rs:1450), and `triangle_normal_constraints` returns None for them (trimesh.rs:2001). Index-degenerate triangles are already dropped by the NIF producers (`finish_trimesh`, shape.rs:875-877).
- **One-sided `FIX_INTERNAL_EDGES` back-face discard is unchanged by the migration.** 0.17.6 `project_local_normal_mut` returns `dot_face >= 0.0`, and 0.31.1 has the same front-cone logic (`triangle_pseudo_normals.rs:34-45`, `pseudo_normals.rs:9-49`). `FIX_INTERNAL_EDGES_TWO_SIDED` is new in 0.31 and deliberately not used.
- **Every primitive goes through `clamp_shape_extent`.** Ball (convert.rs:244), Cuboid (:272-274), Capsule (:286-287), Cylinder (:299-300), and `character_capsule` (world/queries.rs:725-727, #4134 fixed). No production local `.max(1e-3)` remains; world/mod.rs:1242 is inside `mod tests` (starts at :1202).
- **`default_contact_skin_bu`** is applied to every collider part, primitive or mesh: register_newcomers (sync.rs:1123, 1166) and ragdoll bones (ragdoll.rs:441, 458). There is no other production `ColliderBuilder` in `crates/physics/src` or `byroredux/src`. water.rs:1580 and the world/mod.rs builders are all under `#[cfg(test)]`.
- **Contact combine.**
  - Havok's combine is `sqrt(a*b)` for both coefficients in both SDKs (hkpMaterial.inl lines above), so `GeometricMean` matches.
  - In rapier 0.36, `GeometricMean = 5` is the highest discriminant, and `rule1.max(rule2)` wins (coefficient_combine_rule.rs:51-75), so the doc claim at config.rs:66-68 is correct.
  - Both producers set both rules: sync.rs:1155-1156 and ragdoll.rs:455-456. No other production collider exists (character controller is a stack `Capsule` query shape, not a collider; probes are ray/shape casts; water has no colliders).
  - The player body (scene.rs:1170-1180) and NPC fallback capsule (npc_spawn.rs:468-472) go through `register_newcomers`, so they inherit the rule.
- **Collider `user_data` has exactly one meaning.** The writer is sync.rs:1164 (`u128::from(surface_material)`). The readers are world/queries.rs:230 and world/mod.rs:604. No bit-field collision with another consumer exists anywhere in `crates/` or `byroredux/src`. The other `user_data` hits are Vulkan or NIF wire fields. Ragdoll bones leave it at 0 by design (ragdoll.rs:421-432).
- **The `part_surface_materials` `serde(skip)` vs live overlay concern is disproved.** `RigidBodyData` is in `MUTABLE_DELTA_COLUMNS` (byroredux/src/save_io.rs:146), but `apply_deltas` remaps only `FormIdComponent`-keyed rows (crates/save/src/driver.rs:380-397). Bhk collision entities carry `PhysicsSourceForm`, not `FormIdComponent` (spawn.rs:1424-1440), so the overlay cannot clear their per-part sequence.
- **Material width per game.** `read_havok_material` reads the pre-10.0.1.2 `Unknown Int`, then a u32 (crates/nif/src/blocks/collision/mod.rs:83-88), which matches nif.xml's `HavokMaterial` struct (nif.xml:2293-2299). Oblivion (0-31) and FO3/FNV (0-127) fit a byte. Skyrim does not (see -02).

#### Dedup notes

- **`huge_finite_*` described as release-only in the brief**: they are default-lane and pass. This is not a finding; it is recorded in the guards table.
- **One-sided `FIX_INTERNAL_EDGES` drops back-face contacts**: same semantics in 0.17.6 and 0.31.1, so not a migration regression. Dropped.
- **`part_surface_materials` lost on save load (`serde(skip)` + verbatim `MUTABLE_DELTA_COLUMNS` overlay)**: disproved, because bhk collision entities have no `FormIdComponent` (spawn.rs:1424-1440), so `apply_deltas` cannot remap them.
- **Packed-proxy GlobalTransform seeded with scale 1.0** (spawn.rs:287): overwritten by propagation before `Stage::Physics` (#3959 chain, CLOSED). Not a double-scale and not a dropped scale. Dropped.
- **`pose_from_trs` normalising a zero quaternion yields NaN**: identical to nalgebra `UnitQuaternion::new_normalize` on the 0.22 path, so pre-existing. Body-pose finiteness belongs to the Existing: #5530 family. Compound child poses are guarded at convert.rs:235-236.
- **Ragdoll bone colliders carry user-data 0 and no `CONTACT_FORCE_EVENTS`**: by design per the `441d90c2f` commit message ("bones are not footstep surfaces"). The impact-silence consequence is covered by -03. Not filed separately.
- **`cast_ray_corridor` `Ball::new(NaN)` for a NaN `corridor_radius`** (world/queries.rs:296-302): a query shape, not a collider, outside Dim 1 (queries/diagnostics). Left for that dimension.
- **Havok convex radius ignored** (`bhkBoxShape.radius`, `bhkCylinderShape` padding radius): pre-existing, not in this delta, not investigated further.
- **Non-NIF static default friction 0.5 / restitution 0.3** (`RigidBodyData::static_body`, collision.rs:146-158): pre-existing fabricated defaults. GeometricMean vs Average shifts the effective restitution against a 0.8 object from 0.55 to 0.49. There is no Havok/landscape source to calibrate against (no-guessing rule). Not filed.
- **Existing issues matched**: none of the above match an open issue. #2550 (CLOSED, parser capture) is still in place and is the upstream of -01. #4686 (CLOSED) is still in place for the Ball/Capsule/Cylinder/`clamp_shape_extent` path; -05 is the compound-transform remainder plus the new compile break.


### Dim 2 — Step, Wake & the 4(+1)-Phase Sync Tick

**HEAD**: `65cadde2f` (code; the shared header's `455b473a1` code HEAD carries the same physics files) · **Delta reviewed** (`git show` each):
`ac81a09f9` (rapier 0.22→0.36), `52de1bc0a` (quarantine cooldown), `1efdfda4a` (#5352), `e128218eb` (#5353),
`9d826272b` (#5531), `4739c411a` (#5453), `e25d89d17` (contact counts), `04cb6333d` (geometric-mean combine),
`886edcded` + `441d90c2f` + `1291f370a` (surface material → user-data → footsteps + `ContactForceEvent` impacts),
`1291f370a`'s `impact_audio_system`, `738ab7391`, `f4c2b9931`, `26a1277bc`.

**Upstream read** (not assumed): `rapier3d-0.36.1` `pipeline/physics_pipeline/quarantine.rs`, `pipeline/event_handler.rs`,
`geometry/mod.rs:195-260` (`ContactForceEvent::started`), `geometry/narrow_phase/solver_graph.rs:455-500`
(`emit_contact_force_events`), `geometry/narrow_phase/contacts.rs:326-395` (begin-touch transitions),
`geometry/broad_phase_bvh/{mod.rs:235-263, update.rs:35-90}`, `pipeline/query_pipeline.rs` (`get_unknown_gen`),
`dynamics/island_manager/{manager.rs, sleep.rs}`, `dynamics/rigid_body.rs` (`sleep`, `set_enabled`, `set_linvel`,
`set_next_kinematic_position`), `dynamics/integration_parameters.rs:277-395`, `solver/staged_island_solver/worker.rs:640-700, 960-990`;
`parry3d-0.31.1/src/partitioning/bvh/bvh_insert.rs:152-560`; `rapier3d-0.22.0/src/geometry/narrow_phase.rs:415-460, 790-830, 1040-1090` (for the 0.22 baseline).

**Probes**: throwaway `rustc --test` binaries under the session scratchpad, linked against the repo's
`target/debug/deps` rlibs (no private target dir, no repo edit). Five probes; results quoted per finding.

#### Guards checked

| Guard | Result | Vacuity verdict |
|---|---|---|
| `scheduler_access_tests::physics_sync_declaration_reads_all_phase_and_diagnostic_types` | pass | Non-vacuous for its fixed needle list (slice = `physics.rs` tail + `late.rs` header). No new type entered `physics_sync_system` this delta; the contact-event buffer lives inside `PhysicsWorld` (already a declared write). It cannot see `footstep_system`'s new `PhysicsWorld` read (D2-06). |
| `scheduler_access_tests::contact_config_read_is_declared_on_both_physics_systems` | pass | Non-vacuous (exact count 2). |
| `cell_loader/rapier_release_tests.rs` (9) | pass | Non-vacuous (asserts body/collider/joint counts). The new per-body state (`quarantine_cooldown`, `pending_force_events`, `pending_query_leaves`) is not pruned by `remove_body`, but every consumer is generation-checked and drained each step — bounded, no finding. |
| `world/broad_phase_tests.rs::two_fixed_bodies_never_reach_the_narrow_phase` | pass | Non-vacuous: carries its own positive control (the dynamic ball's floor pair must exist). |
| `…::a_fixed_body_turned_dynamic_stays_on_what_it_rested_on` | pass | Non-vacuous: asserts the pair absent before and present after the type change, then 120 steps of resting (a missing pair would drop the crate ~1 300 BU). |
| `…::removing_a_fixed_body_with_withheld_pairs_is_a_no_op_for_them` | pass | Smoke-level only. Under 0.36 the removal is rapier-native, so the test fails only on a panic or a lost floor pair. Acceptable as a crash guard. |
| `sync::tick_documentation_tests::register_newcomers_parallel_section_holds_no_world_guard` | pass | Non-vacuous: cuts at the first `#[cfg(test)] mod`, so it scans production text only. |
| `world::tests::restored_pose_is_visible_to_ray_queries_same_frame` | pass | Non-vacuous: drives a real `step` recovery. |
| `world::tests::step_cost_rationale_is_scoped_to_history_and_names_the_real_cost_centre` | pass | **Partially vacuous since #5311**: it scans `step` only. `run_substep`, where the per-substep body now lives, and `recovery.rs` are unscanned (D2-08). |
| `world::tests::a_quarantined_body_is_re_enabled_and_parked` | pass | **Vacuous for the "cooldown armed a step" claim**: the second step runs on rapier's stale active set, not on the cooldown's wake, and it asserts `body.is_enabled()` (the flag), not that the collider is back (D2-01). |
| `ragdoll::tests::a_quarantined_rig_resting_on_a_floor_recovers_once` | pass | Non-vacuous for re-quarantine churn. It calls `w.wake()` every step, so it cannot see D2-01. |
| `world::tests::in_step_speed_cap_bounds_an_explosion_inside_one_step`, `a_restore_substep_still_clamps_the_bodies_it_did_not_claim` | pass | Non-vacuous. |
| `world::tests::contact_force_started_event_carries_surface_materials` | pass | Non-vacuous: one drop, one `started` event. |

Lane counts at HEAD: physics crate 220 pass; bin filters `physics_sync_declaration contact_config_read rapier_release impact footstep`
gave 27 pass, 0 fail.

#### Verified

- **Accumulator.** `frame_dt.max(0.0)` comes before the clamp (`world/mod.rs:895-899`). `f32::max` returns the non-NaN
  operand, so NaN and negative inputs add 0. `+inf` is clamped to `MAX_SUBSTEPS*PHYSICS_DT`.
- **Budget bail-out.** It runs after at least one substep and forfeits the backlog (`:1176-1179`). There is no other
  wall-clock read in the solver path.
- **No HashMap order feeds the solver.**
  - `explosion_offences`, `body_labels` and `keyframe_refusals_logged` are point lookups only.
  - `clamp_articulation_dofs`' `HashSet` is insert-only; iteration follows the `articulation_joints` Vec.
  - `quarantine_cooldown` is a Vec in rapier's report order.
  - The contact-event drain is a Vec in `force_event_pairs` order, and it feeds audio only.
- **Fast-path predicate holds against 0.36.**
  - `IslandManager::active_bodies()` exists. It is "dynamic and kinematic bodies currently active"
    (`manager.rs:270-277`).
  - Kinematics sleep only at exactly zero velocity (`rigid_body_components.rs:1488-1493`).
  - `has_awake_dynamics` filters to `is_dynamic()` (`world/mod.rs:625-629`), so an active kinematic never pins the
    fast path.
  - The fast path inserts only the queued leaves and zeroes the accumulator (`:961-965`).
- **Spawn exemption intact.**
  - `register_newcomers` builds dynamics `sleeping(true)` (`sync.rs:1099`) and calls only
    `queue_query_refresh` (`:1184`), with no `wake()`.
  - `wake_contract_tests::the_spawn_path_still_does_not_arm_pending_wake` pins it.
  - The contract's intent no longer holds under 0.36 for resting clutter (D2-02).
- **A same-frame wake armed before `step` is never swallowed.** `pending_wake` clears only when `steps > 0`
  (`:998-1000`). Wakes armed *inside* `run_substep` are swallowed (D2-01).
- **Query BVH (leaf insertion).**
  - `set_query_leaf` → `BroadPhaseBvh::set_aabb` handles a not-yet-inserted leaf correctly. Both parry paths insert a
    missing leaf and return `Inserted` (`bvh_insert.rs:524-536`, `:152-158`).
  - `set_aabb` then queues the handle in `pending_set_aabb`, so the next `update` treats it as changed for pair
    finding (`broad_phase_bvh/mod.rs:256-261`, `update.rs:64-72`).
  - Generation-checked `colliders.contains` drops stale handles.
- **Query BVH (removal on a no-substep frame).**
  - The stale leaf survives until the next update.
  - Every query resolves leaves through `ColliderSet::get_unknown_gen(leaf)?` (`query_pipeline.rs:109,123,303,392,433,561`).
    A dead slot yields `None`: no panic, no dead handle.
  - A reused slot resolves to its new occupant, which is tested at its real shape and pose, and whose own queued
    `set_aabb` overwrites the same leaf index.
  - `update` removes `removed_colliders` before processing `modified_colliders` sharing the index (`update.rs:53-57`).
- **`update_query_pipeline()` never runs per frame.**
  - Production callers are the exclusive or transition paths only: `register_newcomers_and_refresh_queries`
    (`scene.rs:1061`, `systems/character.rs:991`, `commands/view.rs:352`).
  - The `render/mod.rs` and `interaction.rs` hits are inside `#[cfg(test)]`.
- **Explosion recovery.**
  - The snapshot precedes every `pipeline.step` (`:1061-1088`), and the bound is a delta (`recovery.rs:63-67`).
  - Restored poses are propagated through `set_aabb` (`recovery.rs:220-232`), and every invalid handle's articulation
    is detached (`:173-178`).
  - The clamp runs on the recovery substep too (`:1165`, #5352).
  - The free-root DOF caps are in place (`recovery.rs:516-528`, #5353), and the rig attribution is in place (#5531).
  - The lifetime ladder is never cleared, `explosive_detaches_total` is gated on `rigid_body_link`
    (`recovery.rs:437-446`, #5356), the joints are swept (`recovery.rs:350-351`, #5355), and `remove_body` prunes
    labels, offences and refusals (`world/mod.rs:520-531`, #5355/#5272).
- **In-step cap.** `normalized_max_linear_velocity = 28_000/70 = 400` and `max_linear_velocity() = 400*70 = 28 000`
  BU/s, exact in f32 (`integration_parameters.rs:390-396`; `BETHESDA_UNITS_PER_METER = 70.0`, `core/src/lighting.rs:16`).
  - It equals rapier's 400 m/s default.
  - It is applied per solver substep to solver bodies (`worker.rs:648-668`), not to multibody links (`:690-697`).
  - Kinematic poses are not written back from the capped velocity (`worker.rs:977-984`), so a teleporting keyframed
    body still lands on its target.
  - A free fall reaches 20 000 BU/s only after ~29 s and ~291 000 BU, far past `KILL_PLANE_Y`. Authored arrows are
    ~6 000 BU/s. No legitimate body is capped.
- **Quarantine regressions.**
  - **Same-frame re-enable: does not occur.** A recovery substep returns `Stop`. A quarantine without a dynamic
    restore re-enables only after a later `pipeline.step` (`:1108-1118`).
  - **Removed body on the cooldown list: safe.** It is skipped by the generation-checked `bodies.get_mut`.
  - **Body in a multibody: detached** by the restore before it can be re-enabled.
  - **Left disabled: partially true** (D2-01).
- **Contact events.**
  - A sender whose receiver was dropped returns `Err` from `std::sync::mpsc::Sender::send`, which rapier discards with
    `let _ =` (`event_handler.rs` `ChannelEventCollector`). There is no panic, no spam and no growth, and no collider
    sets `COLLISION_EVENTS`.
  - The force channel is drained with `try_iter()` after every substep (`world/mod.rs:1097-1103`). Per step it carries
    one event per solver-active force-event pair whose impulse is above 0. The threshold stays at rapier's `0.0`
    default, compared with a strict `>` (`solver_graph.rs:479-485`). The count is bounded per step.
  - Only `started` is retained. The buffer is cleared at frame entry (`:960`).
  - Stale handles in a taken event are safe: `collider_surface_material` and `contact_event_position` both go through
    `colliders.get(...)?` (`:600-620`).
  - Threshold units are mass·BU/s². A probe of a mass-1 box dropped 50 BU gave one `started` crossing at
    17 240, where m·v/dt ≈ 15 700.
- **Kill plane.** Scanned only when `steps > 0`, over the active set, dynamics only (`:1010-1027`).
- **Phase order and locks.**
  - Phase order is collect/register → push kinematic → buoyancy → step → pull (`sync.rs:131-168`).
  - The newcomer read guards drop before the `PhysicsWorld` write (`sync.rs:990`, `:1057`). The parallel convert runs
    before the write (`:1040`).
  - No double registration (`:968`).
  - Pull writes `Transform` (`:1448`).
  - An absent `PhysicsWorld` returns early (`:115`).

#### Dedup notes

- **Swallowed cooldown wake (D2-01).** Searched open and closed issues for `quarantine` and `pending_wake`. #2856 and
  #2871 are the closed "wake consumed early" siblings, for external wakes. No match → NEW.
- **Spawn-asleep woken by begin-touch (D2-02).** Searched open and closed for `spawn asleep`, `settle storm` and
  `sleeping newcomer`. #3969 is the doc contract and #1698 is the settle-storm budget; neither covers the 0.36
  begin-touch wake. The plan's "Checked and ruled out" list covers only `sleep()`-after-reposition. NEW.
- **Footstep self-hit (D2-03).** Closed #2859 is the same class in the fog probe; its fix never reached this new
  query. No open match (#5347/#5348/#5349 are gate and doc issues). NEW.
- **Impact consumer and footstep declaration (D2-04, D2-06).** Searched open and closed for `impact sound` and
  `contact force`. Only #4743/#4744/#4700 (combat sounds) matched, which are unrelated. NEW.
- **Kinematic-teleport clamp (D2-05).** Not in `AUDIT_PHYSICS_2026-10-0{5,8,9}.md`, and no issue matches
  `explosive` + `kinematic`. NEW.
- **Dropped: `set_aabb` on a not-yet-inserted leaf.** Disproved; upstream handles it (Verified).
- **Dropped: stale BVH leaf after a no-substep removal.** Disproved. Queries resolve through `get_unknown_gen(..)?`,
  with no panic and no dead handle.
- **Dropped: dropped-receiver channel growth or panic.** Disproved. `let _ = send` returns immediately, and no collider
  enables `COLLISION_EVENTS`.
- **Dropped: in-step cap throttling legitimate bodies or kinematic teleports.** Disproved. 28 000 BU/s is exact; free
  fall needs ~29 s to reach even the 20 k cap; kinematic poses are not integrated from the capped velocity.
- **Dropped: quarantine cooldown holding stale handles after `remove_body`.** The handles are generation-checked and
  drained every substep, so they are bounded. Same for `pending_force_events` and `pending_query_leaves`.
- **Dropped: kinematic-touching islands never sleeping (0.36 whole-island sleep).** Real upstream behaviour, but any
  animating keyframed body already arms `wake()` every frame through `push_kinematic` (`sync.rs:1313-1316`), so the
  fast path does not change. No incremental impact.
- **Out of scope here.** `set_kinematic_translation` / `register_newcomers` unchecked positions are Existing #5530.
  The ABBA lane is #5535, owned by `/audit-concurrency`. The session-replace purge phantom collider is
  CONC-D5-2026-10-09-01.


### Dim 3 — Ragdoll Articulation & the Per-Game Constraint Seam

**Delta commits reviewed** (`git show`): `ac81a09f9` (rapier 0.22→0.36), `52de1bc0a` (quarantine re-enable one step later), `e128218eb` (#5353 free-root DOF caps), `9d826272b` (#5531 DOF clamp names the rig), `04cb6333d` (GeometricMean combine), `441d90c2f` + `886edcded` (surface material → collider user-data), `f4c2b9931` (comment sweep), `3efd17e5b` (#5470; no ragdoll-path code).

**Guards checked**

| Guard | Run | Verdict |
|---|---|---|
| `ragdoll.rs::ragdoll_solver_budget_is_local_and_preserves_authored_mass_and_limits` | pass (baseline 220) | **Vacuous on two counts.** It reads the per-body `additional_solver_iterations()` field and the global `num_solver_iterations`, so it cannot see that rapier 0.36 applies the extra substeps to every awake body (D3-04). Its expected limits come from `build_joint(..., false)` with symmetric ±π limits, so it cannot catch a sign error (D3-01). |
| `build_ragdoll_is_queryable_on_a_zero_substep_frame` | pass | Real. |
| `byroredux/src/ragdoll.rs::activation_before_first_physics_sync_prevents_duplicate_followers` | pass (bin ragdoll 30) | Real. |
| `exploding_articulation_dofs_are_capped_before_forward_kinematics`, `a_free_ragdoll_falls_at_gravity_without_tripping_the_dof_clamp`, `an_exploding_free_root_dof_is_capped_at_the_body_speed_cap`, `the_dof_clamp_attributes_each_rig_by_its_root_body`, `a_quarantined_rig_resting_on_a_floor_recovers_once` | pass | Real. |
| **FO3 real-data `ragdoll::installed_tests` (6, `#[ignore]`)** | `BYROREDUX_FO3_DATA=…/Fallout 3 goty/Data/ … -p byroredux --bin byroredux -- --ignored ragdoll::installed_tests` | **RED: 2 passed, 4 failed** (`on_flat_floor` yaw=0 step 431 distance 512.3; `in_saloon_geometry` step 429 516.5; `among_saloon_followers` step 429 516.5; `after_physics_world_reuse` step 429 516.5). `in_free_fall` and `when_created_before_restored_geometry` pass. See D3-03. Log: `/tmp/audit/physics/dim3_probe/fo3_installed_tests.log`. |

Note: the filter in this run's prompt and in the skill, `-- --ignored ragdoll_installed`, **matches 0 tests** and exits green ("0 passed … 2824 filtered out"). The module is mounted as `ragdoll::installed_tests` (`byroredux/src/ragdoll.rs:827-829`). That is a skill-drift item.

**Method.** All solver claims below come from a throwaway probe crate. It is built into the repo `target/` with `--target-dir`, with no private target. Sources are in `/tmp/audit/physics/dim3_probe/` (see its README). The probe:
- replays the installed-test fixture: the FO3 / FNV / Oblivion / Skyrim SE male `skeleton.nif`, seeded at bind pose like `activate_ragdoll`, on the test's flat floor, through `build_ragdoll` + `PhysicsWorld::step`;
- has a twin built from `git archive ac81a09f9^` (rapier 0.22) for before/after comparison.

The HEAD probe reproduces the failing test exactly: the worst body crosses 512 BU between tick 430 (511.6) and 435 (515.2), against the test's tick 431.

#### Verified

- **Seeding and frame composition survived the migration.** The only `ragdoll.rs` changes in `ac81a09f9` are mechanical: `iso_from_trs`→`pose_from_trs`, nalgebra→glam, and `scaled_axis`→`rotation_scaled_axis` (short-way canonicalised, `convert.rs:69-72`).
  - rapier `body_to_parent` is unchanged: `frame1·[lin]·R(θ)·frame2⁻¹` (0.36 `multibody_joint.rs:117-131` = 0.22 `:75-85`). So is the 3-DOF `integrate`/`apply_displacement` (0.36 `:134-181`).
  - Probe, FO3 skeleton with no limits and no floor: tick-0 max speed 10.7 BU/s (= g·dt) and max angular 0.0. The seeded reduced coordinates reproduce all 17 body poses exactly. No swapped frame was introduced.
- **`additional_solver_iterations(12)` semantics.** Dividing the substep dt is unchanged: 0.36 `staged_island_solver/init.rs:94-119` versus 0.22 `island_solver.rs:45-49`, so ragdolls run 4+12 = 16 substeps on both.
  - The global `num_solver_iterations` is untouched: `world/mod.rs:418-428` keeps the default of 4.
  - Authored mass is preserved (`ragdoll.rs:433`, `.mass(part_mass)`) and so are limits.
  - **Locality did NOT survive the migration** (D3-04).
- **`FREE_ROOT_DOFS` ordering** (`world/recovery.rs:29`, `:516-524`) matches rapier 0.36. A dynamic root becomes `MultibodyJoint::free` with `assembly_id = 0` and 6 DOFs inserted at row 0 (`multibody.rs:1299-1315`), and `integrate`/`jacobian` walk the linear DOFs `0..DIM` before the angular ones. Linear-then-angular is correct.
- **#5531.** `clamp_articulation_dofs` walks each multibody once, keyed by `multibody.root().rigid_body_handle()`, and returns `(root, dofs)`. The warning resolves `body_labels[root]` (`recovery.rs:473-489`, `:498-541`).
- **Teardown on rapier 0.36.** `RigidBodySet::remove` removes the body, then its colliders, then `impulse_joints.remove_joints_attached_to_rigid_body` and `multibody_joints.remove_joints_attached_to_rigid_body` (`rigid_body_set.rs:203-250`). The multibody removal goes through `rb2mb` and needs no live body (`multibody_joint_set.rs:284-303`). Ordering cannot panic or leak.
  - `PhysicsWorld::remove_body` passes `remove_attached_colliders = true` and prunes `body_labels` / `explosion_offences` / `keyframe_refusals_logged` (`world/mod.rs:507-543`).
  - `remove_ragdoll` calls it per body (`ragdoll.rs:883-887`), guarded by `build_remove_cycle_leaves_no_bodies_colliders_or_multibodies`.
- **Bone→body index mapping.**
  - `activate_ragdoll` builds `spec.bodies` in `template.bodies` order (`byroredux/src/ragdoll.rs:343-399`), and `build_ragdoll` returns `Ragdoll.bodies` in spec order (`ragdoll.rs:571-578`).
  - Writeback zips the two in the same order (`byroredux/src/ragdoll.rs:656`).
  - Constraint indices are remapped through `old_to_new` (`:189-204`).
  - `orient_tree` bounds-checks `body_a/b < n` (`ragdoll.rs:597`).
- **`ragdoll_extra_angular_damping`** defaults to `0.0` (`config.rs:125`, pinned at `:183-188`) and is added once per body at `ragdoll.rs:385-386`. Guard: `extra_angular_damping_is_added_once_per_body_not_once_per_joint`.
- **Seeds are validated before any articulation exists (#5161).**
  - Bin side: the relative and absolute check runs per body (`byroredux/src/ragdoll.rs:356-366`). A rejection counts one refusal and returns before the build (`:443-448`).
  - Builder side: the absolute backstop loops over every body before the first insert (`ragdoll.rs:360-369`).
  - Labels are `actor <id> bone <name>` (`byroredux/src/ragdoll.rs:373-377`, applied `:477-479`), over the builder default `entity N` (`ragdoll.rs:405`).
- **Activation drops every spec body's `RigidBodyData` recipe** even with no follower handles (`byroredux/src/ragdoll.rs:543-547`).
- **Writeback.** It reads `body_pose` and writes `GlobalTransform` (rotation + translation, inverse of the seed composition with the snapshotted `seed_scale`; `byroredux/src/ragdoll.rs:657-687`). There is no Z-up→Y-up conversion here; that is done once at import (`havok_to_engine`, `crates/nif/src/import/collision/mod.rs:699-701`).
- **FO4+.** `summarize_collision_authoring` counts `BhkNPCollisionObject` as `NewPhysicsStub` (`collision/mod.rs:122-152`). `nif_loader.rs:403-414` logs "packed-Havok collision object(s) authored but no ragdoll articulation … BhkSystemBinary" when `needs_packed_collision_fallback()` holds. Correct.
- **Constraint coverage** matches the physal.md §3 table.
  - `extract_ragdoll` handles Ragdoll, LimitedHinge (bare Hinge included, synthesized ±π) and Prismatic (`collision/ragdoll.rs:200-204`).
  - It declines BallAndSocket, StiffSpring, BallSocketChain and Other (`:220-223`, `:323-326`).
  - The malleable inner arms are 7/2/1/6 decoded and 0/8 decoded-then-declined; anything else stays `Other` (`constraints.rs:573-603`).
- **PHYSAL doctrine on the solver side holds.**
  - Grepping `crates/physics/src` and `byroredux/src/ragdoll.rs` for `GameKind|bsver|NifVersion|is_skyrim|havok_scale|GameVariant` finds nothing.
  - The new surface-material path adds no per-game branch on the solver side. `collision_shape_to_parts` takes an opaque `u8` default and leaf slice (`convert.rs:139-160`), and ragdoll bones ignore it (`ragdoll.rs:448`, `_surface_material`, stone by design).
  - The parse side does not translate the value at all (D3-05).

#### Dedup notes
- **FO3+ constraint motor-tail drift** (every FO3 skeleton parse WARNs "17 block(s) parsed Ok but consumed != block_size": `bhkMalleableConstraint` +5×14, `bhkRagdollConstraint` +1×3; `nif_stats --drift-histogram`): by design (`constraints.rs:755-765`). It is already recorded in AUDIT_PHYSICS_2026-10-09 § Skill drift and in AUDIT_NIF_2026-10-05/08. The skill line "Byte consumption exact … drift telemetry live" is still stale. Not re-filed.
- **#4772** (open, FO3 restore first-solve ~1e12 BU): D3-03's chain is its probable root cause. Under 0.36 the probe shows no 1e12 restore, only a clamp → detach. Filed D3-03 as NEW because the observable changed (the gate is now red, not masked by recovery). Cross-linked.
- **Neck cone violation (0.230 rad, FO3)**: part of the known cone/plane approximation (#1982 closed, physal.md "Known approximation"). Folded into D3-03, not filed alone.
- **Authored restitution 0.8–0.9 instability**: on the known-open register, with no issue. It contributes in D3-03, but softness alone fixes the fixture even at authored restitution, so it is not the root.
- **Considered and dropped:**
  - Ball-and-socket, stiff-spring and chain declines: no occupancy evidence, so not filed.
  - `ContactForceEvent` and ragdolls: ragdoll colliders set no `ActiveEvents`, so corpses make no impact sound. That is consistent with the "bones are not footstep surfaces" decision, not a defect here.
- **Per-component angular DOF clamp on a free root** (`recovery.rs:522`: each of three components capped at 100, so |ω| can reach 173 rad/s, versus a magnitude cap on bodies): minor and Dim 2-owned. Noted only.
- **Skill drift for /audit-sync**:
  - The installed-test filter must be `ragdoll::installed_tests`; `ragdoll_installed` matches nothing.
  - The Dim 3 guard `ragdoll_solver_budget_is_local…` does not test locality (D3-04).
- **Probe artifacts**: `target/debug/{ragdoll_probe,leak,ragdoll_probe_pre,leak_pre}` plus their deps (`/tmp`-path metadata) were built into the repo target. Sources and README are in `/tmp/audit/physics/dim3_probe/`. No repo file was edited.


### Dim 4 — Character Controller, Grounding & the NPC KCC (M42.10)

**Delta commits reviewed** (`git show`): `ac81a09f9` (rapier 0.22 → 0.36 KCC migration), `738ab7391` (at-rest grounded
pin + #3799 rationale), `002fbd6ef` (#4134 capsule clamp), `886edcded` / `441d90c2f` (surface material → collider
user-data → `surface_material_under`), `e25d89d17`, `04cb6333d` (GeometricMean), `dd3a506e9` (#5357), `26a1277bc`,
`4739c411a`, `f4c2b9931`. Upstream read: `rapier3d-0.22.0` and `rapier3d-0.36.1` `src/control/character_controller.rs`,
`src/geometry/interaction_groups.rs`, `src/pipeline/query_pipeline.rs`; `parry3d-0.31.1`
`src/query/shape_cast/shape_cast_support_map_support_map.rs`.

**Method note — probes.** Two throwaway crates under `/tmp/audit/physics/d4probe` (byroredux-physics + rapier 0.36,
through `PhysicsWorld::move_character` exactly as production calls it) and `/tmp/audit/physics/d4probe22` (rapier
`=0.22.0` with the pre-migration lockfile, replicating the pre-migration `move_character` parameters verbatim from
`git show ac81a09f9^:crates/physics/src/world/queries.rs`). Built with the pinned toolchain into the repo's
`target/`; no repo file touched. The 0.22 crate is the A/B baseline for every "regression" claim below.

**Guards checked** (all run on 0.36; `cargo test -p byroredux-physics -- kcc capsule surface_material character` →
22 pass; `-p byroredux --bin byroredux -- character locomotion player_body stuck_tests npc_locomotion_constants
footstep` → 92 pass):

| Guard | Verdict |
|---|---|
| `world/mod.rs::kcc_filter_groups_mask_actor_bone_colliders` | PASS, **non-vacuous on 0.36** — the unmasked arm asserts `blocked > -30` (mod.rs:3488-3492) and holds |
| `capsule_clearance_rejects_door_overlap_and_can_exclude_self` | PASS (positive control for the overlap query) |
| `capsule_clearance_ignores_sensors_and_live_actor_bones` | PASS (negative-only; the door test above is its positive control) |
| `kcc_reports_grounded_for_a_capsule_at_rest` (738ab7391) | PASS — but pins only `desired = Vec3::ZERO` at rest; see PHYS-D4-2026-10-11-03 for the walking input it does not cover |
| `grounded_capsule_does_not_sink_through_convex_floors_across_absolute_y` (#2857) | PASS, non-vacuous on 0.36 (`sank_old > 0` still holds) — **player policy only; no NPC variant** (see -01) |
| `character.rs` `integrate_vertical` / `horizontal_motion` / `support_probe_*` / #3799 tests | PASS |
| `locomotion.rs::stuck_tests` + `npc_locomotion_constants_match_their_sources` | PASS |
| `player_body.rs::attach_reparents_offsets_retargets_and_strips_collision` | PASS |
| `surface_material_under_reads_the_hit_colliders_material` | PASS, but vacuous for production (no player capsule in the fixture) — Dim 2's D2-03 |

#### Verified

- **KCC parameters, 0.22 → 0.36**: every field we rely on is set explicitly (`queries.rs:656-673`: `up: Vector::Y`,
  `offset/autostep/snap` as `CharacterLength::Absolute`, `slide: true`, both slope angles); the only defaulted field,
  `normal_nudge_factor`, is `1.0e-4` in both versions (0.36 `character_controller.rs:218`). `CharacterLength::eval`
  is unchanged (`Absolute(x) => x`). Defaults for the fields we override are identical in both versions, so no
  implicit default changed under us.
- **Upstream semantics that DID change** (none recorded in `dependency-update-followups-plan.md` §1.5 or its
  "Checked and ruled out" list except the first two): (a) grounded check now runs on the no-interference path
  (0.36 :401-412; 0.22 :318 broke out without it) and on a zero move (0.36 :429-440); (b) snap fires on
  `translation·up <= 0.0` (0.36 :466) vs `< -1.0e-5` (0.22 :371) — i.e. now on every purely horizontal move from a
  grounded start; (c) `check_and_fix_penetrations` is real code (0.36 :240-290, called at :315-323 when
  `|desired| < 1e-5`; 0.22 :182 was a commented-out stub); (d) `predict_ground` is `offset + 0.05` (0.36 :492) vs
  `offset * 1.1` (0.22 :400) — the grounded band shrinks from 4.4 to 4.05 BU at our 4 BU offset. (b) is the root of
  PHYS-D4-2026-10-11-01; (c) of -02; (d) contributes to -03.
- **Autostep effective values**: both versions add `offset` to `max_height`/`min_width` (0.36 :742-743, 0.22
  :655-656) → effective 36 BU step / 12 BU tread. Unchanged by the delta.
- **`InteractionGroups` 0.36**: `InteractionGroups::new` gained an `InteractionTestMode`; `test()` dispatches on the
  *collider's* mode (`self` in `QueryFilter::test` = collider groups, query_pipeline.rs:703; interaction_groups.rs:148-153).
  Every engine collider is `And`: bones `sync.rs:1127-1131`, everything else `InteractionGroups::all()` (= `And`,
  interaction_groups.rs:97-99), `ColliderBuilder` default `all()` (collider.rs:749). `test_and` equals 0.22's
  `test` (0.22 interaction_groups.rs:64-69). No `InteractionTestMode::Or` / `InteractionGroups::default()` (whose
  memberships changed to `GROUP_1` in 0.36) anywhere in `crates/` or `byroredux/src/`. Old semantics preserved.
- **KCC filter reaches every 0.36 sub-query**: the sweep, `handle_stairs`, snap, the grounded manifold scan and the
  new depenetration all go through the filtered `QueryPipeline` (`intersect_aabb_conservative` applies the filter,
  query_pipeline.rs:437). `handle_stairs`'s `EXCLUDE_DYNAMIC` mutates a by-value copy (0.36 :758), as in 0.22 (:670).
  The `queries.rs:683` cite (`:758`) is correct.
- **NPC step** (`locomotion.rs:189-202`): `filter_groups: Some(actor_move_interaction_groups())` = `ground_probe_groups()`
  (`world/mod.rs:144-149,177-179`); the airborne fallback ray is origin-lifted 256 and bounded 256+256
  (`locomotion.rs:214-221`); its `None` exclusion is correct because the fallback actor capsule also carries
  `ActorBoneCollider` (`npc_spawn.rs:473`). #5452 open (movers discard `blocked`) — not re-filed.
- `LOCOMOTION_NPC_*` derived from sources (`locomotion.rs:55-88`), pinned by `npc_locomotion_constants_match_their_sources`.
- **`capsule_overlaps_solid`** (`queries.rs:488-504`) queries `self.queries(solid_probe_filter())` = the broad-phase BVH
  (`queries.rs:92-99`); production callers (`scene/character_spawn.rs:116,505`) run after
  `register_newcomers_and_refresh_queries` (`character.rs:991`). On a no-substep frame leaves and collider poses are
  both "as of the last step", so the BVH is self-consistent; queued newcomers are inserted at the end of `step`
  (`queries.rs:132-140`).
- **Player**: `kcc_offset_bu = 4.0 > 2 × default_contact_skin_bu (1.0)` (`config.rs:100-124`, pinned by
  `kcc_offset_clears_the_combined_contact_skin`); walkable threshold is the named `scene::min_walkable_normal_y`
  (`character_spawn.rs:68-70`), used by the probe screen (`character.rs:453-454`); `MAX_FRAME_DT = 1/30`
  (`components.rs:177`, applied `character.rs:192`); terminal clamp (`character.rs:1586-1589`); diagonal input
  normalised (`character.rs:1097`); `player_accepts_movement_input` honours `PlayerControlState` + `restrained`
  (`character.rs:142-151`); `snap_character_body_to_camera(&World)` by documented design (`character.rs:840-845`);
  `toggle_player_mode(&mut World)` reachable only from the keyboard handler on `&mut self.world`
  (`app_events.rs:562`); `refresh_player_only_bases` in `player_derived_stats_system` (`character.rs:118-140`,
  scheduled `boot/schedule/update.rs:517`); `CharacterLevel`/`Background` stamped (`inventory.rs:383-391`); HUD via
  `PlayerVitals` (`hud.rs`, `scaleform_hud.rs`).
- **#3799 OR after 738ab7391**: the OR (`character.rs:1686`) is intact; `support_probe_enabled` keeps its three
  suppressions (`character.rs:1662-1664`). Jump apex / launch are not eaten by 0.36's snap: launch frame is upward
  (no snap, 0.36 :466), and an airborne start is outside the 4.05 BU grounded band so `grounded_at_starting_pos`
  is false. Probe P4: idle player at rest, production probe correction, cuboid + trimesh floors at absolute
  offsets 0 / 3e3 / 1e4 / 1e5 BU → KCC `grounded` held 120/120 frames everywhere. The probe and rapier's snap do
  double-count slightly on convex floors (snap lands 0.07 BU inside the offset, the next correction lifts it back:
  a 0.07 BU two-frame cycle) — negligible.
- **#4134** fixed in place: `character_capsule` → `clamp_shape_extent` (`queries.rs:725-728`), guard passes.
- **#5357** fixed in place: marker drift at full strength beside the fraction-scaled plane drift (`character.rs:1126-1147`).
- **GeometricMean (`04cb6333d`)**: no effect on player/NPC "wall friction" — the KCC is a geometric collide-and-slide
  over scene queries; collider friction/combine rules never enter `move_shape`. It only changes solver contacts
  between the kinematic player capsule (`friction: 0.5`, `scene.rs:1170-1180`) and dynamic bodies it pushes.
- **Third-person body**: visual-only; `attach_player_body` strips `CollisionShape`/`RigidBodyData` on every
  descendant before the first sync (`player_body.rs:278-298`), and mid-life player gear does the same
  (`npc_spawn/loot_appearance.rs:966-972`). `THIRD_PERSON_BOOM_BU`
  unswept (documented, `character.rs:757-762`). `player_camera_boom` is the single source: `camera_follow_system`
  (`character.rs:762`) and `interaction::camera_ray` (`interaction.rs:1196`); every gameplay ray caller starts at the
  eye (`interaction.rs:1076,1138`, `combat.rs:246`, `commands/view.rs:68,316` via `camera_pos = body + eye_height`
  `view.rs:369`) — no raw-camera-pose caster.

#### Dedup notes

- **Footstep surface self-hit** — already PHYS-D2-2026-10-11-03 (per coordinator); not re-filed. *Extends
  PHYS-D2-2026-10-11-03* with: (a) per-part resolution is correct — each part is its own rapier collider carrying
  its own byte (`sync.rs:1135-1167`), so the hit collider *is* the hit part, never part 0; but
  `BhkPackedNiTriStripsShape` and `BhkCompressedMeshShape` collapse their per-sub-part / per-chunk materials to the
  first non-zero one (`crates/nif/src/import/collision/shape.rs`, `shape_leaf_materials_inner`), so a mixed
  stone/wood packed mesh reads one material everywhere — documented as intended in `441d90c2f`; (b) there is no NPC
  footstep path — the only `FootstepEmitter`s are the camera (`scene.rs:995`) and the player body (`scene.rs:1245`);
  (c) BVH staleness cannot change the outcome: the origin is the capsule centre, inside the capsule on every frame
  whether or not its leaf moved this frame.
- `impact_audio_system` both-0 skip (D1-03/D2-04) and `footstep_system` access row (D2-06): not re-filed.
- #5452 (movers discard `blocked`), #5535 (lock-order lane): open, not re-filed.
- **Phantom-slope lift as a 0.36 regression** — disproved by the 0.22 A/B (0.22 was worse); filed only as the
  documentation/test-gap finding -03.
- **Grounded margin 4.4 → 4.05 BU breaking idle grounding at large coordinates** — disproved (P4: 0/120 lost at
  0 / 3e3 / 1e4 / 1e5 BU, cuboid and trimesh).
- **#4134 completeness** — the clamp sanitises only the parry shape; `feet_y` / spawn arithmetic still use the raw
  `CharacterController` extents, so a NaN extent from a corrupt save would still reach the Transform. Outside
  #4134's stated scope (the shape), corrupt-save-only, not filed.
- **Snap eating jump apexes / stepping down through stairs** — disproved: the launch frame is upward (no snap),
  airborne frames start outside the grounded band, and the player's step-down is bounded by the probe
  (`character.rs:468`); P8 shows no player sink on 336 convex-floor configurations.
- **`InteractionTestMode` default mismatch** — disproved (see Verified).


### Dim 5 — WATAL Physics Sink: Buoyancy, Damping, Current, Swim

**Delta commits reviewed**: `ac81a09f9` (rapier 0.36: water.rs diff is the `translation()` by-value change, comment wording and the test-fixture `pose_from_trs` swap only; the force, damping, wake and AABB calls are unchanged), `dd3a506e9` (#5357 swim drift, already verified by the 10-09 report and re-checked here), `441d90c2f` / `886edcded` / `1291f370a` (surface material in collider user-data and `ContactForceEvent` impact sounds; no water.rs or character.rs water path touched), `738ab7391` (KCC at-rest grounded pin; only `resolve_ground_contact` docs and tests in character.rs; the swim suppression in `support_probe_enabled` is unchanged).

**Upstream semantics checked (source, not assumed)**:
- `rapier3d-0.36.1/src/dynamics/rigid_body.rs:1286-1362` (`reset_forces` / `add_force`) and `rapier3d-0.22.0/src/dynamics/rigid_body.rs:937-945,963`: `user_force` accumulates. Neither version clears it automatically. 0.36 pins this upstream with `pipeline/physics_pipeline/test.rs:561` `user_force_persists_across_steps`.
- `rigid_body_components.rs:1053-1056` (0.36): effective force = `user_force + gravity·mass·gravity_scale`, recomputed each step.
- Damping is applied once per step as `v / (1 + dt·c)` at `rigid_body_components.rs:859-863`, from `staged_island_solver/worker.rs:974`. **That loop `continue`s for multibody links** (`worker.rs:956-962`). The 0.22 equivalent is `velocity_solver.rs:295-312`, where the multibody branch writes generic velocities with no rigid-body damping. Multibodies use only their own per-DOF `damping` vector (`multibody.rs:621`), whose default is 0.1 on angular DOFs only (`multibody_joint.rs:273-285`).
- `num_solver_iterations` default is 4 in both versions (`integration_parameters.rs:416` in 0.36, `:284` in 0.22): every engine substep runs 4 internal small steps with the force held constant. Ragdoll bodies request `additional_solver_iterations(12)` (`ragdoll.rs:394`), which makes 16 (see PHYS-D3-2026-10-11-04).
- `Collider::compute_aabb()` = `shape.compute_aabb(&self.pos)` (`collider.rs:597-599`). `co.pos` is rewritten from the parent body at the end of every internal substep (`pipeline/physics_pipeline/substep.rs:135-142,196-204`). Probe (`collider_aabb` probe, 5 steps of a falling ball): body Y and AABB-centre Y were identical (diff 0.0000) after every step. A body teleported between steps (`set_translation`) keeps a stale `co.pos` until the next step's user-change pass. That is unchanged from 0.22, and only restore/park frames are affected.

**Guards checked** (pinned toolchain, repo target dir):

| Guard | Result | Vacuous? |
|---|---|---|
| `scheduler_access_tests.rs::water_and_animation_parallel_accesses_are_complete` | pass | No. The physics slice is `physics.rs` after `physics_sync_system,` plus `late.rs:1-16`. All four needles occur only in the physics declaration (`late.rs:5` imports `TotalTime` but not as `.reads_resource::<TotalTime>()`). |
| `character.rs::swim_damping_is_frame_rate_independent` | pass | No |
| `character.rs::zero_dt_tick_while_submerged_does_not_refill_breath` | pass | No (`advance_breath` `dt <= 0` arm, `character.rs:1518-1532`) |
| `character.rs::swimming_drift_scales_the_plane_by_fraction_and_the_marker_at_full_strength` | pass | No |
| `character.rs::the_marker_reaches_the_swimmer_through_drift_not_the_column_state` | pass | No |
| `character.rs::a_marker_outside_every_plane_still_drifts_the_player` | pass | No |
| `character.rs::swimlevel_predicate_agrees_between_pose_and_depth_forms` | pass | No |
| `water.rs::placed_current_volume_is_collected_without_becoming_a_water_surface` | pass (within `-p byroredux-physics water`: 29/29) | No |
| `submersion_runs_after_camera_follow_and_before_water_audio` | pass | No |

#### Verified

- `submerged_fraction` clamps to [0,1], and a zero-height AABB divides by `max(1e-6)`, so the result is 0 or 1 through the clamp (`water.rs:278-285`).
- Lift is `clamp(frac)·ratio·m·|g|` on +Y, which opposes gravity `-686.7` in Y-up (`water.rs:168-176`, `world/mod.rs:454`).
- Phase order is collect/register → push kinematic → buoyancy → step → pull (`sync.rs:131-168`). The `n_new > 0` escape is still in place (`sync.rs:152`) and is consumed by the quiesced fast path's `had_newcomers` term (`water.rs:679`).
- **Force persistence across substeps**: buoyancy and current forces are applied for all N substeps of a frame, not just the first. `PhysicsWorld::step` never calls `reset_forces` between substeps (`world/mod.rs:976-1000`), and rapier keeps `user_force` in both 0.22 and 0.36 (above). Lift is not scaled by N, and nothing here changed in the migration. The consequence of holding the force constant is filed as PHYS-D5-2026-10-11-02.
- Wake discipline:
  - Every per-frame force is applied with `wake_up = false` (`water.rs:1004,1084`).
  - A body is woken once on the dry→wet edge, or when a sleeping body's depth moves more than 0.1 BU (`water.rs:955-961`).
  - `pw.wake()` fires only on frames where some body was woken (`water.rs:1098-1100`).
  - `reset_forces` is scoped to bodies water is acting on (`water.rs:934-936`).
- Two target sources. The first is the `RapierHandles` + `RigidBodyData{Dynamic}` scan (`water.rs:708-728`). The second is the `Ragdoll` scan over `ragdoll.bodies` zipped with `ragdoll.buoyancy` (`water.rs:752-771`); `RagdollBuoyancy` caches the first collider and the effective damping (`ragdoll.rs:378-396,468`). `clear_stale_water_contacts` has the matching `Ragdoll` arm (`water.rs:492-514`).
- One `reference_point` (collider AABB centre) feeds the union prefilter (`water.rs:843-846`), the current-volume containment (`:848-863`) and the surface XZ containment, and is passed to `surface_y_at` (`:880-887`). There is no revert to the body origin on those three. The remaining origin reads are filed as PHYS-D5-2026-10-11-05.
- Current drag is bounded:
  - The target speed is clamped by `WaterFlow::new` to `SPEED_MIN..=SPEED_MAX` (0.5..=25 BU/s, `core/.../water.rs:543-575`), and every production producer goes through it. The only `WaterFlow { .. }` literals are in tests.
  - The response rate is the named `PhysicsWaterConstants::current_drag` (4.0/s).
  - The response is stable under the per-frame hold: c·T ≤ 4 × 5/60 = 0.33.
- `WaterContact` emits one transition frame at zero. It is written on wet→dry in both the in-band and exited arms (`water.rs:1027,1040`) and on plane streamed-out (`water.rs:529-533`). `make_water_interaction_system` derives splash edges from its own `wet_last_frame` set (`systems/water.rs:411-541`).
- Mesh-water height: buoyancy (`water.rs:880`), `player_water_state` (`character.rs:1236`) and `submersion_system` (`systems/water.rs:206`) all call `WaterVolume::surface_y_at`. None uses a flat `max.y`.
- Player sampler:
  - The marker never enters `PlayerWaterState`. `player_current_drift` = plane × fraction + marker × 1.0 in the swim arm, and marker × 1.0 in the wade arm (`character.rs:1126-1146`). This matches `current_force(.., 1.0, ..)` at `water.rs:1077`.
  - A missing `WaterCurrentVolume` storage is tolerated: the `?` is inside `placed_current_flow_at` (`character.rs:1158-1174`), not inside `player_water_state`.
- Swim and drown:
  - `swim_vertical_velocity` uses `exp(-SWIM_DAMPING·dt)`, which is dt-correct (`character.rs:1461-1495`).
  - `advance_breath` with `dt <= 0` preserves breath and the damage remainder (`:1518-1532`).
  - Both death producers (`systems/water.rs:62`, `character.rs:1565`) only `queue_dead_actor_reconciliation`. The single reconciler is `reconcile_pending_dead_actors_system` (`late.rs:214-228`, Late exclusive, after `water_damage_system` at `late.rs:200-210`). There is no inline teardown.
- Late order is ragdoll writeback → submersion → water_damage → death reconcile → water_interaction → water_audio (`late.rs:144-251`). Submersion follows `camera_follow_system` (guard passes).
- Impact sounds vs water: water is never a rapier collider. Phantoms are dropped at the shape boundary (`nif/src/import/collision/shape.rs:461-470`; real Skyrim water NIFs `water1024.nif` and `tundrastreamstraight01waterb.nif` author only `bhkSimpleShapePhantom`, seen via `water_mesh_probe`). `WaterVolume` / `WaterPlane` carry no `CollisionShape` (`cell_loader/water.rs`). A splash is therefore only the `WaterContact` edge. `CONTACT_FORCE_EVENTS` is set only on colliders of `MotionType::Dynamic` bodies (`sync.rs:1143-1150`), so a body entering water produces no impact event. A body striking the lakebed does produce one.
- Contact-wake (PHYS-D2-2026-10-11-02) vs a float resting against a shore: a sleeping float whose shore contact already exists gets no new begin-touch, so the contact-wake rule does not keep it awake. Re-wakes on wavy water come from the per-frame depth rule (PHYS-D5-2026-10-11-04), not from contacts.
- `commands/water.rs` is read-only in production (every `insert` is under `#[cfg(test)]`).

#### Dedup notes

- **#5354** (flowing-water weather damping missing from buoyancy / `player_water_state`) is OPEN and not re-filed. `submersion_system` applies `FLOWING_WATER_WEATHER_TRANSPORT` (`systems/water.rs:212-222`); buoyancy and the player sampler do not.
- **Force persistence 0.22 → 0.36**: disproved as a migration risk. Semantics are identical, and persistence across substeps is correct; the hold consequence is D5-02.
- **Collider AABB freshness in 0.36**: disproved. The probe shows AABB centre == body after every step. Teleport lag is identical to 0.22 and limited to restore frames.
- **Water bodies generating impact `ContactForceEvent`s**: disproved. Water has no rapier collider, and phantoms are dropped at `shape.rs:461-470`.
- **Floating body touching the shore never sleeps under 0.36 contact-wake**: disproved for persistent contacts (no begin-touch while asleep). New-contact wakes are PHYS-D2-2026-10-11-02's scope.
- **Fly-cam with the player standing in hazard water**: the stale player `WaterContact` keeps `water_damage_system` damaging the player while the controller is idle. Dropped as its own finding: it is the same root cause as D5-03, and the body physically remains in the water.
- **Ragdoll `WaterContact` stranded on bone entities after ragdoll removal**: disproved. Re-ragdoll rewrites the same bones (`byroredux/src/ragdoll.rs:459-471`), and unload despawns them.
- **Current-drag stability under the per-frame hold**: c·T ≤ 0.33, which is stable. Not a finding.
- **Sampler parity, wind**: the dynamic path applies `wind_force` to the exposed fraction and the player does not. This is design (the KCC has no wind response on land either) and is not in the brief's parity list. Not filed.
- **Skyrim water NIF collision**: the real-data check (`water_mesh_probe` on Meshes1) shows only a `bhkSimpleShapePhantom` plus a box/transform shape tree feeding it. No layer-11 rigid body was seen in the three meshes checked; this is Dim 1's area if a wider census is wanted.
- **Seams, pointers only**: tri-state `XCLW` water-height decode → `/audit-esm`. The rendered crest vs the CPU sampler (#3207 coherence) → `/audit-renderer` Dim 8.


### Dim 6 — Queries, Diagnostics & Cost

**HEAD**: `65cadde2f` (code; the physics files are identical to the shared header's `455b473a1`).

**Delta reviewed** (`git show` each):
- `e25d89d17`: the rapier pair/contact/constraint trio in `phys.stats`.
- `ac81a09f9`: the query API migration (`QueryPipeline` became a `BroadPhaseBvh`-borrowed view). Every wrapper was diffed against `git show ac81a09f9^:crates/physics/src/world/queries.rs`.
- `1291f370a`, `441d90c2f`, `886edcded`: the surface material reaching collider user-data and `surface_material_under`, plus the `ContactForceEvent` channel.
- `04cb6333d`: the GeometricMean combine rule.
- `f4c2b9931`: the comment sweep.

**Upstream read** (not assumed):
- `rapier3d-0.36.1`:
  - `pipeline/query_pipeline.rs` (whole file)
  - `geometry/broad_phase_bvh/{mod.rs, update.rs:35-330}`
  - `pipeline/physics_pipeline/{substep.rs:80-400, solve.rs:140-160, 360-420, quarantine.rs}`
  - `geometry/collider.rs:623-651`
  - `geometry/interaction_groups.rs:60-165`
  - `control/character_controller.rs:730-800`
  - `counters/{mod.rs, timer.rs}`
  - `dynamics/solver/staged_island_solver/init.rs:30-120, 585-610`
  - `dynamics/solver/solver_contact_graph.rs:204-211`
  - `dynamics/rigid_body_components.rs:1460-1500`
  - `dynamics/island_manager/sleep.rs:1-80`
  - `dynamics/rigid_body.rs:1120-1240`
  - `dynamics/rigid_body_set.rs:427-438`
- `parry3d-0.31.1`: `query/shape_cast/shape_cast.rs:127-169` and `query/ray/ray_trimesh.rs`.
- `rapier3d-0.22.0` (the baseline):
  - `pipeline/query_pipeline/{mod.rs:73-102, 348-385, 590-607, generators.rs}`
  - `dynamics/island_manager.rs` (`active_kinematic_set`)
  - `geometry/interaction_groups.rs:72-76`
  - `pipeline/physics_pipeline.rs:58`

**Probe**: one throwaway `rustc --test` binary at `<session scratchpad>/d6probe/probe.rs`. It links against the repo's `target/debug/deps` rlibs (`libbyroredux_physics-d2cbf87e…`, `librapier3d-692ae5e7…`). No private target dir was used and no repo file was edited. Results are quoted per finding.

#### Guards checked

| Guard | Result | Vacuity verdict |
|---|---|---|
| `sync.rs::census_excluded_body_prevents_a_self_hit_on_the_players_own_capsule` | pass | **Non-vacuous.** It carries its own positive control: without the exclusion it must not report `hit NOTHING`. `physics_sync_system(&world, 0.0)` steps 0 substeps, so the capsule's leaf arrives through `insert_pending_query_leaves`. The queued-leaf path is therefore exercised too. |
| `sync.rs::census_reports_full_extent_and_scale_not_just_the_y_range` | pass | **Non-vacuous.** It asserts the literal `extent=[20, 20, 20]` and `scale=2.00`. |
| `world::tests::corridor_cast_hits_a_bone_the_zero_width_ray_threads` | pass | **Non-vacuous.** Positive control: the plain ray must miss. The TOI band `83.0..=83.2` also pins that `ShapeCastOptions::max_time_of_impact` is BU along a unit direction under parry 0.31. |
| `world::tests::step_cost_rationale_is_scoped_to_history_and_names_the_real_cost_centre` | pass | **Non-vacuous for its needles.** It scans `step` only; that gap is D2-08, not re-filed here. |
| `world::tests::kinematic_count_doc_agrees_with_the_fast_paths_own_rationale` | pass | **Non-vacuous, but it now pins false text.** It requires "not an awake count" and "never empty", both of which rapier 0.36 falsified (D6-01). |
| `restored_pose_is_visible_to_ray_queries_same_frame`, `post_step_ray_finds_a_moved_dynamic_body_with_the_incremental_query_path`, `recovery_snapshot_index_tracks_dynamics_not_the_arena`, `surface_material_under_reads_the_hit_colliders_material`, census tally/sort/cap tests | pass | 19 pass under the filter `census corridor step_cost restored_pose post_step_ray recovery_snapshot_index surface_material`. |

#### Verified

**Query wrappers before and after the migration**

- **`solid` flag.** Every ray wrapper still passes `solid = true` (`queries.rs:208, 229, 261, 355`).
  - parry 0.31 composite ray casts are best-first over parts, with each part tested at `co.position()` (`query_pipeline.rs:112-128, 228-237`).
  - Convex parts return `toi = 0` from inside.
  - TriMesh is a composite of triangles with no containment test (`ray_trimesh.rs:8-26`), so a ray from "inside" a shell is not a hit.
  - Same as 0.22.
- **TOI units.** Directions are unit vectors at every wrapper:
  - `cast_ray` and `cast_ray_corridor` call `normalize_or_zero` (`queries.rs:250, 292`).
  - `line_of_sight_blocked` divides `delta / distance` (`:348`).
  - The downward probes use `Vector::NEG_Y` (`:199, 226, 466`).
  - So `max_toi` and `max_time_of_impact` are BU and `PhysicsRayHit::distance` is BU, as its doc says.
- **`QueryFilterFlags::test` is byte-for-byte the 0.22 logic** (0.36 `query_pipeline.rs:617-646` against 0.22 `mod.rs:73-102`).
  - `EXCLUDE_SENSORS`, `EXCLUDE_DYNAMIC` and `EXCLUDE_FIXED` keep their meaning, including "parentless counts as fixed".
  - `exclude_rigid_body` and `exclude_collider` are unchanged (`:697-707`).
- **`InteractionGroups`.**
  - The probes build an explicit `InteractionTestMode::And` (`world/mod.rs:144-149`).
  - `QueryFilter::test` evaluates `collider_groups.test(filter_groups)`. With the collider's `And` mode that is `test_and` (`interaction_groups.rs:148-154`), the 0.22 rule.
  - 0.36 changed `InteractionGroups::default()` memberships from `ALL` (0.22 `:72-76`) to `GROUP_1` (0.36 `:157-164`). That only affects ragdoll colliders, which set no groups. `GROUP_1 & (ALL & !GROUP_32) ≠ 0`, so ground probes still see corpses and nothing else is affected.
  - Live bones are `(GROUP_32, ALL, And)` (`sync.rs:1126-1131`), which the mask removes.
- **`ShapeCastOptions`.** The struct and its semantics are unchanged (parry `shape_cast.rs:129-147`). Both casts set every field explicitly (`queries.rs:313-318, 468-473`).
  - The corridor sets `stop_at_penetration: false`: a start-overlap with a separating direction is ignored and a closing one hits at 0.
  - `normal1` is world-space on the hit collider in both versions (0.22 `mod.rs:593-594`, 0.36 `query_pipeline.rs:457-459`). Every consumer reads `normal_y.abs()` (`queries.rs:430`, `sync.rs:549`, `systems/character.rs:453-454`), so the trimesh winding sign does not matter.
- **`capsule_overlaps_solid`.**
  - 0.22's `intersection_with_shape(..).is_some()` became `intersect_shape(..).next().is_some()`.
  - This is a lazy BVH-leaf walk, then `filter.test`, then the exact `intersection_test` (`query_pipeline.rs:551-575`).
  - Every engine shape is support-map or composite, so `.is_ok_and` never hides an `Unsupported`.
- **KCC.** The only filter mutation 0.36's KCC makes is `EXCLUDE_DYNAMIC`, and only on the autostep sub-query copy (`character_controller.rs:745-758`). The `move_character` doc citation at `:758` is accurate.
- **Disabled colliders (rapier quarantine).**
  - `QueryFilter::test` has no `is_enabled` check (`query_pipeline.rs:697-707`).
  - A disabled collider leaves queries only when the next `broad_phase.update` removes its leaf. The disabled list counts as removed (`substep.rs:379-387`).
  - `set_query_leaf` refuses disabled colliders (`queries.rs:150-152`), and `update_query_pipeline` walks `iter_enabled` (`:116`).
  - No wrapper relies on disabled colliders being invisible. The one-step window where a quarantined body is still queryable sits at its rolled-back pose (`quarantine.rs:146-152`), so it is harmless. The quiet-scene variant is D2-01.

**Query freshness**

- **Under 0.36 a stale leaf can only MISS.** The BVH leaf (fat AABB) only culls. Each candidate is tested at its live `co.position()` (`query_pipeline.rs:112-128, 561-566`).
  - The engine keeps the leaf consistent with `co.pos` at every writer.
  - Rapier's end-of-step `set_aabb` covers every active body, kinematics included (`substep.rs:124-153, 241-250`).
  - `refresh_query_geometry_after_restore` propagates and re-leafs restored bodies (`recovery.rs:220-232`).
  - `set_position` and `set_next_kinematic_*` do not touch `co.pos` before the next step (`rigid_body.rs:1180-1240`).
  - So the task premise ("a stale leaf hits where the collider WAS") does not occur in 0.36.
- **Kinematic pushes and teleports lag rapier by up to one tick, the same as 0.22.**
  - Player teleports go through `set_kinematic_translation`: `systems/character.rs:907, 1036`, `save_io.rs:781`, `commands/view.rs:482`.
  - Every query that could meet the moved capsule excludes it:
    - door arrival `Some(player)` (`systems/character.rs:999`)
    - `combat.approach` `Some(player)` (`commands/view.rs:362`) and the corridor `excluded_body` (`:297-319`)
    - melee (`combat.rs:229-252`)
    - interaction (`interaction.rs:1065-1076, 1126-1144`)
    - fog (`render/mod.rs:284-298`)
    - faction LOS (`faction_hostility.rs:334-345`)
    - the player support probe (`systems/character.rs:444-449`) and KCC (`:508`)
- **Door and coc arrival** re-leafs all enabled colliders (`register_newcomers_and_refresh_queries`, `sync.rs:228-246`) before probing.
- **Ragdoll activation** queues its colliders (`ragdoll.rs:560-569`).
  - The removed keyframed bones' stale leaves resolve through `get_unknown_gen` to whichever collider reuses the slot.
  - rapier removes them before re-inserting same-index newcomers (`update.rs:53-57`).

**Self-exclusion and the census**

- **Every controller cast excludes its caster.** The player passes its body or collider (`systems/character.rs:449, 508`). NPCs use `actor_move_interaction_groups` / `solid_probe_filter` (`locomotion.rs:199-201, 215-219`).
  - `phys.census` passes `Some(player_body)` whenever the reference is the player (`commands/physics.rs:128-132, 157`).
  - The boot census passes `None` because no capsule exists yet (`scene/character_spawn.rs:596-598`, boot-only caller `scene.rs:1061-1071`).
  - `cast_ray_corridor` excludes one body only (`queries.rs:304-307`). Its only callers are the player swing and `combat.approach`. The player has no bone colliders.
- **The census separates its three cases:**
  - no hit (`sync.rs:782-785`)
  - non-walkable (`:773-781`)
  - authored totals, registry-scoped and refusing the drop claim (#4684, `:805-837`)
  - It also renders full extent plus owner scale (`:846-869`).

**Cost**

- **`colliders_near_xz`** allocates and walks the whole arena per call (`queries.rs:570-624`). Its only production caller is the census (`sync.rs:688`): `phys.census` on demand, and the boot all-rungs-missed path (`scene/character_spawn.rs:584`). There is no per-frame trace and nothing to report to `/audit-performance`.
- **`static_colliders_aabb`** (an O(n) walk) has these non-per-frame callers:
  - `phys.stats`
  - the spawn ladder
  - `character.rs:546` (frame 0)
  - the bench subject-distance retry, until it resolves (`app_step.rs:523`)
- **Cost model holds under 0.36.**
  - There is no separate query tree (`queries.rs:92-99`).
  - A no-step frame inserts only the queued leaves (`world/mod.rs:961-964, 1033-1038`). A stepped frame drops them because rapier's update inserted them as modified colliders.
  - `update_query_pipeline` never runs per frame (D2 verified the callers).
- **Snapshot index.**
  - `dynamic_bodies` is pushed at all three production dynamic-creation and flip points: `sync.rs:1104-1106`, `ragdoll.rs:409`, and `world/mod.rs:818-820` (`set_motion_type`, the only production `set_body_type`).
  - It is pruned every substep (`:1061`).
  - Ragdoll removal goes through `remove_body`, so the prune catches it. There is no other production type flip; the "ragdoll root pin" named in the field doc exists only in a test (`ragdoll.rs:1229`).
- **`contact_counts` costs nothing per frame on the engine side.** It reads three `usize` (`world/mod.rs:577-584`).
  - Rapier fills them per step behind `counters.enabled`, which is the default in both 0.22 (`physics_pipeline.rs:58`) and 0.36 (`physics_pipeline/mod.rs:110`).
  - The only walk is rapier's per-step sum over active manifolds (`init.rs:600-610`).
  - Stage timers compile out without the `profiler` feature (`counters/timer.rs:6-56`).

#### Dedup notes

- **"A ray on a stale leaf hits where the collider WAS"**: disproved for 0.36 (see Verified: Query freshness). The leaf only culls, the exact test uses `co.position()`, and the engine never writes `co.pos` without re-leafing. The failure mode would be a miss, and no writer produces one.
- **Census lists disabled (quarantined) colliders without a flag** (`colliders_near_xz` and `static_colliders_aabb` iterate `colliders.iter()`, `queries.rs:515, 579`): true but dropped.
  - The only realistic producers are D2-01's parked dynamics. The census already reports those as Dynamic, which is invisible to probes, so the conclusion stays correct.
  - Rapier collider-geometry quarantine needs non-finite collider geometry, which `clamp_shape_extent` prevents.
- **Bench forward cast passes `excluded_body = None`** (`app_step.rs:524`). A start inside the player capsule returns `toi ≈ 0`, which `.filter(|d| *d > 1e-3)` drops in favour of the centroid fallback, so there is no wrong value. It is a bench-only path owned by `/audit-runtime`, so not filed.
- **"static colliders: NONE ⇒ every ground probe will miss" ignores Keyframed colliders.** Architecture is always registered as a Static trimesh (`cell_loader/spawn.rs:85-87, 376-379`). Keyframed proxies are Clutter/Actor only (`:88-91`), so the statement holds in practice.
- **`census_tally` buckets `SoftFrame` as orphan** (`sync.rs:612-616`). The engine builds no soft bodies, so this is a dead path.
- **Counter or query cost**: none per frame (see Verified: Cost). The `ContactForceEvent` per-step channel volume was already verified bounded by D2, so not re-litigated.
- **Not re-filed**:
  - D2-01: the quarantined body is unqueryable in a quiet scene.
  - D2-03: the footstep probe self-hits; `surface_material_under` has no exclusion parameter.
  - D2-08: the step-cost guard scans only `step`.
  - D1-02 / D3-05: `user_data as u8` truncation.
  - #5535: lock-order.
- **Open issues searched** (`/tmp/audit/issues.json`) for query, census, phys.stats, kinematic and counter: no match.
- **Closed #3975**: its fix (doc wording) is in place, but rapier 0.36 inverted the premise, so it is filed as NEW D6-01 rather than as a regression.
