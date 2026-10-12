# SCR-D1-2026-10-11-07: `quest_stage_gate::find_advance_event` ignores `State` membership / `is_auto` and every other handler

Labels: medium,scripting,bug

- **Severity**: MEDIUM (latent) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `translate/recognizers/quest_stage_gate.rs:275-309`
- **Status**: NEW
- **Description**: the first `OnActivate`/`OnTriggerEnter` "at top level and inside any State" is accepted. Since #4319 the decompiler hoists the *auto* state's events, so a `State X` item is by construction non-auto. A script whose only single-`SetStage` handler lives in a non-auto state (reached after `GotoState`) is armed from spawn.
- **Suggested Fix**: accept only script-scope events or `is_auto` states, and decline when the script calls `GotoState`.
