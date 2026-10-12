# FO4-2026-10-12-D4-01: The per-shape MSWP swap compares the bare ESM source path verbatim against the NIF's `Materials\…` / absolute-build-path name, so a REFR material swap never fires on real data (and #5497's palette row with it)

- **GitHub**: #5755 (https://github.com/matiaszanolli/ByroRedux/issues/5755)
- **Labels**: high,game:fo4,legacy-compat,esm-plugin,nifal,renderer,test-gap,bug
- **Source**: docs/audits/AUDIT_FO4_2026-10-12.md (`FO4-2026-10-12-D4-01`)

**Audit finding** `FO4-2026-10-12-D4-01` — severity HIGH — from `docs/audits/AUDIT_FO4_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `byroredux/src/cell_loader/spawn/mesh_instance.rs` (`resolve_mesh_paths_with_pre_merge`: `current` from `refr_ov.material_path.or(mesh.material.material_path)`, then `entry.source.eq_ignore_ascii_case(&current)`); `crates/nif/src/import/material/dedicated_shader.rs` + `crates/nif/src/import/mesh/material_path.rs` (the NIF name is interned verbatim); `crates/plugin/src/esm/records/mswp.rs` (`BNAM` kept as read)

## Description
The swap matches by string equality between the MSWP entry's `BNAM` ("Original" material) and the shape's material path (validated at publish time: the compare is still the verbatim `entry.source.eq_ignore_ascii_case(&current)`). The two sides have different canonical forms in the shipped data. MSWP `BNAM`/`SNAM` are **bare** (relative to `Data\Materials\`): across all seven masters, 9,065 of 9,065 `BNAM` and 9,065 of 9,065 `SNAM` have no `Materials\` prefix. A sample of every seventh NIF in `Fallout4 - Meshes.ba2` + `MeshesExtra.ba2` (8,808 `.bgsm`/`.bgem` shader names) is 49.7% `Materials\...`, 42.7% an absolute build path (`C:\Projects\Fallout4\Build\PC\Data\materials\...`), 1.1% `Data\materials\...`, 6.4% `template/...` parents, 0% bare. `ImportedMaterial.material_path` is the NIF name interned verbatim (`material_path_from_name` is a suffix gate; `normalize_material_path`, `byroredux/src/asset_provider/archive.rs`, runs only inside the provider lookup). Every unit test authors the same string on both sides (`r"materials\kit\machinekitquad03.bgsm"` in `mswp_identity_entry_stamps_its_palette_row`), so none can see it. 127 `BNAM` entries use the CK's documented wildcard (`landscape\ground\craterdebris01*.bgsm`); the compare is literal, so a wildcard entry could not match even with the forms aligned. The eager `build_refr_texture_overlay` arm (`refr.rs`) is not affected: it compares the XATO TXST `MNAM`, also bare. The common vanilla shape is XMSP without XATO, which only reaches the per-shape arm.

## Evidence
Real-data simulation of the engine's rule (`/tmp/audit/fo4/py/mswp6.py`) for every `Fallout4.esm` REFR/ACHR with `XMSP` that is not in the cell's XCRI pair list (the placements the per-REFR pass actually spawns): take the base record's `MODL`, read the NIF's `.bgsm`/`.bgem` strings from `Meshes.ba2` / `MeshesExtra.ba2`, test each of the MSWP's `BNAM`s:
```
XMSP refs 154,153 · absorbed by XCRI 146,996 · NOT absorbed 7,157 (312 distinct MSWP)
resolvable placements 7,132
  engine rule (case-insensitive verbatim equality)      matches: 0
  after stripping build prefix / "materials\"            matches: 6,352 (89%)
  placements where the matched entry changes the path   5,403
```
Concrete rows: `PictureFrame05Dest.nif` shader `Materials\SetDressing\PaintingsGeneric\PaintingGeneric_CanvasSwap.BGSM` vs `BNAM setdressing\paintingsgeneric\PaintingGeneric_CanvasSwap.BGSM` -> `SNAM ...\PaintingGeneric21.BGSM`; `VltElevator.nif` `Materials\Interiors\Vault\Vault111\V111ElevPlatform01.BGSM` vs `interiors\vault\vault111\v111elevplatform01.bgsm`; `StationWagon01a_Rust` / `VltLight01`. The DLCs add 85,485 further `XMSP` REFRs (not resolved). Static real-data simulation, not a live capture.

## Impact
Every non-baked `XMSP` placement renders the NIF-default material: wrong picture in a frame, wrong vault number, wrong vehicle paint, no machine-kit colour variant. 7,132 resolvable placements in `Fallout4.esm` alone; the precombine-absorbed 146,996 are fine (the CK baked their effective material) unless their `_oc.nif` is missing, when `effective_absorbed_refs` un-skips them into this same dead path. **#5497's REFR half is inert on real data**: `mswp_remap_row` is only set inside the `eq_ignore_ascii_case` branch, so the CNAM palette row (`MachineKitGray01` 0.805 and the other 12,302 CNAM-carrying REFRs) is never stamped. The swapped target's BGSM chain (`fill_from_bgsm`, the #4290 re-merge) is never reached.

## Suggested Fix
Canonicalize both sides once with the existing `normalize_material_path` (lower-cased, `materials\`-prefixed, build prefix and `data\` stripped) before the compare, in both `mesh_instance.rs` and `refr.rs`, or canonicalize the MSWP entries at parse time and the shape name at capture; honour the `*` wildcard. Add a real-data pin next to `fo4_palette_corpus.rs`: a known unbaked `XMSP` placement (e.g. `PictureFrame05Dest` + a `CanvasSwap` MSWP) must change `material_path`, with the NIF-form string and the bare ESM-form string as inputs (fixtures that share one literal are the gap). Fix together with FO4-2026-10-12-D4-02.

## Related
#971, #973, #3242, #4290, #4400, #5497 (the palette half this makes inert), FO4-2026-10-12-D4-02 (second defect at the same compare site).

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **CANONICAL-BOUNDARY**: Per-game logic stays at the NIFAL parser->`Material` boundary (never pushed into shaders/renderer or re-derived at render time). See `/audit-nifal`.
- [ ] **TESTS**: A regression test pins this specific fix
