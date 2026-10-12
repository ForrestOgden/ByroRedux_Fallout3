# FO4-2026-10-12-D4-02: MSWP `FNAM` is the CK object-window "Tree Folder", decoded and applied as a runtime path-prefix filter

- **GitHub**: #5756 (https://github.com/matiaszanolli/ByroRedux/issues/5756)
- **Labels**: medium,game:fo4,legacy-compat,esm-plugin,doc-rot,bug
- **Source**: docs/audits/AUDIT_FO4_2026-10-12.md (`FO4-2026-10-12-D4-02`)

**Audit finding** `FO4-2026-10-12-D4-02` — severity MEDIUM — from `docs/audits/AUDIT_FO4_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/plugin/src/esm/records/mswp.rs` (`path_filter`: decoded and documented as a "path-prefix filter"); consumers `byroredux/src/cell_loader/refr.rs` (`build_refr_texture_overlay`, `starts_with` filter) and `byroredux/src/cell_loader/spawn/mesh_instance.rs` (`current.to_ascii_lowercase().starts_with(&filter)`)

## Description
The Creation Kit's Material Swap editor dialog lists "Tree Folder:" (with no runtime semantics) before the "Material Substitutions" list (`falloutck-uesp-wiki/(main)/Material Swap.wiki`). The data agrees: an FNAM names the editor tree, not a prefix of the swapped materials. The parser keeps it as `path_filter` and both consumers apply it as a runtime prefix filter (validated at publish time). Same class as #5493 (SCOL/PKIN `FLTR` decoded as a FormID array; it is the CK filter path). Currently masked by FO4-2026-10-12-D4-01 (nothing reaches the filter on real data) and becomes the next failure once that is fixed.

## Evidence
All seven masters, bare-to-bare, the most generous reading of the filter: 3,937 MSWP records, 1,482 carry `FNAM`, 4,455 `BNAM` in those; only 2,285 (51%) have the FNAM as a prefix; **806 records have every swap rejected**, 133 more lose some. Examples: `FNAM Interiors\Vault` with `BNAM SetDressing\Vault\VltLight01.BGSM` (the six `0x24030a-0x24030e` vault-light swaps); `FNAM Effects` with `BNAM Vehicles\Automotive\Car_Glass01.BGEM`. Under the engine's actual forms the filter fails on every record, because the shape name carries `Materials\`.

## Impact
After FO4-2026-10-12-D4-01 is fixed, 54% of FNAM-carrying MSWP records (806 of 1,482) still drop all their swaps, silently. The record doc and the "Set on ~53% of vanilla records" comment bake the wrong semantics in for the next reader.

## Suggested Fix
Drop the runtime filter (keep the string for tooling), fix the `mswp.rs` module and field docs, and pin it with a unit test that applies a swap whose FNAM is not a prefix of its `BNAM`.

## Related
FO4-2026-10-12-D4-01, #5493, ESM-2026-10-12-D5-01 (doc text for XPRI / absorbed_refs / FLTR / MSWP CNAM; `mswp.rs:21`).

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
