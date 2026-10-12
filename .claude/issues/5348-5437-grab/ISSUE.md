=== ISSUE #5348 ===
AUD-2026-10-05-D5-02: Non-body footstep emitters are suppressed whenever PlayerEntity is set, not just the camera's — every future NPC FootstepEmitter would be silent in character mode, and an F-toggled FlyCam stays silent contrary to the comments

- **Severity**: LOW. It is latent: no production NPC carries a `FootstepEmitter` today. The FlyCam behaviour is arguably desirable but disagrees with the code's own stated intent.
- **Dimension**: Engine Consumers
- **Location**:
  - `byroredux/src/systems/audio.rs:155-158`, `:197-204`: the gate.
  - `systems/character.rs:1063-1083`: `toggle_player_mode` leaves `PlayerEntity` set.
  - `components.rs:1835-1844`: the `FootstepScratch` sizing doc ("5–10 walking NPCs, peak ~50").
  - `scene.rs:977-981`: comment.
  - `late.rs:84-86`: comment.
- **Status**: NEW (introduced by `ccc743160`, #5146)
- **Description**: The new branch is `if let Some(body) = player_body { if entity != body { re-seed; continue } … }`. Two problems follow.
  1. **Opt-in is no longer component-driven.** The skill invariant, the system doc ("Spawn a `FootstepEmitter` on the player entity to opt in") and `FootstepScratch`'s capacity rationale all describe a component that any walker can carry. Under #5146, in character mode, which is the only mode where NPC footsteps matter, every emitter except the player body's is re-seeded and never fires. The #5146 intent was to skip the *camera's* emitter, but the predicate skips everything that is not the body. The first NPC or FOOT (3.5b) consumer that inserts a `FootstepEmitter` will be silently muted, and none of the #5146 tests would notice.
  2. **FlyCam after the F toggle.** `toggle_player_mode` flips `PlayerMode` but leaves `PlayerEntity(Some(body))`, and the body is frozen in FlyCam. The camera emitter therefore stays suppressed and flying is silent. That is arguably right, since flying is not walking. But the code says otherwise in three places:
     - The camera re-seed comment (`:199-201`) says it exists "so a later FlyCam switch doesn't replay the whole boom arc as one stride burst", which assumes the camera resumes stepping after the switch.
     - `late.rs:84-86` says "the emitter lives on the active camera in FlyCam".
     - The `footstep_system` doc says "FlyCam scenes (no body) keep the original camera-is-the-mover behaviour".

     So a `--fly` boot steps while flying and an F-toggled FlyCam does not. Neither behaviour is pinned.
- **Evidence**: see Location. `git grep -n 'FootstepEmitter::new()' -- byroredux/src ':!*/systems/audio.rs'` finds only `scene.rs:982` (the camera) and `scene.rs:1230` (the body). `toggle_player_mode` has no `PlayerEntity` write, and the only production `PlayerEntity` insert is `scene.rs:1223`.
- **Impact**: there is no audible regression today. A latent trap is set for the per-NPC and FOOT footstep work, and the documentation and code disagree on FlyCam.
- **Related**: #5146 (closed), FOOT phase 3.5b, D5-03.
- **Suggested Fix**:
  - Skip only the entity that actually stands in for the player: the `ActiveCamera` entity while `PlayerMode == Character`. Alternatively, remove the camera's `FootstepEmitter` when the body spawns and re-add it on Character → FlyCam if stepping while flying is wanted.
  - Pin whichever FlyCam behaviour is chosen. Add a test in which a third, non-player emitter walking in character mode fires.

_Source: `AUDIT_AUDIO_2026-10-05.md` (AUD-2026-10-05-D5-02), audit HEAD `a2c24b16e`._

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / call sites
- [ ] **TESTS**: A regression test pins this specific fix


=== ISSUE #5349 ===
AUD-2026-10-05-D5-03: #5146 left three stale or wrong doc sites about which entity carries the footstep emitter

- **Severity**: LOW
- **Dimension**: Engine Consumers / Manager & Schedule (doc)
- **Location**:
  - `byroredux/src/scene.rs:1227-1229`: "The camera's emitter (inserted in `setup_camera_and_lights`)". No such function exists (`git grep` has 0 definitions; the insert is in `spawn_initial_camera`, `scene.rs:903`/`:982`).
  - `scene.rs:977-981`: "character mode **moves** it onto the player body". Nothing moves: the body gets a second emitter and the camera keeps its own (see D5-02).
  - `byroredux/src/components.rs:1717-1719`: the `FootstepEmitter` doc says "today the fly-camera entity; an `M28.5` character controller will own this in future". The body has owned it since #5146.
  - `byroredux/src/boot/schedule/late.rs:83-99`: the footstep pin rationale still says that in player/third-person mode the system reads "`camera_follow_system`'s pose, not last frame's". In character mode the only accumulating emitter is now the body, whose `GlobalTransform` comes from Early `character_controller_system` plus PostUpdate propagation. `camera_follow_system` no longer feeds any audible footstep, and the camera emitter is only re-seeded. `footstep_runs_after_camera_follow_in_late` still pins an order that is now harmless but unmotivated.
- **Status**: NEW (introduced by `ccc743160`)
- **Description**: see Location. The dead function name is the kind of path/symbol reference that `_audit-validate.sh`-style checks flag. The `components.rs` "in future" line is directly falsified by the commit.
- **Impact**: documentation only. The next reader of the Late pin will look for a camera-pose dependency that no longer exists.
- **Related**: D5-02, #4146 (the earlier comment drift class in the same file).
- **Suggested Fix**:
  - Name `spawn_initial_camera` at `scene.rs:1228`.
  - Say "adds a body emitter" at `:979`.
  - Rewrite the `FootstepEmitter` doc to say the body carries it in character mode and the camera in FlyCam boots.
  - Re-word the `late.rs` rationale. The Late pin now matters only for FlyCam-boot camera emitters, or it could move to PostUpdate after propagation if it is kept for the body alone. Keep the pin test or retire it accordingly.

_Source: `AUDIT_AUDIO_2026-10-05.md` (AUD-2026-10-05-D5-03), audit HEAD `a2c24b16e`._

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / call sites
- [ ] **TESTS**: A regression test pins this specific fix


=== ISSUE #5351 ===
OBL-2026-10-05-D4-01: `OblivionHdrLighting` docs say no renderer reads HNAM (the sunlight dimmer has been consumed since `df59c6362`), and say the 56-byte HNAM is "Oblivion / FO3 / FNV" (FO3/FNV author none)

**Source**: `docs/audits/AUDIT_OBLIVION_2026-10-05.md` (OBL-2026-10-05-D4-01) · audit HEAD `a2c24b16e`

- **Severity**: LOW (doc-rot).
- **Dimension**: Exterior & Lighting Data. The parse-side doc is in `/audit-esm`'s path, but the data fact is Oblivion-only.
- **Location**:
  - `crates/plugin/src/esm/records/weather.rs:113-122`: "**Parse-but-don't-consume gate (TD5-010):** no renderer system reads `OblivionHdrLighting` fields yet".
  - `crates/plugin/src/esm/records/weather.rs:160`: "Wire size of the full 14-field HNAM payload (Oblivion / FO3 / FNV)". This contradicts the same file's `:111` "FNV and Fallout 3 do not ship HNAM at all".
- **Status**: NEW.
  - No match in today's reports.
  - `gh` search "OblivionHdrLighting" finds only the closed #537, #1045, #1057 and #1062.
- **Evidence**:
  - `df59c6362` (2026-09-18, "consume the Oblivion HNAM sunlight dimmer on exterior sun"). `env_translate.rs:1628-1630` translates `hdr.sunlight_dimmer` onto `WeatherDataRes`, and `systems/weather.rs:852-854` / `:929-931` multiply the sampled sun by it.
  - Raw census this run (`/tmp/audit/oblivion/wthr_hnam.py`): WTHR with HNAM is Oblivion 37 / 37, FalloutNV 0 / 63, Fallout3 0 / 27.
- **Impact**:
  - A reader trusting the gate doc would conclude that HNAM is inert and might move or retype it without checking the live consumer.
  - The `WIRE_SIZE` doc invites an FO3/FNV HNAM arm that no data supports.
  - The 13 other fields (eye-adapt, bloom, grass/tree dimmer) are still unconsumed, so the gate statement is only partly true.
- **Related**: #537 (closed), `df59c6362`, `sunlight_dimmer_translates_from_the_hnam_block`.
- **Suggested Fix**:
  - Restate the gate as: `sunlight_dimmer` consumed via `translate_weather` (EXAL); the other 13 fields are still parse-only.
  - Change `:160` to "Oblivion only (FO3/FNV author no HNAM: 0 / 27, 0 / 63)".

## Completeness Checks
- [ ] **SIBLING**: Same drift checked in sibling docs / comments / skill files
- [ ] **TESTS**: Where a number is restated, a test or measured source pins it


=== ISSUE #5365 ===
PERF: narrow the top-of-frame all-slots fence wait — GPU-bound frames drain the queue before the CPU records (#4606's throughput half)

**Successor of #4606** (closed on its doc half; the throughput defect stayed in the code). Filed from #5117 (REG-2026-09-29-01) so the plan is tracked instead of invisible.

**The defect** (`crates/renderer/src/vulkan/context/sync_and_acquire_frame.rs`): `draw_frame` opens with `wait_for_fences(&self.frame_sync.in_flight, …)` — every FIF slot, not just this frame's. On a GPU-bound frame the "other" fence belongs to frame N-1, the submission the GPU is still executing, so the wait drains the queue before the CPU records anything. The frame period becomes `max(T_pre, GPU) + T_post` instead of `max(T_pre + T_post, GPU)`; the engine keeps one frame in flight at this boundary, not two. Bench-of-record medians (`BENCH_stepped-camera_4c9a5b36`): fence_ms / wall_ms = 10.59/13.08 (Prospector TAA), 7.53/10.74 (Whiterun TAA), 16.12/37.64 (MedTek TAA) — the wait dominates. This is also the largest measured frame-time sink behind ROADMAP's fence-bound bench residual.

**The fix**: narrow to `in_flight[frame]` alone — but ONLY after every rider on the all-slots wait is migrated, because the all-slots argument is currently the *only* safety guarantee for resources that outlive one slot. The argument and the nine-site rider list are pinned by `the_all_slots_wait_argument_is_pinned` in `sync.rs` and the #870 const-assert; the pins must be re-derived, not deleted.

**Acceptance criteria** (from the #4606/#3442/#870/#3643/#4601 contract):
1. Every rider on the all-slots wait is per-FIF or defer-destroyed: the skinned BLAS scratch free, the depth-capture/screenshot staging destroys, the terrain tile buffer, mapped morph-weight writes, ground-cover model buffers/readback, skinned output/BLAS/bind-inverse staging + immediate unload frees, `images_in_flight`'s signaled-handle invariant (update both call sites in lockstep), and the SVGF temporal pass's previous-slot G-buffer read (#282) — which additionally needs the in-command-buffer barrier before its read stops depending on the wait.
2. The #870 const-assert and `the_all_slots_wait_argument_is_pinned` are updated in the same change that narrows the wait — never deleted ahead of it.
3. `BYRO_VALIDATION=1` clean on both upscaler modes (native TAA and FSR) per the speculative-Vulkan rule, on real content (Prospector or a cell scene).
4. Bench-of-record re-run (stepped-camera matrix, pinned `--camera-pos` per #5128): the fence_ms component drops from ~10-16 ms to the per-slot floor and R6a-regress-22's fence-bound residual retires; no SSIM/quality change on the upscaler matrix.

**Related**: #4606 (closed — doc half), #5117 (this issue's filer), #282 (in-buffer barrier), #3442, #870, #3643, #4601, #3645, R6a-regress-22.

=== ISSUE #5370 ===
FO4 Institute: dense per-pixel speckle over every bright surface at full light

User-reported 2026-10-08, live engine, current build (exact commit not recorded; HEAD at report time was `00f580e09`). The user re-checked and reports it is still present.

## What it looks like

In an Institute interior (the elevator/corridor area; the cell EDID was not recorded), every bright white surface is covered with dense per-pixel speckle at full light: walls, ceiling soffit, door frame and floor. It is not limited to shadowed or dim regions, unlike #5159.

- The speckle is mostly dark dots on white, densest on the large lit wall planes and the recessed ceiling panel.
- The floor shows heavy salt-and-pepper grain.
- The scene seen through the door window is speckled too.
- Two captures of the same view show a different speckle pattern, so the noise changes between frames rather than being a fixed pattern.

Screenshots (local): `/mnt/data/tmp/noise-issue/1.png`, `/mnt/data/tmp/noise-issue/2.png`.

The same failure appears in a reduced form in the game-data-free Cornell box (filed alongside, linked below). That issue is the reproducer to work from. The Institute is the worst case because it combines bright, high-albedo white surfaces, many lights per pixel, and a lot of glass/transmissive material.

## Leads (none verified)

- **Direct-light / stochastic-shadow convergence.** The ReSTIR finalize's temporal shadow-ratio accumulator in `crates/renderer/shaders/triangle.frag` (~3443-3560). If the history is rejected every frame, each pixel shows one frame's binary visibility for its picked light. On a multi-light white room that reads as dark dots wherever the pick landed on an occluded light. See the Cornell issue for the gate list and discriminators.
- **The #5249 transmission shadow trace** (`34c3adf14`, 2026-10-07; compile fix `ebf1d0492` / #5368). It adds a closest-hit shadow loop of up to 8 hops to every transmission-lobe fragment's ReSTIR finalize. Between those two commits `triangle.frag` did not compile and the committed `.spv` was stale, so the build's commit matters. From the 2026-10-08 audit suite:
  - REN-D10-2026-10-08-01: the trace passes an opaque-only mask to its foreign-hit handoff.
  - PERF-D5-2026-10-08-01: the loop's cost, in the main-render hotspot.
- **CPU-side light identities / reservoir light remap** (`c705c310d`, #5055/#4784, 2026-10-02). Many lights make a broken remap far more visible than in the 2-light Cornell box.
- **Not the cause on its own:** #5211 (the SVGF nearest-tap fallback re-accepts a NaN/Inf history texel). That leaves isolated stuck pixels, not a whole-frame pattern.

## Triage

1. Record the build commit and the cell EDID (`cell.info` or equivalent over byro-dbg).
2. Run `render.debug direct_only`, `indirect_only`, `shadow_visibility` and `restir_light` on this view. Note which ones keep the speckle.
3. Launch-time A/B via `BYROREDUX_RENDER_DEBUG`: `DBG_DISABLE_TEMPORAL` (0x40000) and `DBG_DISABLE_RESTIR` (0x8000).
4. If step 2 isolates the direct term, work the Cornell reproducer first. If it does not, check transmission/glass on this cell specifically (`render.debug material_lobe`).

No fix is proposed until a capture confirms the mechanism (no speculative render-path changes).

## Related

- Cornell reproducer: #5369
- #5159 (terrain dark-patch MC noise, possibly the same family)
- #5211 (SVGF NaN history re-accept)


=== ISSUE #5403 ===
SAVE-D2-2026-10-08-01: the shape-fingerprint guard reads only the last `#[…]` attribute before a declaration, so `StoryManagerNodeState` and `SmNodeRuntime` (save derive placed above a plain `#[derive(Debug…)]`) are invisible to it

**Source**: `docs/audits/AUDIT_SAVE_2026-10-08.md` — `SAVE-D2-2026-10-08-01` (HEAD `00f580e09`)

**Publish note**: Validated at HEAD: `normalized_serialized_shapes` uses `prior.rfind("#[")`; both SM types put the save `cfg_attr` above a plain `#[derive(Debug, …)]`.

- **Severity**: MEDIUM
- **Dimension**: Format & Schema Discipline
- **Data-Loss Class**: irrecoverable-write (latent). A future shape change without a bump makes same-major saves fail the typed preflight.
- **Location**:
  - `byroredux/src/save_io/serde_default_guard_tests.rs:241-253` (`prior.rfind("#[")`, then the derive/Serialize test on that one span);
  - `crates/scripting/src/story_manager.rs:161-181` (`#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]` followed by `#[derive(Debug, Clone, Default, PartialEq)]` on both types).
- **Status**: NEW. It is a different mechanism from #5059: that issue is about files outside the scan, while this one is about types inside a scanned file. Same family as the closed #3164.
- **Description**:
  - `normalized_serialized_shapes` takes the *nearest* `#[` above each `struct`/`enum` line and requires that one span to contain both `derive` and a Serialize needle.
  - For both new Story Manager types, the nearest span is the plain `#[derive(Debug, Clone, Default, PartialEq)]`, so they are skipped. `story_manager.rs` is in `save_type_sources()` (it carries a `cfg_attr(feature = "save"` derive and defines a registered type), but it contributes no shape.
  - Empirical confirmation: `26b6a779c` added two serde-derived types to a scanned file and did not refresh `BASELINE_SHAPE_FINGERPRINT`, yet the guard stayed green. Compare `14cff35ae`: `DialogueSpokenInfoForms` has the derives in the other order, and that commit had to refresh.
  - A Python emulation of the guard's attribute test over every workspace `.rs` file finds these two as the only in-scan saved types with the blind ordering. The other hits are sdk and inspect-only types outside the scan, the known #5059 territory.
- **Impact**: a field added to, retyped in, or newly `Option`-wrapped inside `SmNodeRuntime` or `StoryManagerNodeState` passes every guard without a `FORMAT_MAJOR` bump. The `serde(default)` guard would still catch a defaulted field. Saves of the same major then fail `validate_snapshot_types` with a decode error instead of a clean version refusal. Any future type that copies this attribute order joins the blind spot silently.
- **Suggested Fix**:
  - Treat the whole contiguous attribute block above the declaration as `between`, not just the last `#[`.
  - Add a coverage assertion: every registered type defined in a scanned file must contribute a shape.
  - Refresh the baseline in the same commit, as a refresh without a bump with a justification comment. The two types join the hash and have not changed shape since registration.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (the `serde_default` guard's own attribute scan, which may share the nearest-`#[` heuristic)
- [ ] **TESTS**: A regression test pins this specific fix


=== ISSUE #5406 ===
SCR-D5-2026-10-08-04: Phase B ignores `start_quest`'s refusal. Completed and failed radiants stay "eligible" forever, are logged as started, consume pool marks and rewrite event fills, and stopped quests resume without Skyrim's start-reset

**Source**: `docs/audits/AUDIT_SCRIPTING_2026-10-08.md` — `SCR-D5-2026-10-08-04` (HEAD `00f580e09`)

**Publish note**: Validated at HEAD: phase B calls `stages.start_quest(..)` and discards the result (`story_manager.rs` ~490); `start_quest` returns `None` unless status is `Stopped`.

- **Severity**: MEDIUM
- **Dimension**: Scene/Package/Dialogue (Story Manager); quest lifecycle touchpoint Dim 2
- **Untrusted-Input**: No
- **Location**:
  - `crates/scripting/src/story_manager.rs:486-524`: `stages.start_quest(..)`'s `Option` return is discarded. The
    fired/last-fire marks, the `started_event_fills` push and the "started quest" `info!` all follow unconditionally.
  - `:712-727`: eligibility is `!is_running`.
  - `crates/scripting/src/quest_stages.rs:172-195`: `start_quest` returns `None` unless the status is `Stopped`.
  - `crates/scripting/src/quest_stages.rs:240-254`: completed and failed quests also cannot be stopped.
- **Status**: NEW
- **Description**:
  - **Completed/Failed quests.** Phase 3 moved eligibility from `is_started` to `is_running` so radiants can re-fire.
    But a quest that reached a QSDT Complete/Fail stage is neither running nor startable. In a stacked pool it is
    `preferred[0]` on every event, so later entries never start. In a random or do-all pool it periodically wins the
    pick and wastes the event. Each time, phase B logs "started quest …", marks it fired, and overwrites its
    `StoryEventAliasFill`.
  - **Stopped quests.** These resume with their old `stages_done` and stage. The CK *Quest Data Tab*
    (`ck-uesp-wiki/(main)/Quest Data Tab.wiki`) says "Run Once: Prevents the Quest from being reset when it starts",
    which means a non-Run-Once quest **is** reset on start. A re-fired radiant therefore keeps its previous run's history,
    and `GetStageDone` gates plus non-repeatable `SetStage`s see a finished run.
- **Impact**:
  - Radiant pools jam once an entry completes.
  - The `sm1` attribution grep (`started quest … via node …`) can pass on a start that never happened.
  - Reruns of stopped radiants are semantically wrong.
- **Related**: SCR-D5-2026-10-08-02. The consume and start decisions should use the real start result.
- **Suggested Fix**: treat `start_quest == None` as "not started": no marks, no fill, no log, and no consume. For SM
  starts of non-Run-Once quests (QUST `DNAM` flag), reset the quest (`QuestStageState::reset` + start) so that completed
  and failed radiants can rerun. Pin both behaviours.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (other `start_quest` callers that ignore the `Option` return)
- [ ] **LOCK_ORDER**: If a RwLock scope changes, TypeId-sorted acquisition is preserved
- [ ] **TESTS**: A regression test pins this specific fix


=== ISSUE #5407 ===
SCR-D1-2026-10-08-01: `AddRaceSpells`/`RemoveRaceSpells` lower a *script-defined* helper by name and discard any receiver, so `x.AddRaceSpells()` on any object lowers onto the player

**Source**: `docs/audits/AUDIT_SCRIPTING_2026-10-08.md` — `SCR-D1-2026-10-08-01` (HEAD `00f580e09`)

**Publish note**: Validated at HEAD: `prim_race_spells_arity_ok` accepts both `Ident` and `MemberAccess` callees and `lower_race_spells` always emits `ActorRef::Player`.

- **Severity**: MEDIUM. This is latent: the skill escalation (emit on an unmodeled term) would make it HIGH, but the
  runtime is currently a no-op on the player, per CHAR-2026-10-08-D4-02.
- **Dimension**: Recognizer Chain
- **Untrusted-Input**: Yes (`.pex` from possibly-modded archives)
- **Location**: `crates/scripting/src/translate/effects.rs:1444-1495` (`lower_race_spells`,
  `prim_race_spells_arity_ok`, both `prim_*`).
- **Status**: NEW. CHAR-2026-10-08-D4-02 covers the runtime *target* (the player never receives `RaceSpells`). This
  finding covers the lowering contract.
- **Description**: neither name is an `actor.psc` native. They are functions defined on MQ101QuestScript. The primitive
  lowers any unqualified call by name, without checking that the calling script's own definition is the vanilla body. It
  also accepts any `MemberAccess` receiver ("The receiver is NOT distinguished") and always emits `ActorRef::Player`. A
  mod script that defines its own `AddRaceSpells()` with other semantics, or calls `SomeAlias.AddRaceSpells()` on
  another script type, is lowered as a re-apply/clear of the player's racial spells.
- **Impact**: none today. Once CHAR-D4-02 stamps the player's `RaceSpells`, a mod's same-named helper would add or remove
  the player's racial spells and their constant modifiers.
- **Related**: CHAR-2026-10-08-D4-02, #4415.
- **Suggested Fix**: accept only the unqualified spelling, and only when the calling script defines that function
  argless (the `ScriptSource` has the function table). Decline every member-call spelling.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (other primitives that lower a script-defined (non-native) helper by name)
- [ ] **TESTS**: A regression test pins this specific fix


=== ISSUE #5408 ===
SKY-D3-2026-10-08-01: #5095's FaceGen-miss head fallback loads only the NPC's own `PNAM` parts — the vanilla Skyrim player gets a hair mesh with no face, mouth or eyes, because RACE base head parts and HDPT extra parts are never decoded

**Source**: `docs/audits/AUDIT_SKYRIM_2026-10-08.md` — `SKY-D3-2026-10-08-01` (HEAD `00f580e09`)

**Publish note**: Validated at HEAD: `prebaked_head_fallback_paths` still maps only `npc.face_morphs.head_parts` (PNAM); `parse_hdpt` reads only DATA[0] into `flags` (no PNAM/HNAM); `RaceRecord.head_parts` doc still says "empty on Skyrim+"; `p3-player-body.sh` still gates on `head_parts > 0`. Not a duplicate of #5095 (closed) — incomplete fix, filed fresh.

- **Severity**: MEDIUM
- **Dimension**: 3 — NPC equip + FaceGen
- **Location**:
  - `byroredux/src/npc_spawn/resumable/prebaked.rs:174-185`: `prebaked_head_fallback_paths` reads PNAM only.
  - `:296-357`: the `HeadParts` phase.
  - `crates/plugin/src/esm/records/actor/race.rs:55-66`: the doc says "empty on Skyrim+, which moved head parts out to standalone `HDPT` records". The head section's `HEAD` sub-record has no arm; only `MODL` is collected.
  - `crates/plugin/src/esm/records/misc/character.rs:12-21`: `HdptRecord` has no `PNAM` type and no `HNAM` extras. Its flags doc is wrong.
  - `byroredux/src/player_body.rs:636`: the `head_parts` count.
  - `docs/smoke-tests/p3-player-body.sh`: the `head_parts > 0` gate.
- **Status**: NEW. #5095 (CLOSED 2026-10-06) is an incomplete fix, not a regression: its fallback never covered race base parts. No sibling report or issue matches. `gh` "head part race default" across all states finds only #5095, #591 and #967.
- **Description**:
  - **The fix's premise.** The commit and the code doc rest on two claims: "Skyrim RACEs carry no head-part table at all", and the player's authored PNAM list is "ManHead / eyes / hair / brows".
  - **Both claims are false on the shipped data.**
    - A TES5 RACE carries per-gender **Head Data**: a `NAM0` marker, then `MNAM` / `FNAM`, then `wbHeadParts` = `INDX` + `HEAD` → HDPT (xEdit `wbDefinitionsTES5.pas:9282`, `:9470-9497`). The CK Race page lists these as "Base Head Parts", and the CK Character Gen Parts tab lists the base head parts as Face, Eyes, Hair, Facial Hair and Eyebrows.
    - The vanilla player record authors a single hair part.
    - The parser decodes no RACE `HEAD`, no HDPT `PNAM` type and no HDPT `HNAM` "Extra Parts" (xEdit `:5296-5309`).
  - **Result.** The fallback loads only what PNAM names. For the player that is one hair NIF, so the third-person player still has no face.
- **Evidence**: byte walk of the installed SE masters.
  - **Player record.** `Skyrim.esm` NPC_ `0x00000007` (`Player`, ACBS `0x30`, male, race `NordRace` `0x13746`) has PNAM = `[0x051507 HairMaleNord01]` (type 3 Hair, `Hair\Male\Hair01.nif`, HNAM extra `HairLineMaleNord01`). No DLC overrides the player record.
  - **Race defaults.** `Update.esm` overrides NordRace.
    - Male base head parts: `HairMaleNord10` (Hair), `MaleHeadNord` (Face), `MaleMouthHumanoidDefault` (Misc), `MaleEyesHumanLightBlue` (Eyes).
    - Female base head parts: 5, including `FemaleBrowsHuman01`.
    - In `Skyrim.esm`, 32 of 99 RACEs author head parts.
  - **What the fix itself reported.** Its live check reports `head_parts=1 (meshes 8→9)`, and that single part is the hair.
  - **Extra parts.** 230 of 766 HDPTs author `HNAM` extras (271 refs). Of the 2,912 NPC_ records that author PNAM, 2,443 reference an extra part that is absent from their own PNAM list. Every facegeom-less NPC (modded NPCs, any record without baked facegeom) loses those parts too.
  - **Why the gate misses it.** `p3-player-body.sh` fails only on `head_parts == 0`. The count is a world-wide `PrebakedHeadPart` query, not one scoped to the player root, so a hair-only player passes.
  - **The flags doc is wrong.** `HdptRecord.flags` says Skyrim "bits 0-2 encode the head-part type slot". xEdit has DATA = Playable / Male / Female / Is Extra Part / Use Solid Tint, and the type is the separate `PNAM` u32.
- **Impact**:
  - **The player.** On every Skyrim session, the third-person player shows a floating hair shell over an empty neck: no face, eyes, mouth or hairline. This is the case #5095 was filed to fix, and the issue is now closed.
  - **Other NPCs.** Any facegeom-less humanoid NPC gets the same partial head.
  - **Detection.** The fallback is silent and the gate stays green.
- **Related**: #5095 (closed, incomplete). The CHARACTER 2026-10-08 note (the fallback reads the shell's PNAM, not the TPLT terminal) is a separate axis. Also related: #3409 (hidden-partition handling, reused correctly) and #5359 (NPC_ `WNAM`, same "Skyrim body source" family).
- **Suggested Fix**:
  1. Decode RACE Head Data `HEAD` per gender section into `RaceRecord`, and decode HDPT `PNAM` (type) and `HNAM` (extras).
  2. Build the fallback list from the race's base parts for the NPC's gender. For each type the NPC's PNAM authors, use the NPC's part instead. Then expand `HNAM` extras recursively, with a cycle guard.
  3. Source the exact per-type override rule before coding it (for example SKSE's `TESNPC::GetCurrentHeadPartByType` race fallback). Do not guess it.
  4. Tighten the P3 gate to require a Face-type part on the player root, and fix the `race.rs` / `character.rs` docs.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (FO4 RACE head data / HDPT extras on the FO4 prebaked-FaceGen path; the TPLT-terminal PNAM axis noted by AUDIT_CHARACTER_2026-10-08)
- [ ] **TESTS**: A regression test pins this specific fix (player root carries a Face-type part; HNAM extras expanded with a cycle guard)


=== ISSUE #5437 ===
NIFAL-D1-2026-10-08-01: The Phase-2 resolver table describes `resolve_unresolved_gloss_neutral_roughness`'s trigger backwards, in both the module doc and `nifal.md`

**Source**: `docs/audits/AUDIT_NIFAL_2026-10-08.md` — `NIFAL-D1-2026-10-08-01` (HEAD `00f580e09`)

- **Severity**: LOW · **Dimension**: Material · **Tier Violated**: doc / record-keeping · **Game Affected**: FO4 / FO76 (BGSM
  content)
- **Location**: `byroredux/src/material_translate.rs:42` and `docs/engine/nifal.md:797`. The predicate itself is at
  `material_translate.rs:1409-1433`.
- **Status**: NEW. The text was introduced by `c9b02ba4a` on 2026-09-12. It is not covered by #4246 (the `placement_lod`
  exemption rationale) or by #5279 (the "backstop" wording).
- **Description**: Both table rows say the resolver writes `Material::roughness` "when a gloss/smoothness map **resolved** with
  **no authored BGSM PBR scalars** to interpret it". The code does the opposite on both counts:
  - It returns `None` unless `bgsm_pbr_scalars_authored` is true.
  - It returns `None` unless `gloss_map_index == 0`, meaning no gloss map resolved.
  - It acts only when roughness is at or below the near-mirror floor.

  `nifal.md:823-824` states the gate correctly ("only acts on authored BGSM PBR scalars"), so the spec contradicts itself 26 lines
  apart. The function's own rustdoc (`:1383-1386`) is also correct.
- **Evidence**: `unresolved_gloss_neutral_roughness` contains the line `if !bgsm_pbr_scalars_authored { return None; }`, then
  `if gloss_map_index != 0 { return None; }`, then the floor test.
- **Impact**: A reader of the table would scope the resolver to legacy, non-BGSM content with a working gloss map. That is
  exactly the population it must never touch. Widening the gate to match the table would neutralise legacy roughness.
- **Related**: #3905, #3639, #4246, #5230.
- **Suggested Fix**: Reword both rows to: "when authored BGSM PBR scalars sit at the near-mirror floor and no gloss map
  *resolved* to modulate them (#3905; forced mirror panes exempt, #5230)". Fold this into the #4246 / #5279 doc pass.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (`nifal.md` Phase-2 table + the `material_translate.rs` module-doc table (fold into #4246 / #5279 doc pass))
- [ ] **CANONICAL-BOUNDARY**: If the fix touches `byroredux/src/material_translate.rs` (`translate_material`), `Material::resolve_pbr` (`crates/core/src/ecs/components/material.rs`), or the material merge path, per-game logic stays at the NIFAL parser→`Material` boundary — never pushed into shaders/renderer, never re-derived at render time. See `/audit-nifal`.


