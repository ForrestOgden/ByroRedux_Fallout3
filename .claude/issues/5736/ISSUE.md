# TD2-2026-10-12-01: Seven SPIR-V reflection functions repeat the same parse prologue; two repeat a 25-line named-struct lookup

- **GitHub**: #5736 (https://github.com/matiaszanolli/ByroRedux/issues/5736)
- **Labels**: low,renderer,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD2-2026-10-12-01`)

**Audit finding** `TD2-2026-10-12-01` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/renderer/src/vulkan/reflect.rs` (seven public fns: the `is_multiple_of(4)` + `Loader::new()` + `parse_bytes` prologue; `uniform_block_size_by_name` vs `uniform_block_member_offsets_by_name` named-struct lookup)

## Description
Every public function in `reflect.rs` opens with `if !spirv_bytes.len().is_multiple_of(4) { bail!(...) }` then `Loader::new()` + `binary::parse_bytes(...)`. `uniform_block_size_by_name` and `uniform_block_member_offsets_by_name` additionally share byte-for-byte: the `OpName` scan for the struct id, the `HashMap<u32, Instruction>` type table, and the `OpTypeStruct` check. The newest sibling was a paste of the older one (`a5a0b2598`, #4778, 2026-10-02), which is the divergence recipe: a fix to one error message is not made in the other.

## Evidence
`grep -n 'is_multiple_of(4)\|Loader::new()' crates/renderer/src/vulkan/reflect.rs` returns seven pairs; the clone scan finds 17 identical 12-line windows between the two struct-lookup functions.

## Impact
The module is the only guard that the shipped `.spv` matches the host `#[repr(C)]` structs (`GpuCamera`, `VolumetricsParams`), so its helpers are what the GPU-struct lockstep tests stand on; they should not be pasted.

## Suggested Fix
Add `fn parse_module(bytes) -> Result<Module>` and `fn named_struct<'m>(module, name) -> Result<Option<(u32, &'m Instruction, HashMap<u32, Instruction>)>>` in `reflect.rs` and call them from all seven functions.

## Related
#5593 / #5594 (same family: renderer boilerplate with no builder), #4778.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
