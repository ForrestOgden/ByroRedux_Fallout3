# #5669 — PAR-D4-2026-10-12-01: the #5400 header census gates only Oblivion `textures\menus\`, not `menus80`/`menus50` (which the loader also serves) or FO3/FNV `textures\interface\`

**Labels**: `low`, `bug`, `import-pipeline`, `ui`, `test-gap`, `game:oblivion`, `game:fo3`, `game:fnv`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D4-2026-10-12-01) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Corpus Gates
- **Location**: `crates/menuxml/tests/vanilla_corpus.rs:204-212`, the `starts_with("textures\\menus\\")` filter in `menu_texture_header_census_decodes`.
- **Status**: NEW. It is a scope gap in #5400's census.
- **Trigger Input**: n/a.
- **Description**:
  - The census covers 1,975 of Oblivion's 5,476 base-game menu-set DDS. It skips `menus80` (1,712) and `menus50` (1,789), both of which `MenuRenderer::texture` resolves.
  - FO3/FNV HUD art lives in `textures\interface\`, which no test decodes. That includes 24-bpp `hud_comp_direction_vertical.dds` and the 32-bpp `target_*` and `shared\line\*` files.
  - Nothing asserts a vanilla count, so the census can pass vacuously.
- **Evidence**: `ddscensus` decodes all of them today: Oblivion (all three sets plus SI) 6,023/6,023, FO3 1,157/1,157, FNV 1,767/1,767.
- **Impact**: no current failure; this is lost regression coverage for the FO3/FNV HUD driver's art.
- **Related**: #5400, #4665.
- **Suggested Fix**:
  - Widen the Oblivion filter to `textures\menus*`.
  - Add the census over `textures\interface\` to `fo3_corpus.rs` and `fnv_corpus.rs`.
  - Assert the vanilla counts.

## Completeness Checks
- [ ] **SIBLING**: The FO3 and FNV corpus files (`crates/menuxml/tests/fo3_corpus.rs`, `fnv_corpus.rs`) gain the same census
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
