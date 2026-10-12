=== #5508 ===
# SAVE-D2-2026-10-09-01: #5412's new `ActorValue.set_override` layer is folded into `current()` but missed by three readers that re-sum the layers by hand: the HUD bar fraction, the debug vitals, and the SDK actor-value projection

**Source**: `docs/audits/AUDIT_SAVE_2026-10-09.md` — finding `SAVE-D2-2026-10-09-01` · suite `/audit-suite --preset streaming-deep` (2026-10-09) · HEAD `3bcf6c8e8`

- **Severity**: MEDIUM
- **Dimension**: Format & Schema Discipline. This is a cross-domain finding, traced from the v33→v34 field. The read sites belong to `/audit-character`, `/audit-ui` and `/audit-tooling`, none of which runs in this suite.
- **Data-Loss Class**: none (the field round-trips correctly; this is read-side only)
- **Location**:
  - `byroredux/src/hud.rs:849` (`fraction()`, shared by the MenuXml and Scaleform HUD drivers): `let max = entry.base + entry.permanent_mod + entry.temporary_mod;`
  - `byroredux/src/inventory.rs:317` (`vitals_snapshot()`, the debug-UI vitals): same `max`, and `current: max - entry.damage`.
  - `byroredux/src/extensions/capture.rs:235-240` builds `ActorValueState::new(base, permanent_mod, temporary_mod, damage)`. `crates/sdk/src/actor_values.rs:14-48` has no override slot, and its `current()` is `base + permanent + temporary - damage`.
- **Status**: NEW. It is a regression introduced by the #5412 fix (`a614eb273`). Before #5412, the player `SetBase` value lived in `permanent_mod`, which all three readers include.
- **Trigger Conditions**: `setav <player> Health|AP <v>` (console `edit_av`), or an SDK `SetBase` on a player `PlayerOnly` derived pool. Only these two writers reach `set_override` (`extensions/commands.rs:446-460`).
- **Description**: #5412 added a fifth layer: `current = base + set_override + permanent + temporary − damage` (`crates/core/src/ecs/components/actor_values.rs:78`). The commit's own test shows the effect: after `setav Health 500`, the values are base 105, `set_override` 500, current 605. The three hand-composed readers then go wrong as follows:
  - **HUD**: the bar's max stays at 105, so `current/max` clamps to 1.0. The bar shows full health until damage exceeds 500 (the old ratio was 405/605 = 0.67 after 200 damage).
  - **Debug vitals**: they report 105 − damage, which goes negative while the player actually has 405.
  - **SDK**: `EntityProjection::actor_value(Health).current()` returns the pre-`SetBase` value. A mod reading back its own `SetBase` sees it as not applied.
- **Evidence**: The grep `permanent_mod\s*\+` finds exactly these sites plus `CharacterRuleset::actor_value` (`ruleset.rs:169`). That last one is correct as written, because `set_override` is written only on `PlayerOnly` pools, and that arm composes `ActorGeneral` formulas.
- **Impact**: The player's Health and AP bars, the debug vitals panel and the SDK projection are wrong after any console or SDK `SetBase` on a derived pool. The bar under-reports damage, the panel can show negative health, and the SDK value is stale. Nothing is lost from a save.
- **Related**: #5412, #5239, #4675.
- **Suggested Fix**:
  - Give `ActorValue` an undamaged-max helper (`base + set_override + permanent_mod + temporary_mod`) and use it in both HUD readers.
  - Add the layer to the SDK `ActorValueState` (or fold it into `base` at the projection boundary). Pin it with a projection test after a routed `SetBase`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (other walkers / record decoders / games / call sites)
- [ ] **TESTS**: A regression test pins this specific fix
=== #5509 ===
# SF-2026-10-09-D6-01: #5277 forwards only `BlendingMode == "AlphaBlend"` (0 vanilla shapes); the 491 shapes whose CDB material authors an additive-family mode render as opaque lit surfaces, and nifal.md has no parked row for them

**Source**: `docs/audits/AUDIT_STARFIELD_2026-10-09.md` — finding `SF-2026-10-09-D6-01` · suite `/audit-suite --preset streaming-deep` (2026-10-09) · HEAD `3bcf6c8e8`

- **Severity**: MEDIUM (the "translatable data silently dropped" row; visible artifacts on effect cards)
- **Dimension**: Material Flow (NIFAL boundary), CDB arm
- **Location**:
  - `byroredux/src/asset_provider/material/merge.rs:331-351`: the comment reads "any other authored string stays untouched until its mapping is sourced", and the gate is `:348`;
  - `docs/engine/nifal.md:710` (records only the AlphaBlend arm);
  - capture `crates/sfmaterial/src/index.rs` (`effect_blend`).
- **Status**: NEW. #5277 (closed by 9bb6b7dce) scoped AlphaBlend only. NIFAL-D8-01 (this suite) covers the inheritance gap that leaves AlphaBlend and IsGlass inert. It does not cover these modes, and they are live without inheritance. No issue mentions Starfield additive blending.
- **Description**:
  - Starfield shapes almost never carry `NiAlphaProperty` (884 blocks), so the CDB `EffectSettingsComponent.BlendingMode` string is their only blend signal.
  - 9bb6b7dce captures it for every material, but forwards it only for `"AlphaBlend"`.
  - Every other authored mode leaves `has_alpha == false`. The surface goes down the opaque, depth-writing, lit path, and its glow sprite becomes a solid card.
- **Evidence**: The probe (`/mnt/data/tmp/starfield-audit-probe`, release, MemoryMax=4G; output `/tmp/audit/starfield/probe/blend_alpha.out`) visited every `BSGeometry` in `Meshes01/02/Patch` (344,355 shapes), looked up its shader-property material in the vanilla base CDB, and crossed the result with `NiAlphaProperty`:

  | `BlendingMode` | shapes | NiAlphaProperty | samples |
  |---|---|---|---|
  | Additive | 182 | none | `shipwep_*_projectile_*` (`ShipBallisticBoltCore`, `ShipEM_SwirlGlow_01`, `ShipKineticCannonGlow`), `starborntempleintpedestal_b02` (`SBStoneTrimGlowFXOff`) |
  | SourceSoftAdditive | 277 | none | `smod_fx_enginemain_*` (`EngineGlowFlames.mat`), `spacemine.nif` (`RadialGlow_01_Red`) |
  | DestinationInvertedSoftAdditive | 29 | none | `nathemast_topwaterfall.nif` (`Water\Waterfall01.mat`) |
  | DestinationSoftAdditive / Multiply / TakeSmaller | 1 / 1 / 1 | none | `effects\debugfiles\testblendingmodes_*` |
  | **AlphaBlend** | **0** | — | — |

  All 491 shapes sit on `BSLightingShaderProperty` stubs. Raw CDB (this suite's NIFAL probe, `cdb_raw.txt`): `BlendingMode` is authored on 526 `EffectSettingsComponent`s (Additive 234, SourceSoftAdditive 283, AlphaBlend 2).
- **Impact**:
  - Ship engine flames, weapon projectile glows, space-mine glows and the New Atlantis MAST waterfall render as opaque, depth-writing cards instead of additive effects.
  - This is visible wherever ships and those set pieces appear.
- **Related**: #5277 (closed), NIFAL-D8-01 (inheritance; would add more hits), NIFAL-D8-02 (emissive on the same effect materials), #1651 / #1823 (the FO4 BGSM additive history).
- **Suggested Fix**:
  - Record the dropped modes now as a named parked row in nifal.md, with these counts.
  - Then source the mapping:
    - **Additive / Multiply.** The same studio's FO4 BGSM preset vocabulary exists locally (`/mnt/data/src/reference/Material-Editor/MaterialLib/BaseMaterialFile.cs:363-427`: Additive = (SRC_ALPHA, ONE), Multiplicative = (DEST_COLOR, ZERO)). The renderer already keys those pipelines (`crates/renderer/src/vulkan/pipeline.rs:752-755`). Confirm the cross-game carry-over against a capture before landing it.
    - **The soft variants and `TakeSmaller`.** These have no local source and stay parked until one is found.

## Completeness Checks
- [ ] **UNSAFE**: If the fix adds `unsafe`, a safety comment states the upheld invariant
- [ ] **SIBLING**: Same pattern checked in related files (other walkers / record decoders / games / call sites)
- [ ] **DROP**: If Vulkan objects change, the Drop impl is still reverse-order correct
- [ ] **CANONICAL-BOUNDARY**: per-game logic stays at the NIFAL parser→`Material` boundary (`translate_material` / `Material::resolve_pbr`) — never pushed into shaders/renderer, never re-derived at render time. See `/audit-nifal`.
- [ ] **TESTS**: A regression test pins this specific fix
=== #5520 ===
# EXT-D6-2026-10-09-01: #5387's majority-residue prune breaks 3 of its 7 multi-residue ties on an arbitrary `Reverse(residue)`, and in two of them the dropped generation covers 2 authored cells the kept one does not

**Source**: `docs/audits/AUDIT_EXTERIOR_2026-10-09.md` — finding `EXT-D6-2026-10-09-01` · suite `/audit-suite --preset streaming-deep` (2026-10-09) · HEAD `3bcf6c8e8`

- **Severity**: LOW. Two edge cells in each of two small worldspaces lose distant objects. The rule decides with no data in 3 of 7 cases.
- **Dimension**: Distant LOD and trees
- **Location**: `byroredux/src/cell_loader/lod_bands.rs:453-488` (`prune_same_level_overlaps`; the tie-break is at `:470`), called from `select_authored_lod_quads` at `:504`. Test: `same_level_leftover_generation_quads_are_pruned` (`:616`) has no tie case.
- **Status**: NEW. This is a follow-up of #5387 (closed); the fix is correct in every non-tie case.
- **Tier Violated**: no-fabrication. The tie-break is an arbitrary choice even though the selection could be anchored in data.
- **Game Affected**: Fallout 3 (FNV and both games' terrain tables are single-residue)
- **Description**:
  - The prune keeps the residue class with the most quads per level. Ties go to the smallest `(rx, ry)`, "for determinism".
  - Census of every FO3 object table with more than one residue class:
    - **washmontop**: the 13-quad majority (4,4) covers every authored cell that any class covers.
    - **dcworld01, dcworld17**: the majority covers every authored cell that any class covers.
    - **dcworld06 (3/3)**, **dcworld12 (4/4)** and **dlc02baileyscrossroads (7/7, Anchorage)**: exact ties, so the tie-break decides.
- **Evidence** (`tie_loss.py`: FO3 BSA name tables against the `XCLC` grids in `Fallout3.esm` / `Anchorage.esm`):

  | Worldspace | Kept residue → authored cells covered | Dropped residue → authored cells covered | Authored cells only the dropped set covers |
  |---|---|---|---|
  | dcworld06 (L8) | (1,5) → 69 | (1,6) → 71 | 2: (15,−3), (16,−3) |
  | dlc02baileyscrossroads (L4) | (0,2) → 67 | (0,3) → 69 | 2: (12,−8), (12,−7) |
  | dcworld12 (L8) | (2,6) → 87 | (4,6) → 87 | 0 |

- **Impact**:
  - Four edge cells across two worldspaces get no distant-object LOD.
  - The choice of which generation's geometry draws in these three worldspaces does not depend on the data.
  - Which lattice the shipped game itself loads is not established.
- **Related**: #5387, #5222 (closed); EXT-D6-2026-10-08-01.
- **Suggested Fix**:
  - Break count ties on authored-cell coverage. `wctx`'s exterior-cell set is already in hand at both ring call sites. Fall back to the residue order only when coverage also ties.
  - Add a tie fixture. Record in exal.md that which lattice the engine loads is still open.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (other walkers / record decoders / games / call sites)
- [ ] **TESTS**: A regression test pins this specific fix
=== #5522 ===
# FO4-2026-10-09-D1-03: The precombine job collapses the worker's negative cache entry into a miss and re-parses on the main thread, unguarded

**Source**: `docs/audits/AUDIT_FO4_2026-10-09.md` — finding `FO4-2026-10-09-D1-03` · suite `/audit-suite --preset streaming-deep` (2026-10-09) · HEAD `3bcf6c8e8`

- **Severity**: LOW. This is a defense-in-depth gap with zero vanilla reach (235,082 of 235,082 parse clean).
- **Dimension**: 1, exterior streaming route agreement.
- **Location**:
  - `byroredux/src/cell_loader/precombined.rs:261-264`.
  - Compare `byroredux/src/cell_loader/references/synth_child.rs:594-607` and `byroredux/src/streaming_helpers.rs:663-668`.
- **Status**: NEW.
- **Description**:
  - When the worker's pre-parse of an `_oc.nif` fails, `finish_streaming_import` caches a negative entry
    (`reg.insert(key, None)`). The failure can be a missing file, a parse error, or a parser panic caught by
    `pre_parse.rs:256`.
  - The REFR loader honours that entry as a cached miss.
  - `PrecombinedSpawnJob` reads `reg.get(&path).and_then(|opt| opt.clone())`, which turns `Some(None)` into `None`. It
    then re-runs extract, `parse_nif` and the CSG decode **on the main thread**, where there is no `catch_unwind`, and
    re-inserts the negative entry.
- **Evidence**: `NifImportRegistry::get` returns `Option<&Option<Arc<CachedNifImport>>>`
  (`nif_import_registry.rs:619`). Only the REFR path distinguishes the two `None` levels.
- **Impact**:
  - A parser panic that the worker contained (#854's contract) is replayed on the main thread and takes the engine down.
  - A parse error costs one main-thread parse every time the cell is applied.
  - The skill's checklist requires the two routes to agree on the fail-closed fallback. They agree for owner, key and
    blend, but not for the negative case.
- **Related**: #854, CONC-D7-02 (the worker's pre-parse skip memo), and NIF-D4-2026-10-09-01 (a stack overflow is not
  catchable either way).
- **Suggested Fix**: in `advance`, treat `Some(None)` as a terminal miss (`misses += 1; next_hash += 1; continue`),
  matching the REFR loader. Add a unit test that seeds a negative entry and asserts there is no re-extract.

## Completeness Checks
- [ ] **UNSAFE**: If the fix adds `unsafe`, a safety comment states the upheld invariant
- [ ] **SIBLING**: Same pattern checked in related files (other walkers / record decoders / games / call sites)
- [ ] **TESTS**: A regression test pins this specific fix
=== #5523 ===
# FO4-2026-10-09-D4-02: SCOL CM path is taken verbatim from `MODL` (114 stale DLCRobot MODLs), and the documented "absent CM → expand" fallback does not exist

**Source**: `docs/audits/AUDIT_FO4_2026-10-09.md` — finding `FO4-2026-10-09-D4-02` · suite `/audit-suite --preset streaming-deep` (2026-10-09) · HEAD `3bcf6c8e8`

- **Severity**: LOW. Today's reach is 1 + 13 unbaked placements, plus the `pc_spawned == 0` fallback path.
- **Dimension**: 4, SCOL expansion.
- **Location**:
  - `byroredux/src/cell_loader/refr.rs:689-692`: the doc claim.
  - `byroredux/src/cell_loader/refr.rs:720-724`: the `must_expand` gate.
  - The SCOL `MODL` → `statics[base].model_path` mapping (`crates/plugin/src/esm/cell/support.rs:73`).
- **Status**: NEW.
- **Description**:
  - The expansion gate only tests `model_path.is_empty()`.
  - The doc says that "vanilla SCOLs whose CM file is absent under a previsibine-bypass loadout hit the expansion branch".
    They do not. An absent CM file is an `extract_mesh` miss, and the REFR spawns nothing.
  - The authored `MODL` is also not always where the CM ships.
- **Evidence** (probes `scolderive` and `scolcheck`; Python `scol_reach2.py`, `scol_cells.py` and `lair.py`):
  - **Across all seven masters**: 3,509 SCOLs that each master defines itself. Of those, 3,375 `MODL` paths resolve.
    **114 (all DLCRobot)** carry dev-plugin paths, such as `SCOL\Gravato_DLC01SCOL.esp\CM0000F3AD.NIF` and the
    `rwisnewski_DLC01.esp` and `scornett-DLC01.esp` folders. Those paths miss, but
    `meshes\scol\dlcrobot.esm\cm<local id>.nif` resolves for all 114. 19 SCOLs resolve under neither name.
  - **Stale-MODL reach**: 279 DLCRobot REFRs place stale-MODL SCOLs: `DLC01Lair01` 259 and
    `DLC01FortHagenSatelliteArray01` 20. 278 of those are XCRI-baked, which is why the stale path is invisible in the
    shipped game. One `DLC01Lair01` placement is not baked.
  - **Absent-CM reach**: 13 REFRs place absent-CM SCOLs: DLCRobot 6, DLCCoast 1 and DLCNukaWorld 6.
- **Impact**:
  - Today the stale-MODL refs render nothing. Because of FO4-2026-10-09-D1-01 that happens to avoid a double draw.
  - Once D1-01 is fixed, the baked refs are correctly skipped. The 1 unbaked stale-MODL placement and the 13 absent-CM
    placements still render nothing.
  - If a cell's CSG is unavailable (`pc_spawned == 0`), all 279 vanish.
- **Related**: #585, #1182, and FO4-2026-10-09-D1-01.
- **Suggested Fix**:
  - On an extract miss for a SCOL base, try the derived `scol\<owner>\cm<local:08x>.nif`.
  - If that also misses, fall back to `expand_scol_placements`.
  - Correct the `refr.rs:689-692` doc.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (other walkers / record decoders / games / call sites)
- [ ] **TESTS**: A regression test pins this specific fix
=== #5524 ===
# NIF-D4-2026-10-09-02: #5389 follow-through has three gaps: its doc insertion orphaned `widen_packed_bone_indices`' doc comment, its warning mislabels the classic arm, and the FO4 `BsSkinInstance` arm it targets has no test

**Source**: `docs/audits/AUDIT_NIF_2026-10-09.md` — finding `NIF-D4-2026-10-09-02` · suite `/audit-suite --preset streaming-deep` (2026-10-09) · HEAD `3bcf6c8e8`

- **Severity**: LOW (doc rot plus a test gap. The code is correct, and the census confirms it.)
- **Dimension**: Geometry Extraction & Import Handoff
- **Game Affected**: FO4, Skyrim SE (the `BsSkinInstance` arm untested). Doc only for the rest.
- **Location**:
  - `crates/nif/src/import/mesh/skin.rs:595-611`: the original `widen_packed_bone_indices` doc, lines `:595-605` (nifly `OptimizeFor` / Draugr evidence), is now directly followed by the #5389 lines `:606-611` and attaches to the private `decline_unbounded_packed_indices` (`:612`). The public `widen_packed_bone_indices` (`:627`) is left with no doc.
  - `:618-621`: the warning says "BSTriShape skin: …" but is also emitted from `extract_skin_ni_tri_shape` (`:156`). NIFAL's 2026-10-09 report noted this as cosmetic.
  - `crates/nif/src/import/mesh/bs_tri_shape_partition_remap_tests.rs:401-560`: both new tests build a `NiSkinInstance`. There is no test for the `BsSkinInstance` arm (`skin.rs:301`, every FO4 skinned actor, which the commit message names), and none for the SSE `SseSkinGlobalBuffer` route.
- **Status**: NEW (follow-through of #5389, which is closed).
- **Description**: the decline logic is right. It covers all four lanes, including zero-weight lanes, and never clamps. The classic arm is provably unreachable: `densify_sparse_weights` emits enumerate-positions in `data.bones`, and `skin.rs:135` already enforces `data.bones.len() == bone_refs.len()`. That makes it harmless defense. The census shows no vanilla mesh trips the decline: 0 declines over 72,538 skinned meshes across FNV, FO3, Oblivion, LE, SSE and FO4, and 0 out-of-range lanes, weighted or zero-weight, in FO4's 16,343 inline skinned `BSTriShape`s.

  The gaps are only in documentation and coverage:
  - the misplaced doc block;
  - the mislabelled log;
  - a decline that is untested on the arm it was written for. A future refactor of that arm could drop the call without any test failing.
- **Impact**:
  - Rustdoc and readers attribute the packed-index-space rationale (#613/#2577) to the wrong function.
  - A future refactor of that arm could silently drop the bound. The render-side palette invariant (`render/skinned.rs`) relies on it.
- **Related**: #5389, #4268, #613, #2577.
- **Suggested Fix**:
  - Move the #5389 doc lines and the function below `widen_packed_bone_indices`, restoring its doc.
  - Make the log prefix arm-neutral, for example "skin:".
  - Add a decline test and a `bone_count - 1` boundary test that build a `BsSkinInstance` + `BsSkinBoneData` shape. Optionally add an SSE global-buffer variant.

## Completeness Checks
- [ ] **UNSAFE**: If the fix adds `unsafe`, a safety comment states the upheld invariant
- [ ] **SIBLING**: Same pattern checked in related files (other walkers / record decoders / games / call sites)
- [ ] **TESTS**: A regression test pins this specific fix
=== #5525 ===
# NIFAL-D8-2026-10-09-03: `parse_loose_mat` and `MaterialIndex::lookup` are two producers of `CdbMaterial`, and 9bb6b7dce extended only the compiled one, so 16 of the 20 installed loose `.mat` files drop their authored `EmissiveSettingsComponent`

**Source**: `docs/audits/AUDIT_NIFAL_2026-10-09.md` — finding `NIFAL-D8-2026-10-09-03` · suite `/audit-suite --preset streaming-deep` (2026-10-09) · HEAD `3bcf6c8e8`

- **Severity**: LOW. This is Creation (mod) content only, and the arm would be inert anyway until D8-01 and D8-02 are
  settled.
- **Dimension**: Shader-flags/Effects
- **Tier Violated**: single-boundary (the same authored component lowers differently depending on its container)
- **Game Affected**: Starfield (Creation archives)
- **Location**: `byroredux/src/asset_provider/material/loose_mat.rs:84-143`. The match has arms for `MRTextureFile`,
  `TextureReplacement`, `MaterialParamFloat`, `AlphaSettingsComponent`, `EffectSettingsComponent` (`IsGlass` only) and
  `Translucency`. There is no `EmissiveSettingsComponent` or `LayeredEmissivityComponent` arm, and no `BlendingMode` read.
  Compare the compiled side at `crates/sfmaterial/src/index.rs:492-539`.
- **Status**: NEW. #5396, which closed the layout defect, is verified. This is the parity gap it left once 9bb6b7dce landed
  the same evening.
- **Description**: All 20 installed loose files were extracted (to `/tmp/audit/nifal/matsamples/`):
  - All 20 decode (each carries `MRTextureFile`), so each one wins over the CDB.
  - 16 author `BSMaterial::EmissiveSettingsComponent`. These are the 11 QOG pawnshop-terminal parts and the 5 requisition-kiosk
    splash screens. None of them author `Enabled`: they inherit it from the shader model they `Import`. `ColorEmissive.mat`
    looks up with `Enabled = true`.
  - 1 (`lasersight_white.mat`) authors `LayeredEmissivityComponent` with `Enabled "true"` and a luminance of 432.
  - All of it is ignored. Their emissive maps, for example `TerminalCase_emissive.dds` at slot 7, bind at zero weight.
- **Evidence**: A JSON census of the extracted files (component-type counts: `EmissiveSettingsComponent` 16,
  `EmittanceSettings` 16, `LayeredEmissivityComponent` 1). `loose_mat.rs` has no arm for either component.
- **Impact**: Glowing terminal screens from installed Creations render unlit. The two decoders will keep drifting each time
  one of them gains a capture.
- **Related**: #5396 (closed), D8-01, D8-02, #5466 (open, the loose `.mat` doc text).
- **Suggested Fix**: Factor the per-component capture (class name plus field map → `CdbMaterial` fields) into one function
  that both decoders call. `serde_json` string-typed values and CDB `Value`s can both feed one small accessor trait. Then
  apply D8-01's inheritance rule and D8-02's emissive decision once, in that function.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (other walkers / record decoders / games / call sites)
- [ ] **CANONICAL-BOUNDARY**: per-game logic stays at the NIFAL parser→`Material` boundary (`translate_material` / `Material::resolve_pbr`) — never pushed into shaders/renderer, never re-derived at render time. See `/audit-nifal`.
- [ ] **TESTS**: A regression test pins this specific fix
=== #5526 ===
# OBL-2026-10-09-D5-01: Phase V dialogue voice never resolves on Oblivion. The lookup needs NPC `VTCK` → `VTYP` and composes the FNV `.ogg` / 10-25-truncation name, but Oblivion authors neither record and ships its voices as race\sex\*.mp3. Every…

**Source**: `docs/audits/AUDIT_OBLIVION_2026-10-09.md` — finding `OBL-2026-10-09-D5-01` · suite `/audit-suite --preset streaming-deep` (2026-10-09) · HEAD `3bcf6c8e8`

- **Severity**: LOW (enhancement). The subtitle-estimate fallback works, and no wrong behaviour results.
- **Dimension**: Gameplay & UI Data Slice. The mechanism is owned by `/audit-gameplay` / `/audit-audio`.
- **Location**:
  - `byroredux/src/systems/dialogue_voice.rs:235-246`: the voice-type resolve.
  - `byroredux/src/systems/dialogue_voice.rs:71-100`: `voice_path_candidates`, which hard-codes `.ogg`, the FNV truncation and the voice-type folder.
  - `byroredux/src/systems/dialogue_voice.rs:1-8` and `docs/engine/dialogue-trees.md:178-188`: both call the FNV shape "Bethesda's authored convention".
- **Status**: NEW.
  - `gh` searches for "Oblivion voice", "mp3 voice" and "Phase V voice" find #5433 (open, a sibling: the FO3 profile mounts no sound archives) and #5395/#5393 (closed, FO3/FNV naming).
- **Description**:
  - `Oblivion.esm` authors 0 `VTCK` on its 2,482 NPC_ and 0 `VTYP` records, so `index.voice_types.get(npc.voice_form_id)` is always `None`. The lookup returns at `debug` level.
  - The authored convention, from `Oblivion - Voices1/2.bsa` (77,780 entries: 38,898 `.mp3` and 38,882 `.lip`), is `sound\voice\oblivion.esm\<voice race full name, lowercased>\<m|f>\<quest EDID>_<topic EDID>_<formid 8-hex>_<n>.mp3`. Example: `…\high elf\f\dark17following_greeting_0000566f_1.mp3`.
    - The race folders are argonian, breton, dremora, high elf, imperial, nord and redguard. Other races borrow a folder through RACE `VNAM` voice races. That field is already decoded for Oblivion as `RaceRecord::voice_forms` (`race.rs:591`) but has no consumer.
    - Names are **not** truncated: quests run up to 22 characters, and quest + topic up to 46.
- **Impact**:
  - No Oblivion line is voiced, including the generic greetings that every activation now opens.
  - The module doc presents the FNV rule as universal, so an Oblivion implementer gets no warning.
- **Related**: #5367 Phase V, #5433, #5395, #5393, OBL-2026-10-08-D2-01 / #5397 (Phase G liveness).
- **Suggested Fix**:
  - Add a per-game voice-path table entry for Oblivion: the speaker's race, mapped through `RACE.VNAM` (by sex) to the voice race's FULL name; the `m`/`f` folder; no truncation; `.mp3`.
  - Until then, record Oblivion as out of scope in `dialogue_voice.rs` and dialogue-trees.md.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (other walkers / record decoders / games / call sites)
- [ ] **TESTS**: A regression test pins this specific fix
=== #5530 ===
# PHYS-D2-2026-10-09-02: `accept_keyframe_target` gates only `push_kinematic`; newcomer registration and `set_kinematic_translation` place Rapier bodies at unchecked translations

**Source**: `docs/audits/AUDIT_PHYSICS_2026-10-09.md` — finding `PHYS-D2-2026-10-09-02` · suite `/audit-suite --preset streaming-deep` (2026-10-09) · HEAD `3bcf6c8e8`

- **Severity**: LOW. A hardening gap in the same panic class. No content evidence: the field panic did not come
  through these entries.
- **Dimension**: Step & Sync (streamed-collider insert)
- **Location**:
  - `crates/physics/src/sync.rs:1066`: `register_newcomers`, `.position(iso_from_trs(n.global.translation, …))`.
  - `crates/physics/src/sync.rs:87-111`: `set_kinematic_translation`. Its callers are
    `byroredux/src/save_io.rs:781` (`apply_player_pose`, a save-file pose with no finiteness or range check),
    `byroredux/src/systems/character.rs:635,907,1036` and `byroredux/src/commands/view.rs:482`
    (`combat.approach`).
  - Compare `sync.rs:1243-1255`: the only gate, `accept_keyframe_target`, at `recovery.rs:227-254`.
- **Status**: NEW
- **Trigger Conditions**: any of the following:
  - a streamed entity whose `GlobalTransform` at first registration is finite but beyond the multi-SAP saturation
    window (from about 2.1e9 BU for small colliders up to 2.68e11 BU for bone-sized ones), for example a broken
    first animation sample on a live actor's bone, or a corrupt placement;
  - a save whose player pose is corrupt;
  - a debug teleport to such a coordinate.
- **Description**: #5161 refused insane keyframe targets because "the substep recovery only snapshots `Dynamic`
  bodies, and live actor bones are keyframed — this boundary check is the only guard they have". Two other entry
  points put a body into the broad phase at an arbitrary position:
  - **Newcomer registration** builds every streamed body, keyframed bones included, at
    `n.global.translation` with no check. The body is never in the snapshot; a fixed or keyframed one never will
    be.
  - **`set_kinematic_translation`** drives the player capsule. `apply_player_pose` passes it the saved position
    verbatim.

  NaN is harmless here, because rapier rejects non-finite AABBs. A finite out-of-range value is the panic
  precondition analysed in D2-01.
- **Evidence**: `sync.rs:1063-1070` builds the body straight from the newcomer's transform. `save_io.rs:720-781`
  performs no check between `Vec3::from_array(pose.position)` and `set_kinematic_translation`.
- **Impact**: a corrupt transform or save crashes the process instead of being refused or parked. Low
  likelihood; same blast radius as D2-01.
- **Related**: PHYS-D2-2026-10-09-01, whose broad-phase withholding fix also closes both entries; #5161.
- **Suggested Fix**: apply the `accept_keyframe_target` bound in `register_newcomers` (skip the entity and log
  once) and in `set_kinematic_translation` (refuse and return `false`). `apply_player_pose` should reject a
  non-finite or out-of-range pose and fall back to the cell's spawn point. Alternatively, rely on D2-01's
  broad-phase filter as the single backstop.

## Completeness Checks
- [ ] **UNSAFE**: If the fix adds `unsafe`, a safety comment states the upheld invariant
- [ ] **SIBLING**: Same pattern checked in related files (other walkers / record decoders / games / call sites)
- [ ] **LOCK_ORDER**: If a RwLock scope changes, TypeId-sorted acquisition is preserved
- [ ] **TESTS**: A regression test pins this specific fix
