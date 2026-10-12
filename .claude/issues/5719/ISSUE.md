# SAFE-D2-2026-10-12-01: The #5456 CWS inflate is bounded by the file's own `u32` length field, which a hostile header can set to ~4 GiB

- **GitHub**: #5719 (https://github.com/matiaszanolli/ByroRedux/issues/5719)
- **Labels**: low,safety,ui,bug
- **Report**: docs/audits/AUDIT_SAFETY_2026-10-12.md

**Severity**: LOW. A decompression-bomb hardening gap on an archive-sourced input. It is not a regression: the stock Ruffle `decompress_swf` has the same property, and before #5479 the shim's `read_to_end` was fully uncapped.

## Location
`crates/ui/src/prepare.rs:115-130` (`decompress_zlib_after_header`).

## Description
The fix reads the header's declared uncompressed length (`u32` at `[4..8]`) and caps the inflate with `.take(declared_body + 1)`, declining a body that overruns it. The declared value is attacker-controlled and can be `0xFFFF_FFFF`, so a ~4 MB zlib stream (about 1000:1) can make `read_to_end` grow `body` toward 4 GiB. The bound defends against a stream that lies about a small length, not against a stream that claims a large one. Genuine Scaleform movies are small (the corpus census in `AUDIT_PARSERS_2026-10-12.md` inflates every archived `.swf` in five games without trouble). The depth half of #5456 is correct and tested; this is the residual of its "bound the inflate" half.

## Evidence
```rust
let declared_body =
    u64::from(u32::from_le_bytes(swf_data[4..8].try_into().ok()?).saturating_sub(8));
...
let decoder = flate2::read::ZlibDecoder::new(&swf_data[8..]).take(declared_body + 1);
let mut body = Vec::new();
{ decoder }.read_to_end(&mut body).ok()?;
```

## Impact
A crafted mod archive can make the engine allocate up to ~4 GiB while preparing a menu or HUD movie, risking an OOM kill on smaller machines. Retail content is unaffected.

## Suggested Fix
Apply an absolute ceiling (for example `declared_body.min(MAX_SWF_BODY_BYTES)` with a few tens of MiB, well above any shipped movie) before `.take`, and add a test with a header claiming `u32::MAX` over a small bomb stream.

**Related**: #5456, #5479

## Completeness Checks
- [ ] **SIBLING**: Other `ZlibDecoder`/`read_to_end` sites fed by archive content (`/audit-parsers` and `/audit-ui` co-own the reader)
- [ ] **TESTS**: Header claiming `u32::MAX` over a small bomb stream is refused without a multi-GiB allocation

Fix with: `/fix-issue 5719`
