# #5547: PHYS-D5-2026-10-11-02: Buoyancy is re-derived once per render frame and held constant across every substep and solver iteration, so thin floats never settle and the threshold scales with frame rate

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5547
- **Labels**: medium,physics,water,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

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

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
