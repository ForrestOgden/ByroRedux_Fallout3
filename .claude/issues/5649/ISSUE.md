# SCR-D2-2026-10-11-01: `npc_dialogue_selection` Access row (and its source-scan guard) omit the condition evaluator's acquisitions, including the new `StoryManagerRng` write

Labels: low,scripting,concurrency,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 2
- **Location**: `boot/schedule/late.rs:457-535`; `mod.rs:1059-1071`
- **Status**: NEW

The `npc_dialogue_selection` Access row omits what `evaluate_function` acquires, including the new `StoryManagerRng` W. The guard never scans `condition.rs`. Exclusive row, so nothing is mis-scheduled today. Add `(CONDITION_SRC, "evaluate_function")` and declare the types

