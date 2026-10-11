# #5563: PHYS-D6-2026-10-11-01: rapier 0.36 sleeps idle kinematic bodies, so `phys.stats`' "kinematic bodies=" became an active count; the #3975 docs, the fast-path rationale, the source guard that pins them, and the follow-ups plan's "ruled out" entry all…

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5563
- **Labels**: low,physics,documentation,doc-rot
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Queries, Diagnostics & Cost
- **Location**:
  - `crates/physics/src/world/mod.rs:544-557` (`active_island_counts` doc), `:622-624` (`has_awake_dynamics` doc), `:947-952` (fast-path NOTE), `:2984-3020` (guard)
  - `byroredux/src/commands/physics.rs:189-192, 225`
  - `crates/physics/src/sync.rs:172-173`
  - `docs/engine/dependency-update-followups-plan.md:528-529`
- **Status**: NEW. The premise was inverted by `ac81a09f9`. Related: closed #3975, whose doc fix is still in place but is now false.
- **Description**:
  - Under rapier 0.22 kinematic bodies lived in `active_kinematic_set`. That set was only ever pruned on removal or a user type or enable change (0.22 `island_manager.rs:15, 43-90`, `user_changes.rs:81-104`). #3975 therefore documented `active_island_counts().1` as "every live kinematic body, not an awake count".
  - Rapier 0.36 gives kinematics a real sleep test, "Platforms only sleep if both velocities are exactly zero" (`rigid_body_components.rs:1488-1493`). Idle keyframed bodies hit exactly that: `push_kinematic` skips no-op targets (`sync.rs:1273-1282`), so the interpolated kinematic velocity is 0.
  - The line the follow-ups plan cites for "still correct" (`island_manager/sleep.rs:30`, "kinematic and fixed bodies don't sleep") is the doc of `wake_up`. It is not the sleep decision.
- **Evidence**: the probe's `kinematic_sleep_tick` test:
  - Setup: 5 idle kinematic balls plus one free-falling dynamic, so `step` is never fast-pathed.
  - Output:
    ```
    tick 0: kinematic active = 5
    tick 29: kinematic active = 0
    live kinematic bodies in the set: 5
    ```
  - After a one-shot `set_next_kinematic_translation`, the count returns to 1 and drops to 0 again once the body idles (`idle_kinematics_leave_the_active_set`).
  - Every site listed under Location still states the 0.22 behaviour:
    - `world/mod.rs:548-553`: "the kinematic bodies, which rapier does not put to sleep … counts kinematic bodies whether or not anything moved them".
    - `:623`: "kinematics (which never leave it)".
    - `:948-951`: "Rapier keeps kinematic bodies in its active set without a sleep test … the set is never empty in a cell with authored-keyframed clutter".
    - `commands/physics.rs:189-192`: "Rapier's kinematic active set is never drained, so this is every *live* kinematic body (asleep or not)".
    - `sync.rs:172-173`: the same claim for `BYRO_PROFILE`.
    - The guard `kinematic_count_doc_agrees_with_the_fast_paths_own_rationale` asserts `doc.contains("not an awake count")` and `rationale.contains("never empty")`, so it now enforces the false text.
- **Impact**:
  - **Diagnostics only.** No simulation change: `has_awake_dynamics` filters to dynamics, so the fast-path decision is unaffected.
  - `phys.stats` (and the `BYRO_PROFILE` line) report e.g. `kinematic bodies=0` about 0.5 s after a cell's doors, keyframed clutter and an idle player capsule settle, even though hundreds of live kinematic bodies exist. The output's own comments tell the operator the number is the live total.
  - The plan's "Checked and ruled out — don't redo" entry steers the next reader away from the fact.
- **Trigger Conditions**: any cell with keyframed bodies (doors, FO4+ packed proxies, actor bones of non-animating actors, an idle player capsule) after about 30 idle ticks; `phys.stats` or `BYRO_PROFILE=1` read at that point.
- **Related**: #3975 (closed); D2's verified note "Kinematics sleep only at exactly zero velocity".
- **Suggested Fix**:
  - Re-word the five comment sites and the `phys.stats` label to "active kinematic", meaning moved within the last ~0.5 s.
  - Flip the guard's needles to the 0.36 claim.
  - Strike or correct plan line 528. If a live total is wanted, add `bodies.iter().filter(is_kinematic).count()`; it is on-demand only.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
