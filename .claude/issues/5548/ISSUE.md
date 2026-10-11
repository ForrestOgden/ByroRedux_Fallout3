# #5548: PHYS-D5-2026-10-11-03: The swimming player takes harmful-water damage twice per frame, from the controller and from `water_damage_system`, since the player gained `ActorVitals` and publishes its own `WaterContact`

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5548
- **Labels**: medium,physics,water,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: MEDIUM
- **Dimension**: WATAL Physics Sink (two samplers / damage)
- **Location**: `byroredux/src/systems/character.rs:618-627` (controller applies `water_damage_for_contact` while swimming); `character.rs:1332-1341` (`sync_player_water_contact` publishes `damage_per_second` + `submerged_fraction` on the player); `byroredux/src/systems/water.rs:21-65` (`water_damage_system` damages every `WaterContact` row with `ActorVitals`, with no player exclusion)
- **Status**: NEW. It is an interaction between `4c9a5b368` (2026-09-09, the player publishes `WaterContact`) and the #4458 fix (closed 2026-09-19), which stamps `ActorValues` + `ActorVitals` on the player through `inventory::attach_to_player` (`inventory.rs:762-763`, `scene.rs:1219`). Before #4458, `water_damage_system`'s `vitals_q.get(entity)?` skipped the player.
- **Description**:
  - `water_damage_system`'s doc says "The kinematic player has its own controller path; this covers dynamic NPC/creature bodies". Its filter is only `WaterContact` + `ActorVitals` + not `Dead`.
  - The player now matches. In the same frame, `character_controller_system` (Stage::Early) applies `dps × fraction × dt` when `swim` is set, and `water_damage_system` (Late) applies `dps × fraction × dt` again from the published contact.
  - Swimming in hazard water therefore costs 2× the authored rate. Wading below swimlevel costs 1×, but only from the Late system, so the controller's swim-only gate is not the rule that actually holds.
  - Meanwhile the doc's intended target class is empty in production. NPC actors are `CharacterKinematic`, which the buoyancy scan skips (`water.rs:723-725`). Ragdoll bones carry contacts but no vitals. The player is the system's only live target.
- **Evidence**: the code path above. Both writers run every frame. `apply_player_drowning_damage` and `water_damage_system` both call `ActorValues::apply_damage` on the same `health` key.
- **Impact**: double hazard damage, and a wading-vs-swimming rule that contradicts itself. Hazard water is authored on Oblivion (5/23 WATR, `watal.md:732`) and on modded FO3/FNV. Vanilla FO3/FNV `DATA` damage is a sentinel 0, so those games are unaffected unless modded.
- **Trigger Conditions**: the player in a `WaterPlane` with `damage_per_second > 0` in Character mode.
- **Related**: #3119 (single death reconciler, intact), #4458
- **Suggested Fix**: pick one owner. Either `water_damage_system` skips `PlayerEntity` and the controller keeps its swim-gated damage, or the controller drops its arm and the Late system owns damage for every wet actor. Correct the "dynamic NPC/creature bodies" doc either way, and pin with a test that the player loses exactly `dps·frac·dt` per frame.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
