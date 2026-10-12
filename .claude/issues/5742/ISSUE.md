# TD4-2026-10-12-01: Four "known-open" callouts in two skills name issues that closed today

- **GitHub**: #5742 (https://github.com/matiaszanolli/ByroRedux/issues/5742)
- **Labels**: low,documentation,doc-rot,tech-debt
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD4-2026-10-12-01`)

**Audit finding** `TD4-2026-10-12-01` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `.claude/commands/audit-nifal/SKILL.md` (#5284 clause in the Vocabulary bullet; #5285 clause in the boundary-inventory bullet); `.claude/commands/audit-tooling/SKILL.md` (#5149 and #5147 clauses in the Guards line)

## Description
`audit-nifal` says "the CDB resolve tail is copied at four `merge.rs` sites (#5284)" and "known-open #5285: the declared boundary lists six beam converters, `CachedNifImport::beam_volumes` chains seven"; `audit-tooling` says "a `0.0.0.0` bind would pass every guard (known-open #5149). Known-open #5147: default output names are not unique". All four issues closed 2026-10-12: `c961653f4` (#5284), `661be4eaa` (#5285), `224180508` (#5149), `72a63616b` (#5147). A pass that dedups by "known-open, cite" now suppresses a regression of fixed code.

## Evidence
`gh issue list --state open` contains none of the four numbers; commit subjects above; the clauses are still present in both skills (validated at publish time).

## Impact
Same fix-wave rot as 10-11's TD4-02 (#5606); the dedup rule in `_audit-common.md` makes a stale "known-open" an active blindfold (#3199).

## Suggested Fix
Replace the four clauses with the fix commit and the pin that now guards it (`beam_volumes` count test; a loopback-address test; the self-unique default names), per the "Fixed since ..." convention `audit-oblivion` already uses.

## Related
#5606, #3199.

## Completeness Checks
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
