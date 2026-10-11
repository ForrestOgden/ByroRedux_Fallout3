# TD6-2026-10-11-01: `ScriptTimer` / `TimerExpired` / `timer_tick_system` run every frame but nothing ever starts a timer or reads the expiry - Papyrus `OnTimer` can never fire

URL: https://github.com/matiaszanolli/ByroRedux/issues/5608
Labels: low, scripting, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW (amplifier considered: feature is documented "shipped", but no panic path -> stays LOW)
- **Dimension**: 6
- **Location**: `crates/scripting/src/timer.rs:17-60`, `crates/scripting/src/events.rs:86`, `crates/scripting/src/translate/tables.rs:71` (`CanonicalEvent::Timer`), `byroredux/src/boot/schedule/early.rs:109-112`; docs `docs/engine/m47-0-design.md:99`, `docs/engine/scripting.md:94`, `docs/engine/game-loop.md:142`
- **Status**: NEW
- **Age**: `87639d703` (2026-03-29); never wired since
- **Effort**: small (decide: wire `StartTimer`/`CancelTimer` lowering + an `OnTimer` dispatcher, or delete and fix the docs)
- **Description**: `timer.rs` declares "This replaces Papyrus's `StartTimer(time, id)` -> `OnTimer(id)` flow". The tick system is scheduled in `Stage::Early`, but the flow has no producer and no consumer.
- **Evidence**:
  - `grep -rnw ScriptTimer crates byroredux --include='*.rs'` outside `timer.rs`: only `lib.rs` re-export / `world.register`, doc comments, and `cleanup.rs`/save-registry names. The sole `ScriptTimer { .. }` constructions are inside `timer.rs`'s own `#[cfg(test)]`.
  - `grep -rnw "StartTimer\|CancelTimer" crates byroredux --include='*.rs'` (non-test): 1 hit, the `timer.rs:7` doc comment. No recognizer lowers `StartTimer`.
  - `grep -rnw TimerExpired crates byroredux ... | grep -v "events.rs\|cleanup.rs\|lib.rs\|timer.rs"`: only doc comments, the schedule `.writes::<TimerExpired>()` declaration, and a save-registry exemption string. No `query::<TimerExpired>()` reader; `CanonicalEvent::Timer` is never matched (`grep "CanonicalEvent::Timer"` = 0).
  - By contrast every sibling marker has a prod emitter and reader (ActivateEvent 31 readers, OnUpdateEvent 24, ...).
- **Impact**: An `OnTimer` handler in any Papyrus script is silently dead; `m47-0-design.md:99` marks `TimerExpired` as "emit sites: check timer_tick_system" which reads as shipped. The system pays a per-frame query for nothing. Same family as the closed `ItemEventBatch` (#4713) and `SetInChargen` (#4372) - but those got honest "No consumer yet" docs; this one did not.
- **Related**: #4713 (ItemEventBatch doc fix), #4372, `RecurringUpdate`/`OnUpdateEvent` (the working sibling, `recurring_update.rs`)
- **Suggested Fix**: Either (a) add a "No producer, no consumer yet" paragraph to `timer.rs` + `events.rs::TimerExpired` and correct the three doc rows, or (b) lower `StartTimer`/`CancelTimer` in `translate/effects.rs` into a `ScriptTimer` insert and dispatch `CanonicalEvent::Timer` in `papyrus_provider/execute.rs` the way `OnInitEvent` is.
