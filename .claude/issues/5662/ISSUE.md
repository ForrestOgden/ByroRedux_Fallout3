# #5662 — PAR-D2-2026-10-12-02: one prop-attachment bone with NaN translation/scale bounds rejects the whole HKX clip (59 SE, 36 LE vanilla clips); finite garbage on the same bones passes unchecked

**Labels**: `medium`, `bug`, `import-pipeline`, `animation`, `game:skyrim`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-2026-10-12-02) · audited at HEAD c1a99c25e

- **Severity**: MEDIUM
- **Dimension**: Error Semantics
- **Location**:
  - `crates/hkx/src/animation.rs:759-763`: `read_vector_curve` reads the dynamic min/max through `read_f32`, which hard-fails on a non-finite value at `:946-951`. The `?` propagates to `decode_spline_animation`.
  - `animation.rs:783`: finite magnitudes are not validated.
  - Consumer: `byroredux/src/asset_provider/animation.rs:403-487` (`convert_hkx_clip`).
- **Status**: NEW. This is the remaining 59 of the 288 baseline that #5326 pinned.
- **Trigger Input**:
  - A translation or scale track with mask `0x70` (quantisation `0x45`) whose per-component min and max are both NaN. Observed bit patterns: `ffffffff`/`7fffffff` 198 each, `ffc00000`/`7fc00000` 60 each, and smaller counts of others.
  - Always on one bone per rig: track 41 `AnimObjectR` on the 99-bone human rig, track 40 `AnimObjectL` on the dragon-mount clips, or that bone's slot in paired rigs.
- **Description**:
  - The NaNs are genuinely in the files.
  - Rotation and every other track decode; a NaN-tolerant decode succeeds on 59/59.
  - The decoder turns one unused prop-attachment channel into a whole-clip `Err`. That contradicts the crate's own #3018 per-element policy ("a bad annotation costs the annotation, not the clip").
  - The policy is also inconsistent: 12 *accepted* clips carry the same artifact as finite garbage on the same bones (`|T|` up to 1.41e22), which flows unchecked into `AnimationClip` keys.
  - That vanilla Skyrim ignores these channels is inferred, not sourced: `AnimObject*` matters only when an IDLE attaches a prop.
- **Evidence**:
  - `nanprobe`: `TOLERANT-OK` 59/59.
  - Bone distribution: 156 hits on `tc=99` track 41, 6 on `tc=97/98` track 40, plus the paired-rig slots.
  - Affected clips include `dialogueangryhips.hkx`, `ritualspell_aimrelease.hkx`, `_1stperson\…\dlc01\crossbow_{idledrawn,release,blockbash}.hkx`, `beasth2h_*attack*`, `paired_1hmkillmove*`, `sharedkillmoves\human&{dragon,werewolf,giant,…}\paired_*` and `dlc2_paireddragonmount*`.
- **Impact**:
  - About 1% of vanilla clips cannot decode, including most killmoves. No live consumer is affected today.
  - Simply accepting NaN would be worse: it would put a NaN bone in the palette.
- **Related**: #3018, #3013, #3765, #4396, PAR-D1-2026-10-12-02.
- **Suggested Fix**:
  - On non-finite bounds for a dynamic T or S channel, keep the clip, drop just that track from `tracks` and `track_to_bone`, and `log::warn!` once naming the track. This mirrors `convert_hkx_clip`'s #3013 arm, so the bone keeps its bind pose and no value is invented.
  - Decide in the same change whether `|T| > 1e6` on these bones gets the same treatment.

## Completeness Checks
- [ ] **SIBLING**: Rotation and float-track decode paths get the same per-channel policy, and the finite-garbage case on the same bones is decided in the same change
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
