# Skyrim (SE + LE) Compatibility Audit — 2026-10-12

**HEAD**: 5d31fd637 · **Baseline**: `docs/audits/AUDIT_SKYRIM_2026-10-09.md` (HEAD `3bcf6c8e8`, 264 commits ago) · **Audited**: Dim 3 (NPC equip + FaceGen, plus the Skyrim dialogue data it reaches), Dim 4 (load order + TES5 cell load, plus the Skyrim-side verification of the exterior audit's LOD findings), Dim 5 (archives + corpus, gates re-run) · **Unchanged since baseline (skimmed)**: Dim 1 (only `656e2bdc3` doc/log/test and `cce3d59b5` byte-helper refactor), Dim 2 (only a docs commit and a whitespace commit touch its paths).

Run context:
- One leg of `/audit-suite --preset comprehensive`, run solo with no sub-agents. Scratch notes are `/tmp/audit/skyrim/dim_{1..5}.md`.
- **No engine launch** (the user's FNV instance is running). The Whiterun control bench, the DLC repro, door transitions and the live render trace were not re-driven. The control bench is INCOMPLETE, not FAILED.
- **Private cargo target**: the probe crate `/mnt/data/tmp/skyrim-audit-probe` builds into its own `target/` (path deps on the repo crates, repo `Cargo.lock` copied in). It holds three read-only binaries: an `AttachLight` / shader-type / multi-parent NIF census (`src/main.rs`), a real-engine Hello-greeting probe (`src/bin/greet.rs`) and an ARMA mesh presence census (`src/bin/armamiss.rs`). Python byte-walk census scripts are in `/tmp/audit/skyrim/`.
- **`--ignored` plugin tests ran only by exact name** (about 1 GB RSS each, one at a time).

---

## Executive Summary

**Parse, archive and skinning layers are green at HEAD:**
- **NIF corpus (release, `parse_rate_skyrim_se` / `_le`):** SE 33,468 / 33,468 clean across 8 archives; LE 22,466 / 22,466. No truncated, failed or recovered blocks.
- **Archive sweep:** every file of the 11 SE base archives extracts: 65,637 files, 0 errors, 0 magic mismatches.
- **SSE packed bone indices:** 19,606 weighted lanes, 0 changed by the importer.
- **Unit runs:**
  - nif `--lib` (skyrim / bs_tri_shape / sse / tangent / bs_lod / alpha_flag / shader): 382 passed.
  - plugin `--lib` (bodt / arma / equip / wnam / force_greet / dialogue_override): 88 passed.
  - bin crate (rustc 1.96): `npc_spawn|equip|skin|lod_support|material_translate` 283 passed; `load_order|lod|attach_light|story` 209 passed; `siblings|archive` 49 passed.
  - bsa and renderer `dds` (76) pass; scripting `--ignored story_manager` (3, incl. the every-node-reachable floor after #5405) passes.
- **Exact-name real-data tests:** `force_greet_skyrim_tree_dialect_floor`, `skyrim_npc_wnam_skin_census_matches_the_byte_walk`, `skyrim_default_water_promotes_underwater_tail`, `skyrim_ltex_grass_links_resolve_to_real_gras_records`, `ms01_eltrys_authored_placement`, `parse_real_skyrim_esm`, `skyrim_bodt_body_templates_decode_their_biped_masks`, `parse_rate_skyrim_info_response_flags` all pass.

**Baseline's open items, re-checked:**
- Both baseline MEDIUMs are fixed and verified on data. #5510 (`d678dc578`): one decision site (`equip.rs` `pick_path`), 172 of 766 ARMAs are male-only. #5511 (`72e863d80`): the Skyrim.esm force-greet census now reads 339 / 334 / 320 through the real `force_greet`.
- #5408 (race base head parts), #5015 (XRGD), #4913 (`.btt` trees), #4256 (`shader_type`) are unchanged and still open.
- #5405 is fixed by `76a927f8a`; the reachability floor passes.

**Findings: 2 NEW (0 CRITICAL, 0 HIGH, 1 MEDIUM, 1 LOW).**
- **SKY-D4-2026-10-12-01 (MEDIUM).** Skyrim's generic `DialogueGenericHello` is the quest-less-patron greeting, and 57 of its 60 INFOs are gated on `GetIsVoiceType` (CTDA fn 426), which is outside the 24-function catalog and evaluates to 0. Run through the engine's own `select_first_info`, all 5,118 non-creature `NPC_` bases receive one of the same three unconditioned lines. Nothing voice-, race- or sex-specific can ever be selected.
- **SKY-D4-2026-10-12-02 (LOW).** ROADMAP's M43 row still records the pre-#5511 Skyrim force-greet census.

**Skyrim-side verification of the exterior audit's LOD findings (cited, not re-reported):** EXT-D6-2026-10-12-01 is confirmed on Skyrim data, including LE (see Cross-Audit Verification).

**Totals:** 2 NEW (0C / 0H / 1M / 1L) · 0 regressions · matched-existing (open): #4256, #5015, #4913, #5408, #5462, #5463, #5674, plus suite IDs OBL-2026-10-12-D2-01 (mechanism), EXT-D6-2026-10-12-01/02/03, GAME-D1-2026-10-12-01, FO3-2026-10-12-D2-01, NIF-D1-2026-10-12-01.

---

## Dimension Findings

### Dimension 1 — BSTriShape Packed Geometry + SSE Skinned Reconstruction (unchanged)

First step: `git log 3bcf6c8e8..HEAD -- crates/nif/src/blocks/tri_shape crates/nif/src/import/mesh crates/nif/src/blocks/mod.rs` shows two commits, neither functional:
- `656e2bdc3` (#5524) restores `widen_packed_bone_indices`' doc, makes the decline log arm-neutral and adds `BsSkinInstance` boundary tests.
- `cce3d59b5` (#5596) moves little-endian readers into `byroredux_core::bytes` (`sse_recon.rs`, `tangent.rs`).

Guard spot-check: the nif `--lib` subset (382) and the real-data `packed_sse_indices_match_partition_palette_expansion_on_real_data` (19,606 lanes, 0 changed) pass.

**#5485 revisit guard on Skyrim** (`cf8730adc`, outside the Dim 1 paths but it touches every walker): the commit's premise "vanilla has zero multi-parent / self-referenced children" holds on Skyrim. Over 22,047 SE and 17,216 LE NIFs the census counts 0 multi-parent and 0 self-ref blocks, so the guard cannot drop legitimately shared subtrees there.

No findings.

### Dimension 2 — Shader-Type Dispatch + Skyrim Material Slice (unchanged)

First step: `git log 3bcf6c8e8..HEAD -- crates/nif/src/blocks/shader crates/nif/src/import/material byroredux/src/material_translate.rs` shows `214e63e41` (docs) and `3efd17e5b` (whitespace). `0d698d16c` (alpha test needs authored `HasOpacity`) is the BGSM merge path, which vanilla Skyrim never takes.

**Shader-type census over the shipped meshes** (new this run, feeds the matrix below):

| `shader_type` | `ShaderTypeData` arm | SE blocks | LE blocks |
|---|---|---:|---:|
| 0 | `None` | 45,458 | 36,963 |
| 1 | `EnvironmentMap` | 6,726 | 3,176 |
| 2 | `None` (Glow) | 1,395 | 729 |
| 3 | `None` (Parallax) | 11 | 14 |
| 4 | `None` (Face Tint) | 3,158 | 2,554 |
| 5 | `SkinTint` | 1,631 | 1,423 |
| 6 | `HairTint` | 10,817 | 8,748 |
| 7 | `ParallaxOcc` | 0 | 12 |
| 11 | `MultiLayerParallax` | 662 | 502 |
| 14 | `SparkleSnow` | 19 | 19 |
| 16 | `EyeEnvmap` | 3,251 | 2,628 |
| total | | 73,128 | 56,768 |

Types 8–10, 12–13, 15 and 17–20 never occur in a shipped NIF (landscape, LOD and cloud kinds are runtime-assigned). Every occurring type lands in the arm `parse_shader_type_data` documents, and the 100% clean corpus shows none over-reads.

No findings. Cited: NIF-D1-2026-10-12-01 (`opaque_tail_len` reported on every game), #4256.

### Dimension 3 — NPC Equip + FaceGen (M41) and the Skyrim Dialogue Data It Reaches

First step: `git log 3bcf6c8e8..HEAD -- byroredux/src/npc_spawn crates/plugin/src/equip.rs …` shows 19 commits. Each was reviewed for Skyrim reach:

- **#5510 (`d678dc578`)** — verified. `pick_path` in `crates/plugin/src/equip.rs` is the only reader of `female_biped_model` outside the parser, so both resolution passes inherit the male fallback; `GameKind::Skyrim`-gated; FO4+ keeps the strict reading. Pinned by `skyrim_female_wearer_falls_back_to_the_male_biped_model`. Census at HEAD: 766 ARMAs, 172 male-only, and 1 of 1,356 authored model paths is absent from `Skyrim - Meshes0/1.bsa` (`AlduinMouthFireSkin.nif`, ARMA `0x00109491`).
- **#5511 (`72e863d80`)** — verified end to end through the real `PackRecord::force_greet` with the `PKCU` template resolved (339 force-greet packs, 320 with their own topic, ≥ 17 generic).
- **#5312 (`2f1a76e08`)** — the shared `spawn_armor_unit` / `spawn_skeleton_unit` are arm-for-arm equivalent to the two removed copies, so the pre-baked Skyrim arm is unchanged.
- **#5157 (`75aff29f4`)** — head-mounted hair alignment lives in `spawn_shared_skeleton_part`, the FO3/FNV runtime-head path. The Skyrim pre-baked arm does not call it.
- **#5031 (`ec41e33e8`)** — GAME-D1-2026-10-12-01 (the exclude set holds only the latest failed form) has no vanilla-Skyrim reach: only one Skyrim.esm ARMA mesh is absent from the archives, so no actor accumulates two failing forms.
- **#5300 (`f18224b02`, INFO behaviour flags)** — a Skyrim data decode, so checked against the source. The UESP INFO table gives `ENAM` = `Flags u16, ResetTime u16` and the legacy 8-byte `DATA` = `Tab u16, Flags u16, Days f32`, with bit 0 Goodbye, 1 Random, 2 Say once, 5 Random end. The decode matches. On `Skyrim.esm`, all 31,465 INFOs now carry flags (30,541 `ENAM` + 924 legacy `DATA`), 1,116 are Say-Once, 3,321 are Goodbye. 63 of the 1,413 Say-Once INFOs across the five Skyrim plugins also author a non-zero reset time, which no code consumes. The reset semantics are not sourced locally (the CK says a Say-Once line is never said again by that actor), so this is recorded as an observation and not filed.
- **#5408** (race base head parts, prebaked head fallback) — still open and unchanged (`prebaked.rs` not in the window).

The dialogue data the Skyrim spawn path reaches produced the one MEDIUM finding of this run (filed under Dim 4 because it is a TES5 data-through-mechanism defect): see SKY-D4-2026-10-12-01.

### Dimension 4 — Multi-Master Load Order + TES5 Cell Load (+ Skyrim dialogue data)

First step: `git log 3bcf6c8e8..HEAD -- byroredux/src/cell_loader/load_order.rs crates/plugin/src/esm/reader.rs crates/plugin/src/esm/cell crates/plugin/src/esm/records/index.rs` shows 8 commits: `5356c8780` (#5494), `335a0179c` (#5428), `65cadde2f` (#5516/5517/5519), `777351ec1` (#5484), plus layout, cite and whitespace commits.

- **#5494 (INFO head-run fold) is a no-op on Skyrim.** A census of every override DIAL in `Update.esm`, `Dawnguard.esm`, `HearthFires.esm` and `Dragonborn.esm` that overrides a `Skyrim.esm` topic finds no topic with two or more `PNAM == 0` INFO heads (46 / 12 / 58 / 7 head INFOs in 58 / 18 / 85 / 36 overridden topics, at most one head each). The reversed-run bug it fixes cannot occur on vanilla Skyrim data.
- **#5428 / #5516-5519 / #5484** are gated on Oblivion, SCPT or FO4 precombine data and do not touch the TES5 path.
- **Multi-master:** `parallel_walk_matches_the_inline_walk_on_a_localized_master_chain` and the `load_order` tests pass. The `Dawnguard.esm` / `Forelhost01` repro was not re-driven (no engine launch).
- **TES5 walk:** `parse_real_skyrim_esm` and `ms01_eltrys_authored_placement` (DIAL `QNAM` ownership) pass.
- **Story Manager** (Skyrim-hosted, owners `/audit-gameplay` and `/audit-scripting`): `76a927f8a` (#5404 consume-only-on-fire, #5405 every sibling placed) is covered by the three ignored scripting tests above, all green.
- **Control bench:** INCOMPLETE (no engine launch). Reference is ROADMAP's Bench-of-record.
- **Open Skyrim-hosted gate:** #5674 — `p5-soak` on the default game `skyrim_se` runs an unmeasured route because `skyrim_se.env` declares no `FIXTURE_GATES`.

#### SKY-D4-2026-10-12-01: Skyrim's generic Hello greeting can only ever select its three unconditioned lines, because 57 of its 60 INFOs are gated on `GetIsVoiceType` (fn 426), which is outside the 24-function condition catalog
- **Severity**: MEDIUM. Incorrect, silently degraded behaviour with no crash; it affects the Phase G fallback every quest-less Skyrim NPC takes.
- **Dimension**: 4 — TES5 data through the dialogue selector (mechanism owners `/audit-scripting` and `/audit-gameplay`).
- **Location**:
  - `crates/scripting/src/condition.rs:57-59` and `:1086` (`ConditionFunction::Unknown` evaluates to 0.0; the catalog table is `:22-56`).
  - `crates/scripting/src/dialogue.rs:293-370` (`select_info`).
  - `byroredux/src/systems/npc_dialogue.rs:291-305` (`generic_greeting_record`) and `:328-336` (`open_conversation` fallback).
- **Status**: NEW (the Skyrim leg of the mechanism OBL-2026-10-12-D2-01 files for Oblivion: 7,533 of 19,278 Oblivion INFOs gate on uncatalogued functions; rated MEDIUM here because the measured outcome is a total collapse of the fallback topic's selection, not a partial gap). Searched open issues and the closed set ("GetIsVoiceType", "uncatalogued condition function", "generic greeting", "Hello"); nothing covers it. Related but not covering: #5526 (Oblivion voice, `VTCK`), FO3-2026-10-12-D2-01 (quest-level conditions ignored; its suggested fix already names the same prerequisite for fn 427 on FO3/FNV), #5380 (decline on uncatalogued condition functions, Story Manager only).
- **Description**:
  - `DialogueGenericHello` (`0x000142B5`, owner quest `DialogueGeneric`) holds 60 INFOs. All 60 are Random-flagged and `previous_info == 0`.
  - 57 of them carry CTDA conditions, and every one of those includes `GetIsVoiceType` (fn 426, 124 conditions in the topic) against a `VTYP` or a voice-type `FLST` (`DefaultNPCVoiceTypes`, `GuardVoiceTypes`, `VoicesUnique`, `FemaleNord`). Some add fn 69 / 70 / 74 / 130 / 131 terms (6 / 6 / 6 / 15 / 4 conditions); of those only 69 and 74 are catalogued.
  - Function 426 is not in the catalog, so `evaluate` maps it to 0.0 and `fn426 == 1` can never hold. The 57 gated INFOs never pass. Only the three INFOs with no CTDA (`0x00087940`, `0x000142C0`, `0x00087947`) survive.
  - Scope of the gap beyond this topic: on `Skyrim.esm`, 13,154 of 31,465 INFOs reference at least one uncatalogued function, and fn 426 alone appears in 4,222 INFOs (the most common uncatalogued functions are fn 566 in 4,388 INFOs, fn 426 in 4,222 and fn 629 in 2,626). The Hello topic is the one place where the gap is the whole selection.
- **Evidence** (the engine's own `select_first_info` over the shipped master; probe `/mnt/data/tmp/skyrim-audit-probe/src/bin/greet.rs`, which parses `Skyrim.esm`, installs `DialogueQuestPriorities` and the 330 Start-Game-Enabled quests, then selects the Hello topic for every non-creature `NPC_` base):
  ```
  started 330; greeting 000142B5 DialogueGenericHello infos=60
  NPC_ bases (non-creature) 5118; no passing INFO 0
    1738 00087947  ctda=0
    1702 000142C0  ctda=0
    1678 00087940  ctda=0
  hello INFO condition fn histogram: {69: 6, 70: 6, 74: 6, 130: 15, 131: 4, 426: 124}
  ```
  All 5,118 bases split across the three unconditioned INFOs, so a female Nord, an Orc guard and a child receive the same three lines. The skill's earlier claim that the Phase G fallback "selects" for Skyrim is true only in this degenerate sense.
- **Impact**:
  - Every Skyrim NPC that takes the quest-less fallback speaks one of three voice-agnostic lines. The 57 voice-, race- and sex-specific Hello lines, which are most of the topic's content, are unreachable.
  - Skyrim `.fuz` voice is V2 scope, so the visible effect today is the subtitle text. Once voice lands, the selected INFO decides which recording plays, and a voice-agnostic line cannot resolve a per-voice-type file.
  - The existing guards cannot see it: `dt2-skyrim-forcegreet.sh` gates the force-greet topic and the activation-less open (`DialogueGenericHello` opens), not which INFO wins; there is no Skyrim equivalent of the FO3/FNV `dialogue_greeting_and_forcegreet_*_floor` winner assertion.
- **Related**: OBL-2026-10-12-D2-01 (same mechanism, Oblivion), FO3-2026-10-12-D2-01 (the quest-level condition list ignored; its Oblivion cross-ref makes the same catalogue-first point). The `DialogueGeneric` quest's own dialogue conditions are fn 700 (no parameter) and fn 426 (`DefaultNPCVoiceTypes`), so the fix for that finding must catalogue 426 and 700 on Skyrim first, or it would decline the whole generic quest (fail-closed) instead of improving the greeting. Skyrim reach of that finding is otherwise bounded: a topic has a single owner quest, so priority ordering cannot pre-empt, and the engine's alias-owned topic path only reaches quests with live alias bindings, and only 132 of the 1,811 Skyrim quests author dialogue conditions at all.
- **Suggested Fix**:
  1. Catalogue `GetIsVoiceType` (Skyrim 426: parameter is a `VTYP` or `FLST`, true when the subject's `NPC_` `VTCK` voice type is in it) next to `GetIsRace`; the voice-type lookup already exists for the dialogue voice resolver (`VTCK` → indexed `VTYP`).
  2. Add a Skyrim real-data assertion beside `force_greet_skyrim_tree_dialect_floor`: a bare speaker and a named speaker must not both resolve to the same three INFOs, and the winning INFO for a known voice type must carry that voice type in its CTDA.

### Dimension 5 — Archives + Corpus Gates

First step (`parse_rate_skyrim_se` / `parse_rate_skyrim_le`, release) is green: SE 33,468 / 33,468, LE 22,466 / 22,466, no truncated, failed or recovered blocks. Delta commits all verified:
- `fa2af0e89` (#5660 / #5073, empty compressed placeholders extract as empty files): the SE sweep extracts all 65,637 files across the 11 base archives with 0 errors. The sibling-load `siblings` tests (49 with the other archive tests) pass.
- `5402b2530` (#5288, shared opened archive handles) and `9b19aa815` (#5402, DDS luminance): no effect on the Skyrim path. `parse_uncompressed_x8r8g8b8_expands_channels_and_forces_opaque` and `parse_uncompressed_a8r8g8b8_zero_copies_as_bgra` still pass inside the 76 renderer `dds` tests.
- `cce3d59b5` (#5596): the DDS header read now errors instead of panicking; covered by the same tests.

PAR-D6-2026-10-12-01 / #5663 (`open_shared` stamp) is the suite's archive-handle residual; no Skyrim-specific reach beyond the shared provider.

No findings.

---

## Cross-Audit Verification (Skyrim-side evidence)

### EXT-D6-2026-10-12-01 (HIGH, exterior audit) — confirmed on Skyrim SE and LE data

An independent re-derivation, not a copy of the exterior audit's census:

| Fact | SE | LE |
|---|---|---|
| Tamriel WRLD `NAM0` min → engine origin | (-57, -43) | (-57, -43) |
| `lodsettings\tamriel.lod` (16 bytes) | `a0ff a0ff 0001 0000 0400 0000 2000 0000` → SW cell (-96, -96) | byte-identical |
| Baked LOD files present for Tamriel | `Skyrim - Meshes1.bsa` (`tamriel.<L>.<x>.<y>.btr/.bto`) | `Skyrim - Meshes.bsa` (same names; 36 `tamriel.32.*` files listed) |

Simulation of the engine's quadtree, `quad_origin(g, level, origin)`, against the SE name table (2,304 / 576 / 144 / 36 `.btr` files at levels 4 / 8 / 16 / 32, and 517 / 152 / 48 `.bto` at 4 / 8 / 16):

| Origin | `.btr` cell → file hits, every level | `.bto` cell → file hits |
|---|---|---|
| NAM0 min (-57, -43), current code | **0 / 36,864** at each of the four levels | **0** at each level |
| `.lod` SW (-96, -96) | 36,864 / 36,864 at each of the four levels | 8,272 / 23,616, 9,728 / 25,536, 12,288 / 28,160 (the rest is open ocean the archive correctly omits) |

Notes for the exterior owner:
- **LE is affected too** (the exterior report said "likely"): the same game kind, the same `.lod` and the same file names. LE ships its baked LOD in `Skyrim - Meshes.bsa`.
- The mechanism of the regression is visible in the original issue: #2586's own text says "Tamriel and Solstheim work" at absolute alignment, because (-96, -96) is a multiple of 4 / 8 / 16 / 32. Switching to the `NAM0` minimum moved Tamriel off that lattice while fixing the DLC worlds.
- **Child worldspaces (EXT-D6-02 / -03).** Whiterun, Windhelm, Riften, Solitude, Whiterun Dragonsreach, Katariah and the Windhelm Pit have zero baked LOD files of their own and no `lodsettings` entry; they inherit Tamriel's. Markarth is the exception: 286 own files and its own `lodsettings\markarthworld.lod`.
- **Documentation consequence.** `ROADMAP.md:215` (Skyrim SE compat row: "Exterior Tamriel with `.btr`/`.bto` LOD") describes behaviour that, per the above, holds only up to #2586's fix. It should be corrected with EXT-D6-01, not separately.

### Other suite items with Skyrim reach
- GAME-D1-2026-10-12-01 (#5031 incomplete): no vanilla-Skyrim reach (Dim 3).
- FO3-2026-10-12-D2-01 (quest-level dialogue conditions): bounded on Skyrim; see SKY-D4-2026-10-12-01 for the coupling.
- NIF-D1-2026-10-12-01 (`opaque_tail_len`): Skyrim shader blocks included; no Skyrim-specific fault.
- Census note: the `attach_light.rs` module doc says 74 Skyrim SE NIFs author an `AttachLight` node. The importer resolves an offset for 84 SE files (74 plain `NiNode` + 10 node subclasses; 71 in LE), never more than one per file and no non-identity root. The doc figure counts plain nodes only, so this is not a defect.

---

## SKY-D4-2026-10-12-02: ROADMAP's M43 row still records the pre-#5511 Skyrim force-greet census
- **Severity**: LOW
- **Dimension**: 4 — documentation rot (the data was re-censused by `72e863d80`).
- **Location**: `ROADMAP.md:289` (M43 "Quests & dialogue" row, the 2026-10-08 force-greet clause).
- **Status**: NEW.
- **Description**: the row says the Skyrim `ForceGreet` leaf has "census 5 packs / 2 authored topics / 3 generic". `72e863d80` (#5511) found that 334 of the 339 force-greet packs are `PKCU` template instances the old count could not see: the corrected census is 339 packs (5 own-tree + 334 instances), 320 instances with their own topic, ≥ 17 generic. `docs/engine/dialogue-trees.md` and the real-data floor were updated in that commit; ROADMAP, the suite's status floor, was not.
- **Evidence**: `crates/plugin/tests/parse_real_esm.rs` `force_greet_skyrim_tree_dialect_floor` (passes: ≥ 339 / ≥ 17 / ≥ 320) against `ROADMAP.md:289`.
- **Impact**: the status document reports 5 of the 339 force-greet packs the runtime now recognises.
- **Related**: #5511, `docs/engine/dialogue-trees.md`.
- **Suggested Fix**: replace the clause with the #5511 census and point at the floor test.

---

## Shader-Type Coverage Matrix

Parse and import are unchanged since 2026-09-22. The counts below are measured over the shipped SE / LE meshes at this run.

| Variant | Numeric type(s) (SE / LE blocks) | Parse | Import (→ `ImportedMesh.material`) | Render |
|---|---|---|---|---|
| `None` (no trailing data) | 0 (45,458 / 36,963), 2 (1,395 / 729), 3 (11 / 14), 4 (3,158 / 2,554); 8–10, 12, 13, 15, 17–20 never authored in NIFs | Complete (0 bytes; #4252 pin) | N/A (kind carried as `material_kind`) | Default lit; kinds 2..=20 protected from the glass classifier |
| `EnvironmentMap` | 1 (6,726 / 3,176) | Complete | Complete (`env_map_scale`) | Consumed; alchemy glass → glass (#4392) |
| `SkinTint` | 5 (1,631 / 1,423) | Complete | Complete | Consumed; #4423 tint alpha weight |
| `HairTint` | 6 (10,817 / 8,748) | Complete | Complete | Consumed |
| `ParallaxOcc` | 7 (0 / 12) | Complete | Complete (`parallax_max_passes` / `_height_scale` flow to `Material`) | Consumed |
| `MultiLayerParallax` | 11 (662 / 502) | Complete | Complete | Consumed |
| `SparkleSnow` | 14 (19 / 19) | Complete | Complete | Consumed |
| `EyeEnvmap` | 16 (3,251 / 2,628) | Complete | Complete | Consumed |
| FaceTint (no payload) | 4 (see `None` row) | Complete | `material_kind = 4` keys the FaceGen tint override (#4421) | Consumed |
| `Fo76SkinTint` etc. | FO76 table | N/A for Skyrim (`parse_shader_type_data_fo76`) | — | — |

Residual: #4256 (`shader_type` is still not canonical). SE has no type 7 block; the `ParallaxOcc` arm is exercised only by LE meshes and the unit tests.

---

## Cell-Load Regression Status

- **TES5 walk:** `parse_real_skyrim_esm` passes; `ms01_eltrys_authored_placement` (DIAL `QNAM` ownership through the merged index) passes. No window commit touches STAT / REFR / LIGH resolution or the LAND scale. The walker commits in the window are gated off TES5 or byte-neutral.
- **Multi-master:** the #3813 ordered fold is unchanged and its guard passes. DLC override INFO folding (#5375 / #5494) is exercised by 58 / 18 / 85 / 36 overridden topics across the four Skyrim plugins, none of which has more than one `PNAM`-less head.
- **Distant LOD:** Tamriel baked LOD selection is broken at HEAD (EXT-D6-2026-10-12-01, confirmed on SE and LE data above). Child worldspaces and `.btt` trees: EXT-D6-02/-03, #4913.
- **Whiterun BanneredMare control bench:** INCOMPLETE, not FAILED, because there was no engine launch. The reference is ROADMAP's Bench-of-record. `/audit-runtime` owns the live baseline.
- **Smoke gates:** #5674 (open) — `p5-soak` on `skyrim_se` is unmeasured.

---

## Totals

2 NEW (0 CRITICAL, 0 HIGH, 1 MEDIUM, 1 LOW) · 0 regressions · Skyrim-side confirmation of EXT-D6-2026-10-12-01 (HIGH, filed by `/audit-exterior`) · Matched-existing (open): #4256, #4913, #5015, #5408, #5462, #5463, #5674; suite IDs OBL-2026-10-12-D2-01 (mechanism), EXT-D6-2026-10-12-01/02/03, GAME-D1-2026-10-12-01, FO3-2026-10-12-D2-01, NIF-D1-2026-10-12-01.

Suggest: `/audit-publish docs/audits/AUDIT_SKYRIM_2026-10-12.md`
(label every finding `game:skyrim` + `legacy-compat`, plus its own domain label: SKY-D4-01 `dialogue` `scripting` `medium` `bug`; SKY-D4-02 `doc-rot` `low` `documentation`.)
