# SCR-D5-2026-10-11-04: `player_controller_system`'s Access row omits the KILL producer's acquisitions (#5434 hop into `story_events.rs`)

Labels: low,scripting,concurrency,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 5
- **Location**: `boot/schedule/early.rs:23-76`; scan sources `mod.rs:603-617`
- **Status**: NEW

`player_controller_system`'s Access row omits the KILL producer's acquisitions (`StoryEvent` W, `LoadedCellIndex`, `CurrentCellContext`, `CurrentExteriorContext`) reached via #5434's `raise_kill_story_event`. The scan does not list `story_events.rs` for that system. Not a live race; declare the four types and add the hop to `PARALLEL_SYSTEMS`

