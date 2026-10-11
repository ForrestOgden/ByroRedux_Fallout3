# TD4-2026-10-11-03: The validator's skill-symbol advisory is the only signal for renamed guards, and its output buries it among false positives

URL: https://github.com/matiaszanolli/ByroRedux/issues/5607
Labels: low, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 4, Audit-Finding Rot (gate coverage)
- **Location**: `.claude/commands/_audit-validate.sh` (ADVISORY audit-skills block, plus the basename block at the top of the output).
- **Status**: NEW (the prior report's TD4-01 covered pathspecs only)
- **Effort**: small
- **Description**:
  - The "audit skills" symbol advisory had 6 true positives out of 8 (TD4-01). The two others are `*.env` constants that the source scan does not read.
  - The "deleted-file refs" advisory has 73 entries. Its 3 skill entries are all non-defects:
    - `triangle_early.frag` is the extractor truncating `triangle_early.frag.spv` (5 sites in audit-performance and audit-renderer).
    - `ImportedMaterial.sh` is the truncated `ImportedMaterial.shader_type` (audit-skyrim:60).
    - `streaming.rs` at audit-nif:345 is a deliberate historical mention of the split.
  - The remaining 70 are `docs/engine` entries, many of them non-path tokens such as `time.sh`, `quest.sh` and `mem.frag`.
  - A reader scanning the output learns to ignore the advisory sections.
- **Evidence**: `/tmp/audit/tech-debt/d4_validate.txt` (the 73-entry basename list and the 8-entry skill symbol list).
- **Impact**: The gate answers "OK" while real renames wait in an advisory block. This is the TD4-2026-10-08-01 failure mode, in a smaller form.
- **Suggested Fix**:
  - Treat the skill-symbol advisory (8 entries) as a failing gate, and allow-list the `.env` constants by also scanning `docs/smoke-tests/fixtures/*.env`.
  - Teach the basename extractor to keep a `.spv` suffix and to ignore the `Type.field` shape.
  - Italicise `streaming.rs` at audit-nif:345.
  - Keep the `docs/engine` advisory advisory.
