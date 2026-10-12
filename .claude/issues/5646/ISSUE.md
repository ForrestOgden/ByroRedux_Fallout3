# SCR-D3-2026-10-11-05: `GetRandomPercent` returns 0..=100 (101 values); the CK / GECK define 0-99

Labels: low,scripting,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 3
- **Location**: `condition.rs:90-92, 746-755`
- **Status**: NEW

`GetRandomPercent` draws `pick(101)` (0..=100); the CK/GECK define 0–99. `pick(100)`, and fix the variant doc

