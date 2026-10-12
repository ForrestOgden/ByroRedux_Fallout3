# TOOL-D2-2026-10-12-01: `debug-cli.md` "Currently registered (72 components)" table lists 67 rows; the five newest registrations are missing

Labels: low,tech-debt,documentation,doc-rot
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5694

Filed from `docs/audits/AUDIT_TOOLING_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Protocol & Registry (doc rot)
- **Exposure**: developers only
- **Location**: `docs/engine/debug-cli.md:186-272` (table); `:124` and `:370` (default-name wording)
- **Status**: NEW (the count guard #5417 only checks the number, not the rows)
- **Description**: The heading and the transcript footer were bumped to 72, but the table below has 67 rows. Missing: `EatBehavior`, `SleepBehavior`, `EatSleepState` (registered since M42 Eat/Sleep), `EditorPlacement` (`42aab4c09`, field `translation`) and `CreatureActor` (`9bb8bc475`, no fields). The M42 trio was already absent when the count read 67 -> 70. The table also lists `FactionReputation`, which is registered by the hand-built `register_faction_reputation` rather than `register_component::<`; that is fine, but it is why a grep-based comparison is off by one. Separately, #5147 changed the default names (`tex_dump.png`, `tex_dump_2.png`, ...; `screenshot_<secs>.png`, `screenshot_<secs>_2.png`, ...), but `:124` and `:370` still describe a single fixed name.
- **Evidence**: python diff of `register_component::<T>(registry, "Name"` names in `registration.rs` production text against the first-column names in the doc table gives `['CreatureActor', 'EatBehavior', 'EatSleepState', 'EditorPlacement', 'SleepBehavior']`.
- **Impact**: someone looking up which fields `set_field` accepts on an Eat/Sleep actor or the new markers finds nothing; the heading promises a complete list.
- **Related**: #5417, #5147
- **Suggested Fix**: Add the five rows (field lists from `registration.rs`: `EatBehavior`/`SleepBehavior` -> radius, location, form_id; `EatSleepState` -> destination; `EditorPlacement` -> translation; `CreatureActor` -> none) and mention the `_N` suffix at `:124` and `:370`. Extending `debug_cli_component_counts_match_the_registry` to compare names, not just the count, would stop the next drift.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites

