# REG-2026-10-12-01: `leveled_spells` still documents a consumer that shipped (#4415)

- **GitHub**: #5759 (https://github.com/matiaszanolli/ByroRedux/issues/5759)
- **Labels**: low,documentation,doc-rot,esm-plugin
- **Source**: docs/audits/AUDIT_REGRESSION_2026-10-12.md (`REG-2026-10-12-01`)

**Audit finding** `REG-2026-10-12-01` — severity LOW — from `docs/audits/AUDIT_REGRESSION_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/plugin/src/esm/records/index.rs` (`EsmIndex::leveled_spells` field doc)

## Description
The field doc says "No production reader yet: the `SPLO` -> spell-list -> MGEF runtime that consumes it is tracked by #4415". #4415 closed 2026-10-06 and its runtime reads the map: `crates/plugin/src/equip.rs:888` passes `&index.leveled_spells` into the spell-list resolver that `resolve_actor_spells` / `resolve_racial_spells` (and #5413's player stamp) use.

## Evidence
`grep -rn leveled_spells --include='*.rs'` -> `index.rs` (decl), `dispatch_container.rs` (insert), `index.rs` (category map), `equip.rs:888` (read), plus real-data tests. The stale sentence is still present at HEAD (validated).

## Impact
The doc tells a reader the leveled-spell tier is unreachable in production; it is reachable. Costs an auditor a wrong "dead map" finding.

## Suggested Fix
Replace the sentence with "read by `equip::resolve_actor_spells` (`equip.rs:888`), #4415".

## Related
#4415, #3617, #5413.

## Completeness Checks
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
