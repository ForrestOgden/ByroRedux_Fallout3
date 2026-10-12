# NIFAL-D9-2026-10-12-01: `geometry_only_cached`'s doc comment is glued onto the new `PrecombineCacheLookup` enum

- **GitHub**: #5730 (https://github.com/matiaszanolli/ByroRedux/issues/5730)
- **Labels**: low,nifal,game:fo4,documentation,doc-rot
- **Report**: docs/audits/AUDIT_NIFAL_2026-10-12.md

**Severity**: LOW

## Location
`byroredux/src/cell_loader/precombined.rs:1055-1058` (the orphaned doc), `:1059-1083` (enum + `lookup_precombine_cache`), `:1085` (`geometry_only_cached`)

Tier violated: n/a (doc-rot). Game affected: FO4 (precombine path).

## Description
#5522 (2ca9bc313) inserted `PrecombineCacheLookup` and `lookup_precombine_cache` directly under the doc block "Wrap precombine-decoded meshes in a geometry-only `CachedNifImport` …", which belonged to `geometry_only_cached`. rustdoc now renders that text, followed by the enum's own, as the enum's documentation. `geometry_only_cached`, a `CachedNifImport` producer, is left without its doc.

## Evidence
`precombined.rs:1055-1058` runs straight into `/// What the import registry already knows about one _oc.nif path.` with no blank doc separator.

## Impact
Documentation only.

## Suggested Fix
Move the two-line "Wrap precombine-decoded meshes …" block back above `pub(super) fn geometry_only_cached`.

**Related**: #5522, REN-D6-2026-10-12-01

## Completeness Checks
- [ ] **SIBLING**: Same splice class at `byroredux/src/asset_provider/material/merge.rs` (REN-D6-2026-10-12-01) — tracked separately

Fix with: `/fix-issue 5730`
