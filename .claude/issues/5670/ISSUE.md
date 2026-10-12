# #5670 — PAR-D2-2026-10-12-05: a present-but-undecodable texture in the preferred resolution set is silently replaced by another set's copy

**Labels**: `low`, `bug`, `import-pipeline`, `ui`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-2026-10-12-05) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `crates/menuxml/src/menu.rs:461-482`. `undecodable` is logged only when *no* candidate decodes.
- **Status**: NEW (5c53ad9c7, #5400).
- **Trigger Input**: with the preferred set `menus`, `textures\menus\x.dds` is present but undecodable while `textures\menus80\x.dds` decodes.
- **Description**:
  - Keeping the search going past an undecodable candidate is correct.
  - But when a lower-priority set then succeeds, the substitution is silent, and the result is cached.
- **Evidence**: probe `fallback` (real `MenuRenderer`, counting logger, 3 frames): probes `["textures\\menus\\hud\\a.dds", "textures\\menus80\\hud\\a.dds"]`, warnings `[]`.
  - The same probe confirms that priority order is preserved, each candidate is probed once, and nothing is retried per frame.
- **Impact**: a degraded result (another set's art) with no log line. Cosmetic.
- **Related**: #5400.
- **Suggested Fix**: when `decoded.is_some() && !undecodable.is_empty()`, warn once per cache key.

## Completeness Checks
- [ ] **SIBLING**: The font-atlas resolution path is checked for the same silent substitution
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
