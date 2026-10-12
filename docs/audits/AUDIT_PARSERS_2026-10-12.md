# Parser Discipline Audit: 2026-10-12

**HEAD**: c1a99c25e · **Baseline**: `AUDIT_PARSERS_2026-10-08.md` (HEAD 00f580e09) · **Audited**: Dims 1–6. Every dimension's Paths had commits since 00f580e09:
- menuxml: #5377, #5399, #5400.
- sfmaterial: #5277/#5283, #5502 (partial), #5503, #5578.
- Real-data gates: #5326.
- Consumers: #5288 (shared archive registry) and #5291 (background CDB index build).
- Text sweep: #5470.
- Clippy: #5477.
- The 2026-10-10 dependency refresh: `317d4b7a9` (flate2 1.1.9 → 1.1.10) and `20ad630a6` (lz4_flex 0.11 → 0.14).

**Unchanged since baseline (skimmed)**: `crates/bsa/src/` (doc and log-string edits only), `crates/bgsm/src/`, `crates/facegen/src/`, `crates/hkx/src/` outside the census test, and `crates/game-detect/src/`. Their guards were spot-checked by running the suites.

**Method**: three sub-agents, each read-only on the tree with a pinned target dir and no engine launch:
- **A**: menuxml decoder and includes.
- **B**: archive registry and CDB worker.
- **C**: CDB index and HKX rejects.

The orchestrator ran the unit and strict real-data lanes, the archive and ESM decompression sweeps, and the remaining dimensions, then re-verified every agent finding against the code. The scratch notes and probe crates (`probe-{a,b,c,o}`, named in the evidence below) were removed at cleanup. Each finding quotes the probe output it relies on, so the report stands on its own.

## Executive Summary

**NEW findings: 0 CRITICAL · 2 HIGH · 3 MEDIUM · 11 LOW.**

**HIGH**
- **The menu texture cache has no total bound (PAR-D1-2026-10-12-01).**
  - Each distinct `<filename>` pins up to 8192² RGBA (256 MiB) for the HUD's lifetime, and nothing caps the total.
  - Measured: about 48 KB of compressed archive entry pins 256 MiB, so about 6 MB of mod archive aborts a 32 GiB machine at HUD load.
- **The HKX `mask_size` gate rejects 229 vanilla Skyrim SE clips, and 167 on LE (PAR-D1-2026-10-12-02).**
  - Havok pads the field to 4 bytes, but the gate demands the unpadded value.
  - This is the undiagnosed bulk of the 288-clip baseline that #5326 pinned. It covers all 117 Falmer clips, about 30 animated traps and dungeon objects, and a few human, Draugr and killmove clips.
  - With the gate relaxed, all 229 decode byte-exactly.
  - No live consumer is affected today.

**MEDIUM**
- **The 2026-10-10 lockfile refresh made #5073 live and turned the strict BSA lane red (PAR-D2-2026-10-12-01).**
  - flate2 1.1.10 returns `Err(UnexpectedEof)` on an empty zlib stream, where 1.1.9 returned `Ok`.
  - 37 vanilla `.txt` placeholders now hard-fail extraction. `oblivion_all_bsas_v103_brute_force_extract_zero_errors` fails at HEAD.
  - The CI job has no `--no-fail-fast`, so the red binary also stops `csg_real` from running.
- **5c14ebdf8's Parent walk changed what `has_opacity == None` means (PAR-D5-2026-10-12-01).**
  - The merge still reads `None` as "opacity present", so 1,265 of 10,152 referenced Starfield materials are alpha-tested against colour alpha.
  - 426 of them were newly flipped by 5c14ebdf8, and the commit does not mention it.
  - The `HasOpacity` default of `false` is inferred from the census, not sourced, so it needs confirming first.
- **One NaN prop-bone channel rejects a whole HKX clip (PAR-D2-2026-10-12-02).**
  - Affects 59 SE and 36 LE clips, mostly killmoves and DLC01 crossbow clips.
  - The NaNs are really in the files, and every other track decodes.

**Already tracked and still live:**
- #5009, #5010 and #5011 are unchanged in code.
- #5073 is now live; see PAR-D2-2026-10-12-01.
- #5329 is unchanged.
- #5446 is unchanged. PAR-D1-2026-10-12-02 adds that none of its fixtures exercise a non-zero `float_count`.
- #5502 (texture/colour half), #5509 and #5525 are untouched.

**Prior findings (AUDIT_PARSERS_2026-10-08):**
- **Fixed, with the fix in place:**
  - #5377: zero/narrow masks, L8 luminance, payload sized first.
  - #5399: refuse before cache, plus a 2× fetch budget.
  - #5400: 16-bpp decode, plus the split warning.
  - PAR-D3-2026-10-08-02 (loose `.mat` `as u8` / hex alpha): fixed by faf09c6e6 under #5396 (`u8::try_from`, and the hex path deleted).
- **Residuals filed this run:** PAR-D1-2026-10-12-03 (BC paths still allocate first) and PAR-D2-2026-10-12-05 (#5400's silent fallback).
- **Still open:** #5446.

**Crates swept**:
- `byroredux-bsa`, `-bgsm`, `-sfmaterial`, `-hkx`, `-facegen`, `-menuxml` (parse, `tex.rs`, `font.rs`, the `menu.rs` loader) and `-game-detect`.
- Consumers:
  - `byroredux/src/asset_provider/{archive,texture,animation}.rs`
  - `byroredux/src/asset_provider/material/{cdb,merge,loose_mat,provider}.rs`
  - `byroredux/src/boot/cli.rs` (optional-archive tier)
  - `byroredux/src/npc_spawn/resumable/runtime.rs` (EGM)

**Unit tests** (toolchain 1.96.0, isolated target): **356 passed, 52 ignored, plus 6 doctests. All green.**

| Crate | Passed | Ignored |
|---|---|---|
| bsa | 111 | 31 |
| bgsm | 33 | 3 |
| sfmaterial | 47 (41 lib + 6 `header_smoke`) | 4 |
| hkx | 25 | 3 |
| facegen | 31 | 4 |
| menuxml | 50 | 7 |
| game-detect | 59 | 0 |

`cargo test -p byroredux --bin byroredux asset_provider`: 230 passed, 10 ignored. Four targeted `--ignored` archive and CDB tests (`prefetch_builds_the_vanilla_index_for_a_later_lookup`, `count_shadowed_entries_reports_the_collision`, `later_listed_bsa_wins_a_mesh_path_collision`, `later_listed_sound_bsa_wins_a_collision`): 4/4 passed.

**Real-data suites** were run in the strict lane: `BYROREDUX_REQUIRE_GAME_DATA=1`, `--release`, `--test-threads=1`, one crate at a time, with all seven `BYROREDUX_<GAME>_DATA` variables and `BYROREDUX_SKYRIMLE_DATA` set, matching the CI `parsers` job. **51 tests ran; 50 passed and 1 FAILED.**

| Crate | Result | Notes |
|---|---|---|
| bsa | **29 passed, 1 failed**, plus `csg_real` 1 passed when run separately | `oblivion_all_bsas_v103_brute_force_extract_zero_errors` fails with 16 errors (the cap), all `incomplete deflate stream` on `.txt` placeholders. Cargo then never ran `csg_real` |
| bgsm | 3 passed | FO4: 6,616/6,616 BGSM and 283/283 BGEM. FO76: 29,989/29,991, plus 2 JSON files in the allowed bucket |
| sfmaterial | 4 passed | Base CDB: 500,403 keys in 1.6 s, HWM 247 MB, 115 MB after the blob is dropped. DLC: 500,385 keys plus 5 slots. The new inherited glass/emissive test passes |
| hkx | 3 passed | Census: 6,126 clips, 1,573 skipped, **288 rejected** (equal to the baseline). The LE/SE parity test ran |
| facegen | 4 passed | The Oblivion EGM test still pins nothing (#5011) |
| menuxml | 7 passed | 89 documents and 1,868 tiles. `menu_texture_header_census_decodes`: 1,975 headers |

**Orchestrator probes** (all under `/tmp/audit/parsers/probe-o/`):
- **`zsweep`**: extracts every entry of every archive in all eight installs, with real `BsaArchive` and `Ba2Archive`.
  - 3,928,089 entries in total; 37 errors.
  - All 37 are #5073 placeholders: Oblivion 35, FO3 1, FNV 1.
  - Skyrim LE, Skyrim SE, FO4, FO76 and Starfield had 0 errors. This also clears the lz4_flex 0.14 bump on the v105 frame and BA2 v3 block paths.
- **`zsweep119`**: the same 37 entries with flate2 pinned to 1.1.9 gave 0 errors.
- **`esmz`**: all 597,097 compressed records in the 98 vanilla master ESMs, decoded exactly as `crates/plugin/src/esm/reader.rs` does, under both flate2 versions. The results were identical: one #3720 Adler-32 recovery (FNV LAND 00150FC0) in both, and no new failures.
- **`swfz`**: CWS inflate of every archived `.swf` in Skyrim LE, Skyrim SE, FO4, FO76 and Starfield. Identical under both versions.
- **`cuwp - textures.ba2`**: a truncated *mod* archive in the FO4 install. Its `name_table_offset` is 1,250,980,735, past the 214,135,265 B EOF. It is rejected cleanly with field context, which is contract (b) working as intended.

**Cross-audit notes:**
- **/audit-ui**: Skyrim SE `interface\fonts_pl.swf` inflates one byte past its CWS header length. `prepare.rs` returns `None` and falls back to the stock loader. This is intended behaviour, recorded for that audit.
- **/audit-esm**: no action. The flate2 bump does not affect the ESM reader on vanilla data.

---

## Findings

### PAR-D1-2026-10-12-01: MenuRenderer's decoded-texture cache has no total bound, so a small mod archive pins gigabytes of RGBA for the HUD's lifetime
- **Severity**: HIGH
- **Dimension**: Size Discipline
- **Location**:
  - `crates/menuxml/src/tex.rs:89`: the only bound is per image, `width > 8192 || height > 8192`.
  - `crates/menuxml/src/menu.rs:97`: `tex_cache: HashMap<String, Option<Arc<Rgba8>>>`.
  - `crates/menuxml/src/menu.rs:424-485`: `MenuRenderer::texture` keys on the lowercased authored spelling, inserts on every decode and never evicts.
  - Driven every frame from `byroredux/src/hud.rs:703` (`self.renderer.render_frame(&self.assets)`).
- **Status**: NEW. Searches for "menuxml texture cache", "tex_cache", "MenuRenderer memory" and "decoded texture budget" found nothing. It is the texture-side sibling of #5314, which bounded include bytes per document but left decoded textures unbounded.
- **Trigger Input**:
  - A driven menu XML with K visible `<image>` tiles, each naming a distinct `<filename>Menus\bigN.dds</filename>`.
  - Each `bigN.dds` is an 8192×8192 DXT1 of constant blocks: 33,554,560 B raw, 48,898 B compressed in an archive (measured).
  - The cache key is the authored spelling, not the resolved archive path. `Menus\x.dds`, `Menus80\x.dds`, `Menus50\x.dds` and `textures\menus\x.dds` can all resolve to one archive file, and each spelling decodes and keeps its own copy.
- **Description**:
  - `decode_dds` caps one image at 8192², which is 256 MiB of RGBA and 16× the area of the largest vanilla menu texture (2048²).
  - Nothing bounds how many decodes the renderer keeps. `MAX_DOCUMENT_TILES` (16,384) admits thousands of distinct image tiles.
  - Decoding runs synchronously on the HUD render path on first visibility. Allocation failure aborts the process, and no `catch_unwind` can catch an abort.
- **Evidence**: probe `texcache` (real `MenuRenderer::load` and `render_frame`, release):
  ```
  k=1  first render_frame 213.8 ms  VmRSS after   268,708 kB
  k=2  first render_frame 423.8 ms  VmRSS after   530,868 kB
  k=4  first render_frame 840.2 ms  VmRSS after 1,055,188 kB
  second render_frame ~0.15 ms (cache hit), RSS unchanged
  ```
  - Growth is exactly 256 MiB per distinct filename, and the memory is retained.
  - Vanilla calibration (`ddscensus`, decoding every menu-set DDS):

    | Game | Menu DDS files | Largest | Total RGBA if all decoded |
    |---|---|---|---|
    | Oblivion | 6,023 | 2048² | 961 MiB |
    | FO3 | 1,157 | 2048² | 421 MiB |
    | FNV | 1,767 | 2048² | 689 MiB |

- **Impact**:
  - About 1.5 MB of mod archive (32 entries) pins about 8 GiB for the session. The first HUD frame stalls about 7 s in release and about 14 s in debug.
  - About 6 MB of archive (128 entries) aborts a 32 GiB machine at `--hud` load.
  - Amplification is about 5,500:1 against archive bytes, the shape #5314 was rated HIGH for. Unlike the parse-time budget, this memory is held for the renderer's lifetime.
  - Vanilla is unaffected.
- **Related**: #5314, #5329 (the same key-versus-resolved-path class), #5400, PAR-D1-2026-10-12-03.
- **Suggested Fix**:
  - Give `MenuRenderer` a decoded-texel budget sized from the vanilla census (a few multiples of the 961 MiB worst case is still generous for the driven menus). Refuse over-budget decodes as `None` with one warning.
  - Consider lowering the per-image cap toward 4096².
  - Key `tex_cache` on the resolved archive path, so alternate set spellings share one decode.

### PAR-D1-2026-10-12-02: the HKX `mask_size` gate demands the unpadded size, but Havok pads it to 4 bytes, so 229 Skyrim SE and 167 LE vanilla clips are rejected (all 117 Falmer clips included)
- **Severity**: HIGH
- **Dimension**: Size Discipline (dimension gate)
- **Location**: `crates/hkx/src/animation.rs:392`, the clause `|| mask_size != transform_count * 4 + float_count`, which returns `"unsupported spline clip dimensions"` at `:394`.
- **Status**: NEW. d4352d477 (#5326, closed) pinned these clips in `KNOWN_VANILLA_REJECTS` as "not yet diagnosed per file". No issue tracks the cause. #5446 is a different problem: its fixtures leave `mask_size` at 0.
- **Trigger Input**:
  - Field: `hkaSplineCompressedAnimation.m_maskAndQuantizationSize` when `float_count % 4 != 0`.
  - Havok stores `align4(4*tc + fc)` there.
  - Example: the Falmer clips have `tc = 85` and `fc = 1`. They store 344; the gate expects 341.
- **Description**:
  - Every SE dimension-gate reject fails this one clause and no other. Each satisfies `mask_size == align4(4*tc + fc)`:

    | `fc` | Padding | Clips |
    |---|---|---|
    | 1 | +3 | 168 |
    | 2 | +2 | 28 |
    | 3 | +1 | 15 |
    | 5 | +3 | 11 |
    | 11 | +1 | 5 |
    | 7 | +1 | 2 |

  - Every *accepted* clip with float tracks has `fc` = 4 (2,402 clips) or 8 (2 clips). Those need no padding, which is why the gate never failed on them.
  - The frames-versus-blocks (#4655), `MAX_FRAMES_PER_BLOCK` (#5006) and `MAX_TRANSFORM_SAMPLES` (#5316) clauses reject 0 vanilla clips.
  - The block decoder already aligns the mask end to 4 bytes (`align_up(mask_end, 4)`). Only the gate is wrong.
  - The padding rule is measured, not sourced. The Havok 2013 header declares the field but ships no `.cpp`.
- **Evidence**: probe `hkxprobe` on the real `Skyrim - Animations.bsa` (`/tmp/audit/parsers/probe-c/census-*.txt`):
  - Default gate: SE gives `clips=6126 rejects=288` (229 dimension + 59 non-finite), all with `pad4ok=true`.
  - Gate relaxed to `>=`: SE gives `rejects=59`, only the non-finite class remaining.
  - LE (32-bit): 167 of 203 rejects are this clause. Relaxed, only the 36 non-finite rejects remain.
  - Byte accounting: in every relaxed block the transform data ends exactly at `float_start`. The gap histogram is `{0: 235}`, the same as the strictly accepted population `{0: 6182}`.
  - The Falmer rig decodes to 85 bones. Relaxed, `falmer\animations\1hmidle.hkx` decodes to 85 tracks × 146 frames with `max(track_to_bone) = 84`.
  - Rejected / total by directory:

    | Directory | Rejected / total |
    |---|---|
    | `falmer\animations` | **117/117** |
    | `traps\bowtrap` | 8/8 |
    | `armillary` | 7/7 |
    | `cascoffinpuzzlebrazier` | 7/7 |
    | `dwetowerentrance01` | 6/6 |
    | `portalgem01`, `scportalbowl`, `plaincrossbow`, `lexiconstand` | 5/5 each |
    | `caveworm` | 4/4 |
    | `catapult` | 2/2 |
    | `character\animations` (`2hw_attack*`, `2hm_runrightattackrt*`, `idlepickaxe_*`) | 9/1301 |
    | `draugr` (`bleedout_*`) | 2/213 |
    | `dlc02\hmdaedra` | 3/43 |

    About 30 further single-asset dungeon objects are also rejected.
- **Impact**:
  - Vanilla clips hard-fail in the only HKX decoder.
  - No live consumer is affected today. The consumers are the MQ101 cart idles, `1hm_walkforward` and three Draugr clips, and none of them is in the reject set.
  - It blocks every planned expansion of HKX use: creature locomotion, animated statics and traps, and the documented ~1,400 Skyrim IDLEs.
  - Per the skill's rubric, vanilla content failing to load is HIGH.
- **Related**: #5326, #5446, #5006, #4655, #5316, PAR-D2-2026-10-12-02, PAR-D4-2026-10-12-02.
- **Suggested Fix**:
  - Gate on `mask_size != align_up(transform_count * 4 + float_count, 4)`. It stays exact, so #5006's file-bytes argument still holds.
  - Add a `PackfileBuilder` fixture with `float_count = 1` and a padded `mask_size`.
  - Lower `KNOWN_VANILLA_REJECTS` to 59 in the same change.

### PAR-D2-2026-10-12-01: the flate2 1.1.10 lock refresh (317d4b7a9) makes #5073 live and the strict BSA real-data lane red; no default-lane test pins empty/truncated-stream semantics, and the CI job's missing `--no-fail-fast` masks `csg_real`
- **Severity**: MEDIUM
- **Dimension**: Error Semantics / Corpus Gates
- **Location**:
  - `Cargo.lock` at `317d4b7a9` ("chore(deps): refresh Cargo.lock within existing semver ranges"): flate2 1.1.9 → 1.1.10.
  - `crates/bsa/src/safety.rs` (`inflate_bounded_zlib`, about :157-200). Its doc says "A *short* decode is deliberately still `Ok`".
  - `crates/bsa/src/archive/extract.rs`: the compressed branch, which has no empty-payload short-circuit.
  - `crates/bsa/tests/bsa_real.rs:160-245`: `oblivion_all_bsas_v103_brute_force_extract_zero_errors`.
  - `.github/workflows/real-data-gates.yml:207-208`: `cargo test --release -p "byroredux-${crate}" -- --ignored …`, with no `--no-fail-fast`.
- **Status**: NEW. The defect class is #5073 (open, LOW, filed 2026-09-29 as a hard error). What is new:
  1. At the repo's own lock it was **not** an error until 2026-10-10. Under 1.1.9 these entries extract as `Ok(empty)`. The 09-29 Oblivion brute force recorded 147,629 files and 0 errors on 1.1.9. #5073's error was most likely observed on an unlocked probe crate that resolved a newer flate2.
  2. It is now live in the engine.
  3. The strict lane is red at HEAD.
- **Trigger Input**: a BSA v103/v104 entry flagged compressed whose record size is 4: a 4-byte original-size header of 0 and no zlib stream.
- **Description**:
  - flate2 1.1.10 changed `read::ZlibDecoder` on an empty or truncated input from `Ok(0)` to `Err(UnexpectedEof, "incomplete deflate stream")`.
  - `inflate_bounded_zlib` returns that error. Its raw-DEFLATE retry needs `len >= 2`.
  - So every empty-payload compressed entry now fails `extract`, while `declared_size` still returns `Ok(0)`.
  - Nothing in the default lane pins this behaviour. `short_decode_stays_ok_for_the_shipped_padding_deltas` uses a *complete* stream with a larger declared size, so it stays green under both versions.
  - The documented "short decode stays `Ok`" contract therefore depends silently on the decoder version for truncated streams.
  - The gate that caught the change is the `--ignored` Oblivion brute force. Its CI lane has no registered runner (`gh api …/actions/runners` reports `total_count: 0`), and the scheduled runs on 10-08 through 10-11 were all cancelled. So the change landed unseen.
  - When the bsa suite does run, `bsa_real` failing stops cargo before the `csg_real` binary.
- **Evidence**:
  - `flprobe`, `ZlibDecoder::new(&[]).take(1).read_to_end`:
    - 1.1.9: `Ok(0)`
    - 1.1.10: `Err(Custom { kind: UnexpectedEof, error: "incomplete deflate stream" })`
  - `zsweep` (flate2 1.1.10, all 8 installs, 3,928,089 entries): 37 errors, every one `incomplete deflate stream` with `declared=Some(0)`.
    - Oblivion: 35 × `textures\menus*\…\*.txt` in `Oblivion - Textures - Compressed.bsa`.
    - FO3: `Fallout - Meshes.bsa : meshes\dungeons\vaultruined\placeholder.txt`.
    - FNV: `Fallout - Misc.bsa : menus\do_not_delete.txt`.
  - `zsweep119` (the same 37 entries, flate2 pinned to 1.1.9): 0 errors.
  - Blast radius checked:
    - No vanilla archive has a truncated stream; the only errors are the 37 empty ones.
    - ESM compressed records: 597,097, identical under both versions.
    - Archived SWFs: identical under both versions.
  - Strict lane at HEAD: `bsa_real … FAILED. 7 passed; 1 failed`, after which `csg_real` never ran. Run alone, `csg_real` passes 1/1.
- **Impact**:
  - **Engine:** none visible. No content loads these `.txt` files.
  - **Extract-all tooling and corpus sweeps:** they see 37 hard errors on vanilla data.
  - **Gates:** the BSA strict lane stays red until #5073 is fixed. Any other regression in `bsa_real` hides behind the known failure, and `csg_real` does not run at all.
  - **Future bumps:** the next semver-compatible refresh can change decoder semantics again with no default-lane signal.
- **Related**: #5073, #3812, #3720, #622/#812, #4659 (nightly lane), `20ad630a6` (lz4_flex 0.14, cleared by the same sweep).
- **Suggested Fix**:
  - Fix #5073: in the compressed branch, `original_size == 0 && compressed_len == 0` → `Ok(Vec::new())`.
  - Add default-lane unit tests in `safety.rs` that pin the empty-stream and truncated-stream outcomes, so a decoder bump fails `cargo test --workspace` rather than only the unrunnable nightly.
  - Add `--no-fail-fast` to the `parsers` job's `cargo test` call.

### PAR-D5-2026-10-12-01: after 5c14ebdf8 walks the full `Parent` chain, `has_opacity == None` means "class default", but `apply_cdb_material` still reads it as "opacity present"; 1,265 referenced Starfield materials are alpha-tested, 426 newly
- **Severity**: MEDIUM
- **Dimension**: Decode-Consumer Wiring
- **Location**:
  - Merge gate: `byroredux/src/asset_provider/material/merge.rs:306-316`, `threshold > 0.0 && cdb_mat.has_opacity != Some(false)`.
  - Test that pins it: `:2135-2189` (`alpha_test_gates_on_an_explicit_has_opacity_false`, third arm `has_opacity: None`).
  - Producer: `crates/sfmaterial/src/index.rs:378-405` (`collect_settings` / `collect_own_settings`). The threshold and `HasOpacity` come from the same `AlphaSettingsComponent` capture, nearest-wins up the chain.
- **Status**: NEW. It is a side effect of the #5502 partial fix (5c14ebdf8) meeting #5196's `None` rule. Open #5502 covers only the texture/flat-colour half.
- **Trigger Input**: a referenced CDB material whose entire `Parent` chain authors `AlphaTestThreshold > 0` but never `HasOpacity`.
- **Description**:
  - Before 5c14ebdf8, `None` meant "this object does not author it". Now the lookup walks to the parentless top object, so a remaining `None` means no object in the chain authors the field, i.e. the class default.
  - The vanilla diff census points to `false` as that default:
    - `HasOpacity = true` is authored 1,449 times, always over an inherited `None`.
    - `HasOpacity = false` is authored 28 times, and 27 of those override an inherited `true`.
  - The merge therefore alpha-tests, against base-colour alpha, surfaces that author no opacity channel. Starfield's real opacity source, slot 2, is parked.
  - The default is **not sourced**: neither Gibbed.Starfield nor Material-Editor in `/mnt/data/src/reference` names it. Under the no-guessing rule it must be confirmed before a fix lands.
- **Evidence**: `cdbprobe` over the vanilla CDB and the 10,152 NIF-referenced material paths.
  - Current alpha-test state:

    | `alpha_test` | `has_opacity` | Threshold | Materials |
    |---|---|---|---|
    | on | `None` | 0.333333 | 1,227 |
    | on | `None` | other | 38 |
    | on | `Some(true)` | any | 864 |
    | off | any | any | 8,023 |

  - Flips caused by 5c14ebdf8:
    - `alpha_test`: 585 off→on, 0 on→off. 426 of the 585 have `HasOpacity` absent along the whole chain.
    - `bgem_glass`: 213 off→on, the documented #5502 intent.
  - Example chain: `Materials\Architecture\City\Akila\AKTarpLayered02.mat` → `AKTarpLayered01` (`thr = 0.333333`, no `HasOpacity`) → `3LayerStandard` → `#7`.
  - The 5c14ebdf8 message and its `real_cdb.rs` census report only IsGlass 4→217 and emissive 0→619.
- **Impact**: about 12% of referenced Starfield materials take the alpha-test path on a channel that is not their opacity. They lose early-Z, pay RT any-hit cost, and may cut holes where colour alpha is below 0.333; the hole extent was not measured. The canonical flag at the NIFAL boundary is silently wrong.
- **Related**: #5502, #5196, #3398, #5525 (the loose `.mat` producer uses the same rule).
- **Suggested Fix**:
  - Source the `AlphaSettingsComponent.HasOpacity` default from the CK schema or a capture.
  - If it is `false`, gate on `has_opacity == Some(true)` and update the #5196 test's `None` arm.
  - Record the alpha-test delta in the commit, as 5c14ebdf8 should have.

### PAR-D2-2026-10-12-02: one prop-attachment bone with NaN translation/scale bounds rejects the whole HKX clip (59 SE, 36 LE vanilla clips); finite garbage on the same bones passes unchecked
- **Severity**: MEDIUM
- **Dimension**: Error Semantics
- **Location**:
  - `crates/hkx/src/animation.rs:759-763`: `read_vector_curve` reads the dynamic min/max through `read_f32`, which hard-fails on a non-finite value at `:946-951`. The `?` propagates to `decode_spline_animation`.
  - `animation.rs:783`: finite magnitudes are not validated.
  - Consumer: `byroredux/src/asset_provider/animation.rs:403-487` (`convert_hkx_clip`).
- **Status**: NEW. This is the remaining 59 of the 288 baseline that #5326 pinned.
- **Trigger Input**:
  - A translation or scale track with mask `0x70` (quantisation `0x45`) whose per-component min and max are both NaN. Observed bit patterns: `ffffffff`/`7fffffff` 198 each, `ffc00000`/`7fc00000` 60 each, and smaller counts of others.
  - Always on one bone per rig: track 41 `AnimObjectR` on the 99-bone human rig, track 40 `AnimObjectL` on the dragon-mount clips, or that bone's slot in paired rigs.
- **Description**:
  - The NaNs are genuinely in the files.
  - Rotation and every other track decode; a NaN-tolerant decode succeeds on 59/59.
  - The decoder turns one unused prop-attachment channel into a whole-clip `Err`. That contradicts the crate's own #3018 per-element policy ("a bad annotation costs the annotation, not the clip").
  - The policy is also inconsistent: 12 *accepted* clips carry the same artifact as finite garbage on the same bones (`|T|` up to 1.41e22), which flows unchecked into `AnimationClip` keys.
  - That vanilla Skyrim ignores these channels is inferred, not sourced: `AnimObject*` matters only when an IDLE attaches a prop.
- **Evidence**:
  - `nanprobe`: `TOLERANT-OK` 59/59.
  - Bone distribution: 156 hits on `tc=99` track 41, 6 on `tc=97/98` track 40, plus the paired-rig slots.
  - Affected clips include `dialogueangryhips.hkx`, `ritualspell_aimrelease.hkx`, `_1stperson\…\dlc01\crossbow_{idledrawn,release,blockbash}.hkx`, `beasth2h_*attack*`, `paired_1hmkillmove*`, `sharedkillmoves\human&{dragon,werewolf,giant,…}\paired_*` and `dlc2_paireddragonmount*`.
- **Impact**:
  - About 1% of vanilla clips cannot decode, including most killmoves. No live consumer is affected today.
  - Simply accepting NaN would be worse: it would put a NaN bone in the palette.
- **Related**: #3018, #3013, #3765, #4396, PAR-D1-2026-10-12-02.
- **Suggested Fix**:
  - On non-finite bounds for a dynamic T or S channel, keep the clip, drop just that track from `tracks` and `track_to_bone`, and `log::warn!` once naming the track. This mirrors `convert_hkx_clip`'s #3013 arm, so the bone keeps its bind pose and no value is invented.
  - Decide in the same change whether `|T| > 1e6` on these bones gets the same treatment.

### PAR-D6-2026-10-12-01: `open_shared`'s (length, mtime) stamp misses a same-length, mtime-preserving in-place rewrite, so a reused handle returns another entry's bytes as `Ok`
- **Severity**: LOW
- **Dimension**: I/O & Paths
- **Location**:
  - `byroredux/src/asset_provider/archive.rs:111-160`: `open_shared` and `ArchiveStamp { len, modified }`.
  - Test: `archive.rs:925-950`, which rewrites with a *longer* payload only.
  - Path-keyed CDB caches that are never invalidated: `byroredux/src/asset_provider/material/cdb.rs` (`sf_cdb_cache`, `sf_cdb_index_cache`).
- **Status**: NEW (5402b2530, #5288).
- **Trigger Input**: an opened BSA or BA2 is rewritten in place mid-session to a different layout of the same byte length, and either:
  - the writer restores the mtime (`cp -p`, `rsync -t --inplace`, `touch -r`, Windows `CopyFile`), or
  - `Metadata::modified()` fails, so the stamp becomes `(len, None)`.
- **Description**:
  - The cached reader keeps its fd and its parsed table. After an in-place rewrite, the fd sees the new bytes at the old offsets. The length is unchanged, so every read stays in bounds.
  - Neither format carries a checksum, so a stored entry returns `Ok` holding another entry's bytes. A compressed entry usually fails to inflate and falls through with a warning.
  - After a rename replacement, the fd still reads the old inode: the content is stale but self-consistent.
  - The skill contract says "Bounds checks must fail on a replaced archive, not return garbage". The registry now holds tables across rebuilds; before #5288 each rebuild re-parsed.
- **Evidence**: probe _probe-b/stale_ (`open_shared` copied verbatim, real `Ba2Archive`; ext4 and tmpfs gave the same result):
  ```
  S1 in-place rewrite, mtime preserved: ctime changed=true inode changed=false
    second open_shared reused=true
    stale handle extract a (want 'aaaaaaaa'): Ok("bbbbbbbb")
  S3 rename replacement, mtime preserved: reused=true → Ok("AAAAAAAA") (old inode)
  S4 same-tick same-length rewrite without mtime restore: detected 2000/2000 (multigrain timestamps)
  ```
- **Impact**: the trigger is unlikely, since an archive has to be replaced mid-session by a same-size file with the same mtime. If it happens, every later rebuild (door cover, destination apply, save-load) serves wrong bytes for stored entries.
- **Related**: #5288, #3637, #4658.
- **Suggested Fix**:
  - Add `dev`, `ino` and `ctime`/`ctime_nsec` (Unix) or the file index (Windows) to `ArchiveStamp`. `utimensat` cannot set ctime back.
  - Add a same-length, mtime-restored rewrite test.
  - Key the CDB caches on the stamp, or document that they live for the whole process.

### PAR-D6-2026-10-12-02: reused handles skip the #3637 shadow count, so an archive first opened in another pool never logs its override count for this pool
- **Severity**: LOW
- **Dimension**: I/O & Paths
- **Location**:
  - `byroredux/src/asset_provider/archive.rs:699-712`: `log_opened_archive` returns early when `reused`.
  - `byroredux/src/boot/cli.rs:230-253`: `push_optional_archive_args` emits `--bsa P --textures-bsa P --sounds-bsa P`.
- **Status**: Regression of #3637 (diagnostic only; introduced by 5402b2530 / #5288).
- **Trigger Input**: every `--game oblivion` (GOTY) and `--game skyrim_se` (AE / Creation Club) boot.
- **Description**:
  - The early return assumes the count "was already reported… the answer for a repeat open in the same order is the same". That is true within one pool.
  - The registry is keyed by path, not by pool. The `--bsa P` open computes the count against the mesh chain, and the `--textures-bsa P` open is then a registry hit that logs "(shared handle)" and never computes the textures-chain count.
- **Evidence**: probe _probe-b/stale/src/bin/shadow.rs_ (`count_shadowed_entries` copied verbatim, real installs):

  | Archive | Mesh-pool count (logged) | Textures-pool count (now suppressed) |
  |---|---|---|
  | Oblivion `DLCShiveringIsles - Textures.bsa` | 0 | 5 |
  | Oblivion `DLCFrostcrag.bsa` | 3 | 3 |
  | Skyrim `ccBGSSSE037-Curios.bsa` | 3 | 3 |

- **Impact**: diagnostic only. Precedence is unchanged, because it follows push order and `every_content_provider_resolves_collisions_last_wins` still passes. The silent texture overrides #3637 exists to surface are no longer logged.
- **Related**: #3637, #3924, #2584.
- **Suggested Fix**: memoise the count on the shared handle per pool kind, or keep a static set of `(kind, path)` pairs already reported. Skip only on a repeat for the same pool.

### PAR-D2-2026-10-12-03: the single-flight CDB slot does not memoise a *panicking* build, and the `sf-cdb-index` worker has no `catch_unwind`
- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `byroredux/src/asset_provider/material/cdb.rs`:
  - `:70-75`: slot doc ("`Some(None)` memoises a failed build").
  - `:136-141`: `cdb_material_index`.
  - `:144-195`: `build_cdb_material_index`.
  - `:202-223`: `prefetch_cdb_material_index`.
- **Status**: NEW. This is defence in depth: no current input is known to make `MaterialIndex::build` panic (the spot-check of `crates/sfmaterial/src/{index,reader}.rs` found none).
- **Trigger Input**: a mod or Creation `materialsbeta.cdb` that makes `MaterialIndex::build` panic, for example after a future reader regression.
- **Description**:
  - `OnceLock::get_or_init` stores only a returned value. A panic leaves the cell empty, and every waiter and later caller re-runs the closure.
  - So the worker panics, and the parked main-thread lookup re-runs the ~1.6 s, ~247 MB build on the main thread and panics there.
  - The `merge_external_materials` callers have no `catch_unwind`.
  - The sibling worker `byroredux/src/asset_provider/texture_prefetch.rs:224-229` already wraps its read in `catch_unwind`, for exactly this reason.
- **Evidence**: probe _probe-b/oncelock_, same slot type and gate:
  ```
  thread 'sf-cdb-index' panicked … build #1
  [main] build #2 starts … thread 'main' panicked … build #2
  slot initialised after panics: false
  total builds run: 3 (a memoised failure would be 1)
  ```
- **Impact**: a hypothetical panicking CDB crashes the engine on the main thread at the first `.mat` lookup, after paying the build twice.
- **Related**: #5291, #5319, #2614, `byroredux/src/asset_provider/texture_prefetch.rs`.
- **Suggested Fix**:
  - Wrap the extract and `MaterialIndex::build` in `catch_unwind(AssertUnwindSafe(..))`. Map a panic to a `warn!` and `None`, so the slot memoises it.
  - Add an injected-panic test that asserts exactly one build.

### PAR-D5-2026-10-12-02: the prefetch worker takes `PREFETCH_GATE` before creating its slot, so a lookup for a still-queued CDB builds on the main thread, ungated, alongside the gated worker
- **Severity**: LOW
- **Dimension**: Decode-Consumer Wiring
- **Location**:
  - `byroredux/src/asset_provider/material/cdb.rs:202-223`: the `contains_key` check, the spawn, `PREFETCH_GATE.lock()`, and only then the slot creation inside `cdb_material_index`.
  - `:229-241`: `lookup_cdb_material` walks sources last-registered-first.
- **Status**: NEW (dd759b762, #5291).
- **Trigger Input**:
  - Two or more valid CDBs, with the first `.mat` lookup arriving before the serial prefetch queue drains.
  - Not reachable with the shipped `starfield` profile, which has one CDB. It needs DLC or Creation `* - Main.ba2` archives named manually.
- **Description**:
  - The doc says a mid-build lookup "parks on that build" and that N CDBs "must not peak at N× that at once".
  - A queued worker has no slot yet, so the lookup builds that CDB itself on the main thread, outside the gate and concurrently with the gated worker.
  - When the queued worker later takes the gate, it parks on the main-thread build while still holding the gate, which stalls the queue behind it.
  - There is no deadlock: the main thread never takes the gate, and the map locks are leaves.
- **Evidence**: probe _probe-b/oncelock/src/bin/queue.rs_:
  ```
  build base on thread 'sf-cdb-index[base]' (concurrent builds now 1)
  build dlc on thread 'main' (concurrent builds now 2)
  main-thread lookup walk blocked 400.069539ms; peak concurrent builds = 2
  ```
- **Impact**: performance and doc accuracy only. On multi-CDB runs the ~1.6 s main-thread build that #5291 removed returns inside a streamed apply slice, with a doubled memory peak.
- **Related**: #5291, PAR-D2-2026-10-12-03.
- **Suggested Fix**:
  - Create the slot synchronously in `prefetch_cdb_material_index` and take the gate *inside* the `get_or_init` closure, so a lookup always parks on an owned slot. Alternatively, have lookups take the same gate.
  - Correct the doc to match.

### PAR-D1-2026-10-12-03: the BC1/BC2/BC3 decoders still allocate the full RGBA canvas before checking the block payload; #5377's "size before allocate" covers only the uncompressed path, and its guard cannot fail
- **Severity**: LOW
- **Dimension**: Size Discipline
- **Location**:
  - `crates/menuxml/src/tex.rs:190-191`, `:205-206`, `:232-233`: `decode_dxt{1,3,5}` call `Rgba8::new` before the per-block `off + N > bytes.len()` check.
  - Guard test: `tex.rs:476` (`truncated_payload_is_rejected_before_allocation`).
- **Status**: NEW. It is a residual of #5377, whose suggested fix was applied to `decode_uncompressed` only.
- **Trigger Input**: a 128-byte DDS with FourCC `DXT1` or `DXT5`, 8192×8192, and no payload.
- **Description**:
  - The BC paths reserve 256 MiB and then return `None` at the first missing block.
  - The test asserts only `is_none()`. A mutant that allocates before the uncompressed check still passes it (`mut-allocfirst`).
- **Evidence**: `ddscases` (counting allocator, real `decode_dds`):
  - `DXT1 8192² no payload`: `None`, peak allocation 268,435,456 B.
  - `A8R8G8B8 8192², 4 B payload`: peak allocation 0 B.
- **Impact**: low. The allocation is `calloc`, so it is mostly virtual and freed on return. The real cost is that the guard gives false assurance.
- **Related**: #5377, PAR-D1-2026-10-12-01.
- **Suggested Fix**:
  - Check `128 + div_ceil(w,4) * div_ceil(h,4) * block_bytes <= len` before `Rgba8::new` in each BC decoder.
  - Make the test assert the allocation order, for example with a counting allocator or a size-returning helper.

### PAR-D2-2026-10-12-04: once an include budget is spent, every further distinct include path still logs its own warning, and the byte-denominated fetch budget admits about 131k fetches of 2-byte fragments
- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `crates/menuxml/src/parse.rs`:
  - `:675-705`: `warn_once`, keyed `"{cause}:{path}"`.
  - `:788-805`: the budget, bytes and fetched early returns.
  - `:845`: `bytes_fetched += bytes.len()`, with no per-fetch floor.
- **Status**: NEW. The "fetched" cause comes from 6a132a967 (#5399); "budget" and "bytes" predate it.
- **Trigger Input**: a menu XML whose includes exhaust a budget and then name K further distinct prefabs. Variant: after splicing to 262,143 B, K distinct 2-byte fragments.
- **Description**:
  - The "exhausted" notices are per path, not per document.
  - The fetch budget counts bytes, so tiny refused fragments each still cost one archive extract.
  - Both effects are linear in document size: a log flood and a main-thread stall, not an amplification.
- **Evidence**: probe `includes`, real `parse_document`, release:

  | Case | Root | Probes | Fetched | Warn lines |
  |---|---|---|---|---|
  | 100k distinct missing includes | 2.7 MB | 300,000 | 0 B | 100,000 |
  | Splice to the cap, then 200k distinct 2-byte refusals | 5.5 MB | 131,077 | 524,289 B | 200,000 |

- **Impact**: a 5.5 MB mod menu XML emits 200,000 warn lines and up to 131k main-thread archive extracts at `--hud` load. Memory stays bounded.
- **Related**: #5399, #5314, #5007.
- **Suggested Fix**:
  - Warn once per document for the exhausted-budget causes.
  - Charge each fetch a floor, for example `max(len, 1 KiB)`, so the fetch budget also bounds the fetch count.

### PAR-D4-2026-10-12-01: the #5400 header census gates only Oblivion `textures\menus\`, not `menus80`/`menus50` (which the loader also serves) or FO3/FNV `textures\interface\`
- **Severity**: LOW
- **Dimension**: Corpus Gates
- **Location**: `crates/menuxml/tests/vanilla_corpus.rs:204-212`, the `starts_with("textures\\menus\\")` filter in `menu_texture_header_census_decodes`.
- **Status**: NEW. It is a scope gap in #5400's census.
- **Trigger Input**: n/a.
- **Description**:
  - The census covers 1,975 of Oblivion's 5,476 base-game menu-set DDS. It skips `menus80` (1,712) and `menus50` (1,789), both of which `MenuRenderer::texture` resolves.
  - FO3/FNV HUD art lives in `textures\interface\`, which no test decodes. That includes 24-bpp `hud_comp_direction_vertical.dds` and the 32-bpp `target_*` and `shared\line\*` files.
  - Nothing asserts a vanilla count, so the census can pass vacuously.
- **Evidence**: `ddscensus` decodes all of them today: Oblivion (all three sets plus SI) 6,023/6,023, FO3 1,157/1,157, FNV 1,767/1,767.
- **Impact**: no current failure; this is lost regression coverage for the FO3/FNV HUD driver's art.
- **Related**: #5400, #4665.
- **Suggested Fix**:
  - Widen the Oblivion filter to `textures\menus*`.
  - Add the census over `textures\interface\` to `fo3_corpus.rs` and `fnv_corpus.rs`.
  - Assert the vanilla counts.

### PAR-D2-2026-10-12-05: a present-but-undecodable texture in the preferred resolution set is silently replaced by another set's copy
- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `crates/menuxml/src/menu.rs:461-482`. `undecodable` is logged only when *no* candidate decodes.
- **Status**: NEW (5c53ad9c7, #5400).
- **Trigger Input**: with the preferred set `menus`, `textures\menus\x.dds` is present but undecodable while `textures\menus80\x.dds` decodes.
- **Description**:
  - Keeping the search going past an undecodable candidate is correct.
  - But when a lower-priority set then succeeds, the substitution is silent, and the result is cached.
- **Evidence**: probe `fallback` (real `MenuRenderer`, counting logger, 3 frames): probes `["textures\\menus\\hud\\a.dds", "textures\\menus80\\hud\\a.dds"]`, warnings `[]`.
  - The same probe confirms that priority order is preserved, each candidate is probed once, and nothing is retried per frame.
- **Impact**: a degraded result (another set's art) with no log line. Cosmetic.
- **Related**: #5400.
- **Suggested Fix**: when `decoded.is_some() && !undecodable.is_empty()`, warn once per cache key.

### PAR-D4-2026-10-12-02: the HKX census asserts `rejects <= 288` while claiming to catch "drift in either direction", pins a count rather than a set, and its block-consistency clause is inert
- **Severity**: LOW
- **Dimension**: Corpus Gates
- **Location**: `crates/hkx/src/animation.rs`:
  - `:1326-1342`: `assert!(rejects.len() <= KNOWN_VANILLA_REJECTS)`; the comment at `:1332` says "catches drift in either direction".
  - `:1347-1350`: `div_ceil(MAX_FRAMES_PER_BLOCK) >= max_blocks || max_blocks <= 64`.
- **Status**: NEW (d4352d477, the #5326 fix). Distinct from #5446, which covers the unit fixtures.
- **Trigger Input**: n/a.
- **Description**:
  - A one-sided `<=` catches only growth. Once PAR-D1-2026-10-12-02 lands (288 → 59), a regression re-rejecting up to 229 clips stays green unless the constant is lowered too.
  - A swap of which clips are rejected passes, because only the count is pinned.
  - The block clause holds only through its `|| max_blocks <= 64` arm: measured `1471.div_ceil(256) = 6 < 15`. Its "consistency" half never constrains anything.
- **Evidence**: see the location above. The strict run printed `census: 288 clips rejected by the real decoder` and passed.
- **Impact**: the real-data gate can bless a regression of the decoder finding it was written to track.
- **Related**: #5326, #5446, PAR-D1-2026-10-12-02, PAR-D2-2026-10-12-02.
- **Suggested Fix**:
  - Use `assert_eq!` against the baseline, or better, pin the sorted set of rejected paths split by error variant.
  - Drop or correct the inert block clause.

### PAR-D2-2026-10-12-06: the CDB `Objects` table decode and the Parent-chain walk degrade silently (shape mismatch drops a row or an edge; a chain past `MAX_PARENT_DEPTH` or a non-start cycle ends without a warning)
- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `crates/sfmaterial/src/index.rs`:
  - `:686-692`: `id_named(o, "DBID")` / `id_named(o, "Parent")` returning `None` silently skips the row or the edge.
  - `:743-751`: `id_named`.
  - `:378-387`: `for _ in 0..=MAX_PARENT_DEPTH`, with only `parent != obj` as a cycle check.
- **Status**: NEW. #5578 added the default-lane decode test but not a signal. The #5320 strictness covers `Components` only.
- **Trigger Input**: a non-vanilla CDB whose `ObjectInfo` declares `DBID` or `Parent` in a shape other than `{ Value: u32 }`; a chain deeper than 64; or a cycle that does not pass back through the start.
- **Description**:
  - Contract (c) requires a degraded decode to be `Ok` plus a warning, or an error. These cases are neither.
  - A Parent shape drift would silently disable all #5502 inheritance (IsGlass 217 → 4, alpha-test −585).
  - The walk is bounded, so it terminates, but cycles and truncation are not detected.
- **Evidence**: `cdbprobe` on vanilla:
  - 500,403 rows: DBID misses 0, Parent-shape misses 0, `Parent = 0` on 6 rows.
  - Maximum chain depth 24 (2.7× headroom under the 64 cap), 0 cycles.
  - Vanilla is clean. The finding is the missing signal on non-vanilla input.
- **Impact**: schema drift in a mod or DLC CDB degrades Starfield materials wholesale with only an info-level key count to show for it.
- **Related**: #5320, #5578, #5502, #5465.
- **Suggested Fix**:
  - Count `Objects` rows whose DBID or Parent fails `id_named`, and error or `warn!` once, mirroring `RowInstanceMismatch`.
  - Detect revisits and depth-cap hits in `collect_settings` and warn once per lookup.

### PAR-D5-2026-10-12-03: the CDB translucency capture keys on `BSMaterial::TranslucencySettings`, which never occurs as a top-level component, so `use_sss` and `transmissive_scale` are always `None`
- **Severity**: LOW
- **Dimension**: Decode-Consumer Wiring
- **Location**:
  - `crates/sfmaterial/src/index.rs:571-591`: the match arm.
  - `:99-102`: field docs.
  - `crates/sfmaterial/examples/cdb_join_probe.rs:130`.
  - Source of the error: `docs/audits/SF_CDB_PHASE2_SPIKE_2026-08-29.md:183`.
- **Status**: NEW. It predates this delta (224a19372) and was found while auditing the 5c14ebdf8 settings pass.
- **Trigger Input**: every vanilla `BSMaterial::TranslucencySettingsComponent`: 19,768 instances, with fields `Enabled` plus an inline `Settings: BSMaterial::TranslucencySettings { …, TransmissiveScale, …, UseSSS }`.
- **Description**:
  - The arm matches the nested struct's class name as if it were the component, so it never fires.
  - The emissive arm (#5283) already shows the correct shape: match the `…Component` and read the inline `Settings`.
  - The fields are parked downstream on purpose (#5196), so nothing renders differently today. But un-parking them would be inert, the same "wired but cannot fire" shape #5502 was filed for.
- **Evidence**: `cdbprobe` class census: `CLASS 19768 BSMaterial::TranslucencySettingsComponent`, no top-level `BSMaterial::TranslucencySettings`, `translucency captures with data: 0`.
- **Impact**: dead capture, and the docs overstate the capture set. No rendering impact while the field stays parked.
- **Related**: #5196, #3398, #5502, #5525 (the loose `.mat` arm probably misses the nested `Settings` too; unverified).
- **Suggested Fix**:
  - Match `TranslucencySettingsComponent`, read `Enabled` and `Settings.{UseSSS, TransmissiveScale}`, and add a synthetic nested fixture.
  - Correct the spike doc's class table.

---

## Already-tracked findings re-verified

| Issue | State at c1a99c25e |
|---|---|
| #5009 debug-load raw extract loop | Unchanged: `byroredux/src/debug_load.rs:181` `if let Ok(data) = archive.extract(path)` |
| #5010 BGSM diagnostics silent on the template path | Unchanged: `crates/bgsm/src/template.rs:257` calls `parse_bgsm` |
| #5011 Oblivion EGM / BGSM sweep weakness | Unchanged: the strict run still prints `(record these in the EXPECT table)` |
| #5073 empty compressed BSA payload | **Now live at the repo's lock** (flate2 1.1.10). See PAR-D2-2026-10-12-01 |
| #5329 cycle check over-approximation | Unchanged. Agent A confirmed the include cache key normalises the way the BSA lookup does: 5 spellings, 1 fetch |
| #5446 HKX dimension-gate fixtures | Unchanged. None of the fixtures carries `float_count ≠ 0`, so none could have caught PAR-D1-2026-10-12-02 |
| #5502 / #5509 / #5525 | Untouched. PAR-D5-2026-10-12-01 and PAR-D5-2026-10-12-03 are new, adjacent defects |

## Prior-finding verification (AUDIT_PARSERS_2026-10-08)

| Prior finding | Issue | State now |
|---|---|---|
| D2-01 menuxml mask underflow | #5377 (closed) | Fixed. 1.62 M fuzzed headers in a debug build, 0 panics. L8 decodes grey and R3G3B2 is not taken for luminance. The BC-path residual is PAR-D1-2026-10-12-03 |
| D1-01 refused fragments retained | #5399 (closed) | Fixed. The fetch-budget check discriminates under mutation. The cache-as-`None` half is unpinned but bounded by the 512 KiB fetch budget (not filed). The warn-flood residual is PAR-D2-2026-10-12-04 |
| D3-01 16-bpp menu art | #5400 (closed) | Fixed. All 6,023 Oblivion menu-set DDS decode. Residuals: PAR-D2-2026-10-12-05 (silent fallback) and PAR-D4-2026-10-12-01 (census scope) |
| D4-01 HKX gate tests vacuous | #5446 (open) | Unchanged |
| D3-02 loose `.mat` `as u8` / hex alpha | (folded into #5396, closed) | Fixed by faf09c6e6: `u8::try_from`, the hex path deleted, `out_of_range_index_is_dropped_not_wrapped` |

## Gate Matrix

| Crate / format | Size caps | Error policy | Version gate | Corpus sweep (strict lane) | CI lane |
|---|---|---|---|---|---|
| bsa: BSA | ✓ `checked_entry_count` / `checked_chunk_size` / `inflate_bounded(_zlib)`; file-relative `capacity_hint` | ✓ labelled `Err`. ✗ empty compressed entry is a hard error (#5073, live since flate2 1.1.10). Consumers go through `extract_first`, except `debug_load.rs` (#5009) | ✓ {103, 104, 105}; LZ4 **frame** on v105 | **Oblivion brute force RED** (37 placeholders across Oblivion/FO3/FNV); SSE Meshes0 0 errors; FNV, FO3 open+list; `declared_size` = extract | nightly `parsers`, but no runner (0 registered) and no `--no-fail-fast` |
| bsa: BA2 | ✓ per-chunk and per-record caps | ✓ path-named; `chunk_hdr_len` is a hard `Err`; truncated mod BA2 rejected with field context | ✓ {1, 2, 3, 7, 8}; method 0/3 | FO4 / FO76 / Starfield (the orchestrator sweep extracted all 3.1 M BA2 entries with 0 errors) | same |
| bsa: CSG / UVD | ✓ | ✓ | magic | FO4 object 0 (masked behind `bsa_real` in CI); UVD has no consumer | same |
| asset_provider registry (#5288) | 1024-entry cap (arbitrary eviction) | ✓ leaf lock, poison recovered. ✗ stale reuse on same-length mtime-preserving rewrite (D6-01); shadow count suppressed across pools (D6-02) | n/a | n/a | bin unit lane |
| bgsm | ✓ | ✓. Diagnostics silent on the template path (#5010) | per-field gates | FO4 100%, FO76 100% + 2 JSON; no diagnostic asserts (#5011) | nightly |
| sfmaterial (tree) | ✓ nesting 64 | ✓ typed | ✓ skip ≡ read | exact 97 / 1,438,780 | nightly |
| sfmaterial (`MaterialIndex`) | per-instance decode; Parent depth 64 | ✓ typed `Components` join. ✗ `Objects` / Parent degrade silently (D2-06) | inherits | base 500,403 + 5 slots; DLC 500,385 + 5 slots (#5326 ✓); inherited glass/emissive | nightly |
| CDB merge / worker (byroredux) | ~247 MB build, ~115 MB resident | ✓ `Err` warned and memoised. ✗ panic not memoised (D2-03); ✗ `HasOpacity` `None` read as present (D5-01) | n/a | `prefetch_builds_the_vanilla_index_for_a_later_lookup` (passes even when prefetch is a no-op) | bin unit lane |
| loose `.mat` | archive-bounded | ✓ `try_from` index (#5396) | n/a | none (no vanilla sample) | bin unit lane |
| hkx | strings ≤ 256 B; mfpb ≤ 256; samples ≤ 1 M. ✗ `mask_size` gate rejects padded vanilla clips (D1-02) | ✓ labelled. ✗ one NaN channel drops the whole clip (D2-02) | ✓ #4332 gates | census pins 6,126 clips and ≤ 288 rejects (one-sided, D4-02) | nightly; fixtures cannot fail on their own clause (#5446) |
| facegen | ✓ caps + exact size | ✓ | ✓ magics | FNV/FO3 pinned; Oblivion EGM weak (#5011) | nightly |
| menuxml (parse) | depth 48/64; splices 256; bytes 256 KiB; fragment 64 KiB; fetched 512 KiB; tiles 16,384 | ✓. Warn per distinct path past the budget (D2-04) | n/a | Oblivion / FO3 / FNV (tile totals printed, not pinned) | nightly |
| menuxml (`tex.rs` + loader) | dims ≤ 8192; uncompressed payload checked first. ✗ BC paths allocate first (D1-03); ✗ **no total decoded budget** (D1-01) | ✓ no panic (1.62 M fuzzed); "present but undecodable" split. ✗ silent fallback (D2-05) | FourCC DXT1/3/5; masked 8/16/24/32 | Oblivion `textures\menus\` only (D4-01) | debug unit fixtures now cover L8/A8/narrow masks |
| game-detect | ✓ VDF depth 32 | ✓ | n/a | unit only | ✓ `ci.yml` |

## Guards confirmed live (not `#[ignore]`d, passing this run)

**Size ceilings:**
- `entry_count_rejects_attacker_u32_max`, `over_ratio_payload_is_rejected_at_the_ceiling`
- `malicious_bsa_folder_count_u32_max_rejected`, `malicious_file_count_u32_max_rejected_before_allocation`
- `lz4_flex_is_pinned_to_the_safe_decoder` (0.14.0 keeps `safe-decode`; checked in the registry `Cargo.toml`), `lz4_decompress_is_panic_guarded`
- `oversized_read_len_is_rejected_before_it_is_reserved`
- `decode_spline_animation_rejects_*` (see #5446), `rejects_morph_count_over_cap`
- `parse_with_limits_rejects_object_tree_before_materialising_it`

**Nesting and includes:**
- `include_cycles_terminate`, `self_including_fragment_costs_one_fetch`, `budget_exhausted_includes_never_fetch`
- `refused_includes_are_bounded_by_the_fetch_budget` (discriminates under mutation), `oversized_include_fragment_is_skipped_and_not_refetched`
- `resolve_breaks_self_reference_cycle`, `self_referential_struct_is_rejected_instead_of_overflowing_the_stack`, `pathological_nesting_hits_the_depth_cap`
- `lookup_terminates_on_a_parent_cycle`

**Error semantics:**
- `dx10_open_rejects_a_non_24_chunk_hdr_len_as_invalid_data`, `extract_dx10_rejects_a_short_non_final_chunk_naming_the_entry`
- `short_decode_stays_ok_for_the_shipped_padding_deltas` (complete-stream only; see D2-01)
- `corrupt_adler32_trailer_recovers_via_raw_deflate`, `corrupt_deflate_body_still_errors`
- `read_annotations_skips_an_out_of_range_time_and_keeps_the_rest`
- `unfetchable_cdb_resolves_none_and_memoizes_the_failure`
- `l8_grey_decodes_as_luminance_not_red`, `a2r10g10b10_two_bit_alpha_replicates_to_opaque`, `r5g6b5_16bpp_decodes_opaque`

**Version and layout:**
- `synthetic_v105_block_codec_payload_is_rejected_by_frame_reader`, `unknown_version_rejected`, `v3_unknown_compression_method_rejected`, `build_dds_header_is_148_bytes`
- `chunk_type_recognized_set_is_pinned`, `builtin_type_recognized_set_is_pinned`, `class_flags_known_mask_is_pinned`
- `read_order_reorders_xmcolor_shape`, `skip_and_read_consume_identically_on_a_reordered_class`
- `parent_edge_is_decoded_from_the_dbfile_index_and_inherited`
- `rejects_packfiles_that_are_not_havok_2010_msvc_layout`, `layout_walk_reproduces_the_skyrim_se_offsets`
- `rejects_size_count_mismatch`, `non_square_header_maps_rows_to_height_and_columns_to_width`

**Paths and registry:**
- `siblings_*` (7), `every_content_provider_resolves_collisions_last_wins`
- `open_shared_reuses_an_unchanged_archive_and_rereads_a_replaced_one` (longer-payload case only; see D6-01)
- `cdb_paths_are_scanned_once_per_opened_archive`, `corrupt_override_falls_through_and_is_named_once`
- `prefetch_fills_the_lookup_slot_without_a_lookup`
- `malformed_documents_are_rejected_rather_than_half_read`, `only_a_bare_directory_name_is_a_valid_installdir`

**Constants:** exactly one definition each of `MAX_ENTRY_COUNT`, `MAX_CHUNK_BYTES` and `MAX_RECORD_TOTAL_BYTES` (`crates/bsa/src/safety.rs`).

## Publishing

`/audit-publish docs/audits/AUDIT_PARSERS_2026-10-12.md`

| Finding | Suggested labels |
|---|---|
| PAR-D1-2026-10-12-01 | `high` `bug` `ui` `memory` `safety` |
| PAR-D1-2026-10-12-02 | `high` `bug` `animation` `game:skyrim` |
| PAR-D2-2026-10-12-01 | `medium` `bug` `import-pipeline` `dependencies` `test-gap` (comment on #5073 too; its root cause is now known) |
| PAR-D5-2026-10-12-01 | `medium` `bug` `nifal` `game:starfield` |
| PAR-D2-2026-10-12-02 | `medium` `bug` `animation` `game:skyrim` |
| PAR-D6-2026-10-12-01 | `low` `bug` `import-pipeline` |
| PAR-D6-2026-10-12-02 | `low` `bug` `import-pipeline` (regression of #3637) |
| PAR-D2-2026-10-12-03 | `low` `bug` `nifal` `game:starfield` `safety` |
| PAR-D5-2026-10-12-02 | `low` `bug` `performance` `game:starfield` |
| PAR-D1-2026-10-12-03 | `low` `bug` `ui` `test-gap` |
| PAR-D2-2026-10-12-04 | `low` `bug` `ui` |
| PAR-D4-2026-10-12-01 | `low` `bug` `ui` `test-gap` |
| PAR-D2-2026-10-12-05 | `low` `bug` `ui` |
| PAR-D4-2026-10-12-02 | `low` `bug` `animation` `test-gap` |
| PAR-D2-2026-10-12-06 | `low` `bug` `nifal` `game:starfield` |
| PAR-D5-2026-10-12-03 | `low` `bug` `nifal` `game:starfield` |

## Summary

| Severity | NEW | Already tracked (live) |
|---|---|---|
| CRITICAL | 0 | 0 |
| HIGH | 2 | 0 |
| MEDIUM | 3 | 0 |
| LOW | 11 | 6 (#5009, #5010, #5011, #5073, #5329, #5446) |

---

## Re-run at HEAD 5d31fd637

**HEAD**: 5d31fd637 · **Baseline**: this report (HEAD c1a99c25e) · **Audited**: delta `c1a99c25e..HEAD` over the parser Paths: `fa2af0e89` (#5660/#5073), `eca0238e7` (#5659), `9707cb367` (#5662), `5dce486b6` (#5658), `0d698d16c` (#5661), `c961653f4` (#5284), `a7e05dd7b` (#5650, source-scan pins) and the `.github/workflows/real-data-gates.yml` hunk of `fa2af0e89`. These are the fixes for five of this report's own findings, so the work was to verify each fix in place and read the new code for residuals. **Unchanged since baseline (skimmed)**: `crates/bgsm/`, `crates/facegen/`, `crates/game-detect/`, `crates/sfmaterial/`, `crates/bsa/src/{ba2,csg,uvd,naming,read_at}.rs`, `crates/menuxml/src/{parse,font}.rs`, `asset_provider/{archive,texture}.rs`. No commits touch them. Dims 3 and 6 are therefore unchanged.

**Result: 0 CRITICAL, 0 HIGH, 1 MEDIUM, 1 LOW new.** The MEDIUM is a residual of closed #5662. The LOW is a residual of the #5658 fix.

### Verification of the fixes landed since c1a99c25e

| Finding | Issue | State at 5d31fd637 |
|---|---|---|
| PAR-D1-2026-10-12-01 menu texture cache unbounded | #5658 (closed) | **Fixed.** `MAX_DECODED_TEXTURE_BYTES` 1 GiB is charged from the DDS header via `Rgba8::decoded_len` (shares `dds_dimensions` with the decoder) before any allocation. Decodes are keyed by resolved archive path, so the `Menus\` / `Menus80\` / `textures\menus\` spellings share one decode. `decoded_textures_stop_at_the_renderer_budget` and `spellings_of_one_menu_texture_share_one_decode` pass. Residual: PAR-D1-2026-10-12-04 below. Font atlases are outside the budget but bounded by the 5-slot per-game font table. |
| PAR-D1-2026-10-12-02 HKX `mask_size` gate | #5659 (closed) | **Fixed.** `mask_size != pad(4*transform_count + float_count, 4)` stays an exact tie to the file's own counts. `decode_spline_animation_accepts_a_padded_mask_table` uses float counts 1–5, and `..._rejects_a_mask_size_off_the_padded_value` pins the exact tie. These are the first HKX gate fixtures with a non-zero `float_count`, which closes the clause PAR-D1-2026-10-12-02 added to #5446. The rest of #5446 is unchanged. |
| PAR-D2-2026-10-12-01 empty compressed BSA record | #5660 / #5073 (closed) | **Fixed.** `BsaArchive::extract` answers `compressed_len == 0` before any decoder. A declared size of 0 gives `Ok(empty)`, a non-zero size gives `InvalidData` naming the entry. The 4-byte placeholder test covers v103, v104 and v105. `inflate_bounded_zlib` also pins the truncated-stream and empty-stream decoder semantics in the default lane. The CI `parsers` and `nif` loops now run `--no-fail-fast`. **Strict lane re-run: `oblivion_all_bsas_v103_brute_force_extract_zero_errors` passes (21.8 s, release).** |
| PAR-D2-2026-10-12-02 one NaN channel rejects the HKX clip | #5662 (closed) | **Fixed for non-finite *stored* scalars.** The NaN track is dropped, its bytes are still consumed, the bone is reported in `HkxAnimation::dropped_bones`, and `convert_hkx_clip` warns with the clip path. A clip whose every track is flagged is still `Err`. Strict HKX census: **6,126 clips, 0 rejected, 59 decoded with a dropped track.** Residual: PAR-D2-2026-10-12-07 below (the finite-garbage half of the original title is not addressed). |
| PAR-D5-2026-10-12-01 `has_opacity == None` | #5661 (**still open**) | **Fixed in code** by `0d698d16c`: the alpha test needs `has_opacity == Some(true)`. `alpha_test_requires_an_authored_has_opacity_true` pins `None` and `Some(false)` as not alpha-tested. The loose `.mat` decoder sets `has_opacity` from the authored `"HasOpacity"`, so a CK-authored `.mat` is unaffected. The commit is titled `fix(material): …` and carries no `Fix #5661`, so the tracker stays open: close it on the commit (publish hygiene, not a code finding). |
| `c961653f4` (#5284) | n/a | `resolve_through_cdb` is behaviour-preserving. The `.mat` arm still records `ImportedTextureSource::Mat`, and the sidecar-miss arms now trace. The `the_cdb_resolve_tail_has_one_home` pin discriminates (count == 1 on production text via `strip_test_modules`). `source` is used only as `is_some()` and the `Mat` variant is hard-coded, which is a nit and not filed. |

**Tests run (toolchain 1.96.0):** the 7 parser crates are green (bsa 114, bgsm 33, sfmaterial 41 + 6, hkx 29, facegen 31, menuxml 52, game-detect 59). `cargo test -p byroredux --bin byroredux asset_provider` gives 231 passed and 10 ignored (+1 over the baseline run, the #5284 pin). Strict real-data runs: bsa Oblivion brute force (above) and the HKX suite (3/3 passed, including the census and the LE/SE parity test).

**Superseded notes in the baseline report.** PAR-D4-2026-10-12-02 (#5671) is half-moot: `KNOWN_VANILLA_REJECTS` is now 0, so the `<=` assertion is effectively exact. The new `partial <= 59` clause is one-sided, and the set-not-count and inert block-clause points still stand.

### PAR-D2-2026-10-12-07: #5662's policy ("a non-finite scalar drops its track") is bypassed by finite bounds whose span overflows, so `decode_spline_animation` returns `Ok` with NaN/inf translation or scale samples
- **Severity**: MEDIUM
- **Dimension**: Error Semantics
- **Location**: `crates/hkx/src/animation.rs:855-856` (`values.push(min + quantized * (max - min))` in `read_vector_curve`). The finite test is `:822` and covers only the stored `min` and `max`. Consumer: `byroredux/src/asset_provider/animation.rs:403-487` (`convert_hkx_clip`) copies the samples into `AnimationClip` keys without a finiteness check.
- **Status**: NEW. It is the residual of closed #5662, whose title's second half (finite garbage on the same bones "passes unchecked") was left undecided by the fix. Searches of open and closed issues for "HKX non-finite" and "spline overflow" found nothing else.
- **Trigger Input**: a dynamic translation or scale component with finite bounds `min = -3.0e38`, `max = +3.0e38` (both pass `is_finite()` and `min <= max`). `max - min` overflows to `+inf`, so a control point with quantized value 0 evaluates `0 * inf = NaN` and one with value 1 evaluates `inf`.
- **Description**:
  - #5662 made non-finite stored scalars drop the track instead of failing the clip. The sampled value is never checked, so the same artifact reaches the clip by arithmetic.
  - The original report counted 12 *accepted* vanilla clips with finite garbage up to `|T| = 1.41e22` on the same prop-attachment bones. They are still accepted, and nothing downstream sanitises a key value. The only `is_finite` guards in `crates/core/src/animation/` are on phase and time.
  - This is a mod-file hazard: vanilla bounds do not overflow.
- **Evidence**: probe crate `/tmp/audit/parsers/probe-hkx`, a copy of `crates/hkx/src` (real-data tests cut) with one added test built on the repo's `synthetic_clip` / `dynamic_x_translation` fixtures:
  ```
  let curves = dynamic_x_translation((-3.0e38, 3.0e38));
  decode_spline_animation(&synthetic_clip(0, 4, &[[0, 0x10, 0, 0]], &curves))
  -> Ok, dropped=[], translation=[NaN, 0.0, 0.0] on both frames
  ```
- **Impact**: a hostile or corrupt mod clip puts NaN translation or scale into `AnimationClip`, then `Transform`, then the palette and TLAS instance. It affects only that clip while it plays. Downstream GPU behaviour of a NaN bone was not exercised, so it is not rated higher.
- **Related**: #5662, #3013, #4396 (same class: finite components overflowing to a bad result), PAR-D1-2026-10-12-02.
- **Suggested Fix**: after the `Dynamic` control points are pushed, mark the track `non_finite` if any value is non-finite (or compute `max - min` in `f64`). Optionally clamp `|T|` to a sane bound for the 12 vanilla finite-garbage clips. Add a fixture using the `dynamic_x_translation((-3.0e38, 3.0e38))` shape, since the existing `non_finite_*` tests only use NaN and inf stored scalars.

### PAR-D1-2026-10-12-04: an over-budget menu texture is refused after the archive fetch, is not remembered by resolved path, and the warning overstates what is refused
- **Severity**: LOW
- **Dimension**: Size Discipline
- **Location**: `crates/menuxml/src/menu.rs:494-519` (`MenuRenderer::texture`, the #5658 budget arm)
- **Status**: NEW (residual of the #5658 fix).
- **Trigger Input**: a menu naming K spellings (`Menus\x.dds`, `Menus80\x.dds`, `textures\menus\x.dds`, …) of one 8192² archive file after the budget is spent.
- **Description**:
  - `assets.texture(path)` runs before the budget check, so every refusal pays a full archive extract of the offending file. The refusal is cached only under the authored spelling (`tex_cache`), and the `decoded` map is keyed only on successful decodes. Each additional spelling of a *refused* file therefore re-extracts it (about 33 MB inflated per 8192² DXT1). The cost is transient and bounded by `MAX_DOCUMENT_TILES` (16,384) once per spelling, with no retained memory. It is a first-render CPU stall.
  - The log text says "it and any further new textures are not drawn". The check is per texture (`decoded_bytes + len > budget`), so a later, smaller texture that still fits is decoded and drawn. Which textures draw depends on first-visible order. The tests use equal-size textures and cannot tell.
- **Evidence**: code reading, `menu.rs:494-519`, with `self.decoded` populated only at `:521-525`.
- **Impact**: first-render hitch on a hostile mod menu, plus a misleading operator message. No memory growth.
- **Related**: #5658, #5667 (BC paths still allocate before checking the payload).
- **Suggested Fix**: record refused resolved paths in a small `HashSet<String>` consulted before the fetch, and reword the warning to "this texture is not drawn; later textures that fit the budget still are".

### Gate Matrix deltas

| Crate / format | Change since c1a99c25e |
|---|---|
| bsa: BSA | Empty-record policy decided before the decoder, with default-lane decoder-semantics pins. Strict Oblivion lane green again. CI loop is `--no-fail-fast`. |
| hkx | `mask_size` gate padded (exact tie kept). Non-finite scalar drops its track (`dropped_bones`). Census: 0 rejects, 59 partial. Open: PAR-D2-2026-10-12-07. |
| menuxml (`tex.rs` + loader) | Total decoded budget 1 GiB with a shared decode per resolved path, charged from the header. Open: PAR-D1-2026-10-12-04. |
| CDB merge | `HasOpacity` default is `false`. One CDB resolve tail (`resolve_through_cdb`). |

### Publishing

`/audit-publish docs/audits/AUDIT_PARSERS_2026-10-12.md` publishes only findings that are not already filed. Of the earlier findings, #5658, #5659, #5660/#5073 and #5662 are closed and #5661 is open although fixed.

| Finding | Suggested labels |
|---|---|
| PAR-D2-2026-10-12-07 | `medium` `bug` `animation` `game:skyrim` |
| PAR-D1-2026-10-12-04 | `low` `bug` `ui` |
