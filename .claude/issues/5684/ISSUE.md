# PEX-D4-2026-10-12-01: `papyrus-parser.md` and two code docs still describe the pre-#5398 flag sets

Labels: low,scripting,documentation,doc-rot
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5684

Filed from `docs/audits/AUDIT_PAPYRUS_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Papyrus Lexer & Parser
- **Untrusted-Input**: No
- **Location**: `docs/engine/papyrus-parser.md:78` (grammar line lists only `[Native] [Const] [DebugOnly]
  [Hidden]`), `:128` (`Variable` fields omit `is_hidden`), `:148` (`ScriptFlags` lists four flags;
  now seven), `:151` (`GroupFlags` omits `COLLAPSED`); `crates/papyrus/src/parser/script.rs:680`
  (`parse_struct` doc: "no flag annotations beyond what `Variable` carries", but a member may now be
  `Hidden`)
- **Status**: NEW
- **Description**: `ce90b481c` changed `ast.rs` and the parser but did not touch the reference doc.
  The doc's own flag table (the only place a reader looks up what the AST can express) now omits
  `CONDITIONAL`, `BETA_ONLY`, `DEFAULT`, `GroupFlags::COLLAPSED` and `Variable.is_hidden`, and the
  grammar summary rejects scripts the parser accepts. `docs/engine/papyrus-parser.md` was last
  touched in `3b7ffa670` (#5021).
- **Impact**: Documentation only.
- **Suggested Fix**: Update the four table rows and the grammar line, note that `Default` and
  `Collapsed` are matched contextually (not lexer keywords), and fix the `parse_struct` doc.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites

