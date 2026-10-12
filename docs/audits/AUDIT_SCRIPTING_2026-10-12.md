# Scripting Subsystem Audit — 2026-10-12

**HEAD**: 5d31fd637 · **Baseline**: `docs/audits/AUDIT_SCRIPTING_2026-10-11.md` (HEAD `71b86091b`) · **Audited**: Dims 1, 2, 3, 5, 6, 7 (changed since baseline), Dim 4 (spot check) · **Unchanged since baseline (skimmed)**: Dim 4 (attach path / reference identity: only the `script.rs` source-pin test changed, `a7e05dd7b`; `cell_loader/references/import.rs` / `mod.rs` edits are NIFAL light-attach work, not scripting), Dim 2 production code (`fragment/**`, `quest_stages.rs`, `globals.rs` carry no commits; only `boot/schedule/` rows changed)

Single-agent, read-only run (suite preset `comprehensive`). Scratch notes: `/tmp/audit/scripting/dim_{1..7}.md`. No engine launch.

## Build & test state: CLEAN

```
$ <1.96.0 cargo> test -q -p byroredux-scripting
    562 passed; 0 failed; 5 ignored          (10-11 baseline: 536 / 4)
$ <1.96.0 cargo> test -q -p byroredux --bin byroredux -- boot::schedule cell_loader::references \
    reference_enable_gate npc_dialogue cinematic strip_ active_tether attach::tests extensions story forcegreet quest
    258 passed; 0 failed; 1 ignored
```

## What changed since 2026-10-11

78 commits on `main`; 24 touch the scripting scope. They are almost entirely the fix wave for the 10-11 report. **Verified fixed against current code, each with a pinning test** (read in place, not trusted from the commit message):

| 10-11 ID | Issue | Verified at | Verdict |
|---|---|---|---|
| SCR-D5-01 (HIGH) | #5628 | `story_manager.rs:811-820, 945-955`; the dispatcher no longer owns a `rng` guard, shuffle and pool-pick take `try_resource_mut::<StoryManagerRng>()` for the draw alone, so `GetRandomPercent` inside `evaluate` cannot re-enter. Pin `get_random_percent_gate_dispatches_with_the_rng_installed` installs the resource (every other SM test world omitted it, which is why this was invisible). | Fixed, sound |
| SCR-D6-01, -02 (HIGH) | #5629, #5630 | `obscript_vm.rs`: refs are `&[ScriptRef]` (interleaved SCRO+SCRV), `resolve_ref_var` returns `None` for a `Variable` slot, an unresolved explicit caller emits `CMD_DECLINED` instead of running on the owner, a tag-`r` `Set` is declined. `ScriptRecord.refs` is built in `parse_scpt` with the load-order remap applied to `Object` entries only. Host side: `quest_target` declines `Ref(0)` for all six quest commands. Real-data pin `fnv_patch04_quest_script_targets_its_own_scro_quests` (`#[ignore]`). | Fixed, sound |
| SCR-D1-02/03/04 (HIGH/MED) | #5631, #5632, #5638 | `effects.rs:539-640`: `Return` only as the depth-0 final statement; `Scope::rebound_by` declines a branch that rebinds a pre-`If` local; a bare-identifier assignment that is no declared local declines. | Fixed, sound |
| SCR-D1-05 (HIGH) | #5633 | `motion_type_arg`: only literal `1` lowers. | Fixed |
| SCR-D1-06 (HIGH) | #5634 | `quest_via`: `OwningQuest` only from argless `Self.GetOwningQuest()`. | Fixed |
| SCR-D7-01 (HIGH) | #5635 | `execute_statements` is an explicit cursor stack. The `Wait` tail is rebuilt from the whole pending stack in execution order (checked: `rest` is pushed before the match, so a `Wait` inside a branch resumes with branch tail then outer tail). | Fixed, sound |
| SCR-D5-03 (MED) | #5636 | `GetQuestRunning` = `is_running` alone. | Fixed |
| SCR-D3-03 (MED) | #5637 | `first_param_is_form_id` is an exhaustive match; `param1_form_id_table_matches_the_condition_catalog` walks 0..2048 against the plugin-side `param1_is_form_id`. | Fixed, with a drift guard |
| SCR-D1-07 (MED) | #5639 | `find_advance_event`: Auto State handler overrides script scope, non-auto-state handlers ignored. | Fixed |
| SCR-D7-02 (MED) | #5640 | `registrations_by_entity` grouped once, only when events are pending. | Fixed |
| SCR-D1-08/09/10 (LOW) | #5651, #5652, #5653 | `ENABLE_/DISABLE_PLAYER_CONTROLS_DEFAULT`; `bool_arg(args, 2)?`; `prim_lock` two-parameter arity (literal `abAsOwner` parsed, not applied — deliberate, no model for the cell public/private flag). | Fixed |
| SCR-D6-04 (LOW) | #5655 | `decode_args` bounds every read against the argument region. | Fixed |
| SCR-D7-03/04 (LOW) | #5656, #5657 | `lower_statements` counts the elseif index into `depth` (arithmetic re-derived: clause k body at `depth+2+k`, trailing else at `depth+1+N`, matches `validate_provider_statements`); `MAX_PROVIDER_WAIT_SECONDS` = 86 400 s at lowering. | Fixed |
| SCR-D3-07, SCR-D2-01, SCR-D4-01, SCR-D6-03 (LOW) | #5648, #5649, #5650, ROADMAP | Module-doc table pinned to `CATALOG.len()`; `npc_dialogue_selection` row declares the evaluator's 18 acquisitions; stub-spawner pins scan `production_text`; ROADMAP M47.3 row corrected. | Fixed |
| #5068 (extra) | named call arguments | `method_call` declines any call carrying a named argument; every `static_call`/primitive goes through it, and `prim_race_spells_arity_ok` is the one direct `Expr::Call` match (argless only). | Fixed |

## Executive Summary

**Findings: 0 CRITICAL, 0 HIGH, 0 MEDIUM, 4 LOW** (2 new this run, 2 carried from 10-11 with no issue, re-verified present). Existing issues still open and unchanged: #5406, #5407, #5641, #5643, #5645, #5647 (see "Existing").

The 10-11 fix wave is sound. The audit found no regression in it. Both of the HIGH families on code that had not changed (the ObScript reference-index/`Set` pair and the fragment lowerer's silent-drop group) were closed with the right shape: decline rather than approximate.

**Untrusted-input verdict (runtime-side decoders): NO panic / OOB / OOM / overflow / unbounded recursion found.**
- `SCDA` decoders (`obscript_vm.rs`, `obscript.rs`, `obscript_runtime.rs`): every statement and argument read is now bounded by the statement/argument region, recursion capped by `MAX_OBSCRIPT_VM_NESTING`. Clean.
- Provider runtime (`papyrus_provider/execute.rs`): the 10-11 "NOT clean" verdict is withdrawn. `execute_statements` no longer recurses or clones per executed `If`; `Wait` is capped at lowering; the elseif depth now agrees between lowering and validation. Clean.
- Provider manifests and caps: hold.
- One hardening gap remains on the fragment path (SCR-D2-2026-10-12-01, LOW): the *fragment* continuation queue is uncapped and its `Utility.Wait` literal is unbounded.

## Decline-Invariant Audit (Dims 1, 5, 6, 7)

| Decline point | Verdict |
|---|---|
| `classify_guard_atom` / `split_and`, `||` not split, `If` guard shape, no `elseif`, `has_latent`, `While` poll shape | Verified conservative |
| `lower_statements`: `Return` position (#5631), branch rebinding (#5632), non-local assignment (#5638), recursion cap | Verified conservative (each pinned) |
| Named call arguments (`method_call`, #5068) | Verified conservative; unreachable from `.pex` |
| Receiver binding: `GetOwningQuest` only on `Self` (#5634), `SetMotionType` literal `1` only (#5633) | Verified conservative |
| `abAsOwner` (`Lock`), `abVerbose` (`AddSpell`), `abForce` (`SetObjectiveDisplayed`): literal-only, parsed, not applied | Verified conservative (non-literal declines) |
| `quest_stage_gate` boot-state handler (#5639) | Verified conservative |
| `AddRaceSpells`/`RemoveRaceSpells` receiver discarded | Leaks (known): **Existing #5407** |
| ObScript VM: unresolved caller, variable slot, remote `Set`, `Ref(0)` quest argument | Verified conservative (`CMD_DECLINED` counted in `unknown_commands`) |
| Provider lowering: `elseif` depth, `Wait` cap, cap-before-descend | Verified conservative |
| Story Manager: node with an uncatalogued CTDA function | Verified conservative (#5380). Start refusal ignored in phase B: **Existing #5406** |

## Runtime Lifecycle Matrix (Dims 2–5)

| Item | State at HEAD |
|---|---|
| Marker drains (A: `event_cleanup_system`, B: owner head) | Unchanged vs the 10-11 appendix; no new marker type in the delta. `StoryEvent` Pattern B drain still precedes the `is_empty` return. One-slot overwrite: **Existing #5647** |
| Two-phase lock discipline (`timer_tick`, `trigger_detection`, `recurring_update`) | Unchanged |
| SM walk locks | `StoryManagerRng` write is now taken for the shuffle/pick draw only, under the walk's `registry`/`stages`/`node_state` read guards; the same nesting `evaluate` already takes for fn 77, so no new edge. Exclusive system |
| Fragment lock nesting | `fragment/**` unchanged; `apply_effect` inventory unchanged |
| Cascade bound `MAX_CASCADE = 64`; objective transitions; journal subscribers | Unchanged |
| Edge-trigger seed (`occupant_inside: None`) | Unchanged |
| CTDA OR-precedence | Unchanged (`or_precedence_quirk_*` green) |
| Schedule access rows | `equipment_appearance_system` (#5069) and `npc_dialogue_selection` (#5649) rows extended; both exclusive, scans pin them. Open: **Existing #5641** (`player_controller_system` row omits the KILL producer hop) |

## Findings

### CRITICAL / HIGH / MEDIUM

None.

### LOW

### SCR-D2-2026-10-12-01: the fragment continuation queue is uncapped and `prim_wait` accepts any finite literal — a mod `.pex` can park immortal saved continuations
- **Severity**: LOW
- **Dimension**: Fragment Dispatch & Locks (lowering side: Recognizer Chain)
- **Untrusted-Input**: Yes (mod-supplied fragment `.pex`)
- **Location**: `crates/scripting/src/translate/effects.rs:1760-1767` (`prim_wait`), `crates/scripting/src/fragment/effects.rs:1963-1971` (push), `crates/scripting/src/fragment/state.rs:502-506` (`FragmentExecutionQueue`)
- **Status**: NEW — the fragment-path sibling of closed #5657 (provider path)
- **Description**: #5657 capped `Utility.Wait` at one game day for the *provider* lowerer (`lower_wait`) because the bounded queue drops the newest entry and an effectively-infinite literal parks a slot for the rest of the session and across saves. The quest/scene/INFO *fragment* lowerer has the identical primitive and no cap: `prim_wait` only checks `seconds.is_finite() && seconds >= 0.0`, and `3.0e38` is a finite `f32`. The destination is worse than the provider's: `FragmentExecutionQueue.pending` is a plain `Vec<PendingFragmentExecution>` with no length bound at all, saved by the registry. `fragment_continuation_system` only decrements `remaining_seconds` by `dt`, so such an entry is never ready and is re-extended into the queue every frame.
- **Evidence**:
  ```rust
  // effects.rs:1760
  let seconds = as_num(&args[0].value.node)?;
  (seconds.is_finite() && seconds >= 0.0).then_some(Effect::Wait { seconds })
  // fragment/effects.rs:1964
  queue.pending.push(PendingFragmentExecution { ..., remaining_seconds, resume_when });
  ```
  `grep -rn 'pending.len()\|MAX_.*CONT' crates/scripting/src/fragment*` finds no bound.
- **Impact**: One immortal entry per execution of the mod's fragment. A quest stage fires once per advance, so the quest-stage case is bounded by gameplay; an INFO `OnBegin` fragment or a repeatable scene fragment re-fires per selection, so growth is linear in player action, and the per-frame `take` + `extend` cost scales with the queue. No panic, no OOB; hardening gap. No vanilla content reaches it (vanilla waits are fractions of a second to minutes).
- **Related**: #5657 (fixed provider path), #2288 (`MAX_ACTORS_3D_LOADED_WAIT_SECONDS`, which already bounds the other resume condition on this queue)
- **Suggested Fix**: share the provider's `MAX_PROVIDER_WAIT_SECONDS` (or one common constant) in `prim_wait` and decline above it; optionally cap `FragmentExecutionQueue.pending` with the provider queue's drop-newest-with-warn shape.

### SCR-D5-2026-10-12-01: dialogue selection evaluates every INFO's conditions twice per open/click, so a `GetRandomPercent` (fn 77) gate can pass the menu filter and fail the selection
- **Severity**: LOW
- **Dimension**: Scene/Package/Cinematic (dialogue selection)
- **Untrusted-Input**: No
- **Location**: `byroredux/src/systems/npc_dialogue.rs:191-203` (`top_level_menu`), `:239-247` (`select_on_topic`), `:314-336` (`open_conversation`: blocking probe then `select_on_topic`), `:768-805` (`select_topic_by_form_id` recomputes the whole menu, then selects)
- **Status**: NEW — latent since #5380 `395d2d831` catalogued fn 77 (before it, fn 77 evaluated as the fixed Unknown default 0.0, so the evaluation was deterministic)
- **Description**: `select_first_info(record, ..)` is the INFO gate, and it is called once to decide whether a topic belongs in the list/qualifies as Blocking, and again inside `select_on_topic` to pick the line. `select_first_info` evaluates the INFO CTDA set through the shared evaluator, where fn 77 draws from `StoryManagerRng` (installed by `install_story_manager`). A topic whose INFOs are all fn-77-gated passes the first evaluation with probability p and the second independently. When the second fails, `select_on_topic` returns `None` and `open_conversation` returns it directly (no fall-through to the generic greeting), so an NPC that listed a topic opens nothing; `select_topic_by_form_id` reports "no INFO … passes its conditions right now" for a topic the player just clicked in the list. The list shown after a line (`top_level_menu` fallback) also re-rolls on every click.
- **Evidence**: census of INFO CTDA function 77 (read-only script over the installed masters, GRUP walk, CTDA function at subrecord offset 8): Skyrim.esm 522 of 31 465 INFOs in 47 of 15 037 DIALs, 12 DIALs with *every* INFO gated (named ones: `DCETIdle`, `DCETTaunt`, `FreeformFrostRiverDeliverRepeat`, the rest EDID-less); FalloutNV.esm 9 INFOs in 3 DIALs, none all-gated. The all-gated Skyrim set is bark/freeform topics, not owned-quest menu topics.
- **Impact**: Intermittent "NPC opens nothing / clicked topic does nothing" on any plugin that gates a menu topic's INFOs on `GetRandomPercent`. Vanilla reach is near zero (the topics above are not Player-category menu entries), so LOW; the design hazard is that any other stochastic or stateful catalog function would behave the same.
- **Related**: #5645 (the pre-filter facet of the same stochastic-evaluator problem), #5380
- **Suggested Fix**: evaluate once per open — carry the chosen `&InfoRecord` from the filtering pass into the selection (`select_on_topic` takes the already-selected INFO), or draw the fn-77 value once per selection pass.

### Carried from 2026-10-11, re-verified present at HEAD, no issue exists

### SCR-D3-2026-10-11-05 (carried): `GetRandomPercent` draws `pick(101)`, the engine defines 0–99
- **Severity**: LOW · **Dimension**: Event Runtime · **Status**: NEW (still unfiled)
- **Location**: `crates/scripting/src/condition.rs:800` (`rng.pick(101)`), doc at `:95`
- **Description**: the GECK page for `GetRandomPercent` (`/mnt/data/src/reference/geck-uesp-wiki/(main)/GetRandomPercent.wiki`) returns "an integer between 0 and 99 inclusive". The evaluator returns 0..=100 and the variant doc says `[0, 100]`. Gates written `GetRandomPercent <= N` / `< N` are biased by one value in 101 (e.g. `<= 99` should always pass, `< 100` likewise).
- **Suggested Fix**: `rng.pick(100)`; fix the variant doc and the `[0, 100]` comments (condition.rs:95, :223, :793).

### SCR-D5-2026-10-11-05 (carried): force-greet Pass 2 opens every in-radius greeter against a stale `open_partner` snapshot
- **Severity**: LOW · **Dimension**: Scene/Package/Cinematic · **Status**: NEW (still unfiled; `forcegreet.rs` changed only for the #5452 walk step)
- **Location**: `byroredux/src/systems/forcegreet.rs:119-122` (snapshot), `:200-206` (`match open_partner`), `:236-251` (Pass 2 opens each `Step::Open`)
- **Description**: `open_partner` is read once before Pass 1. If it is `None`, every greeter whose distance is within radius that frame becomes `Step::Open`, and Pass 2 calls `forcegreet_open` for each in turn. The second open strips the first greeter's just-applied line (`apply_selection` fires the outgoing OnEnd) and both directives are consumed — the exact mid-sentence pre-emption #5392 guards against for an already-open partner.
- **Suggested Fix**: re-read `DialogueSurfaceState::npc` before each `forcegreet_open`, or open at most one greeter per frame and leave the rest pending.

## Existing (open, unchanged, not re-filed)

| Issue | State at HEAD |
|---|---|
| #5406 (SM start refusal ignored / no start-reset) | Open, unchanged. `story_manager.rs:619` still discards `start_quest`'s result; the 10-11 facet (SCR-D5-02: eligibility is `!is_running`, start needs absent/Stopped) is still accurate |
| #5407 (`AddRaceSpells` receiver discarded; three siblings noted 10-11) | Open, unchanged (`effects.rs:1537-1585`) |
| #5641 (`player_controller_system` Access row omits the KILL producer) | Open; reported again by ECS/Concurrency this suite |
| #5643 (SM tree build O(g^3), walk depth bounded only by node count) | Open, unchanged |
| #5645 (`subject_requirement` evaluates fn 77 once "for every subject") | Open, unchanged |
| #5647 (`StoryEvent` one slot per entity, three producers) | Open, unchanged; reported by ECS/Concurrency this suite |
| ECS-2026-10-12-D3-01 (hand-rolled `Children` walks duplicate `collect_subtree`) | Reported by the ECS audit this suite; the scripting-side copy is `cell_loader/unload.rs:65-78` (Dim 5 cinematic retention). Not re-filed |
| SCR-D5-2026-10-11-07 (`fold_info_into_topic` O(N×M) per override INFO, `crates/plugin/src/esm/records/index.rs:113-150`, Untrusted-Input: Yes) | Still present (linear `position` scans per INFO, no `FormId → position` map). ESM-owned code; not re-scored here, no panic/OOB |
| #5608 (`ScriptTimer` never started), #5610 (`PerkList` orphan, now correctly *not* read by `HasPerk`; `condition.rs:1020-1033` reads `Perks`) | Open elsewhere (tech-debt) |
| #4113 / #4115 | Live in `crates/papyrus` / `crates/pex` (`/audit-papyrus`) |

Noted for the Character/Lock-order owners, not filed here: `GetActorValue` (fn 14) is in `param1_is_form_id` and documented as an "AVIF FormID", but on TES4/FO3/FNV/Skyrim the CTDA parameter type is an actor-value *enum index*, not a FormID (FO4 alone uses a form). The remap is the identity whenever `masters[0]` sits at global slot 0, which holds for Bethesda masters, so there is no observable defect on shipped content; whether `ActorValues` is keyed by AVIF FormID or by index per game is `/audit-character` Dim 4's contract. Not verified here.

## Future-Phase Readiness

- **ObScript phase 2** (object-script blocks, Message UI, per-quest delay, actor-state functions): the VM now has a clean three-way contract for non-modelled constructs (`CMD_DECLINED` counted, unknown command counted, `Malformed`). A phase-2 command that needs a reference-variable (SCRV) target has no assignment model any more — the old `ref_vars`/`last_ref` shortcut was removed, so `set rVar to Ref` is intentionally inert until a real ref-variable model lands.
- **Quest alias follow-ups** (`m47-3-quest-alias-design.md` boundary list): unchanged, not findings.
- **Stochastic condition functions**: fn 77 is the first catalog function whose value changes between evaluations. Three consumers now rely on it being deterministic within a pass (SM pre-filter #5645, dialogue double-select SCR-D5-2026-10-12-01, and `subject_requirement`); an `is_deterministic()` predicate on `ConditionFunction` would let each decide once.
- **Fragment/provider wait caps**: a single shared `MAX_*_WAIT_SECONDS` constant would stop the two lowerers drifting again (SCR-D2-2026-10-12-01).

## Findings Count

| Severity | New | Carried (unfiled) | Existing issues re-verified |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 0 | 0 | 0 |
| LOW | 2 | 2 | 6 |

Next step: `/audit-publish docs/audits/AUDIT_SCRIPTING_2026-10-12.md` (domain label `scripting`; add `dialogue` for SCR-D5-2026-10-12-01 and SCR-D5-2026-10-11-05).
