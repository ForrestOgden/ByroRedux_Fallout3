# #5712 — CHAR-2026-10-12-D5-03: `charal.md` cites `ruleset.rs:128` for `impl Resource for CharacterRuleset`; it is at line 261

**Labels**: `low`, `character`, `documentation`, `doc-rot`

**Source**: `docs/audits/AUDIT_CHARACTER_2026-10-12.md` (CHAR-D5-03) · audited at HEAD 5d31fd637

- **Severity**: LOW (doc rot)
- **Dimension**: Coverage & Doctrine
- **Game**: all
- **Location**: `docs/engine/charal.md:642`
- **Status**: NEW

## Description
The skill's convention is to cite symbols, not line ranges. This is the only `*.rs:<line>` cite left in `docs/engine/charal*.md`, and it is wrong: `charal.md` says `impl Resource for CharacterRuleset` is at `ruleset.rs:128`, but it is at `crates/core/src/character/ruleset.rs:261`.

## Impact
Documentation only.

## Suggested Fix
Drop the line number: "`impl Resource for CharacterRuleset`, `ruleset.rs`".

## Completeness Checks
- [ ] **SIBLING**: confirm no other `*.rs:<line>` cite remains in `docs/engine/charal*.md`
