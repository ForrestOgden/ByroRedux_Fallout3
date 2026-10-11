# TD6-2026-10-11-05: Entity-based one-shot audio (`OneShotSound` + `spawn_oneshot_at`) has no producer in the engine; module docs still pitch it as the gameplay entry point

URL: https://github.com/matiaszanolli/ByroRedux/issues/5612
Labels: low, audio, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 6
- **Location**: `crates/audio/src/lib.rs:36-60` (module doc "Phase 3 (this commit)"), `:1010-1016` (`OneShotSound`), `:1331-1490` (dispatch + marker-clear arms), `:1586-1620` (`spawn_oneshot_at`)
- **Status**: NEW as a tracked item (noted and waved off as "latent" in AUDIT_AUDIO_2026-08-16 and AUDIT_AUDIO_2026-07-16)
- **Age**: `45a986485` (2026-05-05)
- **Effort**: small (document) / medium (delete the entity path and its ~10 tests)
- **Description**: The only non-test insert of `OneShotSound` is inside `spawn_oneshot_at` (`lib.rs:1614`), and `spawn_oneshot_at` itself has zero callers outside `crates/audio/src/tests.rs`. Every shipped sound (footsteps, impacts, splash, voice) goes through `AudioWorld::play_oneshot`. The module header still says the "intended consumer is gameplay code (footstep timer, weapon-fire trigger, dialogue dispatcher)" and carries three "(this commit)" phase headers.
- **Evidence**: `grep -rnw OneShotSound crates byroredux --include='*.rs' | grep -v tests` -> only `crates/audio/src/lib.rs` (def, dispatcher, spawn helper). `grep -rnw spawn_oneshot_at ... | grep -v tests` -> definition only. Also `load_streaming_sound_from_file` (lib.rs:1671, "Phase 5 ... loose Data/Music files") has zero uses anywhere, test or not.
- **Impact**: Two dispatch paths to keep correct (the marker-clear failure arms exist because of #2394) where one is exercised only by tests; a contributor following the docs spawns entities the engine never needed.
- **Related**: #2394 (closed), AUDIT_AUDIO_2026-08-16 line ~201
- **Suggested Fix**: Mark the entity path "test/tooling only; engine uses `play_oneshot`" in the module doc and drop the "(this commit)" headers, or delete it; hand `load_streaming_sound_from_file` to Dim 8.
