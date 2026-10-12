# SCR-D7-2026-10-11-03: Provider lowering accepts `elseif` chains the runtime validator always rejects

Labels: low,scripting,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 7
- **Location**: `lower_program.rs:483-498, 363-372`; `execute.rs:294-297, 436-438`
- **Status**: NEW

**Untrusted-Input: Yes.** `elseif` chains deeper than `MAX_PROVIDER_HANDLER_NESTING` lower fine, but `validate_provider_statements` rejects them on every dispatch with a warn. The handler never runs. Count the elseif index into `depth` at lowering

