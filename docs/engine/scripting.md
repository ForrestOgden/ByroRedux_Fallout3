# Scripting Architecture: From Papyrus VM to ECS-Native

> **Status (verified 2026-08-25).** The ECS-native scripting model is now a
> production subsystem rather than only the R5 design proof. M47.0/M47.1 and
> the `.psc` parser remain shipped; the runtime now also decodes `.pex`, lowers
> and dispatches QUST and SCEN fragments, resolves quest aliases and VMAD
> properties, persists fragment continuations, globals, quest progress and
> reference-enable state, and applies a growing conservative `Effect` catalog.
> The live code lives in [`crates/scripting/`](../../crates/scripting/) and
> [`crates/papyrus/`](../../crates/papyrus/). Sections marked **DESIGN** are
> still forward-looking; sections marked **SHIPPED** describe code in the tree
> today. See [Current Implementation](#current-implementation-shipped) for the
> ground-truth map of what exists.

## Three Generations of Bethesda Scripting

### ObScript (Morrowind → Fallout: New Vegas)

The original scripting system ran synchronously in the game loop:

- **GameMode block** executed every frame — any timed behavior required manual
  `GetSecondsPassed()` tracking and state variables
- No user-defined functions, no loops, no states — just sequential IF/ELSE
- `set variable to 25` syntax, `Begin OnTriggerEnter Player` events
- Single-threaded: script execution directly affected framerate
- All variables public, no encapsulation

ObScript was simple but limited. Complex behavior required dozens of tracking
variables and deeply nested conditionals. The per-frame execution model meant
heavy scripts visibly dropped FPS.

### Papyrus (Skyrim → Fallout 4)

Papyrus introduced modern language features but moved scripts into a VM:

- User-defined functions, While loops, states, latent functions (Wait, MoveTo)
- Properties with editor configuration and optional get/set validation
- Multithreaded VM — scripts time-sliced, don't directly affect framerate
- 136 events (Fallout 4), type-safe compilation, script inheritance

But the VM architecture created new problems: *"Telling a script to wait for
0.5 seconds really means 0.5 seconds plus the time it takes for the script to
get its number called."* Under load, this queue delay grows to seconds.
Multiple instances of the same event can run simultaneously (e.g., two
OnTriggerEnter events overlapping), requiring States or control variables.

### ByroRedux (this engine)

Takes the best of both: **ObScript's deterministic synchronous execution** with
**Papyrus's modern features**, implemented as ECS components and systems. No VM,
no event queue, no separate threading model. Scripts run in the ECS scheduler
like any other system — deterministic timing, zero overhead for idle scripts,
and component data is the only state that exists.

This is no longer purely a design claim. The **R5 prototype** hand-translated
`defaultRumbleOnActivate.psc` — a real 50-LOC Skyrim script attached to
hundreds of vanilla references (pressure plates, ritual buttons, shrines) —
into plain ECS components + dt-driven systems, and validated that all three
hard cases (latent `Utility.Wait()`, multi-state dispatch, cross-subsystem
call) translate cleanly with no VM, no fibre, no suspendable script frame. The
load-bearing finding: **a Papyrus event handler with a latent wait splits into
two systems** — code-before-`Wait` runs on the event, code-after-`Wait` runs on
whichever frame the dt counter hits zero. That is the entire pattern. The
verdict was *go ECS-native*; full evaluation at
[`docs/r5-evaluation.md`](../r5-evaluation.md), reference fixtures at
[`docs/r5/source/`](../r5/source/).

| | ObScript | Papyrus | ByroRedux |
|---|---|---|---|
| Execution | Synchronous, per-frame | Async VM, time-sliced | Synchronous ECS scheduler |
| Framerate impact | Direct | None (but script lag) | None (queries skip empty) |
| Functions | No | Yes | Yes (Rust functions) |
| Loops | No | While | Rust loops |
| States | No | GoToState | Enum component field |
| Save safety | Variables only | Stacks + properties (fragile) | Components only (robust) |
| Timing | Deterministic | Non-deterministic (queue) | Deterministic |
| Mod conflicts | Variable collision | Script-level override | Per-component, auto-merge |

---

## Current Implementation (SHIPPED)

The scripting crate ([`crates/scripting/`](../../crates/scripting/)) is the
ECS-native event runtime. It registers its storages via `byroredux_scripting::register(&mut world)`
([`crates/scripting/src/lib.rs`](../../crates/scripting/src/lib.rs)), called
once during engine init. Test counts change quickly; the authoritative
workspace total is maintained in [ROADMAP.md](../../ROADMAP.md#project-stats).

### Module map

| Module | File | Role |
|---|---|---|
| Event markers | [`events.rs`](../../crates/scripting/src/events.rs) | Transient marker components (the event types) |
| Timers | [`timer.rs`](../../crates/scripting/src/timer.rs) | `ScriptTimer` + `timer_tick_system` |
| Recurring updates | [`recurring_update.rs`](../../crates/scripting/src/recurring_update.rs) | `RecurringUpdate` + `OnUpdateEvent` + `recurring_update_tick_system` |
| Cleanup | [`cleanup.rs`](../../crates/scripting/src/cleanup.rs) | `event_cleanup_system` — end-of-frame marker sweep |
| Quest stages | [`quest_stages.rs`](../../crates/scripting/src/quest_stages.rs) | `QuestStageState` resource + `QuestStageAdvanced` event |
| Conditions | [`condition.rs`](../../crates/scripting/src/condition.rs) | `ConditionFunction` + OR-precedence `evaluate` (M47.1) |
| Script registry | [`registry.rs`](../../crates/scripting/src/registry.rs) | `ScriptRegistry` — editor_id → spawn-fn map (M47.0) |
| Fragment runtime | [`fragment.rs`](../../crates/scripting/src/fragment.rs) | QUST/SCEN fragment population, cascade dispatch, deferred effects, enable state |
| Fragment lowering | [`translate/`](../../crates/scripting/src/translate/) | Conservative `.pex`/AST recognizers and canonical `Effect` lowering |
| Globals | [`globals.rs`](../../crates/scripting/src/globals.rs) | Mutable GLOB values consumed by conditions and `SetValue` effects |
| Triggers and packages | [`trigger.rs`](../../crates/scripting/src/trigger.rs), [`package.rs`](../../crates/scripting/src/package.rs) | Trigger-enter detection and runtime package re-evaluation |
| Perk entry points | [`perk_entry_points.rs`](../../crates/scripting/src/perk_entry_points.rs) | `PerkEntryTable` + `compose_entry_point` — composes an actor's ranked perks into a calculation hook (M47.2) |
| Locations | [`location.rs`](../../crates/scripting/src/location.rs) | `LocationTable` (LCTN hierarchy) + current / editor location + `LocationAliasBindings` (M47.2) |
| Scenes | [`scene/`](../../crates/scripting/src/scene/) | SCEN population, phase playback, actor binding and phase fragments |
| Papyrus demos | [`papyrus_demo/`](../../crates/scripting/src/papyrus_demo/) | Hand-translated R5 scripts (the "go ECS-native" proof) |

### The frame lifecycle (M47.0)

The scheduler in [`byroredux/src/main.rs`](../../byroredux/src/main.rs) wires the
scripting systems in this order:

1. **Tick systems** (Update stage): `timer_tick_system`,
   `recurring_update_tick_system` count down their components and emit
   `TimerExpired` / `OnUpdateEvent` markers.
2. **Handler systems** (exclusive, Update stage): the `papyrus_demo` dispatchers
   — `rumble_on_activate_system`, `quest_advance_system`,
   `dlc2_ttr4a_on_init_system`, `dlc2_ttr4a_on_update_system`,
   `mg07_on_load_system`, `mg07_on_activate_system` — plus the `rumble_tick_system`
   and `mg07_tick_system` continuations. They read event markers + script-state
   components and produce effects.
3. **Cleanup** (`event_cleanup_system`, exclusive, Late stage, registered last):
   drains every transient marker so each event is visible for exactly one frame.

The core lifecycle is the ECS replacement for the Papyrus event queue: **a marker
component appears → handler systems process it the same frame → cleanup removes
it at end of frame.** No enqueue, no dispatch table, no scheduling latency.

### Shipped event markers

`events.rs` defines the live marker catalog. Note: these are the **actual
shipped names and fields** — they differ from the speculative table further down
in the design narrative.

| Marker | Fields | Replaces | Emit site |
|---|---|---|---|
| `ActivateEvent` | `activator: EntityId` | `OnActivate` | shared interaction path used by gameplay input and diagnostics |
| `HitEvent` | `aggressor`, `source`, `projectile: EntityId` + damage/attack flags | `OnHit` | combat runtime |
| `EquipmentEventBatch` | `Vec<EquipmentChange>` (`item_form_id`, `equipped`), keyed by the wearer entity | actor `OnObjectEquipped` / `OnObjectUnequipped` | **live** — [`emit_equipment_changes`](../../crates/scripting/src/equipment.rs) appends to the wearer's one-frame batch (`get_mut`-then-`extend`); callers are [`byroredux/src/inventory.rs`](../../byroredux/src/inventory.rs) and the `EquipItem` fragment effect |
| `TimerExpired` | `timer_id: u32` | `OnTimer` | `timer_tick_system` |
| `AnimationTextKeyEvents` | `Vec<AnimationTextKeyEvent { label: FixedString, time: f32 }>` | KF text keys | `byroredux::systems::animation` (live — fires on every clip) |
| `OnUpdateEvent` | (unit) | `OnUpdate` | `recurring_update_tick_system` |
| `OnInitEvent` | (unit) | `OnInit` for translated provider handlers | `attach_papyrus_provider_program`; drained after one frame |
| `OnCellLoadEvent` | (unit) | `OnLoad` / `OnCellLoad` | **live** — cell loader's `attach_script_for_refr` ([`byroredux/src/cell_loader/references/attach.rs`](../../byroredux/src/cell_loader/references/attach.rs)) |
| `OnTriggerEnterEvent` | triggerer set | `OnTriggerEnter` (Skyrim+) / `OnTrigger` (FO3/FNV) | `trigger_detection_system` over live trigger volumes |
| `QuestStageAdvanced` | `quest: QuestFormId`, `previous_stage`, `new_stage: u16` | quest stage-advance fragments | consumed by `quest_fragment_dispatch_system`, including bounded cascades |

The marker pattern is zero-cost for entities that don't participate — no
registration list. `AnimationTextKeyEvents` is the most exercised marker today:
the animation system fires it on every clip whose `.kf` carries text keys (`hit`,
`sound: wpn_swing`, `FootLeft`, …). The `label` is an interned
`FixedString` (#231 / SI-04) — resolve via the world's `StringPool`.

### Timers and recurring updates

- **`ScriptTimer { id, remaining }`** — one-shot countdown. `timer_tick_system`
  decrements `remaining` by `dt`; at ≤ 0 it removes the timer and inserts a
  `TimerExpired { timer_id: id }` marker on the same entity. This is the ECS
  replacement for Papyrus `StartTimer(time, id)` → `OnTimer(id)`.
- **`RecurringUpdate { interval_secs, seconds_until_next }`** — periodic
  subscription, the substrate for Papyrus's `RegisterForUpdate(N)` /
  `UnregisterForUpdate()` / `Event OnUpdate()` triad. Construct via
  `RecurringUpdate::every(interval_secs)` (first fire is `N` seconds out, matching
  Papyrus). `recurring_update_tick_system` emits `OnUpdateEvent` when the counter
  crosses zero and **accumulates** `interval_secs` rather than resetting, so a
  long-frame `dt` still fires only once — the same "missed fire drops" contract
  the Papyrus runtime documents. Cancellation is simply removing the component
  (including from inside an `OnUpdate` handler — the fire-once-then-cancel idiom).

### Quest stage runtime

`QuestStageState` is a **resource** (global game state, not bound to a cell or
actor), keyed by `QuestFormId(u32)`. It exposes the Papyrus surface:

- `set_stage(quest, stage) -> prev` — Papyrus `SetStage`. Advances
  `current_stage` and records `stage` in a `stages_done: HashSet<u16>`.
- `get_stage(quest) -> u16` — Papyrus `GetStage` (default 0 for untouched quests).
- `get_stage_done(quest, stage) -> bool` — Papyrus `GetStageDone(N)`. Returns
  true for any stage the quest *ever* passed through, even after advancing past
  it. The DA10 translation surfaced that `GetStageDone(37) && !GetStageDone(40)`
  needs per-stage history, not a single `current_stage` — Bethesda's runtime
  carries the same shape.
- `reset(quest)` — Papyrus `Quest.Reset()`.

Entries are lazy: a quest only gets a map entry on first mutation, keeping the
resource proportional to "quests touched" rather than "every quest in every
plugin". Stage-fragment dispatch and objective state are now live through
`QuestStageFragments`, `QuestObjectiveState`, and
`quest_fragment_dispatch_system`; the dispatcher queues cascaded stage changes
and caps them so cyclic authored content cannot loop forever in one frame.

### Condition evaluator (M47.1)

The universal predicate system (CTDA conditions) is split across two crates:

- **Parse side** — `byroredux_plugin::esm::records::condition`
  ([`crates/plugin/src/esm/records/condition.rs`](../../crates/plugin/src/esm/records/condition.rs))
  parses CTDA sub-records (28-byte FO3/FNV + 32-byte Skyrim+) into
  `Condition { function_index, comparator, comparand, param_1, param_2, run_on,
  reference_form_id, extra_data_id, or_next }`. Supporting types: `ComparisonOp`,
  `RunOn`, `ConditionValue` (`Literal(f32)` | `Global(form_id)`).
- **Eval side** — `byroredux_scripting::condition`
  ([`crates/scripting/src/condition.rs`](../../crates/scripting/src/condition.rs))
  interprets function indices against ECS state and combines per-condition
  booleans with the **OR-precedence rule**.

**The OR-precedence quirk** (load-bearing, with regression tests): the default
operator between conditions is AND. Setting `or_next` combines a condition with
the *next* via OR, and **consecutive ORs form a block that binds tighter than
the surrounding AND chain.** So `A AND B OR C AND D` evaluates as
`A AND (B OR C) AND D`, *not* the standard `(A AND B) OR (C AND D)`. This is the
opposite of standard boolean precedence; Bethesda designers exploit it via the
distributive law. `evaluate(&list, world, &ctx)` walks the list grouping OR
blocks, short-circuits AND failures, and returns `true` for empty lists
("no conditions = always fires").

**Function catalog.** Bethesda ships hundreds of condition functions across
the lineage. The initial seven-function M47.1 catalog has grown to **19 modeled
variants** (plus `Unknown`): actor value/distance/death, quest stage/history,
cell/faction/level/equipment, XP-to-next-level, scene-action completion,
loaded-3D, class/race/base-ID, perks, FNV reputation and VM script variables.
Their per-game indices and exact ECS fallbacks are documented beside the
authoritative `ConditionFunction` enum in
[`condition.rs`](../../crates/scripting/src/condition.rs); keeping a second
index table here previously let the numbers drift.

Unknown indices fall to `ConditionFunction::Unknown(u32)` and evaluate to 0.0 —
Bethesda's "unknown function → safe-default" contract — trace-logged for catalog
tracking. `ConditionContext` resolves the abstract Run-On targets (`Subject` /
`Target` / `CombatTarget` / `LinkedReference`) to concrete `EntityId`s; missing
slots fail the predicate (Bethesda's "missing reference → false"). The first
real consumer is `quest_advance` (the DA10 demo), migrated in M47.1 Phase 4 from
bespoke `require_done`/`forbid_done` fields to a generic `ConditionList`.

### Perk entry points (M47.2)

A `PERK` entry-point entry modifies a hardcoded calculation (weapon damage,
experience, carry weight, …). `install_perk_entry_points` folds every PERK's
value-math entries into the `PerkEntryTable` resource at load-order install
(FO3/FNV and Skyrim; other games get an empty table), and a calculation asks
`compose_entry_point(world, owner, hook, base, tab_subjects)` for the value the
owner's perks make of `base`. The first consumer is `combat::attack_damage_against`
(FNV hook 0; Skyrim hooks 35 then 0) on both the player and NPC strike paths.

The second is New Vegas **Damage Threshold**. `DamageThresholdRules` installs
when the load order authors the `AVDamageThreshold` actor value (the GECK
*Stats List*: "the actor's base Damage Threshold level") and
`fMinDamMultiplier` — New Vegas; Fallout 3 authors neither. A hit's DT is the
target's DT actor value plus its equipped armors' `DNAM` DT. The target's
hook-56 perks compose it ("modifies the DT of the player" when hit), then the
attacker's hook-58 perks ("modifies the DT of the enemy"). The hit deals
`max(damage − DT, damage · fMinDamMultiplier)`, the GECK's floor on what DT
negates. Both hooks' tabs 1 and 2 are weapons, so weapon-conditioned entries
(Stonewall, Piercing Strike) wait on weapon entities as above. DR is not
applied: its order against DT is not documented in the references here. The
FO3/FNV ARMO `DNAM` decode was corrected to xEdit's layout on the way. It had
read the DR field as DT and the DT float as DR (FNV: `AR ×100, 2 unused, DT,
flags`; FO3: `AR, flags`).

The third is Fallout 3 / New Vegas **kill experience** (`ExperienceRules`,
from the GECK *Experience Settings* page). A kill pays the player the
`iXPRewardKillNPC*` / `iXPRewardKillOpponent*` of the difficulty band the
victim's level falls in (band tops `iXPLevelKill*`; above the last band,
"very hard"). It pays only if the player dealt at least
`iXPDeathRewardHealthThreshold` percent of the victim's health. The award is
composed by hook 9 (`Adjust Experience Points` — Swift Learner) and rolls
levels by the ruleset's `LevelingModel`, up to its cap. The masters author no
`iXPRewardKillNPC*`, so those take the engine defaults 0 / 10 / 20 / 30 / 50
(the GECK page, confirmed in `Fallout3.exe`'s static setting table).
`CharacterLevel` is saved since (v46), which retired #2947's
`validate_progression_state` guard.

Each level the player reaches (`grant_experience`, shared by kill experience
and the `levelup xp` console command) banks the ruleset's
`LevelReward::SkillPoints` in the saved `LevelUpPoints`. Fallout 3 grants
`10 + INT` skill points and a perk every level. New Vegas grants `10 + INT/2`
with the half point carried, and a perk every other level (Fallout wiki
*Skill point*; the GMSTs both masters author, `iLevelUpSkillPointsBase` 11 and
`Interval` 1, give FO3's rule). Points are composed by hook 10 (`Adjust
Gained Skill Points`: `Educated`, +3 in FO3 and +2 in FNV). With no level-up
menu yet, the `levelup` console command spends them: a skill rises one per
point, never past 100, and a perk point adds a perk rank (requirements are
not checked). Live (FNV, INT 5): 600 XP reached level 3 and banked 25 points
and one perk.

The fourth is **encumbrance** (`byroredux/src/encumbrance.rs`). The player's
carried weight is the inventory's item weights times counts (the inventory
menu's total). It is written to the `InventoryWeight` actor value every frame,
so conditions and scripts read it. Capacity is `CarryWeight` composed by Get
Max Carry Weight (FO3/FNV hook 22, Skyrim hook 10; neither master authors an
entry, so this is for mods). The GECK *Stats List*: "If InventoryWeight >
CarryWeight, the actor is over-encumbered". On FO3/FNV, movement speed is
then multiplied by `fMoveEncumEffect` (0.4, the GECK and Fallout3.exe
default). The holstered-weapon variant is not split out, since the player has
no holstered state yet. On Skyrim the player is "limited to walking" (UESP
*Carry Weight*), so sprint is refused. Oblivion's hard-coded encumbrance value
and FO4 / Starfield's effects are not modelled. Live (FNV Goodsprings): 21
chainsaws put the player at 420 of 200, and they moved slower afterwards.

The record layout and the composition rules are pinned to xEdit's
`wbDefinitionsFNV` / `wbDefinitionsTES5` and checked against the vanilla masters
(`cargo run --release -p byroredux-plugin --example perk_entry_census -- <ESM>`,
zero decode mismatches on FalloutNV / Fallout3 / Skyrim):

- **`PRKE`/DATA** is `[entry point, function, condition-tab count]`; **`EPFT`** is
  the *parameter type* (not the math function) and selects the **`EPFD`** shape
  (float, two floats, AV + float, leveled item, Skyrim spell / lstring). The
  actor value inside an AV-scaled `EPFD` is stored as a *float* index
  (`0x43080000` = 136), rounded by xEdit. The pre-M47.2 decoder let `EPFT`
  overwrite the function and indexed `EPFD` by the wrong table, so every vanilla
  `Float` entry decoded to nothing.
- **Rank.** `PRKE` ranks are zero-based and an entry is authored per rank it
  applies at: `DemolitionExpert` is ×1.2 / ×1.4 / ×1.6 at ranks 0 / 1 / 2, each a
  *total*. An actor holding (one-based) rank `N` gets only the `rank == N - 1`
  entries.
- **Functions.** 1 Set, 2 Add, 3 Multiply, 4 Add Range, 5 Add AV Mult, 6
  Absolute, 7 Negative Absolute, and on Skyrim 12 Set to / 13 Multiply / 14
  Multiply 1 + AV Mult. On FNV hooks 53 / 56 / 58 function 1 reads "Subtract
  Value" in the GECK and subtracts (`WeaponHandling`, `PiercingStrike`).
- **Condition tabs** (`PRKC`): tab 0 is the perk owner, the rest are subjects the
  calculation supplies. A conditioned tab with no supplied subject, or a
  condition the evaluator does not model, keeps the entry from firing — the
  engine cannot say the weapon is a one-hander, and guessing would give every
  weapon the bonus. Equipped weapons are not entities today, so weapon-tab
  entries (e.g. `DemolitionExpert`) are inert until they are.
- **Priority** composes ascending (ties: the actor's perk order, then the
  record's entry order). The two CKs disagree on which number "takes
  precedence"; no vanilla FO3/FNV entry has a non-zero priority, and on Skyrim
  the only visible case is a `Set Value` at a high number (`EagleEye30`), which
  only makes sense applied last.

### Quest script variables and the dialogue speaker (M47.2)

Fragments write quest-script *auto properties* (`kmyQuest.Flag = 1`,
`(Self.GetOwningQuest() as MS11QuestScript).GetInHjerimTovaPointer = true`),
and dialogue, package and scene conditions read them back through
`GetVMQuestVariable` (CTDA 629, Skyrim / FO4 — 3,319 conditions in Skyrim.esm,
774 in Fallout4.esm). `Effect::SetQuestVariable` records a literal write in
the saved `QuestScriptVariables` resource under the backing `::Name_var`
spelling the conditions use (`ctda_variable_census` example); the quest
`VMAD`'s authored Int / Float / Bool values live in the unsaved
`QuestScriptVariableDefaults` (re-derived every boot, so a save load cannot
wipe them). A computed value, a compound operator or a bare `Self` receiver
(a topic-info fragment's own script) declines.

Topic-info (`TIF_…`) fragments receive their speaker through the
`akSpeakerRef` parameter. `lower_info_fragment` binds it (and the
`akSpeaker = akSpeakerRef as Actor` local) to `ActorRef::Speaker`, which the
dialogue caller resolves to the NPC saying the line
(`apply_spoken_info_fragment(.., Some(npc))`). A speaker fragment with a
latent tail (a wait, a 3D-loaded poll, a provider barrier) declines: the
speaker is a live entity a saved continuation could not name.

A call into the fragment quest's own script (`kmyQuest.Startup()`) inlines
the callee's body when it takes no arguments. In a quest fragment the
fragment's VMAD is the quest's, so the effects splice in directly. From an
INFO or scene fragment (2026-10-13) the callee's properties live on the
quest's VMAD, not the fragment's. Its effects are wrapped in
`Effect::QuestScope`, which runs them against the quest and its own VMAD
(`QuestScriptInstances`, every quest's VMAD, snapshotted before the quest
guards). A scope never holds a latent effect. Inside an inlined body, writes
to the script's own variables lower too: a literal, `X = X ± c`, and
`X = Y` (`Effect::CopyQuestVariable`). FO4's radio rotation
(`kmyQuest.UpdateRadio()`, 437 INFO fragments) is the main beneficiary.

Fragment `If`s lower when their condition is built from `FragmentGuard`s
with `&&`, `||` (`FragmentGuard::Any`) and `!`. Negation is pushed to the
atoms by De Morgan. The atoms: `GetStageDone` / FO4 `IsStageDone`,
`IsRunning`, `IsCompleted`, `GetStage` compared with a literal,
`IsObjectiveDisplayed` / `IsObjectiveCompleted`, a global's `GetValue()` /
`GetValueInt()` / `.Value`, a quest-script variable, an actor's `IsDead`,
`HasPerk` and `GetItemCount`, a scene's `IsPlaying`,
`<actor>.GetCurrentLocation().HasKeyword(kw)` (the location runtime), and —
2026-10-13 — `IsDisabled` / `IsEnabled`, `IsInFaction`, `Is3DLoaded` and
`GetDistance(target)` against a literal. `IsDisabled` falls back to the
authored state by the cell loader's own rule (`AuthoredDisabledReferences`),
and a reference with no live entity reads as not loaded and infinitely far.
Each
compares with a literal on either side. A guard whose quest, global or actor
does not resolve declines the whole conditional at runtime; a quest-variable
guard sees a write made earlier in the same fragment. An actor's own
keywords are not modelled (`NPC_` keywords are not parsed).

`AddToFaction` / `RemoveFromFaction` / `SetFactionRank` (CK semantics: join
at rank 0 unless already a member; leave; set, joining if needed) lower to
`Effect::ChangeFaction`. `FactionRanks` is re-derived from the record at every
spawn and, for the player, on every load, so the edit is also recorded in the
saved `ScriptedFactionChanges` ledger by reference FormID (PlayerRef `0x14`)
and replayed by `reapply_scripted_faction_changes` from `stamp_quest_reference`
and the load path.

Quest-stage fragments inline calls into their quest's own script
(`kmyQuest.StartUp()`, `(Self.GetOwningQuest() as X).Reset()`): when the
receiver is the fragment's own quest and the callee takes no arguments, the
callee's decompiled body (fetched through a `ScriptLibrary` — the app's
`ProviderScriptLibrary` over `--scripts-bsa`) is lowered in its script's scope
and spliced in, transitively through `Self.Other()` calls, cycle-guarded and
four frames deep at most. Only quest fragments inline: their `VMAD` is the one
that carries the quest script's properties. A callee that does not fully lower
declines its caller, so this gains as the effect table grows (+52 fragments
today; most callees are long, bespoke bodies).

A fragment's `Game.GetPlayer().MoveTo(<marker>)` lowers to
`Effect::MovePlayerTo`: the runtime records the destination reference's FormID
in `PlayerMoveRequest`, and the app's `step_player_move_request` moves the
player in place when the reference is resident (camera, then the body grounded
under it) or queues the door-transition machinery to the reference's cell at
its placement. `AddItem` / `RemoveItem` counts may read a global
(`ItemCount::Global`, truncated toward zero when applied); `G.Mod(x)` and
`G.SetValue(S.GetValue() ± c)` lower to `Effect::SetGlobalRelative`, and so
do assignments to a global's `.Value` (directly or through the quest
script's property, `kmyQuest.SomeGlobal.Value`); a quest-variable counter
`kmyQuest.X = kmyQuest.X ± c` lowers to `Effect::ModQuestVariable`;
FO4's alias `TryToSetActorValue` joins `SetValue` as `Effect::SetActorValue`,
and Skyrim's `SetAV` / `SetActorValue("Name", v)` lower to
`Effect::SetActorValueNamed`. The name resolves through `ActorValueNames`,
the AVIF editor IDs with their `AV` prefix stripped (`AVVariable06`); a
legacy editor ID such as Illusion's `AVMysticism` does not resolve, and that
write skips. `AddPerk` / `RemovePerk` edit the actor's `Perks` (parked with the rest of an
evicted actor's state); and the FO4 / Starfield spellings `EnableNoWait`,
`DisableNoWait`, `TryToEnable`, `TryToDisable` and `Unlock` reuse the existing
enable and lock effects.

`BlockActivation([abBlocked])` (FO4 adds a prompt flag) records the
reference in the saved `ReferenceActivationBlocks` by FormID. The CK says a
blocked reference "will still send OnActivate events", so the
`ActivateEvent` still fires for scripts. The engine's default handlers skip
the reference: the door transition, pickup and container looting, and the
activation conversation.

`<marker>.AddToMap([abFastTravel])` records the marker in the saved
`MapMarkers`. `IsMapMarkerVisible` / `CanFastTravelToMarker` guards read it
together with the marker's authored REFR `FNAM` flags (`AuthoredMapMarkers`;
the map-marker block is now decoded onto `PlacedRef::map_marker`). Skyrim.esm
has 397 markers: 20 start visible, 11 of them travelable.

`<alias>.ForceRefTo(akRef)` and `<alias>.Clear()` lower to
`Effect::ForceAlias`. The runtime resolves the alias property to its quest and
alias index (another quest's alias included) and records the override in the
saved `AliasOverrides`, keyed by reference FormID (`0x14` for the player),
once the quest-state guards are released. The alias refresh applies an
override at the alias's place in the fill order, so dependent aliases see it.
`Clear` also empties a location alias, and a stopped quest's overrides
drop.

### Script-raised story events (M47.2)

`Keyword.SendStoryEvent(akLoc, akRef1, akRef2, aiValue1, aiValue2)` lowers to
`Effect::SendStoryEvent`. Once the quest-state guards drop, the runtime
queues an `SCPT` event (`PendingScriptStoryEvents`) carrying the keyword
(`K1`), the location (a property or a location alias's binding), the two
references and the two integers (`V1` / `V2`). The Story Manager dispatcher
drains the queue with the engine markers, so several events raised in one
frame all dispatch.

`SCPT` nodes route on `GetEventData` (fn 576, 174 Skyrim SM uses), now in the
condition catalog with xEdit's packing: the event function in `param_1`'s low
16 bits, the member in the high 16, and the compared form in `param_2`. It
covers `GetIsID` on `K1` / `L1` / `L2`, `HasKeyword` on a location member
(through the location runtime) and `GetValue` on `V1` / `V2`; `IsInList` and
`GetItemValue` read 0.

Selection also honours the CK *Quest Alias Tab* rule that a non-optional
alias which fails to fill "will cause the quest to fail to start". A
`FromEvent` alias whose event slot is empty makes its quest ineligible, and
the walk continues, so the unconditioned `MarriageNode` no longer takes every
script event. Live in Dragonsreach: the College radiant starter's four boot
events start `MGR01` / `MGR02` / `MGR11` / `MGR20` through the four teacher
nodes, and `MQ104` stage 110 starts `MQGreybeardCall` through
`MQGreybeardCallNode`.

### Locations and location aliases (M47.2)

`LCTN` records parse into `EsmIndex.locations` (parent `PNAM`, keywords, the
population / unique-actor tables that give persistent references their editor
location, and the `ACSR`/`LCSR` location-ref-type table). `install_location_table`
flattens them into the `LocationTable` resource with each cell's `XLCN`.

- **Current location** of a cell-loaded entity: its `CellRoot`'s `CellFormId` →
  that cell's `XLCN`. The player is not cell-owned, so
  `story_change_location_system` publishes the player's cell location as
  `PlayerLocation` (only when the load order authors locations).
- **Editor location**: the `LCTN` table that lists the reference (unique
  actors carry their own location field), else its placed cell's.

The CK condition pages define the location family over that hierarchy — "is
(or has a parent location of)": `GetInCurrentLoc` (359), `GetInCurrentLocAlias`
(360), `LocationHasKeyword` (562, the location itself, not its parents),
`LocationHasRefType` (563), `GetIsEditorLocation` (565), `GetIsAliasRef` (566),
`GetIsEditorLocAlias` (567), `LocAliasIsLocation` (605) and
`HasSameEditorLocAsRefAlias` (181). FO3/FNV/Oblivion author none of these
indices; 180 and 610 are left out because Oblivion and FNV author them as
other functions.

Location aliases (`ALLS`) fill in list order with the reference aliases, in
`refresh_scene_actor_bindings`, by the CK *Quest Alias Tab*'s four mechanisms:
Specific Location (`ALFL`), Reference Alias Location (`ALFA` + optional
keyword-selected parent), External Alias Location, and Find Matching Location
(the event's `L1`/`L2` slot with "From Event", otherwise a random location
passing the alias conditions run on the candidate location). Reuse within a
quest and other quests' "Reserves Location" aliases are honoured. A binding
holds until its quest stops and is saved (`LocationAliasBindings`), so a random
pick survives a reload.

**Cleared locations.** The CK *Cell Reset* page: "Locations are cleared when
all of their bosses (creatures tagged with the 'Boss' LocRefType) are dead [or]
a script calls `SetCleared()`." `LocationTable` indexes each location's
`Boss`-typed references (the `LCRT` whose editor ID is `Boss`, `0x130F7` in
Skyrim.esm; 374 locations have bosses, 159 of them several).
`reconcile_dead_actor`, the death path every kill, start-dead actor and save
load funnels through, calls `note_actor_death`. That records the dead boss by
reference FormID in the saved `LocationClearState` and clears the location
once its last boss is dead. `<location>.SetCleared([abCleared])` (a location
property or a location alias's `GetLocation()`) sets it directly, and
`IsCleared()` guards read it. Conditions `GetLocationCleared` (250) and
`GetLocationAliasCleared` (598) read it too. A Find Matching location alias
never picks a cleared location unless flagged "Allow Cleared", and a flagged
one still prefers an uncleared match (CK *Quest Alias Tab*).

Not modelled: an unloaded reference's death for `LocationHasRefType` (it sees
scripted disables only). Against `Skyrim.esm` with no references loaded, all
162 Specific aliases, 3 of 4 External aliases and 28 of 136 Find Matching
aliases bind. The rest of the Find Matching aliases condition on a reference
alias. The 294 From Event aliases wait for their events (223 are
script-raised `SCPT` events).

### Essential and Protected actors, bleedout (M47.2)

The FO4 CK *Actor* page: "Essential actors cannot be killed. When they reach 0
health, they go into a special 'bleedout' state and recover over time.
Protected actors are treated as essential to all damage except that delivered
by the player." Essential is `ACBS` bit `0x2` in every game. Protected is bit
`0x800` from Skyrim on; FO3/FNV use that bit for "No Blood Spray". Both are
resolved through the template's "Use Base Data" (`0x0080`) and stamped as
`ActorProtection` at spawn. An alias's Essential / Protected flag applies while
its quest runs (CK *Quest Alias Tab*). The `essential_census` example counts
269 essential and 81 protected bases in Skyrim.esm, 139 / 253 in Fallout4.esm,
102 in Fallout3.esm, 28 in FalloutNV.esm and 259 in Oblivion.esm.

Every live death producer (combat hits, SDK actor-value writes, water
damage) asks `spare_from_death` before marking an actor `Dead`. A spared
actor gets a saved `Bleedout` and counts as unconscious for every #5017
consumer (no AI, no combat, no conversation). It gets back up after
`fEssentialDeathTime` with `fEssentialHealthPercentReGain` of its health (the
GECK page: "the time … an NPC marked essential will remain unconscious before
getting back up"). Skyrim.esm and Fallout4.esm author a 20 s death time. The
other values are the executables' defaults: 10 s on Oblivion and FO3/FNV, 30 s
on Skyrim and FO4, and 0.3 everywhere. Starfield has no such setting and
does not recover. The player is never spared.
Conditions `IsEssential` (354), `IsProtected` (476) and `IsBleedingOut` (580)
read the state, and fragments lower `IsEssential()` / `IsBleedingOut()`
guards. `<actor>.GetActorBase().SetEssential([b])` / `SetProtected([b])` (or
an ActorBase property's) record a per-base override in the saved
`ActorBaseProtectionOverrides`. Setting one flag clears the other (CK
*SetEssential / SetProtected - ActorBase*), and the override wins over the
base's stamp for every actor of that base. `<actor>.Kill([akKiller])` queues
`PendingScriptKills`, which `scripted_kill_system` drains through the same
path: an essential victim goes into bleedout (the CK *Kill* page: "Essential
actors will be made unconscious"), and Health is untouched ("this function
doesn't kill by modifying the health"). Otherwise the victim gets `Dead`, the
KILL story event with the script's killer, and the shared teardown. A protected
alias's victim dies only when the player is passed as the killer, which is the
CK's own documented workaround. Live (Skyrim, Riverwood's Sleeping Giant Inn): Delphine, essential,
went down at −4 health instead of dying and got back up 20 s later at 30
health. Orgnar, not essential, died to the same blows.

Not modelled: `SetNoBleedoutRecovery` / `GetNoBleedoutRecovery` (694), and
attackers keep striking an actor in bleedout (harmlessly) until their own
combat ends.

### Script registry (M47.0)

`ScriptRegistry` is a resource mapping SCPT/Papyrus `editor_id` strings to
`ScriptSpawnFn = fn(&mut World, EntityId)` spawners that install ECS state
components on a target entity. The cell loader's per-REFR walk drives it: when a
REFR's base record carries a `SCRI` cross-ref → SCPT → `editor_id`, the loader
looks the editor_id up here and runs the spawner. Keying on **editor_id** (not
FormID) is deliberate — editor IDs are stable across plugin loads, so two SCPT
records sharing an editor_id (e.g. `defaultRumbleOnActivate`) resolve to the
same spawner without per-plugin registration. Re-registering an editor_id
overrides the prior spawner (the intended mod-override path).

Today exactly **one** hand-written spawner is registered: `defaultRumbleOnActivate`
(`register_spawners` in [`papyrus_demo/mod.rs`](../../crates/scripting/src/papyrus_demo/mod.rs)).
The other three R5 demos carry per-instance properties that live in Skyrim+
`VMAD` subrecords the parser doesn't decode yet, so spawning them would attach
inert components — they defer to M47.2's VMAD decode. Unregistered scripts are
the common case (vanilla FO3 ships ~1257 SCPT records; M47.0 hand-translates a
handful). Before that fallback, the loader now conservatively translates
pre-Skyrim handlers containing engine-native load-order assignments and nested
`if`/`elseif`/`else` branches. It prefers preserved `SCTX`, then applies the same
boundary to source-less `SCDA`: exact statement framing, local indices resolved
through `SLSD` metadata, and one complete supported command expression per
assignment or condition. `GameMode`, `OnLoad`, and `OnActivate` execute in the
ECS schedule against the live immutable content catalog; numeric results land
in save-backed `ScriptVariables`. Nesting and `elseif` chain length are capped at 32. Any
unsupported statement, expression tail, event filter, or malformed branch
rejects the whole handler. A registry miss is therefore silent only when
neither path can attach behavior.

**The FO3/FNV quest-script census (2026-10-11).** Quest SCPTs execute on
every pre-Papyrus game, not just Oblivion: `install_quest_scripts`
(`asset_provider/script.rs`) resolves each quest's `script_ref` into the
obscript runtime with no game gate, and the 5 s GameMode tick
(`obscript_quest_tick_system`, registered in `boot/schedule/update.rs`) runs
whatever it installed. Command ids are shared across the three games, so the
interpreter needs no per-game table. The 2026-10-11 agent census measured the
live installed base: **322 FNV** and **101 FO3** quest scripts install and
tick ungated, with the wrong-target reference-slot defects of that audit's
SCR-D6-01/02 affecting 3–6 % of vanilla quest scripts — tracked separately
from this execution fact.

### The R5 demo translations (`papyrus_demo/`)

The proof-of-concept hand-translations that closed R5. Each is a faithful
port of a real shipped script, kept in-tree as living evidence and a test
surface; together they cover the four major Papyrus pattern families:

| Module | Source `.psc` | Pattern exercised |
|---|---|---|
| `papyrus_demo` (root) | `defaultRumbleOnActivate` | latent `Utility.Wait()` + multi-state dispatch + cross-subsystem call |
| `quest_advance` | `DA10MainDoorScript` | stage-gated `SetStage` (uses `ConditionList`) |
| `dlc2_ttr4a` | `DLC2TTR4aPlayerScript` | `RegisterForUpdate` / `OnUpdate` poll-fire-cancel |
| `mg07_door` | `MG07LabyrinthianDoorScript` | stage-gated activation + cross-reference method call after a latent wait |

`RumbleOnActivate` is the canonical example: its `state: RumbleState { Active,
Busy { wait_remaining_secs }, Inactive }` enum replaces Papyrus's
`Auto State active / busy / inactive` trio, and the `Busy` variant's
`wait_remaining_secs` field replaces Papyrus's stack-suspended `Utility.wait()`
frame with plain data. The single Papyrus event body splits into
`rumble_on_activate_system` (the immediate camera-shake + transition into `Busy`)
and `rumble_tick_system` (the post-wait `If repeatable / Else` branch) — exactly
the two-system split that is R5's load-bearing finding. Cross-subsystem calls
(`Game.shakeCamera`, `Game.shakeController`) become `CameraShakeCommand` /
`ControllerRumbleCommand` markers emitted on the `PlayerEntity` resource's
entity; the camera/input subsystems will drain them once they land.

### Papyrus parser (M30 + M30.2)

[`crates/papyrus/`](../../crates/papyrus/) parses `.psc` source into a typed AST.
It does **not** execute anything — the AST feeds the future M47.2 transpiler that
will emit ECS component + system definitions of exactly the shape the
`papyrus_demo` translations hand-built.

- **M30 Phase 1** — logos lexer ([`lexer.rs`](../../crates/papyrus/src/lexer.rs),
  [`token.rs`](../../crates/papyrus/src/token.rs)) + Pratt expression parser
  ([`parser/expr.rs`](../../crates/papyrus/src/parser/expr.rs)).
- **M30.2** (closed 2026-05-23, `ab0eee96`) — the full `.psc` parse:
  - statement parser ([`parser/stmt.rs`](../../crates/papyrus/src/parser/stmt.rs)):
    Return, If/ElseIf/Else/EndIf, While/EndWhile, local `VarDecl` with a
    speculative-type disambiguator, expr-stmt, assignment with compound operators;
  - top-level item parser ([`parser/script.rs`](../../crates/papyrus/src/parser/script.rs)):
    ScriptName + Extends header, Property (short + full, six `PropertyFlags`),
    Function (typed/untyped + four `FunctionFlags` incl. bodyless `Native`), Event,
    Auto State / State, Struct, CustomEvent, Group, Variable, Import;
  - public `parse_script(source) -> Result<(Script, Vec<ParseError>), …>` driver
    with per-item error recovery (`crates/papyrus/src/lib.rs`).

  Load-bearing finding: `Parser::peek()` silently skips newlines, so
  Return-with-value vs without needs `peek_raw()`. All four R5 source scripts
  round-trip end-to-end with zero recovered errors — asserted in
  [`crates/papyrus/tests/r5_round_trip.rs`](../../crates/papyrus/tests/r5_round_trip.rs).
  FO4 extensions (`Const`, `Hidden`, `Mandatory`, `BetaOnly`, `DebugOnly`) land
  as flag tokens decorating existing items. Semantic validation and the
  transpiler proper are M47.2. (#1270 later added a recursion-depth guard to the
  Pratt parser.)

The parser is *not* yet on the cell-loading hot path; M47.0's runtime attaches
hand-written components via `ScriptRegistry`, and the transpiler that would
consume parsed `.psc` automatically is M47.2.

---

## Why Replace Papyrus?

Papyrus is the scripting language used in Skyrim and Fallout 4. It runs inside a
separate virtual machine with its own threading model, execution queue, and save
serialization. This architecture causes several well-known problems:

- **Save corruption** — the VM serializes execution stacks (suspended function
  calls) into save files. Orphaned stacks from removed mods consume RAM
  indefinitely and can corrupt saves beyond recovery.
- **Script lag** — the VM has a per-frame budget for executing scripts. Under
  load (many mods, many NPCs), the queue backs up and scripts run seconds
  behind game state. Players see this as delayed quest updates, broken
  animations, and unresponsive activators.
- **Forced multithreading** — the VM runs on its own thread pool to maintain
  stability despite the single-threaded game engine. This creates race
  conditions, and latent functions (Wait, MoveTo) suspend execution mid-stack,
  creating invisible state that is expensive to serialize and fragile to restore.
- **Memory overhead** — every running script instance allocates a stack frame.
  Const scripts (introduced in Fallout 4) were an explicit attempt to let the
  engine unload scripts to save memory — an admission that the VM's footprint
  is a problem.
- **No direct memory sharing** — scripts access game state through a slow native
  function bridge. Every `GetActorValue()` or `GetPositionX()` crosses the
  VM/engine boundary.

ByroRedux eliminates all of these by making scripts first-class ECS citizens:
script state is component data, script logic is systems, and the ECS scheduler
owns all execution. No VM, no stacks, no separate threads. The R5 prototype
confirmed this isn't theoretical — `RumbleOnActivate`'s `Busy {
wait_remaining_secs }` field *is* the serialized suspended-wait frame, except
it's plain data the save system already knows how to round-trip.

### The Numbers: Bethesda's Own Profiling Data

Bethesda's official [Papyrus performance guide](https://falloutck.uesp.net/wiki/Performance_(Papyrus))
includes profiling data that quantifies the problems:

**Queue push latency dominates execution time:**
```
ObjectReference.GetPositionY:  0.04ms actual execution
                            5,760.60ms waiting in queue
```
That's a **144,000x overhead** — the work is instant, the VM scheduling is
catastrophic. A function that calls `GetPositionX`, `GetPositionY`, and
`GetPositionZ` separately spent 7,233ms total, of which ~7,200ms was queue
waiting. In ECS, reading `transform.translation` is a single field access —
nanoseconds, no queue.

**Object contention serializes access:**
When multiple scripts need data from the same object (e.g., the player), they
queue behind each other. 10 scripts asking for the player's level execute
serially, each waiting for the previous one to finish. In ECS, component reads
are concurrent (`RwLock<T>` allows multiple readers).

**Native function framerate sync:**
Most native function calls sync to the game's framerate — 30 native calls at
30fps takes ~1 second. "Non-delayed" functions bypass this, but most common
operations (position, inventory, actor values) don't qualify. In ECS, component
access is immediate — no frame boundary synchronization.

**Persistence traps:**
Storing an ObjectReference in a script variable makes that object persistent —
it cannot be unloaded to save memory. Function-local variables release on
return, but script-level variables persist indefinitely. Bethesda's advice:
"prefer function variables over script variables" and "set to None when done."
In ECS, components are owned by the world — no persistence surprises.

**Bethesda's own recommended patterns validate ECS:**
- "Use states with empty handlers instead of condition checks" → ECS queries
  skip entities without the relevant component (zero-cost non-participation)
- "Don't re-do work; cache in local variables" → component data is always
  available, no need to call through a native bridge
- "Prefer single point of entry over multiple cross-script calls" → in ECS,
  a system reads all components it needs in one query
- "Prefer events over polling loops" → ECS systems are inherently event-like
  (marker components appear, systems process, markers removed)
- "Break large functions into smaller ones" → each system does one thing

---

## Papyrus Language Model

### What Modders Write

A Papyrus script is a text file (`.psc`) that gets compiled to bytecode (`.pex`).
It declares a type (`extends ObjectReference`, `extends Quest`, etc.) and contains:

- **Properties** — typed variables exposed to the Creation Kit editor for
  per-instance configuration. Can be `Auto` (simple get/set), `Const`
  (editor-set, never changes at runtime), or `Mandatory` (editor warns if empty).
- **Events** — callbacks the game fires when something happens: `OnActivate`,
  `OnDeath`, `OnHit`, `OnTimer`, etc. 136 events in Fallout 4.
- **Functions** — reusable logic. Can be `Global` (no `Self`), `Native`
  (engine-implemented), or custom. Support default parameters and named
  out-of-order arguments.
- **States** — named modes (`Auto State AtRest`, `State Busy`) where different
  versions of the same event handler are active. `GoToState("Busy")` switches
  which handlers respond.
- **Fragments** — inline script snippets on quest stages, topic infos, packages,
  scenes, and terminals. Run at specific narrative moments.

The ByroRedux Papyrus parser (M30 + M30.2) covers all of these top-level item
kinds plus the FO4 flag extensions — see
[Papyrus parser](#papyrus-parser-m30--m302) above.

### Expression Grammar

Standard operator precedence:
```
|| → && → comparison → +/- → */% → unary(-,!) → cast(as) → dot(.) → array([]) → atoms
```

No bitwise operators. Cast (`as`) is runtime downcasting. Dot chaining
(`MyFunc().Prop[0]`) is the fluent API modders expect. This is implemented by the
Pratt parser in [`parser/expr.rs`](../../crates/papyrus/src/parser/expr.rs)
(with a recursion-depth guard, #1270).

### Type System

- Primitives: `Bool`, `Int`, `Float`, `String`
- Object types: `ObjectReference`, `Actor`, `Quest`, `Form`, etc.
- Arrays: `Int[]`, `Actor[]`, dynamically resizable (Fallout 4+)
- `Var` type (Fallout 4+): holds any value, used for custom event arguments
- Structs (Fallout 4+): grouped variables, passed by reference

### Inheritance

Scripts extend base types: `Scriptname MyScript extends ObjectReference`. The
child script inherits all functions and events from the parent, can override
them, and access the parent's version via `Parent.DoStuff()`. The base script
hierarchy mirrors the game object hierarchy (Actor extends ObjectReference
extends Form).

---

## The 136 Events — Mapped to ECS Patterns (DESIGN)

Every Papyrus event maps to one of three ECS mechanisms. No event queue, no VM
dispatch table, no registered callback lists. The shipped catalog so far is in
[Shipped event markers](#shipped-event-markers); the tables below are the
**full design plan** — most of these marker types are not yet implemented, and
the names are illustrative (the shipped markers use the names in the catalog
above, e.g. the activation event ships as `ActivateEvent`).

### Mechanism 1: Marker Components (interaction events)

A transient component is added to the entity by an engine system. Script systems
process it and remove it. Covers ~50 events.

| Papyrus Event | Marker Component (illustrative) | Added By |
|---|---|---|
| `OnActivate(akActionRef)` | `ActivateEvent { activator: EntityId }` ✓ shipped | Activation system / console (M47.0) |
| `OnHit(...)` | `HitEvent { aggressor, source, projectile, … }` ✓ shipped (no emit yet) | Combat system |
| `OnTriggerEnter(akRef)` / `OnTriggerLeave` | `OnTriggerEnterEvent { triggerer }` ✓ shipped (no emit yet) | Physics / Rapier sensor |
| `OnEquipped` / `OnUnequipped` | wearer-owned `EquipmentEventBatch` with stable item FormIDs ✓ shipped | Inventory / equip system and translated `EquipItem` fragments |
| `OnItemAdded` / `OnItemRemoved` | `InventoryEvent { item, count, source }` (planned) | Container system |
| `OnContainerChanged` | `ContainerChangeEvent { ... }` (planned) | Container system |
| `OnGrab` / `OnRelease` | `GrabEvent { grabbed: bool }` (planned) | Interaction system |
| `OnWorkshopObjectPlaced` / etc. | `WorkshopEvent { kind }` (planned) | Workshop system |

The marker component pattern is zero-cost for entities that don't participate —
no registration needed. If an entity doesn't have a script system watching for
`HitEvent`, the marker is never added in the first place.

### Mechanism 2: State Field Watches (state change events)

Systems compare current vs previous frame state on existing components. When a
field transitions, the associated logic fires. Covers ~40 events. **Not yet
implemented** — these are planned component/field shapes.

| Papyrus Event | Watched Component | Field Transition |
|---|---|---|
| `OnDeath` / `OnDying` | `ActorState` | `health > 0` → `health <= 0` |
| `OnCombatStateChanged` | `ActorCombat` | `state` field changes |
| `OnLoad` / `OnUnload` | `CellPresence` | `loaded: false` → `true` |
| `OnOpen` / `OnClose` | `DoorState` | `open` field changes |
| `OnLockStateChanged` | `LockState` | `locked` field changes |
| `OnPowerOn` / `OnPowerOff` | `PowerState` | `powered` field changes |
| `OnPackageStart` / `OnPackageEnd` | `ActorPackages` | `active_package` changes |
| `OnStageSet` (Quest) | `QuestStageState` | `stage` field changes (resource shipped; `QuestStageAdvanced` marker emitted, dispatcher pending) |
| `OnLocationChange` | `ActorLocation` | `current_location` changes |

Implementation plan: each watchable component stores a `prev_` shadow field or
the system keeps a parallel `PreviousState<T>` component; the diff system runs
early in the frame, before script systems. (Note: `OnLoad` at spawn time is
already covered by the shipped `OnCellLoadEvent` marker rather than a state
watch.)

### Mechanism 3: Timer/Condition Components (scheduled events)

Components with countdown or condition data, checked by systems each tick.
Covers ~15 events. `ScriptTimer` and `RecurringUpdate` are **shipped**; the rest
are planned.

| Papyrus Event | Component | System Behavior |
|---|---|---|
| `OnTimer(aiTimerID)` | `ScriptTimer { id, remaining }` ✓ shipped | Tick down, fire `TimerExpired` when zero |
| `OnUpdate` (`RegisterForUpdate`) | `RecurringUpdate { interval_secs, seconds_until_next }` ✓ shipped | Tick, fire `OnUpdateEvent`, re-arm |
| `OnTimerGameTime` | `ScriptTimer` (game-time variant, planned) | Tick by game hours |
| `OnDistanceLessThan` / `GreaterThan` | `DistanceWatch { target, threshold }` (planned) | Check distance each frame |
| `OnGainLOS` / `OnLostLOS` | `LOSWatch { target, had_los }` (planned) | Raycast check |
| `OnTranslationComplete` | `TranslationMove { target_pos, speed }` (planned) | Movement system, fire on arrival |

### Remote Events (cross-entity listening)

Papyrus (Fallout 4+): `RegisterForRemoteEvent(akSender, "OnDeath")` — script
on entity A receives events from entity B.

ECS plan: entity A has a `RemoteWatch { target: EntityId, event: EventKind }`
component. The event system queries both the source entity (for the marker
component) and all watchers (for `RemoteWatch` pointing at the source). No
registration list — the query is the registration. (Not yet implemented; R5's
notes record that the chosen R5 candidate scripts didn't exercise this, so it
remains design.)

### Custom Events (user-defined broadcast)

Papyrus: `CustomEvent MyEvent` + `SendCustomEvent("MyEvent", args)`.

ECS plan: `CustomEvent { name: FixedString, sender: EntityId, args: Vec<ScriptValue> }`
marker added to interested entities; `SendCustomEvent` adds the marker to every
entity holding a matching `CustomEventWatch { name, source }` component — the
query is the registration. (R5 follow-up 3 recorded a "vanilla CustomEvent
non-finding": the R5 candidate scripts didn't use custom events, so this stays
design until a consumer needs it.)

---

## ECS Mapping: Papyrus Concepts → ByroRedux

The examples below are illustrative of the translation strategy. Where a concrete
shipped translation exists, the corresponding `papyrus_demo` module is named.

### Properties → Component Fields

Papyrus:
```papyrus
Quest Property MyQuest Auto
Int Property StageToSet Auto
Bool Property IsEnabled Auto Const
Actor Property TargetActor Auto Mandatory
```

ECS (illustrative; cf. `RumbleOnActivate`'s real `Float Property` → `f32` field
mapping in [`papyrus_demo/mod.rs`](../../crates/scripting/src/papyrus_demo/mod.rs)):
```rust
pub struct MyActivatorScript {
    pub my_quest: FormId,          // Auto → regular field
    pub stage_to_set: i32,         // Auto → regular field
    pub is_enabled: bool,          // Const → field with default, skip in save serialization
    pub target_actor: Option<FormId>, // Mandatory → Option, validated at load time
}
impl Component for MyActivatorScript {
    type Storage = SparseSetStorage<Self>;
}
```

Editor configuration: the Creation Kit sets property values per-instance. In
ByroRedux the per-instance values live in Skyrim+ `VMAD` subrecords; the spawner
in `ScriptRegistry` installs the component with `.psc` defaults today, and
VMAD-override decode is the M47.2 follow-up (this is exactly why three of the
four R5 demos aren't registered yet).

### Events → System Scheduling

Papyrus:
```papyrus
Event OnActivate(ObjectReference akActionRef)
    if akActionRef == Game.GetPlayer()
        MyQuest.SetStage(StageToSet)
    endif
EndEvent
```

ECS (illustrative; cf. `rumble_on_activate_system` /
`quest_advance_system`):
```rust
fn activator_on_activate(world: &World) {
    let Some(events) = world.query::<ActivateEvent>() else { return };
    let Some(mut scripts) = world.query_mut::<MyActivatorScript>() else { return };

    for (entity, event) in events.iter() {
        if let Some(script) = scripts.get_mut(entity) {
            if event.activator == player_entity(world) {
                set_quest_stage(world, script.my_quest, script.stage_to_set);
            }
        }
        // No remove here — event_cleanup_system sweeps ActivateEvent at end of frame.
    }
}
```

No VM dispatch. The system runs in the ECS scheduler like any other system. The
`ActivateEvent` marker *is* the event — present → process; absent → empty query,
effective no-op. (Note: handler systems do **not** remove the marker themselves;
the shipped contract is that `event_cleanup_system` drains all transient markers
at end of frame, so multiple handlers can observe the same event.)

### States → Enum Fields

Papyrus:
```papyrus
Auto State AtRest
    Event OnActivate(ObjectReference akActionRef)
        GoToState("Busy")
        PlayAnimation("Pull")
        GoToState("AtRest")
    EndEvent
EndState

State Busy
    Event OnActivate(ObjectReference akActionRef)
        ; ignore while busy
    EndEvent
EndState
```

ECS (this is exactly the `RumbleState { Active, Busy { wait_remaining_secs },
Inactive }` shape shipped in `papyrus_demo`):
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PullChainState { AtRest, Busy }

pub struct PullChainScript {
    pub state: PullChainState,
    pub animation: FixedString,
}

fn pull_chain_system(world: &World) {
    let Some(events) = world.query::<ActivateEvent>() else { return };
    let Some(mut scripts) = world.query_mut::<PullChainScript>() else { return };
    for (entity, _event) in events.iter() {
        if let Some(script) = scripts.get_mut(entity) {
            match script.state {
                PullChainState::AtRest => {
                    script.state = PullChainState::Busy;
                    // play_animation(...); a tick/animation-completion system
                    // transitions back to AtRest — the latent-op split.
                }
                PullChainState::Busy => {} // ignore (empty Papyrus body)
            }
        }
    }
}
```

The state is visible data. Any system can read it. No hidden VM state machine.
Papyrus's runtime `GoToState("active")` string lookup (which couldn't catch
typos until the branch fired) becomes a compile-time-exhaustive `match`.

### PEX Extender Compatibility Preflight

Before a compiled Papyrus script is decompiled, `Pex::call_sites()` walks
state functions and full-property accessors in bytecode order. Static,
instance, and parent calls retain their script object, lexical scope,
instruction offset, argument count, and debug source line when present.

The scripting compatibility pass classifies recognized extender-era calls
against the SDK service catalog. Calls with an engine semantic destination
(for example, StorageUtil collections or mod events) report the target service
and required migration; unsafe facilities such as arbitrary JSON file access,
physical-key injection, and raw Scaleform access produce explicit policy
diagnostics. Vanilla and unknown mod calls are left alone, while malformed call
metadata is reported rather than silently omitted.

`analyze_source_compatibility` applies the same catalog directly to a parsed
Papyrus AST. It recursively covers functions, events, states, full properties,
groups, initializers, defaults, nested expressions, and control-flow bodies,
returning byte spans and one-based source lines in source order. This gives
authoring and porting tools the same compatibility policy before a `.pex` is
compiled.

Loose scripts can be checked without launching the engine:

```bash
cargo run -p byroredux-scripting --example extender_preflight -- \
  path/to/source.psc path/to/compiled.pex
```

The tool accepts multiple mixed inputs, prints each attributed finding and an
aggregate summary, exits `1` for input/parse errors, and exits `2` when an
unsupported or malformed call makes the compatibility gate fail. Mapped calls
remain visible but do not fail the command because they have a documented
engine-service migration target.

The engine also records compatibility evidence while it loads ordinary VMAD
scripts, quest fragments, and scene fragments. Each boundary reuses its
existing PEX parse, fingerprints the bytes, and publishes only relevant calls
to a bounded `CompatibilityRegistry`; attaching the same compiled script to
many references therefore counts it once. Operators can run `sdk.compat` in
`byro-dbg` to see deterministic provider/function aggregates, occurrence and
unique-script counts, mapped engine services, malformed-call totals, and
whether the registry hit its budget. This is an observed-script report for the
current engine world, not yet an eager scan of every PEX in every archive.
For exact `StorageUtil` global integer/string read aliases, each aggregate also
prints the target `byro.storage` operation, expected value variant, and
`ObjKey == None` constraint. Writes and other `StorageUtil` families are
reported unsupported rather than receiving a provider-wide compatibility
claim.

### Fragments → Triggered Systems

Papyrus fragments are inline script snippets that run at specific moments:
quest stage set, topic info played, package started, scene phase begun.

ECS: systems that watch for the corresponding state changes. The production
path populates `QuestStageFragments`/`SceneFragments` from compiled `.pex`,
then `quest_fragment_dispatch_system` and the scene runtime react to stage and
phase transitions. Lowering is conservative: an unmodelled statement declines
the whole fragment rather than approximating it; runtime resolution failures
skip the affected effect and leave the remaining chain available.

```rust
fn quest_stage_0010(world: &World) {
    let Some(advances) = world.query::<QuestStageAdvanced>() else { return };
    for (_entity, adv) in advances.iter() {
        if adv.quest == MY_QUEST_ID && adv.new_stage == 10 {
            // Fragment logic here — idempotency is free because the marker
            // only exists on the frame the stage actually advanced.
        }
    }
}
```

### Functions → Rust Functions / System Utilities

| Papyrus | ByroRedux |
|---|---|
| `Global Function` (no Self) | Regular Rust function |
| `Native Function` (engine-implemented) | System function exposed to scripting |
| Custom function on a script | Method on a component or standalone system |
| `Self` variable | Entity ID passed as parameter |
| `Parent.DoStuff()` | Not needed — no inheritance, compose components |
| `DebugOnly` / `BetaOnly` | `#[cfg(debug_assertions)]` on systems |
| Default parameters | Rust default trait or builder pattern |

### Inheritance → Composition

Papyrus: `Scriptname MyActor extends Actor` — inherits all Actor functions and
events, adds custom logic.

ECS: attach `ActorTraits`, `ActorStats`, `ActorAI`, etc. as individual
components, then add `MyActorScript` as an additional component. No inheritance
hierarchy. An entity can have any combination of components. Two mods can add
different script components to the same entity without conflict.

### Inter-Mod Communication → Component Queries

Papyrus (Fallout 4+): special API for calling functions on scripts from other
mods without compile-time dependency.

ECS: query by component type. If the component exists on an entity, interact
with it. If not, the query returns nothing. No special API, no registration,
no dependency declaration. This is free.

```rust
// Mod A adds a "Bounty" component to NPCs
// Mod B wants to read bounties if Mod A is installed
fn read_bounties(world: &World) {
    // If no entity has BountyComponent, this query returns None — zero cost
    if let Some(bounties) = world.query::<BountyComponent>() {
        for (entity, bounty) in bounties.iter() {
            // Mod B can read Mod A's data with no dependency
        }
    }
}
```

---

## What Bethesda Fixed in Fallout 4 (and Why It Validates ECS)

Several changes Bethesda made from Skyrim to Fallout 4 were explicit attempts
to fix problems inherent to the VM architecture. Each one validates our
approach:

| Bethesda's Fix | The Problem | ECS Solution |
|---|---|---|
| Replaced `OnUpdate` with per-script timers | Global per-frame callback too expensive | `ScriptTimer` / `RecurringUpdate` component, ticked by system (shipped) |
| `OnHit`/`OnMagicEffectApply` require registration | Too many scripts receiving unwanted events | Only entities with marker participate |
| `OnItemAdded`/`Removed` requires filters | Player dumps 100 items → VM floods | Query-based, zero cost for non-participants |
| `Const` properties | Save bloat from storing values that never change | Component field defaults, skip in serialization |
| `Const` scripts | VM memory overhead, "game may unload to save memory" | Stateless systems, zero footprint when idle |
| Remote Events | Cross-entity scripting required scripting both entities | Component referencing other entity + system query |
| Custom Events | No script-defined broadcast mechanism | Marker component with `Vec<ScriptValue>` args |
| Structs | No way to group related variables | Components |
| Namespaces | Script name collisions between mods | Rust module paths |
| Inter-mod communication | No way to interact with other mods' scripts | Query by component type — if present, interact |
| Fragment improvements | Fragment scripts created invisible dependencies | Triggered systems with explicit state queries |

Every one of these is a patch on a VM architecture. In ECS, the underlying
problems don't exist in the first place.

---

## Save Serialization

### Papyrus Save Problem

The Papyrus VM serializes:
1. All script instances and their property values
2. All active stack frames (suspended function calls)
3. All timer registrations
4. All event registrations

Orphaned stacks (from removed mods) persist forever, consuming RAM and
eventually corrupting the save. Bethesda's official position: *"Removing mods
and continuing to use the same save game is not supported by the game."*

The save can happen **between lines or even mid-line**. When scripts change
between saves, the engine must reconcile old serialized execution state with
new code. [Bethesda's official rules](https://falloutck.uesp.net/wiki/Save_File_Notes_(Papyrus))
reveal the complexity:

**Property/variable rules:**
- Auto property: saved value overrides masterfile value (unless const)
- Non-auto property: backing variable saved, won't get masterfile value on new add
- Const property: always uses masterfile value, ignores save — but still
  serializes the old value, and other variables assigned from it keep the old value
- Changing const↔non-const swaps which value wins on load
- `OnInit` runs only once ever — new variables added after first save won't be initialized
- Rename = delete + create (value lost)
- Type change = value discarded

**Function rules (mid-execution changes):**
- Removed function mid-execution: old bytecode loaded from save, finishes
  running, then future calls fail with errors
- Changed function: old serialized version runs to completion, new version
  used for subsequent calls — two versions coexist in one session
- Native→scripted change: **stack thrown out entirely**

**Persistence trap:** putting an ObjectReference in a script variable makes
that object permanently loaded in memory. Function-local variables release
on return, but script-level variables persist indefinitely.

### ECS Save Solution

Only component data is serialized. There are no stacks, no suspended frames,
no registration lists. None of the above complexity exists. (The R5 prototype
made this concrete: `RumbleOnActivate.state == Busy { wait_remaining_secs:
0.13 }` is the entire serialized form of a suspended `Utility.wait()` — a single
`f32`, not a heap stack frame.)

- **Properties** → component fields → serialized as regular data
- **Const properties** → skipped (value comes from the plugin, not the save)
- **Timers** → `ScriptTimer` / `RecurringUpdate` components → serialized as a
  remaining-time float
- **States** → enum field on component → serialized as an integer
- **Quest stages** → `QuestStageState` resource → serialized as `current_stage`
  + the `stages_done` set (sorted by FormID before writing)
- **Event registrations** → `RemoteWatch` / `CustomEventWatch` components
  (planned) → serialized as entity + event references

**Mod changes between saves — clean semantics:**
- Adding a component field → field gets default value, no initialization timing issues
- Removing a component field → field ignored on load, no orphaned state
- Changing a field type → migration system handles conversion or resets to default
- No mid-execution saves — systems complete atomically within their frame tick
- No function versioning — systems are compiled Rust code, not serialized bytecode
- No persistence traps — components are world-owned, not reference-counted

Removing a mod removes its components from all entities. No orphaned state.
Adding a mod adds new components to entities that match its records.
**Removing mods and continuing to play is fully supported.**

---

## Legacy Script Compatibility

For loading existing game content, ByroRedux needs to understand Papyrus
scripts from ESM/ESP files. Three approaches, not mutually exclusive — and the
shipped runtime already uses (1):

### Approach 1: Record-Level Mapping (SHIPPED for a handful of scripts)

The record loader extracts the *effect* of scripts (what properties are set,
what stages they respond to) and attaches equivalent ECS components. This
doesn't execute Papyrus — it interprets the configured state. This is exactly
the M47.0 `ScriptRegistry` path: a REFR's `SCRI`→SCPT→`editor_id` is looked up
in the registry and a hand-written spawner installs the component. Today only
`defaultRumbleOnActivate` is mapped; the hand-translated `papyrus_demo` modules
are the worked examples this approach generalises.

### Approach 2: Papyrus Transpiler (M47.2, parser SHIPPED)

Parse `.psc` source files (the M30/M30.2 parser — already shipped) and transpile
them to Rust component definitions + system functions of the same shape the
`papyrus_demo` translations hand-built. The grammar is simple enough that a
transpiler is feasible, and R5 proved the target shape works. The parser feeds
this; the transpiler itself is M47.2.

### Approach 3: PEX Bytecode Interpreter (stretch goal)

Load compiled `.pex` bytecode directly and interpret it within the ECS. This
would support mods that distribute only compiled scripts (no `.psc` source).
Requires understanding the `.pas` assembly-level opcodes. The pipeline is:
`.psc` source → `.pas` assembly → `.pex` bytecode → interpreter → ECS actions.

### API Surface to Support

The complete Papyrus API surface that legacy content uses is documented in
[`docs/legacy/papyrus-api-reference.md`](../legacy/papyrus-api-reference.md).
Key numbers:

- **101 script types** in the inheritance hierarchy
- **ScriptObject root:** registration functions, events, state machine, timers,
  reflection
- **Form base:** native + F4SE functions (keyword checks, name/weight/value)
- **Actor:** ~150 member functions decomposable into ~15 ECS components, ~40 events
- **Utility scripts:** global function containers (Game, Math, Debug, etc.)
- **ObjectMod system:** data-driven property modification with operators,
  value types, weapon properties

### UI Compatibility

Legacy content uses Scaleform GFx (Flash) for all UI menus. 34 named menus
are accessed via the `UI` script object. ByroRedux will use the Ruffle
project (Rust-native Flash emulator) as a compatibility layer for loading
original `.swf` files. See
[`docs/legacy/creation-engine-ui.md`](../legacy/creation-engine-ui.md).

---

## Milestone Status (verified 2026-10-12)

| Milestone | Subject | Status |
|---|---|---|
| **R5** | Papyrus quest prototype | **Closed 2026-05-16** — verdict *go ECS-native*. Hand-translated `defaultRumbleOnActivate.psc` to `papyrus_demo/`. See [`docs/r5-evaluation.md`](../r5-evaluation.md). |
| **M30 Phase 1** | Papyrus lexer + expression parser | Closed — logos lexer + Pratt parser. |
| **M30.2** | Papyrus statement + item parser (full `.psc` parse) | **Closed 2026-05-23** (`ab0eee96`). All four R5 scripts round-trip with zero recovered errors. Unblocks M47.2. |
| **M47.0** | Event-hooks runtime | **Closed 2026-05-23** (6 phases, `6c51af55..03837739`). `ScriptRegistry`, marker catalog, `script.activate` console command, e2e tests. |
| **M47.1** | Condition eval | **Closed 2026-05-23** (`ea9d0cfa`, `0a835e3e`). CTDA parser + OR-precedence evaluator; the initial 7-function catalog has since grown to 19 modeled variants and multiple quest/scene/package consumers. |
| **M47.2** | Full scripting runtime | **Closed 2026-10-12** — `.pex` decompilation, QUST/SCEN/INFO fragment lowering (58.2% of the 43,818 behavioral Skyrim + FO4 + Starfield fragments), VMAD binding, quest aliases, quest-script variables, saved faction membership, quest-script call inlining, perk entry-point composition, deferred waits/cascades. It intentionally does not promise exhaustive arbitrary-Papyrus execution; the follow-ups are listed in [`m47-2-design.md` §M47.2 closeout](m47-2-design.md). |
| **M43** | Quests & dialogue | **Quest/scene core shipped; broader integration open** — lifecycle, stages, objectives, aliases, fragments and scene phases are live. Story Manager search/payloads and full dialogue/UI integration remain open; see ROADMAP. |

Design doc for the event runtime: [`docs/engine/m47-0-design.md`](m47-0-design.md).
Design doc for the full scripting runtime: [`docs/engine/m47-2-design.md`](m47-2-design.md).
Ground-truth milestone state: always confirm against [ROADMAP.md](../../ROADMAP.md).

---

## References

- [Papyrus Category](https://falloutck.uesp.net/wiki/Category:Papyrus)
- [Papyrus Introduction](https://falloutck.uesp.net/wiki/Papyrus_Introduction)
- [Events Reference](https://falloutck.uesp.net/wiki/Events_Reference)
- [Function Reference](https://falloutck.uesp.net/wiki/Function_Reference)
- [Expression Reference](https://falloutck.uesp.net/wiki/Expression_Reference)
- [Differences from Skyrim to Fallout 4](https://falloutck.uesp.net/wiki/Differences_from_Skyrim_to_Fallout_4)
- [Differences from Previous Scripting](https://falloutck.uesp.net/wiki/Differences_from_Previous_Scripting) (ObScript → Papyrus)
- [Script Objects](https://falloutck.uesp.net/wiki/Category:Script_Objects) (101 script types)
- [Script File Structure](https://falloutck.uesp.net/wiki/Script_File_Structure) (`.psc` grammar)
- [Extending Scripts](https://falloutck.uesp.net/wiki/Extending_Scripts_(Papyrus)) (inheritance rules)
- Internal: [`docs/legacy/papyrus-api-reference.md`](../legacy/papyrus-api-reference.md) (full API surface documentation)
- Internal: [`docs/legacy/creation-engine-ui.md`](../legacy/creation-engine-ui.md) (Scaleform UI system, Ruffle strategy)
- Internal: [`docs/engine/lighting-from-cells.md`](lighting-from-cells.md) (cell-based lighting, probe seeding)
- Internal: [`docs/engine/m47-0-design.md`](m47-0-design.md) (event-hooks runtime design)
- Internal: [`docs/r5-evaluation.md`](../r5-evaluation.md) + [`docs/r5/source/`](../r5/source/) (R5 prototype evaluation + reference `.psc` fixtures)
- Code: [`crates/scripting/`](../../crates/scripting/), [`crates/papyrus/`](../../crates/papyrus/)
- Memory: `papyrus_reference.md`, `papyrus_events_catalog.md`, `scripting_as_ecs.md`
