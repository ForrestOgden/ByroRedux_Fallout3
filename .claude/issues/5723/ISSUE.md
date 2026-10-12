# NIF-D5-2026-10-12-01: Two comments still say Oblivion `LimitedHinge` zeroes `Perp Axis In B1`, which #5537 made false; they sit beside the plain-hinge arm that does still zero it

- **GitHub**: #5723 (https://github.com/matiaszanolli/ByroRedux/issues/5723)
- **Labels**: low,nif-parser,nif,physics,game:oblivion,documentation,doc-rot
- **Report**: docs/audits/AUDIT_NIF_2026-10-12.md

**Severity**: LOW (doc rot after a fix)

## Location
- `crates/nif/src/blocks/collision/constraints.rs:346-348` (`parse_hinge_oblivion` doc: "`Perp Axis In B1` … is zeroed, exactly as [`Self::parse_oblivion`] does for the limited hinge")
- `crates/nif/src/import/collision/ragdoll.rs:387-394` (`limited_hinge_joint` doc: "`perp_axis_in_b1` is zeroed by `LimitedHingeCInfo::parse_oblivion` on Oblivion/Morrowind content … matching this function's pre-fix behavior exactly for that era")

Game affected: Oblivion (NIF <= 20.0.0.5, `bsver 11`) and Morrowind.

## Description
`526cb07d9` (#5537) changed `LimitedHingeCInfo::parse_oblivion` (`constraints.rs:391-415`) to derive `perp_axis_in_b1 = PerpB2 × AxisB`, and corrected `docs/engine/physal.md:171-177`. It missed the two code comments above. The `parse_hinge_oblivion` doc now cites `parse_oblivion` as the zeroing precedent, which is backwards. The `ragdoll.rs` comment tells a reader that Oblivion limited hinges still hit the synthesised-perpendicular fallback.

Note (unverified, not part of this fix): the plain Oblivion `bhkHingeConstraint` arm (`parse_hinge_oblivion`, `constraints.rs:349-373`) still zeroes both `perp_axis_in_b1` and `perp_axis_in_b2`, because the layout authors neither. Its limits are the full +-pi, so the arbitrary `frame_rot` reference is mostly inert; a bind-pose angle near +-pi would be the exception. `/audit-physics` owns that.

## Evidence
See the two quoted comments against `LimitedHingeCInfo::parse_oblivion` at `constraints.rs:391-415`.

## Impact
A reader trusts the stale comment and re-introduces the zeroing, or concludes the #2448 defect is still live for Oblivion limited hinges.

## Suggested Fix
Reword the `parse_hinge_oblivion` doc to say the hinge authors no B perpendiculars (so both are zeroed) while the limited hinge derives B1. Rewrite the `limited_hinge_joint` paragraph to say Oblivion limited hinges derive `perp_b`; only the plain-hinge arm still delivers a zero.

**Related**: #5537 (closed), #2448, `docs/engine/physal.md:171`

## Completeness Checks
- [ ] **SIBLING**: `docs/engine/physal.md:171` already corrected by #5537; check `docs/engine/nif-parser.md` and `docs/engine/physics.md` for the same claim

Fix with: `/fix-issue 5723`
