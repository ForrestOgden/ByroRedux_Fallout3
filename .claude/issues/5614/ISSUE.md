# TD7-2026-10-11-03: MGEF DATA decode uses bare byte offsets, flag masks, actor-value ids and an archetype id in two functions that already disagree on whether to name the archetype

URL: https://github.com/matiaszanolli/ByroRedux/issues/5614
Labels: low, esm-plugin, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md


- **Severity**: LOW
- **Dimension**: 7
- **Location**: `crates/plugin/src/esm/records/misc/magic.rs:779-935` (`decode_mgef_archetype`, `parse_mgef_for_game`)
- **Status**: NEW
- **Age**: 4c0053322, 2026-09-16 (consumable effects parsing). Only `decode_mgef_archetype` predates it (#4415).
- **Effort**: small
- **Description**:
  - Three copies of the `read = |offset| u32::from_le_bytes(data[offset..offset+4]…)` closure read offsets `0, 20, 56, 60, 64, 68, 80, 84, 88, 128, 132, 136` with no field names.
  - Hand-typed masks: `1|2|4|0x100|0x80000|0x100000` (FO3/NV), `1|2|4|0x400` and `0x200|0x20000|0x100|0x10000000` (Skyrim).
  - Gate sizes: `data.len() == 72` (FO3/NV) and `>= 152` (Skyrim+).
  - Actor-value ids: `matches!(av, 12 | 16)`, `(24..=26).contains(&av)`, `read(64) == 34 && av == 16`.
  - The archetype is *named* in `decode_mgef_archetype` (`34 => MagicArchetype::PeakValueModifier`, `MagicArchetype` enum) but raw `34` in `parse_mgef_for_game:864`.
- **Evidence**:
  - `magic.rs:857` `if data.len() == 72`, then `read(64) == 34` at `:865,869`.
  - `magic.rs:893-908` Skyrim branch: `read(80) == 1`, `read(84) == 0`, `read(128) == 0 && read(132) == 0 && read(136) == 0`.
  - `decode_mgef_archetype` documents the layout only in its doc comment (`@64`, `@68`, `@60`, `@88`).
  - Neither function names 56, 80, 84, 128-136.
- **Impact**: The flag bits and AV ids define what counts as a "plain restoration" consumable. A wrong bit reclassifies a poison, ingestible or disease effect as a heal, or the reverse. Nothing in the code says which xEdit field each offset is, so a reviewer has to re-derive the layout from xEdit.
- **Related**: #4415, #2414 (shared walker).
- **Suggested Fix**:
  - In `magic.rs`, add `mod mgef_data { pub const ARCHETYPE: usize = 64; PRIMARY_AV: usize = 68; … }`.
  - Add a `MgefFlags` bitflags-style struct, one per game, with `NO_DURATION`, `HOSTILE`, `RECOVER`, `DETRIMENTAL`, …
  - Reuse `MagicArchetype::from_u32` and compare variants instead of `34`.
  - Add `FO3NV_DATA_LEN` and `TES5_DATA_LEN`.
  - Do this as part of Dim 2's three-copy `read` closure cleanup.
