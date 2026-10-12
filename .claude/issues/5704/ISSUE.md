# REN-D6-2026-10-12-01: `resolve_through_cdb` was inserted between `record_external_texture_sources` and its doc comment

- **GitHub**: #5704 (https://github.com/matiaszanolli/ByroRedux/issues/5704)
- **Labels**: low,nifal,renderer,documentation,doc-rot
- **Report**: docs/audits/AUDIT_RENDERER_2026-10-12.md

**Severity**: LOW (doc-rot)

## Location
`byroredux/src/asset_provider/material/merge.rs`, the doc block ending "where the old loop's separate index walk lived." (#3903) and the `/// #5284 — the CDB resolve tail, once.` block, `fn resolve_through_cdb`, then `fn record_external_texture_sources`.

## Description
The existing doc comment for `record_external_texture_sources` (the #3903 `zip_map_ref` rationale) ran straight into the new `/// #5284 …` lines, so rustdoc glues both into one comment on `resolve_through_cdb`. `record_external_texture_sources` is left with no doc comment, and `resolve_through_cdb`'s docs open with an unrelated paragraph about texture provenance roles.

## Evidence
No blank or non-doc line separates the two `///` blocks (`merge.rs` ~lines 100-124); the next item after them is `fn resolve_through_cdb(`; `fn record_external_texture_sources(` (~156) follows the closing brace with no `///` above it. Introduced by `c961653f4` (#5284).

## Impact
rustdoc and anyone reading `merge.rs` see the wrong description on both functions. Code behaviour is unaffected.

## Suggested Fix
Move the #3903 block down to sit directly above `record_external_texture_sources`.

**Related**: #5284, #3903, #5029 (the same class of splice)

## Completeness Checks
- [ ] **SIBLING**: Same splice class at `byroredux/src/cell_loader/precombined.rs` (NIFAL-D9-2026-10-12-01) — tracked separately

Fix with: `/fix-issue 5704`
