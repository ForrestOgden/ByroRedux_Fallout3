# SCR-D6-2026-10-11-04: ObScript argument decode is bounded by the buffer, not by the argument region

Labels: low,scripting,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 6
- **Location**: `obscript_vm.rs:591-614, 655-660`
- **Status**: NEW

**Untrusted-Input: Yes.** Argument payload reads are bounded by the buffer, not the argument region. A straddling last argument reads the next statement's bytes instead of reporting `Malformed`. Not memory-unsafe. Add `o + 3 > end` checks

