# SpeedTree Subsystem Audit — 2026-10-12

**HEAD**: 5d31fd637 · **Baseline**: [AUDIT_SPEEDTREE_2026-10-08.md](AUDIT_SPEEDTREE_2026-10-08.md) (HEAD `00f580e09`) · **Audited**: Dimension 3 (TREE→Billboard Wiring; changed cross-cut paths), with Dimension 4 and Dimension 6 re-verified from source · **Unchanged since baseline (skimmed)**: Dimension 1 (Walker Byte-Accounting), Dimension 2 (Placeholder Fallback), Dimension 5 (Tag Dictionary). `crates/spt/` has two commits since baseline and neither changes behaviour: `86cd60621` (#5361, doc/comment only) and `07ccfdd89` (#5477, a clippy re-wrap of the same doc block).

**Depth**: `deep`; the real-data corpus was re-run for all three games, one lane at a time.
**Execution**: single pass, no sub-agents, no engine launch. Scratch notes: `/tmp/audit/speedtree/dim_1.md` … `dim_6.md`.

## Commands run

| Lane | Result |
|---|---|
| `cargo test -q -p byroredux-spt` (1.96.0) | Pass (55 unit + 3 synthetic; corpus tests ignored) |
| `cargo check -p byroredux-spt --features recon --examples` | Clean |
| `cargo test -p byroredux-spt --features recon --lib` | 60/60 |
| Corpus `--release --test parse_real_spt -- --ignored --nocapture --test-threads=1` with `BYROREDUX_{FNV,FO3,OBL}_DATA`, `BYROREDUX_REQUIRE_GAME_DATA=1` | 100 % unknown-tag-clean: FNV 10/10, FO3 10/10, Oblivion 113/113 (22,092 entries). `walker_stops_on_true_tlv_boundary`: FNV 10, FO3 10, OBL 113, SI 26 — all on the boundary, shift 0, 0 EOF stops, 0 extract-fail, 0 parse-fail |
| `cargo test -p byroredux --bin byroredux -- {billboard, parse_and_import_spt, vanilla_tree, tree_model, tree_icon, spt, speedtree, malformed_spt} --include-ignored` (game data present) | All pass (billboard 14, parse_and_import_spt 2, vanilla_tree 2, tree_model 2, tree_icon 2, spt 9, speedtree 7, malformed_spt 1) |

## Delta reviewed

| Commit | SpeedTree surface | Verdict |
|---|---|---|
| `86cd60621` (#5361) | `parser.rs:82-90` docstring, `parse_real_spt.rs` | Doc only. The five fatal conditions are now stated as two kinds (`InvalidData` x4, `UnexpectedEof` x1) and all three #4122 culprits are named. Closes the baseline's carried LOW. |
| `07ccfdd89` (#5477) | same docstring | Markdown-quote re-wrap, no code change. |
| `c1a99c25e` (AttachLight refactor) | `CachedNifImport.flame_attach_offset` → `attach_light_offset`; `spawn/mesh_instance.rs` light spawn | The SpeedTree placeholder carries `attach_light_offset: None` (`references/import.rs:533`) and `lights: Vec::new()`, so the new light-entity branch is unreachable for `.spt`. The mesh-level `Billboard` insert (`mesh_instance.rs:1289`) and `SpeedTreeWind` insert (`:1295`) are untouched. One stale comment is left behind (SPT-2026-10-12-D3-01). |
| `0b2ffed89` (#5497) | `mesh_instance.rs` MSWP palette row | Applies only to REFRs with `material_swaps`; a `.spt` TREE placeholder has none. |
| `3031d8976` (#5281), `7d1de4ad5` (#5210) | `material_translate.rs` texture-only boundary | Renaming/collapse of the texture-only variants; the placeholder goes through `translate_material*`, not the texture-only boundary. |
| `b5ae3e5cb` (#5512), `b6aeecf56` (#5491) | `streaming/pre_parse.rs` | `is_spt_model_path` (`:628`) and `PreflightDecision::SkipSpt` (`:753`, `:769`) intact; the stale-payload salvage does not touch the `.spt` skip. |
| `51feb00a4` (#5472), `3efd17e5b` (#5470) | comment/log re-points in `nif_loader.rs`, `references/` | No behaviour change. |

Invariants re-checked at HEAD:
- Both routes call `parse_spt` then `import_spt_scene`: `references/import.rs:404/498`, `scene/nif_loader.rs:268/292` (`SptImportParams::default()`).
- `detect_variant` only logs (`import.rs:414`, `nif_loader.rs:276`); nothing branches on it.
- `late.rs:82` declares `reads::<SpeedTreeWind>()`; `billboard.rs` unchanged since baseline.
- Loose route still inserts `SpeedTreeWind::new(1.0, 0.0)` (`nif_loader.rs:1428`, `:1648`).
- Placeholder foliage overrides (`metalness_override: Some(0.0)`, `roughness_override: Some(0.85)`, alpha-test func 6 threshold 0.5, two-sided) hold (`import/mod.rs:389-396`).

## Dedup and baseline carry-over

| Prior item | State at HEAD |
|---|---|
| #5360 (m-trees.sh `(no entities)` misread) | CLOSED, fixed by `811499b65`. Not regressed. |
| #5361 (spt docstring / gate docstring) | CLOSED, fixed by `86cd60621`. Not regressed. |
| #5324 (skill rot incl. audit-speedtree open question on closed #3740) | OPEN, tech-debt-owned; skill text, not code. Not re-filed. |
| REFR.XSED SpeedTree seed (ESM audit, 129,302 Oblivion / 39,813 FO3 / 18,586 FNV occurrences) | **Existing: AUDIT_ESM_2026-10-12 (D2 skipped-subrecord census).** No consumer exists in `byroredux/src` or `crates/`. This audit's dimension gives no reason to raise it: the placeholder is a procedural-definition-free billboard (no geometry, #3808), so the per-REFR seed has nothing to vary until real branch/leaf geometry is generated (`exal-trees.md` §3/§10). Not a new finding. |

Searches: open issues (`/tmp/audit/issues.json`) for `spt`, `speedtree`, `tree`, `billboard`, `m-trees`, `wind`, `xsed` match only #5324, #4913 (Skyrim tree LOD, exterior-owned) and unrelated items. `gh issue list --state all --search "speedtree OR spt OR TREE"`: no regression of any closed SpeedTree issue.

---

## Findings

### SPT-2026-10-12-D3-01: SpeedTree placeholder's `attach_light_offset` comment still says "flame markers / Phase 18"
- **Severity**: LOW
- **Dimension**: TREE→Billboard Wiring
- **Location**: `byroredux/src/cell_loader/references/import.rs:530-533`
- **Status**: NEW (same doc-rot class as NIFAL-D3-2026-10-12-01, but a different site; that finding covers `attach.rs:866-868`)
- **Description**: `c1a99c25e` renamed `CachedNifImport.flame_attach_offset` to `attach_light_offset` and redefined it as the model's exact-name `AttachLight` node offset (the Phase 18 substring "flame marker" rule was retired). The `.spt` constructor's comment was not updated.
- **Evidence**:
  ```rust
  // SpeedTree placeholders carry no flame markers — they're
  // pure billboard quads. Phase 18.
  attach_light_offset: None,
  ```
- **Impact**: Documentation only. The value `None` is correct (a billboard quad has no `AttachLight` node), but the comment describes a mechanism that no longer exists.
- **Related**: NIFAL-D3-2026-10-12-01, `c1a99c25e`.
- **Suggested Fix**: Reword to "SpeedTree placeholders have no `AttachLight` node — they are pure billboard quads."

---

No other findings. Dimensions 1, 2 and 5 are unchanged since 2026-10-08 (guards green, corpus 159/159 on the true TLV boundary); Dimension 4 route parity and `detect_variant` log-only use re-verified from source; Dimension 6 placeholder overrides unchanged and the single `translate_material*` boundary intact.

`/audit-publish docs/audits/AUDIT_SPEEDTREE_2026-10-12.md` (labels `speedtree` + `doc-rot`; one LOW, may be folded into the NIFAL doc-rot batch).
