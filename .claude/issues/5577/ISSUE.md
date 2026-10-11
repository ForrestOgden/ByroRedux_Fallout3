# NIFAL-D7-2026-10-11-05: B-spline budget exhaustion is logged once per import and uncounted

Labels: low,nifal,animation,test-gap,bug

- **Severity**: LOW
- **Dimension**: Animation
- **Tier Violated**: harness-gap
- **Game Affected**: none in vanilla (worst measured file 200,151 of 1,000,000 units, 5× headroom)
- **Location**: `crates/nif/src/anim/bspline.rs:95-122` (`warned` flag), `crates/nif/src/anim/entry.rs:32-80`
- **Status**: NEW
- **Description**: One `log::warn!` per `BsplineSampling`, with no count of degraded channels. Which sequences degrade depends on `NiControllerManager.sequence_refs` order, then block order. No test pins "a hostile early sequence does not starve a later honest one".
- **Impact**: Diagnosability only.
- **Related**: #5486
- **Suggested Fix**: Count degraded channels and include the count in the single warn, or expose a `degraded()` accessor the importer logs once.

_Source: `docs/audits/AUDIT_NIFAL_2026-10-11.md` finding D7-05 (HEAD c6e48cbc6)._

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix
