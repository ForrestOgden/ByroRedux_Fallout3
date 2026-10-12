# REN-D3-2026-10-12-01: shader-pipeline.md and memory-budget.md still quote the 1 366-slot bind-inverse upload cap; #5583 made it 1 364

- **GitHub**: #5703 (https://github.com/matiaszanolli/ByroRedux/issues/5703)
- **Labels**: low,renderer,documentation,doc-rot
- **Report**: docs/audits/AUDIT_RENDERER_2026-10-12.md

**Severity**: LOW (doc-rot)

## Location
`docs/engine/shader-pipeline.md` (constants table row `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME`, "1 366", line ~701); `docs/engine/memory-budget.md` (footnote to the bone-buffer table, "`1 366 × MAX_BONES_PER_MESH(144) × 64 B ≈ 12.6 MB`", line ~187).

## Description
#5583 defined `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME = MAX_SKIN_SLOTS` = 1364. The Rust docs and `main.rs` comments were updated; the two reference docs were not. `shader-pipeline.md` still tells a reader the cap differs from the 1 364 allocatable slots listed one row above it, which was true before #5583 and is the exact mismatch #5583 removed.

## Evidence
`MAX_SKIN_SLOTS = 196608 / 144 - 1` = 1364 (`crates/renderer/src/vulkan/scene_buffer/constants.rs`); `docs/engine/shader-pipeline.md` row directly below `MAX_TOTAL_BONES` quotes 1 366. `1364 × 144 × 64 B` = 12.57 MB, so the "≈ 12.6 MB" figure survives, only the multiplier is stale.

## Impact
Documentation only. The skill's `gpu_material_size_claims` guard does not cover these two docs.

## Suggested Fix
Write "1 364 (= `MAX_SKIN_SLOTS`)" in both places and say the cap is the slot pool's capacity by construction.

**Related**: #5583, #1284, #5603 (docs catch-up family)

## Completeness Checks
- [ ] **SIBLING**: `grep -rn '1 366\|1366' docs/ crates/ byroredux/` for any other quote of the old cap

Fix with: `/fix-issue 5703`
