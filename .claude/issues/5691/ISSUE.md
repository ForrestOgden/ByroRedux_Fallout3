# #5691 — PAR-D2-2026-10-12-07: #5662's policy ("a non-finite scalar drops its track") is bypassed by finite bounds whose span overflows, so `decode_spline_animation` returns `Ok` with NaN/inf translation or scale samples

**Labels**: `medium`, `import-pipeline`, `animation`, `bug`, `game:skyrim`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-07) · audited at HEAD 5d31fd637

- **Severity**: MEDIUM
- **Dimension**: Error Semantics
- **Location**: `crates/hkx/src/animation.rs` — `read_vector_curve` (`values.push(min + quantized * (max - min))`); the finite test covers only the stored `min` and `max`. Consumer: `byroredux/src/asset_provider/animation.rs` `convert_hkx_clip` copies samples into `AnimationClip` keys without a finiteness check.
- **Status**: NEW. Residual of closed #5662, whose title's second half (finite garbage on the same bones "passes unchecked") was left undecided by the fix. Searches of open and closed issues for "HKX non-finite" and "spline overflow" found nothing else.
- **Trigger Input**: a dynamic translation or scale component with finite bounds `min = -3.0e38`, `max = +3.0e38` (both pass `is_finite()` and `min <= max`). `max - min` overflows to `+inf`, so a control point with quantized value 0 evaluates `0 * inf = NaN` and one with value 1 evaluates `inf`.

## Description
- #5662 made non-finite stored scalars drop the track instead of failing the clip. The sampled value is never checked, so the same artifact reaches the clip by arithmetic.
- The original report counted 12 *accepted* vanilla clips with finite garbage up to `|T| = 1.41e22` on the same prop-attachment bones. They are still accepted and nothing downstream sanitises a key value. The only `is_finite` guards in `crates/core/src/animation/` are on phase and time.
- This is a mod-file hazard: vanilla bounds do not overflow.

## Evidence
Probe crate (copy of `crates/hkx/src`, real-data tests cut) with one added test built on the repo's `synthetic_clip` / `dynamic_x_translation` fixtures:
```
let curves = dynamic_x_translation((-3.0e38, 3.0e38));
decode_spline_animation(&synthetic_clip(0, 4, &[[0, 0x10, 0, 0]], &curves))
-> Ok, dropped=[], translation=[NaN, 0.0, 0.0] on both frames
```

## Impact
A hostile or corrupt mod clip puts NaN translation or scale into `AnimationClip`, then `Transform`, then the palette and TLAS instance. It affects only that clip while it plays. Downstream GPU behaviour of a NaN bone was not exercised, so it is not rated higher.

## Related
#5662, #3013, #4396 (same class: finite components overflowing to a bad result), PAR-D1-2026-10-12-02.

## Suggested Fix
After the `Dynamic` control points are pushed, mark the track `non_finite` if any value is non-finite (or compute `max - min` in `f64`). Optionally clamp `|T|` to a sane bound for the 12 vanilla finite-garbage clips. Add a fixture using the `dynamic_x_translation((-3.0e38, 3.0e38))` shape, since the existing `non_finite_*` tests only use NaN and inf stored scalars.

## Completeness Checks
- [ ] **SIBLING**: The rotation/scale paths of the spline decoder and the non-spline HKX channel decoders get the same sampled-value finiteness check
- [ ] **TESTS**: A regression test pins the overflowing-span fixture (`-3.0e38 .. 3.0e38`)
