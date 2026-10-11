# #5561: PHYS-D5-2026-10-11-04: A sleeping float re-wakes for waves only when the per-frame depth change exceeds 0.1 BU, so wave-following depends on frame rate and stops entirely at high refresh rates

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5561
- **Labels**: low,physics,water,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

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

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
