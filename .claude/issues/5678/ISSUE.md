# AUD-2026-10-12-D4-01: A despawned or session-replaced speaker's voice line is not cut; it plays out at the last position

Labels: low,audio,dialogue,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5678

Filed from `docs/audits/AUDIT_AUDIO_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Manager & ECS Lifecycle
- **Location**: `crates/audio/src/lib.rs` (`stop_sounds_for`, `sync_emitter_positions`), `byroredux/src/systems/dialogue_voice.rs:339` (`stop_line_voice`); callers `combat.rs:581`, `npc_dialogue.rs:368`, `:573`
- **Status**: NEW (the open check the skill asked for; #5432 covers topic change, close and death only)
- **Description**: `stop_sounds_for` has exactly three production routes, all in dialogue/combat code. Nothing in `cell_loader/unload.rs`, the session-replace purge or `save_io` calls it. When a speaker entity is despawned mid-line (cell unload, F9 load), its `Anchored` segments lose their `GlobalTransform` and `sync_emitter_positions` skips them. `prune_stopped_sounds` does not touch `Anchored` by design, so they play to the end at the last position, up to the length of the line (several segments chained with delayed starts).
- **Evidence**: `git grep -n 'stop_sounds_for\|stop_line_voice' -- byroredux/src` lists only `dialogue_voice.rs`, `combat.rs`, `npc_dialogue.rs`. `world.rs` `despawn` does not reclaim ids, so there is no aliasing: the sound is stale, not misattributed.
- **Impact**: A force-greet or hello line that outlives its cell keeps talking in the next cell at stale coordinates. Interior and exterior spaces overlap, so it can sit near the new listener. Short and rare.
- **Related**: #5432 (closed), #5410 (closed).
- **Suggested Fix**: In `prune_stopped_sounds`, treat an `Anchored` sound whose followed entity has no `GlobalTransform` for a tick as orphaned and stop it with the standard fade; or call `stop_line_voice` from the despawn/purge path for entities carrying `NpcDialogueTopic`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **LOCK_ORDER**: If a RwLock scope changes, TypeId-sorted acquisition is preserved
- [ ] **TESTS**: A regression test pins this specific fix

