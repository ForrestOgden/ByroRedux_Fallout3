# TD9-2026-10-11-03: No-panic-only / misnamed tests that assert nothing about the behaviour in their names

URL: https://github.com/matiaszanolli/ByroRedux/issues/5626
Labels: low, tech-debt, test-gap, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

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
