# SCR-D6-2026-10-11-03: ROADMAP still says FO3/FNV SCPT bytecode is not executed; it is

Labels: low,scripting,documentation,doc-rot

- **Severity**: LOW
- **Dimension**: Scripting Dim 6
- **Location**: `ROADMAP.md:687-688`, `:269`; `asset_provider/script.rs:573`; `update.rs:456`
- **Status**: NEW

ROADMAP says FO3/FNV SCPTs are not executed. They are (322 FNV and 101 FO3 quest scripts), with ungated install and tick. Command ids are shared across the three games (agent census). Update the ROADMAP and record the census in `docs/engine/scripting.md`

