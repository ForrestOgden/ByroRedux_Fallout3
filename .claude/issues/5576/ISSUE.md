# NIFAL-D7-2026-10-11-04: Sampled B-spline colour keys have no `is_key_value_sane` gate

Labels: low,nifal,animation,bug

- **Severity**: LOW
- **Dimension**: Animation
- **Tier Violated**: no-leak
- **Game Affected**: FO3 / FNV with `NiBSplineCompPoint3Interpolator` colour controllers
- **Location**: `crates/nif/src/anim/channel.rs:344-349`
- **Status**: NEW (a pre-existing asymmetry with the float and transform samplers, #3765; not introduced by 0c4d4af08)
- **Description**: `sample_color_keys_bspline_point3` pushes `[p[0], p[1], p[2]]` straight from `deboor_cubic`. The static fallback is gated, but a finite-yet-huge `half_range` can overflow to inf through the blending and reach an `AnimColorKey`.
- **Impact**: Reachable only via malformed quantisation parameters.
- **Related**: #3765, #4397
- **Suggested Fix**: Skip a sample whose three components fail `is_key_value_sane`, as the float sampler does.

_Source: `docs/audits/AUDIT_NIFAL_2026-10-11.md` finding D7-04 (HEAD c6e48cbc6)._

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test pins this specific fix
