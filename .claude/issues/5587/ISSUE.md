# TD2-2026-10-11-03: Per-procedure `*_DEFAULT_RADIUS = 512.0` x5, a duplicated repath threshold, and the walk-speed fallback chain x9

URL: https://github.com/matiaszanolli/ByroRedux/issues/5587
Labels: low, ai, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `wander.rs:89`, `patrol.rs:51`, `travel.rs:85`, `guard.rs:77`, `escort.rs:94` (`512.0`); `follow.rs:77` `FOLLOW_REPATH_THRESHOLD` vs `escort.rs:99` `ESCORT_COLLECT_REPATH_THRESHOLD` (both 64.0); walk-speed fallback at `wander.rs:335`, `patrol.rs:149`, `travel.rs:257`, `guard.rs:216`, `escort.rs:245`, `follow.rs:174`, `forcegreet.rs:167`, `eat_sleep.rs:344`, `combat_ai.rs:195`
- **Status**: NEW
- **Age**: constants 2026-07/08 (M42); `forcegreet`/`eat_sleep` fallbacks 2026-10.
- **Effort**: trivial
- **Description**:
  - Each doc comment says "same value as `wander.rs::WANDER_DEFAULT_RADIUS`" and `escort.rs:96` says "kept as its own constant rather than imported". The value is one fact (v0 default package radius), copied five times; guard/patrol/escort docs cross-reference each other to keep them in step by hand.
  - Nine call sites repeat `world.query::<WalkSpeed>() ... .map(|s| s.0).unwrap_or(LOCOMOTION_WALK_SPEED)` (7 identical `walk_speed_q.as_ref().and_then(..).map(|s| s.0).unwrap_or(..)` chains).
- **Evidence**: `grep -n 'DEFAULT_RADIUS: f32 = 512.0' byroredux/src/systems/*.rs` -> 5 lines; `grep -n 'LOCOMOTION_WALK_SPEED)' byroredux/src/systems/*.rs` -> 9 sites.
- **Impact**: low; a retune of the default radius is a 5-file edit.
- **Related**: TD2-2026-10-11-02.
- **Suggested Fix**: in `byroredux/src/systems/locomotion.rs` add `pub(crate) const AI_PACKAGE_DEFAULT_RADIUS: f32 = 512.0` and `pub(crate) fn walk_speed_of(q: Option<&impl Query>, entity) -> f32` (or `WalkSpeed::or_default(Option<&WalkSpeed>)` next to the component); point the 5+9 sites at them and have `escort.rs` import `FOLLOW_REPATH_THRESHOLD`.
