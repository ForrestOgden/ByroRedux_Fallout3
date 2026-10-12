# #5728 — GAME-D4-2026-10-12-01: Console `setav`/`modav` on Health can leave an actor at Health ≤ 0 without `Dead`, KILL or reconciliation

**Labels**: `low`, `gameplay`, `combat`, `bug`

**Source**: `docs/audits/AUDIT_GAMEPLAY_2026-10-12.md` (GAME-D4-01) · audited at HEAD 5d31fd637

- **Severity**: LOW
- **Dimension**: Health writers outside `combat_damage_system` (#4702's rule)
- **Location**: `byroredux/src/commands/actor_value.rs` — `edit_av`
- **Status**: NEW
- **Trigger**: `setav`/`modav` with a Health AV and a value that drives the composed current to ≤ 0, on an NPC or the player.

## Description
#4702 made the SDK AV batch (`commit_actor_value_deaths`, `byroredux/src/extensions/commands.rs`) apply the alive→dead transition, with the stated rule that every Health writer outside `combat_damage_system` owes it. The console's `edit_av` calls `set_base`, `set_override` or `mod_permanent` and returns, with no death check. The actor stays alive at ≤ 0 HP: its AI keeps running, `consume_item` refuses it (`values.current(health) <= 0.0` returns `Unavailable`), and `restoration_system` clears its timers, so it cannot be healed by consumables either. A later hit kills it.

## Impact
Developer console only; no shipped gameplay path. Recorded because it breaks the "single owner of the alive→dead edge" invariant.

## Related
#4702 (closed), `extensions/commands.rs` (`commit_actor_value_deaths`).

## Suggested Fix
Share `commit_actor_value_deaths` with `edit_av` (it already takes the committed entity list).

## Completeness Checks
- [ ] **SIBLING**: Any other non-combat Health writer (console, SDK, scripts) routes through the shared death commit
- [ ] **TESTS**: A regression test pins that `setav Health 0` raises Dead + KILL
