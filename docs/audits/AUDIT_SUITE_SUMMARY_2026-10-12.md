# Audit Suite Summary — `--preset comprehensive` — 2026-10-12

**HEAD**: 5d31fd637 · **Audits run**: 29 of 30 (`runtime --game all` deferred — see below) · **Baseline**: each audit's previous report (mostly 2026-10-08/09/11)

| Audit | Findings | CRITICAL | HIGH | MEDIUM | LOW | Report |
|---|---|---|---|---|---|---|
| renderer | 3 | 0 | 0 | 0 | 3 | AUDIT_RENDERER_2026-10-12.md |
| performance | 4 | 0 | 0 | 0 | 4 | AUDIT_PERFORMANCE_2026-10-12.md |
| ecs | 2 | 0 | 0 | 0 | 2 | AUDIT_ECS_2026-10-12.md |
| concurrency | 2 | 0 | 0 | 1 | 1 | AUDIT_CONCURRENCY_2026-10-12.md |
| safety | 3 | 0 | 0 | 0 | 3 | AUDIT_SAFETY_2026-10-12.md |
| nif | 3 | 0 | 0 | 0 | 3 | AUDIT_NIF_2026-10-12.md |
| nifal | 3 (+#5502 carried) | 0 | 0 | 0 | 3 | AUDIT_NIFAL_2026-10-12.md |
| esm | 2 | 0 | 1 | 0 | 1 | AUDIT_ESM_2026-10-12.md |
| parsers | 2 (re-run section) | 0 | 0 | 1 | 1 | AUDIT_PARSERS_2026-10-12.md |
| physics | 5 | 0 | 0 | 1 | 4 | AUDIT_PHYSICS_2026-10-12.md |
| character | 4 | 0 | 0 | 0 | 4 | AUDIT_CHARACTER_2026-10-12.md |
| gameplay | 6 | 0 | 0 | 2 | 4 | AUDIT_GAMEPLAY_2026-10-12.md |
| exterior | 4 | 0 | 1 | 2 | 1 | AUDIT_EXTERIOR_2026-10-12.md |
| ui | 2 | 0 | 0 | 0 | 2 | AUDIT_UI_2026-10-12.md |
| audio | 5 | 0 | 0 | 1 | 4 | AUDIT_AUDIO_2026-10-12.md |
| speedtree | 1 | 0 | 0 | 0 | 1 | AUDIT_SPEEDTREE_2026-10-12.md |
| scripting | 4 | 0 | 0 | 0 | 4 | AUDIT_SCRIPTING_2026-10-12.md |
| papyrus | 3 | 0 | 0 | 1 | 2 | AUDIT_PAPYRUS_2026-10-12.md |
| save | 3 | 0 | 0 | 0 | 3 | AUDIT_SAVE_2026-10-12.md |
| tooling | 3 | 0 | 0 | 0 | 3 | AUDIT_TOOLING_2026-10-12.md |
| legacy-compat | 1 | 0 | 0 | 0 | 1 | AUDIT_LEGACY_COMPAT_2026-10-12.md |
| tech-debt | 17 | 0 | 0 | 0 | 17 | AUDIT_TECH_DEBT_2026-10-12.md |
| fnv | 2 (+1 carried M) | 0 | 0 | 0 | 2 | AUDIT_FNV_2026-10-12.md |
| fo3 | 2 | 0 | 1 | 0 | 1 | AUDIT_FO3_2026-10-12.md |
| skyrim | 2 | 0 | 0 | 1 | 1 | AUDIT_SKYRIM_2026-10-12.md |
| oblivion | 3 | 0 | 0 | 1 | 2 | AUDIT_OBLIVION_2026-10-12.md |
| fo4 | 2 (+1 low cross-check) | 0 | 1 | 1 | 1 | AUDIT_FO4_2026-10-12.md |
| starfield | 2 | 0 | 0 | 1 | 1 | AUDIT_STARFIELD_2026-10-12.md |
| regression | 4 | 0 | 0 | 0 | 4 | AUDIT_REGRESSION_2026-10-12.md |

**Total: ~101 findings reported (0 critical, 4 high, 13 medium, ~84 low). No CRITICAL.**
Unchanged-since-baseline dimensions were skimmed by each audit (listed in each report header).

## HIGH
- **FO3-2026-10-12-D2-01** (#5734) — QUST quest-level dialogue conditions parsed but never evaluated; with quest-priority ordering the generic GREETING is hijacked (all 1,647 FO3 NPC_ bases; FNV 3,788/3,816). Reaches Oblivion/Skyrim/FO4 (latent). Fix needs fns 426/427/372/254/365/566/700 catalogued first (unknown fns fail closed to 0.0).
- **ESM-2026-10-12-D2-01** (#5687) — WTHS DIFF walker recursion has no depth cap → stack overflow on a hostile RDIF blob (reproduced).
- **FO4-2026-10-12-D4-01** (#5755) — FO4 REFR material swap never fires on real data (0/7,132 XMSP match).
- **EXT-2026-10-12-D6-01** — baked-LOD lattice origin for Tamriel wrong (0/36,864 `.btr` hits vs 36,864/36,864). Confirmed on SE + LE. **Regression of closed #2586 — not filed (see below).**

## Regressions of closed issues — NOT filed (no open tracker)
Awaiting a decision on reopening or filing:
- EXT-D6-01 (HIGH, #2586), EXT-D6-02/03 (MEDIUM, #5496 incomplete), GAME-D5-01 (MEDIUM, #5495 — `CreatureActor` never stamped).
- TD2-06 (#5469), TD3-02 (#5470), TD3-03 (#5471); also cited by regression sweep: #5031, #5662 (residual filed as #5691), #5512 (side effect filed as #5715).

## Publishing (`/audit-publish`, all reports, NEW findings only)
88 issues created: #5675–#5762 (ranges interleaved by parallel publishers). 0 stale, 0 unverifiable; skipped as already-open: SCR-D3-2026-10-11-05 → #5646, SCR-D5-2026-10-11-05 → #5642.
Label gaps (no new labels created): `mod-runtime` (#5718), launcher/byro-detect (#5717, #5739), `hkx`/`menuxml` (#5691, #5693, #5744), `bsa`/`material` (#5735, #5760), audit infrastructure (#5758).

## Not run
- **`runtime --game all`** — started as the last leg, then cut off before writing a report (session ended). Re-run `/audit-runtime --game all` when the engine is idle. Known baseline gap: `.claude/audit-baselines/runtime/fo4-InstituteBioScience.tsv` predates the #5484 fix.
- Needs-GPU items carried: Cornell glass-pane probe for #5565; `cornell_forced_low_blas_budget_preserves_rt_shadows` (REG-2026-10-12-03, #5761); live Tamriel `lod.coverage` check for EXT-D6-01.

## Housekeeping
Per-audit scratch dirs removed; four private cargo/probe targets under `/mnt/data/tmp` removed (~1.2 GB). The runtime agent's 21 orphan Xvfb displays were killed by PID; older `:99`/`:100` left alone.
