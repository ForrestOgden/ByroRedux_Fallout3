# TD7-2026-10-11-04: `weather_settings.rs` re-types the CDB builtin type ids and the IS_USER class flag that `byroredux-sfmaterial` already names

URL: https://github.com/matiaszanolli/ByroRedux/issues/5615
Labels: low, esm-plugin, game:starfield, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md


- **Severity**: LOW
- **Dimension**: 7 (overlaps Dim 2; whoever takes the fix should dedupe)
- **Location**:
  - `crates/plugin/src/esm/records/weather_settings.rs:68-82` (`builtin_type_label`)
  - `crates/plugin/src/esm/records/weather_settings.rs:316-376` (`skip_diff_value`)
  - `crates/plugin/src/esm/records/weather_settings.rs:333` (`flags & 0b100`)
  - canonical: `crates/sfmaterial/src/types.rs:14-60`, `ClassFlags::IS_USER`
- **Status**: NEW
- **Age**: a5484faf6 (#5364, WTHS) and f0f8dceef (#5514), 2026-10-09
- **Effort**: small
- **Description**: `weather_settings.rs` carries the 16 builtin type ids twice as bare negative i32 literals: `-255` Null, `-254` String, `-253` List, `-252` Map, `-251` Ref, `-248..-238` the integer, bool, float and double types. One copy is a label table and the other is the `skip_diff_value` switch. It also hard-codes the `0xFFFF` diff terminator twice (`:302,335`) and the user-class bit `0b100`. `byroredux-sfmaterial` defines `BuiltinType` (`0xFFFFFF01 … 0xFFFFFF12`, i.e. the same values as i32) and `ClassFlags::IS_USER = 1 << 2`. The doc comment on `skip_diff_value` even cites sfmaterial's `skip_value`, so the author knew.
- **Evidence**:
  - `0xFFFFFF01 as i32 == -255`, and `1 << 2 == 0b100`, so the values are identical.
  - `crates/plugin/Cargo.toml` does not depend on `byroredux-sfmaterial`. `sfmaterial` has only `thiserror` as a runtime dependency, so the dependency would be acyclic.
  - Three copies of the table now exist: the sfmaterial enum, the label fn and the skip fn.
- **Impact**: The id table only grows by editing three places. The `_ => false` arm in `skip_diff_value` ("unknown built-in — refuse to guess") is the only defence against a missed id. Parse truncation is silent: `collect_diff_indices` returns `None` and the DIFF decodes as unset.
- **Related**: #5364, #5514 (closed).
- **Suggested Fix**:
  - Either depend on `byroredux-sfmaterial` and use `BuiltinType::from_u32(raw as u32)` plus `ClassFlags::is_user`, or move `BuiltinType` / `ClassFlags` into a tiny shared crate or into `byroredux-core`.
  - At minimum, add `const` names (`BUILTIN_NULL: i32 = -255` …) in `weather_settings.rs`, and make `builtin_type_label` and `skip_diff_value` share one `match`.
