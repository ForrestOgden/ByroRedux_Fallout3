# SCR-D6-2026-10-11-01: ObScript reference indices resolve against the SCRO-only list, but compiled `r`-indices address the interleaved SCRO+SCRV list

Labels: high,scripting,game:oblivion,game:fnv,game:fo3,bug

- **Severity**: HIGH · **Dimension**: Legacy ObScript · **Untrusted-Input**: No for vanilla (fires on shipped masters); Yes in general
- **Location**: `crates/scripting/src/obscript_vm.rs:129`, `:668-676` (`resolve_ref_var`); producers `crates/plugin/src/esm/records/script.rs:77`, `:198-209` (SCRV → `ref_var_indices`, SCRO → `ref_form_ids`, two separate vectors)
- **Status**: NEW — follow-on of closed #4069, which split SCRV out for FormID remapping and left no consumer aligned
- **Description**: the `u16` after an `r`/`G` tag, and the caller index of a `0x1c` statement, is a 1-based index into the interleaved SCRO+SCRV list. `resolve_ref_var` indexes only `ref_form_ids`. After the first SCRV every literal reference shifts, and an index past the SCRO count resolves to `None` → `Ref(0)` → `QuestFormId(0)`.
- **Evidence** (agent-run read-only census, 2026-10-11; code path confirmed by the orchestrator):
  - `SCHR.num_refs == #SCRO + #SCRV` in 2393/2393 Oblivion and 2576/2576 FNV scripts.
  - Mis-resolved quest-stage ops: Oblivion 39/511 calls (8 scripts: `SE13Script`, `SE05QuestScript`, `SE02QuestScript`, `MQ16Script`, `MQ09Script`, `MQ05Script`, `MQ04Script`, `MS45Script`); FNV 50/290 (19 scripts, 9 resolve to form 0); FO3 16/148.
  - FNV `Patch04Quest00SCRIPT`: `r5` resolves to `0x163429` instead of `0x163427`, `r9` to `0x17BA15` instead of `0x15F4FF`, and `r10` to form 0. One of five quests is never started, and `StopQuest` targets form 0, so the script never stops itself.
  - The same shift applies to `' G<n>'` globals.
  - A `Ref` kept in the `f32` local map loses bits for FormIDs ≥ 2^24.
  - The M47.3 gate `vanilla_oblivion_quest_scripts_execute_and_setstage` checks that a SetStage was issued, not which quest.
- **Impact**: quest-state corruption (wrong quests started, stopped or staged; phantom `QuestStageState` entries). FO3/FNV quest scripts run too (SCR-D6-03).
- **Related**: #4069, #5088, SCR-D6-02.
- **Suggested Fix**: keep the interleaved order in `ScriptRecord` (`refs: Vec<ScriptRef::{Form, Var}>`, `num_refs == refs.len()` asserted) and index that. Treat `Ref(0)` / unresolved as a decline, not form 0. Strengthen the real-data gate to assert target identity (`Patch04Quest00SCRIPT`).
