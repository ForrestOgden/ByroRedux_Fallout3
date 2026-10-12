**HEAD**: 5d31fd637 · **Baseline**: `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (HEAD `d6362e652`, 99 commits ago) · **Audited**: all 9 dimensions, delta-first (deep) · **Unchanged since baseline (skimmed)**: Dim 5 (markers: same 24 hits, no new marker), Dim 6 (no new stub; the three 10-11 findings are still open, re-verified), Dim 9's CI-lane half (`.github/` touched once, `real-data-gates.yml` in `fa2af0e89`)

# Tech-Debt Audit — 2026-10-12

Solo run inside the `/audit-suite --preset comprehensive` pass: no sub-agents, no engine launch, no source edits. The only cargo runs
were three `clippy` passes with the pinned 1.96.0 toolchain in the repo `target/` (workspace `-D warnings`, workspace plain, workspace
`--all-targets`) and `cargo machete`. Findings were deduplicated against `/tmp/audit/issues.json` (122 open), the 500 most recent
`tech-debt` issues (any state), targeted `gh --search` queries, the 10-11 report, and `/tmp/audit/REPORTED_THIS_SUITE.md` (re-read at
the end; overlaps are cited as `Existing:`, not re-reported).

## Executive Summary

| Severity | NEW | Regression | Existing (grew, not re-filed) |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 0 | 0 | 0 |
| LOW | 12 | 3 | 2 (#5607, #5621) |

Total: 17 findings (12 NEW, 3 Regression of closed issues that were never re-opened, 2 Existing that grew).

- **The 10-11 debt list was mostly paid down in 24 hours.** 99 commits landed. Closed and verified in code: the six-mover Pass-2 write-back
  (#5580), `CellData: Default` (#5581), the shared water-wave tuning (#5582), the single `MAX_SKIN_SLOTS` constant (#5583), the
  `env_translate` split (#5584; the largest of its five submodules is 661 production LOC), the shared LE readers (#5596), the navmesh fixtures
  (#5597), the CLI-flag readers (#5598), nine doc-rot items (#5599–#5605), and the Dim 7 literal batch (#5614–#5620). `_audit-validate.sh`
  exits 0 with 0 STALE.
- **The gate that was red on 10-11 is still red** (TD8-01, Existing #5621) and now hides more: 2 new lints in the unlinted `byroredux`
  bin crate, 11 test-target lints, and a **deny-level** clippy error in `crates/hkx` that `--all-targets` trips (TD8-02). ROADMAP.md still
  says `clippy --workspace --all-targets` is clean (TD3-01).
- **Three 10-11 findings were never published and are still live** (TD2-06, TD3-02, TD3-03). All three are *Regression* rows: their
  parent issues (#5469, #5470, #5471) were closed on partial fixes. Re-verified against HEAD below.
- **Skill rot is again a fix-wave artefact** (TD4-01): four "known-open" callouts in two skills name issues that closed *today*
  (#5147, #5149, #5284, #5285).
- **New duplication is small and concrete.** Five fresh consolidation sites (TD2-01..05): SPIR-V reflection prologues, five identical
  shader-type arms, two audio dispatch loops, the launcher/detect CLI twin, and the synth-child light branches. None carries a divergent
  bug history yet, so all stay LOW.
- **Production files over 2000 LOC: unchanged at 4** (`shader_constants_data.rs` is the data table; `components.rs` 2076, `fragment/effects.rs` 2018,
  `save_io.rs` 2011 are all tracked). One function-level watch row is new (TD1-01).
- **Clean:** Dim 5 (24 markers, all `XXXX`/FourCC/upstream-FIXME/in-grace prose; MIT attribution block atop `triangle.frag` intact),
  `unimplemented!/todo!()` = 0, no stub comment on a reachable path, `cargo machete` clean, NIF version-gate literals (comment-only hits),
  shader `#define` provenance, every dimension cross-reference between skills resolves, every skill LOC claim within 20 %,
  `cxx-bridge` (26+17 lines) and `platform` (60 lines) still placeholders with one consumer each.

## Baseline Snapshot (measured at `5d31fd637`; `prod_loc` self-test ok)

```text
markers (TODO/FIXME/HACK/XXX/TBD/WIP/KLUDGE): 24   (10-11: 24)   unchanged: 15 XXXX + 2 FO4 HACK + 3 upstream-FIXME + 1 documented TBD + 3 TODO prose
allow(dead_code):                             44   (10-11: 44)   11 are cfg(test) TypeId markers in lock_tracker.rs
unimplemented!/todo!():                        0   (10-11: 0)
#[ignore] tests:                             299   (10-11: 297; tools-inclusive)  all data/device gated; 0 bare
files >2000 production LOC:                    4   (10-11: 5)    -env_translate (split, #5584); shader_constants_data 2769 (table), components 2076, effects 2018, save_io 2011
test files >2000 total LOC:                   77   (10-11: 74)
production fns >200 LOC:                     192   (10-11: 193; same brace scanner)
clippy --workspace -D warnings (1.96.0):       3   physics x1 + renderer x2 (#5621, unchanged); the bin crate is still unlinted
clippy --workspace (no -D), bin crate:         2   NEW hidden lints, see TD8-01
clippy --workspace --all-targets:    11 warnings + 1 deny-level error (hkx), see TD8-01/02   (10-11: 5 warnings)
cargo machete:                             clean
_audit-validate.sh:                           OK — 0 STALE; 74 basename + 5 skill-symbol advisories (73 / 8 on 10-11)
```

## Top 10 Quick Wins (trivial / small)

1. **TD8-02** (trivial): `assert!(rejects.is_empty(), …)` in `crates/hkx/src/animation.rs:1413` clears the only deny-level lint and makes #5671's census exact.
2. **TD8-01** (trivial): the 3 gate lints (`physics/world/mod.rs:935`, `dynamic_rgba.rs:38-39`) plus the 2 bin-crate lints; then #5621 can close.
3. **TD4-01** (trivial): delete four closed "known-open" clauses in `audit-nifal` and `audit-tooling`.
4. **TD3-02 / TD3-03** (trivial): re-wrap four `main.rs` literals and delete its gate exemption; fix the M42 "10 of ~17 / remaining 10" arithmetic in three docs.
5. **TD3-01** (trivial): ROADMAP Gates paragraph, the ABBA Known-Issue bullet (fixed by `1cb12fb3d`), and the `boot.rs` path at `ROADMAP.md:136`.
6. **TD2-02** (trivial): `parse_shader_type_data_fo4` ends in `_ => parse_shader_type_data(stream, shader_type)` and loses five copied arms.
7. **TD7-01** (trivial): name the footstep probe depth.
8. **TD9-01** (trivial): fix the `.nfa` typo in the skeever probe's recipe.
9. **TD2-04** (small): lift `absolutise_against_cwd` and the `--profiles` parser into `byroredux-game-detect`.
10. **TD2-06** (small): route the 18 surviving weak `#[cfg(test)]` cuts through `core::source_scan` and make the guard see the bare spelling.

## Top 5 Medium Investments

1. **#5593 / #5594 / #5595** (open, still the largest duplication family): `layout_binding` helper (now 162 `DescriptorSetLayoutBinding::default()` blocks, the 10-11 count was ~150), the 12 `try_or_cleanup!` macros, groundcover's `write_storage_buffer`. TD2-01 (reflect prologues) is the same family on the SPIR-V side.
2. **#5589**: `load_cell_with_masters` as a wrapper over `InteriorCellApplyJob::begin`.
3. **TD1-01 + #5467 / #5468 / #5585**: split axes for `merge_bgsm_arm`, `components.rs`, `fragment/effects.rs`, `save_io.rs`.
4. **TD2-03**: one `start_spatial_sound` in `crates/audio/src/lib.rs` for the two dispatch loops, which also takes the file (1977 LOC, +19 since 10-11) off the watch-list edge.
5. **#5592 / #5591 / #5590**: the dispatch-loop, player-teleport and animation-sink triplets from the 10-11 list (counts re-checked: unchanged).

## Deferred (gated on a milestone or a product decision)

- **CLAUDE.md Workspace Structure omits 16 of 29 crates** (`audio`, `bgsm`, `boot-request`, `debug-ui`, `facegen`, `fsr3-sys`, `game-detect`, `hkx`,
  `mod-runtime`, `pex`, `physics`, `save`, `sdk`, `settings-io`, `sfmaterial`, `spt`). #5602 closed after adding ROADMAP's rows and
  *deliberately* leaving CLAUDE.md alone because edits to it are the user's call. Not re-filed; `_audit-common.md` calls that tree
  "authoritative", so the call is worth making once.
- **#5608 / #5609 / #5610** (`ScriptTimer`, the plugin `DataStore` family, `PerkList`): still open, re-verified (no producer, no consumer).
- **Three leaf crates keep private LE readers** (`bsa/src/uvd.rs`, `facegen/src/lib.rs`, `sfmaterial/src/reader.rs`): #5596 left them because
  they do not depend on `byroredux-core`. A leaf `bytes` module would fold them; needs a dependency decision.
- **#5313, #5324, #5327, #5290**: older tech-debt issues, still open, nothing new to add.

## Findings

### LOW — Dimension 1: Complexity

### TD1-2026-10-12-01: `merge_bgsm_arm` is 703 lines and still growing, in a file 10 % under the 2000 line
- **Severity**: LOW
- **Dimension**: 1 - File / Function / Module Complexity
- **Location**: `byroredux/src/asset_provider/material/merge.rs:760-1462` (`merge_bgsm_arm`); file is 1794 production LOC
- **Status**: NEW (the 09-29 report measured 669 lines at `:469`; no finding was filed, `merge_external_material` was #3857)
- **Age**: +34 lines since 2026-09-29 (loose `.mat` resolver `c2f28e06c`, CDB blend/emissive capture `9bb6b7dce`, #5503 `4667b884b`, #5284 `c961653f4`); the fn grew 5 % in two weeks
- **Effort**: medium
- **Description**: `fn merge_bgsm_arm(material, provider, pool, path, cdb_pbr_fallback, touched, texture_exists)` walks the whole BGSM template chain and carries ~16 `set_*` "already claimed" sentinels as locals, so every new BGSM field adds a sentinel plus an arm. Its own doc rejects a `MergeSentinels` struct on the argument that the flags are arm-local; that is true, but it is also why the function cannot be split by field group today. Two siblings are over 200: `merge_bgem_arm` (280) and `merge_external_material` (263). The file is the destination for the Starfield `.mat` work, so it will keep growing.
- **Evidence**: `python3 /tmp/audit/tech-debt/fnscan.py` (brace-matched, rustfmt-indent) lists it as the 13th-longest production function in the workspace; `grep -n '^fn \|^pub(crate) fn ' merge.rs` shows `merge_external_material` (`:482`), the BGSM arm (`:760`), the BGEM arm (`:1468`) and `apply_loose_mat` (`:1757`).
- **Impact**: each new BGSM field edit is a >700-line review; the sentinel locals make it easy to miss a branch.
- **Related**: #3857 (the same fn's ancestor, closed on "awareness only"), #5096 (per-frame driver budgets, the precedent for pinning a function size).
- **Suggested Fix**: split by output group (texture slots / scalar + PBR / alpha + flags / emissive) with one small sentinel struct per group, each owned by its group helper; pin the arm's size with a `fn_loc` test in the style of #5096 so it cannot regrow unnoticed.
- **Watch-list (within 5 % of the line, no finding)**: `crates/nif/src/import/types.rs` 1997, `crates/audio/src/lib.rs` 1977 (TD2-03 is its shrink path), `crates/plugin/src/esm/cell/mod.rs` 1935, `crates/scripting/src/translate/effects.rs` 1933, `crates/scripting/examples/mq101_conformance.rs` 1927, `crates/renderer/src/vulkan/context/init.rs` 1879. **Carried, tracked**: #5467 (`components.rs` 2076, +1), #5468 (`fragment/effects.rs` 2018, unchanged), #5585 (`save_io.rs` 2011, unchanged), #5586 (`mq101_conformance.rs::run` 1454 lines, unchanged).

### LOW — Dimension 2: Logic Duplication

### TD2-2026-10-12-01: Seven SPIR-V reflection functions repeat the same parse prologue; two repeat a 25-line named-struct lookup
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/renderer/src/vulkan/reflect.rs:64-71, 165-172, 218-225, 251-258, 330-340, 430-437, 460-467` (prologue); `:259-300` vs `:341-380` (struct lookup)
- **Status**: NEW
- **Age**: `uniform_block_member_offsets_by_name` landed as a copy of `uniform_block_size_by_name` in `a5a0b2598` (#4778, 2026-10-02)
- **Effort**: small
- **Description**: every public function opens with `if !spirv_bytes.len().is_multiple_of(4) { bail!(…) }` then `Loader::new()` + `binary::parse_bytes(…)`. `uniform_block_size_by_name` and `uniform_block_member_offsets_by_name` additionally share byte-for-byte: the `OpName` scan for the struct id, the `HashMap<u32, Instruction>` type table, the `OpTypeStruct` check. The newest sibling was a paste of the older one, which is the divergence recipe (a fix to one error message is not made in the other).
- **Evidence**: `grep -n 'is_multiple_of(4)\|Loader::new()' crates/renderer/src/vulkan/reflect.rs` returns seven pairs; the clone scan finds 17 identical 12-line windows between `:252-275` and `:334-357`.
- **Impact**: the module is the only guard that the shipped `.spv` matches the host `#[repr(C)]` structs (`GpuCamera`, `VolumetricsParams`), so its helpers are what the GPU-struct lockstep tests stand on; they should not be pasted.
- **Related**: #5593 / #5594 (same family: renderer boilerplate with no builder), #4778.
- **Suggested Fix**: add `fn parse_module(bytes) -> Result<Module>` and `fn named_struct<'m>(module, name) -> Result<Option<(u32, &'m Instruction, HashMap<u32, Instruction>)>>` in `reflect.rs` and call them from all seven functions.

### TD2-2026-10-12-02: `parse_shader_type_data_fo4` re-implements five arms its own comment says are "same as Skyrim"
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/nif/src/blocks/shader/lighting.rs:922-1034` (FO4) vs `:838-921` (Skyrim); duplicated arms `6` (hair tint), `7` (parallax occlusion), `11` (multi-layer parallax), `14` (sparkle snow), `16` (eye envmap), about 55 lines
- **Status**: NEW
- **Age**: FO4 variant added with the BSVER 130-139 split (#1552 / SK-D2-01 era)
- **Effort**: trivial
- **Description**: the FO4 function handles types `1` (env-map scale gate) and `5` (skin-tint alpha) differently, then spells `// All other types same as Skyrim.` and re-types the Skyrim arms, byte-reads included. A field-order fix in either copy (these are the layouts `nif.xml` gates by BSVER) would have to be made twice.
- **Evidence**: the clone scan reports 11 identical windows at `:860-874` / `:974-986`; both functions end in `_ => Ok(ShaderTypeData::None)`.
- **Impact**: parse-correctness code where the cost of a missed twin is a stream-position drift on one game only.
- **Related**: `/audit-nif` Dim 3; #1552.
- **Suggested Fix**: end the FO4 match with `_ => parse_shader_type_data(stream, shader_type)` and delete arms 6/7/11/14/16 from it (the Skyrim function already returns `None` for the rest).

### TD2-2026-10-12-03: Two audio dispatch loops share ~60 lines of spatial-track build, `play`, and `ActiveSound` push; `play_oneshot` / `play_oneshot_following` share their prologue
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/audio/src/lib.rs:1286-1346` (`drain_pending_oneshots`) vs `:1418-1500` (`dispatch_new_oneshots`); `:669-700` vs `:706-740` (`play_oneshot`, `play_oneshot_following`)
- **Status**: NEW (the entity path is documented test/tooling-only since #5612, which makes the twin harder to justify)
- **Age**: Phase 3 / Phase 3.5 of M44
- **Effort**: small
- **Description**: both loops build `SpatialTrackBuilder`, call `apply_reverb_send`, `apply_underwater_filter`, `add_spatial_sub_track`, `linear_volume_to_db`, `track.play` and push an `ActiveSound` with the same `last_position` seeding and `stop_issued: false`; they differ in the source tag, the loop-region flag and `unload_fade_ms`. Each setup call was added to both by hand (the underwater filter #3086, reverb send Phase 6). The two public `play_oneshot*` functions repeat the `oneshots_requested` bump, the `manager.is_none()` gate and the 256-entry FIFO cap, including a duplicated `const MAX_PENDING`.
- **Evidence**: clone scan windows `:669-677` ↔ `:712-720` and `:1287-1291` ↔ `:1426-1430`.
- **Impact**: any new send/filter (the next one after reverb and underwater) has to be added at two places; the file is at 1977 LOC.
- **Related**: #5612, TD1-01's watch-list row.
- **Suggested Fix**: `fn start_spatial_sound(audio_world, mgr, listener_id, position, attenuation, sound, volume, looping) -> Result<(StaticSoundHandle, SpatialTrackHandle, Option<…>)>` and a `push_pending(&mut self, item)` for the queue cap; both loops keep only their source-specific tail.

### TD2-2026-10-12-04: `byro-detect` and `byro-launcher` carry the same `--profiles` parser and `absolutise_against_cwd`
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `tools/byro-detect/src/main.rs:164-200` and `tools/byro-launcher/src/main.rs:67-100`
- **Status**: NEW
- **Age**: #5167 (detect) then #5294 ("the launcher twin of #5167"), then #5476 edited both copies on the same day
- **Effort**: small
- **Description**: the strict flag loop (`-h`, `--profiles <path>` that rejects a following flag and a second occurrence) and `absolutise_against_cwd` are identical modulo the launcher's longer comment. The detect copy says "same absolutisation as the launcher", i.e. the duplication is acknowledged in prose and fixed twice by hand. Both tools already depend on `byroredux-game-detect`, which owns `load_with_user_path` (the seam both use for this very path).
- **Evidence**: clone scan, 6 identical 8-line windows at detect `:171-178` / launcher `:73-84`.
- **Impact**: a third consumer of `--profiles` (or the next #5476-style cwd bug) is a two-file edit.
- **Related**: #5167, #5294, #5476.
- **Suggested Fix**: add `parse_profiles_arg(args: &mut impl Iterator<Item = &String>) -> Result<PathBuf, String>` and `absolutise_against_cwd` beside `load_with_user_path` in `crates/game-detect/src/profiles.rs`.

### TD2-2026-10-12-05: `spawn_synth_child`'s two ESM-light branches repeat the `LightSource` build and flicker attach; a census test locks the copies in
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/cell_loader/references/synth_child.rs:380-420` (LIGH-only branch) and `:486-520` (fxlight branch); census at `byroredux/src/cell_loader/spawn/mesh_instance.rs:1890-1925`
- **Status**: NEW
- **Age**: #2439 / #4514 era
- **Effort**: small
- **Description**: both branches run `translate_light`, build `LightSource::from_legacy_world_units(light_radius_or_default(ld.radius), ld.color, ld.flags, canonical_light_falloff_exponent(game, …), geometry.kind, geometry.direction, geometry.outer_angle, canonical_light_shadow_flags(…))`, then `canonical_light_animation_flags` and `attach_light_flicker_if_needed`. The #4514 guard counts the call sites (`assert_eq!(synth.matches("LightSource::from_legacy_world_units(").count(), 2, …)`) instead of removing the second one, so the test now protects the duplication.
- **Evidence**: 13 identical 12-line windows between `:388-402` and `:494-508`.
- **Impact**: the NIFAL light boundary (`canonical_light_*`) is the place a per-game change lands; the next one is three call edits per branch.
- **Related**: #4514, #2439, NIFAL-D3-2026-10-12-01 (a stale doc on the neighbouring `attach_light_flicker_if_needed`).
- **Suggested Fix**: `fn esm_light_source(ld, game, ref_rot) -> LightSource` plus `fn attach_esm_light(world, entity, ld, game, ref_pos, ref_rot)` in `synth_child.rs`; the census becomes "one call site here".

### TD2-2026-10-12-06: The weak `#[cfg(test)]` source cut survives at 18 sites in 12 files; the #5469 guard cannot see the bare spelling (carried unfiled from 10-11 TD2-01)
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/{boot/world.rs:538, cell_loader/load.rs:1398, cell_loader/terrain_lod_btr.rs:616, material_translate.rs:1522, systems/cinematic.rs:1932, systems/combat_ai.rs:689}`, `crates/debug-server/src/registration.rs:463,511,556,641`, `crates/nif/src/import/{collision/mod.rs:786,830, walk/lights.rs:265,318}`, `crates/physics/src/water.rs:1372`, `crates/renderer/src/vulkan/{presentation.rs:916, svgf.rs:1784,2122}`; the guard is `no_source_scan_cuts_with_the_weak_cfg_test_needle` (`byroredux/src/workspace_hygiene_tests.rs:710-760`)
- **Status**: Regression of #5469 (CLOSED). 10-11's TD2-01 counted 34 cuts; no issue was opened for it.
- **Age**: `334458b3f` (#5469) landed the helper and the guard; the survivors predate it
- **Effort**: small
- **Description**: the guard's needles are `find("…")`, `rfind("…")`, `split("…")` and `split_once("\n…")`. The common spelling in the survivors is `.split_once("#[cfg(test)]")` with no leading newline, which none of the four needles matches, so the guard stays green. Four of the sites (`nif/import/collision/mod.rs:786,830`, `nif/import/walk/lights.rs:265,318`) also fall back to the whole file with `.unwrap_or(src)`, the vacuity #4842 removed elsewhere (a source-scan guard whose needle appears in its own test literal). `crate::source_scan` is reachable from all of these crates except `pex`/`facegen` (already allowlisted).
- **Evidence**: `git grep -nF '"#[cfg(test)]"' -- 'crates/*.rs' 'byroredux/*.rs' 'tools/*.rs'` minus the six allowlisted files = 22 lines, 18 of them code (the rest are a prose comment, two lines in `registry_completeness_tests.rs` and the needle constant in `version_literal_tests.rs`).
- **Impact**: each is a guard that can be silently weakened by a test module in its file, which is the class #4604 / #4842 / #5164 / #5469 chased.
- **Related**: #5469, #5100.
- **Suggested Fix**: migrate the 18 sites to `production_text` / `production_text_or_whole` / `strip_test_modules`, then add `split_once("{attr}")` (bare) and `.split_once(` formatted across lines to the guard's needle set (scan with a regex on the joined text, not per line).

### LOW — Dimension 3: Stale Documentation & Comments

### TD3-2026-10-12-01: ROADMAP.md Gates paragraph, ABBA Known Issue and one path are stale against HEAD
- **Severity**: LOW
- **Dimension**: 3 - Stale Documentation & Comments
- **Location**: `ROADMAP.md:27-35` (Gates), `:694-701` (Known Issues bullet), `:136` (`byroredux/src/boot.rs`); `docs/engine/archives.md:372` (`asset_provider.rs`)
- **Status**: NEW
- **Age**: `1cb12fb3d` (#5535, 2026-10-12) fixed the ABBA lane after the 2026-10-10 close wrote these lines
- **Effort**: trivial
- **Description**: four claims no longer hold. (1) "Hosted CI is **red at HEAD on the ABBA lock-order lane alone**": #5535 reordered three production inversions and fixed two test bodies, so the lane is green, and the CI clippy gate is red (TD8-01), so "alone" is wrong both ways. (2) "`cargo clippy --workspace --all-targets` is clean (re-run at this close)": at HEAD it has 11 test-target warnings and one deny-level error (TD8-02). (3) The Known Issues bullet "ABBA lock-order lane red since the `7ef007898` push … Not yet triaged" describes a fixed, triaged problem. (4) `ROADMAP.md:136` points `expand_game_profile_args` at `byroredux/src/boot.rs`, which is `boot/cli.rs` (a directory since #3170); `archives.md:372` names `asset_provider.rs`, now `asset_provider/archive.rs`. `ROADMAP.md:448` (`env_translate.rs`) is already EXT-D1-2026-10-12-01.
- **Evidence**: `git show 1cb12fb3d --stat`; `ls byroredux/src/boot/`; the clippy output in TD8-01; `grep -n 'boot.rs\|asset_provider.rs' ROADMAP.md docs/engine/archives.md`.
- **Impact**: ROADMAP is the document the suite and `/session-close` treat as the single home for status; a reader trusting the Gates paragraph skips the clippy gate, which is the one that is actually red. Held at LOW because `/session-close` refreshes it by design and no audit was observed misled.
- **Related**: #5535 (closed), #5621, #5532 / #5335 (earlier ROADMAP drift).
- **Suggested Fix**: rewrite the Gates paragraph to "CI is red on the clippy step (#5621)"; delete the ABBA bullet (or move it to a one-line "fixed 2026-10-12 `1cb12fb3d`"); fix the two paths.

### TD3-2026-10-12-02: The #5470 space-run gate still exempts `byroredux/src/main.rs`, hiding four collapsed-continuation messages (carried unfiled from 10-11 TD3-01)
- **Severity**: LOW
- **Dimension**: 3 - Stale Documentation & Comments (text hygiene)
- **Location**: `byroredux/src/main.rs:332, 338, 409, 417`; exemption `"byroredux/src/main.rs"` at `byroredux/src/workspace_hygiene_tests.rs:397`
- **Status**: Regression of #5470 (CLOSED); no issue was opened for the 10-11 report
- **Age**: `f8ea9d46f` (2026-08-31)
- **Effort**: trivial
- **Description**: re-verified at HEAD: the same four literals carry 14-18 space runs from a `\`-continuation collapse (`"…orders of magnitude,              got {blocked}"`). 22 of the gate's 23 exempt files are column-aligned tables; `main.rs` is not.
- **Evidence**: `grep -nE '"[^" ][^"]*[^ "] {6,}[^ "][^"]*"' byroredux/src/main.rs | grep -v '\\n'` returns the four lines.
- **Impact**: cosmetic (assertion messages); the exemption is a blind spot for the next collapsed literal in that file.
- **Related**: #5470, #5481.
- **Suggested Fix**: re-wrap with `\` continuations and delete the `main.rs` entry from `exempt`.

### TD3-2026-10-12-03: M42 procedure status is still self-contradictory after #5471 (carried unfiled from 10-11 TD3-02)
- **Severity**: LOW
- **Dimension**: 3 - Stale Documentation & Comments
- **Location**: `docs/feature-matrix.md:89-105`; `docs/engine/npc-spawn-ai-packages.md:9` (vs `:522`); `ROADMAP.md:61`, `:94`; `docs/engine/game-loop.md:155` (PostUpdate NPC row)
- **Status**: Regression of #5471 (CLOSED); no issue was opened for the 10-11 report
- **Age**: Eat/Sleep `00f580e09` (2026-10-08), Dialogue `14cff35ae` (10-07)
- **Effort**: trivial
- **Description**: re-verified at HEAD. `feature-matrix.md:89` says "10 of ~17 `PACK` procedures have a runtime" and lists seven, then `:102` calls "the remaining 10 procedures (Find/Eat/Sleep/…/Dialogue/…)" parse-only (10 + 10 > 17; Eat, Sleep and Dialogue are named as parse-only although `make_eat_sleep_system()` is registered at `boot/schedule/post_update.rs:129` and force-greet runs). `npc-spawn-ai-packages.md:9` says "exactly seven procedures execute", `:522` says "Ten procedures of ~17 execute". `ROADMAP.md:61` says "Seven AI package procedures run by default", `:94` says "10 of ~17 … need subsystems that do not exist (… dialogue)". The `game-loop.md` PostUpdate row lists sandbox/wander/travel/follow/escort/guard/patrol only.
- **Evidence**: `grep -n "exactly seven\|remaining 10\|10 of ~17\|Seven AI package\|Ten procedures" docs/feature-matrix.md docs/engine/npc-spawn-ai-packages.md ROADMAP.md`.
- **Impact**: `feature-matrix.md` is the status floor and `npc-spawn-ai-packages.md` the reference `/audit-gameplay` is told to believe; both tell an auditor `eat_sleep_system` should not exist.
- **Related**: #5471, #5367, #5376, #5390.
- **Suggested Fix**: add Eat/Sleep and Dialogue to the run-by-default list, rewrite the "remaining" sentence to the 7 genuinely missing procedures, change `npc-spawn-ai-packages.md:9` to "ten", `ROADMAP.md:61` to "Ten", `:94` to "7 of ~17 (item use, package combat, magic)", and add eat/sleep to the `game-loop.md` row.

### LOW — Dimension 4: Audit-Finding Rot

### TD4-2026-10-12-01: Four "known-open" callouts in two skills name issues that closed today
- **Severity**: LOW
- **Dimension**: 4 - Audit-Finding Rot
- **Location**: `.claude/commands/audit-nifal/SKILL.md:165` (#5284), `:179` (#5285); `.claude/commands/audit-tooling/SKILL.md:42` (#5149, #5147)
- **Status**: NEW
- **Age**: all four fixed 2026-10-12: `c961653f4` (#5284), `661be4eaa` (#5285), `224180508` (#5149), `72a63616b` (#5147)
- **Effort**: trivial
- **Description**: `audit-nifal:165` says "the CDB resolve tail is copied at four `merge.rs` sites (#5284)" and `:179` "known-open #5285: the declared boundary lists six beam converters, `CachedNifImport::beam_volumes` chains seven"; `audit-tooling:42` says "a `0.0.0.0` bind would pass every guard (known-open #5149). Known-open #5147: default output names are not unique". All four are closed. A pass that dedups by "known-open, cite" now suppresses a regression of fixed code. Every other closed-issue-as-open mention found by the sweep (`gh`-state check of 151 `#N` refs on "open"-worded skill lines) is either history ("fixed in …") or still open (#5459, #5505, #5525, #5534, #5661).
- **Evidence**: `gh issue list --state open` (the suite snapshot) contains none of the four numbers; commit subjects above.
- **Impact**: the same fix-wave rot as 10-11's TD4-02 (#5606); the dedup rule in `_audit-common.md` makes a stale "known-open" an active blindfold (#3199).
- **Related**: #5606, #3199.
- **Suggested Fix**: replace the four clauses with the fix commit and the pin that now guards it (`beam_volumes` count test; a loopback-address test; the self-unique default names), per the "Fixed since …" convention `audit-oblivion` already uses.

### TD4-2026-10-12-02: The validator's skill-symbol advisory is 60 % noise, and two real convention violations hide in it
- **Severity**: LOW
- **Dimension**: 4 - Audit-Finding Rot
- **Location**: `.claude/commands/audit-oblivion/SKILL.md:101,103` (`inherit_all_up_chain`, `env_translate.rs`), `.claude/commands/audit-nifal/SKILL.md:90` (`SourceSoftAdditive`), `.claude/commands/audit-nif/SKILL.md:350` (`streaming.rs`); noise: `FIXTURE_DATA_ENV` (audit-runtime:108), `FIXTURE_GATES` (audit-fo3:102), `ragdoll_installed` (audit-physics:15)
- **Status**: Existing: #5607 (the validator-buries-signal finding); this adds the measured split
- **Age**: `65cadde2f` (#5516) deleted `inherit_all_up_chain`; `c140cb48e` (#5584) deleted `env_translate.rs`
- **Effort**: trivial
- **Description**: `_audit-validate.sh` exits 0 with 5 skill-symbol and 8 skill-basename advisories. Three of the five symbols are not Rust identifiers (two shell variables in `docs/smoke-tests/lib/fixture.sh`, one file stem), so the validator cannot tell them from rot. The other two are historical names written in backticks, against the Path-Reference Convention ("deleted names go in italics"): `inherit_all_up_chain` (deleted by #5516), `SourceSoftAdditive` (a CDB blend-mode string, not a symbol). Basename advisories add `env_translate.rs` (`audit-oblivion:103`) and `streaming.rs` (`audit-nif:350`, the #5092 split). `triangle_early.frag` in `audit-renderer`/`audit-performance` is the compile-time variant of `triangle.frag` (only the `.spv` is tracked) and would also read better in italics.
- **Evidence**: `/tmp/audit/tech-debt/validate.txt`; `grep -rn "inherit_all_up_chain" crates byroredux` is empty.
- **Impact**: the advisory list is where renamed guards show up (TD4-01 on 10-11 came from it); 74 + 5 lines of scroll hide the two that matter.
- **Related**: #5607.
- **Suggested Fix**: italicise the five historical names, teach the validator to skip `*.sh` variable names (match against `docs/smoke-tests/` too), and make the skill-symbol advisory fail the script as #5607 proposes.

### LOW — Dimension 7: Magic Numbers

### TD7-2026-10-12-01: The footstep surface probe's 128-unit reach is a bare literal
- **Severity**: LOW
- **Dimension**: 7 - Magic Numbers & Hardcoded Constants
- **Location**: `byroredux/src/systems/audio.rs:278` (`pw.surface_material_under(*pos, 128.0, own_body)`); the same literal is hand-typed in 3 `physics/world/mod.rs` tests
- **Status**: NEW
- **Age**: `886edcded` (2026-10-10); `own_body` added by #5545 / #5556
- **Effort**: trivial
- **Description**: the probe starts at the walker's capsule centre and reaches 128 BU; against the `capsule_y(46.0, 18.0)` the physics tests use (centre 64 BU above the feet) that is roughly twice the centre height, but nothing in the code says so. The neighbouring systems name their equal values (`FOLLOW_DEFAULT_DISTANCE`, `ESCORT_COLLECT_DISTANCE`, `FORCE_GREET_RADIUS`, all `128.0`) while this one does not.
- **Evidence**: `git grep -n 'surface_material_under(' -- byroredux crates` shows the single production caller.
- **Impact**: a taller capsule (creature, crouch) or a different kinematic origin silently makes footsteps read `None` and take the default sound; there is no test tying the reach to the capsule.
- **Related**: #5545, #5556, AUD-2026-10-12-D5-01.
- **Suggested Fix**: `const FOOTSTEP_SURFACE_PROBE_REACH: f32` next to the capsule constants in `crates/physics`, with a test asserting it exceeds the player capsule's centre height.

### LOW — Dimension 8: Dead Code & Gates

### TD8-2026-10-12-01: CI clippy gate is still red, and the bin crate it leaves unlinted now carries two lints (extends #5621)
- **Severity**: LOW
- **Dimension**: 8 - Dead Code & Backwards-Compat Cruft (clippy gate)
- **Location**: gate: `crates/physics/src/world/mod.rs:935` (`map` returning `()`), `crates/renderer/src/texture_registry/dynamic_rgba.rs:38-39` (doc list indentation). Hidden by the red gate: `byroredux/src/systems/navmesh_path.rs:742` (`too_many_arguments`, 8/7 on `cached_walk_step`), `byroredux/src/systems/story_events.rs:37` (`!x.is_some_and(…)` simplifiable). Test targets: `byroredux/src/cell_loader/reference_state.rs:1160`, `byroredux/src/systems/character.rs:1884`, `crates/physics/src/water.rs:2382,2642` (`field_reassign_with_default`), `crates/plugin/src/esm/records/misc/dialogue.rs:1862`, `crates/nif/src/blocks/collision/bhk_constraint_tests.rs:560`, `byroredux/src/systems/story_events.rs:264,284`, `byroredux/src/render/groundcover.rs:1295`, `crates/scripting/src/story_manager.rs:1543`, `crates/scripting/src/papyrus_provider/tests/execute.rs:1395`
- **Status**: Existing: #5621 (open); the two bin-crate lints and the longer test bucket are new since 10-11
- **Age**: gate lints from #5528 / #5539 (10-11); `cached_walk_step` from `6316719ed` (2026-10-11); `story_events.rs:37` from #5447
- **Effort**: trivial
- **Description**: `cargo clippy --workspace --keep-going -- -D warnings` at HEAD fails `byroredux-physics` (1 lint) and `byroredux-renderer` (2 lints); the `byroredux` bin crate is therefore never linted by the gate. Running it without `-D` shows what the gate hides: two lints, both introduced after the 10-11 audit. Under `--all-targets`, 11 test-target warnings (10-11: 5) and one error (TD8-02). `cargo machete` is clean. `#[allow(clippy::…)]` sites without a reason comment: 31 by a comment-above/inline heuristic (#5622 counted 28, tracked).
- **Evidence**: `/tmp/audit/tech-debt/clippy_ws.txt`, `clippy_nod.txt`, `clippy_all.txt` (1.96.0, repo `target/`).
- **Impact**: the highest-churn crate is a lint-blind zone for as long as the gate is red; a third unlinted lint arrived within a day.
- **Related**: #5621, #5622, TD3-2026-10-12-01, ECS and SAFETY suite notes (same three gate lints).
- **Suggested Fix**: fix the 3 + 2 production lints (give `cached_walk_step` a `WalkStep { current, rotation, goal, repath_threshold, dt, speed }` input struct, which also shortens its 8 call sites), then the 11 test lints, then close #5621.

### TD8-2026-10-12-02: A deny-level clippy error in the HKX census breaks `cargo clippy --all-targets` for `byroredux-hkx`
- **Severity**: LOW
- **Dimension**: 8 - Dead Code & Backwards-Compat Cruft (clippy gate, test targets)
- **Location**: `crates/hkx/src/animation.rs:1413` (`assert!(rejects.len() <= KNOWN_VANILLA_REJECTS, …)` with `const KNOWN_VANILLA_REJECTS: usize = 0` at `:1410`)
- **Status**: NEW (consequence of #5662's fix; related open #5671 which asks for an exact assert)
- **Age**: `9707cb367` (#5662, 2026-10-12) lowered the baseline from 288 to 0 under the existing `<=` form from `d4352d477`
- **Effort**: trivial
- **Description**: `usize <= 0` trips `clippy::absurd_extreme_comparisons`, which is `deny` by default, so `error: could not compile byroredux-hkx (lib test)` appears under `cargo clippy --workspace --all-targets`. The test is `#[ignore]`d (real Skyrim SE archive), so `cargo test` does not notice, and CI only runs the non-`--all-targets` form. ROADMAP says the all-targets form is clean (TD3-01).
- **Evidence**: `/tmp/audit/tech-debt/clippy_all.txt` lines 159-169.
- **Impact**: the `--all-targets` developer command stops at this crate's test unit; the dependents of that unit are not linted.
- **Related**: #5671, #5662, #5326.
- **Suggested Fix**: `assert!(rejects.is_empty(), …)` with the message that now names the clips; the same edit delivers #5671's "make it exact".

### TD8-2026-10-12-03: Fifteen `pub` items are referenced nowhere in the workspace, one of them a consolidation helper with no caller
- **Severity**: LOW
- **Dimension**: 8 - Dead Code & Backwards-Compat Cruft
- **Location**: `crates/plugin/src/esm/records/common.rs:287` (`read_lstring_sub`); `crates/core/src/lighting.rs:69` (`per_bethesda_unit`); `crates/core/src/character/derived.rs:310` (`b_from_base`); `crates/core/src/ecs/components/actor_values.rs` (`set_permanent`); `crates/core/src/ecs/resources/skin_slot_pool.rs` (`allocated_count`, `free_list_depth`); `crates/core/src/settings/mod.rs:141` (`requiring_restart`); `crates/menuxml/src/menu.rs` (`set_override_str`, `clear_override`, `set_screen`); `crates/papyrus/src/parser/mod.rs:418` (`check_keyword`); `crates/papyrus/src/token.rs:349` (`can_start_expr`); `crates/spt/src/scene.rs:178` (`count_by_kind`); `crates/ui/src/profile.rs:57` (`is_avm2`); `crates/plugin/src/esm/cell/mod.rs:1219` (`OBLIVION_WORLDSPACE`)
- **Status**: NEW
- **Age**: oldest `f03a7711e` (2026-04-12, the Papyrus pair); `read_lstring_sub` `a0e64568a` (2026-05-15, #1045)
- **Effort**: trivial
- **Description**: whole-repo identifier count (tests, examples and docs-in-code included) is 1 for each: the definition itself. Most are crate-public accessors the skill says to note rather than delete. The exception is `read_lstring_sub`: its doc says "Use this instead of the open-coded pair" (#1045), nothing calls it, and the pair it was written to replace (`find_sub(…).map(read_lstring_or_zstring)`) is still open-coded at 2 sites, i.e. the consolidation was written but never applied. (The `shader_flags.rs`/`bsx.rs`/`scene_flags.rs` flag-bit constants were excluded as vocabulary tables.)
- **Evidence**: identifier-count scan in `/tmp/audit/tech-debt/` (`git ls-files '*.rs'`, word count == 1, definitions outside `examples/` and `tests/`).
- **Impact**: dead surface that documents itself as the preferred idiom (`read_lstring_sub`) misleads the next ESM record author; the rest is noise.
- **Related**: #1045.
- **Suggested Fix**: migrate the two open-coded pairs to `read_lstring_sub` (or delete it); delete the Papyrus pair and `per_bethesda_unit`; leave the menuxml/ui/settings accessors with a note unless a consumer is planned.

### LOW — Dimension 9: Test Hygiene

### TD9-2026-10-12-01: The skeever isolation probe's run recipe has a typo, and the pin for closed #5161 is hand-run only
- **Severity**: LOW
- **Dimension**: 9 - Test Hygiene
- **Location**: `byroredux/src/ragdoll_skeever_probe_tests.rs:21-24, 90`
- **Status**: NEW
- **Age**: `5ae7f8ad4` (#5161), recipe edited in `ac81a09f9` (rapier 0.36 bump, 10-10)
- **Effort**: trivial
- **Description**: the module doc extracts to `/tmp/skeever_skeleton.nif` and then runs `BYRO_SKEEVER_NIF=/tmp/skeever_skeleton.nfa`, a file that does not exist; the test's own default is `/tmp/byro-5161-probe3/skeever_skeleton.nif`. The probe `#[ignore]`s on "needs BYRO_SKEEVER_NIF" and, per its own doc, "now pins the containment" of an explosion that previously poisoned the broad phase, but it needs a hand-extracted NIF plus a hand-set env var, so no lane ever runs it (the sibling `ragdoll_installed_tests` do run in the default lane).
- **Evidence**: `grep -n 'nfa\|skeever_skeleton' byroredux/src/ragdoll_skeever_probe_tests.rs`.
- **Impact**: the only copy-pasteable command for the #5161 containment check fails with "missing: /tmp/skeever_skeleton.nfa".
- **Related**: #5161, #5488, `docs/smoke-tests/README.md` (the place a data-gated lane would live).
- **Suggested Fix**: correct `.nfa` to `.nif`; if the containment claim is meant to hold, read the NIF from the Skyrim archive the way the other real-data tests do (`BYROREDUX_SKYRIMLE_DATA` fallback) so the `--ignored` corpus run exercises it.

## Dimension notes (no finding)

- **Dim 5 (Markers)**: 24 hits, identical to 10-11: 15 `XXXX` tags, 2 FO4 `HACK` FourCC, 3 upstream-FIXME quotes, 1 documented `TBD` (FNV `WEAP DNAM`), and 3 `TODO` prose lines in `byroredux/src/loading_screen.rs:10,47` (MOD2 camera-path NIFs, 10 days old, no issue to close) and `scene.rs:1304` (a "closes the #242 consumer-side TODO" past-tense note). The MIT/Burley attribution block atop `triangle.frag` (`:28-39`) is intact. Shader directory: no markers.
- **Dim 6 (Stubs)**: `unimplemented!/todo!()/panic!("not …")` = 0; no stub comment on a CLI-reachable path; `papyrus_demo/mg07_door.rs:198` is a documented test-setup stub. Production-reachability sweep of the 53 `pub` fns added in the 99-commit window found no zero-caller addition. Carried and re-verified: #5608 (`ScriptTimer` has no producer), #5609 (`DataStore` unused by `byroredux/src`), #5610 (`PerkList` orphan), #5623 (`perk_flags`).
- **Dim 7 (Magic numbers)**: NIF version-gate literals appear only in comments; every shader `#define` outside the generated header is an alias (`GC_*`, `renderOrigin`, `push`). A float-literal scan of the window's production diff (117 lines outside the `env_translate` move) found one bare tunable (TD7-01); the rest are test fixtures. ESM sub-record `data.len() >= N` checks (72 sites) are the long-standing convention and were not re-litigated.
- **Dim 9 (Tests)**: all 299 `#[ignore]`s carry a reason and all non-keyword ones are data gates (5 spot-triaged). 209 `#[test]` fns added in the window were scanned for assertion-free bodies: none (the one hit, `transmission_skip_walk_lit_exits_run_the_glass_leg`, asserts through a multi-line macro). `[features]` set unchanged since 10-11; #5624 (parallel-scheduler OFF) and #5625 (golden frames lane) remain open.
- **Fix-wave verification**: the 10-11 findings closed in the window were re-checked against code, not against the commit message: `CellData` derives `Default` (`crates/plugin/src/esm/cell/mod.rs`), `MAX_SKIN_SLOTS` feeds the slot pool, `env_translate/` submodules are 160-661 LOC, `contributing.md` / `testing.md` carry `--features inspect`, ROADMAP is exactly 800 lines and held by `readme_and_roadmap_hold_their_line_ceilings`.

## Cross-dimension dedup and overlap with this suite

- Doc rot already reported by sibling audits and **not** re-reported here: REN-D2-2026-10-12-01 (`shadow_transport.glsl` docblock), REN-D3-2026-10-12-01 (bind-inverse cap 1366 → 1364 in docs), REN-D6-2026-10-12-01 / NIFAL-D9-2026-10-12-01 / TOOL-D1-2026-10-12-01 (helpers inserted between a doc comment and its fn), NIFAL-D6-2026-10-12-01 (`shape_leaf_materials` naming, incl. `physics.md:436`), NIF-D4-2026-10-12-01 (`nif-parser.md` module map, the `anim.rs` basename advisories), TOOL-D2-2026-10-12-01 (`debug-cli.md` component table), CHAR-D5-* (charal docs), ESM-D5-2026-10-12-01, EXT-D1-2026-10-12-01 (`env_translate` doc rot incl. `ROADMAP.md:448`), PEX-D4-2026-10-12-01, UI-D1-2026-10-12-02, AUD-2026-10-12-D5-02 / D3-01, SAVE-D5-2026-10-12-01, PHYS-D3-2026-10-12-02, CONC-D5-2026-10-12-01.
- Clippy: the ECS and SAFETY audits noted the same three gate lints (SAFETY cites #5621); TD8-01/02 add the bin-crate and all-targets half.
- HKX: PAR-D4-2026-10-12-02 (#5671) is the census-exactness issue; TD8-02 is the compile-level consequence.
- `Existing` trackers the 10-11 list is still waiting on, re-checked unchanged: #5585, #5586, #5587, #5588, #5589, #5590, #5591, #5592, #5593, #5594, #5595, #5606, #5607, #5608, #5609, #5610, #5621, #5622, #5623, #5624, #5625, #5626, #5467, #5468.

## Report notes

- Scratch: `/tmp/audit/tech-debt/` (kept for the suite's reconciliation; contains the clippy logs, the validator output and the scan scripts).
- Suggested next step: `/audit-publish docs/audits/AUDIT_TECH_DEBT_2026-10-12.md`.
