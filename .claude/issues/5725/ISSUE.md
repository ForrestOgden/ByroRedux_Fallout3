# NIF-D4-2026-10-12-01: `docs/engine/nif-parser.md` and `docs/engine/animation.md` still describe the pre-window import module map, the #1269-only walker guard and the unbounded B-spline resampler

- **GitHub**: #5725 (https://github.com/matiaszanolli/ByroRedux/issues/5725)
- **Labels**: low,nif-parser,nif,animation,documentation,doc-rot
- **Report**: docs/audits/AUDIT_NIF_2026-10-12.md

**Severity**: LOW (doc rot; `nif-parser.md` is the reference `/audit-nif` tells auditors to prefer over re-deriving)

## Location
- `docs/engine/nif-parser.md:88-103` (module map): lists `collision.rs  Havok→engine transform + bhk*Shape → …`, but `crates/nif/src/import/collision/` is a directory (`mod.rs`, `ragdoll.rs`, `shape.rs`); the map omits `attach_light.rs` (new in `c1a99c25e`), `units.rs` and `precombine.rs`.
- `docs/engine/nif-parser.md:534-535`: "Depth + cycle guards on the import walkers (#1269)". It does not mention that #5485 (`cf8730adc`) added the shared per-walk `revisit_guard` on all five walkers, and #1269 alone left them unbounded.
- `docs/engine/animation.md:333-343` (B-splines): describes only "resampling at a fixed `BSPLINE_SAMPLE_HZ`". It does not mention the sequence-span intersection, the 108 k per-channel ceiling, the 1 M-key per-`import_kf` budget (`BsplineSampling`, #5486/#5572/#5575/#5577), or the new NiPathInterpolator composition (#5571; that one is documented in `nifal.md:597`).

Game affected: all.

## Description
The window's code changes updated `nifal.md` and `physal.md` but not these two reference documents.

## Evidence
`ls crates/nif/src/import/collision` gives `mod.rs`, `ragdoll.rs`, `shape.rs`; the doc still shows `collision.rs` as a single file.

## Impact
An auditor or contributor who follows the module map looks for `import/collision.rs`, doesn't find `attach_light.rs`, and takes the walker-bound doctrine from a bullet that describes the pre-#5485 state.

## Suggested Fix
Update the `import/` block of the module map, extend the "Depth + cycle guards" bullet to name `revisit_guard` / `new_visited` and #5485, and add a "Bounds" paragraph to the `animation.md` B-spline section (span intersect sequence, 108 k ceiling, 1 M-key budget, static-pose fallback, `usable_sequence_span`).

**Related**: #5485, #5486, #5571, #5313 (a different issue: code comments pointing at moved files)

## Completeness Checks
- [ ] **SIBLING**: Run `.claude/commands/_audit-validate.sh` after the edit so no backticked path in the two docs is stale

Fix with: `/fix-issue 5725`
