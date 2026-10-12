# #5658 — PAR-D1-2026-10-12-01: MenuRenderer's decoded-texture cache has no total bound, so a small mod archive pins gigabytes of RGBA for the HUD's lifetime

**Labels**: `high`, `bug`, `import-pipeline`, `ui`, `memory`, `safety`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D1-2026-10-12-01) · audited at HEAD c1a99c25e

- **Severity**: HIGH
- **Dimension**: Size Discipline
- **Location**:
  - `crates/menuxml/src/tex.rs:89`: the only bound is per image, `width > 8192 || height > 8192`.
  - `crates/menuxml/src/menu.rs:97`: `tex_cache: HashMap<String, Option<Arc<Rgba8>>>`.
  - `crates/menuxml/src/menu.rs:424-485`: `MenuRenderer::texture` keys on the lowercased authored spelling, inserts on every decode and never evicts.
  - Driven every frame from `byroredux/src/hud.rs:703` (`self.renderer.render_frame(&self.assets)`).
- **Status**: NEW. Searches for "menuxml texture cache", "tex_cache", "MenuRenderer memory" and "decoded texture budget" found nothing. It is the texture-side sibling of #5314, which bounded include bytes per document but left decoded textures unbounded.
- **Trigger Input**:
  - A driven menu XML with K visible `<image>` tiles, each naming a distinct `<filename>Menus\bigN.dds</filename>`.
  - Each `bigN.dds` is an 8192×8192 DXT1 of constant blocks: 33,554,560 B raw, 48,898 B compressed in an archive (measured).
  - The cache key is the authored spelling, not the resolved archive path. `Menus\x.dds`, `Menus80\x.dds`, `Menus50\x.dds` and `textures\menus\x.dds` can all resolve to one archive file, and each spelling decodes and keeps its own copy.
- **Description**:
  - `decode_dds` caps one image at 8192², which is 256 MiB of RGBA and 16× the area of the largest vanilla menu texture (2048²).
  - Nothing bounds how many decodes the renderer keeps. `MAX_DOCUMENT_TILES` (16,384) admits thousands of distinct image tiles.
  - Decoding runs synchronously on the HUD render path on first visibility. Allocation failure aborts the process, and no `catch_unwind` can catch an abort.
- **Evidence**: probe `texcache` (real `MenuRenderer::load` and `render_frame`, release):
  ```
  k=1  first render_frame 213.8 ms  VmRSS after   268,708 kB
  k=2  first render_frame 423.8 ms  VmRSS after   530,868 kB
  k=4  first render_frame 840.2 ms  VmRSS after 1,055,188 kB
  second render_frame ~0.15 ms (cache hit), RSS unchanged
  ```
  - Growth is exactly 256 MiB per distinct filename, and the memory is retained.
  - Vanilla calibration (`ddscensus`, decoding every menu-set DDS):

    | Game | Menu DDS files | Largest | Total RGBA if all decoded |
    |---|---|---|---|
    | Oblivion | 6,023 | 2048² | 961 MiB |
    | FO3 | 1,157 | 2048² | 421 MiB |
    | FNV | 1,767 | 2048² | 689 MiB |

- **Impact**:
  - About 1.5 MB of mod archive (32 entries) pins about 8 GiB for the session. The first HUD frame stalls about 7 s in release and about 14 s in debug.
  - About 6 MB of archive (128 entries) aborts a 32 GiB machine at `--hud` load.
  - Amplification is about 5,500:1 against archive bytes, the shape #5314 was rated HIGH for. Unlike the parse-time budget, this memory is held for the renderer's lifetime.
  - Vanilla is unaffected.
- **Related**: #5314, #5329 (the same key-versus-resolved-path class), #5400, PAR-D1-2026-10-12-03.
- **Suggested Fix**:
  - Give `MenuRenderer` a decoded-texel budget sized from the vanilla census (a few multiples of the 961 MiB worst case is still generous for the driven menus). Refuse over-budget decodes as `None` with one warning.
  - Consider lowering the per-image cap toward 4096².
  - Key `tex_cache` on the resolved archive path, so alternate set spellings share one decode.

## Completeness Checks
- [ ] **SIBLING**: The other menuxml DDS decode sites that bypass `tex_cache` (`crates/menuxml/src/font.rs` font-atlas fallback, the `byroredux/src/hud.rs` compass strip) are bounded by the same budget
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
