# Audio Subsystem Audit (M44) — 2026-10-12

**HEAD**: 5d31fd637 · **Baseline**: [`AUDIT_AUDIO_2026-10-08.md`](AUDIT_AUDIO_2026-10-08.md) (HEAD `00f580e09`) · **Audited**: Dim 1 (Spatial Dispatch & Listener — `SoundSource` split, `play_oneshot_following`, `stop_sounds_for`), Dim 2 (Send Graph — kira 0.12 re-check), Dim 3 (Music & Caches — kira 0.12 feature set, `VoiceSoundCache`), Dim 4 (Manager & Schedule — new `impact_audio_system` row, voice row), Dim 5 (Engine Consumers — Wave-4 footsteps/impacts, voice rework, provider precedence) · **Unchanged since baseline (skimmed)**: none — every dimension had commits (kira 0.10 → 0.12 touches Dims 1–4; Dim 5 had 30+ commits). REGN music dispatch (`asset_provider/audio.rs`) is unchanged apart from the shared-handle provider rebuild (#5288).

- **Command**: `/audit-audio` (default delta scope), one leg of `/audit-suite --preset comprehensive`. Solo, no sub-agents; scratch in `/tmp/audit/audio/dim_1.md`–`dim_5.md`. No engine launch, no audio played on the user's device.
- **kira**: `0.12.5` (`Cargo.toml:209`: `default-features = false`, `cpal`, `cpal-realtime` (no `-dbus`), `wav`, `pcm`, `ogg`, `vorbis`, `mp3`, `flac`). This is kira's own 0.12.5 default set minus `cpal-realtime-dbus`, so no codec the old 0.10 defaults decoded was dropped. `adpcm` is absent, and it was absent from 0.10's `wav` feature too.
- **Tests run** (toolchain 1.96.0):
  - `cargo test -q -p byroredux-audio`: **38 passed, 0 failed, 9 ignored**.
  - `cargo test -q -p byroredux --bin byroredux -- audio dialogue_voice footstep impact scheduler_access combat_anim`: **93 passed, 0 failed, 1 ignored**.
  - `--ignored real_fnv_sounds_decode_through_kira` (device-free, FNV data): **1 passed**. A WAV at 48 kHz and an Ogg Vorbis at 32 kHz both decode through the new 0.12 feature set.
  - **Not run**: the eight device-gated lifecycle guards (`looping_emitter_survives_natural_duration_and_stops_on_emitter_remove`, `non_looping_emitter_stops_on_emitter_remove_regression_858`, `play_music_looping_survives_track_end`, `play_music_drives_streaming_playback_on_real_ogg`, `emitter_position_follows_the_source_entity_regression_3086`, `anchored_voice_follows_survives_prune_and_stops`, `play_oneshot_queue_drives_real_playback`, `audio_system_full_lifecycle_on_real_fnv_sound`). The suite forbids opening the user's audio device. Despawn truncation, follow, anchored-voice stop and music looping were verified by tracing the code against the kira 0.12.5 source.
- **Headless-mode boot: PASS** (`audio_world_constructs_without_panic_on_any_environment`, `explicit_headless_world_discards_playback_without_retaining_sound`).
- **Measurements (device-free, read-only)**:
  - A Python BSA header census of every `.wav` in the shipped sound archives found **only RIFF format tag 1 (PCM)**. FNV `Fallout - Sound.bsa` 4,233/4,233 and `Update.bsa` 3/3; FO3 2,643; Skyrim SE 5,929; Skyrim LE 4,681; Oblivion 1,524 parsed (9 more were not parsed by my census script, the v103 embedded-name handling). The `pcm` + `vorbis` set therefore covers every vanilla SFX WAV, and there is no ADPCM gap.
  - Every key in `FOOTSTEP_CANDIDATES`, `WATER_SPLASH_CANDIDATES` and the three hardcoded Skyrim combat paths exists in its tagged game's real sound archive.
  - FO3 sound-archive overlap (the profile comment says "not measured"): `Fallout - Sound.bsa` ∩ `Fallout - Voices.bsa` = 0, `Sound` ∩ `MenuVoices` = 0, `Voices` ∩ `MenuVoices` = 98 keys. Last-wins precedence is moot for Sound vs voices.

## Executive Summary

Since the baseline the audio surface changed in four ways, all of which trace clean except where a finding below says otherwise:

1. **kira 0.10 → 0.12 (`804c715`, `fb902f4bc`).** The spatial sub-track, filter, reverb and send code is unchanged and still compiles against the same call shapes. I re-read the kira 0.12.5 source for every behavioural claim in the crate docs. `Tween::default()` is still 10 ms linear, `listener_capacity` is still 8, and `Mix::DRY` bypass holds. One doc claim about sends is wrong (D2-01).
2. **Voice rework (`d1a7ee8c6` #5410, `40b77828b` #5432, `8410ad1e5` #5382, `080b35f35` #5383, `5b68792b7`/`4d6294bb1` file naming).** The three baseline MEDIUM/LOW findings (cache growth, lock-order inversion, frozen position and no stop handle) are fixed. `VoiceSoundCache` is a byte-budgeted LRU, voice is entity-anchored, and `stop_sounds_for` has production callers. Residual: a despawned or session-replaced speaker's line is not cut (D4-01).
3. **Wave-4 surface footsteps and impacts (`886edcded`, `441d90c2f`, `1291f370a`, `3909778eb`, `ac710964e`, `e1f4fb921`, `71b86091b`).** The footstep path is sound (#5545 self-hit exclusion, #5556 snapshot before the `AudioWorld` guard). `impact_audio_system` is a production no-op today because both maps and `default_sound` ship empty, but the mechanism computes the impact **position wrongly** — halfway to the body *origins*, and every exterior terrain tile's body origin is the world origin (D5-01).
4. **Provider precedence flip (`#3917`).** The code is correct (last-listed wins); three documentation sites, one of them the user-facing CLI table, still tell users to list overrides first (D5-02).

| Severity | NEW | Existing (residual, not re-reported) | Total |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 1 | 0 | 1 |
| LOW | 4 | 11 | 15 |
| **Total** | **5** | **11** | **16** |

Existing residuals: #5347, #5348, #5349, #4743, #4747, #3816, #3301, #5526, #5464, plus the suite-reported PERF-D1-2026-10-12-03 and ECS-2026-10-12-D6-01. No regressions: #3086, #4146, #5432, #5148, #5382, #5383, #5410 and #5305 all hold. Archive precedence is still `.rev()`.

## Lifecycle Invariant Matrix (HEAD `5d31fd637`)

| Invariant | State | Anchor |
|---|---|---|
| Field-drop order `active_sounds → pending_oneshots → oneshots_requested → music → reverb_send → reverb_send_db → listener → manager → multi_listener_warned → underwater → emitter_position_updates` | HOLDS (declaration order unchanged) | `crates/audio/src/lib.rs` `AudioWorld` |
| `headless()` never contacts a device; `SUB_TRACK_CAPACITY` 512 / `SEND_TRACK_CAPACITY` 32 applied in `new()` | HOLDS | `lib.rs` `AudioWorld::new` |
| Sticky listener (never cleared; kira `listener_capacity` = 8 in 0.12.5 `ManagerSettings`) | HOLDS | `sync_listener_pose` |
| `audio_system` body order: listener → emitter follow → underwater → drain → dispatch → prune | HOLDS | `lib.rs` `audio_system` |
| One unit seam (`bu_to_audio_space`) on listener, both dispatch paths and the follow site | HOLDS; guard `every_kira_position_site_goes_through_the_unit_seam` covers all four sites (#5411) | `tests.rs` |
| Despawn truncation: only `SoundSource::Emitter` is emitter-coupled; `Anchored` stops only via `stop_sounds_for`; `stop_issued` debounce | HOLDS by trace (device guards not run). Residual: a despawned speaker is never routed to `stop_sounds_for` (D4-01) | `prune_stopped_sounds` |
| `EntityId` is `u32` and never reclaimed | HOLDS (`world.rs` `despawn` doc), so an `Anchored` voice can never alias a recycled id | `ecs/world.rs` |
| Late exclusive order `ragdoll → submersion → water_damage → reconcile → water_interaction → water_audio → audio_system`; `footstep` and `impact_audio_system` before them; `reverb_zone_system` a parallel registration | HOLDS | `boot/schedule/late.rs:117-260` |
| `npc_dialogue_selection` registered after `audio_system`, so a voiced line drains the next frame | HOLDS (one-frame latency, no loss); its Access row now declares the voice path (#5414/#5649) and the source-scan guard follows `play_line_voice` | `late.rs`, `schedule/mod.rs` |
| `SoundCache` → `SoundArchiveProvider` is the single nesting direction (shared cache and `VoiceSoundCache` alike) | HOLDS; `voice_and_combat_sound_paths_share_one_cache_provider_order` pins it | `combat_anim.rs`, `dialogue_voice.rs` |
| Open-ended-key consumer never lands on `SoundCache` | HOLDS: dialogue voice rides `VoiceSoundCache` (64 MiB LRU, negatives capped at 4096, telemetry via `ownership_sample.rs`) | `lib.rs`, `boot/world.rs` |
| REGN music redispatch gated on `music_form` change | HOLDS at all three sites | `cell_loader/load.rs:713`, `:1100`, `scene/world_setup.rs:614` |
| Footstep lock nesting acyclic (body gate resolved before guards; `FootstepConfig` snapshot before the `AudioWorld` write) | HOLDS (#5305, #5556) | `systems/audio.rs:170-300` |
| Archive precedence `.rev()` last-wins | HOLDS (code); docs drifted (D5-02) | `asset_provider/audio.rs:141` |

## Findings

### AUD-2026-10-12-D5-01: Clutter-impact sounds play at the midpoint of the two parent *body origins*, not the contact. Every exterior terrain tile's body origin is the world origin
- **Severity**: MEDIUM (dormant today: `ImpactConfig` ships with no `default_sound` and an empty map, so `impact_audio_system` returns before it computes a position. It is wrong the moment IPDS/IPCT content lands.)
- **Dimension**: Engine Consumers
- **Location**:
  - `crates/physics/src/world/mod.rs:706-716` (`contact_event_position`)
  - `byroredux/src/systems/audio.rs:405` (the consumer)
  - `byroredux/src/cell_loader/terrain.rs:1276-1285` (terrain ghost placed at `Vec3::ZERO`)
  - `byroredux/src/cell_loader/spawn.rs:398-418` (static trimesh ghosts placed at the REFR origin)
- **Status**: NEW (introduced by `1291f370a`)
- **Description**: The impact position is `(body_a.translation + body_b.translation) / 2`. That is the midpoint of the two *rigid-body origins*, not of the contact manifold.
  - For a dynamic clutter body hitting a static one, the second origin is wherever the static's body sits.
  - Static architecture ghosts are placed at the placement origin (`spawn_trimesh_collider_ghost(world, positions, indices, pos, rot, scale, …)`), which for a large kit piece can be many metres from the point of contact.
  - Exterior terrain is worse. The LAND tile's vertices are already world-space and the ghost is spawned with `Vec3::ZERO`, so its body translation is exactly the world origin.
  - The `ContactForceEvent` carries no contact point. The point has to come from the narrow phase (`contact_pair(c1, c2)` manifold points) or from the dynamic body's own position.
- **Evidence**:
  ```rust
  // physics/src/world/mod.rs:708-715
  let pa = self.bodies.get(a)?.translation();
  let pb = self.bodies.get(b)?.translation();
  Some(Vec3::new((pa.x + pb.x) * 0.5, (pa.y + pb.y) * 0.5, (pa.z + pb.z) * 0.5))
  // cell_loader/terrain.rs:1276-1284
  spawn_trimesh_collider_ghost(world, &positions, &indices, Vec3::ZERO, Quat::IDENTITY, 1.0, None)
  ```
  The only test is `contact_force_started_event_carries_surface_materials` (`physics/src/world/mod.rs:4102`), which asserts `contact_event_position(&event).is_some()`. Nothing pins the value, and `systems/audio.rs` tests cover selection only (`impact_selects_the_surface_sound_when_authored`, `an_unmapped_material_skips_only_its_own_impact`, `impact_material_resolution_keeps_both_zero_pairs`), never the position handed to `play_oneshot`.
- **Impact**: Once impact content is authored, a clutter item landing on exterior terrain at (x, z) plays at (x/2, z/2). At 300 m from the origin that is 150 m from the item, past the 24 m attenuation range, so exterior terrain impacts are inaudible. Interior statics shift the sound by half the kit piece's origin offset. No effect on the current build.
- **Related**: #5562 (the same "body origin where a contact point belongs" family on the water side), PERF-D1-2026-10-12-03 (same system, perf aspect).
- **Suggested Fix**: Resolve the position from the narrow-phase contact pair (deepest manifold point, converted to world space), falling back to the dynamic collider's body translation when the manifold is empty. Add a test that drops a body onto an off-origin static and asserts the resolved point is within the contact's extent.

### AUD-2026-10-12-D5-02: `--sounds-bsa` is documented as "first hit wins, list overrides first" in three places; the provider has been last-listed-wins since #3917
- **Severity**: LOW
- **Dimension**: Engine Consumers (archive precedence)
- **Location**:
  - `docs/engine/game-loop.md:56` (the user-facing CLI table)
  - `byroredux/src/asset_provider/audio.rs:83`
  - `byroredux/src/asset_provider/texture.rs:297` (and `try_load_default_water_splash`'s "first archive/candidate hit wins")
- **Status**: NEW
- **Description**: The #3776 wording ("override/mod archives listed before the vanilla one, first hit wins") predates #3917, which flipped `SoundArchiveProvider::extract` to `extract_first(self.archives.iter().rev(), …)`. The same file's `extract` doc, `assets/debug_profiles.toml` (which lists `Update.bsa` *after* `Fallout - Sound.bsa` for exactly this reason) and the CLI-profile comments all say last-wins. The three drifted sites were not updated.
- **Evidence**:
  ```text
  game-loop.md:56   Repeatable — override/mod archives listed before the vanilla one, first hit wins (#3776)
  audio.rs:141      extract_first(self.archives.iter().rev(), archive_path)
  ```
- **Impact**: A user following the CLI documentation lists their override first; the vanilla archive then shadows it, and the footstep, splash, REGN and voice lookups serve unmodded audio with no log line. Workaround: list overrides last.
- **Related**: #3776 (closed), #3917 (closed). `script.rs:935` documents `ScriptProvider`'s deliberate first-wins correctly.
- **Suggested Fix**: Change all three wordings to "later `--sounds-bsa` archives override earlier ones (last-listed wins; `ScriptProvider` is the first-wins exception)".

### AUD-2026-10-12-D2-01: The "send level can't retro-apply" limitation is documented as a kira limit; kira 0.12.5 has `SpatialTrackHandle::set_send`, and the limit is our own `with_send` gate
- **Severity**: LOW (doc claim; no behavioural effect today)
- **Dimension**: Send Graph
- **Location**: `crates/audio/src/lib.rs:869-882` (the `set_reverb_send_db` doc), `crates/audio/src/tests.rs:501`, `apply_reverb_send` at `lib.rs:295`
- **Status**: NEW (the claim is inherited from the closed #847 / 2026-06-23 audit and was never re-checked, including by the kira 0.12 bump)
- **Description**: The docs say "kira 0.12's `with_send` is build-time only … there is no `SpatialTrackHandle::set_send_volume`, so a level change cannot retro-apply to already-playing tracks". kira-0.12.5 `src/track/sub/spatial_handle.rs:126` has `pub fn set_send(&mut self, to, volume, tween) -> Result<(), NonexistentRoute>`; it changes the volume of an existing route, and 0.10.8 has the same method. The real constraint is that a route has to exist, and `apply_reverb_send` only calls `with_send` when the level is audible, so a track built dry has no route.
- **Evidence**: `kira-0.12.5/src/track/sub/spatial_handle.rs:121-135`: "This can only be used to change the volume of existing routes, not to add new routes."
- **Impact**: None today. Long-running looping emitters (REGN ambient loops, #3301) are not shipped. It matters at design time: a live fader is possible if every sub-track is built with a route at `SILENCE_DB` and `set_reverb_send_db` retargets live tracks.
- **Related**: #847 (closed).
- **Suggested Fix**: Reword the doc to "routes can be retargeted but not added; tracks built while the send was silent have no route", and note the always-build-the-route option for the future per-cell reverb work.

### AUD-2026-10-12-D4-01: A despawned or session-replaced speaker's voice line is not cut; it plays out at the last position
- **Severity**: LOW
- **Dimension**: Manager & ECS Lifecycle
- **Location**: `crates/audio/src/lib.rs` (`stop_sounds_for`, `sync_emitter_positions`), `byroredux/src/systems/dialogue_voice.rs:339` (`stop_line_voice`); callers `combat.rs:581`, `npc_dialogue.rs:368`, `:573`
- **Status**: NEW (the open check the skill asked for; #5432 covers topic change, close and death only)
- **Description**: `stop_sounds_for` has exactly three production routes, all in dialogue/combat code. Nothing in `cell_loader/unload.rs`, the session-replace purge or `save_io` calls it. When a speaker entity is despawned mid-line (cell unload, F9 load), its `Anchored` segments lose their `GlobalTransform` and `sync_emitter_positions` skips them. `prune_stopped_sounds` does not touch `Anchored` by design, so they play to the end at the last position, up to the length of the line (several segments chained with delayed starts).
- **Evidence**: `git grep -n 'stop_sounds_for\|stop_line_voice' -- byroredux/src` lists only `dialogue_voice.rs`, `combat.rs`, `npc_dialogue.rs`. `world.rs` `despawn` does not reclaim ids, so there is no aliasing: the sound is stale, not misattributed.
- **Impact**: A force-greet or hello line that outlives its cell keeps talking in the next cell at stale coordinates. Interior and exterior spaces overlap, so it can sit near the new listener. Short and rare.
- **Related**: #5432 (closed), #5410 (closed).
- **Suggested Fix**: In `prune_stopped_sounds`, treat an `Anchored` sound whose followed entity has no `GlobalTransform` for a tick as orphaned and stop it with the standard fade; or call `stop_line_voice` from the despawn/purge path for entities carrying `NpcDialogueTopic`.

### AUD-2026-10-12-D3-01: Audio documentation drifted behind the kira 0.12 / voice / Wave-4 work
- **Severity**: LOW
- **Dimension**: Music & SoundCache / docs
- **Location**:
  - `crates/audio/src/lib.rs:42` ("WAV + OGG covered by kira's default features") and `:1640-1642` ("kira pulls in symphonia with the `wav`, `ogg`, `mp3`, and `flac` features by default")
  - `docs/COMPATIBILITY.md:317` ("kira 0.10")
  - `docs/feature-matrix.md` Audio table (143-155): no row for the shipped Phase-V dialogue voice or the Wave-4 impact mechanism
  - `byroredux/src/asset_provider/texture.rs` (`DefaultSoundCandidate.games` doc says tags are `debug_profiles.toml` profile names; the profile is `skyrim_se`, the tag `skyrimse`)
- **Status**: NEW for the lib.rs, COMPATIBILITY.md and tag items. The feature-matrix gap is **Existing: #4747** (status docs; the REGN row is now qualified, the voice and impact rows are still absent). `docs/feature-matrix.md:147` "kira 0.10" was fixed after TD3-2026-10-11-04.
- **Description**: Since `804c715` the workspace sets `default-features = false` and lists the codecs explicitly (0.12 split container from codec: `wav` needs `pcm`, `ogg` needs `vorbis`). The crate docs still describe the features as kira's defaults, which is the framing that hides the "drop a codec and decode silently stops" trap the workspace `Cargo.toml` comment warns about.
- **Evidence**: `Cargo.toml:209-218` vs `lib.rs:42`, `:1641`.
- **Impact**: Documentation only.
- **Related**: #4747, #5600 (tech-debt).
- **Suggested Fix**: Point the two lib.rs sites at `Cargo.toml`'s explicit feature list and the `pcm`/`vorbis` requirement; update COMPATIBILITY.md; add voice and impact rows to the feature matrix.

## Prior-Finding Disposition

| Issue / ID | State at HEAD | Evidence |
|---|---|---|
| 10-08 D5-01 (voice cache growth) / #5382 | **Fixed** | `VoiceSoundCache` LRU 64 MiB, `inactive_audio_session_resolves_and_caches_no_voice` |
| 10-08 D5-02 (provider/cache order) / #5383 | **Fixed** | cache → provider inside `get_or_load`; `voice_and_combat_sound_paths_share_one_cache_provider_order` |
| 10-08 D5-03 (frozen position, origin default, no stop) / #5410, #5432 | **Fixed** | `play_oneshot_following`, `stop_sounds_for`, transform-less NPC skipped (`dialogue_voice.rs:233`) |
| 10-08 D4-01 (Access row) / #5414, #5649 | **Fixed** | `late.rs:525-540`, guard lists `play_line_voice` |
| 10-08 D1-01 (follow site and `with_start_delay` unpinned) / #5411 | **Fixed** | `tests.rs:1481-1550` |
| #5347 (swim gate duplicates `is_grounded`) | **Open, unchanged** | `systems/audio.rs:170-240` |
| #5348 (non-body footstep emitters suppressed) | **Open, unchanged** | `if entity != body \|\| gated` |
| #5349 (stale doc sites) | **Open, unchanged** | `components.rs:1751`, `scene.rs:1214` |
| #4743 (combat sound keyed to take-state) | **Open, unchanged** | no `combat_anim.rs` production diff beyond #5148 counters |
| #4747 (status docs) | **Open** | D3-01 notes the remaining feature-matrix gap |
| #3816, #3301 (MUSC/REGN) | **Open, unchanged** | `dispatch_region_ambient_music` mechanism intact |
| #5526 (Oblivion voice), #5464 (`.fuz`) | **Open** | unbuilt, documented in `dialogue_voice.rs` header |
| PERF-D1-2026-10-12-03 (impact system clones the table per frame) | Reported this suite | not re-reported |
| ECS-2026-10-12-D6-01 (footstep silent when `PhysicsWorld` absent) | Reported this suite | not re-reported; reduced worlds only |
| ECS-2026-10-12 observation (`reconcile_pending_dead_actors_system` Access row omits the `AudioWorld` write `stop_line_voice` takes) | Noted there, not filed | exclusive, so no scheduling effect |

**Noted, not findings**: the footstep (0.5–12 m), impact (1–24 m), splash/ripple (1–24 m, 0.45 damping) and combat (0.5–24 m) attenuation constants and the `REVERB_*` trio remain uncited by any authored source; the code already says so for the latter two. `audio_system` is a bare `add_exclusive` with no Access row by design (comment at `late.rs:262-270`).

## Future-Phase Readiness

- **FOOT / IPDS / IPCT**: the mechanism is live (Havok `surface_material` u32 → `FootstepConfig` / `ImpactConfig` `surface_sounds`) but both maps ship empty, so footsteps take the default and impacts are a no-op. **D5-01 must be fixed before impact content lands.** `footstep_system` also does a ray cast per step whose result is unused while the map is empty (negligible at ~2.6 steps/s).
- **MUSC / MUST / MSET / RDMD (#3816)**: the dispatch mechanism fails closed through every layer and stops stale playback; unchanged. CELL-level `music_type_form` is still hardcoded `None`.
- **Per-cell acoustics / live reverb fader**: possible with `SpatialTrackHandle::set_send` if every track is built with a route (D2-01).
- **Voice**: Skyrim+ `.fuz` needs an XMA2 decoder (kira/symphonia has none); Oblivion needs RACE `VNAM` + `.mp3` race/sex resolution (#5526). The backend now has the LRU, anchored-stoppable lines and a single lock order, so neither needs backend changes except D4-01.
- **Occlusion**: none.
- **Codecs**: FO4+ `.xwm` is not decoded (documented). All vanilla Bethesda SFX WAVs in FO3/FNV/Oblivion/Skyrim are PCM, so no `adpcm` feature is needed.

## Dedup Method

- **Open issues**: `/tmp/audit/issues.json` (122). Searched for audio, sound, footstep, music, reverb, voice, dialogue, impact, kira, region and access-row titles. Matches: #5526, #5349, #5348, #5347, #4747, #4743, #3816, #3301, #5641 (different row), #5463 (unrelated). None covers D5-01, D5-02, D2-01, D4-01 or D3-01.
- **Closed** (`gh issue list --state closed --search`): "impact sound position" → #5410, #5562, #5545 (different bugs); "contact_event_position", "set_send reverb", "kira default features doc", "stop_sounds_for despawn voice" → no match; "first hit wins sounds-bsa" → #3776 (its wording is the stale text; the code later changed under #3917).
- **Suite overlap** (`/tmp/audit/REPORTED_THIS_SUITE.md`, re-read before finalizing): PERF-D1-2026-10-12-03, ECS-2026-10-12-D6-01 and the ECS access-row observation are referenced, not re-reported. The physics report contains nothing on `contact_event_position`.

Publish with `/audit-publish docs/audits/AUDIT_AUDIO_2026-10-12.md`. Labels: `audio` on all; `physics` + `bug` on D5-01; `documentation` + `doc-rot` on D5-02, D2-01 and D3-01; `bug` on D4-01 (add `dialogue`); `game:fnv` is not warranted for any finding.
