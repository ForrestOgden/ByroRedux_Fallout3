# NIFAL-D3-2026-10-12-01: `attach_light_flicker_if_needed`'s doc still says `base_translation` is "captured from `ref_pos`"

- **GitHub**: #5729 (https://github.com/matiaszanolli/ByroRedux/issues/5729)
- **Labels**: low,nifal,documentation,doc-rot
- **Report**: docs/audits/AUDIT_NIFAL_2026-10-12.md

**Severity**: LOW

## Location
`byroredux/src/cell_loader/references/attach.rs:866-868`

Tier violated: n/a (doc-rot). Game affected: all with flickering LIGH-with-model placements.

## Description
c1a99c25e changed the `mesh_instance.rs` call to pass the light entity's local `Transform` translation (the `AttachLight` child or the first mesh entity). The `synth_child.rs` callers still pass `ref_pos`, which is correct there because that entity is an unparented root with `Transform::new(ref_pos, …)`. The function's doc still says the value is "captured from `ref_pos`", while the field's doc (`LightFlicker::base_translation`, `light.rs:185`) correctly says "un-jittered local translation".

## Evidence
`attach.rs:867` vs `mesh_instance.rs:1619-1629` (`light_local = world.get::<Transform>(light_entity)…`).

## Impact
None today. A new caller that follows the doc would seed a child entity with a world position, which was the latent bug on `mesh_instance.rs`'s old fallback path.

## Suggested Fix
Reword to "the entity's local translation (`ref_pos` only where the entity is an unparented root)".

**Related**: c1a99c25e

## Completeness Checks
- [ ] **SIBLING**: `byroredux/src/cell_loader/references/import.rs:530-533` carries a related stale `.spt` placeholder `attach_light_offset` comment (SPT-2026-10-12-D3-01, tracked separately)

Fix with: `/fix-issue 5729`
