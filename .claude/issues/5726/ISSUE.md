# #5726 — GAME-D5-2026-10-12-04: #5500's In-Cell search radius is unbounded, so the seat snap becomes an arbitrary-distance teleport

**Labels**: `low`, `gameplay`, `ai`, `bug`, `game:fnv`, `game:fo3`

**Source**: `docs/audits/AUDIT_GAMEPLAY_2026-10-12.md` (GAME-D5-04) · audited at HEAD 5d31fd637

- **Severity**: LOW
- **Dimension**: NPC Spawn → AI Package Selection → Locomotion
- **Location**: `byroredux/src/systems/eat_sleep.rs` (`EatSleepLocation::InCell(_) => f32::INFINITY`, `seat_at_marker`); `byroredux/src/systems/sandbox.rs` (`pick_nearest_seat`)
- **Status**: NEW (a consequence of the #5500 fix, `bcaf63aa8`)
- **Trigger**: FNV/FO3. An Eat or Sleep package with PLDT type 1 (In Cell); all 70 FNV In-Cell Eat/Sleep packages author radius 0.

## Description
For In Cell the walk destination is the actor's current position, so the walk leg is a no-op and the actor is "arrived" immediately. #5500 widened the seat search from 512 BU to the whole cell. `apply_seat_assignments` then snaps the root's translation and rotation to the chosen marker. The nearest free marker can be rooms away. The actor teleports there at the first tick in its schedule window, in view of the player if the player is present, and (on FO3/FNV, where every legacy marker reads as Sit, #5390) can be a chair.

## Evidence
`pick_nearest_seat`'s filter is `d2 <= r2` with `r2 = INFINITY`; no walk leg exists between the pick and the snap.

## Impact
Visible pop-in at schedule boundaries; far from the diner/sleeper's room. Nothing is mis-persisted.

## Related
#5500, #5390, GAME-D5-2026-10-09-02 item 1.

## Suggested Fix
When the pick falls outside `ARRIVE_RADIUS`, set `EatSleepState.destination` to the chosen marker and let `cached_walk_step` walk there before seating, instead of snapping.

## Completeness Checks
- [ ] **SIBLING**: Sandbox seat picking checked for the same unbounded-radius snap
- [ ] **TESTS**: A regression test pins that a far In-Cell marker is walked to, not snapped to
