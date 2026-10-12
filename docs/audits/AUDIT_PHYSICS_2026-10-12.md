**HEAD**: `5d31fd637` · **Baseline**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`) · **Audited**: all six dimensions (Dim 1 Shape Translation, Dim 2 Step & Sync, Dim 3 Ragdoll & Constraint Seam, Dim 4 Character & NPC Controller, Dim 5 Water / Buoyancy, Dim 6 Queries & Diagnostics) · **Unchanged since baseline (skimmed)**: Dim 6 logic (only `phys.stats` labels and the `surface_material_under` signature changed; both checked)

# PHYSAL / Physics Audit — 2026-10-12

**Run**: `/audit-physics` inside the `comprehensive` suite. Solo: no sub-agents, no engine launch, read-only except this report and `/tmp/audit/physics/`.

**Delta audited**: `65cadde2f..5d31fd637` (176 commits repo-wide; about 45 touch physics scope). Essentially the whole 2026-10-11 audit's fix wave landed in this window, so this pass verifies those fixes and hunts the new code for regressions:
- Ragdoll: #5536 (Havok-convention joint limits), #5538 (30 Hz soft joint limits), #5537 (derived Oblivion `Perp B1`), #5546 (solver budget back to the world default), #5540 (link damping as force/torque).
- Step / wake: #5543 (substep-armed wake survives the post-loop clear), #5544 (spawn re-sleep), #5554 (rolled-back substep drops its impact events), #5555.
- Surface materials: #5542 (u32 end to end), #5541 / #5550 (one walk produces shape and per-leaf materials), #5545.
- Water: #5547 (lift re-derived per substep), #5561, #5562, #5354, #5582, #5548.
- NPC controller: #5539 (NPC step owns its vertical), #5559, #5560.
- Test hygiene: #5551, #5552, #5558, #5563, #5564.

**Lane counts at HEAD** (toolchain 1.96.0):
- `cargo test -p byroredux-physics`: **244 passed**, 0 failed (baseline 220).
- `cargo test -p byroredux --bin byroredux`, by filter:
  - `ragdoll` 30 passed, 7 ignored
  - `character` 61
  - `locomotion` 8
  - `water` 164 passed, 4 ignored
  - `scheduler_access` + `workspace_hygiene` + `rapier_release` + `physics` + `footstep` + `submersion`: 98 passed, 1 ignored
- `cargo test -p byroredux-nif --lib -- collision bhk`: 182 passed.
- **FO3 real-data ragdoll gate is green again**: `-- --ignored ragdoll::installed_tests` gives **6 passed, 0 failed** (17 s). The 2026-10-11 report had it at 2 passed / 4 failed. The "re-measure before re-opening" instruction in the known-open register is satisfied.
- Not run: the `#[ignore]`d skeever probe (`BYRO_SKEEVER_NIF`) and every plugin `--ignored` test.

**Probe** (a throwaway crate at `/tmp/audit/physics/probe`, linked to the repo's `byroredux-physics` rlib, built in the repo `target/`): measured PHYS-D3-2026-10-12-01 and PHYS-D2-2026-10-12-02 / -03 below. No repo file was edited.

---

## Executive Summary

| Severity | NEW | Regression | Existing (matched, cited) |
|---|---:|---:|---:|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | **1** | 0 | 0 |
| LOW | **4** | 0 | 3 (#5621, CONC-D5-2026-10-12-01, PERF-D1-2026-10-12-02/03) |

The 2026-10-11 wave (5 HIGH, 8 MEDIUM) is fixed and verified in code. No fix regressed. Each one has a pin, and the pins that matter ran green. The new code has one fidelity gap with a false premise (angular link damping) and three small second-order issues around the same new `apply_multibody_link_damping` pass and the spawn re-sleep.

**PHYSAL doctrine verdict: HOLDS.** `grep -E "GameKind|bsver|NifVersion|is_skyrim|havok_scale|GameVariant"` over `crates/physics/src` and `byroredux/src/ragdoll.rs` finds nothing, apart from one test fixture's plugin file name. The per-game seams remain parse-side. The Wave-4 material path (u32, raw per-game id, no solver-side branch) stays documented as deliberately untranslated (`nifal.md` "Havok surface material", #5574).

**Collision data traced**: FO3 male skeleton through `build_ragdoll` + `PhysicsWorld::step` (the installed gate, six environments x two yaws). NIF collision resolve and material walk through the synthetic dispatch fixtures (182 tests). No new vanilla-archive census this pass; the 2026-10-11 census stands.

---

## Solver Invariant Matrix

| Invariant | State | Evidence |
|---|---|---|
| Fixed step + accumulator clamp + NaN/negative dt | **Verified** | `world/mod.rs:1105-1108` (`f32::max`, clamp to `MAX_SUBSTEPS*PHYSICS_DT`). The budget bail-out is still the only wall-clock read. |
| Determinism (no hash order in the solver path) | **Verified** | The new `water_force_targets` `HashSet` and `spawn_sleep_pending` `Vec` feed only membership tests and an in-order walk. `rest_depth` is a point-lookup latch. |
| Static-scene fast path | **Verified** | Gate unchanged at `world/mod.rs:1173`. `apply_multibody_link_damping` runs after it, so a settled cell pays nothing. |
| Wake discipline, spawn | **Verified** | Spawn still arms nothing. `step` re-sleeps each `spawn_sleep_pending` newcomer after the first stepping frame (#5544); the new test `spawned_asleep_clutter_resleeps_after_its_first_stepping_frame` is green. The water exemption (`water_force_targets`) is respected. |
| Wake armed inside a substep (#5543) | **Verified** | `substep_wake_requested` is set at both arming sites in `run_substep` and re-armed after the post-loop clear (`mod.rs:1219-1226`). One-shot; `a_quarantined_body_is_re_enabled_and_parked` pins it end to end (ray finds the body again, then the fast path returns). |
| Query BVH freshness / no per-frame `update_query_pipeline` | **Verified** | The #5558 guard now scans `step`, `run_substep` and `refresh_query_geometry_after_restore`, each window checked to be the function it names. |
| Lock order / phase order | **Verified** | Collect, register, push kinematic, buoyancy, step, pull. `refresh_water_lift` runs inside `run_substep` under the single `PhysicsWorld` write guard. No new cross-resource nesting in the physics crate. |
| Explosion recovery + #5161/#5246 ladder | **Verified** | The snapshot precedes every substep. #5554 truncates the event buffer back to its pre-substep length on rollback. #5555 adds the Dynamic re-check in the clamp. Recovery counters unchanged. |
| Rapier quarantine round-trip | **Verified** | Cooldown re-enable one step later, wake survives the clear (#5543). |
| Ragdoll solver budget locality (#5546) | **Verified** | `RAGDOLL_EXTRA_SOLVER_ITERATIONS = 0`; `a_ragdoll_does_not_change_an_unrelated_bodys_solve` compares bit-exact trajectories. |
| Ragdoll joint-limit convention (#5536) | **Verified by derivation** | Non-flip (A parent): rapier theta = -theta_Havok, so the authored range negates and swaps. Flip (A child): theta = +theta_Havok, pass-through. Same for twist and prismatic. Pins: `authored_limits_apply_to_the_havok_angle_on_both_flip_orientations`, `hinge_flexion_seed_holds_and_hyperextension_is_corrected`. |
| Damping on articulations (#5540) | **Partially drifted** | Linear half works (`dry_multibody_link_damping_decays_drift_velocity`). The angular half is a mass-scaled stand-in that is about 60x too weak, on a false premise (PHYS-D3-2026-10-12-01). |
| Buoyancy lift persistence (#5547) | **Verified** | `WaterLift.applied` is recorded before link drag and flow are added, so `refresh_water_lift` re-derives only the lift and heave term and leaves the other contributors alone. `water_lift` is cleared before every early return in the scan. |
| Teardown | **Verified** | `remove_body` unchanged; the new `spawn_sleep_pending` / `water_lift` / `water_force_targets` entries tolerate dead handles (`get`/`get_mut` returns `None`, skipped). |
| Contact events | **Bounded; one cost note** | Cleared once at `step` entry, appended per substep, rolled back with the substep. Cost of the new post-step filter: PHYS-D2-2026-10-12-03. |

---

## Findings — MEDIUM

### PHYS-D3-2026-10-12-01: Angular damping on multibody links is a mass-scaled stand-in about 60x weaker than authored, on the false premise that rapier exposes no inertia
- **Severity**: MEDIUM
- **Dimension**: Ragdoll & Constraint Seam (also touches Water / Buoyancy)
- **Location**: `crates/physics/src/world/mod.rs:926-938` (`apply_multibody_link_damping`, dry links); `crates/physics/src/water.rs:1160-1175` (wet links, `pending_torque`)
- **Status**: NEW (the linear half of #5540 is correct and pinned; this is its angular half)
- **Trigger Conditions**: any ragdoll (every dead NPC and creature on every game with a decoded constraint graph) with a nonzero authored or submerged angular damping.
- **Description**: #5540 found that rapier skips `RigidBody` damping for multibody links and moved both damping terms onto the user force/torque channel. The torque is computed as `-c * m * w`. Its comment justifies this with "Rapier 0.36 exposes no world angular inertia on a RigidBody (mprops is crate-private)". That is false: `RigidBody::mass_properties()` is `pub` (`rapier3d-0.36.1/src/dynamics/rigid_body.rs:360`), and `RigidBodyMassProps::effective_angular_inertia()` is `pub` (`rigid_body_components.rs:395`). The correct torque for exponential decay at rate `c` is `-c * I_world * w`. The code's `m * w` therefore damps at `c * m / I` instead of `c`. A bone-sized capsule has `I / m` of about 66 (see the measurement).
- **Evidence**: Probe: two-link ragdoll, one capsule (half-height 10, radius 5, mass 10) per bone, gravity 0, spin imparted by torque with damping set to zero, then authored `angular_damping = c` restored and 4 s simulated through the production `PhysicsWorld::step`.

  | authored `c` | root `|w|` after 4 s (measured) | authored exponential expectation |
  |---|---:|---:|
  | 0.05 | 12.18 (from 13.5) | 11.07 |
  | 0.5 | 12.68 (from 13.5) | 1.83 |
  | 0.9 | 10.47 (from 13.5) | 0.37 |

  `body.mass_properties().effective_angular_inertia()` read from outside the crate returns diagonal entries of 665.6 / 118.8 / 665.6 for that mass-10 bone, so `I/m` is about 66 about the perpendicular axes. At `c = 0.9` the spin loses 22 % in 4 s where the authored value would remove 97 %. The same probe confirms the linear half by construction (the in-repo test passes).
- **Impact**: Ragdoll limb spin is nearly undamped, dry and wet. Authored bone angular damping and the `ragdoll_extra_angular_damping` lever are effectively inert, which is the state #5540 was filed to remove. Visible as corpses that keep rotating, and floating corpses that roll longer. Settling relies on contact friction and the sleep thresholds instead. Not a crash, and bounded by the DOF clamp.
- **Related**: #5540 (closed), `ragdoll_extra_angular_damping` (`physal.md` §4).
- **Suggested Fix**: Compute `torque = -c * (I_world * w)` from `body.mass_properties().effective_angular_inertia()` (check rapier's "square root" wording in that accessor's doc against the values above, which look like the plain inertia). Apply it in both the dry pass and the wet scan. Add an angular decay pin beside `dry_multibody_link_damping_decays_drift_velocity`, whose assertion only measures `linvel.y`.

---

## Findings — LOW

### PHYS-D2-2026-10-12-01: The dry-link damping pass resets every awake link's user force, wiping placed-current forces; its skip set is stale on every early-return path of the water scan
- **Severity**: LOW
- **Dimension**: Step & Sync / Water
- **Location**: `crates/physics/src/world/mod.rs:902-940` (`apply_multibody_link_damping`: `reset_forces` at 922), `:1183` (call in `step`); `crates/physics/src/water.rs:896` (the only `water_force_targets.clear()`), `:1074` (the only per-body `note_water_force_target`, inside the surface arm), `:1286` (current arm flags a body only when it wakes it)
- **Status**: NEW
- **Trigger Conditions**: a ragdoll that is awake and inside a placed `WaterCurrentVolume` that overlaps no `WaterPlane` in XZ (the marker-only case #3114 and #5129 explicitly support for dynamic bodies), or that has just left water whose scan then early-returned.
- **Description**: Each stepping frame, `apply_multibody_link_damping` does `reset_forces(handle)` on every awake dynamic multibody link not in `water_force_targets`, then adds the damping force. Any other user force on such a link is discarded before the substeps run. The water scan protects its own forces by adding the body to `water_force_targets`, but only in the surface arm, or in the current arm when the current wakes a sleeping body. A link inside a marker-only current: the scan resets, adds the marker's current force, never flags the body; the pass then resets it away. The placed current does nothing to an awake ragdoll link. The set is also cleared only by a full scan: `surfaces.is_empty()`, the quiesced fast path and `targets.is_empty()` all return first, so after a plane streams out, ex-wet links stay "owned by the water pass" and lose dry damping until another full scan runs. `clear_stale_water_contacts` restores damping and force but not this set.
- **Evidence**: Probe: a ragdoll link with `angular_damping > 0`, `add_force(h, (0, 5000, 0))` then `step`: no motion at all (y stays at 1000). With the damping set to 0 the same force moves it (pass skipped by `lin <= 0 && ang <= 0`).
- **Impact**: Small. Placed currents are an FO3/FNV hazard detail, ragdolls in them are rarer still, and the effect is "no push", not instability. It also quietly breaks `PhysicsWorld::add_force`'s documented "accumulates until reset_forces" contract for links.
- **Related**: #3114, #3268, #5129, PHYS-D3-2026-10-12-01.
- **Suggested Fix**: Flag the body in the current arm too (any frame it receives a current force). Better, have the dry pass add its damping into a force it computes itself rather than resetting. Clear `water_force_targets` at the top of the scan, before the early returns, like `water_lift`.

### PHYS-D2-2026-10-12-02: `apply_multibody_link_damping` walks the whole body arena (and SipHashes each awake dynamic) on every stepping frame
- **Severity**: LOW
- **Dimension**: Step & Sync (cost)
- **Location**: `crates/physics/src/world/mod.rs:902-914`
- **Status**: NEW (same hashing class as PERF-D1-2026-10-12-02 — Existing, cited, not re-reported)
- **Description**: The pass iterates `self.bodies.iter()`, which visits fixed bodies too, testing `is_dynamic()`, then `is_sleeping()`, then `water_force_targets.contains` (std SipHash `HashSet`), and only then `rigid_body_link`. #4682 moved the recovery snapshot off exactly this kind of arena walk, measured at 1.77 ms per substep on a 95 k-body world. Stepping frames are the normal state in an occupied cell (any animated NPC bone pushes a kinematic target and arms `pending_wake`).
- **Evidence**: Probe, same loop shape, opt-level 3: 0.039 ms/call at 20 000 fixed bodies, 0.21 ms/call at 95 000 (1.7 ms measured for the old snapshot walk includes its pose copies). The engine's dev profile builds `byroredux-physics` itself at opt-level 0, so the in-engine number is higher than the probe's.
- **Impact**: About 0.2 ms or more per stepping frame on a Commonwealth-class exterior, for a pass that has no work unless a ragdoll is awake. Not a hitch; a steady tax on the CPU-bound path.
- **Related**: #4682, PERF-D1-2026-10-12-02.
- **Suggested Fix**: Walk `self.islands.active_bodies()` (awake only) or the ragdoll body list, and test `rigid_body_link` before the hash lookup.

### PHYS-D2-2026-10-12-03: The spawn re-sleep filters the impact buffer with `Vec::contains` (quadratic) and also drops genuine impacts of newcomers that moved
- **Severity**: LOW
- **Dimension**: Step & Sync
- **Location**: `crates/physics/src/world/mod.rs:1269-1297` (`settled_colliders.extend` at 1282, drift test at 1283, `retain` at 1290)
- **Status**: NEW
- **Trigger Conditions**: the first stepping frame after a large newcomer batch (grid load, teleport, save load) with thousands of dynamic clutter bodies.
- **Description**: (a) `settled_colliders` is a `Vec` and each buffered event does two `contains` scans, so the filter is O(events x colliders). The collider force-event threshold is rapier's default 0.0, so every resting item's first contact emits a `started` event, which makes events about equal to newcomers on that frame. (b) `settled_colliders.extend(...)` runs before the drift test, so the filter also removes the events of newcomers that did drift more than 4 BU (a real push, or an unsupported item falling), while the doc says those "keep rapier's verdict". Only their first stepped frame is affected.
- **Evidence**: Probe, `Vec<ColliderHandle>` `retain` with two `contains`: 1 000 -> 0.20 ms, 5 000 -> 4.7 ms, 15 000 -> 42.8 ms. Reading the code for (b): `extend` precedes `let drifted = ...`.
- **Impact**: A one-off 5-40 ms spike on the first stepping frame of a very large load (per-cell batches of a few hundred are well under 1 ms), inside the streaming-hitch-sensitive window. For (b): a lost first-impact sound on a newcomer that falls or is hit on its first frame. Neither is a correctness failure.
- **Related**: #5544, PERF-D1-2026-10-12-03 (all dynamic colliders armed for force events).
- **Suggested Fix**: Build a hash set (or sort + binary search) of the settled colliders, and extend it only for bodies that were actually re-slept.

### PHYS-D3-2026-10-12-02: `physal.md` §4 still describes soft limits and a raised ragdoll solver budget as future work; `physics.md` omits the dry-link damping pass
- **Severity**: LOW (doc-rot)
- **Dimension**: Ragdoll & Constraint Seam
- **Location**: `docs/engine/physal.md:250-253` (solver iteration count and joint-limit softness "staged as the limits get refined") and `:236-237` ("limit-fidelity refinement ... are follow-ups"); `docs/engine/physics.md:113` (damping described as set on the body at registration)
- **Status**: NEW
- **Description**: #5538 shipped soft limits (`RAGDOLL_JOINT_SOFTNESS`, 30 Hz, zeta 1) and #5536 fixed limit fidelity. #5546 deliberately set the ragdoll solver budget to the world default (`RAGDOLL_EXTRA_SOLVER_ITERATIONS = 0`), so "ragdolls want more than clutter" is now the opposite of the code. `physics.md` Phase 1 says `linear_damping` / `angular_damping` are applied from `RigidBodyData`, with no mention that rapier ignores them on links and `step` re-applies them every stepping frame through `apply_multibody_link_damping` (which also wipes other user forces, see PHYS-D2-2026-10-12-01). The doc-side rot is not captured by NIFAL-D6-2026-10-12-01, which covers naming only.
- **Impact**: A contributor reading §4 would re-raise the iteration count (a measured 2x whole-world cost) or re-implement soft limits.
- **Related**: #5538, #5546, #5540.
- **Suggested Fix**: Rewrite the two bullets as "done" with the #5538 / #5546 rationale, and add a sentence on the dry-link pass to `physics.md`.

---

## Checked, no finding

- **Lead: plain Oblivion `bhkHinge` (`parse_hinge_oblivion`, `constraints.rs:349`).** Confirmed it zeroes both `perp_axis_in_b1` and `perp_axis_in_b2`. Neither B perpendicular is serialized in that layout (only `Pivot A`, `Perp A1`, `Perp A2`, `Pivot B`, `Axis B`), so unlike the limited hinge's B1 (#5537) it cannot be derived. The solver's `frame_rot` then synthesizes a B-side zero-reference. The joint is limited to +-pi about that arbitrary reference (`constraints.rs:371-372`). The seed (`rotation_scaled_axis.x`, in (-pi, pi]) always lies inside the limits, so the total window is still a full turn; only its centring on the bind pose is arbitrary. For a ragdoll (dead-body) swing that is inert in practice, so not filed. The FO3+ hinge has the same +-pi encoding with an authored zero reference. The stale comment at `import/collision/ragdoll.rs:387-394` is NIF-D5-2026-10-12-01 (Existing).
- **NPC vertical probe (#5539).** `cast_ray_down` starts at the capsule centre (`lift = 52` BU) with `max = 88`; the request is `clamp(-(feet - support - offset), -32, +4)`; the origin sits inside the NPC's own fallback capsule, which carries `ACTOR_BONE_GROUP` and is masked. A hit inside another solid at the origin could lift an NPC 4 BU per frame, but the KCC keeps capsules from overlapping other solids and the player capsule (unmasked, so solid to NPCs), so only teleport-onto-NPC overlaps could trigger it. No evidence it occurs.
- **Player drowning / hazard damage (#5548, #5434).** One owner (`water_damage_system`) plus breath-driven drowning in the controller. The controller returns early when `Dead` (`character.rs:207`), so KILL is raised once. `water_damage_system`'s advisory `Access` row also omits the `StoryEvent` write; same class as #5641 and noted in the concurrency report (not re-reported).
- **Surface-material path (#5542, #5541, #5550).** One material per leaf from the same walk (`debug_assert_eq` on `collision_shape_leaf_count`), failed children truncate their entries, `0` is authored STONE only when `bsver < SKYRIM_LE`, `collider_surface_material` reads `user_data as u32`. The footstep caller passes the player's own body (`audio.rs:278`).
- **Water stack.** `WaterLift.applied` ordering, `rest_depth` sweep, `waterline` at the AABB centre, `weather_scroll_for` at every CPU sampler, `wave_scrolls` as the one scroll prep.
- **Save.** `WaterContact.waterline` is not saved (`registry_completeness_tests.rs:580`); `WaterContactScratch` carries the new `rest_depth` latch and is excluded with a stated re-derivation.

---

## Existing findings referenced (not re-reported)

- **CONC-D5-2026-10-12-01** (LOW): `world/mod.rs:291` comment says the force receiver is called from rayon workers; rapier 0.36 is built without `parallel`. The `Mutex` and `Sync` requirement itself stays correct.
- **PERF-D1-2026-10-12-02 / -03** (LOW): `physics/src/water.rs:153` std `HashMap`/`HashSet` latches per wet body; `CONTACT_FORCE_EVENTS` armed on all dynamic colliders.
- **#5621** (open, LOW): workspace clippy red. The physics site is the `option_map_unit_fn` on `.map(|b| b.add_torque(torque, false))`, now at `world/mod.rs:935-937`.
- **NIF-D5-2026-10-12-01** (LOW): stale "Oblivion LimitedHinge zeroes Perp Axis In B1" comments.
- **NIFAL-D6-2026-10-12-01** (LOW): stale `shape_leaf_materials` / `first_shape_material` naming (`physics.md:436`, `convert.rs:1480`, `ragdoll.rs:423`).
- **#5530** (open, LOW): `accept_keyframe_target` gates only `push_kinematic`. Still true.
- **#5591** (open, LOW): the player-teleport sequence is written four times.
- **#4772** (open): FO3 copied-save first-solve jump. The FO3 real-data gate that measured it is green (6/6) after #5536/#5538; re-measure the issue's own repro before closing.
- **#5015** (open): XRGD corpse poses never decoded. Unchanged; outside this pass.

---

## Known-Open Register — what this pass changed

- **Closed by verification**: the entire 2026-10-11 finding set (PHYS-D3-01/-02/-03/-04, D4-01/-02/-03, D5-01..-05, D1-01..-07, D2-01..-08, D6-01/-02). Each has a landed commit, a pin, and the relevant pins ran green. The FO3 real-data ragdoll gate is green (6/6).
- **Register entries to refresh in the audit skill**: the "FO3 real-data articulation gate is green again" note can drop its "re-measure" caveat; add that the link-damping pass (`apply_multibody_link_damping`) is part of `step`, and that its angular term is the open PHYS-D3-2026-10-12-01.
- **New known-open**: PHYS-D3-2026-10-12-01 (MEDIUM, angular link damping), the four LOWs above.
- **Unchanged**: Water-walking / freezing unbuilt; ball-and-socket / stiff-spring / chain constraints declined by design; the upstream mass-inverted rig root cause (no issue); skyrim mass=0 reclassification.

Suggested next step: `/audit-publish docs/audits/AUDIT_PHYSICS_2026-10-12.md` (labels `physics`, plus `water` for PHYS-D2-2026-10-12-01 and `doc-rot` for PHYS-D3-2026-10-12-02).
