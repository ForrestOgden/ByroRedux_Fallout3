# TD8-2026-10-12-03: Fifteen `pub` items are referenced nowhere in the workspace, one of them a consolidation helper with no caller

- **GitHub**: #5745 (https://github.com/matiaszanolli/ByroRedux/issues/5745)
- **Labels**: low,esm-plugin,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD8-2026-10-12-03`)

**Audit finding** `TD8-2026-10-12-03` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `crates/plugin/src/esm/records/common.rs` (`read_lstring_sub`); `crates/core/src/lighting.rs` (`per_bethesda_unit`); `crates/core/src/character/derived.rs` (`b_from_base`); `crates/core/src/ecs/components/actor_values.rs` (`set_permanent`); `crates/core/src/ecs/resources/skin_slot_pool.rs` (`allocated_count`, `free_list_depth`); `crates/core/src/settings/mod.rs` (`requiring_restart`); `crates/menuxml/src/menu.rs` (`set_override_str`, `clear_override`, `set_screen`); `crates/papyrus/src/parser/mod.rs` (`check_keyword`); `crates/papyrus/src/token.rs` (`can_start_expr`); `crates/spt/src/scene.rs` (`count_by_kind`); `crates/ui/src/profile.rs` (`is_avm2`); `crates/plugin/src/esm/cell/mod.rs` (`OBLIVION_WORLDSPACE`)

## Description
Whole-repo identifier count (tests, examples and docs-in-code included) is 1 for each: the definition itself (re-verified at publish time for `read_lstring_sub`, `per_bethesda_unit`, `b_from_base`, `requiring_restart`, `check_keyword`, `can_start_expr`, `count_by_kind`, `is_avm2`, `OBLIVION_WORLDSPACE`, `allocated_count`, `free_list_depth`, `set_override_str`, `clear_override`, and `set_permanent`). Most are crate-public accessors the skill says to note rather than delete. The exception is `read_lstring_sub`: its doc says "Use this instead of the open-coded pair" (#1045), nothing calls it, and the pair it was written to replace (`find_sub(...).map(read_lstring_or_zstring)`) is still open-coded at 2 sites, i.e. the consolidation was written but never applied. (The `shader_flags.rs`/`bsx.rs`/`scene_flags.rs` flag-bit constants were excluded as vocabulary tables.)

## Evidence
Identifier-count scan in `/tmp/audit/tech-debt/` (`git ls-files '*.rs'`, word count == 1, definitions outside `examples/` and `tests/`). Oldest: `f03a7711e` (2026-04-12, the Papyrus pair); `read_lstring_sub`: `a0e64568a` (2026-05-15, #1045).

## Impact
Dead surface that documents itself as the preferred idiom (`read_lstring_sub`) misleads the next ESM record author; the rest is noise.

## Suggested Fix
Migrate the two open-coded pairs to `read_lstring_sub` (or delete it); delete the Papyrus pair and `per_bethesda_unit`; leave the menuxml/ui/settings accessors with a note unless a consumer is planned.

## Related
#1045. `ActorValues::set_permanent` is also tracked on its own, with its #5412 hazard, as REG-2026-10-12-04.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **GATE**: The relevant gate (`cargo clippy`, hygiene test, `_audit-validate.sh`) is green after the change
