**HEAD**: 5d31fd637 · **Baseline**: docs/audits/AUDIT_STARFIELD_2026-10-09.md (HEAD 3bcf6c8e8) · **Audited**: Dims 3, 4, 6 (delta-touched) plus Dim 2 (delta read); Dim 1 corpus gate run · **Unchanged since baseline (skimmed)**: Dims 1, 5 (no logic commits on their paths; guards run)

# Starfield Compatibility Audit — 2026-10-12

This report is one leg of `/audit-suite --preset comprehensive`. One auditor ran every dimension in sequence, with no sub-agents.

The window is `3bcf6c8e8..5d31fd637`. The Starfield-relevant commits are:
- CDB / material: 5c14ebdf8 (#5502 `ObjectInfo.Parent` inheritance), 4667b884b (#5503 emissive parked), dd759b762 (#5291 background index build), 7ceda787d (#5578 default-lane `Parent` decode pin), c961653f4 (#5284 one CDB resolve tail), 0d698d16c (#5661 alpha test needs an authored `HasOpacity=true`), d4352d477 (#5326 real-data gate hardening);
- archives: 5402b2530 (#5288 shared opened-archive handles, memoised CDB path scan);
- ESM: 9dbd59b8b (#5346 one decoder for the Starfield lighting tail), 136048a95 (#5493 PKIN/SCOL `FLTR`);
- mesh: 656e2bdc3 (#5524, doc move and log wording in `skin.rs`), cce3d59b5 (#5596, shared LE readers).

Constraints honoured:
- No engine launch, no `--sf-smoke`, no smoke script, no `--ignored` plugin test.
- No source, skill or issue edits.
- One real-data Starfield test at a time, each under `systemd-run --scope -p MemoryMax=3G..5G`.
- No private cargo target dir was used.

## Guard runs and real-data gates (all green)

| Suite | Result |
|---|---|
| bsa `--lib ba2` | 44/44 |
| nif `--lib` `bs_geometry` / `shader::tests::starfield` / `starfield` / `skin` | 72 / 11 / 32 / 118 (1 ign) |
| sfmaterial (lib + header_smoke + integration) | 41 + 6 + 1 (4 ignored) |
| plugin `starfield` / `lgtm` / `xcll` / `spatial_units` / `pkin` / `dat2` / `txst` | 22 (1 ign) / 5 / 25 (2 ign) / 10 / 12 (1 ign) / 7 / 15 |
| byroredux bin: `starfield_mat`, `asset_provider::material`, `material_translate`, `loose_mat`, `cdb`, `light_anim`, `normalize_mesh_path`, `pre_parse`, `pkin` (1.96 toolchain) | 186 passed, 1 ignored |
| **Dim 1 corpus** `parse_rate_starfield_all_meshes` (release, MemoryMax=5G, 8 s) | **13/13 archives, 120,543 NIFs, 100.00% clean, 0 truncated, 0 failed** |
| **sfmaterial `real_cdb.rs --ignored`** (release, MemoryMax=3G, 6 s) | 4/4. Base CDB indexes 500,403 keyed materials in 1.7 s (HWM 248 MB). The DLC CDB resolves the base corpus (500,385 keyed). The streaming validator reports 97 classes and 1,438,780 values, matching the pin. |
| `sf_smoke` GRUP-coverage example, `Starfield.esm` (release, MemoryMax=5G) | handled by dispatch 86.2% of GRUP bytes (86 FourCCs); unhandled head: SFTR 90.6 MB, GBFM 36.1, PNDT 25.8, STDT 12.0, LMSW 8.0, BIOM 5.3 |

## Executive Summary

Starfield is a first-class `GameKind`:
- **NIF + BA2**: unchanged, and re-measured at 100.00% over 120,543 NIFs.
- **CDB Phase 2 is partial.** The #5502 `Parent`-chain inheritance landed for the settings half; the texture-set slot half is still open (Existing #5502). Emissive is parked (#5503). The alpha-test default is fixed in code (#5661).
- **Walkable Cydonia** is not re-verified, because no engine launch is allowed.

**Delta outcome: 0 CRITICAL, 0 HIGH, 1 MEDIUM, 1 LOW (both NEW).**

**Headline (MEDIUM, SF-2026-10-12-D4-01): Starfield's whole material-swap system is dropped.**
- Starfield replaces `MSWP`/`XMSP` with `LMSW` records (`BGSLayeredMaterialSwap`) and a per-REFR `XLMS`.
- `Starfield.esm` has 12,966 `LMSW` and 689,016 REFRs carrying `XLMS`, which is 20.9% of all 3,291,860 REFRs.
- In Cydonia it is 12,384 of 27,823 REFRs (44.5%).
- Neither record is decoded or consumed, so those REFRs render their base-mesh material instead of the authored variant. The existing MSWP/XMSP pipeline (#4290, #5497) is idle on Starfield.

## Dimension Summaries

| Dim | Verdict | Notes |
|---|---|---|
| 1 BA2 + corpus | unchanged | Only comment/whitespace commits (f4c2b9931, 3efd17e5b) on the paths. `ba2` 44/44. The corpus gate is 100.00% on all 13 archives. |
| 2 BSGeometry | clean | 656e2bdc3 only moved a doc and reworded a log; `convert_bs_geometry_skin_weights` still declines and never clamps. cce3d59b5 moved LE readers out of `decode.rs` (not on the BSGeometry path). 5402b2530 shares one `Arc` archive handle: BA2 extraction is positional, so concurrent worker use is safe. |
| 3 CDB | clean (delta) | #5502 Parent chain verified (`collect_settings` is nearest-wins, depth-capped at 64, cycle-safe). Emissive parking confirmed (`let _ = (..)` in `apply_cdb_material`; the pin test passes). #5661 fix verified (`has_opacity == Some(true)`). Open items below are Existing. |
| 4 ESM + bring-up | 1 MEDIUM, 1 LOW | SF-2026-10-12-D4-01 (LMSW/XLMS), SF-2026-10-12-D4-02 (stale `sf-esm` baseline). #5346 verified behaviour-identical. |
| 5 NIF shader blocks | unchanged | 0 commits on the paths; 11/11 guards. |
| 6 Material flow | clean (delta) | `resolve_through_cdb` is the single `lookup_cdb_material` / `apply_cdb_pbr_fallback` site, and `external_material_resolved` is set on every CDB arm. 0b2ffed89 (MSWP palette row) is FO4-only: Starfield has no `MSWP`. |

Per-dimension notes: `/tmp/audit/starfield/dim_{1..6}.md`.

---

## Findings

### MEDIUM

### SF-2026-10-12-D4-01: Starfield's `LMSW` layered material swaps and the per-REFR `XLMS` reference are dropped; 689,016 REFRs (44.5% of Cydonia) render their un-swapped base material
- **Severity**: MEDIUM (translatable authored data silently dropped; it changes colour and material variants, not geometry, so it stays below the HIGH "removes visible content" bar)
- **Dimension**: ESM Resolve Rate + Cell Bring-up (with Material Flow, the swap consumer)
- **Location**:
  - `crates/plugin/src/esm/cell/walkers.rs:900` (the REFR arm decodes `XMSP` only; `XLMS` falls to the catch-all);
  - `crates/plugin/src/esm/records/` (no `LMSW` or `MTPT` parser; `DISPATCH_HANDLED_FOURCCS` lacks both);
  - consumer that is idle on Starfield: `byroredux/src/cell_loader/spawn/mesh_instance.rs` (`resolve_mesh_paths_with_pre_merge`, the `material_swaps` loop and the #4290 re-merge).
- **Status**: NEW. `docs/engine/starfield-esm-roadmap.md:157` and `starfield-esm-phase0-baseline.md:77` name LMSW as "eventually yes" and defer it behind CDB Phase 2. No open or closed GitHub issue covers it (searches "LMSW", "XLMS", "layered material swap"). The 2026-10-12 ESM audit lists `REFR.XLMS 689 041` in its undecoded-sub-record census only as an observation. Nothing has quantified the REFR coverage.
- **Description**:
  - Starfield has no `MSWP` GRUP (absent from the `sf_smoke` top-level list). Its swap record is `LMSW`, whose schema is `BGSLayeredMaterialSwap` with `Entries`, `SwapsCollection` and `BGSLayeredMaterialSwap::Entry` carrying `KeyMaterialPathForm` (a `MTPT` record holding the source material path) and `OverrideMaterialPath` (a `.mat` path).
  - A REFR selects swaps through `XLMS`, a list of one or more 4-byte `LMSW` FormIDs.
  - The cell loader decodes `XMSP` (FO4 `MSWP`) and never `XLMS`. `LMSW` (8.0 MB, 12,966 records) and `MTPT` (0.2 MB, 877 records) are both unhandled top-level GRUPs.
  - The path-swap machinery behind `XMSP` already does what these entries need: source path → target material path, re-merged through `merge_external_material` (which routes `.mat` through the CDB, #4290).
- **Evidence** (read-only `mmap` walks of `Starfield.esm`; scripts `/tmp/audit/starfield/probe/{mswp,lmsw_refs,lmsw_sample,lmsw_cydonia}.py`, run with `python3 -I`):
  - **Records**: `LMSW` 12,966 (every one carries a `REFL` reflection blob); `MSWP` 0; `MTPT` 877; every LMSW FormID is reachable.
  - **Placements**: 689,016 of 3,291,860 REFRs carry an `XLMS` that resolves to an `LMSW` form (`ACHR` 0 of 9,530).
  - **`citycydoniamainlevel` (0x002B3DA2)**: 12,384 of 27,823 REFRs carry `XLMS` (44.5%) from 564 distinct swap forms. Subrecord sizes are 4 B (9,512), 8 B (2,292), 12 B (344), 16 B (186), 20 B (23), 24 B (4), 28 B (23), so several swaps per REFR is routine.
  - **What the swaps do** (decoded strings from the blobs):

    | Cydonia swap form | REFRs | Entry |
    |---|---|---|
    | `IndustrialKit_Swap_BlueA_IndIntMetalTrimGrey03` (0x1A0953) | 2,301 | override `Architecture\IndustrialKit\Interiors\IndIntMetalTrimBlue03.mat` |
    | `Cydonia_Pipe_GreywithOffWhite01_Collection_01` (0x149FA2) | 935 | pipe collection retint |
    | `IndustrialKit_Swap_RustHeavyA_IndIntMetalTileGrey01` (0x1A097B) | 717 | override `...\RustyWorn\IndIntMetalTileGrey01_RustyWorn01.mat` |
    | `StructuralKit_Tint_LightGrey_Theme01` (0x1D9460) | 510 | theme tint |
    | `indintmetaltilegrey01_To_cydonia_paintedmetal02_offwhite01_Swap` (0x249791) | 348 | `IndIntMetalTileGrey01.mat` → `Architecture\City\Cydonia\Cydonia_Paintedmetal02_OffWhite01.mat` |
- **Impact**:
  - Industrial-kit trim, tiles, pipes and structural pieces in Cydonia render the kit's default grey material instead of the authored blue, rusty or off-white variant.
  - Glow lamps lose their authored colour (`floodlamps_genericglow01_blue_To_glowbright01tint02_Swap` is the first LMSW record).
  - Game-wide this is one REFR in five, so it is the dominant authored-material gap now that CDB texture roles land, and it is independent of the #5502 inheritance work.
- **Related**: #4290 / #5497 (the MSWP consumer this would feed), #5502 (CDB inheritance), `starfield-esm-roadmap.md:157`, the ESM 2026-10-12 observation on `REFR.XLMS`, #1576 (model-less forms).
- **Suggested Fix**:
  - Decode `LMSW` `Entries` (the `REFL` blob is the same BETH container the `sfmaterial` reader handles), resolve each `KeyMaterialPathForm` through a new `MTPT` parser, and decode `XLMS` as a FormID list.
  - Feed the resulting (source path, override path) pairs into the existing `RefrTextureOverlay.material_swaps`, so the #4290 re-merge applies them.
  - Land `MTPT` and `LMSW` with parsers and tests before wiring the REFR side, and census-pin the counts above.

### LOW

### SF-2026-10-12-D4-02: `.claude/audit-baselines/sf-esm/*.tsv` predate 10 dispatch arms, so the `handled` column of the skill's "diff against the baseline" first step is wrong for 20 rows
- **Severity**: LOW (tooling baseline rot; no runtime effect)
- **Dimension**: ESM Resolve Rate + Cell Bring-up (audit infrastructure)
- **Location**: `.claude/audit-baselines/sf-esm/Starfield.tsv` (last touched eac915354, 2026-05-28); the other four `*.tsv` share the cause.
- **Status**: NEW. Nothing tracks it, and no test reads these files (a repo-wide search finds only `sf_smoke.rs`, the skills and docs referencing them).
- **Description**: The checked-in baseline was generated before ten dispatch arms existed, so it still records them as silently skipped. Four rows' byte totals also moved with a later `Starfield.esm` build (installed mtime 2026-07-01).
- **Evidence**: `cargo run --release -p byroredux-plugin --example sf_smoke -- Starfield.esm --tsv --recurse` diffed against the baseline (358 rows each):
  - **`handled` column**: 20 rows flip 0 → 1. Ten are top-level (`OMOD`, `WTHS`, `TRNS`, `SMQN`, `SMBN`, `SECH`, `AOPF`, `SMEN`, `DLBR`, `SCEN`) and ten are the same FourCCs' leaf rows.
  - **Bytes**: 4 rows differ (`WRLD` 863,580,438 → 863,580,497; `NPC_` 6,309,275 → 6,309,192; `MGEF` 249,122 → 249,160; `GMST` 157,032 → 157,113), and `GMST`'s record count moved 2,311 → 2,312.
  - Without `--recurse` the output has no `leaf` rows at all, so a default-flag run diffs 180 rows against this file.
- **Impact**: the "silently skipped" set the first step is meant to expose is overstated by ten FourCCs, so a real regression (a dropped dispatch arm) is lost among 25 expected diffs. The baseline cannot gate anything as committed.
- **Related**: #4278 (the dispatch list the example derives from), `docs/engine/starfield-esm-phase0-baseline.md`.
- **Suggested Fix**: regenerate the five baselines with `--tsv --recurse` against the current ESMs and note the build in the header; consider a default-lane test that checks the `handled` column against `DISPATCH_HANDLED_FOURCCS`.

---

## Known open (re-verified at HEAD, not re-reported)

| Issue | State at HEAD |
|---|---|
| #5489 PKIN template-CELL instancing | `refr.rs:588-600` still skips every CELL-typed CNAM; unchanged. |
| #5533 `sf_smoke` PKIN resolve metric | unchanged. |
| #5509 additive-family blend modes | `merge.rs` still forwards only `"AlphaBlend"`. |
| #5502 (remainder) | texture-set slots and flat colours are not `Parent`-inherited (NIFAL 2026-10-12 carried 2,051 of 10,158 referenced materials untextured). The settings half landed in 5c14ebdf8. |
| #5661 | the alpha-test default is fixed in code (`has_opacity == Some(true)`, 0d698d16c); the issue is still open because the commit carried no `Fix` keyword. |
| #5666 / #5665 | `PREFETCH_GATE` ordering and the missing panic memoisation / `catch_unwind` in the prefetch worker (Existing, PERF/CONC/PAR reports). |
| #5672 / #5673 | silent `Objects`/`Parent` degradation; translucency capture key never matches. |
| #5465 / #5466 / #5525 | `PersistentID` column order, Phase-1 text (`probe_starfield_cdb` still logs "Phase 1 ... deferred Phase 2"), loose-`.mat` capture parity. |
| #5534 | `docs/engine/game-compatibility.md:287` still calls the 108-byte XCLL "Skyrim 92 + a 16-byte tail". |
| #5627 | WTHS instance-value reader. |
| #1576 | model-less STAT/BNDS/ACTI/ARMO forms. |
| REN-D6-2026-10-12-01 | `resolve_through_cdb` sits between `record_external_texture_sources` and its doc comment (`merge.rs`); reported in the renderer report. |

## CRC32 Flag Table

Unchanged. `crates/nif/src/shader_flags.rs` has had no commits since 2026-09-16; the full 32-row derivation is in `AUDIT_STARFIELD_2026-09-11.md` / `-09-16.md`. The `shader::tests::starfield` guards (11/11) include the #4279 `Own_Emit` dual read.

## Remaining-Work Chain

Per `docs/engine/starfield-esm-roadmap.md`, Phases 0+1 are done, and Phases 2–4 were invalidated by the parity measurement. Both parsers have shipped, and the NIF truncation tail is cleared.

0. **Pack-in template-cell instancing** (#5489, with the #5533 metric): 1.21 M template REFRs game-wide, 12,218 in Cydonia.
1. **Layered material swaps** (**SF-2026-10-12-D4-01**): `MTPT` + `LMSW` + `XLMS` → the existing swap overlay. 689,016 REFRs game-wide, 12,384 in Cydonia.
2. **CDB Phase 2 remainder** (#3398):
   - texture-set slot `Parent` inheritance (#5502 remainder);
   - the additive-family blend modes (#5509);
   - an emissive scale decision plus `LayeredEmissivityComponent` (NIFAL);
   - canonical roles for the five parked kinds (#4429) and slot-6 height-blend consumer (#5381);
   - `MaterialParamFloat` → roughness/metalness;
   - a `UseSSS` colour source (the capture key itself is wrong, #5673).
3. **PDCL ahead of GBFM.** Unchanged.
4. **Exterior worldspace tiles, space-cell / planet / GBFM records.** Unchanged (SFTR 90.6 MB, GBFM 36.1 MB and PNDT 25.8 MB are the largest unhandled GRUPs).

## Coverage Notes

- **Not run**: `--sf-smoke` and the engine binary (forbidden); the full 9.19 GB generic CDB `parse`.
- **Probes**: Python `-I` read-only `mmap` walks of `Starfield.esm` in `/tmp/audit/starfield/probe/`. The `sf_smoke` example output is in `/tmp/audit/starfield/sf_smoke_starfield{,_rec}.tsv`.
- **Not measured**: a root-versus-layer settings precedence check on the new `Parent` walk (`MaterialIndex::lookup` walks the root's whole `Parent` chain before the layer materials'); the index internals are private and the probe crate used on 10-09 no longer exists. No defect is claimed.

Suggest: `/audit-publish docs/audits/AUDIT_STARFIELD_2026-10-12.md`
(label every finding `game:starfield` + `legacy-compat`, plus its own domain label: D4-01 `esm-plugin`, D4-02 `tech-debt`.)
