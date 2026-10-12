# PEX-D3-2026-10-12-02: `.pex` lowering maps script and variable user flags but not property user flags, so the two frontends still disagree on `PropertyFlags`

Labels: low,scripting,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5685

Filed from `docs/audits/AUDIT_PAPYRUS_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Decompiler Boolean/Control-Flow/Lower
- **Untrusted-Input**: Yes (it is `.pex` input, but only a silent omission)
- **Location**: `crates/pex/src/decompile/lower.rs:360-397` (`lower_property`: `flags` is only ever
  `AUTO` or `AUTO_READ_ONLY`; `prop.user_flags` is never read), against `crates/papyrus/src/ast.rs:108-117`
  (`PropertyFlags` also has `CONST`, `MANDATORY`, `HIDDEN`, `CONDITIONAL`, all of which the `.psc`
  frontend parses)
- **Status**: NEW (the property half of the gap #5398 closed for scripts and variables)
- **Description**: #5398's stated aim is "both frontends agree". It maps the object's `conditional`,
  `hidden` and `default` bits and a variable's `conditional` bit. `model::Property` carries
  `user_flags` just the same, but `lower_property` drops them. A property declared
  `Int Property P Auto Conditional Hidden Mandatory` lowers from `.psc` with `CONDITIONAL | HIDDEN |
  MANDATORY` and from `.pex` with `AUTO` only.
- **Evidence**: `.psc` probe: `Int Property P Auto Conditional Hidden` parses with those flags set.
  `lower_property` builds `flags` solely from `has_auto_var()` / readable / writable. A grep for
  consumers of `PropertyFlags` / `ScriptFlags::CONDITIONAL` finds none outside `crates/papyrus` and
  `crates/pex`, so nothing misbehaves today.
- **Impact**: None today. It would bite the first consumer that keys on a property's `Conditional` (the
  property half of what `GetVMQuestVariable`-style conditions read) or on `Mandatory`, and it makes
  `.psc`-vs-`.pex` AST equality checks impossible for any script with flagged properties. Also note the
  `Variable.is_hidden` flag is never set from `.pex` (struct definitions are not lowered at all).
- **Related**: #5398, #5443 (the other cross-frontend divergence, `is` lowering to `Cast`)
- **Suggested Fix**: Resolve the `hidden`/`conditional`/`mandatory`/`const` bits through the same
  per-`Pex` mask table proposed for finding 01 and OR them into `lower_property`'s `flags`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix

