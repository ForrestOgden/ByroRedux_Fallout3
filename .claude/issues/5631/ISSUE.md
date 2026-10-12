# SCR-D1-2026-10-11-02: `Stmt::Return(None)` is accepted anywhere in a fragment, so an early return inside an `If` lowers as nothing and the tail runs unconditionally

Labels: high,scripting,bug

- **Severity**: HIGH (skill escalation: a wrong AST is accepted) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/translate/effects.rs:514-515`
- **Status**: NEW. The provider-seam sibling restricts it to the tail: `papyrus_provider/lower_program.rs:509`
- **Description**: the arm's comment calls `Return` "Champollion's fragment terminator", but nothing restricts it to the final statement at depth 0. A `.psc` body `If Self.GetStageDone(10) / Return / EndIf / Self.SetStage(20)` lowers to `[Conditional{then: []}, SetStage(20)]`, which applies the `SetStage` even when stage 10 is done. `A(); Return; B()` lowers dead code as live.
- **Evidence**: constructible with the `.psc` parser. What the `.pex` decompiler emits for a mid-function `Return` is **unverified** (no early-return test in `crates/pex/src/decompile`), so reach via `.pex` is unconfirmed.
- **Impact**: a quest stage or object effect fires in a state the authored script guards against, with no fallback.
- **Suggested Fix**: accept `Return(None)` only when `depth == 0 && statement_index + 1 == body.len()`, otherwise decline. Pin the three shapes.
