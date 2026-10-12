# REG-2026-10-12-03: the forced-low-BLAS-budget hardware gate is recorded red, and the doc says it passes

- **GitHub**: #5761 (https://github.com/matiaszanolli/ByroRedux/issues/5761)
- **Labels**: low,renderer,test-gap,bug
- **Source**: docs/audits/AUDIT_REGRESSION_2026-10-12.md (`REG-2026-10-12-03`)

**Audit finding** `REG-2026-10-12-03` — severity LOW — from `docs/audits/AUDIT_REGRESSION_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `byroredux/tests/cornell_rt_oracle.rs` (`cornell_forced_low_blas_budget_preserves_rt_shadows`); claim at `docs/engine/rt-lighting-material-recovery.md` (R-section: "It passes on the RTX 4070 Ti")

## Description
The recovery doc says the ignored gate "passes on the RTX 4070 Ti". Commits `48bed0504` and `4d991bad7` (both 2026-10-11, `Refs #5369`) each state that it "fails identically before and after (pre-existing, `missing_rigid=1` under the BLAS budget override)". The test asserts a complete two-instance TLAS under a one-byte static-BLAS budget; `missing_rigid_blas` is the counter `acceleration/tlas.rs` increments. Nothing in `acceleration/` changed since 2026-10-07 (`72028d4fd` #5250), so the red state either predates the 10-11 commits or is device-specific (the same author's earlier diagnosis that day ran on lavapipe). Validated at publish time: the doc sentence and the commit-message claim are both still as quoted.

## Evidence
The two commit bodies (`git log -1 --format=%B 48bed0504`, `4d991bad7`); the doc sentence; `acceleration/tlas.rs` `missing_rigid_blas`. The gate needs an RT device plus a display and could not be re-run in the audit.

## Impact
Unknown. If it is a real regression of the #4883/#4884/#5195/#5201 BLAS-budget work, a budget-pressured scene (Cydonia, FO4 Institute) can publish a TLAS missing a rigid instance. If it is device-specific, the doc claim is merely unqualified.

## Suggested Fix
Run the gate on the 4070 Ti once; if red, bisect against `72028d4fd`..`5d31fd637` and fix; if green, qualify the commit-message claim in the doc. Either way note the lavapipe/hardware split in the test's doc comment.

## Related
#4883, #4884, #5195, #5201, #5250, #5369.

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
