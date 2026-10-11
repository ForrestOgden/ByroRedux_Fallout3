# TD9-2026-10-11-01: `byroredux-core` with `parallel-scheduler` OFF is compiled by no CI lane

URL: https://github.com/matiaszanolli/ByroRedux/issues/5624
Labels: low, ecs, tech-debt, test-gap, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 9 (feature gate vs CI lane)
- **Location**: `crates/core/Cargo.toml` (`default = ["parallel-scheduler"]`), `crates/core/src/ecs/scheduler.rs:20,499-510` (the `#[cfg(not(feature = "parallel-scheduler"))]` serial dispatch arm); `.github/workflows/ci.yml` (no `--no-default-features` on core)
- **Status**: NEW
- **Age**: feature introduced with M27; lane list last re-audited 10-08
- **Effort**: trivial
- **Description**: Every workspace consumer takes `byroredux-core` with default features (`Cargo.toml:47` plain path dep; all crate manifests `workspace = true`), so feature unification always turns `parallel-scheduler` (rayon) on. The serial phase-1 fallback in `Scheduler::run` and the `rayon`-free build are therefore compiled and tested by nothing in `ci.yml`, `real-data-gates.yml`, `rt-correctness.yml` or `playable-smoke.yml`. `docs/engine/ecs.md:419` and `m27-phase1-survey.md:238` describe the flag as a retained "runtime-disable knob for diagnostics", i.e. a supported state. This is the exact class #3894/#4387/#4390 closed for `recon` / `debug-server` / `dhat-heap`, which each got a one-line lane.
- **Evidence**: `grep -rn 'parallel-scheduler' .github scripts` -> only a comment (ci.yml:214). `cargo check -p byroredux-core --no-default-features --lib --tests` (1.96.0) compiles clean today, so there is no live rot — the finding is the unguarded state, not breakage.
- **Impact**: A change that makes the scheduler's parallel arm and serial arm diverge (new field used only in the rayon arm, an `import` only valid with rayon) lands green and is found by whoever next builds without the feature.
- **Related**: #3894, #4387, #4390 (same class, closed); #5327 (open, warnings on the sibling `--no-default-features` byroredux lane).
- **Suggested Fix**: add to the `cargo-test` job beside ci.yml:176-180: `cargo check -p byroredux-core --no-default-features --lib --tests` (~5 s; verified).
