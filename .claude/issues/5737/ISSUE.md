# TD2-2026-10-12-02: `parse_shader_type_data_fo4` re-implements five arms its own comment says are "same as Skyrim"

- **GitHub**: #5737 (https://github.com/matiaszanolli/ByroRedux/issues/5737)
- **Labels**: low,nif-parser,nif,game:fo4,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD2-2026-10-12-02`)

**Audit finding** `TD2-2026-10-12-02` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/nif/src/blocks/shader/lighting.rs` — `parse_shader_type_data_fo4` (vs `parse_shader_type_data`); duplicated arms 6 (hair tint), 7 (parallax occlusion), 11 (multi-layer parallax), 14 (sparkle snow), 16 (eye envmap), ~55 lines

## Description
The FO4 function handles types 1 (env-map scale gate) and 5 (skin-tint alpha) differently, then spells `// All other types same as Skyrim.` and re-types the Skyrim arms, byte-reads included. A field-order fix in either copy (these are the layouts `nif.xml` gates by BSVER) would have to be made twice.

## Evidence
The clone scan reports 11 identical windows between the two functions; both end in `_ => Ok(ShaderTypeData::None)`. (Validated at publish time: the `// All other types same as Skyrim.` comment is still present in `parse_shader_type_data_fo4`.)

## Impact
Parse-correctness code where the cost of a missed twin is a stream-position drift on one game only.

## Suggested Fix
End the FO4 match with `_ => parse_shader_type_data(stream, shader_type)` and delete arms 6/7/11/14/16 from it (the Skyrim function already returns `None` for the rest).

## Related
`/audit-nif` Dim 3; #1552.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **TESTS**: A regression test pins this specific fix
