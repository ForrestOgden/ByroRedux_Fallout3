# #5689 — ESM-2026-10-12-D5-01: docs still describe the pre-#5484 / #5493 / #5497 semantics (`XPRI` as the absorbed-ref list, `FLTR` as a FormID filter, `CNAM` as an intensity)

**Labels**: `low`, `esm-plugin`, `documentation`, `doc-rot`

**Source**: `docs/audits/AUDIT_ESM_2026-10-12.md` (ESM-D5-01) · audited at HEAD 5d31fd637

- **Severity**: LOW (doc rot; one of the claims is the exact one that produced #5484, every baked STAT/SCOL drawn twice)
- **Dimension**: CELL / WRLD Walkers & Placement Data (docs)
- **Record / Sub-record**: `CELL` / `XCRI`, `XPRI`; `PKIN` / `FLTR`; `MSWP` / `CNAM`
- **Status**: NEW. The fix commits touched code only (`777351ec1` has no `docs/` entry; `0b2ffed89`/`136048a95` did not edit the module docs below).

## Locations
- `docs/engine/esm-records.md:101` and `:680`: `pkin.rs` described as reading "the FLTR workshop build-mode filter". It is the CK object-window filter path (a zstring, never a FormID; #5493).
- `docs/engine/esm-records.md:303`: `CellData` documented with `pub absorbed_refs: HashSet<u32>, // XCRI/XPRI absorbed REFRs`. The live field is `absorbed_ref_bakes: Vec<(u32, u32)>` (XCRI (ref, bake-hash) pairs; `XPRI` is validated but not stored).
- `docs/engine/esm-records.md:699-700`: "reads the cell's `XCRI` / `XPRI` sub-records into `precombined_mesh_hashes` + `absorbed_refs`".
- `docs/engine/game-compatibility.md:229`: "`XPRI` (absorbed-REFR form-IDs)".
- `docs/engine/asset-pipeline.md:120`: names the `absorbed_refs` set.
- `crates/plugin/src/esm/records/mswp.rs:21`: module doc still says "`CNAM` = optional 4-byte f32 intensity / colour multiplier" while the field doc at `:55-67` says that reading was wrong and the value is the palette row (`color_remap_index`).

## Description
The code and field docs are right, but the reference docs and one module header still carry the old model. The XPRI claim is the dangerous one: a reader or future decoder author following `esm-records.md` will re-derive the inverted gate. `the_documented_esm_cell_index_lists_every_field` pins the `EsmCellIndex` listing only, not the `CellData` block at `:303`.

## Evidence
`crates/plugin/src/esm/cell/mod.rs` (`pub absorbed_ref_bakes: Vec<(u32, u32)>`), `crates/plugin/src/esm/cell/helpers.rs` (XCRI decoded, XPRI arm `{}`), `crates/plugin/src/esm/records/pkin.rs` (the corrected module doc).

## Impact
Documentation only; misleads whoever next touches the precombine, PKIN or MSWP code.

## Related
#5484, #5493, #5497, #2699.

## Suggested Fix
Update the six sites to the current field names and meanings (`absorbed_ref_bakes`, XCRI pairs, XPRI = disjoint previs list not stored; `FLTR` = CK filter path; `CNAM` = Color Remapping Index) and add the `CellData` block to a doc-pin test the way the `EsmCellIndex` one works.

## Completeness Checks
- [ ] **SIBLING**: grep `docs/engine` and module headers for any other `absorbed_refs` / "FLTR … FormID" / "CNAM … intensity" mention
- [ ] **TESTS**: A doc-pin test covers the `CellData` block like the `EsmCellIndex` one
