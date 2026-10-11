# TD2-2026-10-11-15: CLI flag lookup: `cli_args::parse_string_arg` (with the `--flag=value` trap warning) is bypassed by ~27 raw `position(|a| a == "--...")` sites and four `--master` collectors

URL: https://github.com/matiaszanolli/ByroRedux/issues/5598
Labels: low, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/cli_args.rs:58` (helper, 16 callers); raw: `scene.rs` (11, e.g. 434-465, 722, 1468), `boot/mod.rs:199,406,425,449`, `scene/nif_loader.rs:86,103`, `hud.rs:390,439`, `scaleform_hud.rs:211`, `cornell/oracle.rs:237,278`, `boot/cli.rs:454`, `app_step.rs:1222`; `--master` collectors: `hud.rs:403`, `scene.rs:488`, `boot/mod.rs:438`, plus `extensions/install.rs:834 flag_values`
- **Status**: NEW
- **Age**: `hud.rs`/`scaleform_hud.rs` `--esm` prologue copy-pasted 2026-09-19 (`62fc0bf22`).
- **Effort**: small
- **Description**: `parse_string_arg`'s doc records the trap it exists for: `--camera-pos=...` matched nothing and silently fell back, costing the W1 water fixture two derivation runs. The raw sites carry the same silent-miss behaviour for `--esm`, `--cell`, `--grid`, `--wrld`, `--radius`, `--kf`, `--bsa`, `--swf`, `--hud-textures` and the rest. `hud_archive_args` (`hud.rs:384-401`) and `hud_scaleform_args` (`scaleform_hud.rs:206-222`) share a verbatim 14-line `--esm` + `esm_dir` prologue. Multi-value `--master` has three inline collectors with three different shapes plus a fourth generic one in `extensions/install.rs`.
- **Evidence**: `grep -rn --include='*.rs' 'position(|a| a == "--\|position(|arg| arg == "--' byroredux | grep -v test | wc -l` -> 27; `grep -rn --include='*.rs' 'parse_string_arg(' byroredux/src | grep -v test | wc -l` -> 16.
- **Impact**: the silent-drop trap applies to every raw flag; the two HUD probes would diverge on the next `--esm` rule.
- **Related**: `cli_args.rs` module doc (the W1 / P1 incident).
- **Suggested Fix**: add `pub fn parse_all_string_args(args, flag) -> Vec<String>` (lifting `install.rs::flag_values`) and `pub fn esm_discovery_root(args) -> Result<Option<(String, PathBuf)>, String>` to `byroredux/src/cli_args.rs`; route the 27 raw sites through `parse_string_arg` and the four `--master` loops through the new function. `hud_archive_args` / `hud_scaleform_args` call `esm_discovery_root`.
