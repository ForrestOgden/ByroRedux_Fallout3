# #5693 — PAR-D1-2026-10-12-04: an over-budget menu texture is refused after the archive fetch, is not remembered by resolved path, and the warning overstates what is refused

**Labels**: `low`, `import-pipeline`, `ui`, `bug`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D1-04) · audited at HEAD 5d31fd637

- **Severity**: LOW
- **Dimension**: Size Discipline
- **Location**: `crates/menuxml/src/menu.rs` — `MenuRenderer::texture`, the #5658 budget arm
- **Status**: NEW (residual of the #5658 fix).
- **Trigger Input**: a menu naming K spellings (`Menus\x.dds`, `Menus80\x.dds`, `textures\menus\x.dds`, …) of one 8192² archive file after the budget is spent.

## Description
- `assets.texture(path)` runs before the budget check, so every refusal pays a full archive extract of the offending file. The refusal is cached only under the authored spelling (`tex_cache`), and the `decoded` map is keyed only on successful decodes. Each additional spelling of a *refused* file therefore re-extracts it (about 33 MB inflated per 8192² DXT1). The cost is transient and bounded by `MAX_DOCUMENT_TILES` (16,384) once per spelling, with no retained memory. It is a first-render CPU stall.
- The log text says "it and any further new textures are not drawn". The check is per texture (`decoded_bytes + len > budget`), so a later, smaller texture that still fits is decoded and drawn. Which textures draw depends on first-visible order. The tests use equal-size textures and cannot tell.

## Evidence
Code reading, `menu.rs` budget arm, with `self.decoded` populated only after a successful `Rgba8::decode_dds`.

## Impact
First-render hitch on a hostile mod menu, plus a misleading operator message. No memory growth.

## Related
#5658, #5667 (BC paths still allocate before checking the payload).

## Suggested Fix
Record refused resolved paths in a small `HashSet<String>` consulted before the fetch, and reword the warning to "this texture is not drawn; later textures that fit the budget still are". Add a test with unequal-size textures.

## Completeness Checks
- [ ] **SIBLING**: Font atlas / `.tex` atlas paths checked for the same fetch-before-budget order
- [ ] **TESTS**: A regression test pins refused-path memoisation and the mixed-size budget behaviour
