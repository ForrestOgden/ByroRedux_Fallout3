# #5659 — PAR-D1-2026-10-12-02: the HKX `mask_size` gate demands the unpadded size, but Havok pads it to 4 bytes, so 229 Skyrim SE and 167 LE vanilla clips are rejected (all 117 Falmer clips included)

**Labels**: `high`, `bug`, `import-pipeline`, `animation`, `game:skyrim`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D1-2026-10-12-02) · audited at HEAD c1a99c25e

- **Severity**: HIGH
- **Dimension**: Size Discipline (dimension gate)
- **Location**: `crates/hkx/src/animation.rs:392`, the clause `|| mask_size != transform_count * 4 + float_count`, which returns `"unsupported spline clip dimensions"` at `:394`.
- **Status**: NEW. d4352d477 (#5326, closed) pinned these clips in `KNOWN_VANILLA_REJECTS` as "not yet diagnosed per file". No issue tracks the cause. #5446 is a different problem: its fixtures leave `mask_size` at 0.
- **Trigger Input**:
  - Field: `hkaSplineCompressedAnimation.m_maskAndQuantizationSize` when `float_count % 4 != 0`.
  - Havok stores `align4(4*tc + fc)` there.
  - Example: the Falmer clips have `tc = 85` and `fc = 1`. They store 344; the gate expects 341.
- **Description**:
  - Every SE dimension-gate reject fails this one clause and no other. Each satisfies `mask_size == align4(4*tc + fc)`:

    | `fc` | Padding | Clips |
    |---|---|---|
    | 1 | +3 | 168 |
    | 2 | +2 | 28 |
    | 3 | +1 | 15 |
    | 5 | +3 | 11 |
    | 11 | +1 | 5 |
    | 7 | +1 | 2 |

  - Every *accepted* clip with float tracks has `fc` = 4 (2,402 clips) or 8 (2 clips). Those need no padding, which is why the gate never failed on them.
  - The frames-versus-blocks (#4655), `MAX_FRAMES_PER_BLOCK` (#5006) and `MAX_TRANSFORM_SAMPLES` (#5316) clauses reject 0 vanilla clips.
  - The block decoder already aligns the mask end to 4 bytes (`align_up(mask_end, 4)`). Only the gate is wrong.
  - The padding rule is measured, not sourced. The Havok 2013 header declares the field but ships no `.cpp`.
- **Evidence**: probe `hkxprobe` on the real `Skyrim - Animations.bsa` (`/tmp/audit/parsers/probe-c/census-*.txt`):
  - Default gate: SE gives `clips=6126 rejects=288` (229 dimension + 59 non-finite), all with `pad4ok=true`.
  - Gate relaxed to `>=`: SE gives `rejects=59`, only the non-finite class remaining.
  - LE (32-bit): 167 of 203 rejects are this clause. Relaxed, only the 36 non-finite rejects remain.
  - Byte accounting: in every relaxed block the transform data ends exactly at `float_start`. The gap histogram is `{0: 235}`, the same as the strictly accepted population `{0: 6182}`.
  - The Falmer rig decodes to 85 bones. Relaxed, `falmer\animations\1hmidle.hkx` decodes to 85 tracks × 146 frames with `max(track_to_bone) = 84`.
  - Rejected / total by directory:

    | Directory | Rejected / total |
    |---|---|
    | `falmer\animations` | **117/117** |
    | `traps\bowtrap` | 8/8 |
    | `armillary` | 7/7 |
    | `cascoffinpuzzlebrazier` | 7/7 |
    | `dwetowerentrance01` | 6/6 |
    | `portalgem01`, `scportalbowl`, `plaincrossbow`, `lexiconstand` | 5/5 each |
    | `caveworm` | 4/4 |
    | `catapult` | 2/2 |
    | `character\animations` (`2hw_attack*`, `2hm_runrightattackrt*`, `idlepickaxe_*`) | 9/1301 |
    | `draugr` (`bleedout_*`) | 2/213 |
    | `dlc02\hmdaedra` | 3/43 |

    About 30 further single-asset dungeon objects are also rejected.
- **Impact**:
  - Vanilla clips hard-fail in the only HKX decoder.
  - No live consumer is affected today. The consumers are the MQ101 cart idles, `1hm_walkforward` and three Draugr clips, and none of them is in the reject set.
  - It blocks every planned expansion of HKX use: creature locomotion, animated statics and traps, and the documented ~1,400 Skyrim IDLEs.
  - Per the skill's rubric, vanilla content failing to load is HIGH.
- **Related**: #5326, #5446, #5006, #4655, #5316, PAR-D2-2026-10-12-02, PAR-D4-2026-10-12-02.
- **Suggested Fix**:
  - Gate on `mask_size != align_up(transform_count * 4 + float_count, 4)`. It stays exact, so #5006's file-bytes argument still holds.
  - Add a `PackfileBuilder` fixture with `float_count = 1` and a padded `mask_size`.
  - Lower `KNOWN_VANILLA_REJECTS` to 59 in the same change.

## Completeness Checks
- [ ] **SIBLING**: Every other clause of the `decode_spline_animation` dimension gate is re-checked against the padded layout, on both the 64-bit SE and 32-bit LE pointer layouts
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
