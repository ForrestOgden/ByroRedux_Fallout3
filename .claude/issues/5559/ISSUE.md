# #5559: PHYS-D4-2026-10-11-02: rapier 0.36's zero-motion depenetration is unaccounted for — the boot `dt = 0` tick pops embedded walking NPCs by offset + embed, an idle player is shoved by overlapping NPC bones and clutter, and two rationale comments stil…

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5559
- **Labels**: low,physics,ai,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Character Controller, Grounding & the NPC KCC (M42.10)
- **Location**: `byroredux/src/app_events.rs:314` (boot `scheduler.run(&self.world, 0.0)`);
  `byroredux/src/systems/wander.rs:313-324` + `locomotion.rs:183-188` (no `dt` guard; `step = speed·0` → `desired = 0`);
  `byroredux/src/systems/character.rs:509` (player `filter_groups: None`), `:391`, `:952-954`
- **Status**: NEW
- **Description**: 0.36 `move_shape` runs `check_and_fix_penetrations` whenever `|desired| < 1e-5`
  (0.36 :315-323, :240-290): it pushes the capsule out of every non-sensor collider passing the filter until the
  `offset` gap is restored, up to `dims.y/4` per call (16 BU player, 13 BU NPC). 0.22 had only a commented-out stub
  (0.22 :182). Neither the migration nor `dependency-update-followups-plan.md` records it. Consequences:
  (1) **NPCs at boot** — the startup `scheduler.run(&world, 0.0)` (`app_events.rs:314`) runs `wander_system`, which
  starts a fresh actor in `Walking` (`wander.rs:313-324`) and steps it with `dt = 0`; no locomotion system has a
  `dt <= 0` guard (wander/travel/guard/escort/follow/patrol/forcegreet/eat_sleep/cinematic), so the step's desired is
  exactly zero and every NPC whose feet are embedded > 1e-5 BU in its floor is popped up by `offset + embed`; it then
  hovers at the 4 BU offset for the rest of its walk (the KCC holds that gap; the fallback ray never fires because
  the KCC reports grounded). (2) **Idle player** — the grounded-branch correction is below 1e-5 on 59–120 of 120 idle
  frames (probe P4), so a standing player now takes the depenetration path most frames; its filter has no
  `ACTOR_BONE_GROUP` mask (`character.rs:509`), so an NPC bone collider or a dynamic body overlapping the capsule
  shoves the player up to 16 BU/frame while standing and never while walking. (3) Stale comments:
  `character.rs:952-954` says "Rapier's penetration recovery is a stub"; `character.rs:391` cites the 0.22
  no-interference branch (`character_controller.rs:317-322`), which in 0.36 is :401-412 and now runs the grounded
  check. `f4c2b9931`'s sweep missed both.
- **Evidence**: Probe P2 (NPC, production params, zero desired): embed 0 → dy 0; 0.5 → +4.5; 2 → +6; 6 → +10; a
  60-frame walk afterwards keeps feet in [3.906, 4.002], whereas the same walk without the zero step keeps the embed
  (−0.5 / −2 / −6). Probe P1 (player at rest, `Vec3::ZERO` desired, bone box overlapping the capsule side by 3 BU):
  translation (−7.0, −0.007, 0) — pushed to the offset gap; same pose walking 3.67 BU: no push. Dynamic box overlap:
  (−6.8, +0.9, 0).
- **Impact**: cosmetic — a 4 BU (≈ 5.7 cm) hover on NPCs embedded at spawn; idle-only nudges of the player by NPC
  limbs/clutter. No crash, no gameplay state. The larger risk is a future fix reasoning from the "stub" comment.
- **Trigger Conditions**: (1) boot cell with a spawned Wander/Patrol NPC whose ACHR feet sit inside its floor;
  (2) idle player overlapped by an NPC bone collider or a dynamic body.
- **Related**: PHYS-D4-2026-10-11-01, PHYS-D4-2026-10-11-03, #3799
- **Suggested Fix**: Early-return the NPC movers (or `step_toward_detailed`) on `dt <= 0`; decide whether the idle
  player should be depenetrated from actor bones (pass `actor_move_interaction_groups()` if not) and record the
  choice; correct the two comments and add the four 0.36 semantics changes to the plan doc's ledger.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
