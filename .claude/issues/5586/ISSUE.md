# TD1-2026-10-11-05: `crates/scripting/examples/mq101_conformance.rs::run` is a 1454-line, depth-15 function — the worst in the workspace

URL: https://github.com/matiaszanolli/ByroRedux/issues/5586
Labels: low, scripting, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 1
- **Location**: `crates/scripting/examples/mq101_conformance.rs:448-1902` (file prod LOC 1927, on the watch list)
- **Status**: NEW (file was on the 10-08 > 1900 list at 1923/1927; the function itself was never called out)
- **Effort**: medium
- **Description**: `fn run()` is the whole probe — ESM load, BSA/PEX/decompiler ingress, per-fragment lowering, quest/scene/package checks, voice archive, and
  report — in one body, with brace depth 15. Everything before it (L117–447) is already small, well-named helpers (`mq101_player_callback_contract`, `effect_kind`,
  `statement_shape`…), so the shape is obviously "helpers extracted, driver never decomposed". It is a manual conformance harness (needs
  a Skyrim install), so correctness risk is low; the cost is that the probe is hard to extend for the next vertical slice. It is also the only
  `examples/` item in the top 20.
- **Evidence**: `scan.py` → `(1454, depth 15, crates/scripting/examples/mq101_conformance.rs, run, L448)`; second-worst nest anywhere is 10.
  `git log -- <file>`: last touched 2026-10-06 (`9813af435`).
- **Impact**: low (example, not shipped). Listed because the SKILL's nesting > 5 / fn > 200 rules have no examples carve-out.
- **Related**: none filed (`gh issue list --search mq101_conformance` → #3498, #2658, both closed doc issues).
- **Suggested Fix**: split `run` into one `fn check_*` per stage returning `Checks` deltas (`check_ingress`, `check_scripts`, `check_fragments`,
  `check_scenes`, `check_voice`), pass `&mut Checks`; flattens depth to ≤ 4.
