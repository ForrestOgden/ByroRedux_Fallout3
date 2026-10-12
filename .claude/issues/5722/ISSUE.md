# #5722 — GAME-D5-2026-10-12-02: #5499 skips only fail-open Dialogue packages; a fully-modeled Say-To or off-target Dialogue package still wins selection and is then declined at install

**Labels**: `low`, `gameplay`, `ai`, `dialogue`, `bug`, `game:fnv`, `game:fo3`

**Source**: `docs/audits/AUDIT_GAMEPLAY_2026-10-12.md` (GAME-D5-02) · audited at HEAD 5d31fd637

- **Severity**: LOW
- **Dimension**: NPC Spawn → AI Package Selection → Locomotion
- **Location**: `byroredux/src/npc_spawn/ai_package.rs` — `select_active_package` (the guard is `package_conditions_fully_modeled` only) and the Dialogue arm of the package install (returns `None` for `targets_player == false` or `dialogue_type != 0`)
- **Status**: NEW (the residual of closed #5499, made reachable by closed #5492)
- **Trigger**: FNV/FO3. An NPC whose first scheduled package at the current hour is a Dialogue package that is Say-To (PKDD type 1) or targets a non-player reference, and whose conditions are all catalogued so that fail-open does not apply.

## Description
#5499 fixed the case where a Dialogue package passes selection through fail-open. The two other install gates are still evaluated only after selection: the player-target check and the Conversation-type check. #5492 (offset-16 decode) made the Say-To gate live for the first time, so a package it declines now wins selection and installs nothing. `from_package` returns `None`, the winner's FormID is still recorded, every later minute tick re-selects it with `changed == false`, and the actor runs none of the packages below it.

## Evidence
A lower-bound census (first scheduled package only, conditions not evaluated, so a conditioned package counts only when it would pass at runtime):
- **FNV**: 1 NPC_, `JacobstownKeeneDialogue` (Say-To, no conditions), at every sampled hour.
- **FO3**: 6 NPC_ bases at hours 3/12/20: 5 off-target (e.g. `CG02GuardGreetOverseer`, `FFEU19NPC1Dialogue` (no conditions), `MQ01BoSPostbattleDEMODialogue`, `CG03DadTalktoJonas`, `EclairZipTalk22x0`) and 1 Say-To (`CG03JonasToPlayer`). All but one carry `GetStage` (fn 58) or `GetInCell` (fn 67) conditions.
- **Package level** (fully-modeled Dialogue packages the install gates decline): FNV 45 (34 off-target Conversation, 1 off-target Say-To, 10 player-targeted Say-To); FO3 41 (36 off-target Conversation, 1 off-target Say-To, 4 player-targeted Say-To).

## Impact
Small and mostly quest-gated: while the package's window and conditions hold, the affected actor stands at its spawn point instead of running the Sandbox/Eat/Sleep/Travel below it. Jacobstown's Keene never runs an ambient procedure.

## Related
#5499 (closed), #5492 (closed), GAME-D5-2026-10-09-01 (the original), #5376.

## Suggested Fix
Extend the selection predicate to the same three gates as the install arm (a shared `dialogue_installs_ambient(package)` used by both), so a package the arm would decline is skipped by the scan.

## Completeness Checks
- [ ] **SIBLING**: Selection and install arm share one predicate so they cannot drift again
- [ ] **TESTS**: A regression test pins that a Say-To / off-target fully-modeled Dialogue package is skipped by `select_active_package`
