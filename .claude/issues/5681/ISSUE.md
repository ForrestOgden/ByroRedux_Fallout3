# SCR-D2-2026-10-12-01: the fragment continuation queue is uncapped and `prim_wait` accepts any finite literal — a mod `.pex` can park immortal saved continuations

Labels: low,scripting,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5681

Filed from `docs/audits/AUDIT_SCRIPTING_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

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

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix

