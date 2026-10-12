# SKY-D4-2026-10-12-02: ROADMAP's M43 row still records the pre-#5511 Skyrim force-greet census

- **GitHub**: #5751 (https://github.com/matiaszanolli/ByroRedux/issues/5751)
- **Labels**: low,documentation,doc-rot,game:skyrim,legacy-compat
- **Source**: docs/audits/AUDIT_SKYRIM_2026-10-12.md (`SKY-D4-2026-10-12-02`)

**Audit finding** `SKY-D4-2026-10-12-02` — severity LOW — from `docs/audits/AUDIT_SKYRIM_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `ROADMAP.md` M43 "Quests & dialogue" row, the 2026-10-08 force-greet clause

## Description
The row says the Skyrim `ForceGreet` leaf has "census 5 packs / 2 authored topics / 3 generic" (still present at HEAD). `72e863d80` (#5511) found that 334 of the 339 force-greet packs are `PKCU` template instances the old count could not see: the corrected census is 339 packs (5 own-tree + 334 instances), 320 instances with their own topic, >=17 generic. `docs/engine/dialogue-trees.md` and the real-data floor were updated in that commit; ROADMAP, the suite's status floor, was not.

## Evidence
`crates/plugin/tests/parse_real_esm.rs` `force_greet_skyrim_tree_dialect_floor` (passes: >=339 / >=17 / >=320) vs the ROADMAP M43 row.

## Impact
The status document reports 5 of the 339 force-greet packs the runtime now recognises.

## Suggested Fix
Replace the clause with the #5511 census and point at the floor test.

## Related
#5511, `docs/engine/dialogue-trees.md`.

## Completeness Checks
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
