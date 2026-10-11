# NIFAL-D7-2026-10-11-02: #5486's span clamp re-parameterises the B-spline: `u` is mapped over the clamped span, time-warping any interpolator wider than its sequence

Labels: medium,nifal,animation,bug,game:fo3,game:fnv

- **Severity**: MEDIUM (vanilla-inert; a regression on mod or trimmed-sequence content — may be downgraded to LOW)
- **Dimension**: Animation
- **Tier Violated**: no-fabrication (the sampled curve no longer equals the authored curve)
- **Game Affected**: FO3, FNV, and any mod KF (Oblivion and Skyrim vanilla have no NIF B-splines)
- **Location**: `crates/nif/src/anim/bspline.rs:95-125` (`reserve` returns the clamped `(start, stop)`); consumers `bspline.rs:316-337` (float), `:406-431` (transform), `crates/nif/src/anim/channel.rs:321-342` (point3 colour)
- **Status**: NEW — Regression introduced by 0c4d4af08 (the fix for #5486, closed); the allocation bound itself is fine.
- **Description**: A B-spline's parameter range `[0, n_cp-3]` spans the interpolator's own `[start_time, stop_time]`. Before 0c4d4af08, `u` was computed against the interpolator's own duration. Now `reserve` returns `start = max(interp_start, seq_start)` and `stop = min(interp_stop, seq_stop)`, and the samplers compute `u = ((t - start_time) / duration) * u_max` with the clamped values. Whenever the clamp trims, the entire curve is stretched over the trimmed window instead of sampling the visible sub-range. Interp [0,2] in a sequence [0,1] now plays the whole 2 s curve in 1 s. Disjoint spans (interp [0,10], seq [20,30]) give `start=20, stop=10, duration=0`: two keys at t=20 holding control point 0.
- **Evidence**: `channel.rs:324-342` (read by the orchestrator): `reserve(interp.start_time, interp.stop_time, 1)` returns the clamped window, then `duration = stop_time - start_time` and `u = ((t - start_time) / duration) * u_max`. The commit's four tests (`sampled_span_is_clamped_to_the_owning_sequence`, the ceiling, the budget decline, `import_kf_bounds_a_shared_huge_span_interpolator`) pin sample counts, never values. Vanilla scan: 0 of 157,863 FO3/FNV/DLC B-spline controlled blocks have a span differing from their sequence.
- **Impact**: KF/NIF content whose interpolator span is wider than its sequence animates at the wrong speed or segment. Previously correct, silently regressed.
- **Related**: #5486, `/audit-nif` Dim 6 (allocation side), D7-03.
- **Suggested Fix**: Keep the original `(interp_start, interp_stop)` as the `u` domain and restrict only which `t` values are emitted. Add a value-level test: the sampled key at the window start equals `deboor_cubic` at the matching `u`, not at `u=0`.

_Source: `docs/audits/AUDIT_NIFAL_2026-10-11.md` finding D7-02 (HEAD c6e48cbc6)._

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test pins this specific fix
