# SF-2026-10-12-D4-02: `.claude/audit-baselines/sf-esm/*.tsv` predate 10 dispatch arms, so the `handled` column of the skill's "diff against the baseline" first step is wrong for 20 rows

- **GitHub**: #5758 (https://github.com/matiaszanolli/ByroRedux/issues/5758)
- **Labels**: low,game:starfield,legacy-compat,tech-debt,bug
- **Source**: docs/audits/AUDIT_STARFIELD_2026-10-12.md (`SF-2026-10-12-D4-02`)

**Audit finding** `SF-2026-10-12-D4-02` — severity LOW — from `docs/audits/AUDIT_STARFIELD_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `.claude/audit-baselines/sf-esm/Starfield.tsv` (last touched eac915354, 2026-05-28); the other four `*.tsv` (BlueprintShips-Starfield, Constellation, OldMars, ShatteredSpace) share the cause

## Description
The checked-in baseline was generated before ten dispatch arms existed, so it still records them as silently skipped. Four rows' byte totals also moved with a later `Starfield.esm` build (installed mtime 2026-07-01). No test reads these files (a repo-wide search finds only `sf_smoke.rs`, the skills and docs referencing them).

## Evidence
`cargo run --release -p byroredux-plugin --example sf_smoke -- Starfield.esm --tsv --recurse` diffed against the baseline (358 rows each):
- `handled` column: 20 rows flip 0 -> 1. Ten are top-level (`OMOD`, `WTHS`, `TRNS`, `SMQN`, `SMBN`, `SECH`, `AOPF`, `SMEN`, `DLBR`, `SCEN`) and ten are the same FourCCs' leaf rows.
- Bytes: 4 rows differ (`WRLD` 863,580,438 -> 863,580,497; `NPC_` 6,309,275 -> 6,309,192; `MGEF` 249,122 -> 249,160; `GMST` 157,032 -> 157,113), and `GMST`'s record count moved 2,311 -> 2,312.
- Without `--recurse` the output has no `leaf` rows at all, so a default-flag run diffs 180 rows against this file.

## Impact
The "silently skipped" set the first step is meant to expose is overstated by ten FourCCs, so a real regression (a dropped dispatch arm) is lost among 25 expected diffs. The baseline cannot gate anything as committed.

## Suggested Fix
Regenerate the five baselines with `--tsv --recurse` against the current ESMs and note the build in the header; consider a default-lane test that checks the `handled` column against `DISPATCH_HANDLED_FOURCCS`.

## Related
#4278 (the dispatch list the example derives from), `docs/engine/starfield-esm-phase0-baseline.md`.

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix
- [ ] **GATE**: The relevant gate (`cargo clippy`, hygiene test, `_audit-validate.sh`) is green after the change
