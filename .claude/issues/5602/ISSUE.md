# TD3-2026-10-11-06: ROADMAP.md "Crate Map" lists 22 of 34 workspace members; CLAUDE.md's Workspace Structure omits 15 crates

URL: https://github.com/matiaszanolli/ByroRedux/issues/5602
Labels: low, tech-debt, documentation, doc-rot
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments
- **Location**: `ROADMAP.md:783-807` (Crate Map); `CLAUDE.md` "Workspace Structure" tree
- **Status**: NEW
- **Age**: map unchanged for the crates added since M44 (`boot-request`, `game-detect`, `settings-io`, `save`, `hkx`, `fsr3-sys`, `mod-runtime`, `menuxml`, `pex`, `sdk`, tools `byro-detect`, `byro-launcher`).
- **Effort**: small
- **Evidence**: workspace `members` block = 34 (matches ROADMAP Project Stats "34"). ROADMAP Crate Map misses the 12 above; CLAUDE.md additionally has no entry for `physics`, `spt`, `sfmaterial`, `facegen`, `audio`, `debug-ui`, `bgsm`/`hkx`, `byro-launcher`, `texture-upscale` (`for c in ...; grep -c "\b$c\b" CLAUDE.md` = 0). Several are the "young/unswept" crates this audit is told to weight.
- **Impact**: the two docs an auditor reads first for "where does X live" cannot answer it for `save`, `mod-runtime`, `sdk`, `pex`, `menuxml`, `physics`, `audio`.
- **Related**: #4351/#5105 (CLAUDE.md structure, closed — fixed file names, not coverage).
- **Suggested Fix**: add the 12 rows to ROADMAP's Crate Map (one line each; `_audit-owners.md` already has the ownership text to lift). For CLAUDE.md, keep it short: a single "Other crates" list; CLAUDE.md edits are a user decision.
