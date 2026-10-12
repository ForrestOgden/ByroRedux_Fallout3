# #5667 — PAR-D1-2026-10-12-03: the BC1/BC2/BC3 decoders still allocate the full RGBA canvas before checking the block payload; #5377's "size before allocate" covers only the uncompressed path, and its guard cannot fail

**Labels**: `low`, `bug`, `import-pipeline`, `ui`, `test-gap`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D1-2026-10-12-03) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Size Discipline
- **Location**:
  - `crates/menuxml/src/tex.rs:190-191`, `:205-206`, `:232-233`: `decode_dxt{1,3,5}` call `Rgba8::new` before the per-block `off + N > bytes.len()` check.
  - Guard test: `tex.rs:476` (`truncated_payload_is_rejected_before_allocation`).
- **Status**: NEW. It is a residual of #5377, whose suggested fix was applied to `decode_uncompressed` only.
- **Trigger Input**: a 128-byte DDS with FourCC `DXT1` or `DXT5`, 8192×8192, and no payload.
- **Description**:
  - The BC paths reserve 256 MiB and then return `None` at the first missing block.
  - The test asserts only `is_none()`. A mutant that allocates before the uncompressed check still passes it (`mut-allocfirst`).
- **Evidence**: `ddscases` (counting allocator, real `decode_dds`):
  - `DXT1 8192² no payload`: `None`, peak allocation 268,435,456 B.
  - `A8R8G8B8 8192², 4 B payload`: peak allocation 0 B.
- **Impact**: low. The allocation is `calloc`, so it is mostly virtual and freed on return. The real cost is that the guard gives false assurance.
- **Related**: #5377, PAR-D1-2026-10-12-01.
- **Suggested Fix**:
  - Check `128 + div_ceil(w,4) * div_ceil(h,4) * block_bytes <= len` before `Rgba8::new` in each BC decoder.
  - Make the test assert the allocation order, for example with a counting allocator or a size-returning helper.

## Completeness Checks
- [ ] **SIBLING**: All three BC decoders (`decode_dxt1/3/5`) are fixed, plus any new FourCC arm
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
