# SCR-D1-2026-10-11-04: assignment to a non-local identifier (script variable or property) lowers to nothing

Labels: medium,scripting,bug

- **Severity**: MEDIUM (latent; the corpus was not censused) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `effects.rs:508-513`, `:590-631` (`bind_local`)
- **Status**: NEW
- **Description**: `Stmt::Assign { target: Expr::Ident(name) }` never checks that `name` is a function-local. A pure-literal write to a script variable is recorded as `Binding::Plain` and emits no effect. Only field/index targets decline. `ScriptVariables` is modeled (`GetVMScriptVariable`), so the write is observable state.
- **Suggested Fix**: require the name to already be a declared local, otherwise decline.
