# SF-2026-10-12-D4-01: Starfield's `LMSW` layered material swaps and the per-REFR `XLMS` reference are dropped; 689,016 REFRs (44.5% of Cydonia) render their un-swapped base material

- **GitHub**: #5757 (https://github.com/matiaszanolli/ByroRedux/issues/5757)
- **Labels**: medium,game:starfield,legacy-compat,esm-plugin,bug
- **Source**: docs/audits/AUDIT_STARFIELD_2026-10-12.md (`SF-2026-10-12-D4-01`)

**Audit finding** `SF-2026-10-12-D4-01` — severity MEDIUM — from `docs/audits/AUDIT_STARFIELD_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/plugin/src/esm/cell/walkers.rs` (the REFR arm decodes `XMSP` only; `XLMS` falls to the catch-all); `crates/plugin/src/esm/records/` (no `LMSW` or `MTPT` parser; `DISPATCH_HANDLED_FOURCCS` lacks both); idle consumer: `byroredux/src/cell_loader/spawn/mesh_instance.rs` (`resolve_mesh_paths_with_pre_merge`, the `material_swaps` loop and the #4290 re-merge)

## Description
Starfield has no `MSWP` GRUP. Its swap record is `LMSW` (`BGSLayeredMaterialSwap`) with `Entries`, `SwapsCollection` and `BGSLayeredMaterialSwap::Entry` carrying `KeyMaterialPathForm` (a `MTPT` record holding the source material path) and `OverrideMaterialPath` (a `.mat` path). A REFR selects swaps through `XLMS`, a list of one or more 4-byte `LMSW` FormIDs. The cell loader decodes `XMSP` (FO4 `MSWP`) and never `XLMS` (validated at publish time: no `LMSW` / `XLMS` anywhere in the Rust sources). `LMSW` (8.0 MB, 12,966 records) and `MTPT` (0.2 MB, 877 records) are both unhandled top-level GRUPs. The path-swap machinery behind `XMSP` already does what these entries need: source path -> target material path, re-merged through `merge_external_material` (which routes `.mat` through the CDB, #4290). `docs/engine/starfield-esm-roadmap.md:157` and `starfield-esm-phase0-baseline.md:77` name LMSW as "eventually yes" behind CDB Phase 2; nothing had quantified the REFR coverage.

## Evidence
Read-only `mmap` walks of `Starfield.esm` (`/tmp/audit/starfield/probe/{mswp,lmsw_refs,lmsw_sample,lmsw_cydonia}.py`): `LMSW` 12,966 (every one carries a `REFL` blob); `MSWP` 0; `MTPT` 877; every LMSW FormID is reachable. 689,016 of 3,291,860 REFRs carry an `XLMS` that resolves to an `LMSW` form (`ACHR` 0 of 9,530). `citycydoniamainlevel` (0x002B3DA2): 12,384 of 27,823 REFRs carry `XLMS` (44.5%) from 564 distinct swap forms; subrecord sizes 4 B (9,512), 8 B (2,292), 12 B (344), 16 B (186), 20 B (23), 24 B (4), 28 B (23), so several swaps per REFR is routine.

| Cydonia swap form | REFRs | Entry |
|---|---|---|
| `IndustrialKit_Swap_BlueA_IndIntMetalTrimGrey03` (0x1A0953) | 2,301 | override `Architecture\IndustrialKit\Interiors\IndIntMetalTrimBlue03.mat` |
| `Cydonia_Pipe_GreywithOffWhite01_Collection_01` (0x149FA2) | 935 | pipe collection retint |
| `IndustrialKit_Swap_RustHeavyA_IndIntMetalTileGrey01` (0x1A097B) | 717 | override `...\RustyWorn\IndIntMetalTileGrey01_RustyWorn01.mat` |
| `StructuralKit_Tint_LightGrey_Theme01` (0x1D9460) | 510 | theme tint |
| `indintmetaltilegrey01_To_cydonia_paintedmetal02_offwhite01_Swap` (0x249791) | 348 | `IndIntMetalTileGrey01.mat` -> `Architecture\City\Cydonia\Cydonia_Paintedmetal02_OffWhite01.mat` |

## Impact
Industrial-kit trim, tiles, pipes and structural pieces in Cydonia render the kit's default grey material instead of the authored blue, rusty or off-white variant; glow lamps lose their authored colour. Game-wide this is one REFR in five, the dominant authored-material gap now that CDB texture roles land, independent of the #5502 inheritance work. It changes colour/material variants, not geometry (hence MEDIUM).

## Suggested Fix
Decode `LMSW` `Entries` (the `REFL` blob is the same BETH container the `sfmaterial` reader handles), resolve each `KeyMaterialPathForm` through a new `MTPT` parser, and decode `XLMS` as a FormID list. Feed the resulting (source path, override path) pairs into the existing `RefrTextureOverlay.material_swaps`, so the #4290 re-merge applies them. Land `MTPT` and `LMSW` with parsers and tests before wiring the REFR side, and census-pin the counts above. Note FO4-2026-10-12-D4-01 (the shared swap compare form) applies to the consumer.

## Related
#4290 / #5497 (the MSWP consumer this would feed), #5502 (CDB inheritance), FO4-2026-10-12-D4-01, `starfield-esm-roadmap.md:157`, #1576.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **CANONICAL-BOUNDARY**: Per-game logic stays at the NIFAL parser->`Material` boundary (never pushed into shaders/renderer or re-derived at render time). See `/audit-nifal`.
- [ ] **TESTS**: A regression test pins this specific fix
