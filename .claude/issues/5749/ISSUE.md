# FO3-2026-10-12-D3-02: `ROADMAP.md`'s M35 row says FO3 and FNV get `_far.nif` object LOD; the placement scheme is Oblivion-only

- **GitHub**: #5749 (https://github.com/matiaszanolli/ByroRedux/issues/5749)
- **Labels**: low,documentation,doc-rot,game:fo3,game:fnv,legacy-compat
- **Source**: docs/audits/AUDIT_FO3_2026-10-12.md (`FO3-2026-10-12-D3-02`)

**Audit finding** `FO3-2026-10-12-D3-02` — severity LOW — from `docs/audits/AUDIT_FO3_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `ROADMAP.md` M35 "Terrain LOD" row ("Oblivion/FO3/FNV `_far.nif` object LOD (#1726)")

## Description
`placement_lod_supported(game)` is `game == GameKind::Oblivion` (`byroredux/src/cell_loader/placement_lod.rs`, validated at publish time), pinned by `placement_lod_supported_is_oblivion_only`. FO3 ships 2 `*_far.nif` (`ravecity_far.nif`, `washmonumentlod_far.nif`, measured across all 6 archives) and 0 `distantlod\*.lod`; FNV ships neither. FO3/FNV distant objects are the `landscape\lod\<world>\blocks\` quads. `docs/feature-matrix.md` and `exal.md` state this correctly; only the ROADMAP row contradicts them. #1726 (closed) was filed for all three games; `placement_lod_supported` was later narrowed (FO3-D4-01 / #2086).

## Evidence
`ROADMAP.md:274` text (still present) vs `placement_lod.rs` `placement_lod_supported` and its pin test.

## Impact
A reader of the roadmap's shipped list believes FO3/FNV have a placement-list LOD path that never runs.

## Suggested Fix
Reword to "Oblivion `_far.nif` placement LOD (#1726); FO3/FNV `blocks\` quad LOD (#3321)".

## Related
#1726, #2086, #3321.

## Completeness Checks
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
