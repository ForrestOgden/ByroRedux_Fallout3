# TD3-2026-10-11-09: Index and cross-reference drift: `docs/engine/index.md` omits 27 of 81 docs, `testing.md` dispatch-test count is 36% low, one `streaming.rs:752` cite remains, `p6-loading-model.sh` is not in the smoke README

URL: https://github.com/matiaszanolli/ByroRedux/issues/5604
Labels: low, tech-debt, documentation, doc-rot
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments
- **Location**:
  - `docs/engine/index.md` (last edited 2026-09-14, `1b01ad64a`): no entry for `story-manager.md`, `dialogue-trees.md`, the six `charal-*-ruleset.md`, `playable-vertical-slice.md`, `rt-lighting-material-recovery.md`, `interior-godrays-status.md`, `exterior-readiness-plan.md`, `m47-*` (4), `p2-*`/`p4-*` fixtures, `sdk-v0.1-*` (2), `starfield-esm-*` (2), `nif-engine-translation-layer.md`, `near-term-action-plan.md`, `m27-phase1-survey.md`, `dependency-update-followups-plan.md` (loop: `for f in docs/engine/*.md; grep -q $b docs/engine/index.md`).
  - `docs/engine/testing.md:51`: "~67 tests today across shader, havok, interpolators, controllers, extra_data, nodes, effects, starfield" — `grep -rc "#\[test\]" crates/nif/src/blocks/dispatch_tests/` sums to 106 across 10 files (adds `legacy_particle.rs`, `mod.rs`).
  - `docs/engine/exterior-grid-streaming.md:96`: `world_pos_to_grid` "(`streaming.rs:752`)"; it is `byroredux/src/streaming/mod.rs:888`.
  - `docs/smoke-tests/README.md`: no entry for `p6-loading-model.sh` (CLAUDE.md names it and says all smokes follow the README pattern); `m41-ragdoll.sh` is in neither index (referenced only from `physal.md:354`).
- **Status**: NEW
- **Effort**: small (index) / trivial (the rest)
- **Impact**: the index is the declared entry point for engine internals; newer subsystems (Story Manager, dialogue, CHARAL rulesets) are discoverable only by `ls`.
- **Suggested Fix**: add the 27 rows; bump testing.md to the live count (or say "see `dispatch_tests/`" without a number); re-point the cite; add the two smoke scripts to the README.
