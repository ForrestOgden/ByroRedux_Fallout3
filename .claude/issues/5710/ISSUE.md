# #5710 — CHAR-2026-10-12-D5-02: The FO3/FNV capture's "NPC stat storage" note was not updated for #5338

**Labels**: `low`, `character`, `documentation`, `doc-rot`, `game:fo3`, `game:fnv`

**Source**: `docs/audits/AUDIT_CHARACTER_2026-10-12.md` (CHAR-D5-02) · audited at HEAD 5d31fd637

- **Severity**: LOW (doc rot — code ahead of capture)
- **Dimension**: Coverage & Doctrine
- **Game**: fo3, fnv
- **Location**: `docs/engine/charal-fnv-fo3-ruleset.md:473-480` ("NPC stat storage — NOTE (distinct from FO4)")
- **Status**: NEW

## Description
- The note still says auto-calc-OFF NPCs "store explicit skill/SPECIAL values in their `NPC_` record (DNAM-era layout)" and that the "Full byte layout" is "a follow-up when wiring FO3/FNV NPC derived stats".
- #5338 has since wired the SPECIAL half: `NPC_ DATA` is `i32 health` + `7 × u8 SPECIAL` (11 or 25 bytes), decoded into `NpcRecord::data_attributes` and used when ACBS bit 4 is clear. The skill half (the `DNAM` block) is still unparsed, and skills for these NPCs derive from the authored SPECIAL through the GECK formula instead of the authored per-skill values.
- `actor_value_derive.rs`'s module docs state exactly this; the capture, which is meant to be the authority for what is built, does not.
- `docs/engine/charal.md` section 8 item 2 ("FNV/FO3 population — shipped (#1663)") also does not mention the authored-SPECIAL branch.

## Evidence
The `DATA` arm in `crates/plugin/src/esm/records/actor/npc.rs`; `derive_autocalc_actor_values` in `actor_value_derive.rs`; `fnv_authored_data_special_reproduces_the_wiki_hit_points` (ignored real-master pin).

## Impact
A reader of the capture sees "unwired follow-up" for something that shipped, and no pointer to the one remaining gap (authored skills).

## Related
#5338 (closed). Audit-esm Dim 4 owns the `DNAM` skill-half parse.

## Suggested Fix
Replace the "follow-up" sentence with the shipped/unparsed split and the ACBS bit-4 discriminator; add the authored-SPECIAL branch to `charal.md` section 8 item 2.

## Completeness Checks
- [ ] **SIBLING**: `charal.md` section 8 and any other FO3/FNV population note updated consistently
