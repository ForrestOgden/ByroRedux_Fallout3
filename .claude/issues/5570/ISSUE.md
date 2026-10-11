# REN-D11-2026-10-11-01: CLAUDE.md still states the pre-#5482 presentation order

GitHub: https://github.com/matiaszanolli/ByroRedux/issues/5570
Labels: low,renderer,documentation,doc-rot
Source: docs/audits/AUDIT_RENDERER_2026-10-11.md (filed snapshot; GitHub is authoritative for state)

- **Severity**: LOW (documentation)
- **Dimension**: FSR/Presentation
- **Location**: `CLAUDE.md`, the `src/vulkan/presentation.rs` line of the workspace tree ("output-res tonemap(graded × exposureTex)").
- **Status**: NEW (sibling sweep gap of closed #5532)
- **Description**: Since #5482 `presentation.frag` computes `exposed = scene.rgb * exposure` first, grades the exposed radiance (saturation → `gradeContrast` → brightness → tint), applies the #5154 chroma compress, and only then `tonemap(compressed)`. `shader-pipeline.md` and `renderer.md` already carry the new order; `CLAUDE.md` is the doc loaded at every session start.
- **Impact**: misleads anyone reasoning about where contrast pivots relative to exposure.
- **Related**: #5482, #5532, #5219.
- **Suggested Fix**: "Presentation pass — output-res exposure → IMGS/IMAD grade → chroma compress → tonemap (ACES|AgX), underwater extinction, swapchain write". (Editing `CLAUDE.md` is the user's call; not done by this audit.)
