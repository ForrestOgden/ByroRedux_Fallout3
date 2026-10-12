# FNV-2026-10-12-D6-01: `m42-eat-sleep.sh`'s seat leg cannot pass at HEAD — the settler's Sleep package is In Cell in `GSHouseInterior02`, not in the saloon, and #5391's In-Cell arm idles him

- **GitHub**: #5748 (https://github.com/matiaszanolli/ByroRedux/issues/5748)
- **Labels**: low,game:fnv,legacy-compat,gameplay,ai,test-gap,bug
- **Source**: docs/audits/AUDIT_FNV_2026-10-12.md (`FNV-2026-10-12-D6-01`)

**Audit finding** `FNV-2026-10-12-D6-01` — severity LOW — from `docs/audits/AUDIT_FNV_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `docs/smoke-tests/m42-eat-sleep.sh:1-16` (header premise) and `:115` (`wait_log ... "[m42] sleep npc=$entity"`); stale claims in `CLAUDE.md` ("the walk-then-seat runtime must seat HIM at the bed") and `docs/smoke-tests/README.md`; behaviour source `byroredux/src/systems/eat_sleep.rs` (`resolve_anchor` `InCell` arm, `cell_is_resident`)

## Description
The header says the settler's "PLDT bed reference lives in an unloaded cell, so the documented fallback seats him at the nearest sleep/sit marker the saloon offers". That was true before #5391, when only `NearReference` reached the runtime and an unresolved reference fell back to the actor's own position. FalloutNV.esm authors `GSSettler04SleepPackage22x8` with `PLDT` type 1 (**In Cell**) and cell `0x001055E1` = `GSHouseInterior02`. The gate boots `GSProspectorSaloonInterior` (`0x00106185`). Since #5391 the In-Cell arm returns `cell_is_resident(...).then_some(current)`, which is `None` here (validated at publish time: the arm is still `cell_is_resident(world, cell_form_id).then_some(current)`). The actor idles on `ANCHOR_RETRY_SECS`, never walks and never seats; the log line the gate waits for (`eat_sleep.rs`, info level) is only emitted after a seat. No other saloon NPC can stand in for him.

## Evidence
Probe `eatsleep.py` / `eatsleep2.py` over FalloutNV.esm. `GSSettlerCM` (ACHR `0x104F08`, the gate's target) packages in order: `GoodspringsFleePackage` (10), `GSSettlerAmbushPackage` (9), `GSSettler04SleepPackage22x8` (4; In Cell -> `GSHouseInterior02`), `GoodspringsSettlerSaloonPackage` (6; NearReference in the saloon). Other saloon actors: `GSSunnySmiles` Sleep In Cell -> `GSHouseInterior01`; `GSTrudy` and `GSJoeCobb` both Initially Disabled with In-Cell Sleep packages in other interiors. Population-wide 67 placed actors have a Sleep package and 14 an Eat package whose In-Cell target is a different cell. Unit tests do not cover it: `legacy_sleep_actor_seats_at_the_sit_fallback` uses `NearCurrentLocation`. Derived from code and shipped data, not a live run (no engine launch in the audit).

## Impact
A live run of the gate times out at the seat wait and exits 1, and with no SKIP path it cannot serve as a regression check for the Eat/Sleep seating runtime that #5449, #5452 and #5500 changed. The shipped runtime behaves as documented (In-Cell sleepers outside their cell stand idle, the v0 contract); only the gate and its three documentation claims are wrong.

## Suggested Fix
Point the gate at an actor whose Sleep package anchors in its own placed cell, or add the matching cell to the harness (candidates: `NellisMedHut` `InjuredBoomer*` sleepers, or a same-cell `NearReference` sleeper). Alternatively assert the idle (`entities(EatSleepState)` empty, `entities(Seated)` without the settler) and gate seating on a second NPC. Update the header, CLAUDE.md and README wording either way.

## Related
#5391, #5427, #5500 / #5527, GAME-D5-2026-10-12-04, FNV-2026-10-08-D6-01.

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
