# REG-2026-10-12-02: `merge.rs` still says the Starfield `.mat` format is unparsed, "tracked in #4277"

- **GitHub**: #5760 (https://github.com/matiaszanolli/ByroRedux/issues/5760)
- **Labels**: low,documentation,doc-rot,game:starfield,import-pipeline
- **Source**: docs/audits/AUDIT_REGRESSION_2026-10-12.md (`REG-2026-10-12-02`)

**Audit finding** `REG-2026-10-12-02` — severity LOW — from `docs/audits/AUDIT_REGRESSION_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `byroredux/src/asset_provider/material/merge.rs` (unknown-extension arm comment, "The .mat format is not yet parsed (tracked in #4277)")

## Description
The unknown-extension arm's comment reads "The .mat format is not yet parsed (tracked in #4277)". #4277 closed 2026-10-06 with `asset_provider/material/loose_mat.rs`, and the very next arm up (`merge.rs`, `if starfield_cdb_gate && path.ends_with(".mat")` -> `apply_loose_mat`) decodes a loose `.mat`; another comment in the same function says so ("A loose file decodes through `loose_mat` (#4277/#5396)"). Only the no-CDB fall-through still reaches the generic arm.

## Evidence
`merge.rs` `apply_loose_mat` arm and the stale sentence (still present at HEAD, validated).

## Impact
A reader following the pointer lands on a closed, delivered issue. No runtime effect.

## Suggested Fix
Say the arm is reached only when no CDB is loaded, and drop the "not yet parsed / tracked" clause.

## Related
#4277, #5396, `/audit-starfield`.

## Completeness Checks
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
