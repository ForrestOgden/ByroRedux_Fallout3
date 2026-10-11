# TD1-2026-10-11-01: `byroredux/src/env_translate.rs` crossed 2000 production LOC (1881 → 2186, +305 in six days)

URL: https://github.com/matiaszanolli/ByroRedux/issues/5584
Labels: low, terrain-exterior, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

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
