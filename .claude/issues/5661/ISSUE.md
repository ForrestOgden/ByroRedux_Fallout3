# #5661 — PAR-D5-2026-10-12-01: after 5c14ebdf8 walks the full `Parent` chain, `has_opacity == None` means "class default", but `apply_cdb_material` still reads it as "opacity present"; 1,265 referenced Starfield materials are alpha-tested, 426 newly

**Labels**: `medium`, `bug`, `import-pipeline`, `nifal`, `game:starfield`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D5-2026-10-12-01) · audited at HEAD c1a99c25e

- **Severity**: MEDIUM
- **Dimension**: Decode-Consumer Wiring
- **Location**:
  - Merge gate: `byroredux/src/asset_provider/material/merge.rs:306-316`, `threshold > 0.0 && cdb_mat.has_opacity != Some(false)`.
  - Test that pins it: `:2135-2189` (`alpha_test_gates_on_an_explicit_has_opacity_false`, third arm `has_opacity: None`).
  - Producer: `crates/sfmaterial/src/index.rs:378-405` (`collect_settings` / `collect_own_settings`). The threshold and `HasOpacity` come from the same `AlphaSettingsComponent` capture, nearest-wins up the chain.
- **Status**: NEW. It is a side effect of the #5502 partial fix (5c14ebdf8) meeting #5196's `None` rule. Open #5502 covers only the texture/flat-colour half.
- **Trigger Input**: a referenced CDB material whose entire `Parent` chain authors `AlphaTestThreshold > 0` but never `HasOpacity`.
- **Description**:
  - Before 5c14ebdf8, `None` meant "this object does not author it". Now the lookup walks to the parentless top object, so a remaining `None` means no object in the chain authors the field, i.e. the class default.
  - The vanilla diff census points to `false` as that default:
    - `HasOpacity = true` is authored 1,449 times, always over an inherited `None`.
    - `HasOpacity = false` is authored 28 times, and 27 of those override an inherited `true`.
  - The merge therefore alpha-tests, against base-colour alpha, surfaces that author no opacity channel. Starfield's real opacity source, slot 2, is parked.
  - The default is **not sourced**: neither Gibbed.Starfield nor Material-Editor in `/mnt/data/src/reference` names it. Under the no-guessing rule it must be confirmed before a fix lands.
- **Evidence**: `cdbprobe` over the vanilla CDB and the 10,152 NIF-referenced material paths.
  - Current alpha-test state:

    | `alpha_test` | `has_opacity` | Threshold | Materials |
    |---|---|---|---|
    | on | `None` | 0.333333 | 1,227 |
    | on | `None` | other | 38 |
    | on | `Some(true)` | any | 864 |
    | off | any | any | 8,023 |

  - Flips caused by 5c14ebdf8:
    - `alpha_test`: 585 off→on, 0 on→off. 426 of the 585 have `HasOpacity` absent along the whole chain.
    - `bgem_glass`: 213 off→on, the documented #5502 intent.
  - Example chain: `Materials\Architecture\City\Akila\AKTarpLayered02.mat` → `AKTarpLayered01` (`thr = 0.333333`, no `HasOpacity`) → `3LayerStandard` → `#7`.
  - The 5c14ebdf8 message and its `real_cdb.rs` census report only IsGlass 4→217 and emissive 0→619.
- **Impact**: about 12% of referenced Starfield materials take the alpha-test path on a channel that is not their opacity. They lose early-Z, pay RT any-hit cost, and may cut holes where colour alpha is below 0.333; the hole extent was not measured. The canonical flag at the NIFAL boundary is silently wrong.
- **Related**: #5502, #5196, #3398, #5525 (the loose `.mat` producer uses the same rule).
- **Suggested Fix**:
  - Source the `AlphaSettingsComponent.HasOpacity` default from the CK schema or a capture.
  - If it is `false`, gate on `has_opacity == Some(true)` and update the #5196 test's `None` arm.
  - Record the alpha-test delta in the commit, as 5c14ebdf8 should have.

## Completeness Checks
- [ ] **SIBLING**: The loose `.mat` producer (`byroredux/src/asset_provider/material/loose_mat.rs`, #5525) applies the same `HasOpacity` rule
- [ ] **CANONICAL-BOUNDARY**: The fix stays at the CDB → `ImportedMaterial` merge (Phase-2 resolver) feeding NIFAL `translate_material`; nothing is re-derived per game at render time (see `/audit-nifal`)
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
