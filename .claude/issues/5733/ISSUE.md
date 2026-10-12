# #5733 — UI-D1-2026-10-12-02: `docs/engine/ui.md` still names `SwfMovie`, gives a stale test count, and never describes the dialect shim or the menu texture budget

**Labels**: `low`, `ui`, `documentation`, `doc-rot`

**Source**: `docs/audits/AUDIT_UI_2026-10-12.md` (UI-D1-02) · audited at HEAD 5d31fd637

- **Severity**: LOW
- **Dimension**: Profile & Bridge (doc rot)
- **Profile**: n/a
- **Location**:
  - `docs/engine/ui.md:104-105` and `:185` (`ruffle_core::tag_utils::SwfMovie::from_data`, "parsed SwfMovie")
  - `docs/engine/ui.md:753` ("99 default tests plus 3 ignored")
  - `docs/engine/ui.md:862-875` (the `tex.rs` and `menu.rs` bullets)
- **Status**: NEW (the earlier `ui.md` rot issues #5029 and #5033 are about other lines)

## Description
- The Ruffle bump (b91e82160) renamed the type to `SwfMovieData`, and `player.rs` and `prepare.rs` now use `SwfMovieData::from_data(…, None)`. `ui.md` still shows the old name in the load diagram and in the `new()` description.
- The Tests section says "99 default tests plus 3 ignored (measured 2026-10-09)". Measured 2026-10-12: 103 passed plus 3 ignored. The four extra tests are the #5456 and #5479 shim pins.
- `ui.md` has no mention of `normalize_scaleform_dialect` (#4470, #5456, #5479) or of `MAX_DECODED_TEXTURE_BYTES` and its resolved-path decode cache in `MenuRenderer` (#5658). Both are load-bearing invariants.
- ROADMAP M48 does mention the shim in one sentence.

## Impact
Documentation only.

## Suggested Fix
- Rename `SwfMovie` to `SwfMovieData` in `ui.md`.
- Refresh the test count line.
- Add one short paragraph each for the shim and the texture budget.

## Completeness Checks
- [ ] **SIBLING**: grep `docs/engine` for other `SwfMovie` mentions
