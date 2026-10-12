# Scripting Subsystem Audit — 2026-10-11

**HEAD**: 71b86091b · **Baseline**: `docs/audits/AUDIT_SCRIPTING_2026-10-08.md` (HEAD `00f580e09`) · **Audited**: all 7 dimensions

Run as an orchestrated audit: three read-only sub-agents (Dim 5; Dims 2–4; Dims 1/6/7), each writing to `/tmp/audit/scripting/dim_N.md`. No agent ran cargo or the engine. The orchestrator ran the test baselines and spot-verified the HIGH findings against the code. Real-data evidence is read-only censuses of the installed Skyrim/FO4/Starfield story-manager groups and the Oblivion/FO3/FNV SCPT records.

## Build & test state: CLEAN

```
$ cargo test -p byroredux-scripting -j 8                                        536 passed; 0 failed; 4 ignored
$ <1.96.0 cargo> test -p byroredux --bin byroredux -- boot::schedule cell_loader::references reference_enable_gate \
    npc_dialogue cinematic strip_ active_tether attach::tests extensions interaction story forcegreet quest
                                                                                291 passed; 0 failed; 1 ignored
```

No finding below is caught by a test. Every Story Manager test world omits `StoryManagerRng`, and the real-data M47.3 gate asserts that a `SetStage` was issued, not which quest it targeted.

## What changed since 2026-10-08

- Dims 1 and 6 have **no commits** in scope. Their findings come from re-deriving the invariants from scratch, and from a real-data census that the previous single-agent run did not do. They are latent defects, not regressions.
- Dim 2: only `boot/schedule/` changed.
- Dim 3: #5380 (condition catalog 56/74/77) and #5431 (fn-0 removal).
- Dim 4: kill-script provenance, cinematic re-adoption/purge, stamp cursor, seated-actor restore. All clean.
- Dim 5: heavy. The Story Manager fixes (#5404/#5405/#5385/#5394/#5419/#5515), dialogue (#5300/#5397/#5392/#5376/#5432), and the KILL/CLOC producers (#5434/#5435/#5447).

## Executive Summary

**Verified fixed this cycle** (evidence in the dimension notes below):

- **SCR-D5-2026-10-08-01** (SM vacuous pass). A node holding any uncatalogued CTDA function declines before evaluation, with no start and no consume. Pinned by `unknown_condition_function_declines_the_node`.
- **SCR-D5-2026-10-08-03** (#5405). Every sibling-group member is placed, and a cycle terminates.
- **SCR-D5-2026-10-08-05** (#5461). The ExitCart doc is fixed and pinned.
- **#5300**. Skyrim Say Once is per actor, and `OnEnd` fires on same-INFO re-select. `FORMAT_MAJOR` is 38 with the ledger entry.
- **#5397, #5392, #5376, #5419, #5394, #5435, #5434**. All sound.
- Lock discipline for the SM two-phase walk, the cinematic re-adoption purge, and the spoken-fragment path holds apart from the RNG finding.

**New findings: 0 CRITICAL, 8 HIGH, 5 MEDIUM, 17 LOW (30 total).** Two more facets extend open issues: #5406 and #5407.

The Story Manager stopped being fail-open on unknown terms (#5380). That fix catalogued three functions, and two of the three are now wrong or unsafe:

- **HIGH — SCR-D5-2026-10-11-01.** `GetRandomPercent` (fn 77) re-takes the `StoryManagerRng` write lock that the SM dispatcher already holds across `evaluate`. The lock tracker panics on same-thread re-entry in release builds too. In vanilla Skyrim, `DA09ChangeLocationSharesEvent` is the first quest node walked on every CLOC and gates on `[GetQuestRunning(DA09), GetRandomPercent]`. The panic hits at the first location change after DA09 has started. `WIChangeLocationThiefNode` reaches fn 77 at night once TG00's stage is done. **Confirmed by the orchestrator against `story_manager.rs:582` + `condition.rs:753` + `world.rs:865-875`.**
- **MEDIUM — SCR-D5-2026-10-11-03.** `GetQuestRunning` returns `is_running || is_started`, so a stopped, completed or failed quest reads as running.
- **MEDIUM — SCR-D3-2026-10-11-03.** The CTDA remap table `param1_is_form_id` was not extended for the newly catalogued fn 56/74 (or fn 71). Mod and ESL load orders read the "absent" default.

The legacy ObScript VM (**SCR-D6-01/02**) and the fragment lowerer (**SCR-D1-02/03/05/06**) both have wrong-target defects on code that has not changed. They are decline-invariant violations ("wrong AST accepted"). The census shows D6-01 firing on shipped vanilla masters, with 3–6 % of vanilla quest scripts affected.

**Untrusted-input verdict:**

- `SCDA` decoders (`obscript_vm.rs`, `obscript.rs`, `obscript_runtime.rs`): **no panic, OOB, unbounded alloc or unbounded recursion found.** Hardening gap SCR-D6-04 (LOW) only.
- **Provider runtime (`papyrus_provider/execute.rs`): NOT clean.** SCR-D7-01 (HIGH) is unbounded recursion and quadratic heap on a mod `.pex`. Its size estimates are not measured; the code path is confirmed.
- Provider manifests and caps: hold.
- SM tree build and dialogue chain merge: no panic or OOB; two LOW complexity and depth hardening gaps (SCR-D5-06, -07).
- The frontends' verdict lives in the newest `AUDIT_PAPYRUS_*`.

## Decline-Invariant Audit (Dims 1, 5, 6, 7)

| Decline point | Verdict |
|---|---|
| `classify_guard_atom` / `split_and`, `||` not split, `If` guard shape, no `elseif`, `has_latent`, `While` poll shape | Verified conservative |
| `lower_statements` recursion cap (`MAX_CONDITIONAL_DEPTH`) | Verified conservative |
| Unlisted statement kinds → decline (`effects.rs:572-575`) | Verified conservative |
| **`Stmt::Return(None)` accepted anywhere** (`effects.rs:515`) | **Leaks a partial lowering**: SCR-D1-02 |
| **`If` branch scopes cloned, rebinds discarded** (`effects.rs:548-553`) | **Leaks**: SCR-D1-03 |
| **`Assign` to a non-local identifier** (`effects.rs:508-513`) | **Leaks**: SCR-D1-04 |
| **`quest_via` maps any `X.GetOwningQuest()`** (`compose.rs:122`) | **Leaks**: SCR-D1-06 |
| **`SetMotionType` integer-literal arm** (`effects.rs:1553-1562`) | **Leaks**: SCR-D1-05; the named-constant arm is conservative |
| `AddRaceSpells`/`RemoveRaceSpells`/`ExitCart`/`PlayerImodAnimation`/`PlayerFurnitureAnimation` receiver | **Leaks**: #5407 + SCR-D1-01 |
| `SetObjectiveDisplayed` third argument dropped | Leaks, LOW: SCR-D1-09 |
| `quest_stage_gate::find_advance_event` ignores `State` / `is_auto` | Leaks, latent: SCR-D1-07 |
| `prim_lock` bounds at 1 argument (never matches compiled content) | Decline-safe coverage loss: SCR-D1-10 |
| Player-receiver declines (`StartCombat`/`StopCombat`/`SetUnconscious`) | Verified conservative |
| Every other `prim_*` argument bound (50 table entries read) | Verified conservative |
| Provider catalog consulted second; `servable_catalog()` gate | Verified conservative |
| `translate_pex` clean `None` on bad bytes and panic | Verified conservative |
| SM node with an uncatalogued CTDA function | Declines before evaluation (holds) |
| **SM node with all-catalogued terms incl. fn 77** | **Panics** (SCR-D5-01) |
| SM `RunOn::EventData` L-tags/unknown | Fail closed (holds) |
| ObScript unknown command → traced no-op | Holds |
| **ObScript reference slot resolution** | **Wrong target on vanilla data**: SCR-D6-01/02 |
| Provider seam: unclaimed `ExprStmt` → whole handler declined | Verified conservative |
| Provider runtime: sequential `If` count | **Unbounded**: SCR-D7-01 |

## Runtime Lifecycle Matrix (Dims 2–5)

| Invariant | Verdict |
|---|---|
| Marker drain coverage | Holds. Every marker is Pattern A or B, and every Pattern-B owner drains before any early return. The delta adds no markers. Full table in the Marker-Drain Appendix |
| `StoryEvent` multi-producer merge | One slot per entity, now three producers: LOW SCR-D3-06 |
| Two-phase lock discipline (`timer`, `recurring_update`, `trigger`) | Holds. Unchanged and spot-checked |
| SM dispatcher lock shape | Phase A (R guards + **RNG W**) then phase B (W), with no reverse edge. **The RNG W guard is held across `evaluate`**: SCR-D5-01 |
| Fragment nested-lock contract / cascade (`MAX_CASCADE`) / journal subscribers / populate-replace | Hold. Zero commits in `fragment*`, `quest_stages`, `globals`; guards read |
| Objective transitions, ledgers (`ReferenceLockState`, `ReferenceEnableState`, `ReferenceScriptState`) | Hold (unchanged) |
| CTDA OR-precedence / empty list = true / RunOn decline | Hold. The catalog is now 24 and fn 0 is correctly Unknown (#5431) |
| Edge-trigger seed (`occupant_inside: None`) | Holds (unchanged) |
| Attach path: silent-miss, gate ordering, stub script-attach helper, XPRM half-extents | Hold. The source pins are half-vacuous: SCR-D4-01 |
| Cinematic retention lifetime / re-adoption / purge | Holds. The `Transform → ActorCinematicState` edge in #5535 is a test-body artifact, not production |
| Force-greet | One same-frame hole: SCR-D5-05 (LOW) |
| Access-declaration rows | Two omissions (LOW): SCR-D2-01, SCR-D5-04 |

## Findings

### CRITICAL

None.

### HIGH

#### SCR-D5-2026-10-11-01: `GetRandomPercent` re-enters the `StoryManagerRng` write lock the Story Manager dispatcher holds across `evaluate` — panic on a vanilla Skyrim CLOC node
- **Severity**: HIGH · **Dimension**: Scene/Package/Dialogue (Story Manager); evaluator side Dim 3 · **Untrusted-Input**: Yes (any plugin can author fn-77 nodes; the vanilla trigger below needs none) · independently found as SCR-D3-2026-10-11-01
- **Location**: `crates/scripting/src/story_manager.rs:582` (guard taken), `:598` / `:850` (walk + `evaluate` under it); `crates/scripting/src/condition.rs:746-762` (re-acquire at `:753`); panic source `crates/core/src/ecs/world.rs:865-875` and `lock_tracker.rs:200-218`
- **Status**: NEW — introduced by #5380 (`395d2d831`), which catalogued fn 77 and routed it to this resource
- **Description**: `story_manager_dispatch_system` binds `let mut rng = world.try_resource_mut::<StoryManagerRng>()` and keeps it for the entire phase-A block. That block calls `evaluate(&node.conditions, ..)` for every node whose conditions are all catalogued. Since #5380, `evaluate_function`'s `GetRandomPercent` arm calls `world.try_resource_mut::<StoryManagerRng>()` again. `try_resource_mut` returns `None` only for a missing resource. A write held on the same thread trips the lock tracker's re-entrancy check, which is always on in debug **and** release, so the first fn-77 evaluation panics the Update-stage exclusive system.
- **Evidence**:
  - Skyrim.esm: `DA09ChangeLocationSharesEvent` (SMQN `0x00081C5A`) has CTDAs `[GetQuestRunning(DA09) == 1, GetRandomPercent <= <GLOB>]`. Both functions are catalogued, so the `modeled` gate passes. It is the first quest node walked on every CLOC. The AND list short-circuits while DA09 is un-started, which hides the defect on a fresh game.
  - `WIChangeLocationThiefNode` (`0x23F1B`, CLOC; `[GetLevel >= 5, GetStageDone(TG00), 77, 74, 74]`) is also fully modeled.
  - FO4 `testDogmeatInteractions` has fn 77 as its first condition (ADIA, not raised today).
  - Twelve more fn-77 nodes are masked only by an uncatalogued sibling term.
  - No test installs the RNG together with a fn-77 node (`setup_world`, `story_manager.rs:1015`, omits it).
- **Impact**: process-level panic in normal play on the first location change after DA09 starts, and (via SCR-D5-03) ever after. Any mod SM node gated on `GetRandomPercent` alone crashes at its first event.
- **Related**: #5380, #5372/#5414 (same lock-nest family), SCR-D5-03, SCR-D2-01.
- **Suggested Fix**: do not hold the RNG guard across the walk. Take it only inside the two pick sites, or clone the small `StoryManagerRng` at phase-A entry and write the state back after. Add a dispatch test that installs the RNG with a lone `[77]` node and a `[56, 77]` node.

#### SCR-D6-2026-10-11-01: ObScript reference indices resolve against the SCRO-only list, but compiled `r`-indices address the interleaved SCRO+SCRV list
- **Severity**: HIGH · **Dimension**: Legacy ObScript · **Untrusted-Input**: No for vanilla (fires on shipped masters); Yes in general
- **Location**: `crates/scripting/src/obscript_vm.rs:129`, `:668-676` (`resolve_ref_var`); producers `crates/plugin/src/esm/records/script.rs:77`, `:198-209` (SCRV → `ref_var_indices`, SCRO → `ref_form_ids`, two separate vectors)
- **Status**: NEW — follow-on of closed #4069, which split SCRV out for FormID remapping and left no consumer aligned
- **Description**: the `u16` after an `r`/`G` tag, and the caller index of a `0x1c` statement, is a 1-based index into the interleaved SCRO+SCRV list. `resolve_ref_var` indexes only `ref_form_ids`. After the first SCRV every literal reference shifts, and an index past the SCRO count resolves to `None` → `Ref(0)` → `QuestFormId(0)`.
- **Evidence** (agent-run read-only census, 2026-10-11; code path confirmed by the orchestrator):
  - `SCHR.num_refs == #SCRO + #SCRV` in 2393/2393 Oblivion and 2576/2576 FNV scripts.
  - Mis-resolved quest-stage ops: Oblivion 39/511 calls (8 scripts: `SE13Script`, `SE05QuestScript`, `SE02QuestScript`, `MQ16Script`, `MQ09Script`, `MQ05Script`, `MQ04Script`, `MS45Script`); FNV 50/290 (19 scripts, 9 resolve to form 0); FO3 16/148.
  - FNV `Patch04Quest00SCRIPT`: `r5` resolves to `0x163429` instead of `0x163427`, `r9` to `0x17BA15` instead of `0x15F4FF`, and `r10` to form 0. One of five quests is never started, and `StopQuest` targets form 0, so the script never stops itself.
  - The same shift applies to `' G<n>'` globals.
  - A `Ref` kept in the `f32` local map loses bits for FormIDs ≥ 2^24.
  - The M47.3 gate `vanilla_oblivion_quest_scripts_execute_and_setstage` checks that a SetStage was issued, not which quest.
- **Impact**: quest-state corruption (wrong quests started, stopped or staged; phantom `QuestStageState` entries). FO3/FNV quest scripts run too (SCR-D6-03).
- **Related**: #4069, #5088, SCR-D6-02.
- **Suggested Fix**: keep the interleaved order in `ScriptRecord` (`refs: Vec<ScriptRef::{Form, Var}>`, `num_refs == refs.len()` asserted) and index that. Treat `Ref(0)` / unresolved as a decline, not form 0. Strengthen the real-data gate to assert target identity (`Patch04Quest00SCRIPT`).

#### SCR-D6-2026-10-11-02: a tag-`r` `Set` is a remote-variable assignment, but `exec_set` treats it as a ref-variable assignment and shadows the target object
- **Severity**: HIGH · **Dimension**: Legacy ObScript · **Untrusted-Input**: No for vanilla; Yes in general
- **Location**: `crates/scripting/src/obscript_vm.rs:416-419` (doc), `:428-432`, `:458-461` (`ref_vars.insert(idx, form)`), `:672-674` (`ref_vars` shadows SCRO)
- **Status**: NEW
- **Description**: in all 1646 (Oblivion) / 4240 (FNV) / 1290 (FO3) vanilla tag-`r` `Set` statements, bytes 3–5 of the target are `s`/`f` + a variable ordinal, not zero as the doc comment claims. The escape is `<ref>.<remote variable>`, the target of `set Quest.var to x`. `exec_set` stores `ref_vars[idx] = value as u32`, and `resolve_ref_var` consults `ref_vars` before the SCRO table. After `set MS51Quest.iFlag to 1`, every later reference to that slot in the same block resolves to form `1`.
- **Evidence** (agent census; textual lower bound): a stage or query op follows such a `Set` on the same slot in 5 Oblivion (`SE07BScript`, `TGCastOut`, `MS51script`, `MQ01Script`, `RufioDieScript`), 9 FNV and 6 FO3 quest scripts. The remote-variable write itself is silently lost and is not counted as an unknown command.
- **Impact**: the later stage op runs on `QuestFormId(<small integer>)`, advancing the wrong quest or creating a phantom entry.
- **Related**: SCR-D6-01. Fixing 01 alone does not fix this.
- **Suggested Fix**: parse the 6-byte target as `ref + variable token`. Do not write `ref_vars` for it. Count it under `unknown_commands` ("remote variable write") or decline the block. Correct the doc comment with the corpus numbers.

#### SCR-D1-2026-10-11-02: `Stmt::Return(None)` is accepted anywhere in a fragment, so an early return inside an `If` lowers as nothing and the tail runs unconditionally
- **Severity**: HIGH (skill escalation: a wrong AST is accepted) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/translate/effects.rs:514-515`
- **Status**: NEW. The provider-seam sibling restricts it to the tail: `papyrus_provider/lower_program.rs:509`
- **Description**: the arm's comment calls `Return` "Champollion's fragment terminator", but nothing restricts it to the final statement at depth 0. A `.psc` body `If Self.GetStageDone(10) / Return / EndIf / Self.SetStage(20)` lowers to `[Conditional{then: []}, SetStage(20)]`, which applies the `SetStage` even when stage 10 is done. `A(); Return; B()` lowers dead code as live.
- **Evidence**: constructible with the `.psc` parser. What the `.pex` decompiler emits for a mid-function `Return` is **unverified** (no early-return test in `crates/pex/src/decompile`), so reach via `.pex` is unconfirmed.
- **Impact**: a quest stage or object effect fires in a state the authored script guards against, with no fallback.
- **Suggested Fix**: accept `Return(None)` only when `depth == 0 && statement_index + 1 == body.len()`, otherwise decline. Pin the three shapes.

#### SCR-D1-2026-10-11-03: branch-local rebinding is lost — `If` branches lower against cloned `Scope`s that are never merged back
- **Severity**: HIGH (receiver resolved to the wrong target) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/translate/effects.rs:548-553`, `:508-513`
- **Status**: NEW
- **Description**: cloned scopes are conservative for *declarations* inside a branch. Papyrus locals are function-scoped, though, so `k = OtherQuestProp` inside a branch changes what `k` means after `EndIf`. `Assign` goes through `bind_local` on the clone, which is dropped. Later statements lower against the pre-branch binding.
- **Evidence**: `Quest k = Self / If Self.GetStageDone(10) / k = OtherQuestProp / EndIf / k.SetStage(20)` lowers to an unconditional `SetStage { SelfRef, 20 }`. No test constructs a rebind inside a branch.
- **Impact**: effect applied to the wrong quest, actor or reference.
- **Suggested Fix**: after lowering both branches, decline when any pre-existing binding differs between a branch scope and the outer scope.

#### SCR-D1-2026-10-11-05: `SetMotionType` integer literals use the NIF `hkMotionType` table, not the Papyrus `Motion_*` constants
- **Severity**: HIGH (wrong lowering accepted) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/translate/effects.rs:1549-1577` (literal arm `:1553-1562`); test pin `:3311-3339` (`lowers_literal_havok_motion_types`)
- **Status**: NEW — premise error in closed #2286, which moved the table to the NIF enum on an unverified claim
- **Description**: the literal arm maps `1..=5|8 → Dynamic, 6 → Keyframed, 7 → Static, 9 → CharacterKinematic`, copied from the NIF importer. The CK wiki (`SetMotionType - ObjectReference.wiki`) gives Skyrim `Motion_Dynamic=1, SphereInertia=2, BoxInertia=3, Keyframed=4, Fixed=5, ThinBoxInertia=6, Character=7`. FO4 (`falloutck-uesp-wiki`) gives `Fixed=0, Dynamic=1, Keyframed=2`. **Confirmed by the orchestrator against the wiki and the code.**
- **Evidence**: Skyrim literal `4` (Keyframed) → `Dynamic`; `5` (Fixed) → `Dynamic`; `6` → `Keyframed`; `7` (Character) → `Static`. FO4 `2` (Keyframed) → `Dynamic`; `0` (Fixed) declines. The named-member arm is correct for Skyrim.
- **Impact**: a keyframed prop is released to physics, or a dynamic one frozen, by a literal-argument call.
- **Related**: #2286, #1652, *no-guessing policy*.
- **Suggested Fix**: the integer arm cannot be game-agnostic. Decline every literal except `1`, or take the game from the attach context and use the per-game Papyrus table. Re-pin the test with the wiki values.

#### SCR-D1-2026-10-11-06: `quest_via` maps `<any receiver>.GetOwningQuest()` to `QuestRef::OwningQuest`
- **Severity**: HIGH (binding resolves to the wrong quest) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/translate/compose.rs:121-124` (`method_call` returns the receiver but `quest_via` ignores it); consumers `effects.rs:1702-1713`, `quest_stage_gate.rs:79-80`, `fragment/effects.rs:17`
- **Status**: NEW
- **Description**: the doc says `Self.GetOwningQuest()`, but any receiver is accepted. `OtherAlias.GetOwningQuest().SetStage(10)`, where `OtherAlias` is another quest's alias property, lowers to `SetStage` on the *context* quest. Guard atoms `X.GetOwningQuest().GetStageDone(n)` classify the same way, so mixed-quest guards pass the same-quest check.
- **Impact**: stage written to or read from the wrong quest.
- **Related**: #1905 (same family, different hole).
- **Suggested Fix**: require the receiver to be `Expr::Ident("self")`; decline any property or local alias.

#### SCR-D7-2026-10-11-01: `execute_statements` recurses once per executed `If` and clones the tail — linear stack and quadratic heap in the number of sequential `If`s
- **Severity**: HIGH (skill: unbounded alloc or recursion on mod-supplied bytes) · **Dimension**: Provider/Extender Layer · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/papyrus_provider/execute.rs:873-895` (the `If` arm builds `ordered_tail` and recurses; confirmed by the orchestrator); entry `:340-347` (`papyrus_provider_system`, Late exclusive, `boot/schedule/late.rs:366-390`)
- **Status**: NEW — the same defect as closed #3935, which was fixed only at `fragment/effects.rs:1867-1880`
- **Description**: `MAX_PROVIDER_HANDLER_NESTING` (32) bounds nesting, not the sequential count. Lowering has no statement cap, and a `.pex` function may carry 65 535 instructions. Each executed `If` clones the rest of the handler and recurses, so depth equals the number of `If`s executed on the path, and live memory is the sum of the remaining tail lengths.
- **Evidence**: a handler with one provider call and N × `If b / EndIf` is valid. The agent's **estimate (not measured)**: N=3 000 → about 1 GB and 3 000 frames; N=10 000 → OOM. `catching_panics` does not wrap this system, and could not catch OOM or stack overflow anyway. Benign scripts (N < 100) are unaffected, which is why no test sees it.
- **Impact**: a mod `.pex` can OOM or stack-overflow the process the first time its event fires (OnInit at cell attach, or every OnUpdate tick).
- **Suggested Fix**: port #3935's cursor stack (borrowed slices, no clone; build an owned tail only at a `Wait` suspension). Add a `MAX_PROVIDER_STATEMENTS` cap at lowering. Test: 20 000 sequential `If`s run in linear time.

### MEDIUM

#### SCR-D5-2026-10-11-03: `GetQuestRunning` answers 1.0 for stopped (and completed/failed) quests — `is_running || is_started`
- **Severity**: MEDIUM · **Dimension**: Event Runtime / SM gate soundness · **Untrusted-Input**: No · independently found as SCR-D3-2026-10-11-02
- **Location**: `crates/scripting/src/condition.rs:728-738` (esp. `:734-736`); `quest_stages.rs:214-216` (`is_started` = map membership)
- **Status**: NEW (#5380)
- **Description**: `is_started` stays true after `Quest.Stop()` and after Complete/Fail. The engine's running predicate everywhere else is `is_running` alone: ObScript `GetQuestRunning` (`obscript_quests.rs:240-249`), dialogue ownership (`dialogue.rs:312`), SM pool eligibility (`story_manager.rs:911`), alias refresh (`scene/quest_alias.rs:718`). The CK/GECK wikis define the function as "currently running".
- **Evidence**: `get_quest_running_gate_evaluates_real_state` covers only never-started vs running. Vanilla gates of the shape `GetQuestRunning(X) == 0` (`CWChangeLocationScenes` on `CWFinale`; 29 Skyrim SM terms use fn 56) stay false forever once X has run and stopped. `== 1` gates (DA09) stay true, which widens SCR-D5-01 from "while DA09 runs" to "ever after DA09 started".
- **Suggested Fix**: use `is_running` only, decide Completed explicitly, and add a Stopped case to the gate test.

#### SCR-D3-2026-10-11-03: the CTDA remap table `param1_is_form_id` was not extended for fn 56 / 74 (and 71)
- **Severity**: MEDIUM · **Dimension**: Event Runtime / ESM handoff · **Untrusted-Input**: Yes
- **Location**: `crates/plugin/src/esm/records/condition.rs:525-544` (table), `:563-578` (use); consumers `crates/scripting/src/condition.rs:728-745`, `:853`
- **Status**: NEW for 56/74 (#5380 catalogued them without the paired remap entries); 71 `GetInFaction` predates the delta, with no issue found
- **Description**: the table doc says it mirrors the scripting catalog. It lacks 56 (quest), 74 (GLOB) and 71 (faction), whose `param_1` is a FormID. The evaluator looks the value up in `QuestStageState` / `Globals`, both keyed in global load-order space (**confirmed by the orchestrator: no 56/71/74 arm**).
- **Impact**: for a plugin whose master list is not a prefix of the load order (any mod, ESL, reordered DLC), these functions silently read the "absent" default. On an SM node that flips an authored gate: `GetQuestRunning(X) == 0` passes vacuously. Vanilla masters in order are unaffected, which is why no census-based test caught it.
- **Suggested Fix**: add 56/71/74 and derive the list from a per-function `param1_is_form_id()` method on the enum, with a test over `CATALOG`.

#### SCR-D1-2026-10-11-01: `AddRaceSpells`/`RemoveRaceSpells` and three siblings lower a script-defined helper by name and discard the receiver
- **Severity**: MEDIUM · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `effects.rs:1453-1495` (`lower_race_spells`, `prim_race_spells_arity_ok`); siblings `:1590-1606` (`prim_exit_cart`), `:1608-1638` (`prim_player_imod_animation`, `prim_player_furniture_animation`, `prim_player_animation_event`)
- **Status**: **Existing: #5407** (open; the code is unchanged). The three siblings are **new** and extend #5407's own "SIBLING" completeness check.
- **Description**: the receiver is "NOT distinguished", and the calling script's function table is never consulted. `ExitCart`, `PlayerImodAnimation` and `PlayerFurnitureAnimation` are `MQ101QuestScript` helpers, yet `receiver_quest` falls through to `quest_via`, which accepts any bare identifier as `QuestRef::Property`.
- **Impact**: #5407's "latent" premise has weakened, because #5413 now stamps the player's `RaceSpells`. A same-named mod helper would add or clear the player's racial spells and their constant modifiers.
- **Suggested Fix**: thread the script's function table and the receiver local's declared type into `Scope`. Accept the helpers only for the script's own object or a local declared `mq101questscript`, otherwise decline. Append the siblings to #5407.

#### SCR-D1-2026-10-11-04: assignment to a non-local identifier (script variable or property) lowers to nothing
- **Severity**: MEDIUM (latent; the corpus was not censused) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `effects.rs:508-513`, `:590-631` (`bind_local`)
- **Status**: NEW
- **Description**: `Stmt::Assign { target: Expr::Ident(name) }` never checks that `name` is a function-local. A pure-literal write to a script variable is recorded as `Binding::Plain` and emits no effect. Only field/index targets decline. `ScriptVariables` is modeled (`GetVMScriptVariable`), so the write is observable state.
- **Suggested Fix**: require the name to already be a declared local, otherwise decline.

#### SCR-D1-2026-10-11-07: `quest_stage_gate::find_advance_event` ignores `State` membership / `is_auto` and every other handler
- **Severity**: MEDIUM (latent) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `translate/recognizers/quest_stage_gate.rs:275-309`
- **Status**: NEW
- **Description**: the first `OnActivate`/`OnTriggerEnter` "at top level and inside any State" is accepted. Since #4319 the decompiler hoists the *auto* state's events, so a `State X` item is by construction non-auto. A script whose only single-`SetStage` handler lives in a non-auto state (reached after `GotoState`) is armed from spawn.
- **Suggested Fix**: accept only script-scope events or `is_auto` states, and decline when the script calls `GotoState`.

#### SCR-D7-2026-10-11-02: ModEvent delivery is O(programs × events × registrations) and the registration map is cloned every frame
- **Severity**: MEDIUM · **Dimension**: Provider/Extender Layer · **Untrusted-Input**: Yes
- **Location**: `papyrus_provider/execute.rs:34-40` (unconditional `registrations.clone()`), `:162-252`
- **Status**: NEW
- **Description**: the innermost loop filters by entity after iterating the whole map. The caps (`MAX_PENDING_PAPYRUS_MOD_EVENTS` 256 × `MAX_PAPYRUS_MOD_EVENT_REGISTRATIONS` 4096) allow about 10^6 iterations per program entity per frame, and about 10^9 for a script on ~1 000 references.
- **Impact**: mod-controlled multi-second hitches.
- **Suggested Fix**: clone only when events are pending, and key the lookup by entity with a `range` scan.

#### SCR-D5-2026-10-11-02: #5404's consume rule trusts the phase-A queue, not the real start
- **Severity**: MEDIUM · **Dimension**: Story Manager · **Untrusted-Input**: No
- **Location**: `story_manager.rs:858-868` (consume), `:905-919` (eligibility), `:613-651` (phase B), `:786-790` (comment); `quest_stages.rs:172-180`
- **Status**: **Existing: #5406** (open, unfixed at HEAD). This is a new facet to append, not a separate publish.
- **Description**: `consumed` is set iff a candidate was queued. Eligibility is `!is_running`, but `start_quest` refuses unless the quest is absent or Stopped. A Completed/Failed pool quest is queued and consumes the event for every later sibling, while phase B's discarded `start_quest` (`:617`) starts nothing. The comment at `:786-790` is wrong. A same-frame duplicate pick is also affected. The #5404 test covers only a *running* quest.
- **Suggested Fix**: eligibility should use a startable predicate (`absent || Stopped`) and derive `consumed` from the phase-B outcome. Land #5406's reset-on-start, fix the comment, and pin a Completed-pool test.

### LOW

| ID | Dim | Where | Issue / fix |
|---|---|---|---|
| SCR-D5-2026-10-11-04 | 5 | `boot/schedule/early.rs:23-76`; scan sources `mod.rs:603-617` | `player_controller_system`'s Access row omits the KILL producer's acquisitions (`StoryEvent` W, `LoadedCellIndex`, `CurrentCellContext`, `CurrentExteriorContext`) reached via #5434's `raise_kill_story_event`. The scan does not list `story_events.rs` for that system. Not a live race; declare the four types and add the hop to `PARALLEL_SYSTEMS` |
| SCR-D5-2026-10-11-05 | 5 | `systems/forcegreet.rs:119-122, 200-208, 225-243` | Pass 2 opens every in-radius greeter against a stale `open_partner` snapshot. Two same-frame greeters open A then B, which strips A's line mid-sentence and spends both directives. Re-read `DialogueSurfaceState::npc` per open |
| SCR-D5-2026-10-11-06 | 5 | `story_manager.rs:387-398, 440-468, 791-882` | **Untrusted-Input: Yes.** The tree build is O(g³) per sibling group, and `walk_siblings` recursion depth is bounded only by node count. Vanilla maxima are tiny (group 50, depth 6–7). Index members once and use an explicit stack with a depth cap |
| SCR-D5-2026-10-11-07 | 5 | `crates/plugin/src/esm/records/index.rs` `fold_info_into_topic` | **Untrusted-Input: Yes.** O(N×M) per DIAL override INFO fold (#5375). No panic or OOB. Build a `FormId → position` map |
| SCR-D3-2026-10-11-04 | 3 | `condition.rs:321-349, 1140-1177`; `scene/quest_alias.rs:932-943` | `subject_requirement` evaluates `GetRandomPercent` once "for every subject". It is stochastic and consumes RNG state, which breaks the exact-superset pre-filter. FO4 has 21 alias CTDAs on fn 77 (Skyrim 0). Add an `is_deterministic()` and treat non-deterministic as unconstrained |
| SCR-D3-2026-10-11-05 | 3 | `condition.rs:90-92, 746-755` | `GetRandomPercent` draws `pick(101)` (0..=100); the CK/GECK define 0–99. `pick(100)`, and fix the variant doc |
| SCR-D3-2026-10-11-06 | 3 | `story_manager.rs:532-543`; `systems/story_events.rs:131, 158`; `character.rs:1570` | `StoryEvent` is one slot per entity with three producers. A player-keyed CLOC and a player-victim KILL in one frame overwrite each other, and the CLOC cursor has already advanced. Make it a per-entity batch merged on insert |
| SCR-D3-2026-10-11-07 | 3 | `condition.rs:22-50` vs `:245` | The module doc says "19 functions" (table of 19). `CATALOG` is 24. Missing rows: 56, 71, 74, 77, 586. Related open #5610 covers only `HasPerk` |
| SCR-D2-2026-10-11-01 | 2 | `boot/schedule/late.rs:457-535`; `mod.rs:1059-1071` | The `npc_dialogue_selection` Access row omits what `evaluate_function` acquires, including the new `StoryManagerRng` W. The guard never scans `condition.rs`. Exclusive row, so nothing is mis-scheduled today. Add `(CONDITION_SRC, "evaluate_function")` and declare the types |
| SCR-D4-2026-10-11-01 | 4 | `cell_loader/exterior.rs:1273-1300`; `asset_provider/script.rs:974-991` | The stub-spawner source pins scan their own file with `include_str!`. The positive `contains` is satisfied by its own literal, so deleting the production call passes. The negative count still works. Use `source_scan::production_text` |
| SCR-D1-2026-10-11-08 | 1 | `player_control.rs:30-42`; `effects.rs:1283-1294` | `PlayerControlSelection::PAPYRUS_DEFAULT` is one hybrid table. The CK wiki's Enable and Disable defaults differ (camera/looking/sneaking, journal tabs). Two constants. Only `.psc` bodies see the fill-in |
| SCR-D1-2026-10-11-09 | 1 | `effects.rs:970-982` | `SetObjectiveDisplayed` never inspects its 3rd argument (`abForce`), so a non-literal or side-effecting expression is silently dropped. Use `bool_arg(args, 2)?` |
| SCR-D1-2026-10-11-10 | 1 | `effects.rs:1207-1217` | `prim_lock` bounds at 1 argument, but compiled `Lock(abLock, abAsOwner)` materializes 2, so it never fires on real content. Decline-safe (#3487 class) |
| SCR-D6-2026-10-11-03 | 6 | `ROADMAP.md:687-688`, `:269`; `asset_provider/script.rs:573`; `update.rs:456` | ROADMAP says FO3/FNV SCPTs are not executed. They are (322 FNV and 101 FO3 quest scripts), with ungated install and tick. Command ids are shared across the three games (agent census). Update the ROADMAP and record the census in `docs/engine/scripting.md` |
| SCR-D6-2026-10-11-04 | 6 | `obscript_vm.rs:591-614, 655-660` | **Untrusted-Input: Yes.** Argument payload reads are bounded by the buffer, not the argument region. A straddling last argument reads the next statement's bytes instead of reporting `Malformed`. Not memory-unsafe. Add `o + 3 > end` checks |
| SCR-D7-2026-10-11-03 | 7 | `lower_program.rs:483-498, 363-372`; `execute.rs:294-297, 436-438` | **Untrusted-Input: Yes.** `elseif` chains deeper than `MAX_PROVIDER_HANDLER_NESTING` lower fine, but `validate_provider_statements` rejects them on every dispatch with a warn. The handler never runs. Count the elseif index into `depth` at lowering |
| SCR-D7-2026-10-11-04 | 7 | `lower_program.rs:662-690`; `execute.rs:363-368` | **Untrusted-Input: Yes.** `Utility.Wait(3.0e38)` lowers, and the continuation cap drops the newest, so one mod can park 4 096 immortal saved continuations and starve every other latent handler. Cap `Wait` at lowering, or evict oldest-first |

## Existing findings re-verified

| Issue | State at HEAD |
|---|---|
| #5406 (SM start refusal ignored / no start-reset) | **Open, unchanged.** `start_quest`'s result is still discarded at `story_manager.rs:617`. New facet: SCR-D5-02 |
| #5407 (`AddRaceSpells` receiver) | **Open, unchanged.** Three sibling primitives found (SCR-D1-01) |
| #5300 | Fixed in `f18224b02`; verified correct and complete |
| #5404 / #5405 / #5380 / #5431 | Fixed. #5404 is incomplete (SCR-D5-02). #5380 introduced SCR-D5-01/03 and SCR-D3-03 |
| #5461 | Fixed and pinned |
| #5535 (lock-order lane) | The `Transform → ActorCinematicState` edge is the test body of `released_rider_exit_heading_matches_the_attached_heading` (`cinematic.rs:2140-2143`). Production `vehicle_attachment_system` keeps Transform-before-ActorCinematicState. Input for triage; not re-filed |
| #5508, #5610 | Open elsewhere; not re-filed |
| #4113 / #4115 | Live in `crates/papyrus` / `crates/pex` (`/audit-papyrus`) |

## Marker-Drain Appendix (Dim 3)

Pattern A = listed in `event_cleanup_system` (`cleanup.rs:86-112`; last Late exclusive, registered at `boot/schedule/late.rs:590`). Pattern B = drained at the head of the single owning consumer.

| Marker | Pattern | Drain site |
|---|---|---|
| `ActivateEvent`, `HitEvent`, `SplashEvent`/`RippleEvent`, `TimerExpired`, `AnimationTextKeyEvents`, `OnUpdateEvent`, `QuestStageAdvancedBatch`, the three demo `*Command` markers, `OnTriggerEnterEvent`/`OnCellLoadEvent`/`OnInitEvent`, `EquipmentEventBatch`/`ItemEventBatch` | A | `cleanup.rs:87-111` |
| `SceneEventBatch`, `SceneFragmentInvocationBatch` | A (+ idempotent head re-drain in `scene_playback_system`, `playback.rs:509-510`) | `cleanup.rs:99-100` |
| `SceneStartRequest`, `SceneStopRequest`, `SceneActionCompletionBatch` | B | `scene/playback.rs:523-525` |
| `DialoguePresentationEventBatch`, `DialogueLineCompletionBatch` | B | `dialogue.rs:466, 469` |
| `ScenePackageEventBatch`, `ScenePackageCompletionBatch`, `EvaluatePackageRequest` | B | `package.rs:640-646` |
| `StoryEvent` | B | `story_manager.rs:568`, before the `is_empty` return (`:569`); three producers (SCR-D3-06) |
| `TwoStateTransitionBatch` | B | `vm_state.rs:132` |
| `MotionTypeChangeRequest` | B (tail-drain) | `systems/cinematic.rs:270` |

New components in the delta (`EditorPlacement`, `CreatureActor`) are persistent state, not markers. The impact buffer lives inside `PhysicsWorld`. The contract tests `every_drained_marker_is_a_documented_pattern_a_marker` and `every_documented_pattern_b_marker_drains_in_its_named_consumer` cover the same set.

## Future-Phase Readiness

- **The SM start contract is still the gating item.** Phase A decides eligibility and consumption, but phase B decides whether a start happened. SCR-D5-02, #5406 and the Completed/Failed handling converge on one primitive: a start attempt that can fail, whose real outcome drives the marks, the log, the fill and the consume decision.
- **Catalog growth has gameplay weight.** Each function added to `ConditionFunction` needs three things: a semantics check against the CK/GECK definition (GetQuestRunning, GetRandomPercent), the `param1_is_form_id` entry (SCR-D3-03), and a lock-shape check (SCR-D5-01). A cheap guard would be a test over `ConditionFunction::CATALOG`. Re-derive the `sm1` expected set whenever a catalogued function changes.
- **ObScript phase 2.** The two VM defects (SCR-D6-01/02) predate phase 2. The reference list, local ref variables and remote variables need a decoded model before object scripts or `Set Quest.var` are built on top. FO3/FNV quest scripts already run, so the correctness bar applies today.
- **Fragment lowerer.** SCR-D1-02/03/04/06 are all "the scope or control-flow model is narrower than Papyrus". One fix pass (track every rebind and assignment, accept `Return` only at the tail, require a `Self` receiver) would close the family and make the `bind_local` conservative-by-default.
- **Provider runtime.** The cursor-stack pattern from #3935 and the statement cap should be shared between the fragment and provider interpreters, not re-implemented per site.
- **Skill-text drift to correct on the next `/audit-sync`:**
  - the Dim 1 `AddRaceSpells` "unqualified argless call" line is not enforced in code;
  - the "branches lowered against cloned Scopes" praise needs a pointer to SCR-D1-03;
  - Dim 6 should say that FO3/FNV quest scripts run.

## Findings Count

| Severity | New | Already tracked (new facet) |
|---|---|---|
| CRITICAL | 0 | 0 |
| HIGH | 8 | 0 |
| MEDIUM | 5 | 2 (#5406 via SCR-D5-02; #5407 via SCR-D1-01, whose three sibling primitives are new) |
| LOW | 17 | 0 |
| **Total** | **30** | **2** |

By dimension (new): Dim 1 has 4 HIGH (02, 03, 05, 06), 2 MEDIUM (04, 07) and 3 LOW (08–10). Dim 2 has 1 LOW. Dim 3 has 1 MEDIUM (03) and 4 LOW (04–07). Dim 4 has 1 LOW. Dim 5 has 1 HIGH (01), 1 MEDIUM (03) and 4 LOW (04–07). Dim 6 has 2 HIGH and 2 LOW. Dim 7 has 1 HIGH, 1 MEDIUM (02) and 2 LOW.
The Dim 5 / Dim 3 duplicates are merged into one finding each: D5-01 = D3-01, and D5-03 = D3-02.

---
*Generated by the `/audit-scripting` skill (orchestrated run, 3 sub-agents; HIGH findings spot-verified by the orchestrator). Suggested next step:*
`/audit-publish docs/audits/AUDIT_SCRIPTING_2026-10-11.md` (domain label `scripting`). Also add:

- `quests` and `game:skyrim` for SCR-D5-01, -02, -03;
- `game:oblivion`, `game:fnv`, `game:fo3` for SCR-D6-01 and -02;
- `game:fo4` where a finding names it (SCR-D1-05's FO4 half);
- `dialogue` for SCR-D5-05;
- `physics` for SCR-D1-05.
