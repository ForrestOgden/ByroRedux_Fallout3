# Oblivion (TES4) Compatibility Audit — 2026-10-12

**HEAD**: 5d31fd637 · **Baseline**: `docs/audits/AUDIT_OBLIVION_2026-10-09.md` (HEAD `3bcf6c8e8`; 264 commits since) · **Audited**: Dims 2, 3, 4, 5 (commits on their Paths), plus a census-driven sweep of the Oblivion NPC spawn path and the INFO condition surface · **Unchanged since baseline (skimmed)**: Dim 1 (the only commit on its Paths that touches Oblivion decode is `526cb07d9`, #5537; corpus lane re-run green)

This run is part of the 2026-10-12 `/audit-suite --preset comprehensive`.
- **How it was run**: solo, read-only, against the vanilla Oblivion + DLC data at `/mnt/data/SteamLibrary/steamapps/common/Oblivion/Data/`.
- **Not run (suite rule)**: the engine binary and every device-bound smoke (`m-exteriors.sh oblivion`, `m48-4-oblivion-hud.sh`, `p0-door-interaction.sh oblivion`). The user's FNV engine instance was running.
- **Scratch**: `/tmp/audit/oblivion/` (`dim_1..5.md`, `corpus_lane.log`, `bin_tests.log`, `tools/` with the TES4 GRUP walker `esmwalk.py`, a BSA v103 lister, and the census scripts `greet.py` / `condcensus.py` / `condcmp.py`).
- **Cargo target**: the repo `target/`, 1.96.0 toolchain. No private target dir.

## Executive Summary

**Corpus lane: GREEN** (exit 0). `BYROREDUX_REQUIRE_GAME_DATA=1 cargo test -p byroredux-nif --release --test parse_real_nifs --test per_block_baselines --test block_coverage_baselines --test oblivion_stream_drift_corpus -- --ignored oblivion no_block_sizes`:
- `parse_rate_oblivion` ok, `oblivion_block_count_parity` ok, `per_block_baseline_oblivion` ok, `no_block_sizes_drift_detector_has_zero_false_positives_on_real_corpus` ok.
- Consistent with the ROADMAP compat row (100%, 9 612 / 9 612).

**Other gates run live (all green):**
- **BSA**: `bsa_real -- --ignored oblivion` 2/2 (17 archives, zero extract errors); bsa lib 114.
- **ESM**: `parse_real_esm` ignored — `parse_rate_oblivion_esm`, `race_oblivion_data_and_subs_against_vanilla`, `clas_oblivion_knight_against_vanilla`, `oblivion_starts_dead_…_census`, `parse_rate_oblivion_pack_legacy_pkdt_and_ptdt`, `oblivion_spawn_time_global_…` 6/6; plugin lib `oblivion` filter 68 pass.
- **NIF**: full `byroredux-nif` suite (1 415 lib tests + integration) and the `--ignored` `dark_texture_role_population_matches_the_documented_census` pass.
- **Bin crate** (`byroredux --bin byroredux`): 2 829 pass, 58 ignored; the three Oblivion `--ignored` real-data tests (LTEX paths, default land textures, creature assets) pass.
- **Renderer** shader-contract guards (`dark_combine`, `pbr_bsdf`, `parallax`): 6/6.
- **M47.3 / MenuXml**: `vanilla_oblivion_quest_scripts_execute_and_setstage` 1/1; scripting lib 562; `menuxml` `vanilla_corpus` 4/4.

**Findings: 0 CRITICAL, 0 HIGH, 1 MEDIUM, 2 LOW (all NEW). No regressions.**
- **Baseline findings**: OBL-2026-10-09-D3-01 (NPC body table, HIGH) is fixed by `1b45763fa` (#5487) and OBL-2026-10-09-D2-01 (PACK `PKDT`/`PTDT`, MEDIUM) by `dae08000d` (#5504); both re-verified on real data and in code. D4-01 (#5505) and D5-01 (#5526) remain open and unchanged.
- **Headline (D3-01, MEDIUM)**: the #5487 body table stops at four parts, so Argonian and (male) Khajiit actors — 169 NPC_ bases / 145 placements in `Oblivion.esm` alone — spawn tailless. The RACE tail `MODL` is parsed and discarded, and the tail's `Bip01 Tail01..08` bones exist only in `skeletonbeast.nif`, which the humanoid spawn never selects.
- **D2-01 (LOW)**: 39% of Oblivion INFOs (42% of GREETING INFOs) gate on a CTDA function outside the 24-function catalog and evaluate against a fixed `0.0`.
- **D3-02 (LOW)**: the `humanoid_skeleton_path` doc block is welded onto `stamp_faction_ranks`, and its "Oblivion is not yet a target for NPC spawning" sentence has been false since #5487.

## Dimension Findings

### Dimension 1: NIF Version Handling & Corpus Integrity — unchanged, clean

- **Commits on the Paths since 3bcf6c8e8**:
  - `526cb07d9` (#5537): Oblivion `bhkLimitedHingeConstraint` now derives `Perp Axis In B1 = Perp B2 x Axis B` (`LimitedHingeCInfo::parse_oblivion`, `crates/nif/src/blocks/collision/constraints.rs:391`). Verified against the code and its two pins (`oblivion_limited_hinge_decodes_typed_cinfo`, `oblivion_limited_hinge_derives_perp_b1_from_perp_b2_cross_axis_b`). The corpus carries 592 LimitedHinge, 572 Ragdoll, 163 Malleable and 6 plain `bhkHingeConstraint` blocks.
  - `0c4d4af08` (#5486, B-spline resample bound), `cce3d59b5` (#5596, LE readers), `3efd17e5b` (#5470, space runs): generic; no Oblivion decode effect.
  - Nothing on `header.rs`, `version.rs`, `stream.rs`, `lib.rs`, `properties.rs`.
- **Plain `bhkHingeConstraint` (6 blocks)**: `parse_hinge_oblivion` (`constraints.rs:349`) still zeroes both B-side perpendiculars. This was noted by `NIF-D5-2026-10-12-01`'s reviewer and by the physics audit as "probably inert"; with the hinge lowered to full-range ±π limits the zero-reference cannot put a pose outside a limit, so I agree it is inert and did not file it. **Existing: NIF-D5-2026-10-12-01** (the stale "Oblivion zeroes B1" comments at `constraints.rs:348` and `import/collision/ragdoll.rs:387-394`).
- **Pins untouched**: #170 dual band, `uses_inline_block_type_names` (#4257), `NiTexturingProperty` u32 shader-map count (#149), `MORPH_LEGACY_CUTOFF` (#1509), #3926 recovery refusal.
- Deep trace of three meshes not repeated: nothing in the import path for them changed.

### Dimension 2: BSA v103 & ESM Data Slice

**Delta reviewed with no finding:**
- **`fa2af0e89` (#5073 / #5660)**: `BsaArchive::extract` decides the empty compressed placeholder before any decoder. `oblivion_all_bsas_v103_brute_force_extract_zero_errors` re-run: 17 archives, 0 errors.
- **`dae08000d` (#5504, closes baseline D2-01)**: re-censused `Oblivion.esm` with an independent GRUP walker. 7 209 PACK; `PKDT` 8 B x 6 648 + 4 B x 561; `PTDT` 12 B x 1 776; `PLDT` 12 B x 6 576. These match the commit's figures, and the parser arms (`pack.rs:832-841`, `:958`) take exactly those shapes. `package_flags` has no consumer anywhere, so the u16-vs-u32 flag read is moot.
- **`4957536ba` (#5338)** is FO3/FNV-only; **`9dbd59b8b`** is the Starfield lighting tail; **`f18224b02`** (#5300) is the Say-Once change (see D2-01 and the notes below); **`29f8fa2ef`** is test/builder consolidation.
- **Existing: LC-2026-10-12-D3-01** — Oblivion `ARMO` `BMDT` is decoded as a u32 and leaks the General Flags byte into `biped_flags` bits 16-23. I checked the one consumer chain that could turn it into a visible defect: `EquipmentSlots::equip` returns the displaced piece, but `build_npc_equip_state`'s retain pass keys on `occupants` still containing the index, and a displaced heavy piece keeps its real bits (head / upper body / foot), so nothing is lost. Latent, as filed.

#### OBL-2026-10-12-D2-01: 39% of Oblivion INFOs gate on a CTDA function outside the 24-function catalog, and evaluate it against a fixed `0.0`
- **Severity**: LOW. A documented, deliberate contract (`condition.rs:57`, "unknown function indices evaluate to `0.0`"); no crash, wrong-line selection only. Owner: `/audit-scripting` (condition catalog) — filed here because the census is Oblivion-specific and not previously measured.
- **Dimension**: 2 (DIAL/INFO selection on TES4 data).
- **Location**: `crates/scripting/src/condition.rs:1086-1092` (`ConditionFunction::Unknown` -> `0.0`), consumed by `select_info` (`crates/scripting/src/dialogue.rs:294`; the `evaluate(&info.conditions, …)` call is at `:364`).
- **Status**: NEW. No open issue tracks catalog coverage for dialogue (title search for catalog / condition / CTDA finds only #5610). Related, not duplicate: the FO3 audit's `FO3-2026-10-12-D2-01` prerequisite note, and #5380 which gave Story Manager nodes an "unknown function declines" rule that INFO conditions do not have.
- **Description**: `Oblivion.esm` has 19 278 INFOs; 7 533 (39%) carry at least one CTDA whose function index is not in the 24-function catalog. For the generic `GREETING` topic, 1 586 of 3 743 INFOs (42%) do. The top uncatalogued indices over all INFOs are 79 (3 774 conditions), 76 (1 751), 70 (841), 84 (716), 47 (687), 53 (655), 50 (540), 161 (284), 125 (270). I did not name them: the xEdit TES4 function table is not on disk, and `Raw Function List.wiki` carries no opcode numbers.
- **Evidence** (`/tmp/audit/oblivion/tools/condcmp.py`, evaluating each condition's comparison against `0.0`):

  | fn | conditions | pass with 0.0 | typical comparison |
  |---|---|---|---|
  | 76 | 1 751 | 709 | `>= 70` x256, `< 70` x215, `>= 30` x203, `< 30` x155 |
  | 79 | 3 774 | 898 | `== 1` x1 774, `== 0` x804 |
  | 84 | 716 | 622 | `== 0` x617 |
  | 50 | 540 | 508 | `== 0` x508 |
  | 70 | 841 | 20 | `== 1` x821 |

  So a line gated `fn 50 == 0` passes for every NPC and a line gated `fn 70 == 1` can never fire. The first passing INFO in priority order wins (`dialogue.rs:368-398`), so the wrong line can win.
- **Impact**: Oblivion greeting selection (every activation opens `GREETING`, #5367 Phase G) is decided partly on constants. No live dialogue gate runs Oblivion (`dt1-dialogue-layers.sh` is FNV), so nothing observes it today.
- **Related**: `FO3-2026-10-12-D2-01` (see the cross-reference below), #5380, #5406.
- **Suggested Fix**: Give `select_info` the #5380 shape — decline (or sort last) an INFO whose condition list contains an uncatalogued function — and catalogue the high-count functions (79, 76, 70, 84, 50, 47, 53) from the xEdit TES4 table, source-first, before widening the catalog. Re-run `condcensus.py` after.

### Dimension 3: Legacy-Property Rendering Path, plus the NPC spawn area

**Delta reviewed with no finding:**
- **`1b45763fa` (#5487, closes baseline D3-01)**: verified end to end. `humanoid_body_paths` returns `upperbody / lowerbody / hand / foot` (+ `female*`), each present in `Oblivion - Meshes.bsa` (listing re-taken). The biped masks `0x04 / 0x08 / 0x10 / 0x20` match Oblivion's slot order; `humanoid_body_path_icon_index` maps INDX 0..3; the RACE parser decodes the body-section `ICON` run (15 of 15 races). The per-gender section filter (`section.is_none_or(|tag| tag == want_gender_tag)`) and the seam-context override read are consistent. The `--ignored` presence pin passes.
- **`c1a99c25e` (attach_light)**: exact-name `AttachLight` lookup composed root to node (`crates/nif/src/import/attach_light.rs`); the old substring rule's false positives (`FlameNode2` and others) are gone; `__MAX_Default_Light` still never spawns (`nif_light_spawn_gate_tests.rs` green). Four unit pins cover the nested / switch / cycle cases.
- Legacy-property pins (`parallax_alpha_gate`, emissive source, alpha flag, double-sided, stencil), the dark-map census and `dark_combine_is_the_pinned_bare_multiply_in_both_paths`: all green.

#### OBL-2026-10-12-D3-01: Argonian and Khajiit actors spawn tailless — the RACE body-section tail `MODL` is parsed into `body_models` and never consumed, and the beast skeleton that carries the tail bones is never selected
- **Severity**: MEDIUM. Missing visible content on two of the ten playable races (169 NPC_ bases), with no crash and no workaround short of a code change. The same class of gap as #5487 (HIGH: handless humanoids), at smaller scale.
- **Dimension**: 3 (NPC body assembly, the area #5487 just rebuilt).
- **Location**:
  - `byroredux/src/npc_spawn.rs:624-661` (`humanoid_body_paths`: four parts, no tail arm) and `:702-718` (`humanoid_body_path_icon_index`: INDX 4 "has no mesh in the arm").
  - `byroredux/src/npc_spawn/resumable/runtime.rs:362` (`skeleton_path: humanoid_skeleton_path(game)`), `byroredux/src/npc_spawn.rs:516-519` (`GameKind::Oblivion | Fallout3NV => skeleton.nif`).
  - `crates/plugin/src/esm/records/actor/race.rs:575-581` (the `MODL` arm pushes every `MODL`, tail included, onto `body_models`).
- **Status**: NEW. Searched open issues and closed issues for tail / Argonian / Khajiit / skeletonbeast: nothing.
- **Description**: `Oblivion.esm` authors a tail mesh in the RACE body section: Argonian under both `MNAM` and `FNAM` (`Characters\Argonian\Tail.NIF`), Khajiit under `MNAM` only (`Characters\Khajiit\KhajiitTail.NIF`), each followed by an INDX 4 `ICON` (`…\tail.dds`). The archive ships both (`meshes\characters\argonian\tail.nif`, `meshes\characters\khajiit\khajiittail.nif`). #5487 decoded the INDX 4 `ICON` into `RaceRecord::body_part_textures` but deliberately wired only INDX 0..3, and the `MODL` is pushed onto `body_models`, whose only reader is the head fallback (`runtime.rs:281`). Both tail NIFs skin to `Bip01 Tail01` .. `Bip01 Tail08`; those bones exist in `skeletonbeast.nif` (8 `Tail0*` names) and not in `skeleton.nif` (0). `humanoid_skeleton_path` returns `skeleton.nif` for every Oblivion humanoid, and `skeletonbeast.nif` is referenced nowhere in `byroredux/src` (only the Skyrim doc comment mentions the name).
- **Evidence** (`/tmp/audit/oblivion/tools`, walker over `Oblivion.esm`):
  - Argonian: 68 male + 43 female bases, 59 + 33 ACHR placements. Khajiit male: 58 bases, 53 placements. Khajiit female: 37 bases / 35 placements, no authored tail (not counted).
  - Tail-bearing total: 169 NPC_ bases, 145 placements in the base game alone, before leveled spawns and the DLC.
  - `strings tail.nif | grep Tail` shows `Bip01 Tail01..08`; `strings skeleton.nif | grep -c Tail0` is 0; `skeletonbeast.nif` is 8.
- **Impact**: every Argonian and male Khajiit NPC, including the quest NPCs of both races, renders without a tail. The skeleton choice also means beast-race actors are bound to the humanoid rig rather than the beast rig the game authors them against.
- **Related**: #5487, the D3-01 baseline finding this extends; #3419 (head/body INDX split).
- **Suggested Fix**: Translate the RACE tail into the body table at the parser boundary (a `tail_model` per gender on `RaceRecord`, skipping an empty `MODL`), spawn it as a fifth body part with the INDX 4 skin, and select `skeletonbeast.nif` for races whose body section authors a tail (or via the Oblivion beast flag, `race_flags & 0x04`, which `is_child_race` currently only has to avoid). Pin it with the same archive-presence test #5487 added.

#### OBL-2026-10-12-D3-02: the `humanoid_skeleton_path` doc block sits on `stamp_faction_ranks`, and its "Oblivion is not yet a target for NPC spawning" sentence is false
- **Severity**: LOW. Doc rot; rustdoc shows a skeleton-path table on a faction-stamping fn.
- **Dimension**: 3 (NPC spawn area).
- **Location**: `byroredux/src/npc_spawn.rs:44-86` (one contiguous `///` block) and `:87` (`fn stamp_faction_ranks`); the orphaned fn is `:516` (`pub fn humanoid_skeleton_path`, no doc).
- **Status**: NEW. The same "helper inserted between a doc comment and its fn" class as `REN-D6-2026-10-12-01`, `NIFAL-D9-2026-10-12-01` and `TOOL-D1-2026-10-12-01`, but a different site (the other reports do not name this file). Introduced when `f73c6fd7b` (2026-06-24) added `stamp_faction_ranks` directly under the doc block.
- **Description**: Lines 44-77 are the skeleton path table ("Path inside the meshes archive for the default humanoid skeleton… FNV / FO3 ship a single `meshes\characters\_male\skeleton.nif`… Skyrim… FO4…"), ending "Oblivion is not yet a target for NPC spawning (M41.0 closes on FNV first); the path is the same as FNV's by convention." Line 78 starts "/// Stamp a [`FactionRanks`] component…" with no blank line, so rustdoc merges the two. Oblivion has been a spawn target since #2567 and #5487, and its humanoid arm is the busiest in `humanoid_body_paths`.
- **Impact**: documentation only.
- **Suggested Fix**: Move lines 44-77 above `humanoid_skeleton_path` at `:516`, and replace the Oblivion sentence with the real statement (`skeleton.nif`, shared with FO3/FNV; the beast rig is D3-01).

### Dimension 4: Exterior & Lighting Data (Tamriel) — reviewed, no finding

- **Guards re-run**: bin crate Oblivion exterior set (placement LOD 13, nif-light spawn gate 24, `oblivion_climate_rung_tests`, HNAM dimmer, falloff sentinel, `default_weather_rule_tests`, child water inheritance); plugin `oblivion_child_gets_the_inherit_all_stamp_and_eras_stay_real`. All green. `m-exteriors.sh oblivion static` is device-bound and was not run.
- **Placement LOD keys on the active worldspace — correct.** `EXT-D6-2026-10-12` left open whether any `distantlod\<child>_*.lod` ships. For Oblivion it does: 41 worldspaces own placement `.lod` files in `Oblivion - Meshes.bsa` (Tamriel 9 347, AnvilWorld 12, ChorrolWorld 16, BravilWorld 10, LeyawiinWorld 13, ICMarketDistrict 2, …; SI has 920 `seworld` entries), so `placement_lod_archive_path(&wctx.worldspace_key, …)` (`placement_lod.rs:208`) is right and the `lod_worldspace_key` rerouting must not be applied to it.
- **The EXT-D6-01/02/03 baked-LOD findings do not reach Oblivion.** `worldspace_lod_grid_origin` returns `(0, 0)` when `combined_lod_supported(game)` is false, and Oblivion's terrain LOD is the legacy FormID-keyed family (`meshes\landscape\lod\<decimal worldspace FormID>.<x>.<y>.32.nif`; 100 files, 36 for Tamriel `0x3C`).
- **`lod_source_worldspace`** (`env_translate/lod.rs:317`): the parse-time stamp sets the LOD-inherit bit on every Oblivion child, so rings resolve on the parent, consistent with #5505's intent. Own placement `.lod` is separate (above).
- **Delta with no Oblivion effect**: `ad280e17c` (#5344, JNAM alphas; Oblivion authors none), `a9577fd69` (#5520, FO3 residue ties), `5a549e664` (#5518).

### Dimension 5: Gameplay & UI Data Slice — reviewed, no new finding beyond the notes

- **M47.3**: `aac6b25b3` (#5629 / #5630), `291af4528` (#5655) and the quest-VM live test all pass. The 255 installed quest scripts and the 5 s cadence are unchanged.
- **MenuXml**: `5dce486b6` (#5658) budget and the 1 975-header census (`menu_texture_header_census_decodes`) green; the HUD driver commits (`627149b62` #5508) do not change Oblivion's all-full bars (no AVIF records, ROADMAP Status). The `HudBar` doc rot is open as #5033; the `menus80` / `menus50` census gap is open as #5669.
- **Cross-reference (Existing: FO3-2026-10-12-D2-01 — reach into Oblivion, not a new finding)**: quest-level `CTDA` lists are parsed into `QuestRecord::dialogue_conditions` (`quest.rs:363`, `:1139`) and never read. In `Oblivion.esm` 339 of 390 QUST carry one: `fn 254 == 1` on 308 quests, `fn 365 == 0` on 290 non-SE quests and `fn 365 == 1` on the 7 SE quests (the Shivering Isles split; consistent with `GetPlayerInSEWorld`, not confirmable offline). Two consequences for whoever fixes the FO3 finding:
  1. **Do not wire evaluation before cataloguing fn 254 and fn 365.** Both are uncatalogued, so both evaluate `0.0`; `fn 254 == 1` would then fail 308 quests and drop their dialogue wholesale (the FO3 report's "coupled prerequisite", at Oblivion scale).
  2. **The leak surface today is small.** All 163 GREETING INFOs from quests with `fn 365 == 1` carry an identity condition (`GetIsID` / `GetInFaction`), so they do not leak into Tamriel; 22 identity-less GREETING INFOs from Tamriel quests (`Generic` 14, `MQEndgame` 4, `NQDBruma` 3, `GenericDremora` 1) can be said by SE actors.

## Notes, not filed

- **Say Once is applied per speaker reference on Oblivion** (`f18224b02` / #5300, `DialogueSpokenInfoForms` as `(speaker, INFO)` pairs). The local sources disagree about Oblivion: `geck-uesp-wiki` (FO3/FNV) says "only said once by any given actor", while `cs-uesp-wiki` `Editing Dialogue` says "this info will only be played once. The player will never hear it again". `Oblivion.esm` has 1 162 Say-Once INFOs, 97 of them with no `GetIsID`. Engine behaviour is not established by either page, so no value is guessed here; `AUDIT_FNV_2026-10-12` accepted the per-actor rule for FNV on the GECK text. Worth a source if Oblivion dialogue is gated live.
- **Existing: GAME-D5-2026-10-12-01** (`CreatureActor` never stamped, #5495 gate dead): with #5504 decoding Oblivion Eat / Sleep packages, Oblivion creatures with those packages (128 CREA bases carried a misread package) reach the humanoid seat paths without the gate. Unchanged from the baseline's FNV-D5-01 cross-reference.
- **Existing: #5664** (`PAR-D6-2026-10-12-02`, tagged `game:oblivion`): reused archive handles skip the #3637 shadow count.
- **Existing**: #5505 (child worldspaces draw no parent LAND), #5526 (Phase V voice), #5351, #5439, #5011, #5033, #5669. All still open, unchanged.

## Regression Guard List (all green this run)

- Stride-drift family (#1506-#1509): corpus lane, `oblivion_stream_drift_corpus`.
- #170 dual-band `BSStreamHeader`; `NiTexturingProperty` u32 shader-map count (#149); `MORPH_LEGACY_CUTOFF` (#1509); #3926 recovery refusal.
- BSA v103 17-archive sweep (#699) and the empty-placeholder edge (#5073 / #5660).
- 16-byte ACBS (#1650), 8-byte `LVLO` (#4638), EFID code map (#5084 / #5076), `Starts Dead` (#5013), legacy `PKDT` / `PTDT` (#5504).
- `parallax_alpha_gate_tests`; the Disney gate (`MAT_FLAG_PBR_BSDF` stays 0 for Oblivion); `dark_combine` pin.
- #5487 body table (`body_paths_kf_era_match_each_games_archive`, `installed_oblivion_body_paths_exist_in_the_meshes_archive`).
- `__MAX_Default_Light` spawn gate (#5189); inherit-all stamp (#5516); distant water (#5335); climate rungs (#5388).
- `vanilla_oblivion_quest_scripts_execute_and_setstage`; `menu_texture_header_census_decodes`.

## Open Work

From `ROADMAP.md` and this run:
- Argonian / Khajiit tail and beast rig (D3-01).
- Oblivion dialogue: catalogue the high-count INFO functions (D2-01) and the two quest-level functions (254, 365) before quest-level conditions are evaluated.
- #5505 (child LAND / terrain LOD), #5526 (voice), the Oblivion CHARAL ruleset unwiring (no AVIF records).
- Unchanged: interiors and the Tamriel exterior render; neither is a blocker.

## Scratch reconciliation

`/tmp/audit/oblivion/dim_1.md` through `dim_5.md` were checked against this report. Every finding in them is above (D2-01, D3-01, D3-02); the Say-Once item, the FO3 cross-reference and the Dim 1 hinge note appear under Notes / Dimension sections, and nothing was dropped.

## Suggested Next Step

```
/audit-publish docs/audits/AUDIT_OBLIVION_2026-10-12.md
```
Labels: every finding `game:oblivion` + `legacy-compat`, plus its own domain — D3-01 `bug` `medium` `ai` `import-pipeline`; D2-01 `bug` `low` `dialogue` `scripting`; D3-02 `documentation` `low` `doc-rot`.
