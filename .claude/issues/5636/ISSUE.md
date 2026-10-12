# SCR-D5-2026-10-11-03: `GetQuestRunning` answers 1.0 for stopped (and completed/failed) quests — `is_running || is_started`

Labels: medium,scripting,quests,game:skyrim,bug

- **Severity**: MEDIUM · **Dimension**: Event Runtime / SM gate soundness · **Untrusted-Input**: No · independently found as SCR-D3-2026-10-11-02
- **Location**: `crates/scripting/src/condition.rs:728-738` (esp. `:734-736`); `quest_stages.rs:214-216` (`is_started` = map membership)
- **Status**: NEW (#5380)
- **Description**: `is_started` stays true after `Quest.Stop()` and after Complete/Fail. The engine's running predicate everywhere else is `is_running` alone: ObScript `GetQuestRunning` (`obscript_quests.rs:240-249`), dialogue ownership (`dialogue.rs:312`), SM pool eligibility (`story_manager.rs:911`), alias refresh (`scene/quest_alias.rs:718`). The CK/GECK wikis define the function as "currently running".
- **Evidence**: `get_quest_running_gate_evaluates_real_state` covers only never-started vs running. Vanilla gates of the shape `GetQuestRunning(X) == 0` (`CWChangeLocationScenes` on `CWFinale`; 29 Skyrim SM terms use fn 56) stay false forever once X has run and stopped. `== 1` gates (DA09) stay true, which widens SCR-D5-01 from "while DA09 runs" to "ever after DA09 started".
- **Suggested Fix**: use `is_running` only, decide Completed explicitly, and add a Stopped case to the gate test.
