# TD8-2026-10-11-03: `PerkRecord.perk_flags` is write-only "for backwards-compat with the prior stub" — no consumer outside one test

URL: https://github.com/matiaszanolli/ByroRedux/issues/5623
Labels: low, esm-plugin, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 8
- **Location**: `crates/plugin/src/esm/records/misc/magic.rs:279-281` (decl), `:343-344` (only write; `is_trait` derived from it), `:1040` (only read: test assertion)
- **Status**: NEW
- **Effort**: trivial
- **Description**: Field comment says "Kept for backwards-compat with the prior stub; new code should prefer `is_trait`". `grep -rn perk_flags crates byroredux tools` -> 4 hits, all in magic.rs; no external consumer exists (no external users of a workspace-internal struct).
- **Impact**: Duplicate representation of the same DATA byte; the `u8` implies richer flag semantics than the parser provides.
- **Suggested Fix**: Delete `perk_flags`, parse into a local and set `is_trait`, update the test at magic.rs:1040 to assert `is_trait`.
