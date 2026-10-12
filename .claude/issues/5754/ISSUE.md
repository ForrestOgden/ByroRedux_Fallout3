# OBL-2026-10-12-D3-02: the `humanoid_skeleton_path` doc block sits on `stamp_faction_ranks`, and its "Oblivion is not yet a target for NPC spawning" sentence is false

- **GitHub**: #5754 (https://github.com/matiaszanolli/ByroRedux/issues/5754)
- **Labels**: low,documentation,doc-rot,game:oblivion,legacy-compat
- **Source**: docs/audits/AUDIT_OBLIVION_2026-10-12.md (`OBL-2026-10-12-D3-02`)

**Audit finding** `OBL-2026-10-12-D3-02` — severity LOW — from `docs/audits/AUDIT_OBLIVION_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `byroredux/src/npc_spawn.rs` (one contiguous `///` block ending at `fn stamp_faction_ranks`; the orphaned fn is `pub fn humanoid_skeleton_path`, no doc)

## Description
Lines 44-77 are the skeleton path table ("Path inside the meshes archive for the default humanoid skeleton... FNV / FO3 ship a single `meshes\characters\_male\skeleton.nif`... Skyrim... FO4..."), ending "Oblivion is not yet a target for NPC spawning (M41.0 closes on FNV first); the path is the same as FNV's by convention." The next line starts "/// Stamp a [`FactionRanks`] component..." with no blank line, so rustdoc merges the two (validated at publish time). Oblivion has been a spawn target since #2567 and #5487, and its humanoid arm is the busiest in `humanoid_body_paths`. Introduced when `f73c6fd7b` (2026-06-24) added `stamp_faction_ranks` directly under the doc block. Same "helper inserted between a doc comment and its fn" class as REN-D6-2026-10-12-01, NIFAL-D9-2026-10-12-01 and TOOL-D1-2026-10-12-01, but a different site.

## Evidence
`byroredux/src/npc_spawn.rs` doc block immediately above `fn stamp_faction_ranks`; `pub fn humanoid_skeleton_path(game: GameKind)` carries no doc.

## Impact
Documentation only; rustdoc shows a skeleton-path table on a faction-stamping fn.

## Suggested Fix
Move the skeleton-path block above `humanoid_skeleton_path`, and replace the Oblivion sentence with the real statement (`skeleton.nif`, shared with FO3/FNV; the beast rig is OBL-2026-10-12-D3-01).

## Related
REN-D6-2026-10-12-01, NIFAL-D9-2026-10-12-01, TOOL-D1-2026-10-12-01.

## Completeness Checks
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
