# OBL-2026-10-12-D2-01: 39% of Oblivion INFOs gate on a CTDA function outside the 24-function catalog, and evaluate it against a fixed `0.0`

- **GitHub**: #5752 (https://github.com/matiaszanolli/ByroRedux/issues/5752)
- **Labels**: low,game:oblivion,legacy-compat,dialogue,scripting,bug
- **Source**: docs/audits/AUDIT_OBLIVION_2026-10-12.md (`OBL-2026-10-12-D2-01`)

**Audit finding** `OBL-2026-10-12-D2-01` — severity LOW — from `docs/audits/AUDIT_OBLIVION_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/scripting/src/condition.rs` (`ConditionFunction::Unknown` -> `0.0`), consumed by `select_info` (`crates/scripting/src/dialogue.rs`, the `evaluate(&info.conditions, ...)` call)

## Description
`Oblivion.esm` has 19,278 INFOs; 7,533 (39%) carry at least one CTDA whose function index is not in the 24-function catalog. For the generic `GREETING` topic, 1,586 of 3,743 INFOs (42%) do. The top uncatalogued indices over all INFOs are 79 (3,774 conditions), 76 (1,751), 70 (841), 84 (716), 47 (687), 53 (655), 50 (540), 161 (284), 125 (270). They are not named: the xEdit TES4 function table is not on disk, and `Raw Function List.wiki` carries no opcode numbers. This is a documented, deliberate contract ("unknown function indices evaluate to `0.0`"); no crash, wrong-line selection only.

## Evidence
`/tmp/audit/oblivion/tools/condcmp.py`, evaluating each condition's comparison against `0.0`:

| fn | conditions | pass with 0.0 | typical comparison |
|---|---|---|---|
| 76 | 1,751 | 709 | `>= 70` x256, `< 70` x215, `>= 30` x203, `< 30` x155 |
| 79 | 3,774 | 898 | `== 1` x1,774, `== 0` x804 |
| 84 | 716 | 622 | `== 0` x617 |
| 50 | 540 | 508 | `== 0` x508 |
| 70 | 841 | 20 | `== 1` x821 |

A line gated `fn 50 == 0` passes for every NPC and a line gated `fn 70 == 1` can never fire. The first passing INFO in priority order wins, so the wrong line can win.

## Impact
Oblivion greeting selection (every activation opens `GREETING`, #5367 Phase G) is decided partly on constants. No live dialogue gate runs Oblivion (`dt1-dialogue-layers.sh` is FNV), so nothing observes it today.

## Suggested Fix
Give `select_info` the #5380 shape - decline (or sort last) an INFO whose condition list contains an uncatalogued function - and catalogue the high-count functions (79, 76, 70, 84, 50, 47, 53) from the xEdit TES4 table, source-first, before widening the catalog. Re-run `condcensus.py` after.

## Related
FO3-2026-10-12-D2-01 (also needs Oblivion fn 254 / 365 catalogued before quest-level conditions are wired), SKY-D4-2026-10-12-01 (Skyrim leg), #5380, #5406. Owner: `/audit-scripting` (condition catalog).

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
