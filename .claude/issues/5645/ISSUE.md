# SCR-D3-2026-10-11-04: `subject_requirement` evaluates `GetRandomPercent` once "for every subject", breaking the exact-superset pre-filter contract (FO4 alias fills)

Labels: low,scripting,quests,game:fo4,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 3
- **Location**: `condition.rs:321-349, 1140-1177`; `scene/quest_alias.rs:932-943`
- **Status**: NEW

`subject_requirement` evaluates `GetRandomPercent` once "for every subject". It is stochastic and consumes RNG state, which breaks the exact-superset pre-filter. FO4 has 21 alias CTDAs on fn 77 (Skyrim 0). Add an `is_deterministic()` and treat non-deterministic as unconstrained

