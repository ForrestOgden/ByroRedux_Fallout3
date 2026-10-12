# TD2-2026-10-12-03: Two audio dispatch loops share ~60 lines of spatial-track build, `play`, and `ActiveSound` push; `play_oneshot` / `play_oneshot_following` share their prologue

- **GitHub**: #5738 (https://github.com/matiaszanolli/ByroRedux/issues/5738)
- **Labels**: low,audio,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD2-2026-10-12-03`)

**Audit finding** `TD2-2026-10-12-03` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/audio/src/lib.rs` — `drain_pending_oneshots` vs `dispatch_new_oneshots`; `play_oneshot` vs `play_oneshot_following` (duplicate `const MAX_PENDING`)

## Description
Both loops build `SpatialTrackBuilder`, call `apply_reverb_send`, `apply_underwater_filter`, `add_spatial_sub_track`, `linear_volume_to_db`, `track.play` and push an `ActiveSound` with the same `last_position` seeding and `stop_issued: false`; they differ in the source tag, the loop-region flag and `unload_fade_ms`. Each setup call was added to both by hand (the underwater filter #3086, reverb send Phase 6). The two public `play_oneshot*` functions repeat the `oneshots_requested` bump, the `manager.is_none()` gate and the 256-entry FIFO cap, including a duplicated `const MAX_PENDING`.

## Evidence
Clone scan windows between the two loops and between the two `play_oneshot*` fns. The entity path is documented test/tooling-only since #5612, which makes the twin harder to justify.

## Impact
Any new send/filter (the next one after reverb and underwater) has to be added at two places; the file is at ~1977 LOC (2000-line watch-list edge).

## Suggested Fix
`fn start_spatial_sound(audio_world, mgr, listener_id, position, attenuation, sound, volume, looping) -> Result<(StaticSoundHandle, SpatialTrackHandle, Option<...>)>` and a `push_pending(&mut self, item)` for the queue cap; both loops keep only their source-specific tail.

## Related
#5612, TD1 watch-list.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
