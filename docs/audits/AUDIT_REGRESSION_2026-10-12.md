# Regression Verification Audit — 2026-10-12

**HEAD**: 5d31fd637 · **Baseline**: `docs/audits/AUDIT_REGRESSION_2026-10-08.md` (HEAD `00f580e09`) · **Audited**: 95 previously-closed issues (26 closed since the baseline that no sibling report cites, 24 closed-since-baseline fixes whose production wiring was re-read, 45 older churn-ranked fixes no earlier sweep verified), the Step 4 unconditional fragile-area guards, whole-workspace test / lock-order / shader-parity / feature-variant gates, and independent confirmation of the nine closed issues the suite's other reports flagged as regressed or incomplete · **Unchanged since baseline (skimmed)**: n/a. This sweep is itself the delta pass (345 commits, 284 distinct `Fix #N` issues, 420 churned source files since `00f580e09`).

Run context: the `/audit-suite --preset comprehensive` run of 2026-10-12, solo and read-only. No engine launch, no smoke script, no `--ignored` corpus test. Scratch: `/tmp/audit/regression/` (`full_test.txt`, `lockorder_test.txt`, `feature_checks.txt`, `validate.txt`, `rows_{A,B,C}.md`, `guards_window.txt`, `erosion*.txt`).

## Summary

| Result | Count |
|---|---|
| Closed issues checked by this sweep | **95** |
| PASS (fix present, a default-lane guard exists and ran green) | **95** |
| PARTIAL / FAIL among them | **0** |
| Regressions and incomplete fixes raised by sibling reports, independently re-confirmed here | **9 issues / 10 findings confirmed, 0 refuted** (table below) |
| Step 4 fragile-area guards | **7/7 green** |
| `cargo test --workspace --no-fail-fast` (1.96.0) | **9,719 passed / 0 failed / 299 ignored** |
| `BYRO_LOCK_ORDER_CHECK=1` lane (CI's own command) | **9,614 passed / 0 failed, no `lock-order cycle`** |
| `scripts/check-shader-artifacts.sh` | **green** (36 shaders + the early-test variant, glslang 16.4.0) |
| CI feature variants (`--no-default-features`, `tracing-tracy`, `dhat-heap`, spt `recon`, `--workspace --all-targets`) | **all compile**; the `--no-default-features` pair of warnings is open #5327 |

**New findings from this sweep: 0 CRITICAL, 0 HIGH, 0 MEDIUM, 4 LOW.** None is a missing fix. Three are stale claims left behind by a fix that landed, one is a recorded-red hardware gate with no tracker. This sweep found no `Regression of #N` the other 28 reports did not already have; its contribution is the independent confirmation, the 95 PASS rows, and the negative results in the last section.

- REG-2026-10-12-01: LOW. `EsmIndex::leveled_spells` still says "No production reader yet … tracked by #4415"; #4415 shipped and `equip.rs:888` reads it.
- REG-2026-10-12-02: LOW. `merge.rs:675` still says the Starfield `.mat` format "is not yet parsed (tracked in #4277)"; #4277 closed with `loose_mat.rs`.
- REG-2026-10-12-03: LOW. The `cornell_forced_low_blas_budget_preserves_rt_shadows` hardware gate is recorded as failing (`missing_rigid=1`) by two 10-11 commits while `rt-lighting-material-recovery.md` says it passes; no issue tracks it.
- REG-2026-10-12-04: LOW. `ActorValues::set_permanent` has no caller and no test after #5412 re-routed the player `SetBase`.

## Regressions and incomplete fixes raised by sibling reports — independently re-confirmed

Each row below was re-derived from the current code, not copied. They are cited, not re-reported, and not counted in the totals above.

| Closed issue | Kind | Sibling ID | What this sweep checked | Verdict |
|---|---|---|---|---|
| #2586 | regression | EXT-D6-2026-10-12-01 (HIGH) | `worldspace_lod_grid_origin` (`cell_loader/lod_support.rs:115-143`) returns the `usable_cell_bounds().min` (`NAM0`) for Skyrim+/FO4, falling back to the exterior-cell extent; nothing reads `lodsettings\<ws>.lod` (no `lodsettings` consumer in `byroredux/src` or `crates`). The Tamriel numbers ((-57,-43) vs (-96,-96)) are the sibling's real-data census; I re-read the code path, not the archive | **Confirmed by code path**; the data half is the sibling's |
| #5496 | incomplete | EXT-D6-2026-10-12-02 (MEDIUM) | `worldspace_cell_bounds` was moved to `wctx.lod_worldspace_key()` (`lod_support.rs:158`) but `worldspace_lod_grid_origin` still does `.get(&wctx.worldspace_key)` at `:123` and `:132` | **Confirmed** |
| #5496 | incomplete | EXT-D6-2026-10-12-03 (MEDIUM) | `spawn_object_lod_quad` builds `object_lod_archive_path(scheme, &wctx.worldspace_key, …)` (`object_lod.rs:433`) and `object_lod_atlas_path(scheme, &wctx.worldspace_key)` (`:466`); selection uses the LOD key | **Confirmed** |
| #5495 | regression (never took effect) | GAME-D5-2026-10-12-01 (MEDIUM) | `grep CreatureActor`: the only `insert`s are inside `#[cfg(test)]` (`sandbox.rs:547`); the readers are `sandbox.rs:313` and `eat_sleep.rs:502`. `a_creature_actor_never_takes_furniture` is green because it hand-inserts the marker. No production code stamps it | **Confirmed** |
| #5031 | incomplete | GAME-D1-2026-10-12-01 (MEDIUM) | `reconcile_worn_gear_excluding(world, wearer, failed_form: Option<u32>)` (`loot_appearance.rs:565`) holds out ONE form (`.filter(|f| Some(**f) != failed_form)`); two forms that both fail alternate, each re-queuing the other every frame | **Confirmed** |
| #5662 | residual | PAR-D2-2026-10-12-07 (MEDIUM) | `read_vector_curve` (`crates/hkx/src/animation.rs:797`, bounds at `:820-831`) accepts finite `min`/`max` and stores them; `values.push(min + quantized * (max - min))` (`:855-856`) overflows to `inf`/NaN when `max - min` exceeds `f32::MAX` (`min=-3e38, max=3e38`), and the `non_finite` latch never trips | **Confirmed** |
| #5469 | regression | TD2-06 (LOW) | `no_source_scan_cuts_with_the_weak_cfg_test_needle` (`workspace_hygiene_tests.rs:710`) bans only `find("#[cfg(test)]")`, `rfind`, `split` and the `\n`-prefixed `split_once`. Bare `split_once("#[cfg(test)]")` survives at `boot/world.rs:538`, `systems/combat_ai.rs:689`, `material_translate.rs:1522`, `systems/cinematic.rs:1932`, `cell_loader/load.rs:1398`, `terrain_lod_btr.rs:616` and `debug-server/src/registration.rs` (four), and `split("\n#[cfg(test)]")` / `find("\n#[cfg(test)]")` at `app_step.rs:1485,1508`, `mesh_instance.rs:1766,1797,1848,1948`, `debug-server/src/evaluator.rs:944` | **Confirmed** |
| #5470 | regression | TD3-02 (LOW) | `no_string_literal_embeds_a_space_run` exempts `byroredux/src/main.rs` by name (`:397`); `main.rs:332,338,409,417` still hold the 6+ space continuation runs the gate exists to ban | **Confirmed** |
| #5471 | regression | TD3-03 (LOW) | `feature-matrix.md:89` "10 of ~17" + `:102` "remaining 10" (10 + 10 > 17, and Eat/Sleep/Dialogue named parse-only); `npc-spawn-ai-packages.md:9` "exactly seven" vs `:522` "Ten"; `ROADMAP.md:61` "Seven", `:94` "10 of ~17" | **Confirmed** |
| #5512 | side effect | CONC-D7-2026-10-12-01 (MEDIUM) | `streaming_helpers.rs:866-896`: the stale-payload `loop { try_recv … salvage_stale_imports … }` has no `budget.should_yield()` between iterations; `should_yield` is checked once at `:860` before it | **Confirmed** |

Related, not a closed-issue regression, and reported by the FO4 sibling as NEW: FO4-D4-01 (HIGH, MSWP per-shape swap compares bare `BNAM` against the NIF's `Materials\…`/absolute shader name) means the guards for #4290 and #5497 pass on synthetic fixtures that author the same string on both sides while the REFR swap never fires on real data (0 of 7,132 placements). I did not re-derive its 7,132-placement census; I confirm the compare at `mesh_instance.rs:113-143` is a verbatim `eq_ignore_ascii_case`.

## Findings

### REG-2026-10-12-01: `leveled_spells` still documents a consumer that shipped (#4415)
- **Severity**: LOW
- **Dimension**: Doc rot after a landed fix
- **Location**: `crates/plugin/src/esm/records/index.rs:195-203`
- **Status**: NEW (stale after the #4415 magic runtime; `Existing`: none, no tracker)
- **Description**: The field doc says "No production reader yet: the `SPLO` → spell-list → MGEF runtime that consumes it is tracked by #4415". #4415 closed 2026-10-06 and its runtime reads the map: `crates/plugin/src/equip.rs:888` passes `&index.leveled_spells` into the spell-list resolver that `resolve_actor_spells` / `resolve_racial_spells` (and #5413's player stamp) use.
- **Evidence**: `grep -rn leveled_spells --include='*.rs'` → `index.rs:203` (decl), `dispatch_container.rs:69` (insert), `index.rs:715` (category map), `equip.rs:888` (read), plus real-data tests.
- **Impact**: The doc tells a reader the leveled-spell tier is unreachable in production; it is reachable. Costs an auditor a wrong "dead map" finding.
- **Related**: #4415, #3617, #5413
- **Suggested Fix**: Replace the sentence with "read by `equip::resolve_actor_spells` (`equip.rs:888`), #4415".

### REG-2026-10-12-02: `merge.rs` still says the Starfield `.mat` format is unparsed, "tracked in #4277"
- **Severity**: LOW
- **Dimension**: Doc rot after a landed fix
- **Location**: `byroredux/src/asset_provider/material/merge.rs:672-676`
- **Status**: NEW (stale after #4277 / #5396)
- **Description**: The unknown-extension arm's comment reads "The .mat format is not yet parsed (tracked in #4277)". #4277 closed 2026-10-06 with `asset_provider/material/loose_mat.rs`, and the very next arm up (`merge.rs:583-592`, `apply_loose_mat`) decodes a loose `.mat`; `:536-540` says so ("A loose file decodes through `loose_mat` (#4277/#5396)"). Only the no-CDB fall-through still reaches the generic arm.
- **Evidence**: `merge.rs:583` `if starfield_cdb_gate && path.ends_with(".mat")` → `apply_loose_mat(...)`; `:675` the stale sentence.
- **Impact**: A reader following the pointer lands on a closed, delivered issue. No runtime effect.
- **Related**: #4277, #5396, `/audit-starfield`
- **Suggested Fix**: Say the arm is reached only when no CDB is loaded, and drop the "not yet parsed / tracked" clause.

### REG-2026-10-12-03: the forced-low-BLAS-budget hardware gate is recorded red, and the doc says it passes
- **Severity**: LOW
- **Dimension**: Fragile-area guards (test gap)
- **Location**: `byroredux/tests/cornell_rt_oracle.rs:438` (`cornell_forced_low_blas_budget_preserves_rt_shadows`); claim at `docs/engine/rt-lighting-material-recovery.md:72-76`
- **Status**: NEW (no issue; searched open and closed for `missing_rigid`, `forced_low`, `blas_budget`)
- **Description**: The recovery doc says the ignored gate "passes on the RTX 4070 Ti". Commits `48bed0504` and `4d991bad7` (both 2026-10-11, `Refs #5369`) each state that it "fails identically before and after (pre-existing, `missing_rigid=1` under the BLAS budget override)". The test asserts a complete two-instance TLAS under a one-byte static-BLAS budget; `missing_rigid_blas` is the counter `tlas.rs:585` increments. Nothing in `acceleration/` changed since 2026-10-07 (`72028d4fd` #5250), so the red state either predates the 10-11 commits or is device-specific (the same author's earlier diagnosis that day ran on lavapipe).
- **Evidence**: the two commit bodies; the doc sentence; `acceleration/tlas.rs:511,585,785`.
- **Impact**: Unknown. If it is a real regression of the #4883/#4884/#5195/#5201 BLAS-budget work, a budget-pressured scene (Cydonia, FO4 Institute) can publish a TLAS missing a rigid instance. If it is device-specific, the doc claim is merely unqualified. This sweep cannot tell: the gate needs an RT device plus a display, and no engine launch is allowed this run.
- **Related**: #4883, #4884, #5195, #5201, #5250, #5369
- **Suggested Fix**: Run the gate on the 4070 Ti once; if red, bisect against `72028d4fd`..`5d31fd637` and file; if green, qualify the commit-message claim in the doc. Either way note the lavapipe/hardware split in the test's doc comment.

### REG-2026-10-12-04: `ActorValues::set_permanent` is dead and untested since #5412
- **Severity**: LOW
- **Dimension**: Dead code after a superseding fix
- **Location**: `crates/core/src/ecs/components/actor_values.rs:184`
- **Status**: NEW (superseded by #5412; `Existing`: none)
- **Description**: #5239 added `set_permanent` as the destination of a player `SetBase` on a derived pool. #5412 re-routed that to `set_override` ("the player `SetBase` reroute no longer lands here"), deleted `set_permanent_survives_the_player_refresh`, and left the method behind with a doc calling it "the `modav` counterpart". `modav` uses `mod_permanent` (additive); nothing calls `set_permanent`. `.claude/commands/audit-character/SKILL.md:191,234` still names it as a hazard.
- **Evidence**: `grep -rn "set_permanent\b" --include='*.rs' byroredux crates` → the definition only.
- **Impact**: A public mutator that discards every constant-spell bonus on the pool (the exact #5412 defect) sits unguarded, one wrong call away from reintroducing it.
- **Related**: #5239, #5412, #5039
- **Suggested Fix**: Delete it, or keep it and add a test that states why `modav` must not use it.

## Per-issue results

Every row below is **PASS**: the fix is present at HEAD and the named guard ran green in the whole-workspace pass. "Fix present" was established by (a) the guard, (b) for the window fixes, `git blame` of the fix commit's added production lines at HEAD (most are 85-100% intact; the remainder were edited by later fixes to the same function, each reviewed), and (c) a direct read for the ones called out in the notes. Fix commits are the first `Fix #N` keyword commit; where a later commit reworked the site it is named in the notes.

### A. Closed since the baseline, not cited by any sibling report (26)

| Issue | Title | Closed | Fix commit | Guard test (ran green) |
|---|---|---|---|---|
| #4134 | Character-controller/ground-probe capsule shapes use a floor-only .max(1e-3),  | 2026-10-10 | `002fbd6ef` | `character_capsule_sanitises_non_finite_and_absurd_extents` |
| #5170 | Starfield WATR noise-UV tile sizes stay unlifted beside the lifted lanes with  | 2026-10-12 | `2dab4ff48` | `starfield_public_index_lifts_watr_dnam_distances`, `starfield_watr_defers_the_unclassified_dnam_lanes` |
| #5171 | No test pins the Starfield LGTM unit lift — #5002 added four height-field lift | 2026-10-09 | `2dab4ff48` | `legacy_index_keeps_authored_lgtm_and_xcll_lighting_distances` |
| #5343 | #5169 left FO76's pigment lanes 0–2 on Starfield's 0..20 scale — a test pins F | 2026-10-10 | `7ef007898` | `resolve_water_material_maps_starfield_roughness_to_sun_exponent` |
| #5352 | A recovery substep skips `clamp_explosive_velocities`, so every body the resto | 2026-10-10 | `1efdfda4a` | `a_restore_substep_still_clamps_the_bodies_it_did_not_claim` |
| #5353 | The articulation-DOF clamp caps a dynamic ragdoll root's free-joint LINEAR vel | 2026-10-10 | `e128218eb` | `a_free_ragdoll_falls_at_gravity_without_tripping_the_dof_clamp`, `an_exploding_free_root_dof_is_capped_at_the_body_speed_cap` |
| #5357 | The swimming player's drift scales the placed current by submerged fraction; t | 2026-10-09 | `dd3a506e9` | `swimming_drift_scales_the_plane_by_fraction_and_the_marker_at_full_strength`, `player_water_state_survives_an_unregistered_current_volume_storage` (+1) |
| #5359 | Skyrim's per-NPC skin (`NPC_.WNAM`) is never decoded — 442 vanilla actors, inc | 2026-10-09 | `edb5fbdfe` | `npc_skyrim_wnam_skin_decodes_remapped`, `prebaked_npc_wnam_skin_overrides_the_race_default_and_follows_the_traits_terminal` |
| #5373 | `forcegreet_system` and `eat_sleep_system` write the walk step into `GlobalTra | 2026-10-09 | `f0c683ef1` | `forcegreet_walk_survives_transform_propagation`, `eat_actor_far_from_destination_walks` (+1) |
| #5381 | CDB slot 6 (`_height`) lands in the canonical `height` role, which the rendere | 2026-10-09 | `46dfdcc6c` | `slot_height_texture_stays_parked_and_never_arms_parallax` |
| #5385 | Story Manager `SNAM` is the *previous* sibling, but it is decoded and dispatch | 2026-10-09 | `846e4a6dd` | `snam_previous_semantics_walk_head_first` |
| #5386 | `SMQN` `RNAM` is stored as 24 × hours (xEdit scale `1/24`), but it is read as  | 2026-10-09 | `a621d5352` | `fnam_unam_pair_per_quest_not_node_level`, `rnam_stored_scale_decodes_to_authored_hours` (+1) |
| #5394 | Story-Manager-started quests lose their event-filled aliases across a save/loa | 2026-10-09 | `03e51dfad` | `reference_form_of_names_the_player_and_placed_references`, `from_event_alias_re_resolves_recorded_reference_forms_after_a_load` (+1) |
| #5416 | The #5261 `rt-integrity` assertion in the vulkan-validation lane has no source | 2026-10-09 | `1c536a02b` | `vulkan_validation_job_requires_live_rt` (`byroredux/src/scheduler_access_tests.rs:593`) pins the workflow grep |
| #5419 | `StoryEventAliasFill` keeps event `EntityId`s for the session and rebinds them | 2026-10-09 | `1dfdeb4f9` | `from_event_alias_skips_stale_and_dead_slot_entities` |
| #5420 | The SM decode leaves xEdit-defined fields marked "[open]/unverified": `QNAM` i | 2026-10-09 | `5489cbe65` | `smqn_decodes_xedit_named_tail_fields` |
| #5421 | REGN `CNAM` "climate" decode has no xEdit definition and zero vanilla occurren | 2026-10-09 | `bd052048a` | `parse_eczn_picks_owner_rank_flags` |
| #5422 | PKIN `VNAM` is xEdit's integer *Version*, but it is documented as a form ID an | 2026-10-09 | `0d436e642` | `pkin_vnam_version_stays_raw_under_remap` |
| #5431 | The new `GetButtonPressed` (CTDA fn 0) mapping rests on a false premise, and i | 2026-10-11 | `26906d3cb` | `function_index_zero_is_not_catalogued`, `get_distance_variant_keeps_its_doc_comment` |
| #5453 | The #5311 "move-only" world split dropped `CharacterMoveResult`'s `Debug`/`Clo | 2026-10-10 | `4739c411a` | `CharacterMoveResult` derive restored (`queries.rs:15`); rationale guard gained the `None is passed` assertion |
| #5516 | #5374 implemented Oblivion inherit-all twice. The parse-time PNAM stamp is the | 2026-10-11 | `65cadde2f` | `oblivion_child_gets_the_inherit_all_stamp_and_eras_stay_real`, `oblivion_child_inherits_the_parent_authored_climate` (+1) |
| #5517 | `wths_only_climate_resolves_the_defaultweather_stand_in`'s "both callers share | 2026-10-11 | `65cadde2f` | `resolvable_wlst_wins_over_the_stand_in` |
| #5519 | Climate / LOD doc rot after the 10-08 fix wave: the "Oblivion-era REGN CNAM" c | 2026-10-11 | `65cadde2f` | docs only; no test cites it |
| #5531 | The articulation-DOF clamp logs no articulation, so its per-substep floods (an | 2026-10-10 | `9d826272b` | `the_dof_clamp_attributes_each_rig_by_its_root_body` |
| #5616 | Consumable CTDA gating duplicates the "known CTDA lengths", the run-on offset  | 2026-10-11 | `5eff8c744` | `run_on_decode_gate_and_enum_agree_on_the_known_range` |
| #5618 | `heave_hold_damping` — the stabilising coefficient is a measured constant for  | 2026-10-11 | `f7ea9d1b1` | `thin_floats_settle_at_a_denser_fluid_ratio`, `heave_hold_damping_scales_with_stiffness_and_stays_stable` |

Notes: #5343/#5516/#5517/#5519 fix sites moved from `env_translate.rs` to `env_translate/{water,climate}.rs` in `c140cb48e` (#5584); the gating predicates (`concentration_pigments_are_starfield_scale`, `resolve_water_material` FO76 arm) and their assertions are intact at `env_translate/water.rs:412,1502`. #5373's `eat_sleep.rs` step still writes `Transform` (`:345-362`); 64 of 83 added lines survive, the rest were reshaped by #5449/#5500/#5527. #5352's recovery-substep clamp is at `world/mod.rs:1444-1450`. #5353 and #5531 sit in `ragdoll.rs`/`world/recovery.rs`; every added production line is still present. #5170/#5171 are pins, not behaviour: they assert the *unlifted* lanes stay unlifted until a capture settles them (`spatial_units_tests.rs:336`).

### B. Closed since the baseline, production wiring re-read (24)

Selected because the fix is a helper or a gate whose production caller is easy to lose (the #5495 shape). Each helper has a production caller at HEAD: `attach_to_player` stamps `RaceSpells` (`inventory.rs:761`); `leave_dialogue_page` is called from all three leave paths (`main.rs:1351,1370,1408`); `raise_kill_story_event` from the four live death producers; `reconcile_player_equipped_weapon` / `reset_player_factions_to_record` / `park_unresolved_snapshot_rows` / `restore_resident` from `save_io.rs`; the #5359 `worn_skin` override at `npc_spawn.rs:1351`.

| Issue | Title | Closed | Fix commit | Guard test (ran green) |
|---|---|---|---|---|
| #5413 | The #4415 racial spell pair always targets the player, but the player never re | 2026-10-09 | `212a71dba` | `attach_to_player_stamps_the_character_seed` asserts `RaceSpells` and the Remove/Add round trip |
| #5409 | Escape (or the Inventory key) on the native dialogue page closes it without `e | 2026-10-09 | `14b0d33f2` | `leaving_the_dialogue_page_ends_the_open_conversation` |
| #5410 | Voice lines ride the fire-and-forget queue path. The position is frozen at lin | 2026-10-09 | `d1a7ee8c6` | `play_oneshot_following_drops_on_inactive_audio`, `stop_sounds_for_is_a_no_op_on_a_headless_world` (+1) |
| #5432 | Dialogue voice segments are fire-and-forget — changing topic, closing or the s | 2026-10-11 | `40b77828b` | `dead_actor_reconciler_stops_the_actors_voice_line`, `melee_charal_bonus_honors_scope_and_output_contracts` (+4) |
| #5434 | The `KILL` story event is raised by one of the four live death producers and n | 2026-10-11 | `c85d50356` | `inventory_projection_aggregates_portable_forms_and_equipment_slots`, `unarmed_hit_uses_explicit_fallback_damage` (+5) |
| #5435 | `StoryLocationCursor` survives an in-process load, so loading a save from anot | 2026-10-11 | `557ead21d` | `load_reset_makes_the_first_cloc_independent_of_the_outgoing_session`, `live_load_drain_resets_the_story_location_cursor` (+1) |
| #5511 | `PackRecord::force_greet` reads only the package's own procedure tree, so 334  | 2026-10-11 | `72e863d80` | data-gated only: `force_greet_skyrim_tree_dialect_floor` (ignored) |
| #5498 | FO4 `NPC_.TPTA` per-flag Template Actors are never decoded, so every Use-X fla | 2026-10-10 | `99c22a8db` | `tpta_entry_overrides_tplt_for_its_flag`, `tpta_entry_without_tplt_still_inherits` |
| #5358 | Skyrim's `BODT` biped-slot sub-record is never decoded (10 ARMOs, 916 ARMAs) — | 2026-10-09 | `479414ffe` | `skyrim_armo_reads_bodt_body_template`, `parse_arma_reads_bodt_biped_flags` (+2) |
| #5635 | `execute_statements` recurses once per executed `If` and clones the tail — lin | 2026-10-12 | `afa5186a1` | `sequential_ifs_execute_without_recursion_or_tail_clones` (+2) in `papyrus_provider/tests/execute.rs` |
| #5629 | ObScript reference indices resolve against the SCRO-only list, but compiled `r | 2026-10-12 | `aac6b25b3` | `reference_indices_address_the_interleaved_scro_scrv_list`, `an_unresolved_explicit_caller_is_declined_not_run_on_the_owner` (+2) |
| #5554 | (latent) `impact_audio_system` `break`s out of the frame on one unmapped mater | 2026-10-11 | `71b86091b` | `crossings_from_a_rolled_back_substep_are_discarded`, `an_unmapped_material_skips_only_its_own_impact` |
| #5547 | Buoyancy is re-derived once per render frame and held constant across every su | 2026-10-11 | `c5bf46e59` | `thin_floats_settle_and_sleep_at_any_frame_rate`, `heave_hold_damping_is_zero_for_tall_bodies_and_grows_as_one_over_h` |
| #5561 | A sleeping float re-wakes for waves only when the per-frame depth change excee | 2026-10-11 | `0d28abfe3` | `a_slept_float_wakes_for_waves_at_high_frame_rate` |
| #5548 | The swimming player takes harmful-water damage twice per frame, from the contr | 2026-10-11 | `c28571563` | `a_wet_player_loses_exactly_dps_times_fraction_times_dt_per_frame`, `the_controller_leaves_authored_water_damage_to_water_damage_system` |
| #5539 | rapier 0.36's snap-on-every-horizontal-move sinks walking NPCs into convex flo | 2026-10-11 | `2917ab372` | `a_zero_dt_boot_step_matches_a_normal_first_step_for_embedded_npcs`, `grounded_npc_does_not_sink_through_convex_floors_across_absolute_y` |
| #5545 | The surface-aware footstep probe starts inside the player's own capsule and ne | 2026-10-11 | `3909778eb` | `surface_material_under_skips_the_walkers_own_capsule`, `footstep_declares_its_physics_reads` (+1) |
| #5560 | On a flat floor the KCC still climbs a phantom slope and drops `grounded` on 1 | 2026-10-12 | `f89802955` | `kcc_reports_grounded_for_a_capsule_at_rest`, `walking_capsule_loses_kcc_grounded_on_frames_the_support_probe_covers` (+1) |
| #5490 | CONC-D5-2026-10-09-01 / SAVE-D4-2026-10-09-01: Regression of #5379 — The sessi | 2026-10-11 | `99cd9fd6c` | `pending_readoption_victims_are_the_whole_subtree_closure`, `pending_readoption_victims_spare_the_player_rig` |
| #5491 | Streaming `shutdown` does not stop the worker — the queued backlog keeps it pa | 2026-10-11 | `b6aeecf56` | `cancel_flag_stops_a_worker_with_a_queued_backlog` |
| #5066 | `npc_dialogue` shadows its `LoadedCellIndex` guard instead of dropping it, so  | 2026-10-09 | `1c15a2270` | fixed under `1c15a2270` (titled #5372); three scoped guards at `npc_dialogue.rs:519,706,793`; lock-order lane green |
| #5372 | `raise_hello_story_event` reads `LoadedCellIndex` under the `StoryEvent` write | 2026-10-09 | `1c15a2270` | `hello_event_carries_greeter_greeted_and_session_location` |
| #5430 | `forcegreet_system` moves NPCs outside the `BYRO_NO_AI_LOCOMOTION` kill switch | 2026-10-11 | `bd366e300` | `locomotion_kill_switch_gates_exactly_the_motion_systems`, `walk_animation_registers_after_all_seven_movers` (+2) |
| #5449 | `eat_sleep_system` retries a whole-world furniture gather every frame, per act | 2026-10-11 | `35a009431` | `walk_animation_registers_after_all_seven_movers`, `unseatable_arrived_actor_retries_on_a_coarse_cadence` |

### C. Older churn-ranked fixes no earlier sweep verified (45)

Ranked by overlap with the 420 churned files; the 08-10 sweep's list was subtracted. #4414/#4416 are feature issues closed with their consumer, included because `byroredux/src/npc_spawn/resumable.rs` and `env_translate.rs` (where they landed) were both split and rewritten since.

| Issue | Title | Closed | Fix commit | Guard test (ran green) |
|---|---|---|---|---|
| #3098 | REFR XLOC is never parsed — every locked door and container in every game open | 2026-08-18 | `1e9723ab2` | `parse_refr_extracts_xloc_lock_data` |
| #3256 | resolve_cached_waypoints has no navmesh-residency invalidation | 2026-08-29 | `d6bc6a74e` | `resolve_cached_waypoints_retries_a_negative_result_after_residency_changes`, `resolve_cached_waypoints_invalidates_a_positive_path_after_residency_changes` (+2) |
| #3278 | Effect::Disable has no production consumer, and its receiver resolution is nar | 2026-08-29 | `d6bc6a74e` | `dispatch_disable_resolves_an_alias_bound_receiver`, `dispatch_disable_then_stage_cascade_and_package_evaluation` (+8) |
| #3332 | Escort ignores both authored collect-distance fields and hardcodes 128 — every | 2026-08-29 | `d2d5e067f` | `parse_pack_reads_pke2_escort_distance`, `authored_collect_distance_beats_the_engine_default` (+1) |
| #3333 | un-seating a Sandbox actor at a package change leaves its `AnimationPlayer` fr | 2026-08-29 | `d2d5e067f` | `sandbox_scratch_clears_every_field_every_frame`, `unseating_restores_the_pre_seat_animation_player` |
| #3354 | `sandbox_seat_system` rebuilds the whole world-space seat table and its `HashM | 2026-08-29 | `d2d5e067f` | `fully_seated_world_skips_the_seat_table_rebuild`, `a_creature_actor_never_takes_furniture` |
| #3368 | the header's folder-records offset (bytes 8..12) is never read — the folder-ta | 2026-08-29 | `d2d5e067f` | `padded_header_folder_offset_is_honoured` |
| #3510 | precombine shared geometry is uploaded once per placed instance — up to 4.6x V | 2026-08-31 | `6a3bdbb23` | `dedup_requires_both_a_cache_key_and_a_matching_length` |
| #3640 | APP_CULLED geometry with a live visibility controller is dropped at import — 1 | 2026-09-02 | `452cccd0b` | `culled_or_marker_particle_systems_are_not_imported_by_either_walker` |
| #4282 | Starfield BSLightingShaderProperty.wetness and .luminance are parsed but have  | 2026-09-23 | `d3e043d3e` | `bslighting_wetness_and_luminance_cross_the_boundary`, `bslighting_wetness_and_luminance_default_to_none` (+2) |
| #4290 | An MSWP material swap swaps the textures but keeps every non-texture material  | 2026-09-14 | `82c4450d6` | `morph_gpu_buffer_preserves_filtered_source_index_holes`, `mswp_swap_merges_the_target_material_onto_the_pre_merge_snapshot` (+3) |
| #4334 | onlyOnce actor-base stage triggers re-arm after a cell reload (no persistent P | 2026-09-23 | `d3e043d3e` | `fire_once_advances_park_the_reference_script_state`, `trigger_enter_respects_player_only_gate` (+2) |
| #4414 | NPC combat: ambient faction hostility from FactionRelations | 2026-09-24 | `9789d8153` | `combat_gate_effects_set_faction_hostility_and_arm_ai_combat_state`, `set_enemy_defaults_omitted_flags_and_carries_literal_ones` (+2) |
| #4416 | IMGS image-space records: DNAM decode + per-cell render consumer | 2026-09-24 | `afd6a73f7` | `imads_compose_onto_the_base_image_space`, `fo3_imgs_dnam_decodes_the_grade_by_size` (+4) |
| #4636 | NIFAL-D8-2026-09-21b-01: BGSM merge's NIF-first texture precedence is an unsou | 2026-09-22 | `78d079707` | `dead_nif_lut_hands_the_bgsm_both_the_slot_and_the_enable_bit`, `a_bgsm_that_takes_the_lut_slot_shields_it_from_ancestor_or_bits` (+3) |
| #4658 | Every archive-extract consumer discards non-NotFound errors, so a corrupt entr | 2026-09-29 | `ec63d2636` | `corrupt_override_falls_through_and_is_named_once`, `unfetchable_cdb_resolves_none_and_memoizes_the_failure` |
| #4661 | Entry-count reservations are absolute rather than file-relative, and BSA's dec | 2026-09-29 | `ec63d2636` | `truncated_record_table_names_the_record_and_declared_count`, `malicious_file_count_u32_max_rejected_before_allocation` (+4) |
| #4662 | BA2 short-decode warnings name neither the archive nor the entry, and DX10 nev | 2026-09-29 | `ec63d2636` | `extract_dx10_rejects_a_short_non_final_chunk_naming_the_entry`, `extract_dx10_tolerates_a_short_final_chunk` (+1) |
| #4663 | A stale duplicate test pins "current" LZ4 under-run behaviour on a false premi | 2026-09-29 | `ec63d2636` | `decompress_chunk_lz4_under_run_returns_actual_length_not_declared` |
| #4697 | Loose-item pickup has no player input path, and the pickup/tombstone flow has  | 2026-09-24 | `b46fd9d33` | `physical_activate_picks_up_a_loose_item_stack` |
| #4698 | Disable()d doors and containers stay interactive — an invisible working door a | 2026-09-24 | `b46fd9d33` | `disabled_door_and_container_are_not_interaction_candidates` |
| #4706 | Pickup always grants one item — the REFR item count (XCNT) is not decoded | 2026-09-24 | `b46fd9d33` | `refr_xcnt_decodes_the_placed_stack_size`, `physical_activate_picks_up_a_loose_item_stack` |
| #4784 | When any transport emitter is active, every froxel in the grid runs the RK2 ba | 2026-10-03 | `c705c310d` | `occupancy_marks_reset_between_builds`, `transport_occupancy_gate_is_wired_into_the_inject_pass` (+1) |
| #4820 | `Enable()` at runtime on a ref that was disabled at cell load makes it interac | 2026-09-29 | `f87490826` | `a_root_spawned_disabled_stays_uninteractive_after_a_live_enable` |
| #4821 | The `npc_combat_ai_system` access declaration claims to cover every pair `clea | 2026-09-29 | `424aad268` | `npc_combat_ai_system_declares_everything_it_acquires` |
| #4930 | The BGSM flow-map UV offset kept the pre-#4728 sign and grows without bound wi | 2026-09-28 | `55dd8fa7e` | `flow_map_offset_is_subtracted_and_bounded`, `water_fragment_shader_uses_authored_underwater_response` (+1) |
| #4931 | #4734's regression guard was deleted by #4727's commit; the dead-default check | 2026-09-28 | `55dd8fa7e` | `wind_direction_authored_is_decided_per_layout`, `dead_default_wind_direction_yields_no_physics_flow_for_named_creeks` (+5) |
| #4936 | Object-LOD mesh uploads carry no provenance — `.bto` and FO3/FNV `blocks\` mes | 2026-09-28 | `55dd8fa7e` | `every_lod_mesh_upload_is_tagged_lod` |
| #4942 | The ReSTIR direct-radiance EMA has no light-change invalidation, so flicker, p | 2026-09-28 | `a37fcba3c` | `history_mode_keeps_the_camera_threshold_and_adds_the_scene_one`, `every_camera_ubo_mirror_describes_dof_w_as_the_history_mode` (+4) |
| #4943 | SVGF's "scene unchanged" signal ignores draw-set changes (spawn/despawn/enable | 2026-09-28 | `a37fcba3c` | `rigid_instance_set_change_covers_spawn_despawn_and_swap`, `scene_static_signal_sees_rigid_instance_set_changes` (+1) |
| #4944 | Under `--upscaler taa`, water pixels are resolved as the lake bed — its stable | 2026-09-28 | `a37fcba3c` | `taa_resolve_consumes_the_reactive_mask`, `first_frame_params_y_is_one_when_frames_since_creation_is_zero` |
| #4954 | `upload_lights` now runs a std SipHash `HashMap` and a `DefaultHasher` over a  | 2026-09-28 | `a37fcba3c` | `remap_change_changes_hash`, `per_frame_light_upload_does_not_use_siphash` (+1) |
| #4955 | #4846's generated `RENDER_LAYER_ARCHITECTURE` / `FOG_VOLUME_SHAPE_*` reached o | 2026-09-28 | `a37fcba3c` | `layer_and_fog_shape_discriminants_are_never_hand_typed`, `dielectric_ior_floor_is_vacuum_everywhere` |
| #4971 | Froxel local-light budget is spent in cluster-list order, and the directional  | 2026-09-28 | `b978bb5a1` | `froxel_light_budget_charges_only_contributing_local_lights` |
| #4972 | Residue of closed #4860 — `spawn_nif_lights` fabricates `SHADOW_OMNIDIRECTIONA | 2026-09-28 | `b978bb5a1` | `default_emitter_uses_the_shared_local_light_visibility`, `spawn_nif_lights_rotates_direction_by_reference_rotation` |
| #4973 | `dielectricF0FromIor`'s `eta` floor does not prevent the mirror-class F0 its c | 2026-09-28 | `b978bb5a1` | `dielectric_ior_floor_is_vacuum_everywhere` |
| #4976 | the "frame parameters absent" native-blit branch hard-codes the scene image's  | 2026-09-28 | `b978bb5a1` | `every_native_blit_sources_from_the_scene_images_actual_layout` |
| #4977 | Debug-UI "GPU passes Σ" double-counts volumetrics — the new inject/integrate b | 2026-09-28 | `b978bb5a1` | `gpu_pass_sum_excludes_nested_child_brackets` |
| #4978 | `render.debug probe` resolves the hit entity through a later frame's SSBO→enti | 2026-09-28 | `b978bb5a1` | `probe_hit_entity_resolves_against_the_armed_frames_map`, `between_frames_is_captured_before_draw_frame_runs` |
| #4979 | `water.frag` and `groundcover_blade.frag` ignore every non-water structured de | 2026-09-28 | `b978bb5a1` | `main_pass_fragment_shaders_honour_structured_debug_views` |
| #4980 | `GeometryTimerPhase` hard-codes query slot 46 and active bit 23 with no range  | 2026-09-28 | `b978bb5a1` | `every_timer_slot_and_active_bit_is_unique_and_in_range` |
| #5037 | NPC topic ownership ignores speaker, DIAL category and branch structure — Eltr | 2026-09-30 | `7ab87c0fb` | `lstring_resolved_via_strings_table_guard` |
| #5039 | #4674 materialises the player's `PlayerOnly` derived stats (Health/AP) into `A | 2026-09-30 | `c9254beb8` | `player_only_refresh_follows_endurance_and_level`, `player_only_refresh_leaves_actor_general_rows_derived` (+1) |
| #5042 | Constant-spell modifiers on formula-derived AVs create a carried `{base 0, +mo | 2026-09-30 | `c9254beb8` | `get_actor_value_composes_a_modifier_only_entry_onto_the_formula`, `modifier_only_entry_composes_onto_the_formula` (+3) |
| #5055 | GpuLight.history_id is CPU-only identity shipped in the GPU struct (+16 B per  | 2026-10-03 | `c705c310d` | `merged_lights_feeds_the_scene_static_signals`, `gpu_light_is_64_bytes` (+1) |

Notes: #4942's `restir_history_mode` gained a third argument and a fourth mode in `2a7223121` (#5369); the `> 0.5` / `> 1.5` thresholds the shader reads (`triangle.frag:3931,4187`) still classify modes 2 and 3 as "deep", and `every_camera_ubo_mirror_describes_dof_w_as_the_history_mode` pins the mirrors. #4943's `rigid_instance_set_changed` is still folded into `scene_static` at `build_and_upload_instances.rs:737,752`. #4588 (not in the table) was superseded by #4831 and is correct. #3256, #3278, #3332, #3333 and #3354 live in the eat/sleep, travel and sandbox systems that took twelve window commits; their guards are green. (The creature-exclusion guard `a_creature_actor_never_takes_furniture` belongs to #5495 and hand-inserts `CreatureActor`, which is why it cannot see that production never stamps it.)

## Step 4 — unconditional fragile-area guards

All run inside the whole-workspace pass (`full_test.txt`), counted by name filter.

| Area | Result |
|---|---|
| GPU struct sizes | `gpu_` filter: 92 ok / 0 failed |
| NIFAL single boundary | `resolve_pbr`: 8 ok / 0 failed (`/audit-nifal` owns the deep pass) |
| Typed particle emitters | `apply_emitter_params`: 3 ok / 0 failed |
| Collision coverage | `::collision`: 160 ok / 0 failed |
| ReSTIR reservoir (`resRadiance` stays retired) | grep of `crates/renderer/shaders/` finds only the two retirement comments (`include/lighting.glsl:173`, `triangle.frag:3160`) |
| Scheduler access | `system_access_declaration_tests`: 8 ok / 0 failed |
| Save shape | `serde_default_guard_tests`: 9 ok / 0 failed; `FORMAT_MAJOR` is 38 and every window bump (v34 #5412, v35 #5394, v36 Wave-4 `RigidBodyData`, v37 #5542, v38 #5300) is documented at `crates/save/src/snapshot.rs` |

Not in the skill's table but run because a regression hides there: `check-shader-artifacts.sh` (committed `.spv` reproducible from GLSL — the staleness class of #5368; nine shader sources changed in the window with no `.spv` change — `caustic_splat.comp`, `cluster_cull.comp`, `exposure_meter.comp`, `groundcover_bench_bake.comp`, `sky_irradiance.comp`, `sky_prefilter.comp`, `triangle.vert`, `water.frag`, `water.vert` — and all still compile to identical SPIR-V, so those edits are comment- or constant-neutral); `check-text-source-integrity.sh` (no NUL bytes); `_audit-validate.sh` (exit 0; 5 advisory skill symbols, all file-name tokens or renamed fixtures, no stale path); the `BYRO_LOCK_ORDER_CHECK=1` workspace lane (#5535 holds: no cycle).

## What was looked for and not found

Recorded so the next sweep does not repeat it.

- **Fix erosion.** For every `Fix #N` commit before the baseline that touched a window-churned file, the added production lines were matched against every `.rs`/GLSL line at HEAD. The only fixes whose added lines vanished were superseded on purpose: #5244 (by #5334), #4588 (by #4831), #4593/#4790/#4511 (by #4881/#4882), #4691 (by #5357), #5239 (by #5412). No fix disappeared without a successor.
- **Deleted guards.** 123 test functions were deleted in the window; 33 names no longer exist anywhere. Each of the 33 was traced with `git log -S` to the window commit that intentionally changed its subject, and the successor test exists (for example `finish_partial_import_populates_furniture_and_flame_offset` → `…_attach_light_offset` in `c1a99c25e`; `parse_regn_picks_weather_and_color` → `…_worldspace_and_color` in `bd052048a`; the nalgebra `*_roundtrip` tests with the adapter in `ac81a09f9`). None was a silent drop of a guard that still has a live subject, which is the #4931 class.
- **Fix wired but never fired (the #5495 class).** Components and resources introduced in the window were checked for a production `insert`. `CreatureActor` is the only one with none; `ImpactConfig`, `VoiceSoundCache`, `StoryEventAliasFill` and `DialogueSpokenInfoForms` have production insert sites. The four live producers of `Dead` (`combat.rs:363`, `extensions/commands.rs:543`, `water.rs:75`, `character.rs:1584`) all call `raise_kill_story_event` after #5434.
- **Dependency-refresh drift.** glam 0.33 (`0eb8896bb`: `rotate_y`, `smoothstep` replaced hand-rolled versions; the commit keeps `a + (b - a) * t` because glam changed `lerp`; `Vec3::rotate_y` is the same matrix as the deleted `yaw_about_up`), rapier 0.36 (`ac81a09f9` keeps the quaternion renormalization and the `[0, π]` scaled axis), image 0.25 trimmed to `png` (every decode/encode site in the repo is PNG, `texture-upscale` re-adds its own codecs).
- **Closed without a commit.** Of 299 issues closed since 10-08, 15 have no `Fix #N` keyword commit in the window. #5159 and #5401 were closed on live evidence / an A/B measurement posted in the issue; the rest are fixed under a sibling's subject (#5066 under #5372, #5335/#5337/#5339 under #5334) or are feature trackers closed by their last phase (#5364, #5366, #5367). #5369 was auto-closed by `Fix #5369 (partial)` and reopened; its two residual commits and the closing comment (RMSE 8.1 → 3.4, gate `cornell_parked_penumbra_converges_at_one_shadow_ray`) hold.
- **Pre-existing and already tracked.** Workspace clippy red at HEAD (#5621); `--no-default-features` warnings (#5327); `byroredux-core` without `parallel-scheduler` built by no lane (#5624); #5661 fixed by `0d698d16c` but still open for want of a keyword.

## Not verifiable here

- `cornell_forced_low_blas_budget_preserves_rt_shadows` and every other `#[ignore = "requires an RT-capable Vulkan device…"]` oracle rung (REG-2026-10-12-03).
- The data-gated halves of #5359, #5358, #5400, #5405, #5410, #5487, #5498, #5511 (`--ignored`, real masters). Each has a default-lane guard that ran green; the real-data census was not re-run (the plugin `--ignored` suite is barred by the suite rules).
- Any visual or frame-time effect of the shader edits in `triangle.frag`, `svgf_temporal.comp`, `presentation.frag` (byte parity with source holds; looks cannot be judged without an engine run).

Publish with `/audit-publish docs/audits/AUDIT_REGRESSION_2026-10-12.md`; the sibling regressions above are published from their own reports.
