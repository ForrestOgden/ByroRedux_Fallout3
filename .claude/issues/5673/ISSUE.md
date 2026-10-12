# #5673 — PAR-D5-2026-10-12-03: the CDB translucency capture keys on `BSMaterial::TranslucencySettings`, which never occurs as a top-level component, so `use_sss` and `transmissive_scale` are always `None`

**Labels**: `low`, `bug`, `import-pipeline`, `nifal`, `game:starfield`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D5-2026-10-12-03) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Decode-Consumer Wiring
- **Location**:
  - `crates/sfmaterial/src/index.rs:571-591`: the match arm.
  - `:99-102`: field docs.
  - `crates/sfmaterial/examples/cdb_join_probe.rs:130`.
  - Source of the error: `docs/audits/SF_CDB_PHASE2_SPIKE_2026-08-29.md:183`.
- **Status**: NEW. It predates this delta (224a19372) and was found while auditing the 5c14ebdf8 settings pass.
- **Trigger Input**: every vanilla `BSMaterial::TranslucencySettingsComponent`: 19,768 instances, with fields `Enabled` plus an inline `Settings: BSMaterial::TranslucencySettings { …, TransmissiveScale, …, UseSSS }`.
- **Description**:
  - The arm matches the nested struct's class name as if it were the component, so it never fires.
  - The emissive arm (#5283) already shows the correct shape: match the `…Component` and read the inline `Settings`.
  - The fields are parked downstream on purpose (#5196), so nothing renders differently today. But un-parking them would be inert, the same "wired but cannot fire" shape #5502 was filed for.
- **Evidence**: `cdbprobe` class census: `CLASS 19768 BSMaterial::TranslucencySettingsComponent`, no top-level `BSMaterial::TranslucencySettings`, `translucency captures with data: 0`.
- **Impact**: dead capture, and the docs overstate the capture set. No rendering impact while the field stays parked.
- **Related**: #5196, #3398, #5502, #5525 (the loose `.mat` arm probably misses the nested `Settings` too; unverified).
- **Suggested Fix**:
  - Match `TranslucencySettingsComponent`, read `Enabled` and `Settings.{UseSSS, TransmissiveScale}`, and add a synthetic nested fixture.
  - Correct the spike doc's class table.

## Completeness Checks
- [ ] **SIBLING**: Every other component arm in `capture_instance` is checked for matching a nested struct name instead of its `…Component` class
- [ ] **CANONICAL-BOUNDARY**: The fix stays at the CDB → `ImportedMaterial` merge (Phase-2 resolver) feeding NIFAL `translate_material`; nothing is re-derived per game at render time (see `/audit-nifal`)
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
