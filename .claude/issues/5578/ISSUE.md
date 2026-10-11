# NIFAL-D8-2026-10-11-05: The `ObjectInfo.Parent` decode in `stream_db_file_index` has no default-lane test

Labels: low,nifal,import-pipeline,test-gap,game:starfield,bug

- **Severity**: LOW
- **Dimension**: Shader-flags/Effects
- **Tier Violated**: harness-gap
- **Game Affected**: Starfield
- **Location**: `crates/sfmaterial/src/index.rs:686-692` (the only writer of `parents`); tests at `:1280-1320` hand-set `idx.parents`; real coverage only in `crates/sfmaterial/tests/real_cdb.rs::material_index_resolves_inherited_glass_and_emissive_settings` (`#[ignore]`)
- **Status**: NEW
- **Description**: The inheritance unit tests inject `idx.parents` directly. A rename or shape change of the `Parent` ID field would silently revert to "no inheritance" in CI without game data, since `id_named` returns `None` on any shape mismatch without a log.
- **Impact**: Silent loss of the whole #5502 settings fix on a decoder refactor in the default lane.
- **Related**: #5502, #5320
- **Suggested Fix**: Give the synthetic CDB fixture a non-zero `Parent` on one object and assert `lookup` returns the inherited setting. Consider an info log when an `Objects` entry has no decodable `Parent`.

_Source: `docs/audits/AUDIT_NIFAL_2026-10-11.md` finding D8-05 (HEAD c6e48cbc6)._

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix
