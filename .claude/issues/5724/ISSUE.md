# #5724 — GAME-D5-2026-10-12-03: Travel, Escort-lead and Guard resolve their `NearReference` target once and freeze the fallback when the target is not spawned yet — the #5500 race, fixed only for Eat/Sleep

**Labels**: `low`, `gameplay`, `ai`, `bug`, `game:fnv`, `game:fo3`

**Source**: `docs/audits/AUDIT_GAMEPLAY_2026-10-12.md` (GAME-D5-03) · audited at HEAD 5d31fd637

- **Severity**: LOW
- **Dimension**: NPC Spawn → AI Package Selection → Locomotion
- **Location**: `byroredux/src/systems/travel.rs` (`resolve_near_reference_target`, `resolve_destination`, and the first-sight resolve whose result is stored in `TravelState`); `byroredux/src/systems/escort.rs` (the lead-target resolve); `byroredux/src/systems/guard.rs` (`unwrap_or(home)`)
- **Status**: NEW (sibling of #5500 / GAME-D5-2026-10-09-02 item 3)
- **Trigger**: FNV/FO3 budgeted cell load (door transition under the loading cover, or boot). A Travel, Escort or Guard actor spawns before the reference its PLDT names.

## Description
#5500 stopped Eat/Sleep from caching the actor's current position when the anchor is unresolved. The same shape remains in the other walk-to-point movers. Travel resolves its destination on its first tick; if `resolve_entity_by_global_form_id` misses, it falls back to a hash-picked point within the radius of the actor and stores that in `TravelState`, which is never re-resolved. Guard falls back to the actor's spawn position, with the same freeze. The scheduler runs every frame between budgeted load slices (`app_events.rs` runs `scheduler.run`, then `step_cell_transition`; `dt` is pinned to 0 under the loading cover, but the first-sight resolve does not need `dt`), so a spawn-time package winner can resolve before its marker exists.

## Evidence
Census (FNV, hour 12, conditions ignored, so an upper bound): Travel `NearReference` winners whose target REFR is placed after the actor in the same cell: 61; target in another cell: 89 (the fallback is the documented behaviour for a non-resident target); Guard, same-cell-after: 20.

## Impact
An affected Travel actor walks to a random point near itself instead of the authored marker, or a Guard holds its spawn position instead of the authored post, for the life of the cell. Many of the counted packages are quest-gated, and the race depends on how the load is sliced, hence LOW.

## Related
#5500 / #5527 (closed, Eat/Sleep), GAME-D5-2026-10-09-02.

## Suggested Fix
For a `NearReference` target that does not resolve but whose form is not known to be non-resident, leave `TravelState` unset and retry on the `ANCHOR_RETRY_SECS` clock `eat_sleep` already uses, falling back only after the load settles.

## Completeness Checks
- [ ] **SIBLING**: Travel, Escort-lead and Guard all share the deferred-resolve behaviour
- [ ] **TESTS**: A regression test pins resolution after the target spawns later than the actor
