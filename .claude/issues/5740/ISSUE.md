# TD2-2026-10-12-05: `spawn_synth_child`'s two ESM-light branches repeat the `LightSource` build and flicker attach; a census test locks the copies in

- **GitHub**: #5740 (https://github.com/matiaszanolli/ByroRedux/issues/5740)
- **Labels**: low,nifal,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD2-2026-10-12-05`)

**Audit finding** `TD2-2026-10-12-05` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `byroredux/src/cell_loader/references/synth_child.rs` (LIGH-only branch and fxlight branch, both calling `LightSource::from_legacy_world_units`); census at `byroredux/src/cell_loader/spawn/mesh_instance.rs`

## Description
Both branches run `translate_light`, build `LightSource::from_legacy_world_units(light_radius_or_default(ld.radius), ld.color, ld.flags, canonical_light_falloff_exponent(game, ...), geometry.kind, geometry.direction, geometry.outer_angle, canonical_light_shadow_flags(...))`, then `canonical_light_animation_flags` and `attach_light_flicker_if_needed`. The #4514 guard counts the call sites (`assert_eq!(synth.matches("LightSource::from_legacy_world_units(").count(), 2, ...)`) instead of removing the second one, so the test now protects the duplication.

## Evidence
13 identical 12-line windows between the two branches. (Validated: two `LightSource::from_legacy_world_units(` call sites in `synth_child.rs`.)

## Impact
The NIFAL light boundary (`canonical_light_*`) is the place a per-game change lands; the next one is three call edits per branch.

## Suggested Fix
`fn esm_light_source(ld, game, ref_rot) -> LightSource` plus `fn attach_esm_light(world, entity, ld, game, ref_pos, ref_rot)` in `synth_child.rs`; the census becomes "one call site here".

## Related
#4514, #2439, NIFAL-D3-2026-10-12-01 (a stale doc on the neighbouring `attach_light_flicker_if_needed`).

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **CANONICAL-BOUNDARY**: Per-game logic stays at the NIFAL parser->`Material` boundary (never pushed into shaders/renderer or re-derived at render time). See `/audit-nifal`.
- [ ] **TESTS**: A regression test pins this specific fix
