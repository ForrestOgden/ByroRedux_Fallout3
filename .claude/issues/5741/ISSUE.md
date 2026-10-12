# TD3-2026-10-12-01: ROADMAP.md Gates paragraph, ABBA Known Issue and one path are stale against HEAD

- **GitHub**: #5741 (https://github.com/matiaszanolli/ByroRedux/issues/5741)
- **Labels**: low,documentation,doc-rot,tech-debt
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD3-2026-10-12-01`)

**Audit finding** `TD3-2026-10-12-01` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `ROADMAP.md:27-35` (Gates), `ROADMAP.md:694-701` (Known Issues bullet), `ROADMAP.md:136` (`byroredux/src/boot.rs`); `docs/engine/archives.md:372` (`asset_provider.rs`)

## Description
Four claims no longer hold. (1) "Hosted CI is **red at HEAD on the ABBA lock-order lane alone**": #5535 (`1cb12fb3d`) reordered three production inversions and fixed two test bodies, so the lane is green, and the CI clippy gate is red (#5621), so "alone" is wrong both ways. (2) "`cargo clippy --workspace --all-targets` is clean (re-run at this close)": at HEAD it has 11 test-target warnings and one deny-level error (see the HKX clippy finding TD8-2026-10-12-02). (3) The Known Issues bullet "ABBA lock-order lane red since the `7ef007898` push ... Not yet triaged" describes a fixed, triaged problem. (4) `ROADMAP.md:136` points `expand_game_profile_args` at `byroredux/src/boot.rs`, which is `boot/cli.rs` (a directory since #3170); `docs/engine/archives.md:372` names `asset_provider.rs`, now `asset_provider/archive.rs`. (`ROADMAP.md:448` `env_translate.rs` is already EXT-D1-2026-10-12-01.)

## Evidence
`git show 1cb12fb3d --stat`; `ls byroredux/src/boot/`; the clippy output in TD8-2026-10-12-01; `grep -n 'boot.rs\|asset_provider.rs' ROADMAP.md docs/engine/archives.md` (validated at publish time: the Gates paragraph and both paths are still present).

## Impact
ROADMAP is the document the suite and `/session-close` treat as the single home for status; a reader trusting the Gates paragraph skips the clippy gate, which is the one that is actually red. Held at LOW because `/session-close` refreshes it by design.

## Suggested Fix
Rewrite the Gates paragraph to "CI is red on the clippy step (#5621)"; delete the ABBA bullet (or move it to a one-line "fixed 2026-10-12 `1cb12fb3d`"); fix the two paths.

## Related
#5535 (closed), #5621, #5532 / #5335 (earlier ROADMAP drift).

## Completeness Checks
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
