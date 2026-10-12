# TD1-2026-10-12-01: `merge_bgsm_arm` is 703 lines and still growing, in a file 10 % under the 2000 line

- **GitHub**: #5735 (https://github.com/matiaszanolli/ByroRedux/issues/5735)
- **Labels**: low,import-pipeline,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD1-2026-10-12-01`)

**Audit finding** `TD1-2026-10-12-01` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `byroredux/src/asset_provider/material/merge.rs` (`merge_bgsm_arm`, also `merge_bgem_arm` ~280 lines and `merge_external_material` ~263 lines); file is ~1794 production LOC

## Description
`fn merge_bgsm_arm(material, provider, pool, path, cdb_pbr_fallback, touched, texture_exists)` walks the whole BGSM template chain and carries ~16 `set_*` "already claimed" sentinels as locals, so every new BGSM field adds a sentinel plus an arm. Its own doc rejects a `MergeSentinels` struct on the argument that the flags are arm-local; that is true, but it is also why the function cannot be split by field group today. Two siblings are over 200 lines: `merge_bgem_arm` (280) and `merge_external_material` (263). The file is the destination for the Starfield `.mat` work, so it will keep growing.

## Evidence
Brace-matched scan (`/tmp/audit/tech-debt/fnscan.py`) lists it as the 13th-longest production function in the workspace; it measured 669 lines on 2026-09-29 and 703 now (+34 from the loose `.mat` resolver `c2f28e06c`, CDB blend/emissive capture `9bb6b7dce`, #5503 `4667b884b`, #5284 `c961653f4`).

Validation note at publish time: a size pin already exists in `merge.rs` (`size_pin_tests::merge_bgsm_arm_stays_within_its_size_pin`, `MERGE_BGSM_ARM_LINE_PIN = 708`, from #5096), so the regrowth is already ratcheted; what is missing is the split itself.

## Impact
Each new BGSM field edit is a >700-line review; the sentinel locals make it easy to miss a branch.

## Suggested Fix
Split by output group (texture slots / scalar + PBR / alpha + flags / emissive) with one small sentinel struct per group, each owned by its group helper; then lower `MERGE_BGSM_ARM_LINE_PIN` to the new measured size.

## Related
#3857 (the same fn's ancestor, closed on "awareness only"), #5096 (the size-pin precedent). Watch-list within 5 % of 2000 LOC: `crates/nif/src/import/types.rs`, `crates/audio/src/lib.rs`, `crates/plugin/src/esm/cell/mod.rs`, `crates/scripting/src/translate/effects.rs`, `crates/scripting/examples/mq101_conformance.rs`, `crates/renderer/src/vulkan/context/init.rs`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **CANONICAL-BOUNDARY**: Per-game logic stays at the NIFAL parser->`Material` boundary (never pushed into shaders/renderer or re-derived at render time). See `/audit-nifal`.
- [ ] **TESTS**: A regression test pins this specific fix
