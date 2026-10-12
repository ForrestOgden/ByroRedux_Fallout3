**HEAD**: 5d31fd637 · **Baseline**: AUDIT_TOOLING_2026-10-08.md (HEAD 00f580e09) · **Audited**: Dim 1 (Debug Trust Boundary), Dim 2 (Protocol & Registry), Dim 3 (SDK Contract), Dim 4 (Boot Handoff & Persistence), Dim 5 (Install Detection), Dim 6 (Tool CLIs) · **Unchanged since baseline (skimmed)**: Dim 3 SDK crate (`crates/sdk` and `studio_host.rs` have no commits; only the extension adapter changed), Dim 4 durability writers (`atomic_file.rs`, `boot-request`, `settings-io` have no commits), `tools/byro-dbg` and `debug-protocol` (no commits)

# Tooling and Host-Contract Audit - 2026-10-12

Scope is `git log 00f580e09..HEAD` (345 commits) filtered by each dimension's `Paths:`.

| Dim | Commits that touched tooling paths |
|---|---|
| 1 | `224180508` (#5149 loopback pin), `72a63616b` (#5147 unique default names), `d8a6e58bf` (#5598 flag readers), `99cd9fd6c` (`debug_load.rs` call args) |
| 2 | `9bb8bc475` (+`CreatureActor`), `42aab4c09` (+`EditorPlacement`, EatBehavior field rename), `f2d0b6ea0` (#5417 count guard), `bfa538beb` |
| 3 | `627149b62` (#5508 projection folds `set_override`), `a614eb273` (#5412 `SetBase` -> override layer), `c85d50356` (#5434 SDK death raises KILL), `d8a6e58bf` (`flag_values` removed) |
| 4 | `61ad29da3` (#5476, finishes #5294), `d8a6e58bf` |
| 5 | `4ef386a1e` (#5433 FO3 sounds), `bb3bdf9f6` (Oblivion sounds), `61ad29da3` |
| 6 | `1ca54a3ac` (read-only output refusal, disk-counter refresh), dependency refreshes |

Method: one auditor, no sub-agents, no engine launch. Notes in `/tmp/audit/tooling/`.

## Test baseline

`cargo test -j 8 --no-fail-fast`, rustc 1.96.0. All green.

| Crate | Tests | Delta vs 10-08 |
|---|---|---|
| `byroredux-sdk` | 104 | = |
| `byroredux-debug-server` | 24 | +2 net (the red registry-count test is green again; +`the_listener_binds_loopback_only`, +`same_second_bare_screenshots_take_free_suffixes`) |
| `byroredux-debug-protocol` | 7 | = |
| `byroredux-debug-ui` | 19 | = |
| `byroredux-boot-request` | 17 | = |
| `byroredux-settings-io` | 14 | = |
| `byroredux-game-detect` | 59 | +3 (#5433 FO3 and Oblivion profile pins, #5476 `launch_defaults_follow_the_selected_user_file`) |
| `byro-launcher` | 25 + 1 ignored (Vulkan) | = (the #5476 case extends `parse_args` test) |
| `byro-detect` | 4 | +1 (`a_relative_profiles_path_comes_back_absolute`) |
| `byro-texture-upscale` | 18 | = |
| `byro-dbg` | 2 | = |

## Executive Summary

**3 findings, all NEW: 0 CRITICAL, 0 HIGH, 0 MEDIUM, 3 LOW.**

The audit's open items from 10-08 are all closed or confirmed fixed at HEAD:

| Issue | State at HEAD |
|---|---|
| #5149 (no loopback pin) | Fixed (`224180508`). `the_listener_binds_loopback_only` binds port 0 through `spawn` and asserts `local_addr().ip().is_loopback()`. |
| #5147 (default output names collide under `create_new`) | Fixed (`72a63616b`). `unique_default_dump_name_in` and `unique_screenshot_name_in` walk to the first free suffix; `create_new` still guards the race. Both have tests. |
| #5476 (TOOL-D4-2026-10-08-01) | Fixed (`61ad29da3`). `[defaults]` follows `selected_user_path()` (`ordered_config_paths_with`); the launcher and `byro-detect` build their registry through `load_with_user_path(Some(&profiles_path))`; a relative `--profiles` is absolutised in both. All three halves have tests. |
| #5417, #5508, #5412, #5598 | Verified at HEAD (count guard now needs two occurrences; the SDK projection folds `set_override` into `base`; `SetBase` on a derived pool writes `set_override`; the plain flag readers go through `cli_args`). |
| #5290 (absolute-path guidance in `debug-cli.md`/byro-dbg) | **Existing, still open, unchanged** (`debug-cli.md` only changed in the two count lines). |
| #5327 (`--no-default-features` lane warnings) | Existing, open, not re-examined. |

Areas swept with no new defect:
- Debug trust boundary: bind literal, caps, path confinement. No new path-taking request or console command was added (101 `registry.register(` calls, same as 10-08). `byroredux/src/commands/` diffs contain no new `fs::` use outside the two #5147 helpers.
- Registry completeness: 71 production `register_component::<` calls plus `register_faction_reputation` = **72**, matching both `debug-cli.md` count sites. `every_inspectable_component_is_registered` passes; no `inspect` derive exists outside `crates/core` (only a source-scan test file mentions the string).
- SDK: `byroredux-sdk` depends on core (only `math::Vec3` used at `studio.rs:3`), `semver`, `serde`, `thiserror`. All 28 `*_CAPABILITY` constants have a non-test call site. `commit_actor_value_deaths` now raises KILL (#5434) for SDK-driven deaths, covered by `deferred_actor_value_batch_that_zeroes_health_kills_the_actor`.
- Persistence: `BootRequest::save` and `save_to_path` still go through `atomic_write`; no `fs::write` in any production writer.
- Install detection: FO3 (`Fallout - Sound.bsa`, `Voices.bsa`, `MenuVoices.bsa`) and Oblivion (`Oblivion - Sounds.bsa`) sound archives named by the profiles exist in the local installs; the `Sounds` validation check stays `Severity::Warn`.
- texture-upscale: the sysinfo 0.39 `is_read_only` flag *is* populated by the `with_storage()` refresh the preflight uses (`unix/linux/disk.rs`, `first` refresh), so the new read-only refusal is live, not a silent no-op.

Cross-audit: `SAFE-D4-2026-10-12-01` (unbounded `CStr::from_ptr` at `tools/byro-launcher/src/preflight.rs:230,237`) was reported by `/audit-safety` this suite. Existing, not re-filed.

## Findings

### TOOL-D1-2026-10-12-01: Two #5147/egui-0.36 helpers were inserted between a doc comment and its function, so `write_screenshot` and `DebugUiState::set_visible` lost their docs
- **Severity**: LOW
- **Dimension**: Debug Trust Boundary (and the debug-ui panel host)
- **Exposure**: developers only (rustdoc and reader confusion)
- **Location**: `crates/debug-server/src/system.rs:223-243` (`write_screenshot`, `72a63616b`); `crates/debug-ui/src/lib.rs:229-248` (`set_visible`, `500fcb8ca` "egui: discard dropped frames")
- **Status**: NEW (same class as REN-D6-2026-10-12-01 and NIFAL-D9-2026-10-12-01, different files)
- **Description**: Both commits added a new function directly under an existing `///` block.
  - `system.rs`: the original docs "Save debug screenshots under the working directory's `screenshots/` folder. Only a single relative filename is accepted ..." now sit on top of the #5147 text and attach to `unique_screenshot_name`, which does not confine anything. `write_screenshot`, the function that actually enforces the filename-only rule and the `starts_with(&cwd)` check, has no doc.
  - `lib.rs`: "Show or hide the overlay explicitly (scripted captures)." now documents `discard_last_output`. The public `set_visible` has none.
- **Evidence**: `sed -n 223,230p crates/debug-server/src/system.rs` shows five consecutive `///` lines followed by `fn unique_screenshot_name`; `sed -n 229,247p crates/debug-ui/src/lib.rs` shows the `set_visible` doc followed directly by "Drop a stashed `FullOutput` ..." and `fn discard_last_output`.
- **Impact**: rustdoc for `set_visible` is empty and `discard_last_output` (private) carries a public-API sentence; the security-relevant confinement contract of `write_screenshot` is no longer attached to it. No behaviour change.
- **Related**: #5147, #5165
- **Suggested Fix**: Move each original doc block back above its function and give the helper only its own comment.

### TOOL-D2-2026-10-12-01: `debug-cli.md` "Currently registered (72 components)" table lists 67 rows; the five newest registrations are missing
- **Severity**: LOW
- **Dimension**: Protocol & Registry (doc rot)
- **Exposure**: developers only
- **Location**: `docs/engine/debug-cli.md:186-272` (table); `:124` and `:370` (default-name wording)
- **Status**: NEW (the count guard #5417 only checks the number, not the rows)
- **Description**: The heading and the transcript footer were bumped to 72, but the table below has 67 rows. Missing: `EatBehavior`, `SleepBehavior`, `EatSleepState` (registered since M42 Eat/Sleep), `EditorPlacement` (`42aab4c09`, field `translation`) and `CreatureActor` (`9bb8bc475`, no fields). The M42 trio was already absent when the count read 67 -> 70. The table also lists `FactionReputation`, which is registered by the hand-built `register_faction_reputation` rather than `register_component::<`; that is fine, but it is why a grep-based comparison is off by one. Separately, #5147 changed the default names (`tex_dump.png`, `tex_dump_2.png`, ...; `screenshot_<secs>.png`, `screenshot_<secs>_2.png`, ...), but `:124` and `:370` still describe a single fixed name.
- **Evidence**: python diff of `register_component::<T>(registry, "Name"` names in `registration.rs` production text against the first-column names in the doc table gives `['CreatureActor', 'EatBehavior', 'EatSleepState', 'EditorPlacement', 'SleepBehavior']`.
- **Impact**: someone looking up which fields `set_field` accepts on an Eat/Sleep actor or the new markers finds nothing; the heading promises a complete list.
- **Related**: #5417, #5147
- **Suggested Fix**: Add the five rows (field lists from `registration.rs`: `EatBehavior`/`SleepBehavior` -> radius, location, form_id; `EatSleepState` -> destination; `EditorPlacement` -> translation; `CreatureActor` -> none) and mention the `_N` suffix at `:124` and `:370`. Extending `debug_cli_component_counts_match_the_registry` to compare names, not just the count, would stop the next drift.

### TOOL-D6-2026-10-12-01: The new read-only output/scratch refusal in texture-upscale has no test
- **Severity**: LOW
- **Dimension**: Tool CLIs (test gap)
- **Exposure**: developers only
- **Location**: `tools/texture-upscale/src/space.rs:36-60` (`ensure_output_and_scratch_space`), added by `1ca54a3ac`
- **Status**: NEW
- **Description**: The commit makes the pre-flight bail with "`output disk <mount> is read-only`" before an hours-long upscale pass. The decision is taken inside `ensure_output_and_scratch_space`, which reads real disks through `volume_for`, so the existing tests (`insufficient_space_is_rejected_before_writes` exercises only `ensure_available`) never reach it. Nothing pins that the check covers both roles, that the input volume is deliberately exempt, or that the message names the role.
- **Evidence**: `grep -rn 'read_only\|read-only' tools/texture-upscale/src` hits only the implementation (`space.rs:11,44-50,116`).
- **Impact**: a refactor of `VolumeSpace` or of the loop would silently drop the refusal; the user then loses the upscale pass at the first write. The behaviour is correct today (sysinfo populates `is_read_only` on the `with_storage()` refresh the code uses).
- **Related**: #4761, #4762
- **Suggested Fix**: Split the loop into a pure `fn refuse_read_only(output: &VolumeSpace, scratch: &VolumeSpace) -> Result<()>` and unit-test it with hand-built `VolumeSpace` values for output, scratch and neither.

## Dimension notes

- **Dim 1**: bind is still the literal loopback pair and is now test-pinned. Caps unchanged (64 queued, 8 clients, 300 s read, 30 s/35 s response pair, 16 MB frame, walk depth 64). The `tex.dump` default-name helper is relative to `texture-dumps/` and `safe_tex_dump_path` still confines every name, default or explicit; `unique_*` use `Path::exists`, which follows symlinks, but `create_new` (no overwrite, no follow) is the real guard. `release_debug_server_requires_explicit_opt_in` and `debug_server_allowed` in `main.rs` are untouched. #5327 (dead `debug_server_allowed` without the feature) remains open.
- **Dim 2**: no protocol or `byro-dbg` commits. `EatBehavior`/`SleepBehavior` registered field names were corrected from `target_form_id` to `location` to match the struct (`42aab4c09`). Finding D2-01.
- **Dim 3**: `capture.rs` folds `base + set_override` into the SDK `ActorValueState.base` because the SDK type has no override slot. This is the documented design and has a round-trip test (`sdk_set_base_on_player_derived_pool_routes_into_the_override_layer` reads the projection back). `apply_pending_actor_value_writes` routes player-derived pools to `set_override`. No SDK crate change.
- **Dim 4**: precedence documented in `launcher.md` is intact; `ordered_config_paths_with` takes an injected path (test seam) and `load_launch_defaults` passes `selected_user_path()`. No private `~/.byroredux/profiles.toml` read remains: the only production `join(".byroredux")` is `user_profiles_path` (`profiles.rs:430`). The launcher moved to eframe's `logic`/`ui` split and parents the rfd dialog to its window; `cargo tree -p byro-launcher -i wgpu` was not re-run (no Cargo change to the dependency set beyond the egui/rfd bump already covered by the 10-10 dependency refresh).
- **Dim 5**: `is_bare_install_dir`, the 1 MiB `esm_opens` probe and the `BYROREDUX_SKYRIM_DATA` fallback are unchanged. Two new profile pins (`fo3_profile_declares_sound_and_voice_archives`, `oblivion_profile_declares_its_sounds_archive`) silently pass when `assets/debug_profiles.toml` is not found relative to the crate (`let Ok(text) = ... else { return; }`); that is the established convention for this file's tests.
- **Dim 6**: `image` features narrowed to `bmp,dds,jpeg,tga` plus the workspace `png`; `accepted_source_extension` still matches. Finding D6-01.

## Exposure Matrix

| Surface | Reachable by | Authenticated | Size-capped | Persistence atomic |
|---|---|---|---|---|
| Debug TCP console (`127.0.0.1:9876`; 101 commands + `SetField`) | any local process on a debug build, or release with `BYRO_DEBUG_SERVER=1`; loopback pinned by test | no | yes (frame 16 MB, 64 queued, 8 clients, depth 64) | n/a |
| `Screenshot` / `tex.dump` file writes | same | no | PNG only; filename-only under `screenshots/` / `texture-dumps/`; `create_new`, unique default names | no (single `create_new` write, removed on write error) |
| `LoadNif` / `LoadInteriorCell` / `LoadExteriorCell` | same | no | paths confined to startup roots | n/a |
| SDK extensions (`--extension`, wasm) | modder with a manifest and grants | capability grants | `ComponentStoreLimits` | n/a |
| `boot.toml` handoff | launcher user | no (versioned) | n/a | yes (`atomic_write`) |
| `settings.toml` | launcher + engine | no (versioned, baseline-merge) | n/a | yes |
| `profiles.toml` `[roots]` merge | `byro-detect --write`, launcher | no | n/a | yes (`atomic_write`, formatting-preserving) |
| texture-upscale manifest | developer | manifest is trusted executable input | disk-space and overflow checks | writes new PNGs, `--overwrite` required to replace |

## Next step

`/audit-publish docs/audits/AUDIT_TOOLING_2026-10-12.md` (labels: `tech-debt` for debug-server/launcher/tools, `doc-rot` for D1-01 and D2-01, `test-gap` for D6-01).
