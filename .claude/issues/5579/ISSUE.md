# NIFAL-D8-2026-10-11-06: Audit-skill and doc text still describes the pre-#5502/#5503/#5497 state

Labels: low,nifal,doc-rot,documentation,game:starfield

- **Severity**: LOW (doc-rot)
- **Dimension**: Shader-flags/Effects
- **Tier Violated**: n/a
- **Game Affected**: Starfield, FO4
- **Location**: `.claude/commands/audit-nifal/SKILL.md:90` and `:163` (inheritance unresolved, emissive forwarded, #5503 open); `.claude/commands/audit-starfield/SKILL.md:66`; `.claude/commands/audit-parsers/SKILL.md:87`; `.claude/commands/audit-fo4/SKILL.md:78` (MSWP CNAM row called open; closed by 0b2ffed89, field is now `color_remap_index`); `docs/engine/nifal.md:728-733` ("98 % texture resolution", see D8-04)
- **Status**: NEW (route to `/audit-sync`)
- **Description**: An auditor following the skills would re-file a parked arm as live or treat solved inheritance as open. Current state: settings inheritance resolved (217 `IsGlass`, 619 emissive `Enabled`, no coverage signal, classifier inert), emissive parked (#5503 closed), texture-set inheritance open (D8-04), MSWP palette row applied (#5497).
- **Suggested Fix**: Re-sync those lines via `/audit-sync`.

_Source: `docs/audits/AUDIT_NIFAL_2026-10-11.md` finding D8-06 (HEAD c6e48cbc6)._

## Completeness Checks
- [ ] **SIBLING**: every listed skill/doc line re-synced (route via `/audit-sync`)
