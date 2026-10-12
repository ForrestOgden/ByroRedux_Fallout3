# REN-D2-2026-10-12-01: the #5249/#5455 docblock now documents the wrong function after #5565 inserted a helper above its subject

- **GitHub**: #5702 (https://github.com/matiaszanolli/ByroRedux/issues/5702)
- **Labels**: low,renderer,shaders,documentation,doc-rot
- **Report**: docs/audits/AUDIT_RENDERER_2026-10-12.md

**Severity**: LOW (doc-rot)

## Location
`crates/renderer/shaders/include/shadow_transport.glsl`, the comment block that opens "#5249 — visibility of a light segment that ignores the receiver's OWN instance" and the function `traceShadowTransmittanceSkippingInstance` below it.

## Description
The long "#5249 … #5455 … #5565" block was written for `traceShadowTransmittanceSkippingInstance`. #5565 appended its paragraph to the block and then inserted the new `traceShadowGlassLegRemaining` between the block and the function. The block now attaches to the helper. The function it describes (self-skip walk, foreign-hit handoff, the exhausted-budget "reports lit" rule) has no leading comment.

## Evidence
File order is the comment block (line ~179), then `vec3 traceShadowGlassLegRemaining(` (~206), then `vec3 traceShadowTransmittanceSkippingInstance(` (~220). The pin test slices both functions by name, so it is unaffected.

## Impact
The contract that the #5455 and #5565 fixes rely on is no longer next to the code it constrains, which is where the next editor will look.

## Suggested Fix
Move the helper above the block, or give the helper its own two-line comment and leave the block on the walk. `.glsl` comments are invisible to `cargo test`, but the SPIR-V does not change (re-run `scripts/check-shader-artifacts.sh` to confirm).

**Related**: #5565, #5455, #5249, #5029 (same splice-into-doc class)

## Completeness Checks
- [ ] **SIBLING**: Other `include/*.glsl` files where a helper was inserted under an existing docblock (#5029 splice class)

Fix with: `/fix-issue 5702`
