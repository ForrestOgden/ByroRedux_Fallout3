**HEAD**: d6362e652 · **Baseline**: `docs/audits/AUDIT_TECH_DEBT_2026-10-08.md` (HEAD `00f580e09`, 246 commits ago) · **Audited**: all 9 dimensions, delta-first (deep) · **Unchanged since baseline (skimmed)**: none (every dimension's `Paths:` had commits)

# Tech-Debt Audit — 2026-10-11

Nine dimension agents ran (three at a time) against a read-only tree. The only cargo runs were Dim 8's clippy passes, in a pinned
target directory that is now deleted. Nothing was edited and no engine was launched. Findings were deduplicated against
`issues_all.json` (500 `tech-debt` issues, any state), the open-issue list, targeted `gh --search` queries and the 10-08 report.
Every prior-report finding was re-verified against current code.

## Executive Summary

| Severity | NEW | Regression | Existing (grew, not re-filed) |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 4 | 0 | 0 |
| LOW | 43 | 3 | 3 (#5467, #5468, #5332) |

Total: 53 findings (47 NEW, 3 Regression, 3 Existing). A 54th, Dim 9's stale skill test names (TD9-04), was folded into TD4-01/TD4-02 (same sites).

- **The 10-08 debt list was mostly paid down, and the gates that landed with it are leaky.**
  - Fixed and closed: #5470 (space-run literals, 48 → 4), #5472 (split-wave doc cites), #5473 (skill pathspecs; the validator now checks them and has a self-test), #5474 (the 9 `--all-targets` lints), #5477 and the `cond_dump.rs` probe.
  - Three of the fixes were partial and show up as Regressions. #5469's weak-needle guard bans 4 spellings (34 inline `#[cfg(test)]` cuts survive in 5 others, TD2-01). #5470's gate exempts `main.rs` by file name (4 literals left, TD3-01). #5471 changed one count and left the contradiction in four files (TD3-02).
- **The CI clippy gate is red at HEAD** (TD8-01): 3 lints from today's commits (#5528 and #5539). The `byroredux` bin crate goes unlinted, so any lint in the most-churned crate is invisible until the gate is green.
- **Four MEDIUMs, all duplicated-lockstep state with a hand-fix history.**
  - TD2-02: six ambient-AI systems share one Pass-2 write-back, and `patrol` is a ~130-line copy of `wander` (#4189 had to edit all six).
  - TD2-05: `CellData` has no `Default`, so 16 full 33-field literals exist (#4480 was a compile break; two more literals arrived in the last 24 h).
  - TD7-01: the CPU water-wave sampler re-types `water.vert`'s tuning constants (#3207, #4728 and #5354 each fixed the divergence by hand).
  - TD7-02: `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME = 1366` is a hand-typed copy of a formula that evaluates to 1364 (#1198, #1284).
- **Production files over 2000 LOC went from 3 to 5.** `env_translate.rs` (1881 → 2186) and `save_io.rs` (1964 → 2011) are new crossers. `components.rs` (#5467) grew again. `effects.rs` (#5468) is unchanged. `shader_constants_data.rs` is the deliberate data table.
  - `audio/src/lib.rs` is on the watch list at 1958 (+274 in 3 days).
- **First sweep with the reachability recipe on the young crates (Dim 6)** found a dead event pipeline (`ScriptTimer`/`TimerExpired`: Papyrus `OnTimer` can never fire) and ~470 production lines of plugin `DataStore`/`DependencyResolver`/`PluginManifest` with no consumer. Four docs and a save-registry row present that code as the live load path.
- **Skill rot is a fix-wave artefact.** The `/audit-sync` re-sync landed at 18:03 on 10-11, then 15 commits made it stale: 6 renamed guards (TD4-01) and 34 closed issues still listed "known-open" across 12 skills (TD4-02).
- **Clean:** Dim 5 (24 markers, all false positives or in-grace prose; the MIT attribution block atop `triangle.frag` intact), every GPU-struct size claim, shader `#define` provenance, NIF version-gate literals, `cargo machete`, `unimplemented!/todo!()` = 0, all 98 console commands real.

## Baseline Snapshot (measured at `d6362e652`; `prod_loc` self-test ok)

```text
markers (TODO/FIXME/HACK/XXX/TBD/WIP/KLUDGE): 24   (10-08: 24)   15 XXXX + 2 FO4 HACK FourCC + 3 upstream-FIXME + 1 documented TBD + 3 TODO prose
allow(dead_code):                             44   (10-08: 34)   +11 cfg(test) TypeId markers (lock_tracker.rs), -1 (#5287) — verified
unimplemented!/todo!():                        0   (10-08: 0)
#[ignore] tests:                             297   (10-08: 281)  +16, all game-data/device gated; 0 bare
files >2000 production LOC:                    5   (10-08: 3)    env_translate, save_io new; components, effects carry over; shader_constants_data = data table
test files >2000 total LOC:                   74   (10-08: 69)
production fns >200 LOC:                     193   (Dim 1's brace-matched scanner; same scanner on the 10-08 tree: 188 — the old "175" is not comparable)
clippy --workspace -D warnings (1.96.0):       3   renderer ×2 + physics ×1 (TD8-01) — bin crate unlinted
clippy --workspace --all-targets (1.96.0):     5   additional test-target lints (physics ×2, nif, plugin)
cargo machete:                             clean
_audit-validate.sh:                           OK — 0 STALE; 73 basename / 8 skill-symbol / 285 docs-engine / 1 tree advisories
```

## Top 10 Quick Wins (trivial / small)

1. **TD8-01** (trivial): fix the 3 gate lints (`dynamic_rgba.rs:38-39`, `physics/world/mod.rs:889`) and the 5 test-target lints. The gate is red.
2. **TD4-01 + TD4-02** (trivial/small): rename the 6 stale guard pointers in four skills and move 34 closed issues out of the "known-open" lists. Make the skill-symbol advisory in `_audit-validate.sh` a failing gate (TD4-03). Re-run `/audit-sync` once the fix wave settles.
3. **TD3-02** (trivial): finish #5471 in `feature-matrix.md`, `npc-spawn-ai-packages.md:9`, `ROADMAP.md:61,94` and the `game-loop.md` schedule table.
4. **TD3-07** (trivial): add `--features inspect` to the 3 bare `cargo test -p byroredux-core` commands in `contributing.md` and `testing.md`.
5. **TD3-01 / TD3-04 / TD3-03** (trivial): re-wrap the 4 `main.rs` literals and drop its gate exemption; flip the 3 under-stated feature-matrix rows; fix "12 parallel-stage systems" → 9.
6. **TD7-02** (trivial): derive the bind-inverse upload cap from the slot pool (`MAX_SKIN_SLOTS`) and add a `const _` assert.
7. **TD2-05** (small): `#[derive(Default)]` on `CellData`, then collapse the 14 test literals.
8. **TD9-01 / TD9-02** (trivial/small): add `cargo check -p byroredux-core --no-default-features --lib --tests` to CI, and a `golden` option to `rt-correctness.yml`.
9. **TD2-04 / TD2-12 / TD7-08** (trivial): one `entity_global_form_id` next to its inverse; delete groundcover's hand-rolled `write_storage_buffer` copies; use `ClusterCullPipeline::grid_buffer_size()`.
10. **TD6-03 / TD8-03 / TD6-06** (trivial): delete the orphan `PerkList` component, the write-only `PerkRecord.perk_flags` and the test-only `latest_slot`, or correct their docs.

## Top 5 Medium Investments

1. **TD2-02**: consolidate the six ambient-AI Pass-2 tails into `navmesh_path.rs`, and fold `patrol` into a generic oscillating-walk in `wander.rs` (with TD2-03/TD2-14, the same family).
2. **TD7-01**: give the water-wave tuning one home in `crates/core/src/ecs/components/water.rs`, feed it to the GLSL header through `shader_constants_data.rs`, and share `authored_scroll_pair` between `render/water.rs` and `physics/water.rs`.
3. **TD1-01 and TD1-02 splits**, `env_translate.rs` first (no `include_str!` self-scans, so the cleanest) and `save_io.rs` second (6 self-scan tests need repointing, see the finding), plus the existing #5467 and #5468.
4. **TD2-06 + TD2-09**: make `load_cell_with_masters` a wrapper over `InteriorCellApplyJob::begin`; collapse the 9 delivery loops in `extensions/dispatch.rs`.
5. **TD2-10 + TD2-11**: add a `layout_binding` helper plus a shared `try_or_cleanup!` and per-frame-UBO helper in the renderer (~150 binding blocks in 22 files, 12 macro copies).

## Deferred (gated on an in-progress milestone or a product decision)

- **TD6-02** (plugin `DataStore` family): keep-or-delete depends on whether the Redux-native manifest/UUID plugin format (a stated goal) is still on the roadmap. Either way the docs need a status banner now.
- **TD6-01** (`ScriptTimer`): wiring `StartTimer`/`OnTimer` is M47 scripting work; the doc correction is not deferred.
- **TD6-04** (Starfield WTHS value reader): EXAL Starfield exterior work; file the tracker now.
- **TD1-02 `save_io.rs` split**: it is the P5 hardening hot file (5 fix commits in 3 days), so consider doing it after P5 settles.
- Not filed: `shader_constants_data.rs` (2657 LOC) is the single-home data table #5097 created, and the 24 `XXXX`/`HACK` marker hits are protocol tags.

## Findings

### MEDIUM

### TD2-2026-10-11-02: Six ambient-AI systems carry the same Pass-2 write-back (Transform + NavPath + stuck timer), and `patrol` is a ~130-line copy of `wander`
- **Severity**: MEDIUM (table row 1, borderline). The family already diverged: #5452 found force-greet and Eat/Sleep followed no navmesh waypoints while the docs claimed they did, and #4189 had to be applied by hand to six files.
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/systems/{wander,patrol,travel,follow,guard,escort}.rs` (Pass 2 blocks: `wander.rs:423-466`, `patrol.rs:233-277`, `travel.rs:342-382`, `follow.rs:302-340`, `guard.rs:288-322`, `escort.rs:426-466`); `patrol.rs:93-232` vs `wander.rs:284-422`
- **Status**: NEW
- **Age**: Guard/Patrol landed `aaed15038` 2026-07-16; the NavPath write-back `a58bb4cf6` (2026-09-19, "Fix #4189") edited all six identically (11 lines each).
- **Effort**: medium
- **Description**:
  - Each system ends with `// ── Pass 2: apply writes (each a scoped single-type lock). ──`, then a byte-identical Transform loop (translation, optional rotation; 5 of 6 verbatim, `follow.rs` differs only by wrapping the tuple), then a verbatim `NavPath` take/insert/remove loop including the 6-line `#4189` comment (`grep -c 'd.nav_path.take()'` = 1 in each of the six files).
  - `wander` and `patrol` additionally share a verbatim `WalkStuckTimer` write-back (`wander.rs:459`, `patrol.rs:268`), the Pass-1a gather (queries, `speed`, `stuck_secs`, `resolve_cached_waypoints`), and the Pass-1b `step_oscillating_wander` + state copy + `nav_path` construction. They differ only in component types (`WanderBehavior/WanderState` vs `PatrolBehavior/PatrolState`) and the radius field name. `patrol.rs`'s module doc says it avoids a copy of the wander state machine, but only the inner step is shared.
  - The newest movers (`eat_sleep`, `forcegreet`) use `navmesh_path::cached_walk_step` (added `6316719ed`, 2026-10-11), a third idiom for the same resolve-cache / step / write-back sequence.
- **Evidence**: `grep -c 'd.nav_path.take()' byroredux/src/systems/*.rs` -> follow/guard/escort/travel/patrol/wander = 1 each; `git show --stat a58bb4cf6` -> 6 files, +48/-18, one hunk shape.
- **Impact**: the next change to the cache contract (residency generation, #3256; take-not-clone, #4189) is a 6-file edit, and a miss is silent. The #5452 divergence already shows it.
- **Related**: #4189, #3256, #5452, #2033.
- **Suggested Fix**:
  - In `byroredux/src/systems/navmesh_path.rs` (home of `resolve_cached_waypoints` and `cached_walk_step`) add `pub(crate) fn write_back_walk<D>(world, decisions: &mut [D], entity, translation, rotation, nav_path)` taking accessor closures, or a small `trait WalkDecision { fn entity(&self); fn translation(&self); fn rotation(&self); fn take_nav_path(&mut self); fn stuck_secs(&self) }`, and call it from all six Pass-2 tails.
  - Collapse `patrol_system_inner` into a generic `oscillating_walk_system_inner::<B, S>` in `wander.rs` (it already owns `OscillateWalk` and `step_oscillating_wander`), with `patrol.rs` supplying the component pair via a two-method trait. Delete `PatrolDecision`/`PatrolScratch`.

### TD2-2026-10-11-05: `CellData` has no `Default`, so 16 full 33-field literals exist (14 in tests); two were added in the last 24 hours
- **Severity**: MEDIUM (table row 1: duplicated construction with a divergent-fix history - #4480 was a red bin-crate test build because six `CellData` fixtures missed a new field, fixed by editing each literal rather than consolidating).
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/plugin/src/esm/cell/mod.rs:191` (struct, 33 fields); literals: `cell/wrld.rs:453`, `cell/walkers.rs:488` (production); tests: `cell/tests/merge.rs:24`, `cell/tests/cell_for_refr.rs:59`, `byroredux/src/cell_loader/exterior.rs:602,779,1049,1705`, `lod_support.rs:371`, `lgtm_fallback_tests.rs:49`, `interior_spawn_tests.rs:73`, `pkin_expansion_tests.rs:41`, `reference_state.rs:1177`, `scene/world_setup.rs:1643`, `systems/story_events.rs:203` (`c85d50356`, 2026-10-11), `systems/eat_sleep.rs:693` (`bcaf63aa8`, 2026-10-10)
- **Status**: NEW (related closed #4480)
- **Age**: each new feature adding a `CellData` field re-edits all of them; `deleted_refs` (2026-08-23) was the last.
- **Effort**: small
- **Description**: `#[derive(Debug)]` only (`mod.rs:190`). Every field type is `Default`-able (`String`, `Option<_>`, `bool`, `Vec<_>`, `CellLighting`/`LandscapeData`/`CellOwnership` behind `Option`, `NavmRecord`/`PathGridRecord` in `Vec`), so a derive is enough. Four of the test constructors are named `empty_cell` with identical bodies up to 2 fields.
- **Evidence**: `grep -rn --include='*.rs' 'deleted_refs:' crates byroredux | grep -v 'pub deleted_refs'` -> 16 literals in 13 files.
- **Impact**: adding the next field is a 16-site edit and a bin-crate test compile break if one is missed - already happened once (#4480), and two new literals arrived in the last day despite it.
- **Related**: #4480, #3864 (same fix for `NifHeader`).
- **Suggested Fix**: `#[derive(Default)]` on `CellData` (plus its field types if any lack it), then rewrite the 14 test literals as `CellData { form_id, grid: Some(..), ..Default::default() }`; keep `wrld.rs`/`walkers.rs` exhaustive-destructure style only if the compiler-checked field coverage is wanted there. Collapse the four `empty_cell` helpers into `CellData::empty(form_id)` under `#[cfg(any(test, feature = "test-fixtures"))]` in `crates/plugin/src/esm/cell/mod.rs`.

### TD7-2026-10-11-01: The CPU water-wave mirror re-types water.vert's wave tuning and render/water.rs's scroll prep; no shared constant, and the shader allowlist's "no Rust counterpart" claim is now false

- **Severity**: MEDIUM (duplicated logic with divergent bug-fix history: #3207, #4728, #5354)
- **Dimension**: 7
- **Location**:
  - `crates/physics/src/water.rs:417-478` (`authored_wave_height_with_weather`)
  - `crates/renderer/shaders/water.vert:191-235`
  - `byroredux/src/render/water.rs:238-247,284-289`
  - `crates/renderer/src/shader_constants.rs` (allowlist rows `("water.vert", "DEFAULT_SCROLL_A"/"DEFAULT_SCROLL_B")` under "Water + caustics")
- **Status**: NEW (related closed: #3207, #4728, #5354; open #5562 is about the sample position, not constants)
- **Age**: 11be73600 / a70f80d94, 2026-08-20 ("synchronize buoyancy with weather waves"). Re-touched by cff1f9c42 on 2026-10-11 (#5354).
- **Effort**: small
- **Description**: The physics sampler is documented as "CPU counterpart of `water.vert`". It re-types these values:
  - `DEFAULT_SCROLL_A = 0.0228254` and `DEFAULT_SCROLL_B = 0.0286531`.
  - Rate clamp `0.25..4.0`.
  - Wave mix `0.60 / 0.40`.
  - Phase-B rate `TAU * 0.75` (the shader writes `4.7123890`).
  - Amplitude clamp `32.0`.
  - Spatial floors `1/2048` and `1/4096`.
  - Fallback directions `[1, 0.35]` and `[-0.4, 1]`.
  - The `±32` angular-rate clamp.
  - The flowmap clamp `0.05..8.0`.
  - The weather coupling `0.65` onto scroll B.
  - The `0.5` wind amplitude boost.

  Beyond that, the whole "authored scroll prep" block is duplicated between the GPU upload path (`render/water.rs`) and the CPU sampler: flowmap clamp `0.05..8.0`, angular clamp `±32`, `rotate_scroll`, layer weights `0.65` (B) and `0.45` (C).
- **Evidence**:
  - `physics/water.rs:457-458` has `weather_scroll[0] * 0.65`, and `render/water.rs:284-285` has the same `* 0.65` (and `* 0.45` at `:288-289`).
  - `physics/water.rs:466-468` divides by `0.0228254` / `0.0286531`; `water.vert:205-206` declares the same two numbers.
  - `physics/water.rs:478` has `phase_a.sin() * 0.60 + phase_b.sin() * 0.40`, and `water.vert:224-225` has `sin(phaseA) * 0.60` / `sin(phaseB) * 0.40`.
  - `grep -rn "0.0228254\|0.0286531" crates byroredux` shows the shader, the physics file, tests and fixtures. Nothing in `shader_constants_data.rs`.
  - No Rust test reads `water.vert` for these values. `grep -rn 'water.vert' --include='*.rs'` finds only descriptor-reflection tests and `svgf.rs`'s include.
  - The allowlist comment says each exempted constant has "no Rust counterpart that could drift out of step". That was true before 2026-08-20; it is false for `DEFAULT_SCROLL_A/B` now.
- **Impact**: A retune of any one copy (for example the B-layer weather coupling, or a wave-mix change in the shader) moves the visible crest but not the buoyancy/submersion crest. This is the exact failure #3207, #4728 and #5354 each fixed by hand. The only coherence mechanism is prose comments ("in lockstep with `water.vert`").
- **Related**: #3207 (render/water.rs re-implemented `weather_wave_adjustment`), #4728 (phase sign), #5354, #5562, and `WEATHER_SCROLL_PER_BU_PER_S` / `WaterKind::weather_scroll_for` (the part that was already consolidated).
- **Suggested Fix**:
  - Add the wave-tuning block (default scrolls, rate clamp, `WAVE_MIX_A/B`, phase-B ratio `0.75`, amplitude/angular/flowmap clamps, spatial floors, fallback directions) to `crates/core/src/ecs/components/water.rs`, next to `WEATHER_SCROLL_PER_BU_PER_S`. The scroll-B and scroll-C weather weights (`0.65`, `0.45`) belong there too (WATAL territory).
  - Re-export it through `shader_constants_data.rs`, which already imports from `byroredux_core::ecs::components`, so the GLSL header carries it. Delete the two `DEFAULT_SCROLL_*` allowlist rows.
  - Have `render/water.rs` and `physics/water.rs` call one `authored_scroll_pair(material, weather_scroll, t)` helper that returns `(scroll_a, scroll_b)`, instead of repeating the clamp/rotate/couple code.

### TD7-2026-10-11-02: `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME = 1366` is a hand-typed copy of a formula that evaluates to 1364, and the formula is re-typed again beside it

- **Severity**: MEDIUM (the lockstep failure it guards has already shipped twice: #1198, #1284)
- **Dimension**: 7
- **Location**:
  - `crates/renderer/src/vulkan/scene_buffer/constants.rs:69,104`
  - `crates/renderer/src/vulkan/context/mod.rs:81` (`SKIN_MAX_SLOTS`)
  - `byroredux/src/main.rs:1155`
  - 8 test files under `byroredux/src/render/` (`bone_palette_overflow_tests.rs:96`, `variant_pack_gating_tests.rs:15`, `parallax_alpha_gate_tests.rs:56`, `static_mesh_fx_skip_tests.rs:10`, `tint_alpha_gate_tests.rs:28`, `water_wave_params_tests.rs:72,174`, `fog_curve_propagation_tests.rs:13`, `render_origin_shared_tests.rs:17`)
- **Status**: NEW
- **Age**: a3c2836a6, 2026-05-28 (#1284)
- **Effort**: trivial
- **Description**: The slot-pool capacity is `MAX_TOTAL_BONES / MAX_BONES_PER_MESH - 1`, which is 1364 (slot 0 is the identity). It is computed three separate times in non-test code: `SKIN_MAX_SLOTS` (`context/mod.rs:81`), `main.rs:1155`, and the eight test copies. The per-frame first-sight upload cap `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME` is not derived from any of them.
- **Evidence**:
  - `constants.rs:104` has `= 1366`. Its doc says "(= `MAX_TOTAL_BONES (196608) / MAX_BONES_PER_MESH (144)`)". That quotient is 1365.33, floor 1365, and the real pool is 1364. 1366 matches neither.
  - `grep -rn '1366\|1364' --include='*.rs' crates byroredux` finds no test or assert tying the cap to the pool size.
  - History: the cap was 16, then 227, then 1366. Each bump was a hand edit to follow `MAX_TOTAL_BONES`. #1198 records the symptom of letting them diverge: "one-frame bind-pose glitch when more than 16 skinned NPCs first-sighted in a single frame".
  - `core/src/ecs/resources/skin_slot_pool.rs:180` tells the next person to "Bump MAX_TOTAL_BONES". Doing so leaves 1366 stale. Staging size is `1366 × 144 × 64 B` at `buffers.rs:691`. `upload.rs:526` silently defers the excess past the cap, so the failure is a one-frame glitch, not a crash.
- **Impact**: Today the cap is 2 slots over capacity, about 18 KB, so it is harmless. The risk is the documented lever: raise `MAX_TOTAL_BONES` or change `MAX_BONES_PER_MESH` and the upload cap stays at 1366 (under-cap, glitch returns) or over-allocates staging.
- **Related**: #1192, #1198, #1284, #1299 (all closed).
- **Suggested Fix**:
  - Define `pub const MAX_SKIN_SLOTS: usize = MAX_TOTAL_BONES / MAX_BONES_PER_MESH - 1;` in `scene_buffer/constants.rs`.
  - Set `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME = MAX_SKIN_SLOTS`.
  - Define `SKIN_MAX_SLOTS` from the same constant, and have `main.rs:1155` and the eight tests import it instead of re-deriving.
  - Add a `const _: () = assert!(MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME >= MAX_SKIN_SLOTS);`.

### LOW

### TD1-2026-10-11-01: `byroredux/src/env_translate.rs` crossed 2000 production LOC (1881 → 2186, +305 in six days)
- **Severity**: LOW
- **Dimension**: 1
- **Location**: `byroredux/src/env_translate.rs` (5909 lines, 2186 production; `mod climate_tod_hours_tests;` sibling + 3700 lines of inline tests)
- **Status**: NEW (on the 10-08 report's > 1800 list at 1881)
- **Age**: eight commits, 2026-10-08..10-11: `67afa4b95` #5374 (+86), `302394f94` #5424 (+143), `b3e679dba` #5423 (+102), `217bcbc9f` #5388 (+196),
  `7ef007898` #5343 (+25), `2ca57c4bc` #5496 (+157), `65cadde2f` #5516/17/19 (+89 −94), `e38ff49d2` #5334 (+88).
- **Effort**: medium
- **Description**: the module doc calls it "the exterior environment translation boundary", and it has five contiguous, mostly independent blocks.
  No function is large (biggest 134 LOC), so this is a pure file-level split with no function surgery:
  1. Terrain/object LOD + distant water (L40–420: `TerrainLodLayout`, `translate_terrain_lod_textures`, `default_water_for_worldspace`,
     `resolve_distant_water`, `translate_lod_water`, `lod_source_worldspace`) ≈ 380
  2. Worldspace/climate/default-weather inheritance chain (L414–840: `mod pnam`, `inherit_up_chain`, `resolve_worldspace_climate`,
     `resolve_exterior_climate`, `oblivion_climate_rungs`, `region_climate_for_center`, `resolve_cell_climate`, `resolve_default_weather`,
     `worldspace_name_chain`) ≈ 430 — this is the block that took 5 of the 8 recent commits
  3. WATR → `WaterMaterial` (L843–1490: colours, specular, noise/rain, layer motion, kind/flow classification, `compose_flow_scrolls`,
     `resolve_water_material`) ≈ 650
  4. Sky / sun / cell-lighting / weather translation (L1489–2050: `translate_exterior_cell_lighting`, `translate_sky`, `fog_*`/`precipitation_*`,
     `weather_sky_state`, `climate_tod_hours`, `translate_weather`, `ImageSpaceSources`) ≈ 560
  5. Procedural fallbacks (L2051–2200: `FB_*` constants, `procedural_fallback_{cell_lighting,sky,weather}`) ≈ 150
- **Evidence**: `prod_loc byroredux/src/env_translate.rs` → 2186 (baseline tree: 1881); `grep -nE '^(pub\(crate\) )?fn ' …` shows the five runs above;
  largest fn `resolve_water_noise_and_rain` 134 LOC.
- **Impact**: every EXAL fix (LOD, climate, water, sky) now lands in one file that also hosts 3700 lines of tests; the water-reading half and the
  climate-inheritance half are independent concerns that edit-conflict only because they share a file.
- **Related**: EXAL doc `docs/engine/exal.md` §7; precedent splits `streaming/`, `volumetrics/{fog_clusters,combustion}.rs`. include_str! hazard:
  grep found NO `include_str!` of `env_translate.rs` itself (the `default_weather_rule_tests` module at L5555 reads `exterior.rs`/`world_setup`
  via `strip_test_modules(include_str!(...))` — those are *other* files and are unaffected). So the split is safe w.r.t. self-scan tests.
- **Suggested Fix**: `byroredux/src/env_translate/{mod.rs (docs + `pub(crate) use`), lod.rs, climate.rs, water.rs, sky_weather.rs, fallback.rs}`;
  move the in-file tests with their block (the `tests` mod is 3700 lines — split it along the same five axes, `climate_tod_hours_tests.rs` is
  already a sibling). Keep `mod pnam` private to `climate.rs`/`lod.rs` (it is used by both blocks 1 and 2 — put it in `mod.rs` as `pub(super)`).
  Use the sed-extract line-range method (feedback_safe_large_function_split) and run `cargo fmt` once at the end.

### TD1-2026-10-11-02: `byroredux/src/save_io.rs` crossed 2000 production LOC (1964 → 2011) — and four test modules self-scan its text
- **Severity**: LOW
- **Dimension**: 1
- **Location**: `byroredux/src/save_io.rs` (2067 lines, 2011 production; tests live in `save_io/*_tests.rs`)
- **Status**: NEW (was 1964 on 10-08 list)
- **Age**: `03e51dfad` #5394 (+6), `557ead21d` #5435 (+21), `620970028` (+20), `99cd9fd6c` #5490 (±0), all 10-09..10-11.
- **Effort**: medium (the include_str! pins make it more careful than env_translate)
- **Description**: the file is the whole save/load app layer: five responsibilities in contiguous runs.
  1. Registry + constants: `MUTABLE_DELTA_COLUMNS` (L84), `PRE_RELOAD_RESOURCES` (L174), `build_save_registry` (L363–626, 259 LOC, a flat
     registration table at nesting 1) ≈ 560
  2. Save-dir discovery + `SaveState` / `PendingSaveLoadSlot` resources (L272–360) ≈ 90
  3. Player pose + player-save-action queue (L196–262, 652–800: `PlayerPose`, `PendingPlayerSaveActions`, `capture_/apply_player_pose`) ≈ 220
  4. Console commands `SaveCommand` (execute = 201 LOC) / `SaveInfoCommand` / `LoadCommand`, quicksave/quickload helpers, save validation
     (`validate_form_ids`, `validate_cinematic_entity_refs`) (L804–1378) ≈ 570
  5. Live load-apply: `reload_interior_session`, `reload_exterior_session` (133), `execute_pending_save_loads` (**306 LOC**, L1744–2050),
     story-cursor / gear-handoff scratch resets (L1379–2050) ≈ 670
- **Evidence**: `prod_loc` → 2011; `scan.py` fn lengths: `execute_pending_save_loads` 306, `build_save_registry` 259, `SaveCommand::execute` 201.
- **Impact**: moderate — it is the P5 hardening hot file (5 fix commits in 3 days), and `execute_pending_save_loads` is a 306-line function that
  is the single most pinned function in the app layer.
- **Related**: **include_str! self-scan hazards — a naive split breaks 6 tests**:
  - `save_io/live_reload_tests.rs:416,602` find `"pub fn execute_pending_save_loads("` in `include_str!("../save_io.rs")` and then scan **from there to
    EOF**, searching for `restore_resources_subset(… PRE_RELOAD_RESOURCES)`, `reload_exterior_session(`, `reset_story_location_cursor(world)`.
  - `save_io/serde_default_guard_tests.rs:87` derives `registered_type_names` from the registry text in `save_io.rs`; also `:932` asserts a scanned
    path `ends_with("save_io.rs")`.
  - `save_io/round_trip_tests.rs:200` requires `save_io.contains(reconciler)` for each removed-at-runtime delta column.
  - `cell_loader/unload.rs:1478,1490` `source_of("save_io.rs")` + `fn_window(.., "fn reload_interior_session(", "\nfn reload_exterior_session(")`.
  After a split these need a `concat!(include_str!(...))` SOURCES constant (the `fragment.rs:SOURCES` pattern, #3854) or must be pointed at the
  new file that owns the scanned item, otherwise they go vacuous or red.
- **Suggested Fix**: `save_io/{mod.rs, registry.rs (1+2), player.rs (3), commands.rs (4), live_load.rs (5)}`; keep `execute_pending_save_loads`,
  `reload_*_session` and `PRE_RELOAD_RESOURCES` together in `live_load.rs` so the "scan from fn to EOF" tests keep their window.
  Do the SOURCES-constant repoint in the same commit. Separately, extract `execute_pending_save_loads` stages (restore → purge → reload → apply
  deltas → post-reload resets) into named helpers — but only with the existing pin tests moved first.

### TD1-2026-10-11-03: `byroredux/src/components.rs` still over 2000 and has grown again (2035 → 2075)
- **Severity**: LOW
- **Dimension**: 1
- **Location**: `byroredux/src/components.rs`
- **Status**: Existing: #5467 (OPEN, no comments, not acted on). **Premise re-verified**: file is 2637 lines / 2075 prod; +40 since the issue was
  filed, from `1291f370a` (+38, `ImpactConfig`/impact audio state), `886edcded` (+10), `c173147da` (+7), `0eb8896bb` (−7).
- **Effort**: medium
- **Description**: do not re-file. Update #5467's body with the current figure and the structure below (domain boundaries re-measured at HEAD):
  - render/material markers + texture handles (L25–460), terrain components (L461–570)
  - cell lighting / region ambient / DALC / weather / sky / cloud resources (L574–1640) + three test modules (≈ 390 test lines)
  - name/subtree/cell-root indexes (L1639–1715), input (L1715), audio config/state/scratch (`FootstepEmitter` … `WaterDisturbanceScratch`, L1758–1980)
  - render tuning (`LightTuning`/`ExposureTuning`/`RenderDebugControl`, L1980–2070)
  - AI / animation runtime (`SeatReservations` … `IdleClipCatalog`, L2070–2316), navmesh (L2316–2437) + tests
  - audio is now the fastest-growing block (impact sound config landed in `1291f370a`).
- **Related**: **include_str! hazards**: `components.rs:2595` (`fx_hash_guard_tests`) asserts the `NameIndex`/`SubtreeCache` `FxHashMap` field text in its own
  file — moves with the index block, must stay in the file that owns `NameIndex`; `commands/env_health_tests.rs:466`
  `declared_field_names(include_str!("../components.rs"), "WeatherSkyState")` — must be repointed to the new weather file. A `components/game_time.rs`
  sibling already exists (L3 `mod game_time;`), so the directory form `components/{mod.rs, render.rs, terrain.rs, environment.rs, audio.rs, ai_anim.rs, navmesh.rs}` is
  the established shape.
- **Suggested Fix**: as in #5467 (domain split); environment resources first — it is ≈ 1050 of the 2075 LOC and the only block with its own test modules.

### TD1-2026-10-11-04: `crates/scripting/src/fragment/effects.rs` still 2018 production LOC
- **Severity**: LOW
- **Dimension**: 1
- **Location**: `crates/scripting/src/fragment/effects.rs` (2018 lines, all production)
- **Status**: Existing: #5468 (OPEN, no comments). Premise verified: unchanged since 10-08 (2018 → 2018; no commits touched it since). Do not re-file.
- **Effort**: medium
- **Description**: re-measured layers: resolvers (L11–230: `resolve_quest*`, `resolve_object`, `resolve_actor`, `resolve_npc_actor`, …), deferred queue
  (`DeferredFragmentEffects` L284–620, with `apply_at_depth` **188 LOC / nesting 6**), per-family appliers (L713–1856: global, inventory, placement, scene,
  lock, player-control, vehicle/cinematic 154, AI/combat, quest-scoped 182), driver `apply_effects` (L1857, 145 LOC, nesting 6).
  Function-level: no function > 200, but three at nesting 6 (`apply_at_depth`, `apply_effects`, `apply_scene_effect`).
- **Related**: **include_str! hazard** — `crates/scripting/src/fragment.rs:83-90` already builds `SOURCES = concat!(include_str!("fragment/state.rs"),
  include_str!("fragment/effects.rs"), …)` (#3854) for scan tests; a split into `fragment/effects/*.rs` must add each new file to that concat, and
  `fragment.rs:94` ("Pins that SOURCES actually covers the module") is the guard that will catch a missed one. `translate/effects.rs:3584` scans a
  *different* `effects.rs` (translate's, not this one) — unaffected.
- **Suggested Fix**: `fragment/effects/{mod.rs (apply_effects + apply_effect dispatch), resolve.rs, deferred.rs, global_inventory.rs, placement_lock.rs,
  scene_cinematic.rs, ai_quest.rs}`; update `SOURCES` in the same commit.

### TD1-2026-10-11-05: `crates/scripting/examples/mq101_conformance.rs::run` is a 1454-line, depth-15 function — the worst in the workspace
- **Severity**: LOW
- **Dimension**: 1
- **Location**: `crates/scripting/examples/mq101_conformance.rs:448-1902` (file prod LOC 1927, on the watch list)
- **Status**: NEW (file was on the 10-08 > 1900 list at 1923/1927; the function itself was never called out)
- **Effort**: medium
- **Description**: `fn run()` is the whole probe — ESM load, BSA/PEX/decompiler ingress, per-fragment lowering, quest/scene/package checks, voice archive, and
  report — in one body, with brace depth 15. Everything before it (L117–447) is already small, well-named helpers (`mq101_player_callback_contract`, `effect_kind`,
  `statement_shape`…), so the shape is obviously "helpers extracted, driver never decomposed". It is a manual conformance harness (needs
  a Skyrim install), so correctness risk is low; the cost is that the probe is hard to extend for the next vertical slice. It is also the only
  `examples/` item in the top 20.
- **Evidence**: `scan.py` → `(1454, depth 15, crates/scripting/examples/mq101_conformance.rs, run, L448)`; second-worst nest anywhere is 10.
  `git log -- <file>`: last touched 2026-10-06 (`9813af435`).
- **Impact**: low (example, not shipped). Listed because the SKILL's nesting > 5 / fn > 200 rules have no examples carve-out.
- **Related**: none filed (`gh issue list --search mq101_conformance` → #3498, #2658, both closed doc issues).
- **Suggested Fix**: split `run` into one `fn check_*` per stage returning `Checks` deltas (`check_ingress`, `check_scripts`, `check_fragments`,
  `check_scenes`, `check_voice`), pass `&mut Checks`; flattens depth to ≤ 4.

### TD2-2026-10-11-01: #5469's weak-needle guard bans 4 spellings; 34 inline `#[cfg(test)]` cuts in 23 files survive in 5 others, 5 of them with a whole-file fallback
- **Severity**: LOW. No live wrong answer today (each first `#[cfg(test)]` is a trailing `mod`).
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/workspace_hygiene_tests.rs:700-756` (`no_source_scan_cuts_with_the_weak_cfg_test_needle`); survivors:
  `debug-server/src/registration.rs:463,511,556,641`, `debug-server/src/evaluator.rs:944`, `byroredux/src/cell_loader/spawn/mesh_instance.rs:1749,1780,1831,1931`,
  `renderer/src/vulkan/context/resources.rs:836,887,932`, `renderer/src/vulkan/svgf.rs:1784,2122`, `renderer/src/vulkan/presentation.rs:916`,
  `renderer/src/vulkan/context/build_and_upload_instances.rs:1933`, `nif/src/import/walk/lights.rs:265,318`, `nif/src/import/collision/mod.rs:786,830`,
  `physics/src/water.rs:1347`, `physics/src/world/mod.rs:1481`, `physics/src/sync.rs:2603`, `byroredux/src/material_translate.rs:1522,2841`,
  `app_step.rs:1487,1510`, `systems/cinematic.rs:1932`, `systems/combat_ai.rs:689`, `npc_spawn/tests.rs:2827`, `cell_loader/terrain_lod_btr.rs:616`,
  `cell_loader/load.rs:1398`, `boot/world.rs:538`, `boot/schedule/mod.rs:657`
- **Status**: Regression of #5469 (CLOSED 2026-10-11, 334458b3f). The prior report's TD2-01 residue is resolved in the spellings it listed; the same class remains in other spellings.
- **Age**: sites span 2026-08 to 2026-10; the guard landed 2026-10-11.
- **Effort**: small
- **Description**:
  - The guard builds its needles as `find("<attr>")`, `rfind("<attr>")`, `split("<attr>")`, `split_once("\n<attr>")` where `<attr>` is the bare attribute.
  - It misses: `split_once("#[cfg(test)]")` (18 sites, the commonest), `.find("\n#[cfg(test)]")` (5), `.split("\n#[cfg(test)]")` (3),
    `find("#[cfg(test)]\nmod tests")` (3), and the strong needle written inline (`split_once("#[cfg(test)]\nmod ")`, `find("\n#[cfg(test)]\nmod ")`: `physics/world/mod.rs:1481`, `physics/sync.rs:2603`, `boot/schedule/mod.rs:657`, `material_translate.rs:2841`).
  - The `\n#[cfg(test)]` spellings still truncate at a `#[cfg(test)] fn` or `use` (the hazard `source_scan.rs` documents).
  - Whole-file fallbacks (the #4842 vacuous-guard class #5100/#5469 removed elsewhere) persist at `nif/.../lights.rs:265,318` and `nif/.../collision/mod.rs:786,830` (`.unwrap_or(src)`), and `physics/world/mod.rs:1481` (`.map_or(src, ..)`, which is exactly `production_text_or_whole`).
  - `nif`, `physics`, `debug-server` and `renderer` all depend on `byroredux-core` (checked in each Cargo.toml), so none of these needs the allowlist's "no core dep" excuse.
- **Evidence**: `grep -rn --include='*.rs' -E '(split_once|split|find|rfind)\("(\\n)?#\[cfg\(test\)\]' crates byroredux tools | grep -v -E 'source_scan.rs|crates/(pex|facegen)/|texture_registry_tests|dynamic_rgba|boot/mod.rs:655'` -> 36 lines (34 after dropping two comment lines) in 23 files.
  `grep -rln 'source_scan' crates/nif crates/debug-server` -> none.
- **Impact**: the hygiene test's doc says "no source may spell the weak cut outside the helper"; it is false, and the next guard that copies `registration.rs` or `mesh_instance.rs` inherits the weak needle.
- **Related**: #5469, #5100, #4842, #5164.
- **Suggested Fix**:
  - Route each survivor through `byroredux_core::source_scan::production_text` (file known to carry a test module), `production_text_or_whole` (may not), or `strip_test_modules` (interleaved, e.g. `app_step.rs`, `material_translate.rs:2841`). Renderer/physics use their `crate::source_scan` re-export.
  - Replace the guard's needle list with a regex over `#\[cfg\(test\)\]"` within a `find|rfind|split|split_once` call so any spelling trips it. Keep the allowlist unchanged.
  - Leave `pex/decompile/boolean.rs:845`, `facegen/src/lib.rs:120` allowlisted (no core dep).

### TD2-2026-10-11-03: Per-procedure `*_DEFAULT_RADIUS = 512.0` x5, a duplicated repath threshold, and the walk-speed fallback chain x9
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `wander.rs:89`, `patrol.rs:51`, `travel.rs:85`, `guard.rs:77`, `escort.rs:94` (`512.0`); `follow.rs:77` `FOLLOW_REPATH_THRESHOLD` vs `escort.rs:99` `ESCORT_COLLECT_REPATH_THRESHOLD` (both 64.0); walk-speed fallback at `wander.rs:335`, `patrol.rs:149`, `travel.rs:257`, `guard.rs:216`, `escort.rs:245`, `follow.rs:174`, `forcegreet.rs:167`, `eat_sleep.rs:344`, `combat_ai.rs:195`
- **Status**: NEW
- **Age**: constants 2026-07/08 (M42); `forcegreet`/`eat_sleep` fallbacks 2026-10.
- **Effort**: trivial
- **Description**:
  - Each doc comment says "same value as `wander.rs::WANDER_DEFAULT_RADIUS`" and `escort.rs:96` says "kept as its own constant rather than imported". The value is one fact (v0 default package radius), copied five times; guard/patrol/escort docs cross-reference each other to keep them in step by hand.
  - Nine call sites repeat `world.query::<WalkSpeed>() ... .map(|s| s.0).unwrap_or(LOCOMOTION_WALK_SPEED)` (7 identical `walk_speed_q.as_ref().and_then(..).map(|s| s.0).unwrap_or(..)` chains).
- **Evidence**: `grep -n 'DEFAULT_RADIUS: f32 = 512.0' byroredux/src/systems/*.rs` -> 5 lines; `grep -n 'LOCOMOTION_WALK_SPEED)' byroredux/src/systems/*.rs` -> 9 sites.
- **Impact**: low; a retune of the default radius is a 5-file edit.
- **Related**: TD2-2026-10-11-02.
- **Suggested Fix**: in `byroredux/src/systems/locomotion.rs` add `pub(crate) const AI_PACKAGE_DEFAULT_RADIUS: f32 = 512.0` and `pub(crate) fn walk_speed_of(q: Option<&impl Query>, entity) -> f32` (or `WalkSpeed::or_default(Option<&WalkSpeed>)` next to the component); point the 5+9 sites at them and have `escort.rs` import `FOLLOW_REPATH_THRESHOLD`.

### TD2-2026-10-11-04: `entity_global_form_id` exists as byte-identical private fns in two crates plus an inline third copy; its inverse is the public one
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/scripting/src/fragment/effects.rs:68`, `byroredux/src/systems/eat_sleep.rs:450`, inline at `byroredux/src/interaction.rs:1440-1445`; inverse at `crates/scripting/src/condition.rs:634` (`pub fn resolve_entity_by_global_form_id`)
- **Status**: NEW
- **Age**: `effects.rs` copy 2026-09-10 (`f5127c1cc`); `eat_sleep.rs` copy 2026-10-10 (`bcaf63aa8`, "Fix #5500"), whose doc says "the same keying `resolve_entity_by_global_form_id` searches (#3278's helper shape)".
- **Effort**: trivial
- **Description**: all three do `FormIdComponent` -> `FormIdPool::resolve(..).local.0`. The `effects.rs` doc itself calls it "the inverse of `resolve_entity_by_global_form_id`, keyed the same way so the two agree" - which is exactly why the pair belongs in one file.
- **Evidence**: `sed -n 68,80p crates/scripting/src/fragment/effects.rs` and `sed -n 450,458p byroredux/src/systems/eat_sleep.rs` are the same 7 lines.
- **Impact**: a change in keying (e.g. the PlayerRef sentinel that `resolve_entity_by_global_form_id` special-cases at #4694) updates one direction only.
- **Related**: #3278, #4694, #5500.
- **Suggested Fix**: add `pub fn entity_global_form_id(world, entity) -> Option<u32>` to `crates/scripting/src/condition.rs` beside `resolve_entity_by_global_form_id`, re-export from `lib.rs:62`, delete the two private copies and the `interaction.rs` inline closure. Consider making the player entity map back to `0x14` there too, mirroring the inverse.

### TD2-2026-10-11-06: `InteriorCellApplyJob::begin` is a ~54% line-for-line twin of `load_cell_with_masters`, and fixes have to land in both
- **Severity**: LOW. Same call set in both (see Evidence), no divergence found yet.
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/cell_loader/load.rs:410-806` (`load_cell_with_masters`) vs `load.rs:840-1157` (`InteriorCellApplyJob::begin` / `finish`)
- **Status**: NEW
- **Age**: job added `6cdf91ec9` 2026-09-03 ("Fix #3671").
- **Effort**: medium
- **Description**: both do plugin-path chaining, `parse_record_indexes_in_load_order`, the cell lookup with the three-tier "cells containing substring / prefix / first 20" error (`load.rs:493` and `:894`, identical text), lighting/sky/water/precombined, `RegionAmbientRes` + `dispatch_region_ambient_music`, `ensure_globals_resource`, resolver install, `CurrentCellContext`, phase-timing log. The module doc states the intent: the sync path "remains synchronous" for a job that can be retained between frames. `FrameTimeBudget::unlimited()` (`work_budget.rs:31`) already exists to run the budgeted path to completion.
- **Evidence**: normalized-line comparison of the two bodies: 125 of the sync body's 231 code lines (>12 chars) occur in the job body. Function-call set difference: sync-only `load_references`, `stamp_cell_root`; job-only `load_references_budgeted`, `advance`, `finish`, `stamp_cell_root_range`, `unload_cell` (mechanics only). `git show -U0 777351ec1` (#5484), `afd6a73f7` (#4416) and `387bb9d9c` (#4909) each carry hunks in BOTH functions.
- **Impact**: every interior-load fix is a two-place edit; callers `debug_load.rs:408` and `save_io.rs:1444` still depend on the sync copy.
- **Related**: #3671, #5484, #4416.
- **Suggested Fix**: make `load_cell_with_masters` a thin wrapper over `InteriorCellApplyJob::begin` + `advance(.., &mut FrameTimeBudget::unlimited())` returning the `InteriorCellApplyProgress::Complete` payload; extract the lookup-with-suggestions error into `fn find_interior_cell<'a>(index, key) -> anyhow::Result<&'a CellData>` in the same file. Verify the `load_references` vs `load_references_budgeted` difference is only the slicing.

### TD2-2026-10-11-07: "clone the clip's channels, then `attach_animation_sinks`, then read the phase" is pasted at three NIF animation spawn sites
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/scene.rs:769-802`, `byroredux/src/scene/nif_loader.rs:607-637`, `byroredux/src/cell_loader/spawn.rs:869-896`; callee `byroredux/src/anim_convert.rs:129`
- **Status**: NEW
- **Age**: all three predate 2026-10-08 (#3345 phase wiring).
- **Effort**: trivial
- **Description**: each site scopes a registry read, clones four channel vectors (`bool/float/color/texture_flip`) into a tuple, drops the guard, calls `attach_animation_sinks(world, &b, &f, &c, &t, Some(ctx), Some(provider), root)`, then separately does `registry.get(handle).map(|c| c.phase).unwrap_or(0.0)` into `AnimationPlayer::new(h).with_phase(phase)`. `anim_convert.rs:123-127` already says every production caller passes both `Some`.
- **Evidence**: `grep -rn 'texture_flip_channels.clone()' byroredux/src` -> exactly those three sites.
- **Impact**: a fifth channel kind (the sink signature already takes four slices) is a 3-site edit plus the helper.
- **Related**: #3345.
- **Suggested Fix**: add `pub(crate) fn attach_clip_sinks_and_phase(world, clip_handle, ctx, tex_provider, root) -> f32` in `anim_convert.rs` that does the registry read, clone, `attach_animation_sinks`, and returns the phase; the three sites call it and build the `AnimationPlayer` from the returned phase.

### TD2-2026-10-11-08: "Teleport the player body" (Transform + zero KCC momentum + `set_kinematic_translation`) is written four times
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/systems/character.rs:878-909` (`snap_character_body_to_camera`), `character.rs:1020-1038` (`ground_character_body_at`), `byroredux/src/save_io.rs:760-782`, `byroredux/src/commands/view.rs:470-489`
- **Status**: NEW
- **Age**: `character.rs` 2026-08-14 (`264f44fd3`); `save_io` 2026-06-21 (`93b06b959`).
- **Effort**: small
- **Description**: each sets `Transform.translation` (and rotation in 3 of 4), writes `GlobalTransform` in one, then `c.vertical_velocity = 0.0; c.is_grounded = false; c.wants_jump = false;` with the same "clear stale free-fall momentum" comment, then `byroredux_physics::set_kinematic_translation`. `character.rs` already hosts two of them and `pin_camera_above_body`.
- **Evidence**: `grep -rn 'wants_jump = false' byroredux/src` -> `save_io.rs:776`, `character.rs:902,1034`, `commands/view.rs:482`.
- **Impact**: a new `CharacterController` field that needs clearing on teleport (a swim state, a stuck timer) is a 4-site edit.
- **Related**: #2134 (lock-order note shared by the first two).
- **Suggested Fix**: add `CharacterController::clear_momentum(&mut self)` in `crates/physics`, and `pub(crate) fn place_character_body(world, body, pos, rotation: Option<Quat>) -> bool` in `systems/character.rs` (Transform + GlobalTransform + clear + kinematic sync). Route all four sites through it; `save_io` and `view.rs` keep their own camera handling.

### TD2-2026-10-11-09: `extensions/dispatch.rs` repeats one delivery loop nine times (eligibility, guest entry, log drain, commit)
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/extensions/dispatch.rs:251-290, 323-358, 392-430, 453-486, 505-541, 553-585, 663-710, 726-760` (eight `stats.deliveries += 1` sites plus the update loop); helper home `byroredux/src/extensions/commands.rs:138` (`enter_guest`), `mod.rs:142` (`delivery_commit_context!`)
- **Status**: NEW (the earlier #3863 consolidated the snapshot prologue; this is the delivery body)
- **Age**: split out in `55cbc0d6c` 2026-09-11; touched 4 times since 10-08.
- **Effort**: medium
- **Description**: each `dispatch_*` builds `for hosted in &mut self.components`, tests `hosted.receives_X && grants().contains(EVENTS_SUBSCRIBE_CAPABILITY) && status() == Active` (8 copies of the three-line predicate), `stats.deliveries += 1`, `enter_guest`, `set_entity_projections(entity_projections.clone())`, calls one `on_*`, drains `take_logs()` into `ExtensionDiagnostic::Log` (9 identical 7-line blocks), then `apply_delivery_result(hosted, result, LifecyclePhase::X, &principal, delivery_commit_context!(self, stats))`. Only the flag, event constructor and phase differ.
- **Evidence**: `grep -c EVENTS_SUBSCRIBE_CAPABILITY dispatch.rs` = 8; `grep -c 'take_logs().into_iter().map' dispatch.rs` = 9; `grep -c 'apply_delivery_result(' dispatch.rs` = 9.
- **Impact**: a lifecycle-wide rule (a new capability gate, a fault policy) is a 9-site edit; the predicate drifting in one loop would silently deliver to a revoked component.
- **Related**: #3863, #3843.
- **Suggested Fix**: add `fn deliver(&mut self, phase: LifecyclePhase, stats: &mut ExtensionDispatchStats, projections: &[EntityProjection], eligible: impl Fn(&HostedComponent) -> bool, call: impl FnMut(&mut HostedComponent) -> DeliveryResult)` to `dispatch.rs` (split-borrow `self.components` vs `self.diagnostics`), and a `fn drain_logs(&mut self.diagnostics, hosted)` for the log block. Each `dispatch_*` becomes ~10 lines.

### TD2-2026-10-11-10: ~150 hand-written `vk::DescriptorSetLayoutBinding::default()` blocks in 22 files; no builder, though `write_*` has one
- **Severity**: LOW (large mechanical surface, no live divergence)
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/renderer/src/vulkan/volumetrics/init.rs` (29), `scene_buffer/buffers.rs` (22), `svgf.rs` (18), `composite.rs` (11), `caustic.rs` (11), `taa.rs` (10), `bloom.rs` (9), `reflect.rs` (8), `skin_compute.rs` (6), `compute.rs` (5), others (`ssao.rs` 3, `presentation.rs` 3, `exposure_meter.rs` 3, ...). One local helper exists: `groundcover/construct.rs:405 storage_binding`.
- **Status**: NEW
- **Age**: spans the renderer's life; `svgf.rs`/`taa.rs`/`caustic.rs`/`ssao.rs` are the oldest compute passes.
- **Effort**: medium
- **Description**: each binding is a 5-line chain `.binding(n).descriptor_type(T).descriptor_count(1).stage_flags(COMPUTE)`; 112 of them are `COMPUTE`-stage with count 1. `descriptors.rs` consolidated the write side (`write_combined_image_sampler`, `write_storage_image`, `write_storage_buffer`, `write_uniform_buffer`, #1752/#2073) and the barrier side but has no layout-side builder, so every pass restates its layout and its pool sizing separately (`DescriptorPoolBuilder::from_layout_bindings` already consumes the slice).
- **Evidence**: `grep -rn --include='*.rs' 'DescriptorSetLayoutBinding::default()' crates/renderer/src | grep -v test | wc -l` -> 150+; `-A4 ... | grep -c 'ShaderStageFlags::COMPUTE'` -> 112.
- **Impact**: hand-copied layouts are the thing `reflect::validate_set_layout` (#427) exists to catch after the fact; a builder prevents the class and removes ~600 lines.
- **Related**: #427, #1752, #2073, #4110.
- **Suggested Fix**: in `crates/renderer/src/vulkan/descriptors.rs` add `pub fn layout_binding(binding: u32, ty: vk::DescriptorType, stages: vk::ShaderStageFlags) -> vk::DescriptorSetLayoutBinding<'static>` and `pub fn compute_binding(binding, ty)`; migrate pass by pass (start with `svgf.rs`, `taa.rs`, `caustic.rs`, `ssao.rs`, `bloom.rs`), replacing `groundcover/construct.rs::storage_binding` with it.

### TD2-2026-10-11-11: Twelve identical local `try_or_cleanup!` macros and an 8-pass per-frame param-UBO loop in the renderer's pass constructors
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: macros: `ssao.rs:162`, `taa.rs:251`, `svgf.rs:480`, `caustic.rs:412`, `bloom.rs:300`, `compute.rs:108`, `exposure_meter.rs:75`, `sky_cube.rs:234`, `composite.rs:422`, `groundcover_bench.rs:465`, `groundcover/construct.rs`, `volumetrics/init.rs` (all `crates/renderer/src/vulkan/`); UBO loop: `ssao.rs:209-219`, `taa.rs:313-323`, `caustic.rs:458-468`, `svgf.rs:553-563`, `composite.rs:693`, `water.rs:537`, `bloom.rs:1077`, `volumetrics/init.rs:318,758`
- **Status**: NEW
- **Age**: 2026-07 onward (each new pass copied its neighbour).
- **Effort**: small
- **Description**: `macro_rules! try_or_cleanup { ($expr:expr) => { match $expr { Ok(v) => v, Err(e) => { unsafe { partial.destroy(device, allocator) }; return Err(e.into()); } } } }` is redefined verbatim (the SAFETY comment even varies in wording only) in 12 constructors. The `let param_size = size_of::<XParams>(); for _ in 0..FRAMES { let buf = try_or_cleanup!(GpuBuffer::create_host_visible(device, allocator, param_size, UNIFORM_BUFFER)); partial.param_buffers.push(buf); }` block follows in at least five.
- **Evidence**: `grep -rn --include='*.rs' 'macro_rules! try_or_cleanup' crates/renderer/src | wc -l` -> 12; `grep -rn --include='*.rs' 'param_buffers.push' crates/renderer/src | grep -v test` -> 12 lines.
- **Impact**: low. A change to the partial-teardown contract (reverse-order destroy, the `Err` conversion) is a 12-site edit.
- **Related**: TD2-2026-10-11-10.
- **Suggested Fix**: one `macro_rules! try_or_cleanup { ($partial:ident, $device:expr, $allocator:expr, $expr:expr) => ... }` in `vulkan/mod.rs` (or `helpers.rs`, `pub(crate)`, `#[macro_use]`), and `pub(crate) fn create_per_frame_uniforms<T>(device, allocator, frames) -> Result<Vec<GpuBuffer>>` in `vulkan/buffer.rs` beside `create_host_visible`.

### TD2-2026-10-11-12: groundcover descriptor writes hand-roll `write_storage_buffer` twice as local fns and seven times inline
- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `crates/renderer/src/vulkan/groundcover_bench.rs:1132-1176` (local `buffer_write` plus 5 inline image writes), `groundcover/frame.rs:245-256` (local `write`), `groundcover_models.rs:747-751` (inline loop)
- **Status**: NEW
- **Age**: `groundcover_bench.rs` 2026-09-15 (`13e00a24b`, which routed its barriers through `descriptors.rs` but left the writes); `frame.rs` 2026-09-06.
- **Effort**: trivial
- **Description**: the local `buffer_write(set, binding, info)` and `write(set, binding, info)` are signature- and body-identical to `descriptors::write_storage_buffer(dst_set, binding, info)` (`descriptors.rs:89`). The five image writes in the bench are `write_combined_image_sampler` (x3) and `write_storage_image` (x3) by another name. `groundcover_bench.rs:73` imports two barrier helpers from `descriptors` but none of the writers.
- **Evidence**: `grep -rn --include='*.rs' 'WriteDescriptorSet::default()' crates | grep -v 'vulkan/descriptors.rs'` -> 9 lines in exactly these three files.
- **Impact**: low, but it is the exact class #1752/#2073 closed.
- **Related**: #1752, #2073, #2071.
- **Suggested Fix**: delete the two local closures and the nine inline builders; import `write_storage_buffer`, `write_combined_image_sampler`, `write_storage_image` from `super::descriptors`.

### TD2-2026-10-11-13: Absolute-offset little-endian readers re-implemented ~16 times across 8 crates; no shared home
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

### TD2-2026-10-11-14: Navmesh test fixtures `zup_from_yup` x7 and `two_triangle_quad_navm` x6 despite a shared `test_tiles` module
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

### TD2-2026-10-11-15: CLI flag lookup: `cli_args::parse_string_arg` (with the `--flag=value` trap warning) is bypassed by ~27 raw `position(|a| a == "--...")` sites and four `--master` collectors
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

### TD3-2026-10-11-01: The #5470 space-run gate exempts `byroredux/src/main.rs` wholesale, hiding 4 collapsed-continuation assertion messages
- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments (text hygiene)
- **Location**: `byroredux/src/main.rs:332, 338, 409, 417`; exemption at `byroredux/src/workspace_hygiene_tests.rs` (`no_string_literal_embeds_a_space_run`, `exempt` list)
- **Status**: Regression of #5470 (CLOSED) — the sweep and gate landed in `3efd17e5b` but left these.
- **Age**: `f8ea9d46f1`, 2026-08-31 (all four lines).
- **Effort**: trivial
- **Description**: the gate exempts 23 files by name as "column-aligned tables". 22 of them are (`display.rs`, `nif_stats.rs`, `commands/assets.rs`, ...); `main.rs` is not. Its only
  4 hits are prose test messages with 14-18 spaces from the exact bug class #5470 fixed.
- **Evidence**: `byroredux/src/main.rs:332`: `"a single blocking frame must stand out by orders of magnitude,              got {blocked}"`; `:338` `...got              {heavy}"`;
  `:409` `...format string —                  BENCH_GPU_KEYS names...`; `:417` `...not a              bare `tlas_ms=`, now that `gpu_tlas_build=` shares the line              (#3629)`.
  Recount of the whole tree with the TD3-01 regex (`{6,}`): 44 hits, of which 40 are `\n`-bearing source-scan needles / aligned tables and 4 are these. Detection:
  `grep -nE '"[^" ][^"]*[^ "] {6,}[^ "][^"]*"' byroredux/src/main.rs | grep -v '\\n'`.
- **Impact**: cosmetic (failure messages only), but it shows the gate's per-file exemption is a blind spot: any future collapsed literal added to an exempt file passes.
- **Related**: #5470, #5481 (open; its `prepare.rs` literal is already gone).
- **Suggested Fix**: re-wrap the 4 literals with `\` continuations and delete `"byroredux/src/main.rs"` from `exempt`. Also consider narrowing the exemption of
  `commands/depth.rs` (the one prose literal there was fixed; only the `:159` table row remains) to a line-level allowlist instead of a whole-file skip.

### TD3-2026-10-11-02: M42 procedure status is still self-contradictory after #5471: feature-matrix says "10 of ~17 have a runtime" then lists seven and calls "the remaining 10" parse-only
- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments
- **Location**:
  - `docs/feature-matrix.md:89-105` — count was bumped to "10 of ~17" but the list still names only the seven locomotion procedures and the next paragraph says "The remaining 10 procedures (Find/Eat/Sleep/Accompany/UseItemAt/Ambush/FleeNotCombat/CastMagic/Dialogue/UseWeapon) are parse-only — each blocked on a subsystem (... dialogue ...)". 10 + 10 > 17; Eat/Sleep/Dialogue are named as parse-only.
  - `docs/engine/npc-spawn-ai-packages.md:9` — "exactly seven procedures execute anything at runtime", contradicted at `:522` ("Ten procedures of ~17 execute").
  - `ROADMAP.md:61` ("Seven AI package procedures run by default") and `:94` ("AI beyond locomotion: 10 of ~17 package procedures need subsystems that do not exist (item use, package combat, magic, dialogue)") — the open set is 7 and dialogue now exists (M42 row `:289` is correct).
  - `docs/engine/game-loop.md:114-170` live-schedule table: the PostUpdate NPC row lists sandbox/wander/travel/follow/escort/guard/patrol only; `make_eat_sleep_system()` (`byroredux/src/boot/schedule/post_update.rs:129`, exclusive PostUpdate) is absent.
- **Status**: Regression of #5471 (CLOSED 2026-10-10; commit `3bb9748f0` changed one count and one table row only; its message claims "Both now say 10 of ~17 ... open list drops to the 7").
- **Age**: `00f580e09` (Eat/Sleep, 2026-10-08), `14cff35ae` (Dialogue, 10-07).
- **Effort**: trivial
- **Description**: `feature-matrix.md` is the status floor and `npc-spawn-ai-packages.md` is the reference the gameplay audit is told to believe; both still tell an auditor that `eat_sleep_system` / `forcegreet_system` are not supposed to exist.
- **Impact**: same as the original TD3-02; held at LOW (no audit shown misled).
- **Related**: #5471, #5367, #5376, #5390.
- **Suggested Fix**: in feature-matrix.md add Eat/Sleep and Dialogue (force-greet) to the run-by-default list and rewrite the "remaining" sentence to the 7 truly missing procedures; change `npc-spawn-ai-packages.md:9` to "ten"; ROADMAP:61 "Ten ...", :94 "7 of ~17 ... (item use, package combat, magic)"; add eat/sleep to the game-loop.md PostUpdate row.

### TD3-2026-10-11-03: ecs.md and architecture.md still say the engine has "12 parallel-stage systems"; it has 9
- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments (authoritative engine doc)
- **Location**: `docs/engine/ecs.md:503`, `docs/engine/architecture.md:135`
- **Status**: NEW
- **Age**: number is the M27 Phase-1 figure (`docs/engine/m27-phase1-survey.md:9` "12 parallel-stage systems (3 declared today, 9 undeclared)"); first falsified by #3111 which serialised the weather writer.
- **Effort**: trivial
- **Evidence**: `grep -rn "scheduler.add_to_with_access(" byroredux/src | wc -l` = 9; `boot/schedule/mod.rs:590` "The nine `add_to_with_access` registrations"; guard `the_parallel_system_table_covers_every_parallel_registration` pins the table to the registration count; `scheduler_access_tests.rs` floors `parallel_system_count() >= 9` ("floor 9 after #3111").
- **Impact**: the lock-order/concurrency audits cite ecs.md as authoritative for the analyzable population; 12 overstates it by a third.
- **Suggested Fix**: say "the engine's nine parallel-stage systems" (or drop the number and point at `PARALLEL_SYSTEMS` in `boot/schedule/mod.rs`).

### TD3-2026-10-11-04: Three `docs/feature-matrix.md` rows under-state shipped code (UV scrolling, NPC/creature physics, "kira 0.10")
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

### TD3-2026-10-11-05: `game-compatibility.md` per-game sections quote NIF counts 30-50% below the ROADMAP matrix the doc says it reconciles to
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

### TD3-2026-10-11-06: ROADMAP.md "Crate Map" lists 22 of 34 workspace members; CLAUDE.md's Workspace Structure omits 15 crates
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

### TD3-2026-10-11-07: `docs/contributing.md` and `docs/engine/testing.md` still recommend the bare `cargo test -p byroredux-core` that #3895 documented as silently dropping tests
- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments (developer command)
- **Location**: `docs/contributing.md:111`, `docs/engine/testing.md:178,183`
- **Status**: NEW (same class as #3895 and #5107; those fixed CLAUDE.md/AGENTS.md only)
- **Effort**: trivial
- **Evidence**: CLAUDE.md:12-22 says `--features inspect` is load-bearing: a bare `cargo test -p byroredux-core` drops `animation::{player,stack}::inspect_tests` (the #486 round-trips) with no diagnostic. `grep -rn "features inspect" docs/*.md docs/engine` finds no mention outside audit reports.
- **Impact**: a contributor following the contributing guide or testing.md gets a green run that omits exactly the tests covering the `AnimationPlayer`/`AnimationStack` serialization they are editing.
- **Suggested Fix**: add `--features inspect` to those three commands (or one sentence pointing at CLAUDE.md's note).

### TD3-2026-10-11-08: ROADMAP.md is 808 lines against its 800-line cap, and the session-97 close left it over
- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments (also Dim 4)
- **Location**: `ROADMAP.md` (808 lines); cap at `.claude/commands/session-close/SKILL.md:264,328`
- **Status**: Existing: #5332 (OPEN, filed at 818)
- **Evidence**: `wc -l ROADMAP.md` = 808; last ROADMAP commits `c43c25624` (session 97 close, 10-11), `4b9472909`, `455b473a1`, `dd720f82d`, `939b644b5`. No always-run gate exists: `grep -rn "ROADMAP" byroredux/src/workspace_hygiene_tests.rs` has no line-count check. README is 580 (cap 600) — OK.
- **Impact**: the cap is enforced only by a session-close step the close itself just failed.
- **Suggested Fix**: as in #5332 — trim ≥ 8 lines and add the `wc -l` ceilings to `workspace_hygiene_tests`. Not re-filed.

### TD3-2026-10-11-09: Index and cross-reference drift: `docs/engine/index.md` omits 27 of 81 docs, `testing.md` dispatch-test count is 36% low, one `streaming.rs:752` cite remains, `p6-loading-model.sh` is not in the smoke README
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

### TD4-2026-10-11-01: Six backticked test/function names in four skills were renamed or deleted by today's fix wave. The validator prints them only as advisories
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

### TD4-2026-10-11-02: 12 skills carry "known-open / open #N / cite, don't re-file" callouts for issues that are now CLOSED, and some describe code defects that are fixed
- **Severity**: LOW
- **Dimension**: 4, Audit-Finding Rot ("Existing:/open issue" callouts)
- **Location**: only the issue refs that are CLOSED yet presented as open are listed. Each site's other refs were checked (the OPEN ones such as #5505, #5523, #5520, #5073, #5351, #5526, #5439, #5011, #5459, #5285, #5534 are correct).
  - `audit-ecs:116`: #5513 ("Known-open … no visited set"). #5513 was closed 10-11 04:20Z (`9da80470d`).
  - `audit-ecs:256` and `audit-save:286`: #5490 ("since #5384 that list holds roots only, so the bare `despawn_batch` orphans their subtrees"). #5490 was closed 10-11 19:35Z (`99cd9fd6c`).
  - `audit-esm:262`: #5515 ("per-quest FNAM/UNAM … is open"), #5514 ("DIFF mis-decode open"). Both closed 10-11 (`0edd1dede`, `f0f8dceef`).
  - `audit-exterior:33`: #5519 (open doc rot), #5496 (LOD rings ignore PNAM 0x02), #5516. Also `:34` #5517 and `:84` #5518. All closed (`65cadde2f`, `2ca57c4bc`, `5a549e664`).
  - `audit-fnv:61` #5492, `:62` #5494, `:100` #5499, `:102` #5500 and #5495. All closed 10-10/10-11 (`dae08000d`, `5356c8780`, `f7ca4a60f`, `bcaf63aa8`, `0ba1dd3af`).
  - `audit-fo3:29`: #5428, #5429, #5433, #5521, #5494, #5499, #5496, #5518. All closed; the open ones in that list, #5520 and #5073, are right.
  - `audit-fo4:89`: #5493 and #5498 (#5523 is open, correct).
  - `audit-oblivion:28`: #5504, #5350, #5516, #5335, #5519 (the open ones #5505, #5351, #5526, #5439, #5011 and #5073 are correct).
  - `audit-performance:28` #5451 ("open"), `:81` #5447/#5448/#5449 ("Known open, do not re-file"), `:105` #5291 and #5528 ("open"), `:129` "#5249's transmission shadow trace … is unmeasured … (#5401)".
    - #5401 was closed 10-11 16:28Z with a comment "Measured, not material", so the "unmeasured" claim is false.
    - #5451/#5291/#5528 were closed 10-11 (`4b9472909`, `dd759b762`, `e588e5e67`).
  - `audit-renderer:133`: #5528 ("open"). `:214`: #5506 and #5532 ("Known open: the toe still crosses zero …; engine docs still show the pre-#5482 order"). Both closed 10-11 15:23Z (`38f711a93`, `b27e52c71`).
  - `audit-save:111`: #5458 ("Known-open (cite, don't re-file)"), closed 10-11 19:23Z (`0aa844dbb`).
  - `audit-gameplay:17` and `audit-audio:19`: #5432 ("nothing stops a voice line on close / topic change / death"; "no … path calls `stop_sounds_for`"). Fixed by `40b77828b`, and `dialogue_voice.rs:332` now calls `stop_sounds_for`.
- **Status**: NEW (the prior report's TD4-01 was a different class). The `d84e7f163` re-sync claims "verified 2026-10-10" for lists that the 10-10 and 10-11 fix commits then invalidated.
- **Age**: closures dated 2026-10-10 to 10-11 (commits listed above).
- **Effort**: small
- **Description**: These callouts tell an auditor to "cite, don't re-file" a gap. `_audit-common.md` § Deduplication step 3 says a CLOSED hit means "verify the fix is in place; if regressed, Regression of #N". A reader who trusts the callout would skip a regression check, or cite a closed issue as the open owner.
- **Evidence**: a script over `.claude/commands/**/*.md`:
  - It reads the state of every `#NNN` from `gh issue list --state all --limit 6000` (5,467 issues).
  - It looks at segments that start at `known-open | still open | open: | Existing` and end at "Fixed since" or end-of-line.
  - It lists CLOSED numbers in the "open" part (29 raw hits). I then hand-triaged the hits: those that really claim "open" came to 34 distinct issues across the 12 skills above, and the rest are historical or "closed" mentions.
  - Spot-checks: `gh issue view 5490/5513/5432/5401` all CLOSED, each with a matching `Fix #N` commit in `git log --grep`.
- **Impact**: audit time and dedup quality. No wrong issue has been filed from it yet. It is a mid-wave snapshot problem: the re-sync is hours old and already behind.
- **Related**: TD4-2026-10-11-01; #5579; `_audit-common.md` line 51 ("a rotting fact is checkable and correctable").
- **Suggested Fix**:
  - Move each closed ref from "Known-open" to the sibling "Fixed since 2026-10-08 — verify, don't re-file" list. `audit-fo3:29` and `audit-oblivion:28` already have that list.
  - Rewrite the code-defect clauses to their post-fix wording (`audit-ecs:116`, `audit-ecs:256`, `audit-save:286`, `audit-gameplay:17`, `audit-audio:19`, `audit-performance:129`).
  - Make `audit-sync/prep.sh` print every `#N` in a "known-open | open" segment whose state is CLOSED (the script above is the prototype). `audit-sync` then has a mechanical close-out check instead of relying on a hand read.

### TD4-2026-10-11-03: The validator's skill-symbol advisory is the only signal for renamed guards, and its output buries it among false positives
- **Severity**: LOW
- **Dimension**: 4, Audit-Finding Rot (gate coverage)
- **Location**: `.claude/commands/_audit-validate.sh` (ADVISORY audit-skills block, plus the basename block at the top of the output).
- **Status**: NEW (the prior report's TD4-01 covered pathspecs only)
- **Effort**: small
- **Description**:
  - The "audit skills" symbol advisory had 6 true positives out of 8 (TD4-01). The two others are `*.env` constants that the source scan does not read.
  - The "deleted-file refs" advisory has 73 entries. Its 3 skill entries are all non-defects:
    - `triangle_early.frag` is the extractor truncating `triangle_early.frag.spv` (5 sites in audit-performance and audit-renderer).
    - `ImportedMaterial.sh` is the truncated `ImportedMaterial.shader_type` (audit-skyrim:60).
    - `streaming.rs` at audit-nif:345 is a deliberate historical mention of the split.
  - The remaining 70 are `docs/engine` entries, many of them non-path tokens such as `time.sh`, `quest.sh` and `mem.frag`.
  - A reader scanning the output learns to ignore the advisory sections.
- **Evidence**: `/tmp/audit/tech-debt/d4_validate.txt` (the 73-entry basename list and the 8-entry skill symbol list).
- **Impact**: The gate answers "OK" while real renames wait in an advisory block. This is the TD4-2026-10-08-01 failure mode, in a smaller form.
- **Suggested Fix**:
  - Treat the skill-symbol advisory (8 entries) as a failing gate, and allow-list the `.env` constants by also scanning `docs/smoke-tests/fixtures/*.env`.
  - Teach the basename extractor to keep a `.spv` suffix and to ignore the `Type.field` shape.
  - Italicise `streaming.rs` at audit-nif:345.
  - Keep the `docs/engine` advisory advisory.

### TD6-2026-10-11-01: `ScriptTimer` / `TimerExpired` / `timer_tick_system` run every frame but nothing ever starts a timer or reads the expiry - Papyrus `OnTimer` can never fire
- **Severity**: LOW (amplifier considered: feature is documented "shipped", but no panic path -> stays LOW)
- **Dimension**: 6
- **Location**: `crates/scripting/src/timer.rs:17-60`, `crates/scripting/src/events.rs:86`, `crates/scripting/src/translate/tables.rs:71` (`CanonicalEvent::Timer`), `byroredux/src/boot/schedule/early.rs:109-112`; docs `docs/engine/m47-0-design.md:99`, `docs/engine/scripting.md:94`, `docs/engine/game-loop.md:142`
- **Status**: NEW
- **Age**: `87639d703` (2026-03-29); never wired since
- **Effort**: small (decide: wire `StartTimer`/`CancelTimer` lowering + an `OnTimer` dispatcher, or delete and fix the docs)
- **Description**: `timer.rs` declares "This replaces Papyrus's `StartTimer(time, id)` -> `OnTimer(id)` flow". The tick system is scheduled in `Stage::Early`, but the flow has no producer and no consumer.
- **Evidence**:
  - `grep -rnw ScriptTimer crates byroredux --include='*.rs'` outside `timer.rs`: only `lib.rs` re-export / `world.register`, doc comments, and `cleanup.rs`/save-registry names. The sole `ScriptTimer { .. }` constructions are inside `timer.rs`'s own `#[cfg(test)]`.
  - `grep -rnw "StartTimer\|CancelTimer" crates byroredux --include='*.rs'` (non-test): 1 hit, the `timer.rs:7` doc comment. No recognizer lowers `StartTimer`.
  - `grep -rnw TimerExpired crates byroredux ... | grep -v "events.rs\|cleanup.rs\|lib.rs\|timer.rs"`: only doc comments, the schedule `.writes::<TimerExpired>()` declaration, and a save-registry exemption string. No `query::<TimerExpired>()` reader; `CanonicalEvent::Timer` is never matched (`grep "CanonicalEvent::Timer"` = 0).
  - By contrast every sibling marker has a prod emitter and reader (ActivateEvent 31 readers, OnUpdateEvent 24, ...).
- **Impact**: An `OnTimer` handler in any Papyrus script is silently dead; `m47-0-design.md:99` marks `TimerExpired` as "emit sites: check timer_tick_system" which reads as shipped. The system pays a per-frame query for nothing. Same family as the closed `ItemEventBatch` (#4713) and `SetInChargen` (#4372) - but those got honest "No consumer yet" docs; this one did not.
- **Related**: #4713 (ItemEventBatch doc fix), #4372, `RecurringUpdate`/`OnUpdateEvent` (the working sibling, `recurring_update.rs`)
- **Suggested Fix**: Either (a) add a "No producer, no consumer yet" paragraph to `timer.rs` + `events.rs::TimerExpired` and correct the three doc rows, or (b) lower `StartTimer`/`CancelTimer` in `translate/effects.rs` into a `ScriptTimer` insert and dispatch `CanonicalEvent::Timer` in `papyrus_provider/execute.rs` the way `OnInitEvent` is.

### TD6-2026-10-11-02: The Redux-native plugin pipeline (`DataStore` + `DependencyResolver` + `PluginManifest`) has no production consumer, while four docs and a save-registry row present it as the live load path
- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/plugin/src/datastore.rs` (155 prod LOC), `resolver.rs` (238), `manifest.rs` (74), `record.rs::{Record::add_component, ErasedComponentData}`; claims in `crates/plugin/src/lib.rs:1-10`, `docs/engine/plugin-loading.md:10-30,131-160`, `docs/engine/architecture.md:283`, `CLAUDE.md` workspace tree, `byroredux/src/save_io/registry_completeness_tests.rs:520`, `crates/core/src/ecs/components/inventory.rs:4`
- **Status**: NEW (adjacent: #4384, which deleted only the `legacy/` half and left this half; AUDIT_TECH_DEBT_2026-09-14 TD8-005 noted "neither does the `DataStore::add_plugin` layer they would feed")
- **Age**: `e472e321e` (2026-03-28), untouched in intent for 6.5 months
- **Effort**: medium (delete + doc rewrite) or large (wire an esm -> `Record` bridge)
- **Description**: Every use of `DataStore`, `DependencyResolver`, `ConflictResolution`, `ResolvedRecord`, `PluginManifest::from_toml`, `ErasedComponentData` is inside `crates/plugin/src/{datastore,resolver,manifest,record}.rs` (own tests included) or a doc comment. `grep -rnw "DataStore\|DependencyResolver\|PluginManifest" crates byroredux tools --include='*.rs'` outside those files: `lib.rs` re-export, `extension.rs` (a doc sentence saying it is *separate* from `PluginManifest`), and `registry_completeness_tests.rs:520`.
- **Evidence**:
  - `plugin-loading.md` diagram: `plugin.toml / FalloutNV.esm -> PluginManifest / ESM parser -> DataStore.add_plugin() -> DependencyResolver -> DataStore.resolve() -> EsmCellIndex / FormIdPool`. The real path is `parse_esm_with_load_order` + `FormIdRemap`/`GlobalSlot` (`cell_loader/load_order.rs`); `DataStore` never reaches `EsmCellIndex`. (The doc even has a "Legacy Bridge (removed)" section explaining the *other* dead half.)
  - `registry_completeness_tests.rs:520` justifies a save exemption for `DataStore` as an "immutable resolved plugin-record database rebuilt from manifests/plugins at boot". Nothing builds one at boot; `grep "insert_resource(DataStore"` = 0.
  - `inventory.rs:4` cites `DataStore` as the base-item source of truth; the real source is `EsmIndex`.
  - `RecordType` (same crate) *is* used by the binary (`StaticObject.record_type`); that is why `record.rs` is not wholly dead.
- **Impact**: Misleading architecture picture for contributors and for the save audit; ~470 prod LOC + ~360 test LOC maintained for nothing; the "UUID-keyed, no load-order slots" selling point of the Redux plugin format has no code path, yet ROADMAP does not list it as owed.
- **Related**: #4384, TD8-005 (2026-09-14), `docs/engine/modern_plugin_system` memory note (design intent: Redux-native format is a stated goal)
- **Suggested Fix**: Decide one way. Keep: add a ROADMAP "Owed" row ("Redux-native manifest/DataStore loader has no caller") and put a status banner at the top of `plugin-loading.md` and `lib.rs`; fix the save-registry row's reason string to "defined, never inserted". Drop: delete `datastore.rs`/`resolver.rs`/`manifest.rs` + `Record::add_component`/`ErasedComponentData` and rewrite the diagram.

### TD6-2026-10-11-03: `PerkList` is an orphan component (zero production readers or writers) whose module doc and the `condition.rs` function table still say `HasPerk` reads it
- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/core/src/ecs/components/perk_list.rs:1-5` (module doc), `crates/scripting/src/condition.rs:44` (table row `449 | HasPerk | PerkList`), `crates/core/src/ecs/components/mod.rs:95` (re-export)
- **Status**: NEW (the code half was settled by #2940/#3158 - `HasPerk` moved to `Perks` - but the component and two doc sites were left behind)
- **Age**: `97fbc66f3` (2026-06-23); superseded by `Perks` read at `condition.rs:972-981`
- **Effort**: trivial
- **Description**: `perk_list.rs` says it "is the ECS surface the perk system ... writes to and the M47.1 `HasPerk` condition function reads from (#1667)". The live evaluator (`condition.rs:972`) reads `byroredux_core::character::Perks` and its own comment says "the legacy `PerkList` projection is not populated by production code". The save-registry test (`registry_completeness_tests.rs:561`) also records "zero production write sites ... only #[cfg(test)]".
- **Evidence**: `grep -rnw PerkList crates byroredux tools --include='*.rs'` -> definition, re-export, 2 doc mentions in `character/components.rs`, its own tests, and `condition.rs:44/977` doc/comment. No `world.insert(_, PerkList..)`, no `query::<PerkList>`; my constructor-token scan flags it as the only component with zero references.
- **Impact**: A contributor wiring perk grants will "attach `PerkList`" per its doc and silently get a no-op, since `HasPerk` reads `Perks`.
- **Related**: #2940, #3158, #1667 (all closed)
- **Suggested Fix**: Delete `perk_list.rs` + re-export + the two `character/components.rs` doc mentions and change `condition.rs:44` to `Perks`; or, if kept as a projection, demote its module doc to "legacy, unpopulated".

### TD6-2026-10-11-04: The Starfield WTHS value-reader "tracked follow-up (see the issue)" points at the closed decode issue; `exal.md` still says WTHS "stays undecoded"
- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/plugin/src/esm/records/weather_settings.rs:50-58,138-143`; `docs/engine/exal.md:153`; consumer-less map `EsmIndex.weather_settings` (`index.rs:225`, dispatch `dispatch_misc_gameplay_a.rs:61`)
- **Status**: NEW
- **Age**: `a5484faf6` (2026-10-09), `f0f8dceef` (2026-10-11)
- **Effort**: trivial (open one tracker + 1-line doc edit)
- **Description**: Both doc comments defer the OBJT/LIST/USER instance-value decode to "the tracked follow-up (see the issue)", but the only referenced issue, #5364, is CLOSED ("Decode Starfield WTHS ... the real SF weather schema") and `gh issue list --search WTHS` returns no open issue. The schema half landed; the value half (fog distances, colours, wind) is the part a renderer would need, and `WeatherSettingsRecord` has zero readers (`grep -rn "weather_settings\|WeatherSettingsRecord"` outside the record module = index/dispatch only). `exal.md:153` ("while Starfield's WTHS stays undecoded") is now half-true and will mislead the next EXAL audit.
- **Evidence**: commands above; `weather_root_class` (new pub fn in the same file) also has no prod caller.
- **Impact**: An untracked placeholder in a 2-day-old record decoder; Starfield exteriors keep using the `DefaultWeather` WTHR stand-in (#5363/#5424) with nothing recording what removes it.
- **Related**: #5364, #5514, #5363, #5424
- **Suggested Fix**: File "Starfield WTHS instance-value reader (OBJT/LIST/USER) -> weather params", cite its number in both doc comments, and reword `exal.md:153` to "schema decoded (#5364), values not".

### TD6-2026-10-11-05: Entity-based one-shot audio (`OneShotSound` + `spawn_oneshot_at`) has no producer in the engine; module docs still pitch it as the gameplay entry point
- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/audio/src/lib.rs:36-60` (module doc "Phase 3 (this commit)"), `:1010-1016` (`OneShotSound`), `:1331-1490` (dispatch + marker-clear arms), `:1586-1620` (`spawn_oneshot_at`)
- **Status**: NEW as a tracked item (noted and waved off as "latent" in AUDIT_AUDIO_2026-08-16 and AUDIT_AUDIO_2026-07-16)
- **Age**: `45a986485` (2026-05-05)
- **Effort**: small (document) / medium (delete the entity path and its ~10 tests)
- **Description**: The only non-test insert of `OneShotSound` is inside `spawn_oneshot_at` (`lib.rs:1614`), and `spawn_oneshot_at` itself has zero callers outside `crates/audio/src/tests.rs`. Every shipped sound (footsteps, impacts, splash, voice) goes through `AudioWorld::play_oneshot`. The module header still says the "intended consumer is gameplay code (footstep timer, weapon-fire trigger, dialogue dispatcher)" and carries three "(this commit)" phase headers.
- **Evidence**: `grep -rnw OneShotSound crates byroredux --include='*.rs' | grep -v tests` -> only `crates/audio/src/lib.rs` (def, dispatcher, spawn helper). `grep -rnw spawn_oneshot_at ... | grep -v tests` -> definition only. Also `load_streaming_sound_from_file` (lib.rs:1671, "Phase 5 ... loose Data/Music files") has zero uses anywhere, test or not.
- **Impact**: Two dispatch paths to keep correct (the marker-clear failure arms exist because of #2394) where one is exercised only by tests; a contributor following the docs spawns entities the engine never needed.
- **Related**: #2394 (closed), AUDIT_AUDIO_2026-08-16 line ~201
- **Suggested Fix**: Mark the entity path "test/tooling only; engine uses `play_oneshot`" in the module doc and drop the "(this commit)" headers, or delete it; hand `load_streaming_sound_from_file` to Dim 8.

### TD6-2026-10-11-06: `save::disk::latest_slot` is claimed to be "used by the player-facing quickload action" but is a test-only wrapper since `slots_by_recency` replaced it
- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/save/src/disk.rs:80-84`
- **Status**: NEW
- **Age**: delegating wrapper since the 2026-08-24 save audit
- **Effort**: trivial
- **Description**: The doc comment says it is "used by the player-facing quickload action". The prod quickload path calls `disk::slots_by_recency` (`byroredux/src/save_io.rs:1249`); `latest_slot` is referenced only by 4 asserts in `disk.rs`'s own tests (`:309-376`).
- **Evidence**: `grep -rn latest_slot crates byroredux --include='*.rs'` -> def + tests only.
- **Impact**: Trivial; a false "used by" claim in a freshly-audited crate.
- **Suggested Fix**: Delete it and retarget its 4 asserts at `slots_by_recency(..).first()`, or reword the doc to "test convenience".

### TD7-2026-10-11-03: MGEF DATA decode uses bare byte offsets, flag masks, actor-value ids and an archetype id in two functions that already disagree on whether to name the archetype

- **Severity**: LOW
- **Dimension**: 7
- **Location**: `crates/plugin/src/esm/records/misc/magic.rs:779-935` (`decode_mgef_archetype`, `parse_mgef_for_game`)
- **Status**: NEW
- **Age**: 4c0053322, 2026-09-16 (consumable effects parsing). Only `decode_mgef_archetype` predates it (#4415).
- **Effort**: small
- **Description**:
  - Three copies of the `read = |offset| u32::from_le_bytes(data[offset..offset+4]…)` closure read offsets `0, 20, 56, 60, 64, 68, 80, 84, 88, 128, 132, 136` with no field names.
  - Hand-typed masks: `1|2|4|0x100|0x80000|0x100000` (FO3/NV), `1|2|4|0x400` and `0x200|0x20000|0x100|0x10000000` (Skyrim).
  - Gate sizes: `data.len() == 72` (FO3/NV) and `>= 152` (Skyrim+).
  - Actor-value ids: `matches!(av, 12 | 16)`, `(24..=26).contains(&av)`, `read(64) == 34 && av == 16`.
  - The archetype is *named* in `decode_mgef_archetype` (`34 => MagicArchetype::PeakValueModifier`, `MagicArchetype` enum) but raw `34` in `parse_mgef_for_game:864`.
- **Evidence**:
  - `magic.rs:857` `if data.len() == 72`, then `read(64) == 34` at `:865,869`.
  - `magic.rs:893-908` Skyrim branch: `read(80) == 1`, `read(84) == 0`, `read(128) == 0 && read(132) == 0 && read(136) == 0`.
  - `decode_mgef_archetype` documents the layout only in its doc comment (`@64`, `@68`, `@60`, `@88`).
  - Neither function names 56, 80, 84, 128-136.
- **Impact**: The flag bits and AV ids define what counts as a "plain restoration" consumable. A wrong bit reclassifies a poison, ingestible or disease effect as a heal, or the reverse. Nothing in the code says which xEdit field each offset is, so a reviewer has to re-derive the layout from xEdit.
- **Related**: #4415, #2414 (shared walker).
- **Suggested Fix**:
  - In `magic.rs`, add `mod mgef_data { pub const ARCHETYPE: usize = 64; PRIMARY_AV: usize = 68; … }`.
  - Add a `MgefFlags` bitflags-style struct, one per game, with `NO_DURATION`, `HOSTILE`, `RECOVER`, `DETRIMENTAL`, …
  - Reuse `MagicArchetype::from_u32` and compare variants instead of `34`.
  - Add `FO3NV_DATA_LEN` and `TES5_DATA_LEN`.
  - Do this as part of Dim 2's three-copy `read` closure cleanup.

### TD7-2026-10-11-04: `weather_settings.rs` re-types the CDB builtin type ids and the IS_USER class flag that `byroredux-sfmaterial` already names

- **Severity**: LOW
- **Dimension**: 7 (overlaps Dim 2; whoever takes the fix should dedupe)
- **Location**:
  - `crates/plugin/src/esm/records/weather_settings.rs:68-82` (`builtin_type_label`)
  - `crates/plugin/src/esm/records/weather_settings.rs:316-376` (`skip_diff_value`)
  - `crates/plugin/src/esm/records/weather_settings.rs:333` (`flags & 0b100`)
  - canonical: `crates/sfmaterial/src/types.rs:14-60`, `ClassFlags::IS_USER`
- **Status**: NEW
- **Age**: a5484faf6 (#5364, WTHS) and f0f8dceef (#5514), 2026-10-09
- **Effort**: small
- **Description**: `weather_settings.rs` carries the 16 builtin type ids twice as bare negative i32 literals: `-255` Null, `-254` String, `-253` List, `-252` Map, `-251` Ref, `-248..-238` the integer, bool, float and double types. One copy is a label table and the other is the `skip_diff_value` switch. It also hard-codes the `0xFFFF` diff terminator twice (`:302,335`) and the user-class bit `0b100`. `byroredux-sfmaterial` defines `BuiltinType` (`0xFFFFFF01 … 0xFFFFFF12`, i.e. the same values as i32) and `ClassFlags::IS_USER = 1 << 2`. The doc comment on `skip_diff_value` even cites sfmaterial's `skip_value`, so the author knew.
- **Evidence**:
  - `0xFFFFFF01 as i32 == -255`, and `1 << 2 == 0b100`, so the values are identical.
  - `crates/plugin/Cargo.toml` does not depend on `byroredux-sfmaterial`. `sfmaterial` has only `thiserror` as a runtime dependency, so the dependency would be acyclic.
  - Three copies of the table now exist: the sfmaterial enum, the label fn and the skip fn.
- **Impact**: The id table only grows by editing three places. The `_ => false` arm in `skip_diff_value` ("unknown built-in — refuse to guess") is the only defence against a missed id. Parse truncation is silent: `collect_diff_indices` returns `None` and the DIFF decodes as unset.
- **Related**: #5364, #5514 (closed).
- **Suggested Fix**:
  - Either depend on `byroredux-sfmaterial` and use `BuiltinType::from_u32(raw as u32)` plus `ClassFlags::is_user`, or move `BuiltinType` / `ClassFlags` into a tiny shared crate or into `byroredux-core`.
  - At minimum, add `const` names (`BUILTIN_NULL: i32 = -255` …) in `weather_settings.rs`, and make `builtin_type_label` and `skip_diff_value` share one `match`.

### TD7-2026-10-11-05: Consumable CTDA gating duplicates the "known CTDA lengths", the run-on offset and the `RunOn` enum range from `condition.rs`

- **Severity**: LOW
- **Dimension**: 7
- **Location**:
  - `crates/plugin/src/esm/records/items/consumable.rs:75-82`
  - `crates/plugin/src/esm/records/condition.rs:354` (the same length set)
  - `crates/plugin/src/esm/records/condition.rs:386-392`
- **Status**: NEW
- **Age**: 4c0053322, 2026-09-16
- **Effort**: trivial
- **Description**:
  - `consumable.rs:75` has `!matches!(sub.data.len(), 20 | 24 | 28 | 32)`, the same literal set as `condition.rs:354`.
  - `consumable.rs:79-82` reads the run-on field with `u32::from_le_bytes(sub.data[20..24]…) > 7`. The offset 20 and the maximum 7 (`RunOn::EventData`) are both facts owned by `condition.rs` (`RunOn::from_u32`, `data[20..24]` at `:386-392`).
  - Whether a new `RunOn` variant 8 gets accepted is decided in a different file.
- **Evidence**: See the line numbers above. `RunOn::from_u32` maps unknown values to `Subject`, which is the fallback the consumable gate is trying to avoid, and the consumable gate hard-codes the boundary. There is no `CTDA_KNOWN_LENGTHS` or `RunOn::try_from_u32`.
- **Impact**: Low. This is a lockstep gap with an easy failure mode: add a `RunOn` variant (or a 36-byte CTDA layout) in `condition.rs` and consumable conditions with that value silently stay rejected, so the consumable effect drops with no log line.
- **Suggested Fix**:
  - Add `pub const CTDA_KNOWN_LENGTHS: [usize; 4] = [20, 24, 28, 32]` and `pub fn RunOn::try_from_u32(u32) -> Option<Self>` to `condition.rs`.
  - Add a `ctda_run_on_raw(data) -> Option<u32>` accessor next to the parser.
  - Use them in `consumable.rs` and at `condition.rs:354`.

### TD7-2026-10-11-06: ReSTIR shadow-ratio history cap/floor are bare shader literals, repeated and pinned only by source text

- **Severity**: LOW
- **Dimension**: 7
- **Location**:
  - `crates/renderer/shaders/triangle.frag:3596,3929-3931`
  - pin: `crates/renderer/src/vulkan/restir.rs:~322-331`
  - the mode encoding at `triangle.frag:3929,4185`
- **Status**: NEW
- **Age**: 2a7223121 (2026-10-09, 64 → 256) and 48bed0504 (2026-10-11, 256 → 1024 and 0.008 → 0.002)
- **Effort**: small
- **Description**:
  - The parked-tail cap `1024.0` is typed twice (`clamp(rpHistLen, 0.0, 1024.0)` at `:3596` and `sceneStatic ? 1024.0 : 16.0` at `:3930`).
  - The floor `0.002` / `0.1` and the moving cap `16.0` are typed once each.
  - The history *mode* is a float in `dofParams.w`, decoded by threshold literals `> 0.5` and `> 1.5` in the shader. Meanwhile `restir_history_mode()` (`assemble_camera_and_lights.rs:95`) encodes 0..3 on the Rust side.
  - A comment ties `1024` to packHalf2x16's integer-exact range (2048), but that ceiling is prose only.
  - None of these sit in `shader_constants_data.rs`.
- **Evidence**:
  - The in-shader comment records that a hard-coded `64` guard once "silently defeated `historyCap`'s raise and pinned the parked tail at ~1/65", so this duplication has already produced a wrong result once. The 2a7223121 → 48bed0504 sequence retyped all three literals again.
  - `restir.rs:322-330` pins them with `src.contains("sceneStatic ? 1024.0 : 16.0") && src.contains("clamp(rpHistLen, 0.0, 1024.0)")`. That is a source-shape test of the same literal text, not a value check.
  - Nothing asserts `cap <= 2048` or `clamp bound >= cap`.
- **Impact**: The failure mode is subtle shadow-noise or ghosting regressions. A future retune above 2048 would lose integer exactness in `histLen`, and the history would stall. No crash.
- **Related**: #5369 (open, partial).
- **Suggested Fix**:
  - Add `RESTIR_SHADOW_HISTORY_CAP_PARKED: f32 = 1024.0`, `_MOVING: f32 = 16.0`, `ALPHA_FLOOR_PARKED = 0.002`, `ALPHA_FLOOR_MOVING = 0.1`, and `HISTORY_MODE_SCENE_STATIC_MIN = 2` to `shader_constants_data.rs`.
  - Use them in `triangle.frag` and in `restir_history_mode()`.
  - Add a `const _` assertion that the cap is `<= 2048` (half-exact).
  - Replace the `contains(...)` pin with a header-presence check.

### TD7-2026-10-11-07: `heave_hold_damping` — the stabilising coefficient is a measured constant for one density ratio and gravity, and its `1e-3` floor does not prevent the instability it floors against

- **Severity**: LOW (analytic, not reproduced; reachability of the failing range unproven)
- **Dimension**: 7
- **Location**: `crates/physics/src/water.rs:196-202,1123`; `crates/physics/src/convert.rs:28-30` (`clamp_shape_extent`)
- **Status**: NEW
- **Age**: c5bf46e59, 2026-10-11 (#5547)
- **Effort**: small
- **Description**:
  - `HEAVE_HOLD_COEFFICIENT = 11.0` is labelled "measured, not derived" and applied as `(11.0 / height.max(1.0e-3) - base)`.
  - The doc derives the needed damping as proportional to `ω² = ρ·|g|/h`, "here ≈ 1146/h". 1146 is `1.67 × 686.7`, i.e. the default `buoyancy_density_ratio` times gravity in BU/s².
  - `density_ratio` is a runtime resource field (`PhysicsWaterConstants::buoyancy_density_ratio`, with no documented range) and `gravity_y` is world state. Both are in scope at the call site (`water.rs:1123-1126`) and carried on `WaterLift` (`:181-182`), but `heave_hold_damping(height, base)` takes neither.
  - `thin_floats_settle_and_sleep_at_any_frame_rate` only runs `PhysicsWaterConstants::default()`.
- **Evidence**: Heave damping is applied as an explicit force, `now.y -= heave_damping * mass * vy`, refreshed every `PHYSICS_DT = 1/60`. That is stable only while `c·dt < 2`. With `c = 11/h − 1.5` this needs `h > 11 / (120 + 1.5) ≈ 0.0905` BU. The `.max(1.0e-3)` floor lets `c` reach `11000`, so `|1 − c·dt| ≈ 182` per tick. A cuboid at `clamp_shape_extent`'s floor (half-extent `1e-3`, so `h = 0.002`) gives `c·dt ≈ 91.6` and a multiplier of about 90 per tick. The lift spring itself (`ω² = 1146/h`, so `ω·dt > 2` below `h ≈ 0.08`) crosses the same stability edge. I did not run this; trimesh and convex-hull shapes are not extent-clamped either.
- **Impact**: Two parts.
  - A retuned `buoyancy_density_ratio`, for example the 2.0 used by a physics fixture, leaves the coefficient mis-sized, and the #5547 limit cycle on thin floats can come back without any test noticing.
  - Bodies thinner than about 0.09 BU (1.3 mm) would diverge in water instead of being damped. Whether such colliders exist in shipped content is unproven.
- **Related**: #5547, #5561.
- **Suggested Fix**:
  - Make the coefficient a function: `heave_hold_damping(height, base, density_ratio, gravity_y.abs())`, with `K = 11.0 / (1.67 * 686.7)` as the measured constant and `K * density_ratio * |g| / height`.
  - Clamp the result to `c <= 1.0 / PHYSICS_DT` (for example `min(c, 0.9 / dt)`) instead of `height.max(1e-3)`, so it cannot overshoot.
  - Add one `thin_float_settle` run at `buoyancy_density_ratio = 2.5`.

### TD7-2026-10-11-08: `post_passes.rs` re-derives the cluster grid and index buffer sizes with a bare `8` although `ClusterCullPipeline` exposes both

- **Severity**: LOW
- **Dimension**: 7
- **Location**: `crates/renderer/src/vulkan/context/post_passes.rs:696-709` (`CLUSTER_ENTRY_SIZE = 8`)
- **Status**: NEW
- **Age**: 94ded998b3, 2026-07-28
- **Effort**: trivial
- **Description**: The block computes `grid_size` / `index_size` for the volumetrics lighting descriptors with a function-local `const CLUSTER_ENTRY_SIZE: vk::DeviceSize = 8`. The comment says it "mirrors the formulas in `ClusterCullPipeline::new`". `ClusterCullPipeline` already exposes `grid_buffer_size()` (`compute.rs:423`, `size_of::<ClusterEntry>() * TOTAL_CLUSTERS`) and `index_buffer_size()` (`:428`), and `context/init.rs:584-586` calls them. The same function already holds the pipeline (`self.cluster_cull`, `:652`).
- **Evidence**: `size_of::<ClusterEntry>()` is the struct size; the bare `8` is its hand-computed value (`{offset:u32, count:u32}`).
- **Impact**: If `ClusterEntry` grows (the likely edit is a third field in `bindings.glsl:340`), the volumetrics descriptor `range` is under-sized. Robust-buffer reads return zeros rather than faulting, so the symptom is wrong froxel lighting, not an error. Debug validation does not catch it either.
- **Suggested Fix**: Use `cc.grid_buffer_size()` and `cc.index_buffer_size()` at `post_passes.rs:703-709` (inside the `vol_lights` closure that already holds `cc`), and delete the local const.

### TD7-2026-10-11-09: Four compute shaders hard-code workgroup size, reduction strides and Rust dispatch counts that the generated header already provides for the other 17

- **Severity**: LOW
- **Dimension**: 7
- **Location**:
  - `shaders/exposure_meter.comp:42,55-56,70,74,95` (`local_size_x = 64`, `shared[64]`, `side = 64`, `idx += 64`, reduction start `s = 32`)
  - `shaders/sky_irradiance.comp:5,8,10,21,23,36` (`local_size_x = 64`, `partialSum[64]`, `+= 64u`, `stride = 32u`, `coefficients[9]`)
  - `shaders/sky_prefilter.comp:7`
  - `shaders/groundcover_bench_bake.comp:23`
  - Rust: `vulkan/sky_cube/irradiance.rs:6,153` (`SH_BYTES = 9 * 16`, `cmd_dispatch(cmd, 9, 1, 1)`), `vulkan/sky_cube/filter.rs:208` (`edge.div_ceil(8)`), `vulkan/groundcover_bench.rs:1359-1360` (`LAND_GRID_VERTS.div_ceil(8)`)
- **Status**: NEW
- **Age**: older than the window (not changed since 2026-10-08)
- **Effort**: small
- **Description**: 17 compute shaders take `local_size_*` from generated `WORKGROUP_X/Y`, `SKIN_WORKGROUP_SIZE`, `GROUNDCOVER_*_WORKGROUP` and so on. These four use bare literals. In `exposure_meter.comp` and `sky_irradiance.comp` the workgroup width is coupled to a hand-written tree reduction (`s = 32`) and to `shared[]` array sizes. The Rust side re-types the group counts and the SH coefficient count (9). The `shader_constants.rs` provenance gate looks at SCREAMING_CASE names, so lower-case locals (`side`, `edge`, `count`) and layout qualifiers are invisible to it.
- **Evidence**:
  - `grep -rn local_size shaders/*.comp` shows the four literals.
  - Changing `exposure_meter.comp:42` to 128 without touching `shared[64]` or the reduction start would write out of bounds of the shared arrays.
  - In `irradiance.rs`, `SH_BYTES` and the dispatch group count are typed as separate 9s.
- **Impact**: Lockstep cost only. Nothing is wrong today and each file is self-consistent. The exposure meter drives the auto-exposure of the whole image, so a mismatched edit there would be visible immediately.
- **Suggested Fix**:
  - Add `EXPOSURE_METER_WORKGROUP = 64` (so `shared[EXPOSURE_METER_WORKGROUP]`, and `s = WORKGROUP/2`) and `SKY_SH_COEFFICIENTS = 9` (with `SH_BYTES = SKY_SH_COEFFICIENTS * 16`, `cmd_dispatch(cmd, SKY_SH_COEFFICIENTS, 1, 1)`) to `shader_constants_data.rs`.
  - For `sky_prefilter.comp` and `groundcover_bench_bake.comp`, use `WORKGROUP_X/Y` (value 8, the same today) and make `filter.rs:208` / `groundcover_bench.rs:1359` use `WORKGROUP_X` instead of a literal 8.

### TD8-2026-10-11-01: CI clippy gate red at HEAD — 3 lints, renderer + physics lib, leaves the `byroredux` bin unlinted; `--all-targets` adds 5 more test-target lints
- **Severity**: LOW (no tech-debt promotion trigger applies; note it is a red CI gate — treat as top quick win)
- **Dimension**: 8 (clippy gate)
- **Location**:
  - Gate: `crates/renderer/src/texture_registry/dynamic_rgba.rs:38,39` (`doc_lazy_continuation` x2; `e588e5e67`, Fix #5528, 2026-10-11)
  - Gate: `crates/physics/src/world/mod.rs:889-891` (`option_map_unit_fn`; `2917ab372`, Fix #5539, 2026-10-11)
  - All-targets only: `crates/physics/src/water.rs:2225-2226` (`field_reassign_with_default`; `cff1f9c42`, #5354) and `:2485-2486` (same; `0d28abfe3`, #5561)
  - All-targets only: `crates/nif/src/blocks/collision/bhk_constraint_tests.rs:560` (`needless_range_loop`; `526cb07d9`, #5537)
  - All-targets only: `crates/plugin/src/esm/records/misc/dialogue.rs:1862` (`needless_borrows_for_generic_args`; `29f8fa2ef`, #5454)
- **Status**: NEW (prior 10-08 TD8-01 / #5474 and TOOL-CI #5477 are CLOSED and their lints are gone; these are fresh)
- **Effort**: trivial
- **Description**: Today's fix commits landed lints the gate rejects. The gate run exits with `could not compile byroredux-renderer (lib) due to 2 previous errors` and `byroredux-physics (lib) due to 1 previous error`; `--keep-going` therefore skips `byroredux` (bin), which depends on both. `gh run list` shows the latest CI runs are in progress/failed (`4b94729` failure).
- **Evidence**:
  - `CARGO_TARGET_DIR=/mnt/data/tmp-audit-target/dim8 cargo clippy --workspace --keep-going -- -D warnings` on 1.96.0: errors only in the two files above.
  - `... --all-targets --keep-going`: `byroredux-physics (lib test)` 3 errors, `byroredux-renderer (lib test)` 2, `byroredux-nif (lib test)` 1, `byroredux-plugin (lib test)` 1.
  - Crates left unlinted in both runs: `byroredux` (bin + its tests) and any renderer/physics dependents' test/example targets.
- **Impact**: Red gate masks any further lint in the bin crate (the largest, most-churned crate). Same recurring class as #5477/#5474/#4765/#4090 — the bucket re-accumulates every active day.
- **Related**: #5474 (CLOSED), #5477 (CLOSED), #5115 (CLOSED), #5308 (toolchain pin)
- **Suggested Fix**: `if let Some(b) = self.bodies.get_mut(handle) { b.add_torque(torque, false); }` (physics/world/mod.rs:889); indent the two doc continuation lines or insert a blank line (dynamic_rgba.rs:38-39); `let mut x = T { field, ..Default::default() }` in water.rs tests; iterate with `.iter().enumerate()` in bhk_constraint_tests.rs:560; drop the `&` in dialogue.rs:1862. Consider the non-blocking `--all-targets` CI step suggested last time (still not present).

### TD8-2026-10-11-02: 28 `#[allow(clippy::...)]` sites carry no reason comment (27 `too_many_arguments`, 1 `large_enum_variant`)
- **Severity**: LOW
- **Dimension**: 8
- **Location** (attribute line; none has a comment above/beside it):
  - `crates/core/src/settings/mod.rs:91`; `crates/spt/examples/spt_tail.rs:420`; `crates/plugin/src/esm/cell/walkers.rs:562`
  - renderer: `groundcover/construct.rs:601`, `bloom.rs:1025`, `caustic.rs:326,366,669`, `groundcover_bench.rs:787`, `composite.rs:370`, `frame_upscaler.rs:196,1060`
  - sfmaterial examples: `cdb_key_hash_probe.rs:118` (`type_complexity`), `cdb_join_probe.rs:645,712`
  - byroredux: `npc_spawn/resumable/runtime.rs:166,479,964,1122`, `prebaked.rs:73,166`, `cell_loader/spawn.rs:262,580`, `exterior.rs:2024`, `references/mod.rs:239`, `precombined.rs:244`, `terrain_lod.rs:360`, `load.rs:831` (`large_enum_variant`)
- **Status**: NEW (the house style was last policed by #5115, CLOSED, for 2 sites; this is the residue and the sites that came after)
- **Effort**: small
- **Evidence**: script over 129 `allow(clippy::...)` attributes, each tested for a `//` comment on the preceding contiguous comment/attribute lines or within 3 lines. 31 flagged; 3 `manual_clamp` sites in `byroredux/src/systems/weather.rs:412-416` are covered by the block comment above (false positives). `git blame` shows all 28 predate 2026-10-08 (newest: `37db35cca` 2026-10-01 resumable runtime/prebaked, 6 sites). 94 `too_many_arguments` allows in total, 64 with a reason.
- **Impact**: A bare `too_many_arguments` allow hides whether the arg list is a deliberate FFI/Vulkan-shaped signature or a missed parameter-struct.
- **Related**: #5115 (CLOSED)
- **Suggested Fix**: Add a one-line reason to each (or fold args into a params struct, e.g. the 6 `npc_spawn/resumable` sites). Optionally add a source-scan test in `workspace_hygiene_tests.rs` (uses `core::source_scan::production_text`) that fails on a bare `allow(clippy::too_many_arguments)`.

### TD8-2026-10-11-03: `PerkRecord.perk_flags` is write-only "for backwards-compat with the prior stub" — no consumer outside one test
- **Severity**: LOW
- **Dimension**: 8
- **Location**: `crates/plugin/src/esm/records/misc/magic.rs:279-281` (decl), `:343-344` (only write; `is_trait` derived from it), `:1040` (only read: test assertion)
- **Status**: NEW
- **Effort**: trivial
- **Description**: Field comment says "Kept for backwards-compat with the prior stub; new code should prefer `is_trait`". `grep -rn perk_flags crates byroredux tools` -> 4 hits, all in magic.rs; no external consumer exists (no external users of a workspace-internal struct).
- **Impact**: Duplicate representation of the same DATA byte; the `u8` implies richer flag semantics than the parser provides.
- **Suggested Fix**: Delete `perk_flags`, parse into a local and set `is_trait`, update the test at magic.rs:1040 to assert `is_trait`.

### TD9-2026-10-11-01: `byroredux-core` with `parallel-scheduler` OFF is compiled by no CI lane
- **Severity**: LOW
- **Dimension**: 9 (feature gate vs CI lane)
- **Location**: `crates/core/Cargo.toml` (`default = ["parallel-scheduler"]`), `crates/core/src/ecs/scheduler.rs:20,499-510` (the `#[cfg(not(feature = "parallel-scheduler"))]` serial dispatch arm); `.github/workflows/ci.yml` (no `--no-default-features` on core)
- **Status**: NEW
- **Age**: feature introduced with M27; lane list last re-audited 10-08
- **Effort**: trivial
- **Description**: Every workspace consumer takes `byroredux-core` with default features (`Cargo.toml:47` plain path dep; all crate manifests `workspace = true`), so feature unification always turns `parallel-scheduler` (rayon) on. The serial phase-1 fallback in `Scheduler::run` and the `rayon`-free build are therefore compiled and tested by nothing in `ci.yml`, `real-data-gates.yml`, `rt-correctness.yml` or `playable-smoke.yml`. `docs/engine/ecs.md:419` and `m27-phase1-survey.md:238` describe the flag as a retained "runtime-disable knob for diagnostics", i.e. a supported state. This is the exact class #3894/#4387/#4390 closed for `recon` / `debug-server` / `dhat-heap`, which each got a one-line lane.
- **Evidence**: `grep -rn 'parallel-scheduler' .github scripts` -> only a comment (ci.yml:214). `cargo check -p byroredux-core --no-default-features --lib --tests` (1.96.0) compiles clean today, so there is no live rot — the finding is the unguarded state, not breakage.
- **Impact**: A change that makes the scheduler's parallel arm and serial arm diverge (new field used only in the rayon arm, an `import` only valid with rayon) lands green and is found by whoever next builds without the feature.
- **Related**: #3894, #4387, #4390 (same class, closed); #5327 (open, warnings on the sibling `--no-default-features` byroredux lane).
- **Suggested Fix**: add to the `cargo-test` job beside ci.yml:176-180: `cargo check -p byroredux-core --no-default-features --lib --tests` (~5 s; verified).

### TD9-2026-10-11-02: Vulkan-gated `golden_frames` + `upscaler_quality` have no automated lane (the `rt-correctness` dispatch has no gate for them)
- **Severity**: LOW
- **Dimension**: 9 (test-gap)
- **Location**: `byroredux/tests/golden_frames.rs:107-117` (2 tests), `byroredux/tests/upscaler_quality.rs:115,189` (2 tests); `.github/workflows/rt-correctness.yml:5-14` (gate choice: `cornell | material-provider-matrix | all`)
- **Status**: NEW
- **Age**: `golden_frames` baselines last regenerated 2026-09-21 (`7cdc0c7ff`); 223 commits touching `crates/renderer/{src,shaders}` since
- **Effort**: small
- **Description**: The only pixel-level render regression guards are `#[ignore]` behind a Vulkan device and referenced by no workflow (`grep -rn 'golden' .github` -> none; only `scripts/renderer-eval.sh:106` mentions the contract and `audit-runtime` SKILL line 19 lists them as a manual gate). The default-lane guard `committed_baseline_matches_the_current_invocation` only proves the *manifest* matches the invocation (#3849/#4227), not that the PNG still matches the renderer. The self-hosted `byroredux-rt` runner and the `rt-correctness` workflow already exist and already run `cornell_rt_oracle`, so the lane is a one-option addition. The `golden_frames` module doc itself says the first version rotted for three months unnoticed behind this same blind spot.
- **Evidence**: `git log -1 --format=%ad -- byroredux/tests/golden/*.png` -> 2026-09-21; `git log --since=2026-09-21 --oneline -- crates/renderer/src crates/renderer/shaders | wc -l` -> 223. Pixel-currency could not be checked (no Vulkan device use permitted in this audit).
- **Impact**: The "Phase-2c-style shader regression" the file was written to catch is only caught when someone remembers to run `audit-runtime`.
- **Related**: #3849, #4227 (closed), #4492 (closed — same "harness in no automated lane" class for the exterior harnesses).
- **Suggested Fix**: add a `golden` option to `rt-correctness.yml` running `xvfb-run -a cargo test --release -p byroredux --test golden_frames -- --ignored --test-threads=1` (+ `upscaler_quality` without the game-data case), mirroring the `cornell` job (rt-correctness.yml:21-60).

### TD9-2026-10-11-03: No-panic-only / misnamed tests that assert nothing about the behaviour in their names
- **Severity**: LOW
- **Dimension**: 9 (test-gap)
- **Location**:
  - `crates/plugin/src/equip.rs:1171-1186` `empty_flags_never_cover_body` — name promises coverage semantics; body only asserts `main_body_bit(game).is_some()` for six games (the "bit-position equivalents" the comment mentions are asserted in the *neighbouring* test).
  - `crates/bsa/src/archive/tests.rs:151-154` `genhash_file_handles_no_extension` — `let _ = genhash_file(b"noextension");`, comment claims "Falls back to empty ext" but nothing is checked.
  - `crates/scripting/src/timer.rs:171-182` `no_timers_is_safe`, `cleanup_with_no_events_is_safe`; `crates/core/src/character/regen.rs:551` `tick_system_is_a_noop_without_config`; `byroredux/src/systems/combat_ai.rs:705` `no_combat_states_is_a_cheap_no_op`; `byroredux/src/cell_loader/rapier_release_tests.rs:131` `release_is_noop_when_physics_resource_absent` — "no-op" tests that never read the world afterwards to prove it is unchanged.
  - `tools/byro-launcher/src/engine.rs:149` `the_parent_directory_is_searched_second` — creates the exe in the parent dir but asserts only `.is_some()`, not that the returned path is the parent's.
- **Status**: NEW
- **Age**: mixed (the churned ones: `rapier_release_tests`, `timer.rs` are pre-10-08; `equip.rs` TPTA tests added since)
- **Effort**: small
- **Description**: These pass if the function returns anything / does not panic. Other "does-not-panic" tests in the tree are legitimate (lock-cycle tests rely on the lock tracker's panic; `flat_walker_caps_at_max_depth` documents "success is returned without overflowing the stack") — these six shapes are the ones where the test *name* claims an observable postcondition that is not checked.
- **Evidence**: bodies read directly (see Location); scripted scan of every `#[test]` fn in the workspace for no-assert and is_ok/is_some-only shapes.
- **Impact**: Low — they cannot pin a regression in the behaviour named. `the_parent_directory_is_searched_second` would still pass if `locate_engine` returned the wrong directory's file.
- **Related**: #4714 (closed, same file's test-only helpers), #4074 (closed, FO76/Starfield arm provenance).
- **Suggested Fix**: per test: assert `locate_engine(&bin) == Some(dir.path().join(ENGINE_EXE))`; for the no-op tests assert the component/resource counts before == after (one `assert_eq!` each); `genhash_file` assert the equality with `genhash_file(b"noextension.")`-style expectation or drop the test; rename `empty_flags_never_cover_body` to `every_supported_game_has_a_main_body_bit` (it is a duplicate of the last line of the previous test) or give it real flag-mask assertions.

---

## Watch list (not filed; file only if they cross)

- `crates/audio/src/lib.rs` — 1958 prod LOC (+274 since 10-08), the next to cross. Split axis `{components, world, systems, loaders, cache}`; four `include_str!("lib.rs")` scans in `audio/src/tests.rs` need a `concat!` SOURCES constant first.
- `crates/nif/src/import/types.rs` — 1997 (unchanged). Axis `types/{node,material,mesh,scene,ragdoll,particle}.rs`; repoint three `include_str!` scans.
- `crates/scripting/examples/mq101_conformance.rs` — 1927 (see TD1-2026-10-11-05).
- `physics::water::apply_buoyancy_with_scratch` — 514 → 622 LOC in 3 days, nesting depth 8; the one function actively regrowing. Raise it with the PHYS owners.
- Dim 9 observations: 11 `crates/ui` tests skip silently when no Vulkan adapter exists (they report green), and `heap_allocation_bounds_{geometry,import}.rs` use the default libtest harness while `heap_allocation_bounds` is `harness = false` (#5050).
