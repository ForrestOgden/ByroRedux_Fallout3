# TD8-2026-10-12-02: A deny-level clippy error in the HKX census breaks `cargo clippy --all-targets` for `byroredux-hkx`

- **GitHub**: #5744 (https://github.com/matiaszanolli/ByroRedux/issues/5744)
- **Labels**: low,import-pipeline,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD8-2026-10-12-02`)

**Audit finding** `TD8-2026-10-12-02` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/hkx/src/animation.rs` (`assert!(rejects.len() <= KNOWN_VANILLA_REJECTS, ...)` with `const KNOWN_VANILLA_REJECTS: usize = 0`)

## Description
`usize <= 0` trips `clippy::absurd_extreme_comparisons`, which is `deny` by default, so `error: could not compile byroredux-hkx (lib test)` appears under `cargo clippy --workspace --all-targets`. The test is `#[ignore]`d (real Skyrim SE archive), so `cargo test` does not notice, and CI only runs the non-`--all-targets` form. Introduced by `9707cb367` (#5662, 2026-10-12), which lowered the baseline from 288 to 0 under the existing `<=` form from `d4352d477`.

## Evidence
`/tmp/audit/tech-debt/clippy_all.txt` lines 159-169; validated at publish time: `KNOWN_VANILLA_REJECTS: usize = 0` and `rejects.len() <= KNOWN_VANILLA_REJECTS` are still present.

## Impact
The `--all-targets` developer command stops at this crate's test unit; the dependents of that unit are not linted. ROADMAP's "all-targets is clean" claim is false (TD3-2026-10-12-01).

## Suggested Fix
`assert!(rejects.is_empty(), ...)` with the message that now names the clips; the same edit delivers #5671's "make it exact".

## Related
#5671, #5662, #5326, #5621.

## Completeness Checks
- [ ] **GATE**: The relevant gate (`cargo clippy`, hygiene test, `_audit-validate.sh`) is green after the change
- [ ] **TESTS**: A regression test pins this specific fix
