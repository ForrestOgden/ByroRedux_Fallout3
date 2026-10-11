# #5554: PHYS-D2-2026-10-11-04: (latent) `impact_audio_system` `break`s out of the frame on one unmapped material, and plays sounds for crossings from a substep the engine rolled back

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5554
- **Labels**: low,physics,audio,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Step & Sync (contact-event consumer)
- **Location**: `byroredux/src/systems/audio.rs:371-389` (material resolve), `:398` (`else { break }`); `crates/physics/src/world/mod.rs:1097-1103` vs `:1147-1166` (drain precedes the restore decision)
- **Status**: NEW (introduced by `1291f370a`)
- **Merge note**: the first bullet below (both-0 pairs skipped) duplicates PHYS-D1-2026-10-11-03 and is counted and fixed there. This finding covers the `break` and the rolled-back crossings.
- **Description**:
  - **STONE/unknown pairs are skipped.** The comment says "Both STONE (0) falls through to the default via the
    selection below" (`:375-376`), and the fn doc says "else the default". The code does `.filter(|m| *m != 0)` on
    both sides and `continue`s when both are 0. `0` is STONE *and* the value of every non-NIF body (terrain trimesh,
    water planes, the player capsule, ragdoll colliders, which `build_ragdoll` registers with material 0 and no
    user-data, `ragdoll.rs:426-458`). So a stone item hitting terrain, or anything hitting a ragdoll or stone floor,
    never plays, even when `default_sound` or `surface_sounds[0]` is authored.
  - **One unmapped material ends the loop.** `select_impact_sound(...) else { break }` abandons every remaining impact
    of the frame the first time a material has no entry and there is no default.
  - **Rolled-back crossings still play.** The drain (`:1097-1103`) keeps `started` crossings from a substep whose
    bodies the engine then restores as an invalid solve. An explosion's insane contact forces clear any
    `min_force_magnitude` and become a burst of impact one-shots at the corpse.
- **Evidence**: The code at `audio.rs:377-383`. `impact_selects_the_surface_sound_when_authored` tests only
  `select_impact_sound`, not the 0-filter or the loop.
- **Impact**: None today: `ImpactConfig::default()` ships empty and defaultless (`components.rs:1892-1900`). Once
  IPDS/IPCT content lands, impacts are missing for stone, terrain and corpses, one unmapped material truncates a
  frame, and explosions clatter.
- **Trigger Conditions**: authored `ImpactConfig`; a contact pair whose two materials are both 0; or a material with no
  entry and no default; or a recovery substep.
- **Related**: D2-02 (spurious first-wake crossings), #4743 (combat sounds, different path)
- **Suggested Fix**:
  - Resolve `material` without the `!= 0` filter on the fallback side, e.g. prefer the first non-zero byte, else 0, and
    let `select_impact_sound` fall to the default.
  - Change the `break` to `continue`.
  - Discard (truncate) the substep's appended events when `restored > 0`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
