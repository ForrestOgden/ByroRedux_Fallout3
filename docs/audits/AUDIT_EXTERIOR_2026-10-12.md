# Exterior Audit (EXAL / SKYAL / WATAL / Ground Cover / LOD) — 2026-10-12

**HEAD**: 5d31fd637 · **Baseline**: [AUDIT_EXTERIOR_2026-10-09.md](AUDIT_EXTERIOR_2026-10-09.md) (HEAD `3bcf6c8e8`, 264 commits since) · **Audited**: Dim 1 (EXAL boundary), Dim 3 (ground cover, selection bake + generated header), Dim 4 (sky / weather, delta only), Dim 5 (WATAL), Dim 6 (distant LOD) · **Unchanged since baseline (skimmed)**: Dim 2 (terrain — one comment-only commit `3efd17e5b`), Dim 7 (gates / harness — no commits)

**Method.** This run is part of `/audit-suite --preset comprehensive`.
- One auditor did every dimension synchronously. No sub-agents. No engine launch, smoke script or capture.
- Delta scope: `git log 3bcf6c8e8..HEAD -- <Paths>` per dimension. The exterior-relevant commits:
  - `c140cb48e` (#5584, `env_translate` split into `lod` / `climate` / `water` / `sky_weather` / `fallback`).
  - `ad280e17c` (#5344, JNAM clamp), `2fd7ee270` (#5529, climate override once per grid cell).
  - `2ca57c4bc` (#5496, LOD rings keyed on the PNAM "Use LOD Data" ancestor), `a9577fd69` (#5520), `5a549e664` (#5518).
  - `e38ff49d2` (#5334/#5335/#5337/#5339) and `310723850` (#5289), the distant-water rebuild.
  - `7cf2bd086` (#5582, water-wave tuning in core), `cff1f9c42` (#5354), `c28571563` (#5548), `46e41fc79` (#5562).
  - `1505b4404` (#4906 half, ground-cover union accept), `fff5ed9a9` (#5620, generated workgroup / SH header).
- Read-only censuses (scripts in `/tmp/audit/exterior/scripts/`):
  - WRLD `NAM0` / `NAM9` / `PNAM` / `WNAM` across `Skyrim.esm`, `Update.esm`, `Dawnguard.esm`, `Dragonborn.esm`, `Fallout4.esm`, `DLCCoast.esm`.
  - BSA / BA2 name tables of the baked LOD files (`strings` over `Skyrim - Meshes0/1.bsa`, `DLCCoast - Main.ba2`).
  - `lodsettings\<ws>.lod` descriptors extracted with the existing `bsa_extract_one` example.

**Test state (green at HEAD, rustc 1.96.0):**

| Suite | Result |
|---|---|
| `cargo test -p byroredux --bin byroredux -- env_translate env_health terrain groundcover weather water lod resident_vwd sky spawner climate legacy_lod default_weather` | 592 passed, 0 failed, 21 ignored (game data / device) |
| `cargo test -p byroredux-renderer --lib -- groundcover sky_ water terrain memory_budget shader_constants` | 254 passed, 0 failed, 1 ignored (Vulkan prefilter) |
| `scripts/check-shader-artifacts.sh` (`TMPDIR=/tmp/audit/exterior`) | 36 shaders + early-test variant byte-identical to the committed `.spv` (glslang 16.4.0). Confirms #5582 / #5620 moved no value. |

## Executive Summary

**4 findings, all NEW: 0 CRITICAL · 1 HIGH · 2 MEDIUM · 1 LOW.**

The three code findings are one family: the distant-LOD **grid origin and file keys** are resolved from the wrong worldspace or the wrong source. The green tests cannot see it because they never run Tamriel's lattice against the shipped file names.

- **D6-01 (HIGH).** `worldspace_lod_grid_origin` anchors the baked `.btr` / `.bto` lattice on WRLD `NAM0`'s minimum cell. That equals the authored `lodsettings\<ws>.lod` SW cell for 11 of 12 vanilla Skyrim worldspaces and is **wrong for Tamriel**: `NAM0` = (-57, -43), the `.lod` and every `tamriel.<L>.*.btr/.bto` name sit on (-96, -96), a lattice of absolute multiples of each level. A simulation of the engine's quadtree against the real name table gives 0 of 960 lattice hits with the NAM0 origin and 1024 of 1024 with the `.lod` origin. Skyrim's main worldspace gets no baked distant terrain and no distant objects, silently. This is the same defect class as #2586, whose fix generalised from the DLC worlds.
- **D6-02 / D6-03 (MEDIUM).** #5496 (closed) keyed the rings on the PNAM "Use LOD Data" ancestor. Two of its sites were missed: the grid origin (still the child's) and the object ring's mesh / atlas paths in `spawn_object_lod_quad` (still the child's). Selection finds the ancestor's quads and the spawn then extracts nothing.
- **D1-01 (LOW).** Doc rot after the #5584 split and the 10-10..10-12 fix wave.

Everything else in the delta was re-read and holds. The distant-water rebuild, the #5520 tie-break, the #5518 gate, the #5529 per-cell climate cache, the #5582 CPU/GPU wave mirror, the #5344 clamp and the #4906 union accept are correct. The #5334 rebuild cost is already filed as PERF-D7-2026-10-12-01 and is not re-reported.

### Verdict per tier invariant

| Invariant | Verdict |
|---|---|
| **single-boundary** | **Holds for environment translation.** One producer each for sky, weather, cell lighting, climate, water and distant water. The #5584 split kept every caller on `crate::env_translate::…`. **Dented for the LOD key:** "which worldspace do the distant rings live on" is decided in `lod_worldspace_key()` but not honoured at the origin and the object spawn (D6-02, D6-03). |
| **no-fabrication** | **Dented once.** The lattice origin comes from `NAM0`, a stand-in for the authored `lodsettings\*.lod` descriptor that no code reads. The stand-in matches 11 of 12 worldspaces and fabricates the wrong value for the largest one (D6-01). |
| **no-leak** | **Holds.** No new parsed-but-unconsumed field. `lodsettings\*.lod` is an unparsed source (D6-01), not a leak. |
| **no-render-time-fallback** | **Holds.** `grep GameKind` in `env_translate/`, `groundcover_translate.rs`, `streaming*`, `cell_loader/*lod*` shows only table-shaped matches. The sky, cloud and ground-cover GLSL carry game names only in comments. |

## Per-Category Matrix

✓ = holds, ~ = dented (see the Findings column).

| Category (boundary fn) | single-boundary | no-fabrication | no-leak | no-render-time-fallback | Findings |
|---|:---:|:---:|:---:|:---:|---|
| **Terrain / splatting** (`spawn_terrain_mesh` + `build_cell_splat_layers`) | ✓ | ✓ | ✓ | ✓ | none (unchanged) |
| **Sky** (`translate_sky`, `pack_sky_dome`) | ✓ | ✓ | ✓ | ✓ | none |
| **Weather / sun / image space** (`translate_weather`, `resolve_default_weather`) | ✓ (#5344: seed and sampler clamp one way) | ✓ | ✓ | ✓ | none |
| **Climate** (`resolve_exterior_climate`, `resolve_cell_climate`) | ✓ | ✓ | ✓ | ✓ | none (#5529 caches per grid cell) |
| **Water** (`resolve_water_material`, `resolve_distant_water`, `attach_mesh_water`) | ✓ (two composition sites only) | ✓ | ✓ | ✓ | none new; PERF-D7-2026-10-12-01 (cost) |
| **Ground cover** (`groundcover_translate.rs`, `resolve_authored_cover`, `selection_weights_and_accept_rate`) | ✓ | ~ #4906 (open, per-lane LTEX→GNAM) | ✓ | ✓ | none new |
| **Distant LOD** (`select_lod_quads` / `select_authored_lod_quads`, `terrain_lod_layout`, `object_lod_scheme`) | ~ D6-02, D6-03 | ~ D6-01 | ✓ | ✓ | D6-01, D6-02, D6-03 |

## Findings

### EXT-D6-2026-10-12-01: The baked-LOD lattice origin is `NAM0`'s minimum cell, but Tamriel's quads sit on the `lodsettings\tamriel.lod` SW cell (-96, -96). Every `tamriel.<L>.*.btr/.bto` probe misses
- **Severity**: HIGH. The flagship Skyrim worldspace loses all baked distant terrain and objects, silently. Terrain falls back to the synthesized ring.
- **Dimension**: Distant LOD and trees
- **Location**: `byroredux/src/cell_loader/lod_support.rs:115-143` (`worldspace_lod_grid_origin`). Consumers: `byroredux/src/streaming_helpers.rs:135` → `LodReconcileInput::lod_grid_origin` → `lod_bands.rs:346` (`quad_origin`), `terrain_lod.rs:472`, `object_lod.rs:185`. Tests: `lod_support.rs:585-645` (no Tamriel case).
- **Status**: NEW. Regression of #2586 (closed): its fix took `NAM0` as the origin source, which is true for the DLC worlds it was measured on but not for Tamriel.
- **Tier Violated**: no-fabrication (a stand-in for an authored descriptor no code reads)
- **Game Affected**: Skyrim SE (likely LE). FO4 spot-checked, not affected (see Evidence).
- **Description**:
  - `worldspace_lod_grid_origin` returns `usable_cell_bounds().min`, i.e. `floor(NAM0 / 4096)`, for Skyrim+ / FO4. `quad_origin` then tiles `origin + k * level`.
  - The shipped files tile from the SW cell of `lodsettings\<ws>.lod` (16 bytes: `i16 x, i16 y, u16 stride, 0, u16 4, 0`). For 11 of 12 vanilla worldspaces that equals the `NAM0` minimum. Tamriel is the exception.
  - With origin (-57, -43) the engine requests `tamriel.32.-57.-43`, `tamriel.32.-25.-43`, … which do not exist. Every level (4 / 8 / 16 / 32) misses, so the descent subdivides to level 4 and misses there too. Objects have no fallback (`ObjectLodBlock::empty()`). Terrain is rescued only by the synthesized block, which has no authored colour.
- **Evidence** (read-only; `scripts/wrld_census.py`, `bsa_extract_one`, `strings` over the BSA name tables):

  | Worldspace | `NAM0` min (cells) | `lodsettings\*.lod` SW | Match |
  |---|---|---|:---:|
  | **Tamriel** | **(-57, -43)** (raw -233472, -176128) | **(-96, -96)** | **no** |
  | DLC2SolstheimWorld | (-64, -64) | (-64, -64) | yes |
  | DLC2ApocryphaWorld | (-50, -50) | (-50, -50) | yes |
  | DLC01SoulCairn | (-52, -51) | (-52, -51) | yes |
  | DLC01FalmerValley | (-16, -13) | (-16, -13) | yes |
  | Blackreach / Skuldafn / Japhet's Folly / Hunter HQ / Sovngarde / Markarth / Deepwood | (-23,-9) / (0,-21) / (-9,-6) / (-33,-9) / (-24,-32) / (-48,-3) / (-39,-1) | identical | yes |

  - All 3,060 distinct `tamriel.{4,8,16,32}.*.btr/.bto` names are `(x mod L, y mod L) == (0, 0)` for every level, minimum -96.
  - Simulation of the engine lattice inside Tamriel's bounds against that name set: origin (-57, -43) gives **0 of 960** hits. Origin (-96, -96) gives **1024 of 1024**.
  - FO4 `Commonwealth` `NAM0` min is (-96, -96), a multiple of 32, so it aligns. `DLC03FarHarbor` `NAM0` (-73, -59) matches its level-16 names `-25.-11` (mod 16 = 7, 5). Neither is affected.
  - No code reads `lodsettings\*.lod` (`grep -ri lodsettings byroredux/src crates --include='*.rs'` finds only a comment and the `probe_lod_corpus` counter).
- **Impact**: In Skyrim Tamriel, distant mountains, buildings and terrain colour never come from `.btr` / `.bto`. Terrain draws as the heightmap-synth ring only. No log line. Smoke gates check overlap and coverage, not baked-quad presence, so nothing fails. Not confirmed at runtime (no engine launch allowed this run). The arithmetic is exact against the shipped name table.
- **Related**: #2586 (closed, this finding's parent), #5222 (the FO3 analogue: a derived origin that reproduces nothing), D6-02.
- **Suggested Fix**:
  - Read the descriptor. Parse `lodsettings\<lod_worldspace_key>.lod` from the opened archives (16 bytes: SW x, SW y, stride, min and max level). Make it the origin and the band floor / ceiling. Fall back to `NAM0` only when the file is absent.
  - Pin it with a real-data `--ignored` test: Tamriel's origin must equal (-96, -96) and an on-disk `tamriel.32.0.0.btr` must be selectable.

### EXT-D6-2026-10-12-02: #5496 keyed the band bounds on the LOD source worldspace but left `worldspace_lod_grid_origin` on the child. A PNAM `0x02` child tiles the parent's quads from its own `NAM0`
- **Severity**: MEDIUM. Visible content missing in the very cases #5496 set out to fix (walled Skyrim hold cities, the FO3 roof worlds).
- **Dimension**: Distant LOD and trees
- **Location**: `byroredux/src/cell_loader/lod_support.rs:123,132` (`.get(&wctx.worldspace_key)` in `worldspace_lod_grid_origin`). The sibling `worldspace_cell_bounds` (`:158`) was switched to `lod_worldspace_key()` in `2ca57c4bc`.
- **Status**: Regression of #5496 (closed, incomplete fix)
- **Tier Violated**: single-boundary (the LOD key is decided in `lod_worldspace_key()` and ignored here)
- **Game Affected**: Skyrim (7 bit-setting children), FO4 (3), FO3 (4 roof / vista children; those use the authored-quad index, so unaffected)
- **Description**:
  - A child that sets the PNAM `0x02` bit ships no LOD of its own; its rings read the ancestor's files (`lod_worldspace_key()`).
  - The ancestor's quads sit on the ancestor's lattice, but the origin passed to `quad_origin` / `block_index` is the child's `NAM0` minimum. The two agree only by chance.
- **Evidence** (Skyrim, `PNAM & 2` children, `NAM0` minimum cell vs Tamriel's lattice of multiples of the level):

  | Child | `NAM0` min | Aligned to Tamriel's lattice at level 4? |
  |---|---|:---:|
  | WindhelmWorld | (-1, -1) | no |
  | RiftenWorld | (-1, -27) | no |
  | WhiterunWorld | (-3, -12) | no |
  | SolitudeWorld | (-19, -3) | no |
  | WhiterunDragonsreachWorld | (-1, -4) | no |
  | KatariahWorld | (-13, 0) | no |
  | WindhelmPitWorldspace | (0, 0) | yes (by chance) |

  - Even with D6-01 fixed, 6 of 7 children probe names that do not exist.
- **Impact**: The `.btr` ring finds no baked asset for the ancestor's terrain and falls back to the synthesized ring over the child's own cells. The `.bto` ring draws nothing. #5496's stated outcome ("draw Wasteland's / Tamriel's distant terrain and buildings") holds for the FO3 legacy family (index-driven, no origin) and is incomplete for Skyrim and FO4.
- **Related**: #5496, D6-01 (same function), D6-03.
- **Suggested Fix**: Resolve the origin on `wctx.lod_worldspace_key()` (the `.lod` descriptor of D6-01, else its `NAM0`). Add a synthetic-chain test: a child with a different `NAM0` returns the parent's origin.

### EXT-D6-2026-10-12-03: `spawn_object_lod_quad` extracts the mesh and atlas from the child's path. #5496 changed only the selection, so inherited object quads are chosen and then fail to load
- **Severity**: MEDIUM
- **Dimension**: Distant LOD and trees
- **Location**: `byroredux/src/cell_loader/object_lod.rs:433` (`object_lod_archive_path(scheme, &wctx.worldspace_key, …)`) and `:466` (`object_lod_atlas_path(scheme, &wctx.worldspace_key)`). Selection already uses `lod_worldspace_key()` at `:222` and `:240`.
- **Status**: Regression of #5496 (closed, incomplete fix)
- **Tier Violated**: single-boundary
- **Game Affected**: FO3 (the four roof / vista children), Skyrim, FO4 (every PNAM `0x02` child)
- **Description**:
  - For the legacy family the ring picks `Wasteland`'s authored quads from `LegacyLodQuadIndex::objects_for(lod_worldspace_key())`. For `.bto` it probes `has_mesh` on the ancestor's path.
  - `spawn_object_lod_quad` then calls `extract_mesh("meshes\landscape\lod\statesmanroofworld\blocks\…")`, which does not exist (the commit message itself says the children author "no `meshes\terrain\<world>` folder"). The result is `None`, and the caller records `ObjectLodBlock::empty()`.
  - The atlas path (`textures\terrain\<child>\objects\<child>.objects.dds`) has the same keying error.
- **Evidence**: `git show 2ca57c4bc -- byroredux/src/cell_loader/object_lod.rs` touches only the two selection lines. The commit's claim "the object archive paths … resolve on it" is not true of the spawn path. Pins (`lod_source_worldspace` tests and `installed_fo3_statesman_roof_inherits_wasteland_lod`) exercise selection and the index, not the spawn.
- **Impact**: Distant buildings stay missing for the inheriting children, which is the visible defect #5496 was filed for. The empty sentinel is cached, so nothing retries. The terrain half is unaffected (`terrain_lod.rs:462` uses the LOD key throughout).
- **Related**: #5496, D6-02. `placement_lod.rs:452` (Oblivion per-cell `.lod`) also keys on the child; children are stamped with the LOD bit at parse. Whether any `distantlod\<child>_*.lod` ships was not checked, so it is noted, not filed.
- **Suggested Fix**: Pass `wctx.lod_worldspace_key()` at both sites. Add a source-shape pin that no `wctx.worldspace_key` appears in `object_lod.rs` production text outside the full-detail masks, via `source_scan::production_text`.

### EXT-D1-2026-10-12-01: Doc rot after the #5584 split and the LOD fix wave
- **Severity**: LOW (doc-rot)
- **Dimension**: EXAL boundary discipline (docs)
- **Location**:
  - `byroredux/src/env_translate/mod.rs:15-22`: "Step 1 (this slice) establishes the module and gathers the two already-single-site **water** translates here verbatim … Later steps fold in the scattered sky / sun / weather producers". They have long been folded in.
  - `ROADMAP.md:448`: the size tripwire still lists `byroredux/src/env_translate.rs` at 5,826 lines. The largest piece is now `env_translate/water.rs` at 2,487.
  - `docs/engine/exal.md` §5: no text for #5496 (PNAM `0x02` "Use LOD Data" owns no quads; every ring key resolves on `lod_source_worldspace`) and none for the authored `lodsettings\<ws>.lod` descriptor (D6-01).
- **Status**: NEW
- **Tier Violated**: n/a
- **Game Affected**: all (docs)
- **Suggested Fix**: Rewrite the `mod.rs` header as the five-submodule layout it already lists below it. Refresh the ROADMAP tripwire at the next `/session-close`. Add one paragraph to exal.md §5 for the inherited-LOD key and the `.lod` descriptor once D6-01 lands.

## Findings count

| Dimension | CRITICAL | HIGH | MEDIUM | LOW |
|---|---|---|---|---|
| D1 EXAL boundary | — | — | — | 1 |
| D2 Terrain (skimmed) | — | — | — | — |
| D3 Ground cover | — | — | — | — |
| D4 Sky / weather / sun | — | — | — | — |
| D5 WATAL | — | — | — | — |
| D6 LOD / trees | — | 1 | 2 | — |
| D7 Gates / harness (skimmed) | — | — | — | — |
| **Total NEW** | **0** | **1** | **2** | **1** |

## Dimension notes (no findings)

- **Dim 1.**
  - `translate_sky`, `translate_weather`, `translate_exterior_cell_lighting` and `procedural_fallback_*` have callers only in `scene/world_setup.rs`, `systems/weather.rs` and `cornell.rs`. Every `SkyParamsRes {` / `WeatherDataRes {` literal outside `env_translate/` and `components.rs` is a `#[cfg(test)]` fixture or `systems/weather.rs`' destructure.
  - `GameKind` appears only as table-shaped matches (`terrain_lod_layout`, `object_lod_scheme`, `grass_grid_spacing`, `btr_normal_path_candidates`) and the Oblivion arm of `default_water_for_worldspace` / `resolve_exterior_climate`, whose inherit-all is the parse-time PNAM stamp.
  - #5344: `translate_weather` now carries `[0, 1]` alphas, so the seed and `sample_weather_sky` agree. The new test runs the real sampler.
  - #5529: `claim_climate_evaluation` is a pure once-per-grid-cell cache over data fixed for the streaming state. A failed resolve is not retried and not re-warned.
- **Dim 3.**
  - `selection_weights_and_accept_rate` re-read, including edge cases (no share, NaN, over-range density). The union accept rate and the share-weighted mix are correct. `density_normalized_by_share` has no stale reference.
  - #5620 moves only constants, with byte-identical SPIR-V. The `groundcover_*` GLSL carries game names only in comments.
- **Dim 4.**
  - Only #5344, #5584, #5620, the source-scan routing and a lerp helper touched it. No sky / cloud / SH logic changed.
- **Dim 5.**
  - #5582: `water.vert` reads every `WATER_WAVE_*` from the generated header. The CPU crest sampler (`authored_wave_height_with_weather`) mirrors it, including the amplitude clamp after the wind scale and `abs(frequency)`. The 6.2831853 literals are τ.
  - Only two production `WaterMaterial {` composition sites remain (`resolve_water_material`, `material_translate.rs`). The `commands/water.rs` and `systems/water.rs` literals are `#[cfg(test)]`.
  - Every CPU sampler goes through `WaterKind::weather_scroll_for` (renderer upload, camera submersion, buoyancy, player).
  - #5334 / #5289: the hole is `loaded.contains_key`, rebuilds are coalesced to 250 ms, and the dirty flag survives a throttled step. The blocking-upload cost is PERF-D7-2026-10-12-01 (`Existing`).
- **Dim 6, other.**
  - #5520: `prune_same_level_overlaps` breaks count ties on `lod_source_cell_exists` (LOD key), then residue order. #5518: `should_scan_legacy_lod_quads` keys on either family table. Both are correct and covered by passing tests.

## Dedup sources

- `/tmp/audit/issues.json` (122 open): no open issue on LOD origin, `lodsettings`, `lod_worldspace_key`, or `spawn_object_lod_quad`.
- `gh issue list --state all` searches: "LOD origin Tamriel", "lodsettings", "grid origin LOD", "quad origin". Hits are #2586 (closed, parent of D6-01), #5222 (FO3 analogue, closed), #5496 (closed, parent of D6-02 / D6-03), #3358 / #2371 (unrelated).
- Reports before 2026-06-07 not relied on. `/tmp/audit/REPORTED_THIS_SUITE.md` re-read before finalizing: PERF-D7-2026-10-12-01 (distant-water rebuild) is referenced, not re-reported. No other overlap.

## Known-Open Register (dated; what this pass changed)

| Item | Status this pass |
|---|---|
| #5222, #5387, #5518, #5520, #5496, #5423, #5424 | CLOSED. #5518 / #5520 verified. **#5496 is incomplete** (D6-02, D6-03). |
| #5334, #5335, #5337, #5339, #5289 | CLOSED, verified (resident-set hole, `resolve_distant_water`, cached projection). Cost: PERF-D7-2026-10-12-01 (open as a finding, not an issue). |
| #5341 | The doc half; unchanged by this pass. |
| #5344 | CLOSED, verified. |
| #5505 (Oblivion child worldspaces draw no ground outside their own LAND) | OPEN, unchanged. |
| #5463 (TES5/FO4 `XCCM` is a REGN reference) | OPEN, unchanged. |
| #4906 (LTEX→GRAS per-lane localization), #4913 (Skyrim `.btt` trees), #3307 (VWD culling), #3301 (REGN RDAT weather) | OPEN, unchanged. |
| #5222 live FO3 / FNV visual verification | Still owed. |
| Skyrim Tamriel baked-LOD live check | **Owed.** D6-01 is a static proof; a `--bench-hold` capture with `lod.coverage` and a `.btr` count would confirm it. |

## Skill drift (for the next `/audit-exterior` edit)

- **Dim 1**: `env_translate/` is now five submodules (the skill already says so). Add `climate_tod_hours_tests.rs`. The Oblivion inherit-all is the stamp walked by `inherit_up_chain`.
- **Dim 6**: add `lod_source_worldspace` / `ExteriorWorldContext::lod_worldspace_key` (#5496) to the key discipline and a check that **every** ring key (origin, paths, atlas) resolves on it. Add the lattice rule: the origin is the `lodsettings\<ws>.lod` SW cell, not `NAM0` (Tamriel is the counter-example). Extend `probe_lod_corpus` (#4933) to count `.lod` descriptors against `NAM0`.
- **Dim 5**: `byroredux/src/streaming/mod.rs` is already in `Paths:`.

## Cross-audit routing

- **`/audit-skyrim`**: D6-01, D6-02 (Tamriel baked LOD, inheriting hold-city children).
- **`/audit-fo3`**: D6-03 (roof / vista children's object LOD).

Suggested next step: `/audit-publish docs/audits/AUDIT_EXTERIOR_2026-10-12.md`.

**Labels:**
- D6-01: `bug`, `terrain-exterior`, `high`, `game:skyrim`.
- D6-02: `bug`, `terrain-exterior`, `medium`, `game:skyrim`.
- D6-03: `bug`, `terrain-exterior`, `medium`.
- D1-01: `documentation`, `doc-rot`, `terrain-exterior`, `low`.
