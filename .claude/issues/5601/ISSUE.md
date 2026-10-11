# TD3-2026-10-11-05: `game-compatibility.md` per-game sections quote NIF counts 30-50% below the ROADMAP matrix the doc says it reconciles to

URL: https://github.com/matiaszanolli/ByroRedux/issues/5601
Labels: low, tech-debt, documentation, doc-rot
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments
- **Location**: `docs/engine/game-compatibility.md:126` (FNV "14,881"), `:156` (FO3 "10,989"), `:170` (Skyrim SE "18,862"), `:207` (FO4 "34,995 + 124,871 MeshesExtra")
- **Status**: NEW (the Oblivion row was fixed by #4358)
- **Age**: counts predate the 2026-08-28/29 sweep widening; the doc's own table at `:89-96` was refreshed by `e034de59a` (2026-10-06) and is correct.
- **Effort**: trivial
- **Evidence**: ROADMAP matrix: FNV 20 746 (11 archives), FO3 17 172 (6), Skyrim SE 33 468 (8), FO4 235 082 (8). The doc header says "ROADMAP wins ... Last reconciled 2026-08-26 (#2365)". FO3 is off by 36%, FNV 28%, Skyrim SE 44%, FO4 32%.
- **Impact**: two different "100% clean" denominators in one document; per-game audits quote the bullet.
- **Related**: #4358 (closed), #3726 (closed).
- **Suggested Fix**: replace the four bullets with a pointer to the table at `:89` (ROADMAP is the single home) or the matrix figures.
