# TD2-2026-10-11-02: Six ambient-AI systems carry the same Pass-2 write-back (Transform + NavPath + stuck timer), and `patrol` is a ~130-line copy of `wander`

URL: https://github.com/matiaszanolli/ByroRedux/issues/5580
Labels: medium, ai, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: MEDIUM (table row 1, borderline). The family already diverged: #5452 found force-greet and Eat/Sleep followed no navmesh waypoints while the docs claimed they did, and #4189 had to be applied by hand to six files.
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/systems/{wander,patrol,travel,follow,guard,escort}.rs` (Pass 2 blocks: `wander.rs:423-466`, `patrol.rs:233-277`, `travel.rs:342-382`, `follow.rs:302-340`, `guard.rs:288-322`, `escort.rs:426-466`); `patrol.rs:93-232` vs `wander.rs:284-422`
- **Status**: NEW
- **Age**: Guard/Patrol landed `aaed15038` 2026-07-16; the NavPath write-back `a58bb4cf6` (2026-09-19, "Fix #4189") edited all six identically (11 lines each).
- **Effort**: medium
- **Description**:
  - Each system ends with `// ── Pass 2: apply writes (each a scoped single-type lock). ──`, then a byte-identical Transform loop (translation, optional rotation; 5 of 6 verbatim, `follow.rs` differs only by wrapping the tuple), then a verbatim `NavPath` take/insert/remove loop including the 6-line `#4189` comment (`grep -c 'd.nav_path.take()'` = 1 in each of the six files).
  - `wander` and `patrol` additionally share a verbatim `WalkStuckTimer` write-back (`wander.rs:459`, `patrol.rs:268`), the Pass-1a gather (queries, `speed`, `stuck_secs`, `resolve_cached_waypoints`), and the Pass-1b `step_oscillating_wander` + state copy + `nav_path` construction. They differ only in component types (`WanderBehavior/WanderState` vs `PatrolBehavior/PatrolState`) and the radius field name. `patrol.rs`'s module doc says it avoids a copy of the wander state machine, but only the inner step is shared.
  - The newest movers (`eat_sleep`, `forcegreet`) use `navmesh_path::cached_walk_step` (added `6316719ed`, 2026-10-11), a third idiom for the same resolve-cache / step / write-back sequence.
- **Evidence**: `grep -c 'd.nav_path.take()' byroredux/src/systems/*.rs` -> follow/guard/escort/travel/patrol/wander = 1 each; `git show --stat a58bb4cf6` -> 6 files, +48/-18, one hunk shape.
- **Impact**: the next change to the cache contract (residency generation, #3256; take-not-clone, #4189) is a 6-file edit, and a miss is silent. The #5452 divergence already shows it.
- **Related**: #4189, #3256, #5452, #2033.
- **Suggested Fix**:
  - In `byroredux/src/systems/navmesh_path.rs` (home of `resolve_cached_waypoints` and `cached_walk_step`) add `pub(crate) fn write_back_walk<D>(world, decisions: &mut [D], entity, translation, rotation, nav_path)` taking accessor closures, or a small `trait WalkDecision { fn entity(&self); fn translation(&self); fn rotation(&self); fn take_nav_path(&mut self); fn stuck_secs(&self) }`, and call it from all six Pass-2 tails.
  - Collapse `patrol_system_inner` into a generic `oscillating_walk_system_inner::<B, S>` in `wander.rs` (it already owns `OscillateWalk` and `step_oscillating_wander`), with `patrol.rs` supplying the component pair via a two-method trait. Delete `PatrolDecision`/`PatrolScratch`.
