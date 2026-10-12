# #5706 — CHAR-2026-10-12-D4-01: `CharacterRuleset::actor_value` re-sums the modifier layers by hand and leaves out `set_override`, the exact pattern #5508 removed from the HUD, vitals and SDK readers

**Labels**: `low`, `character`, `bug`

**Source**: `docs/audits/AUDIT_CHARACTER_2026-10-12.md` (CHAR-D4-01) · audited at HEAD 5d31fd637

- **Severity**: LOW. Unreachable by construction today: `set_override` is written only by the two #5412 sites (`edit_av` in `byroredux/src/commands/actor_value.rs`, and the SDK `SetBase` arm in `byroredux/src/extensions/commands.rs`), and both gate on `is_player_derived_pool` (`PlayerOnly` + `Absolute`). Such a stat takes the `!derives_for_any_actor` early return in `actor_value`, which uses `avs.current()` and does include the override. The hand-sum is only reached for `ActorGeneral` + `Absolute` rows. It becomes wrong the first time one of those rows is also writable through the override path (a second row for the same output, a future per-NPC `SetBase` redirect, or a loaded save carrying an override on such a key).
- **Dimension**: Population Boundary (derived-value composition)
- **Game**: fo3, fnv, fo4 (every family that builds a ruleset)
- **Location**: `crates/core/src/character/ruleset.rs` `CharacterRuleset::actor_value` (the derived branch's `layers` computation)
- **Status**: NEW (code path predates #5412; the omission was introduced when #5412 added the layer, and #5508 fixed only the HUD/vitals/SDK readers).
- **Source**: not numeric. The invariant is the repo's own: `ActorValue::undamaged()` is "the one composition of the ceiling" (`actor_values.rs`, #5508, pinned by `undamaged_includes_every_layer_but_damage`).

## Description
In the derived branch, `actor_value` returns `derived_value(..) + layers`, where `layers = value.permanent_mod + value.temporary_mod - value.damage`. The base is the formula output, so `base` is correctly left out, but the other additive layer (`set_override`) is missing because the sum is written out by hand rather than derived from the entry (`value.current() - value.base` would compose every layer). The doc comment on the same function repeats the formula: `derived_value + permanent + temporary − damage`. This is the "a reader that re-sums layers by hand is the regression" case from #5508's closing note.

## Evidence
`ruleset.rs` `actor_value`; writes of `set_override` exist only at `commands/actor_value.rs` and `extensions/commands.rs`, both behind `is_player_derived_pool`. No test builds an `ActorGeneral` + `Absolute` stat carrying `set_override` through `actor_value`.

## Impact
None today. If it becomes reachable, `GetActorValue` and `melee_damage_charal_bonus` (the two callers) would silently ignore an override that `current()`, the HUD and the SDK all honour.

## Related
#5412, #5508, #5042 (the composer's origin).

## Suggested Fix
Compute the layer term from the entry (`value.current() - value.base`), update the doc comment, and add a one-line test with `set_override` on an `ActorGeneral` row.

## Completeness Checks
- [ ] **SIBLING**: grep for other hand-summed `permanent_mod + temporary_mod` compositions outside `actor_values.rs`
- [ ] **TESTS**: A regression test pins `set_override` on an `ActorGeneral` + `Absolute` row through `actor_value`
