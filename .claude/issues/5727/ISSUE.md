# NIFAL-D6-2026-10-12-01: #5550 / #5549 left stale names and "first non-zero" wording in the surface-material docs

- **GitHub**: #5727 (https://github.com/matiaszanolli/ByroRedux/issues/5727)
- **Labels**: low,nifal,physics,documentation,doc-rot
- **Report**: docs/audits/AUDIT_NIFAL_2026-10-12.md

**Severity**: LOW

## Location
- `crates/core/src/ecs/components/collision.rs:126` ("first non-zero material") and `:143` (`shape_leaf_materials`)
- `crates/physics/src/convert.rs:1480` and `crates/physics/src/ragdoll.rs:423` (`shape_leaf_materials`)
- `docs/engine/physics.md:436` (`first_shape_material`)
- `docs/engine/nifal.md:545` ("body-level first non-zero")

Tier violated: n/a (doc-rot). Game affected: all with Havok collision.

## Description
daddf52f5 (#5550) deleted the `shape_leaf_materials` and `first_shape_material` walkers and moved both reductions into `resolve_shape_into` / `body_surface_material`. The fix for #5549 changed the body-level rule from "first non-zero leaf" to "first *authored* leaf", where `0` is authored STONE on Oblivion / FO3 / FNV and `NONE` on Skyrim. The canonical component's own doc comment and the nifal.md "Havok surface material" paragraph (written by #5574 / 993d52f3a) still carry the old wording, and four places still name functions that no longer exist.

## Evidence
`grep -rn 'shape_leaf_materials\|first_shape_material'` finds no `fn` of either name, only these doc and comment mentions plus a test name (`shape.rs:1090`). `crates/nif/src/import/collision/shape.rs::body_surface_material` documents the new rule: "the first AUTHORED leaf … a `[STONE, WOOD]` body is STONE, not WOOD (#5549)".

## Impact
A future consumer author reading the `RigidBodyData::surface_material` doc would expect the first non-zero leaf and mis-handle a STONE-first body on Oblivion / FO3 / FNV. The canonical type is the contract, so its doc should state the real rule.

## Suggested Fix
Reword `collision.rs:126` and `nifal.md:545` to "first authored leaf (`0` is authored STONE below Skyrim)", and replace the deleted function names with `resolve_shape_with_materials` / `body_surface_material`.

**Related**: #5549, #5550, #5574

## Completeness Checks
- [ ] **CANONICAL-BOUNDARY**: The reworded `RigidBodyData::surface_material` doc states the rule at the NIFAL parser-to-canonical boundary, not a render/consumer-time fallback
- [ ] **SIBLING**: Re-run the grep for both deleted names across `crates/`, `byroredux`-side code and `docs/` after the edit

Fix with: `/fix-issue 5727`
