# TD9-2026-10-11-02: Vulkan-gated `golden_frames` + `upscaler_quality` have no automated lane (the `rt-correctness` dispatch has no gate for them)

URL: https://github.com/matiaszanolli/ByroRedux/issues/5625
Labels: low, renderer, tech-debt, test-gap, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 9 (test-gap)
- **Location**: `byroredux/tests/golden_frames.rs:107-117` (2 tests), `byroredux/tests/upscaler_quality.rs:115,189` (2 tests); `.github/workflows/rt-correctness.yml:5-14` (gate choice: `cornell | material-provider-matrix | all`)
- **Status**: NEW
- **Age**: `golden_frames` baselines last regenerated 2026-09-21 (`7cdc0c7ff`); 223 commits touching `crates/renderer/{src,shaders}` since
- **Effort**: small
- **Description**: The only pixel-level render regression guards are `#[ignore]` behind a Vulkan device and referenced by no workflow (`grep -rn 'golden' .github` -> none; only `scripts/renderer-eval.sh:106` mentions the contract and `audit-runtime` SKILL line 19 lists them as a manual gate). The default-lane guard `committed_baseline_matches_the_current_invocation` only proves the *manifest* matches the invocation (#3849/#4227), not that the PNG still matches the renderer. The self-hosted `byroredux-rt` runner and the `rt-correctness` workflow already exist and already run `cornell_rt_oracle`, so the lane is a one-option addition. The `golden_frames` module doc itself says the first version rotted for three months unnoticed behind this same blind spot.
- **Evidence**: `git log -1 --format=%ad -- byroredux/tests/golden/*.png` -> 2026-09-21; `git log --since=2026-09-21 --oneline -- crates/renderer/src crates/renderer/shaders | wc -l` -> 223. Pixel-currency could not be checked (no Vulkan device use permitted in this audit).
- **Impact**: The "Phase-2c-style shader regression" the file was written to catch is only caught when someone remembers to run `audit-runtime`.
- **Related**: #3849, #4227 (closed), #4492 (closed — same "harness in no automated lane" class for the exterior harnesses).
- **Suggested Fix**: add a `golden` option to `rt-correctness.yml` running `xvfb-run -a cargo test --release -p byroredux --test golden_frames -- --ignored --test-threads=1` (+ `upscaler_quality` without the game-data case), mirroring the `cornell` job (rt-correctness.yml:21-60).
