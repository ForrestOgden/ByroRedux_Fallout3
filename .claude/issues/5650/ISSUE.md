# SCR-D4-2026-10-11-01: The stub-spawner source pins are half-vacuous: the presence assertion is satisfied by its own literal

Labels: low,scripting,test-gap,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 4
- **Location**: `cell_loader/exterior.rs:1273-1300`; `asset_provider/script.rs:974-991`
- **Status**: NEW

The stub-spawner source pins scan their own file with `include_str!`. The positive `contains` is satisfied by its own literal, so deleting the production call passes. The negative count still works. Use `source_scan::production_text`

