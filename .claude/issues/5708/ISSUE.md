# #5708 — CHAR-2026-10-12-D5-01: The layer-composition formula in six comments and docs still reads `base + permanent + temporary − damage`, after #5412 added `set_override`

**Labels**: `low`, `character`, `documentation`, `doc-rot`

**Source**: `docs/audits/AUDIT_CHARACTER_2026-10-12.md` (CHAR-D5-01) · audited at HEAD 5d31fd637

- **Severity**: LOW (doc rot)
- **Dimension**: Coverage & Doctrine
- **Game**: all
- **Location**:
  - `crates/scripting/src/condition.rs:84` and `:718` (`GetActorValue` doc and inline comment)
  - `crates/core/src/character/ruleset.rs:148` (the `actor_value` doc)
  - `docs/engine/charal.md:86` and `:163`
- **Status**: NEW. #5412 updated `actor_values.rs`, `charal.md` section 6 and the `inventory.rs` vitals doc; it did not sweep these.

## Description
The live composition is `base + set_override + permanent + temporary − damage` (`actor_values.rs`). The sites above state the old four-layer sum as the definition. `charal.md:86` calls it "one composition" and `:163` "layered `base + permanent + temporary − damage`", which a reader will take as exhaustive. Note: `charal-skyrim-ruleset.md:668` carries the same old sum inside a dated historical narrative of the Carry Weight base-read gap and is correct for the time it describes; it is not part of this finding.

## Evidence
`grep -rn 'base + permanent' crates byroredux docs/engine` on HEAD. `GetActorValue`'s authored-fast-path returns `avs.current()`, which does include the override, so the function behaves as the new formula says while its doc says otherwise.

## Impact
Documentation only; it is what makes D4-01-style hand-sums look correct.

## Related
#5412, #5508, CHAR-2026-10-12-D4-01.

## Suggested Fix
Point each site at `ActorValue::current` / `undamaged` ("see `actor_values.rs` for the layer list") instead of repeating the sum.

## Completeness Checks
- [ ] **SIBLING**: Re-run the grep after the fix to confirm no site repeats the four-layer sum
