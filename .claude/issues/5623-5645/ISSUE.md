=== #5623
OPEN TD8-2026-10-11-03: `PerkRecord.perk_flags` is write-only "for backwards-compat with the prior stub" — no consumer outside one test
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

- **Severity**: LOW
- **Dimension**: 8
- **Location**: `crates/plugin/src/esm/records/misc/magic.rs:279-281` (decl), `:343-344` (only write; `is_trait` derived from it), `:1040` (only read: test assertion)
- **Status**: NEW
- **Effort**: trivial
- **Description**: Field comment says "Kept for backwards-compat with the prior stub; new code should prefer `is_trait`". `grep -rn perk_flags crates byroredux tools` -> 4 hits, all in magic.rs; no external consumer exists (no external users of a workspace-internal struct).
- **Impact**: Duplicate representation of the same DATA byte; the `u8` implies richer flag semantics than the parser provides.
- **Suggested Fix**: Delete `perk_flags`, parse into a local and set `is_trait`, update the test at magic.rs:1040 to assert `is_trait`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix

=== #5624
OPEN TD9-2026-10-11-01: `byroredux-core` with `parallel-scheduler` OFF is compiled by no CI lane
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

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

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **LOCK_ORDER**: If a RwLock scope changes, TypeId-sorted acquisition is preserved
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix

=== #5625
OPEN TD9-2026-10-11-02: Vulkan-gated `golden_frames` + `upscaler_quality` have no automated lane (the `rt-correctness` dispatch has no gate for them)
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

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

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **DROP**: If Vulkan objects change, the Drop impl is still reverse-order correct
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix

=== #5626
OPEN TD9-2026-10-11-03: No-panic-only / misnamed tests that assert nothing about the behaviour in their names
Filed from `docs/audits/AUDIT_TECH_DEBT_2026-10-11.md` (audit HEAD `d6362e652`).

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

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test / hygiene guard pins this specific fix

=== #5627
OPEN Starfield WTHS instance-value reader (OBJT/LIST/USER chunks) -> weather params
Follow-up named by #5611 (and the closed #5364 / #5514, which landed the WTHS reflection *schema* and the DIFF walk).

`WeatherSettingsRecord` (`crates/plugin/src/esm/records/weather_settings.rs`) decodes the REFL/RDIF container's structure — classes, fields, DIFF field indices, and a census of the instance chunks (`OBJT`/`LIST`/`USER`/`USRD`/`MAPC`) — but not the instance *values* (fog distances, colours, wind, ...). xEdit has no shipped reader for them, so the layouts need grounding from the vanilla corpus first.

Until this lands, Starfield exteriors on WTHS-only climates keep resolving weather through the authored `DefaultWeather` WTHR stand-in (#5363 / #5424), and `EsmIndex.weather_settings` has no production reader.

Done when: an instance-value reader exists for the schema `BgsReflection` already walks, the values map onto the engine's weather parameters (EXAL `translate_weather`), and the `DefaultWeather` stand-in is removed or demoted to a fallback.

Not started; no code depends on it.
=== #5641
OPEN SCR-D5-2026-10-11-04: `player_controller_system`'s Access row omits the KILL producer's acquisitions (#5434 hop into `story_events.rs`)
Filed from `docs/audits/AUDIT_SCRIPTING_2026-10-11.md` (HEAD 71b86091b).

- **Severity**: LOW
- **Dimension**: Scripting Dim 5
- **Location**: `boot/schedule/early.rs:23-76`; scan sources `mod.rs:603-617`
- **Status**: NEW

`player_controller_system`'s Access row omits the KILL producer's acquisitions (`StoryEvent` W, `LoadedCellIndex`, `CurrentCellContext`, `CurrentExteriorContext`) reached via #5434's `raise_kill_story_event`. The scan does not list `story_events.rs` for that system. Not a live race; declare the four types and add the hop to `PARALLEL_SYSTEMS`


## Completeness Checks
- [ ] **LOCK_ORDER**: If a RwLock scope changes, TypeId-sorted acquisition is preserved
- [ ] **SIBLING**: Same pattern checked in related files / primitives
- [ ] **TESTS**: A regression test pins this specific fix

=== #5642
OPEN SCR-D5-2026-10-11-05: Force-greet pass 2 opens every in-radius greeter against a stale `open_partner` snapshot — the second open strips the first's line mid-sentence
Filed from `docs/audits/AUDIT_SCRIPTING_2026-10-11.md` (HEAD 71b86091b).

- **Severity**: LOW
- **Dimension**: Scripting Dim 5
- **Location**: `systems/forcegreet.rs:119-122, 200-208, 225-243`
- **Status**: NEW

Pass 2 opens every in-radius greeter against a stale `open_partner` snapshot. Two same-frame greeters open A then B, which strips A's line mid-sentence and spends both directives. Re-read `DialogueSurfaceState::npc` per open


## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / primitives
- [ ] **TESTS**: A regression test pins this specific fix

=== #5643
OPEN SCR-D5-2026-10-11-06: Story Manager tree build is O(g^3) per sibling group and the walk's recursion depth is bounded only by node count
Filed from `docs/audits/AUDIT_SCRIPTING_2026-10-11.md` (HEAD 71b86091b).

- **Severity**: LOW
- **Dimension**: Scripting Dim 5
- **Location**: `story_manager.rs:387-398, 440-468, 791-882`
- **Status**: NEW

**Untrusted-Input: Yes.** The tree build is O(g³) per sibling group, and `walk_siblings` recursion depth is bounded only by node count. Vanilla maxima are tiny (group 50, depth 6–7). Index members once and use an explicit stack with a depth cap


## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / primitives
- [ ] **TESTS**: A regression test pins this specific fix

=== #5644
OPEN SCR-D5-2026-10-11-07: `fold_info_into_topic` is O(N x M) per DIAL override
Filed from `docs/audits/AUDIT_SCRIPTING_2026-10-11.md` (HEAD 71b86091b).

- **Severity**: LOW
- **Dimension**: Scripting Dim 5
- **Location**: `crates/plugin/src/esm/records/index.rs` `fold_info_into_topic`
- **Status**: NEW

**Untrusted-Input: Yes.** O(N×M) per DIAL override INFO fold (#5375). No panic or OOB. Build a `FormId → position` map


## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / primitives
- [ ] **TESTS**: A regression test pins this specific fix

=== #5645
OPEN SCR-D3-2026-10-11-04: `subject_requirement` evaluates `GetRandomPercent` once "for every subject", breaking the exact-superset pre-filter contract (FO4 alias fills)
Filed from `docs/audits/AUDIT_SCRIPTING_2026-10-11.md` (HEAD 71b86091b).

- **Severity**: LOW
- **Dimension**: Scripting Dim 3
- **Location**: `condition.rs:321-349, 1140-1177`; `scene/quest_alias.rs:932-943`
- **Status**: NEW

`subject_requirement` evaluates `GetRandomPercent` once "for every subject". It is stochastic and consumes RNG state, which breaks the exact-superset pre-filter. FO4 has 21 alias CTDAs on fn 77 (Skyrim 0). Add an `is_deterministic()` and treat non-deterministic as unconstrained


## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / primitives
- [ ] **TESTS**: A regression test pins this specific fix

