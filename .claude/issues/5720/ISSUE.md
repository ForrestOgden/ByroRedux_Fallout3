# NIF-D1-2026-10-12-01: `BSLightingShaderProperty` / `BSEffectShaderProperty` report an opaque-tail length on every game, so `opaque_tail_histogram` is non-empty for every Skyrim / FO4 / FO76 file, against its own "empty for every non-Starfield file" doc

- **GitHub**: #5720 (https://github.com/matiaszanolli/ByroRedux/issues/5720)
- **Labels**: low,nif-parser,nif,game:skyrim,game:fo4,bug
- **Report**: docs/audits/AUDIT_NIF_2026-10-12.md

**Severity**: LOW (telemetry semantics plus a false doc comment; no parse effect)

## Location
- `crates/nif/src/blocks/shader/lighting.rs:1080-1082` (`opaque_tail_len` returns `Some(self.starfield_tail.len())` unconditionally)
- `crates/nif/src/blocks/shader/effect.rs:354-356` (same)
- `crates/nif/src/scene.rs:124-125` (the doc: "Empty for every non-Starfield file — only blocks overriding `opaque_tail_len` contribute")
- consumer `crates/nif/src/lib.rs:505-509`

Game affected: Skyrim LE/SE (`bsver 83`/`100`) and FO4/FO76 (`bsver 130`/`155`). Starfield is the intended population.

## Description
`starfield_tail` is an empty `Vec` unless `read_starfield_tail` captured something, and that only happens for `bsver >= STARFIELD`. But `opaque_tail_len` returns `Some(len)` rather than `None`, and `lib.rs:505` records any `Some`. Every `BSLightingShaderProperty` and `BSEffectShaderProperty` on every game therefore bumps `opaque_tail_histogram[type][0]`.

## Evidence
`driftcensus` / `parsecensus` output:
```
Skyrim SE Meshes1: tails={"BSEffectShaderProperty": {0: 2343}, "BSLightingShaderProperty": {0: 6023}}
files with a non-empty opaque_tail_histogram: Skyrim LE 17,015 of 17,216; SSE Meshes0 18,609 of 18,862; SSE Meshes1 3,143 of 3,185
Oblivion / FO3 / FNV: tails={} (their lighting property is the older BSShaderPP family, not the Skyrim one)
```

## Impact
There is no parse effect. A consumer that follows the documented contract, treating "non-empty tail histogram" as "this file has a Starfield-style opaque tail", gets a false positive on every Skyrim+ file. Each such block also does a histogram bump on the hot parse loop for no signal. The doc is already wrong as written.

## Suggested Fix
Return `Some(len)` only when the block actually captured a tail (`(!self.starfield_tail.is_empty()).then_some(...)`) or when `bsver >= STARFIELD`, so Starfield's real zero-length tails (480,757 blocks) still count. Otherwise, correct the `scene.rs` doc to say "every `BSLightingShaderProperty` / `BSEffectShaderProperty` records its tail length, 0 on pre-Starfield".

**Related**: #2625 (introduced the histogram)

## Completeness Checks
- [ ] **SIBLING**: Every other `opaque_tail_len` override (`grep -rn opaque_tail_len crates/nif`); `BSFaceGenNiNode`/`BSWeakReferenceNode` are Starfield-only and unaffected
- [ ] **TESTS**: Extend `every_tail_capturing_block_reports_it_and_parse_nif_records_it` (`blocks/shader_tests/starfield.rs`) to pin the gating

Fix with: `/fix-issue 5720`
