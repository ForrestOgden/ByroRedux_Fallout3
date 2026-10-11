# TD8-2026-10-11-02: 28 `#[allow(clippy::...)]` sites carry no reason comment (27 `too_many_arguments`, 1 `large_enum_variant`)

URL: https://github.com/matiaszanolli/ByroRedux/issues/5622
Labels: low, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 8
- **Location** (attribute line; none has a comment above/beside it):
  - `crates/core/src/settings/mod.rs:91`; `crates/spt/examples/spt_tail.rs:420`; `crates/plugin/src/esm/cell/walkers.rs:562`
  - renderer: `groundcover/construct.rs:601`, `bloom.rs:1025`, `caustic.rs:326,366,669`, `groundcover_bench.rs:787`, `composite.rs:370`, `frame_upscaler.rs:196,1060`
  - sfmaterial examples: `cdb_key_hash_probe.rs:118` (`type_complexity`), `cdb_join_probe.rs:645,712`
  - byroredux: `npc_spawn/resumable/runtime.rs:166,479,964,1122`, `prebaked.rs:73,166`, `cell_loader/spawn.rs:262,580`, `exterior.rs:2024`, `references/mod.rs:239`, `precombined.rs:244`, `terrain_lod.rs:360`, `load.rs:831` (`large_enum_variant`)
- **Status**: NEW (the house style was last policed by #5115, CLOSED, for 2 sites; this is the residue and the sites that came after)
- **Effort**: small
- **Evidence**: script over 129 `allow(clippy::...)` attributes, each tested for a `//` comment on the preceding contiguous comment/attribute lines or within 3 lines. 31 flagged; 3 `manual_clamp` sites in `byroredux/src/systems/weather.rs:412-416` are covered by the block comment above (false positives). `git blame` shows all 28 predate 2026-10-08 (newest: `37db35cca` 2026-10-01 resumable runtime/prebaked, 6 sites). 94 `too_many_arguments` allows in total, 64 with a reason.
- **Impact**: A bare `too_many_arguments` allow hides whether the arg list is a deliberate FFI/Vulkan-shaped signature or a missed parameter-struct.
- **Related**: #5115 (CLOSED)
- **Suggested Fix**: Add a one-line reason to each (or fold args into a params struct, e.g. the 6 `npc_spawn/resumable` sites). Optionally add a source-scan test in `workspace_hygiene_tests.rs` (uses `core::source_scan::production_text`) that fails on a bare `allow(clippy::too_many_arguments)`.
