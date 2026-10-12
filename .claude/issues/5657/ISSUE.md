# SCR-D7-2026-10-11-04: Unbounded `Utility.Wait` plus a drop-newest continuation cap lets one mod starve every latent provider handler

Labels: low,scripting,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 7
- **Location**: `lower_program.rs:662-690`; `execute.rs:363-368`
- **Status**: NEW

**Untrusted-Input: Yes.** `Utility.Wait(3.0e38)` lowers, and the continuation cap drops the newest, so one mod can park 4 096 immortal saved continuations and starve every other latent handler. Cap `Wait` at lowering, or evict oldest-first

