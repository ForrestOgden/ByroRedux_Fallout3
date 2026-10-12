=== #5596 ===
OPEN | TD2-2026-10-11-13: Absolute-offset little-endian readers re-implemented ~16 times across 8 crates; no shared home
Labels: bug,import-pipeline,low,tech-debt
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `plugin/src/esm/records/misc/world.rs:906` (`u32_at`), `misc/water.rs:1096` (`read_f32_at`, finite-filtered), `misc/imagespace.rs:81` (`read_f32`), `nif/src/import/mesh/decode.rs:6,12` (`read_f32_le`, `read_u16_le`), `nif/.../mesh/tangent.rs:124`, `nif/src/blocks/collision/havok_packfile.rs:251`, `byroredux/src/cell_loader/placement_lod.rs:109,115`, `scripting/src/obscript.rs:499`, `obscript_runtime.rs:402`, `obscript_vm.rs:186`, `bsa/src/uvd.rs:133,139`, `facegen/src/lib.rs:79,96`, `sfmaterial/src/reader.rs:1241`, `renderer/src/vulkan/dds.rs:1000` (panicking, indexes directly)
- **Status**: NEW
- **Age**: spans 2026-04 to 2026-10.
- **Effort**: medium
- **Description**: the same `data.get(off..off+N)?.try_into()` + `from_le_bytes` shape, returning `Option`, `io::Result` or `Result<_, CustomError>`. `crates/plugin/src/esm/sub_reader.rs` documents that it "replaces inline `read_u32_at`/`read_f32_at`/`from_le_bytes` patterns", but it is a sequential cursor; the random-access cases (per-game WATR offsets, RDAT rows, `.lod`, `.uvd`) re-grew local helpers. `dds.rs:1000` can panic on a truncated header rather than return an error.
- **Evidence**: `grep -rn --include='*.rs' -E 'fn \w+\(\s*\w+: &\[u8\],\s*\w+: usize\s*\)\s*->\s*(Option|io::Result|Result)<(u16|u32|f32)' crates byroredux | grep -v 'tests/\|examples'` -> 16 definitions.
- **Impact**: low per site; the DDS copy is the only one with a wrong failure mode.
- **Related**: `sub_reader.rs` module doc; #4521 (a different slice of the same byte-view class).
- **Suggested Fix**: add `crates/core/src/bytes.rs` with `pub fn le_u16_at / le_u32_at / le_f32_at(data: &[u8], offset: usize) -> Option<u16/u32/f32>` (`checked_add` on the offset), and migrate plugin (4), nif (3), scripting (3), byroredux (2), renderer (1). `bsa`, `facegen`, `sfmaterial` do not depend on core (Cargo.toml check), so leave them or add the dependency with approval. Add an `io_err(Option) -> io::Result` adapter only where a caller needs it.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix

=== #5597 ===
OPEN | TD2-2026-10-11-14: Navmesh test fixtures `zup_from_yup` x7 and `two_triangle_quad_navm` x6 despite a shared `test_tiles` module
Labels: bug,low,tech-debt,ai
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW (test-only)
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/systems/{follow.rs:603,travel.rs:624,patrol.rs:412,wander.rs:673,guard.rs:542,escort.rs:731,navmesh_path.rs:828}` (`zup_from_yup`), `follow.rs:609`, `travel.rs:633`, `patrol.rs:418`, `wander.rs:679`, `guard.rs:550`, `escort.rs:737` (`two_triangle_quad_navm`); shared home `systems/navmesh_path.rs:816` (`pub(crate) mod test_tiles`, already holds `two_triangle_quad` at 10 units and `adjacent_quad`)
- **Status**: NEW
- **Age**: 2026-08-23 (`77eda8003`) onward.
- **Effort**: trivial
- **Description**: seven copies of the inverse of `zup_to_yup_pos` (`[x, -z, y]`) - the Z-up knowledge that lives in `crates/core/src/math/coord.rs` - and six copies of the same 2-triangle quad at 1000 units. The comments (`guard.rs:540-549`) cite "each module owns its own test fixtures per this codebase's established convention", while `test_tiles` exists precisely because the EX-16 harness needed cross-module fixtures.
- **Evidence**: `grep -rn 'fn zup_from_yup\|fn two_triangle_quad_navm' byroredux/src/systems` -> 13 lines.
- **Impact**: low; if the Z-up convention for navmesh vertices changes, seven fixtures change in lockstep or one silently lies.
- **Related**: TD2-2026-10-11-02.
- **Suggested Fix**: add `yup_to_zup_pos([f32;3])` beside `zup_to_yup_pos` in `crates/core/src/math/coord.rs` (a round-trip test lives there) and `pub(crate) fn two_triangle_quad_scaled(size: f32) -> NavmRecord` to `navmesh_path::test_tiles`; delete the 13 copies.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **LOCK_ORDER**: If a RwLock scope changes, TypeId-sorted acquisition is preserved
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix

=== #5598 ===
OPEN | TD2-2026-10-11-15: CLI flag lookup: `cli_args::parse_string_arg` (with the `--flag=value` trap warning) is bypassed by ~27 raw `position(|a| a == "--...")` sites and four `--master` collectors
Labels: bug,low,tech-debt
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/cli_args.rs:58` (helper, 16 callers); raw: `scene.rs` (11, e.g. 434-465, 722, 1468), `boot/mod.rs:199,406,425,449`, `scene/nif_loader.rs:86,103`, `hud.rs:390,439`, `scaleform_hud.rs:211`, `cornell/oracle.rs:237,278`, `boot/cli.rs:454`, `app_step.rs:1222`; `--master` collectors: `hud.rs:403`, `scene.rs:488`, `boot/mod.rs:438`, plus `extensions/install.rs:834 flag_values`
- **Status**: NEW
- **Age**: `hud.rs`/`scaleform_hud.rs` `--esm` prologue copy-pasted 2026-09-19 (`62fc0bf22`).
- **Effort**: small
- **Description**: `parse_string_arg`'s doc records the trap it exists for: `--camera-pos=...` matched nothing and silently fell back, costing the W1 water fixture two derivation runs. The raw sites carry the same silent-miss behaviour for `--esm`, `--cell`, `--grid`, `--wrld`, `--radius`, `--kf`, `--bsa`, `--swf`, `--hud-textures` and the rest. `hud_archive_args` (`hud.rs:384-401`) and `hud_scaleform_args` (`scaleform_hud.rs:206-222`) share a verbatim 14-line `--esm` + `esm_dir` prologue. Multi-value `--master` has three inline collectors with three different shapes plus a fourth generic one in `extensions/install.rs`.
- **Evidence**: `grep -rn --include='*.rs' 'position(|a| a == "--\|position(|arg| arg == "--' byroredux | grep -v test | wc -l` -> 27; `grep -rn --include='*.rs' 'parse_string_arg(' byroredux/src | grep -v test | wc -l` -> 16.
- **Impact**: the silent-drop trap applies to every raw flag; the two HUD probes would diverge on the next `--esm` rule.
- **Related**: `cli_args.rs` module doc (the W1 / P1 incident).
- **Suggested Fix**: add `pub fn parse_all_string_args(args, flag) -> Vec<String>` (lifting `install.rs::flag_values`) and `pub fn esm_discovery_root(args) -> Result<Option<(String, PathBuf)>, String>` to `byroredux/src/cli_args.rs`; route the 27 raw sites through `parse_string_arg` and the four `--master` loops through the new function. `hud_archive_args` / `hud_scaleform_args` call `esm_discovery_root`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix

=== #5599 ===
OPEN | TD3-2026-10-11-03: ecs.md and architecture.md still say the engine has "12 parallel-stage systems"; it has 9
Labels: documentation,ecs,low,tech-debt,doc-rot
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments (authoritative engine doc)
- **Location**: `docs/engine/ecs.md:503`, `docs/engine/architecture.md:135`
- **Status**: NEW
- **Age**: number is the M27 Phase-1 figure (`docs/engine/m27-phase1-survey.md:9` "12 parallel-stage systems (3 declared today, 9 undeclared)"); first falsified by #3111 which serialised the weather writer.
- **Effort**: trivial
- **Evidence**: `grep -rn "scheduler.add_to_with_access(" byroredux/src | wc -l` = 9; `boot/schedule/mod.rs:590` "The nine `add_to_with_access` registrations"; guard `the_parallel_system_table_covers_every_parallel_registration` pins the table to the registration count; `scheduler_access_tests.rs` floors `parallel_system_count() >= 9` ("floor 9 after #3111").
- **Impact**: the lock-order/concurrency audits cite ecs.md as authoritative for the analyzable population; 12 overstates it by a third.
- **Suggested Fix**: say "the engine's nine parallel-stage systems" (or drop the number and point at `PARALLEL_SYSTEMS` in `boot/schedule/mod.rs`).

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **LOCK_ORDER**: If a RwLock scope changes, TypeId-sorted acquisition is preserved
- [ ] **DOC**: `.claude/commands/_audit-validate.sh` still exits 0 after the edit

=== #5600 ===
OPEN | TD3-2026-10-11-04: Three `docs/feature-matrix.md` rows under-state shipped code (UV scrolling, NPC/creature physics, "kira 0.10")
Labels: documentation,low,tech-debt,doc-rot
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments
- **Location**:
  - `docs/feature-matrix.md:128` "UV scrolling (animated UV offset) | ✗ | parsed, not rendered" — contradicts `:124` "Embedded ambient controllers (UV scroll, alpha fade, vis flicker) | ✓" in the same table. `AnimatedUvTransform` (`crates/core/src/ecs/components/animated.rs:88`) is written by `animation_system` and consumed at `byroredux/src/render/static_meshes.rs:369-378` (#525).
  - `docs/feature-matrix.md:167` "NPC / creature physics | ✗" — NPCs are driven through Rapier's KCC (`byroredux/src/systems/locomotion.rs:123,240`, M42.10/M42.11) and have ragdolls (row below it, `:169`).
  - `docs/feature-matrix.md:147` "3D spatial audio (kira 0.10)" — workspace is `kira = "0.12"` (`Cargo.toml:209`); ROADMAP crate map, dependencies.md and `crates/audio/src/lib.rs` comments already say 0.12.
  - Aside (same file, `:155`): per-material footsteps `FOOT` `✗` is plausible; not changed since the Wave-4 footstep work (`dd720f82d`) which touches Rapier surface materials, not FOOT records.
- **Status**: NEW
- **Age**: rows predate M28.5/M42.10 (2026-09-18) and the 2026-10 dependency refresh (`f7405adb7`).
- **Effort**: trivial
- **Impact**: the matrix is documented as a status floor; these rows are below the floor.
- **Related**: `docs/engine/dependencies.md:121` also says "kira 0.10's default set" (accurate: refers to 0.10's feature set; leave).
- **Suggested Fix**: flip `:128` to `✓` (or delete as a duplicate of `:124`), `:167` to `~` ("NPC locomotion via KCC; no NPC dynamics"), `:147` to "kira 0.12".

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **DOC**: `.claude/commands/_audit-validate.sh` still exits 0 after the edit

=== #5601 ===
OPEN | TD3-2026-10-11-05: `game-compatibility.md` per-game sections quote NIF counts 30-50% below the ROADMAP matrix the doc says it reconciles to
Labels: documentation,low,tech-debt,doc-rot
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments
- **Location**: `docs/engine/game-compatibility.md:126` (FNV "14,881"), `:156` (FO3 "10,989"), `:170` (Skyrim SE "18,862"), `:207` (FO4 "34,995 + 124,871 MeshesExtra")
- **Status**: NEW (the Oblivion row was fixed by #4358)
- **Age**: counts predate the 2026-08-28/29 sweep widening; the doc's own table at `:89-96` was refreshed by `e034de59a` (2026-10-06) and is correct.
- **Effort**: trivial
- **Evidence**: ROADMAP matrix: FNV 20 746 (11 archives), FO3 17 172 (6), Skyrim SE 33 468 (8), FO4 235 082 (8). The doc header says "ROADMAP wins ... Last reconciled 2026-08-26 (#2365)". FO3 is off by 36%, FNV 28%, Skyrim SE 44%, FO4 32%.
- **Impact**: two different "100% clean" denominators in one document; per-game audits quote the bullet.
- **Related**: #4358 (closed), #3726 (closed).
- **Suggested Fix**: replace the four bullets with a pointer to the table at `:89` (ROADMAP is the single home) or the matrix figures.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **DOC**: `.claude/commands/_audit-validate.sh` still exits 0 after the edit

=== #5602 ===
OPEN | TD3-2026-10-11-06: ROADMAP.md "Crate Map" lists 22 of 34 workspace members; CLAUDE.md's Workspace Structure omits 15 crates
Labels: documentation,low,tech-debt,doc-rot
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments
- **Location**: `ROADMAP.md:783-807` (Crate Map); `CLAUDE.md` "Workspace Structure" tree
- **Status**: NEW
- **Age**: map unchanged for the crates added since M44 (`boot-request`, `game-detect`, `settings-io`, `save`, `hkx`, `fsr3-sys`, `mod-runtime`, `menuxml`, `pex`, `sdk`, tools `byro-detect`, `byro-launcher`).
- **Effort**: small
- **Evidence**: workspace `members` block = 34 (matches ROADMAP Project Stats "34"). ROADMAP Crate Map misses the 12 above; CLAUDE.md additionally has no entry for `physics`, `spt`, `sfmaterial`, `facegen`, `audio`, `debug-ui`, `bgsm`/`hkx`, `byro-launcher`, `texture-upscale` (`for c in ...; grep -c "\b$c\b" CLAUDE.md` = 0). Several are the "young/unswept" crates this audit is told to weight.
- **Impact**: the two docs an auditor reads first for "where does X live" cannot answer it for `save`, `mod-runtime`, `sdk`, `pex`, `menuxml`, `physics`, `audio`.
- **Related**: #4351/#5105 (CLAUDE.md structure, closed — fixed file names, not coverage).
- **Suggested Fix**: add the 12 rows to ROADMAP's Crate Map (one line each; `_audit-owners.md` already has the ownership text to lift). For CLAUDE.md, keep it short: a single "Other crates" list; CLAUDE.md edits are a user decision.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **DOC**: `.claude/commands/_audit-validate.sh` still exits 0 after the edit

=== #5603 ===
OPEN | TD3-2026-10-11-07: `docs/contributing.md` and `docs/engine/testing.md` still recommend the bare `cargo test -p byroredux-core` that #3895 documented as silently dropping tests
Labels: documentation,low,tech-debt,doc-rot
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments (developer command)
- **Location**: `docs/contributing.md:111`, `docs/engine/testing.md:178,183`
- **Status**: NEW (same class as #3895 and #5107; those fixed CLAUDE.md/AGENTS.md only)
- **Effort**: trivial
- **Evidence**: CLAUDE.md:12-22 says `--features inspect` is load-bearing: a bare `cargo test -p byroredux-core` drops `animation::{player,stack}::inspect_tests` (the #486 round-trips) with no diagnostic. `grep -rn "features inspect" docs/*.md docs/engine` finds no mention outside audit reports.
- **Impact**: a contributor following the contributing guide or testing.md gets a green run that omits exactly the tests covering the `AnimationPlayer`/`AnimationStack` serialization they are editing.
- **Suggested Fix**: add `--features inspect` to those three commands (or one sentence pointing at CLAUDE.md's note).

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **DOC**: `.claude/commands/_audit-validate.sh` still exits 0 after the edit

=== #5604 ===
OPEN | TD3-2026-10-11-09: Index and cross-reference drift: `docs/engine/index.md` omits 27 of 81 docs, `testing.md` dispatch-test count is 36% low, one `streaming.rs:752` cite remains, `p6-loading-model.sh` is not in the smoke README
Labels: documentation,low,tech-debt,doc-rot
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments
- **Location**:
  - `docs/engine/index.md` (last edited 2026-09-14, `1b01ad64a`): no entry for `story-manager.md`, `dialogue-trees.md`, the six `charal-*-ruleset.md`, `playable-vertical-slice.md`, `rt-lighting-material-recovery.md`, `interior-godrays-status.md`, `exterior-readiness-plan.md`, `m47-*` (4), `p2-*`/`p4-*` fixtures, `sdk-v0.1-*` (2), `starfield-esm-*` (2), `nif-engine-translation-layer.md`, `near-term-action-plan.md`, `m27-phase1-survey.md`, `dependency-update-followups-plan.md` (loop: `for f in docs/engine/*.md; grep -q $b docs/engine/index.md`).
  - `docs/engine/testing.md:51`: "~67 tests today across shader, havok, interpolators, controllers, extra_data, nodes, effects, starfield" — `grep -rc "#\[test\]" crates/nif/src/blocks/dispatch_tests/` sums to 106 across 10 files (adds `legacy_particle.rs`, `mod.rs`).
  - `docs/engine/exterior-grid-streaming.md:96`: `world_pos_to_grid` "(`streaming.rs:752`)"; it is `byroredux/src/streaming/mod.rs:888`.
  - `docs/smoke-tests/README.md`: no entry for `p6-loading-model.sh` (CLAUDE.md names it and says all smokes follow the README pattern); `m41-ragdoll.sh` is in neither index (referenced only from `physal.md:354`).
- **Status**: NEW
- **Effort**: small (index) / trivial (the rest)
- **Impact**: the index is the declared entry point for engine internals; newer subsystems (Story Manager, dialogue, CHARAL rulesets) are discoverable only by `ls`.
- **Suggested Fix**: add the 27 rows; bump testing.md to the live count (or say "see `dispatch_tests/`" without a number); re-point the cite; add the two smoke scripts to the README.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **DOC**: `.claude/commands/_audit-validate.sh` still exits 0 after the edit

=== #5605 ===
OPEN | TD4-2026-10-11-01: Six backticked test/function names in four skills were renamed or deleted by today's fix wave. The validator prints them only as advisories
Labels: documentation,low,tech-debt,doc-rot
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 4, Audit-Finding Rot
- **Location** (skill:line, stale name, current name, commit):
  - `audit-starfield/SKILL.md:59`: `cdb_emissive_settings_weight_the_emissive_role`; now `cdb_emissive_settings_are_parked_not_forwarded_raw` (`byroredux/src/asset_provider/material/merge.rs:2074`); renamed by `4667b884b` (#5503).
  - `audit-exterior/SKILL.md:76`: `distant_water_hole_is_exactly_the_streaming_boundary`; now `distant_water_holes_only_actual_residency` (`cell_loader/water.rs:2019`); `e38ff49d2` (#5334) also deleted `distant_water_hole_radius`.
  - `audit-exterior/SKILL.md:71`: `recenter_lod_water`; now `refresh_lod_water` (`streaming/mod.rs:633`); `e38ff49d2` / `310723850` (#5289).
  - `audit-exterior/SKILL.md:33`: `inherit_all_up_chain`. `65cadde2f` (#5516) deleted this function, and the sentence calling it a flag-less twin "that the tests pin (open #5516)" is now wrong on both counts. The surviving guard is `oblivion_child_gets_the_inherit_all_stamp_and_eras_stay_real` (`crates/plugin/src/esm/cell/tests/wrld.rs:1043`).
  - `audit-oblivion/SKILL.md:100`: `oblivion_child_inherits_parents_water_without_pnam`; now `oblivion_child_inherits_parents_water_through_the_parse_stamp` (`env_translate.rs:2891`); `65cadde2f`.
  - `audit-scripting/SKILL.md:417`: `purge_despawns_pending_readoption_entities`; now `pending_readoption_victims_are_the_whole_subtree_closure` and `pending_readoption_victims_spare_the_player_rig` (`cell_loader/unload.rs:1242,1292`); `99cd9fd6c` (#5490). That commit also changed `purge_cinematic_retention_state(world)` to `(world, ctx)`.
- **Status**: NEW
- **Age**: all six renames are from 2026-10-11, 18:20 to 18:56 -03. The skills were last re-synced at 18:03 (`d84e7f163`, #5579). 15 commits landed after that re-sync, 13 of them fix commits.
- **Effort**: trivial
- **Description**: These skills cite guard names as "the regression pin". An auditor who greps for the name finds nothing, and cannot tell a deleted guard from a renamed one.
- **Evidence**:
  - `_audit-validate.sh` ADVISORY (audit skills) lists exactly these symbols: `cdb_emissive_settings_weight_the_emissive_role`, `distant_water_hole_is_exactly_the_streaming_boundary`, `inherit_all_up_chain`, `oblivion_child_inherits_parents_water_without_pnam`, `purge_despawns_pending_readoption_entities`, `recenter_lod_water`.
  - `git log -S<name> --format='%h %cs %s'` shows each name last present in the commit above.
  - The other two skill advisories are not rot: `FIXTURE_DATA_ENV` and `FIXTURE_GATES` live in `docs/smoke-tests/fixtures/*.env`, which the symbol scan does not read.
- **Impact**: The guard pointers rot within hours of a fix wave. No misled audit is demonstrated yet, so this stays LOW. It does leave the exterior, oblivion, scripting and starfield skills pointing at removed guards.
- **Related**: #5579 (the re-sync that is now behind); TD4-2026-10-11-02 (same sites). Prior TD4-2026-10-08-01 / #5473 (same class, fixed for pathspecs only).
- **Suggested Fix**:
  - Rename the six pointers to the successor names above.
  - Rewrite `audit-exterior:33`: drop the "inherit_all_up_chain twin … open #5516" clause, and say the Oblivion inherit-all is the parse-time PNAM stamp with one walk (`inherit_up_chain`).
  - Re-run `/audit-sync` after the current fix wave settles.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **DOC**: `.claude/commands/_audit-validate.sh` still exits 0 after the edit

