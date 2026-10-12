# REG-2026-10-12-04: `ActorValues::set_permanent` is dead and untested since #5412

- **GitHub**: #5762 (https://github.com/matiaszanolli/ByroRedux/issues/5762)
- **Labels**: low,character,tech-debt,bug
- **Source**: docs/audits/AUDIT_REGRESSION_2026-10-12.md (`REG-2026-10-12-04`)

**Audit finding** `REG-2026-10-12-04` — severity LOW — from `docs/audits/AUDIT_REGRESSION_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/core/src/ecs/components/actor_values.rs` (`ActorValues::set_permanent`); `.claude/commands/audit-character/SKILL.md` still names it as a hazard

## Description
#5239 added `set_permanent` as the destination of a player `SetBase` on a derived pool. #5412 re-routed that to `set_override` ("the player `SetBase` reroute no longer lands here"), deleted `set_permanent_survives_the_player_refresh`, and left the method behind with a doc calling it "the `modav` counterpart". `modav` uses `mod_permanent` (additive); nothing calls `set_permanent`.

## Evidence
`git grep -nw set_permanent -- '*.rs'` -> the definition only (validated at publish time).

## Impact
A public mutator that discards every constant-spell bonus on the pool (the exact #5412 defect) sits unguarded, one wrong call away from reintroducing it.

## Suggested Fix
Delete it, or keep it and add a test that states why `modav` must not use it.

## Related
#5239, #5412, #5039. Also listed (as one of 15 items) in TD8-2026-10-12-03.

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix
- [ ] **DOC-SWEEP**: Every other doc/skill/comment repeating the stale claim is updated in the same change (grep the old wording)
