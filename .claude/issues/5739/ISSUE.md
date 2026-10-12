# TD2-2026-10-12-04: `byro-detect` and `byro-launcher` carry the same `--profiles` parser and `absolutise_against_cwd`

- **GitHub**: #5739 (https://github.com/matiaszanolli/ByroRedux/issues/5739)
- **Labels**: low,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD2-2026-10-12-04`)

**Audit finding** `TD2-2026-10-12-04` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `tools/byro-detect/src/main.rs` (`absolutise_against_cwd`, flag loop) and `tools/byro-launcher/src/main.rs` (same)

## Description
The strict flag loop (`-h`, `--profiles <path>` that rejects a following flag and a second occurrence) and `absolutise_against_cwd` are identical modulo the launcher's longer comment. The detect copy says "same absolutisation as the launcher", i.e. the duplication is acknowledged in prose and fixed twice by hand (#5167 detect, #5294 launcher twin, #5476 edited both copies on the same day). Both tools already depend on `byroredux-game-detect`, which owns `load_with_user_path` (the seam both use for this very path).

## Evidence
Clone scan: 6 identical 8-line windows between the two files. Both `fn absolutise_against_cwd` definitions exist at HEAD (validated).

## Impact
A third consumer of `--profiles` (or the next #5476-style cwd bug) is a two-file edit.

## Suggested Fix
Add `parse_profiles_arg(args: &mut impl Iterator<Item = &String>) -> Result<PathBuf, String>` and `absolutise_against_cwd` beside `load_with_user_path` in `crates/game-detect/src/profiles.rs`.

## Related
#5167, #5294, #5476. Label gap: tooling/launcher has no own domain label (filed under `tech-debt`).

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
