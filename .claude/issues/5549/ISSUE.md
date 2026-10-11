# #5549: PHYS-D1-2026-10-11-03: Material 0 is treated as "unauthored" everywhere, but it is authored STONE on Oblivion/FO3/FNV; the impact drain drops every both-0 pair, contradicting its own comment

- **URL**: https://github.com/matiaszanolli/ByroRedux/issues/5549
- **Labels**: low,physics,audio,bug
- **Source**: `docs/audits/AUDIT_PHYSICS_2026-10-11.md` (HEAD `65cadde2f`)

- **Severity**: LOW
- **Dimension**: Shape Translation (surface-material plumbing / user-data contract)
- **Location**:
  - Reduction: `crates/nif/src/import/collision/shape.rs:110-118`, `:136`, `:152-157`, `:165`, `:172`, `:179`, `:184`.
  - Impact drain: `byroredux/src/systems/audio.rs:374-383`.
- **Status**: NEW
- **Description**: On Oblivion and FO3/FNV, `*_HAV_MAT_STONE = 0` (nif.xml:410, 446). There is no "unset" value, and only Skyrim reserves 0 (`NONE`). The walker uses `!= 0` as "authored" throughout, with two consequences:
  - **(a) Wrong reduction.** A leaf authored STONE under a container with a non-zero material inherits the container's material (shape.rs:136). The body-level reduction is "first non-zero", so a body whose leaves are [STONE, WOOD] reports WOOD (shape.rs:117). A packed mesh's collapse also skips stone (shape.rs:155).
  - **(b) Silent impact drop.** The impact drain filters both colliders with `.filter(|m| *m != 0)` and then `continue`s:
    ```rust
    // The impacted side's material wins: collider 1's authored byte,
    // else collider 2's. Both STONE (0) falls through to the default
    // via the selection below.
    let Some(material) = (pw.collider_surface_material(event.collider1).filter(|m| *m != 0))
        .or_else(|| pw.collider_surface_material(event.collider2).filter(|m| *m != 0))
    else { continue; };
    ```
    The comment says both-0 pairs reach the default sound. The code skips them. `select_impact_sound(…, 0)` returning the default is pinned by `impact_selects_the_surface_sound_when_authored` (audio.rs:650-705), but the drain never calls it with 0.
- **Trigger Conditions**:
  - (a) Mixed-material Oblivion/FO3/FNV bodies where one part is authored STONE.
  - (b) Any `started` contact-force crossing where both colliders carry 0. That includes every impact between a stone-authored or unauthored clutter body and terrain, synthesized architecture, the player capsule, or ragdoll bones (all 0 by construction), and every FO3/FNV pair given -01.
- **Impact**: Dormant today, because `ImpactConfig::default_sound` and both maps ship `None`/empty (components.rs:1891-1897). Once a default impact sound is configured, the most common impact class on the three Gamebryo-era games stays silent, against the documented "falls back to the default" contract (components.rs:1872-1874).
- **Related**: -01, -02. The audio-side half overlaps whichever dimension owns the `1291f370a` contact-event drain; dedup there if also reported.
- **Suggested Fix**: Make "unauthored" explicit instead of overloading 0. For example, carry `Option<u32>`/a sentinel decided per game at the import boundary (Skyrim 0 → None; Oblivion/FO3 0 → Some(STONE)). In the drain, fall through to `select_impact_sound` with the resolved material rather than `continue`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files (both collider producers `register_newcomers` / `build_ragdoll`; player and NPC controller paths; dynamic and ragdoll buoyancy scans)
- [ ] **TESTS**: A regression test pins this specific fix (non-vacuous: it fails on the pre-fix code)
