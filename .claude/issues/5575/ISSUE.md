# NIFAL-D7-2026-10-11-03: `seq_span` finiteness guard does not do what its comment says

Labels: low,nifal,animation,bug

- **Severity**: LOW
- **Dimension**: Animation
- **Tier Violated**: no-fabrication
- **Game Affected**: all with B-spline content; only reachable on non-vanilla input
- **Location**: `crates/nif/src/anim/sequence.rs:50-57`; defaults at `crates/nif/src/blocks/controller/sequence.rs:395-405`
- **Status**: NEW
- **Description**: The comment says the FLT_MAX/FLT_MIN neutral defaults "parse to non-finite differences", but the guard tests `start.is_finite() && stop.is_finite()` per value, and both defaults are finite. A sequence parsed below 10.1.0.106 therefore sets `seq_span` to (MAX, MIN), giving `duration=0` and two B-spline keys at t≈3.4e38. An authored `stop <= start` sequence likewise trims every interpolator to zero length.
- **Impact**: Wrong-keyed (not crashing) channels on malformed content; the comment misleads.
- **Related**: D7-02
- **Suggested Fix**: Gate `seq_span` on `(stop - start).is_finite() && stop > start`.

_Source: `docs/audits/AUDIT_NIFAL_2026-10-11.md` finding D7-03 (HEAD c6e48cbc6)._

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test pins this specific fix
