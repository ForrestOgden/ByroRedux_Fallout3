# TD2-2026-10-11-08: "Teleport the player body" (Transform + zero KCC momentum + `set_kinematic_translation`) is written four times

URL: https://github.com/matiaszanolli/ByroRedux/issues/5591
Labels: low, physics, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/systems/character.rs:878-909` (`snap_character_body_to_camera`), `character.rs:1020-1038` (`ground_character_body_at`), `byroredux/src/save_io.rs:760-782`, `byroredux/src/commands/view.rs:470-489`
- **Status**: NEW
- **Age**: `character.rs` 2026-08-14 (`264f44fd3`); `save_io` 2026-06-21 (`93b06b959`).
- **Effort**: small
- **Description**: each sets `Transform.translation` (and rotation in 3 of 4), writes `GlobalTransform` in one, then `c.vertical_velocity = 0.0; c.is_grounded = false; c.wants_jump = false;` with the same "clear stale free-fall momentum" comment, then `byroredux_physics::set_kinematic_translation`. `character.rs` already hosts two of them and `pin_camera_above_body`.
- **Evidence**: `grep -rn 'wants_jump = false' byroredux/src` -> `save_io.rs:776`, `character.rs:902,1034`, `commands/view.rs:482`.
- **Impact**: a new `CharacterController` field that needs clearing on teleport (a swim state, a stuck timer) is a 4-site edit.
- **Related**: #2134 (lock-order note shared by the first two).
- **Suggested Fix**: add `CharacterController::clear_momentum(&mut self)` in `crates/physics`, and `pub(crate) fn place_character_body(world, body, pos, rotation: Option<Quat>) -> bool` in `systems/character.rs` (Transform + GlobalTransform + clear + kinematic sync). Route all four sites through it; `save_io` and `view.rs` keep their own camera handling.
