# NIFAL-D6-2026-10-11-01: The dispatch-coverage guard is now satisfied by the surface-material walker for 7 of 16 shapes

Labels: low,nifal,physics,test-gap,bug

- **Severity**: LOW
- **Dimension**: Collision
- **Tier Violated**: harness-gap
- **Game Affected**: all (FO3/FNV/Oblivion/Skyrim collision)
- **Location**: `crates/nif/src/import/collision/mod.rs` (`dispatch_coverage_tests::resolved_shape_structs_in`, scans `shape.rs` up to the first `#[cfg(test)]`); `crates/nif/src/import/collision/shape.rs:110-224` (`shape_leaf_materials_inner`)
- **Status**: NEW (side effect of 886edcded / 441d90c2f; builds on the "known-weak per #4411" caveat)
- **Description**: The guard proves "every dispatched `Bhk*Shape` has a `downcast_ref::<Bhk..Shape>` in `resolve_shape_inner`" by splitting the whole `shape.rs` production text on `downcast_ref::<`. The surface-material commits added a second dispatch chain, `shape_leaf_materials_inner`, in the same production prefix. It names packed, list, convex-list, transform, convex-sweep, MOPP and compressed-mesh via literal `downcast_ref::<..>`. Deleting the real `resolve_shape_inner` arm for any of those seven would still pass.
- **Evidence**: Distinct `downcast_ref::<Bhk*Shape>` in `resolve_shape_inner` (`shape.rs:233-508`) = 16; across the whole production prefix = 16; the seven names appear in both ranges. The other leaf shapes go through `leaf_material!($ty)`, whose `downcast_ref::<$ty>` does not match the `Bhk` prefix scan, so they are still discriminated.
- **Impact**: Guard confidence for the container/mesh shapes that carry the most collision (MOPP, list, packed, compressed) is structurally vacuous. No current bug.
- **Related**: #4411, #5550 (the walker that drifts from `resolve_shape_inner`). Folding material production into the single resolve walk fixes #5550 and this together.
- **Suggested Fix**: Make `resolved_shape_structs_in` scan only the `resolve_shape_inner` body (slice to the next column-0 `fn`) and assert the slice is non-empty, or fold the material production into `resolve_shape_inner`.

_Source: `docs/audits/AUDIT_NIFAL_2026-10-11.md` finding D6-01 (HEAD c6e48cbc6)._

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **TESTS**: A regression test pins this specific fix
