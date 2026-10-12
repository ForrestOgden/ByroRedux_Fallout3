# SCR-D3-2026-10-11-03: the CTDA remap table `param1_is_form_id` was not extended for fn 56 / 74 (and 71)

Labels: medium,scripting,esm-plugin,bug

- **Severity**: MEDIUM · **Dimension**: Event Runtime / ESM handoff · **Untrusted-Input**: Yes
- **Location**: `crates/plugin/src/esm/records/condition.rs:525-544` (table), `:563-578` (use); consumers `crates/scripting/src/condition.rs:728-745`, `:853`
- **Status**: NEW for 56/74 (#5380 catalogued them without the paired remap entries); 71 `GetInFaction` predates the delta, with no issue found
- **Description**: the table doc says it mirrors the scripting catalog. It lacks 56 (quest), 74 (GLOB) and 71 (faction), whose `param_1` is a FormID. The evaluator looks the value up in `QuestStageState` / `Globals`, both keyed in global load-order space (**confirmed by the orchestrator: no 56/71/74 arm**).
- **Impact**: for a plugin whose master list is not a prefix of the load order (any mod, ESL, reordered DLC), these functions silently read the "absent" default. On an SM node that flips an authored gate: `GetQuestRunning(X) == 0` passes vacuously. Vanilla masters in order are unaffected, which is why no census-based test caught it.
- **Suggested Fix**: add 56/71/74 and derive the list from a per-function `param1_is_form_id()` method on the enum, with a test over `CATALOG`.
