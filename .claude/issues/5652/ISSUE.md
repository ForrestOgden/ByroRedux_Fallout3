# SCR-D1-2026-10-11-09: `SetObjectiveDisplayed` drops its 3rd argument (`abForce`) without inspecting it

Labels: low,scripting,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 1
- **Location**: `effects.rs:970-982`
- **Status**: NEW

`SetObjectiveDisplayed` never inspects its 3rd argument (`abForce`), so a non-literal or side-effecting expression is silently dropped. Use `bool_arg(args, 2)?`

