# FO3-2026-10-12-D2-01: Quest-level dialogue conditions (`QustRecord.dialogue_conditions`) are parsed and never evaluated; with quest-priority ordering, the identity-less INFOs of high-priority quests answer every other speaker's greeting

- **GitHub**: #5734 (https://github.com/matiaszanolli/ByroRedux/issues/5734)
- **Labels**: high,game:fo3,game:fnv,legacy-compat,dialogue,quests,scripting,bug
- **Source**: docs/audits/AUDIT_FO3_2026-10-12.md (`FO3-2026-10-12-D2-01`)

**Audit finding** `FO3-2026-10-12-D2-01` — severity HIGH — from `docs/audits/AUDIT_FO3_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/plugin/src/esm/records/misc/quest.rs` (`dialogue_conditions` field and its `push_ctda`; no consumer); `crates/scripting/src/dialogue.rs` (`select_info`: orders by `DialogueQuestPriorities`, then evaluates only `info.conditions`); `byroredux/src/systems/npc_dialogue.rs` (`open_conversation` -> `generic_greeting_record` -> `select_on_topic`); `byroredux/src/asset_provider/script.rs` (priorities installed unconditionally; no quest-condition resource)

## Description
**Scope note: this defect is not FO3-only.** The mechanism (quest-level CTDA list parsed, never evaluated) is shared across games. FO3 and FNV are where it is HIGH (generic-GREETING hijack, measured below). It also reaches Oblivion (339 of 390 QUST carry a quest-level list), Skyrim (bounded: 132 of 1,811 quests; a topic has a single owner quest so priority ordering cannot pre-empt) and FO4 (latent: 96 of 1,336 QUSTs, 4,266 of 8,661 Topic INFOs; no `GREETING` DIAL so no hijack). Filed once, here.

The GECK defines the rule: "Quest Conditions: Any conditions here are considered to apply to all dialogue in the quest. The quest conditions are checked first; only if those are true are the conditions on the infos evaluated" (`geck-uesp-wiki/(main)/Quest Data Tab.wiki`). FO3/FNV author the speaker identity of whole quests there, not on the INFOs:
- `WaterBeggars` (priority 75, Start Game Enabled, no script): quest condition `GetInFaction` (fn 71). Its `GREETING` INFOs: 3 with no CTDA at all, 5 with only `GetActorValue` (fn 14), 1 with only `GetDistance` (fn 50).
- `DLC01DialogDowntown` (Pitt, priority 50): 13 OR'd fn 427 (`GetIsVoiceType`) quest conditions; its "Go away." has no INFO CTDA.
- `FFHitSquadQuest` (50): `GetInFaction`, 4 unconditioned INFOs. `Doctors` (75): 3 quest conditions, 3 INFOs with only fn 14.
- 31 of the 100 `Fallout3.esm` quests that own `GREETING` INFOs carry quest-level conditions (894 of the 2,542 master INFOs sit under them).

`select_info` sorts every INFO of the topic by quest priority (descending, file order in a tie, #5271) and takes the first that passes its own conditions. With the quest list unevaluated, an unconditioned INFO of a priority-75 quest passes for everyone and out-ranks every priority-<=60 speaker-specific greeting (main-quest, companion and town quests are 50-61). Validated at publish time: `grep dialogue_conditions` still finds only the field, the push and one parse test.

## Evidence
The engine's own code over the shipped data (`/tmp/audit/fo3/greetprobe`: parse `Fallout3.esm`, start every Start-Game-Enabled quest as `quest_startup_system` does (141), install `DialogueQuestPriorities`, then `select_first_info(GREETING, world, npc, player)` with the speaker's `SceneAliasCandidate.base_form_id` set to the NPC base):
```
quests started=141  GREETING form=000000C8 infos=2542
        <bare>: INFO 000C6E7E quest=WaterBeggars prio=75 "Please... do you have any water? I'm so thirsty..."
    MoiraBrown: INFO 000C6E7D quest=WaterBeggars prio=75 "Can you help me? I need water... purified water... please..."
      ThreeDog: INFO 000C6E7F quest=WaterBeggars prio=75 "Please! Please help me... I need water! I'm dying!"
MegatonSettler01: INFO 000C6E7D quest=WaterBeggars prio=75 ...
         Amata: INFO 000C6E7C quest=WaterBeggars prio=75 "Oh, thank God. Someone new! I need water! Please! Water!"
all 1647 NPC_ bases (non-creature): 1647 x WaterBeggars
```
Same probe on `FalloutNV.esm`: 3,788 of 3,816 `NPC_` bases select `vDialogueEDE` (priority 75; its INFO `0x001579D7` "{Curious Beep}<Makes beeping sounds>" has zero INFO CTDAs; the ED-E identity lives in 3 OR'd `GetIsID` quest conditions). The 28 exceptions are speakers whose own quest outranks it (`VCG01` is 76, so Doc Mitchell works, which is why `dt1-dialogue-layers.sh` passes). Caveat: bare speakers (no factions / actor values); static + engine `select_first_info` probe, not a live run.

Other-game reach (cross-checks from the Oblivion / Skyrim / FO4 audits): Oblivion 339/390 QUST carry quest-level CTDA (`fn 254 == 1` on 308 quests, `fn 365` split on SE); Skyrim `DialogueGeneric` quest conditions are fn 700 and fn 426; FO4 companion quests use fn 566 (21 quests), 71, 426, 69, 72.

## Impact
Activating nearly any FO3 NPC without a >75-priority greeting speaks a water-begging line; on FNV the same defect gives ED-E beeps. With `ThePitt.esm` loaded, the Pitt's unconditioned "Go away." is out-ranked by `WaterBeggars` today and would surface for every speaker below priority 50 as soon as `WaterBeggars` is gated, so the Pitt's 13-voice-type quest list must be honoured in the same change. The voice resolver composes the name from the selected INFO's quest, so a non-matching voice type resolves nothing; conversation-state side effects (`OnBegin` fragments, Goodbye close, `note_info_spoken`) run for the wrong line. Existing guards cannot see it: `dt1-dialogue-layers.sh` gates Doc Mitchell (priority 76, above the hijackers); the FO3 floor test counts INFOs and PACKs, not the winner; FO3 dt1 is a declared SKIP.

## Suggested Fix
- Install `QustRecord.dialogue_conditions` beside `DialogueQuestPriorities` (a `DialogueQuestConditions` resource, quest -> `ConditionList`; Skyrim's `dialogue_conditions` ride the same field) and evaluate it in `select_info` before each INFO's own list, with the same subject / target / quest `ConditionContext`; memoise per quest within one call.
- **Coupled prerequisite**: an uncatalogued function evaluates to `0.0` (`ConditionFunction::Unknown`), so `fn == 1` fails closed. Quest-level lists use `GetIsVoiceType` (FO3/FNV fn 427, Skyrim/FO4 fn 426), fn 372, Oblivion fn 254/365, Skyrim fn 700, FO4 fn 566 - none catalogued. Gating before cataloguing them would drop the generic fallback quests entirely, so catalogue them first (source-first, no guessing) or decline a quest list that contains an uncatalogued function (the #5380 "unknown-fn decline" shape).
- Pin it with two real-data tests (FO3: the bare-speaker `GREETING` must not resolve to `WaterBeggars`; FNV: not to `vDialogueEDE`) next to `dialogue_greeting_and_forcegreet_*_floor`, plus a synthetic priority-vs-quest-gate case in `dialogue.rs`. Add a named-speaker assertion to the FO3 dt1 so the SKIP can be lifted.

## Related
#5271 (closed; its quest-priority ordering turns the missing gate into a pre-emption), ESM-2026-10-09-D4-01 / #5494 (the Pitt flip, a symptom), #5380, FNV-2026-10-12-D2-01, #5426, #5397, `AUDIT_GAMEPLAY_2026-10-05.md` GAME-D2-2026-10-05-01 (deferred half). Sibling uncatalogued-function findings: OBL-2026-10-12-D2-01, SKY-D4-2026-10-12-01.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
