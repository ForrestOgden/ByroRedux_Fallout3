# SKY-D4-2026-10-12-01: Skyrim's generic Hello greeting can only ever select its three unconditioned lines, because 57 of its 60 INFOs are gated on `GetIsVoiceType` (fn 426), which is outside the 24-function condition catalog

- **GitHub**: #5750 (https://github.com/matiaszanolli/ByroRedux/issues/5750)
- **Labels**: medium,game:skyrim,legacy-compat,dialogue,scripting,bug
- **Source**: docs/audits/AUDIT_SKYRIM_2026-10-12.md (`SKY-D4-2026-10-12-01`)

**Audit finding** `SKY-D4-2026-10-12-01` — severity MEDIUM — from `docs/audits/AUDIT_SKYRIM_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/scripting/src/condition.rs` (`ConditionFunction::Unknown` evaluates to 0.0; the catalog table); `crates/scripting/src/dialogue.rs` (`select_info`); `byroredux/src/systems/npc_dialogue.rs` (`generic_greeting_record`, `open_conversation` fallback)

## Description
`DialogueGenericHello` (`0x000142B5`, owner quest `DialogueGeneric`) holds 60 INFOs. All 60 are Random-flagged and `previous_info == 0`. 57 of them carry CTDA conditions, and every one of those includes `GetIsVoiceType` (fn 426, 124 conditions in the topic) against a `VTYP` or a voice-type `FLST` (`DefaultNPCVoiceTypes`, `GuardVoiceTypes`, `VoicesUnique`, `FemaleNord`). Some add fn 69 / 70 / 74 / 130 / 131 terms (only 69 and 74 are catalogued). Function 426 is not in the catalog (validated at publish time: no 426 / `GetIsVoiceType` in `condition.rs`), so `evaluate` maps it to 0.0 and `fn426 == 1` can never hold. The 57 gated INFOs never pass; only the three INFOs with no CTDA (`0x00087940`, `0x000142C0`, `0x00087947`) survive.

Scope beyond this topic: on `Skyrim.esm`, 13,154 of 31,465 INFOs reference at least one uncatalogued function; fn 426 alone appears in 4,222 INFOs (most common uncatalogued: fn 566 in 4,388, fn 426 in 4,222, fn 629 in 2,626). The Hello topic is the one place where the gap is the whole selection.

## Evidence
The engine's own `select_first_info` over the shipped master (probe `/mnt/data/tmp/skyrim-audit-probe/src/bin/greet.rs`: parse `Skyrim.esm`, install `DialogueQuestPriorities` and the 330 Start-Game-Enabled quests, select the Hello topic for every non-creature `NPC_` base):
```
started 330; greeting 000142B5 DialogueGenericHello infos=60
NPC_ bases (non-creature) 5118; no passing INFO 0
  1738 00087947  ctda=0
  1702 000142C0  ctda=0
  1678 00087940  ctda=0
hello INFO condition fn histogram: {69: 6, 70: 6, 74: 6, 130: 15, 131: 4, 426: 124}
```
All 5,118 bases split across the three unconditioned INFOs, so a female Nord, an Orc guard and a child receive the same three lines.

## Impact
Every Skyrim NPC that takes the quest-less fallback speaks one of three voice-agnostic lines; the 57 voice-, race- and sex-specific Hello lines (most of the topic) are unreachable. Skyrim `.fuz` voice is V2 scope, so today the visible effect is the subtitle text; once voice lands, a voice-agnostic line cannot resolve a per-voice-type file. Existing guards cannot see it: `dt2-skyrim-forcegreet.sh` gates the force-greet topic and the activation-less open, not which INFO wins; there is no Skyrim equivalent of the FO3/FNV `dialogue_greeting_and_forcegreet_*_floor` winner assertion.

## Suggested Fix
1. Catalogue `GetIsVoiceType` (Skyrim 426: parameter is a `VTYP` or `FLST`, true when the subject's `NPC_` `VTCK` voice type is in it) next to `GetIsRace`; the voice-type lookup already exists for the dialogue voice resolver (`VTCK` -> indexed `VTYP`).
2. Add a Skyrim real-data assertion beside `force_greet_skyrim_tree_dialect_floor`: a bare speaker and a named speaker must not both resolve to the same three INFOs, and the winning INFO for a known voice type must carry that voice type in its CTDA.

## Related
OBL-2026-10-12-D2-01 (same mechanism, Oblivion), FO3-2026-10-12-D2-01 (quest-level conditions ignored; the `DialogueGeneric` quest's own conditions are fn 700 and fn 426, so that fix must catalogue 426 and 700 on Skyrim first), #5526, #5380.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
