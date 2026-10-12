# SCR-D3-2026-10-11-06: `StoryEvent` is a one-slot-per-entity marker with three producers; a player-keyed CLOC and a player-victim KILL in one frame overwrite each other

Labels: low,scripting,quests,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 3
- **Location**: `story_manager.rs:532-543`; `systems/story_events.rs:131, 158`; `character.rs:1570`
- **Status**: NEW

`StoryEvent` is one slot per entity with three producers. A player-keyed CLOC and a player-victim KILL in one frame overwrite each other, and the CLOC cursor has already advanced. Make it a per-entity batch merged on insert

