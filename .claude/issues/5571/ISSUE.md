# NIFAL-D7-2026-10-11-01: `NiPathInterpolator` treats the 0..1 path-parameter key times as animation time and drops `percent_data_ref`, `follow_axis`, `bank_*`, `flags`; rotation/scale stamped identity

Labels: medium,nifal,animation,bug

- **Severity**: MEDIUM
- **Dimension**: Animation
- **Tier Violated**: no-fabrication (identity rotation/scale keys invented) + silent drop of a translatable block's fields
- **Game Affected**: Oblivion, FO3, FNV, Skyrim
- **Location**: `crates/nif/src/anim/transform.rs:105-121` (dispatch + comment) and `:194-224` (`extract_transform_channel_path`); fields parsed at `crates/nif/src/blocks/interpolator.rs:560-600`
- **Status**: NEW (the only related issue, #605, is CLOSED. It only made the block reach the extractor; nothing covers percent data or follow-axis.)
- **Description**: `extract_transform_channel_path` converts the referenced `NiPosData` keys into translation keys verbatim and ignores `percent_data_ref` (`NiFloatData`). In Gamebryo the path keys are a *parametric* curve ("The path keys must have a time range starting at 0.0 and ending at 1.0", `NiPathInterpolator.h`), and the percent data maps animation time to that parameter. The comment at `transform.rs:107-115` says vanilla "uses the simple time-keyed form where path keys map directly to animation time"; measurement says the opposite. Rotation is a hard-coded identity key and scale a hard-coded 1.0 key, so follow-axis alignment and banking are never produced. The identity keys also *override* the node's authored local rotation/scale instead of leaving the bind value, which `static_transform_channel` does by leaving the key list empty for an inactive axis.
- **Evidence** (throwaway survey over the vanilla BSAs; the orchestrator re-read the extractor and confirmed it uses `data.keys` directly with identity rot/scale):
  - Every vanilla `NiPathInterpolator` carries a non-null `percent_data_ref`: Oblivion 82/82, FO3 15/15, FNV 52/52, Skyrim SE 71/71. 205/205 path `NiPosData` groups span exactly t=[0,1].
  - `meshes\effects\fxambwatersalmon02a.nif`: percent t=[0,33.3], so the salmon loop imports as a 1 s path loop, about 33× too fast. Also `megatonfanltrays01.nif` (pct t=[0,3.33]), `dlc1fxambbeamgladespot.nif` (t=[15,22]), Oblivion `destruction.nif` (t=[0,1.667], 121 path keys).
  - Non-trivial percent curves (more than 2 keys): Oblivion 15/82, FNV 3/52, Skyrim 27/71. `follow_axis` 0/1/2 present; `max_bank_angle != 0` on most.
  - None of `follow_axis`/`bank_dir`/`max_bank_angle`/`smoothing`/`flags`/`percent_data_ref` is read outside the parser. `nifal.md` does not list the path interpolator as parked.
- **Impact**: Butterflies, fish, hawks, ambient light-ray fans, magic-effect beams and similar paths play at the wrong speed and without facing along the path, and the node's authored rotation is replaced by identity. Visible content, not a crash.
- **Related**: #605 (closed), `docs/engine/nifal.md` §Animation.
- **Suggested Fix**: Evaluate the percent curve at animation time to get the path parameter, then resample the `NiPosData` spline at that parameter into linear keys at import (as the B-spline path does). Emit follow-axis rotation when the FOLLOW flag (0x20) is set and leave rotation/scale empty otherwise. If deferring, park it explicitly in `nifal.md` with the measured census above and stop emitting the identity keys.

_Source: `docs/audits/AUDIT_NIFAL_2026-10-11.md` finding D7-01 (HEAD c6e48cbc6)._

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **CANONICAL-BOUNDARY**: per-game logic stays at the NIFAL parser→canonical boundary — never pushed into shaders/renderer or re-derived downstream. See `/audit-nifal`.
- [ ] **TESTS**: A regression test pins this specific fix
