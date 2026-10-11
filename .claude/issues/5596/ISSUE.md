# TD2-2026-10-11-13: Absolute-offset little-endian readers re-implemented ~16 times across 8 crates; no shared home

URL: https://github.com/matiaszanolli/ByroRedux/issues/5596
Labels: low, import-pipeline, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `plugin/src/esm/records/misc/world.rs:906` (`u32_at`), `misc/water.rs:1096` (`read_f32_at`, finite-filtered), `misc/imagespace.rs:81` (`read_f32`), `nif/src/import/mesh/decode.rs:6,12` (`read_f32_le`, `read_u16_le`), `nif/.../mesh/tangent.rs:124`, `nif/src/blocks/collision/havok_packfile.rs:251`, `byroredux/src/cell_loader/placement_lod.rs:109,115`, `scripting/src/obscript.rs:499`, `obscript_runtime.rs:402`, `obscript_vm.rs:186`, `bsa/src/uvd.rs:133,139`, `facegen/src/lib.rs:79,96`, `sfmaterial/src/reader.rs:1241`, `renderer/src/vulkan/dds.rs:1000` (panicking, indexes directly)
- **Status**: NEW
- **Age**: spans 2026-04 to 2026-10.
- **Effort**: medium
- **Description**: the same `data.get(off..off+N)?.try_into()` + `from_le_bytes` shape, returning `Option`, `io::Result` or `Result<_, CustomError>`. `crates/plugin/src/esm/sub_reader.rs` documents that it "replaces inline `read_u32_at`/`read_f32_at`/`from_le_bytes` patterns", but it is a sequential cursor; the random-access cases (per-game WATR offsets, RDAT rows, `.lod`, `.uvd`) re-grew local helpers. `dds.rs:1000` can panic on a truncated header rather than return an error.
- **Evidence**: `grep -rn --include='*.rs' -E 'fn \w+\(\s*\w+: &\[u8\],\s*\w+: usize\s*\)\s*->\s*(Option|io::Result|Result)<(u16|u32|f32)' crates byroredux | grep -v 'tests/\|examples'` -> 16 definitions.
- **Impact**: low per site; the DDS copy is the only one with a wrong failure mode.
- **Related**: `sub_reader.rs` module doc; #4521 (a different slice of the same byte-view class).
- **Suggested Fix**: add `crates/core/src/bytes.rs` with `pub fn le_u16_at / le_u32_at / le_f32_at(data: &[u8], offset: usize) -> Option<u16/u32/f32>` (`checked_add` on the offset), and migrate plugin (4), nif (3), scripting (3), byroredux (2), renderer (1). `bsa`, `facegen`, `sfmaterial` do not depend on core (Cargo.toml check), so leave them or add the dependency with approval. Add an `io_err(Option) -> io::Result` adapter only where a caller needs it.
