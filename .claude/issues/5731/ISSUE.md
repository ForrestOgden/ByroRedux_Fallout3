# #5731 — EXT-D1-2026-10-12-01: Doc rot after the #5584 split and the LOD fix wave

**Labels**: `low`, `terrain-exterior`, `documentation`, `doc-rot`

**Source**: `docs/audits/AUDIT_EXTERIOR_2026-10-12.md` (EXT-D1-01) · audited at HEAD 5d31fd637

- **Severity**: LOW (doc rot)
- **Dimension**: EXAL boundary discipline (docs)
- **Tier Violated**: n/a
- **Game Affected**: all (docs)
- **Status**: NEW

## Locations
- `byroredux/src/env_translate/mod.rs:15-22`: "Step 1 (this slice) establishes the module and gathers the two already-single-site **water** translates here verbatim … Later steps fold in the scattered sky / sun / weather producers". They have long been folded in.
- `ROADMAP.md:448`: the size tripwire still lists `byroredux/src/env_translate.rs` at 5,826 lines. The largest piece is now `env_translate/water.rs` at 2,487.
- `docs/engine/exal.md` section 5: no text for #5496 (PNAM `0x02` "Use LOD Data" owns no quads; every ring key resolves on `lod_source_worldspace`) and none for the authored `lodsettings\<ws>.lod` descriptor (see EXT-D6-2026-10-12-01).

## Suggested Fix
Rewrite the `mod.rs` header as the five-submodule layout it already lists below it. Refresh the ROADMAP tripwire at the next `/session-close`. Add one paragraph to exal.md section 5 for the inherited-LOD key and the `.lod` descriptor once the lattice-origin fix lands.

## Completeness Checks
- [ ] **SIBLING**: grep other `env_translate/*` module headers for "Step 1 / this slice" language
