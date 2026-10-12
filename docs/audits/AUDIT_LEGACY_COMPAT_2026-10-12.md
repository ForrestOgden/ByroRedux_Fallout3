**HEAD**: 5d31fd637 · **Baseline**: `docs/audits/AUDIT_LEGACY_COMPAT_2026-10-08.md` (HEAD `00f580e09`) · **Audited**: Dim 3 (cross-game decode and branch shape; one new finding) and the in-range Dim 1/Dim 2 commits that touch placement or the scene graph · **Unchanged since baseline (skimmed)**: Dim 1 (guards run, added-line scan clean), Dim 2 (all in-range commits owned by gameplay, CHARAL, NIFAL, EXAL or physics; `#4128` untouched)

# Legacy Compatibility Audit — 2026-10-12

## Method

Delta-scoped against the 2026-10-08 baseline. 345 commits are in range (`00f580e09..5d31fd637`).

- **Legacy source.** The Gamebryo 2.3 drive (`/media/matias/Respaldo 2TB/...`) is **not mounted**. I used `/mnt/data/src/reference/gamebryo-v26` and `gamebryo-v32` (headers only) and the repo's `docs/legacy/`.
- **External references** (fetched to scratch, read-only): xEdit `wbDefinitions{TES4,FO3,FNV,TES5,FO4}.pas` at dev-4.1.6.
- **Raw-data census.** One Python walker over `Oblivion.esm` ARMO and CLOT `BMDT` sub-records (`/tmp/audit/legacy-compat/scripts/obl_bmdt.py`).
- **Tests run** (1.96 toolchain, repo `target/`):
  - `cargo test -p byroredux-core --lib -- math::coord components::camera`: 45 passed.
  - `cargo test -p byroredux-nif --lib -- strip rotation import::coord`: 67 passed.
  - `cargo test -p byroredux --bin byroredux -- euler frustum`: 11 passed (the REFR Euler guards, re-run because of the glam bump).
- **Dedup sources:** `/tmp/audit/issues.json` (122 open), `gh` closed-issue searches, `/tmp/audit/REPORTED_THIS_SUITE.md` (re-read before finalizing; no overlap).

No source, skill or issue was modified, and no engine was launched. Scratch notes are in `/tmp/audit/legacy-compat/` (`dim_1..3.md`, `scripts/`, `xedit/`).

## Executive Summary

| Severity | NEW | Already tracked / cross-referenced |
|---|---:|---:|
| CRITICAL | 0 | 0 |
| HIGH | 0 | 0 |
| MEDIUM | 0 | 0 |
| LOW | 1 | 0 |

**Headline.** One LOW finding: Oblivion `ARMO` `BMDT` is decoded as a `u32` biped mask, but the wire shape is `u16` slots + `u8` general flags + 1 unused byte. 661 of 996 vanilla Oblivion armors therefore carry polluted `biped_flags` bits 16–23. The sibling `CLOT` parser already reads it correctly. No current consumer is visibly wrong, because every polluted bit sits beside real slot bits. It is a latent wrong decode that the existing unit test hides behind a zeroed fixture.

**Prior findings.** `LC-D3-01` / `LC-D3-02` from 10-08 are CLOSED (`#5395`, `#5393`). The voice-name rule was then corrected again by `#5521` (truncate only when quest + topic > 25). The shipped rule in `dialogue_voice.rs:96-120` matches its doc comment. `#4128` is open, with no code change in range.

**Coordinates are clean.** The glam 0.29 → 0.33 bump and `Camera::view_matrix` (`#5566`) both land correctly, and all three guard families pass.

---

## Dimension 1 — Coordinate + placement fidelity (guards run, no findings)

**Listed-path commits in range** (6):
- `c6b8d4584`: glam 0.33 bump. `look_at_rh` / `perspective_rh` are replaced by `camera::rh::view::look_at_mat4` and `camera::rh::proj::directx::perspective`. The engine Y flip after projection is unchanged (`camera.rs:276-281`).
- `e4433baf8` (`#5566`): `Camera::view_matrix` now calls `look_to_mat4(position, forward.normalize(), up)` instead of `look_at(position, position + forward)`. At worldspace magnitude (Markarth, X ≈ −176 000) the focal-point form quantised `forward` to the f32 ulp of the coordinate and rotated the CPU cull frustum up to ~0.45° against the rendered view. The new test `view_matrix_rotation_is_exact_at_worldspace_magnitude` pins the rotation block at the origin and at the Markarth eye. I read the diff and it is correct: the normalisation matches what `look_at_mat4` did to its derived direction.
- `2848895af` (`#5597`): core gains `yup_to_zup_pos`, the exact inverse of `zup_to_yup_pos` (`coord.rs:114`), with a round-trip test. The navmesh fixtures use it instead of hand-rolled swaps.
- `68790cefc`, `51feb00a4`, `3efd17e5b`: docs and whitespace only.

**Added-line scan of the full diff** for `(x, z, -y)`, `4096.0`, `from_mat3`, `EulerRot`, `zup_to_yup`, `cell_grid_to_world_yup` and `EXTERIOR_CELL_UNITS`:
- New `EXTERIOR_CELL_UNITS` uses are in `env_translate/lod.rs` (`uv_at`, quad origin × the constant) and in the `cell_loader/water.rs` distant-water test. Both use the core constant. The test's `as i32` truncation on the quad SW corner is exact at cell multiples.
- The remaining `4096.0` literals are distance or fixture values: `systems/cinematic.rs:352` (route tether length), test fixtures, and `1.0 / 4096.0` water-scale floors.
- The only `(x, z, -y)`-shaped production text outside `coord.rs` is the `skin.rs:731` conjugation (`C·R·Cᵀ`). It is the documented matrix-basis change, with its translation through the coord source of truth (#1617).

**Single-implementation checks (unchanged):**
- Production `Quat::from_mat3` sites are still exactly two: `crates/nif/src/import/collision/mod.rs:745` and `crates/physics/src/ragdoll.rs:915`.
- `strip::destrip` has three production callers: `tri_shape/ni_tri_shape.rs:628`, `skin.rs:335`, `import/collision/shape.rs:842`.
- `normalize_quat` has only its two coord wrappers as callers.
- `--rotation-mode` still reaches the library unclamped (`boot/cli.rs:453-457`).

**Out-of-path placement commit read: `c1a99c25e` (AttachLight light anchor).** `crates/nif/src/import/attach_light.rs` finds the exact-name `AttachLight` node, composing the root-to-node chain through `compose_transforms`. The root transform is included, switch and LOD nodes follow only their active child, and there is a cycle guard and a depth cap. It then goes through `zup_point_to_yup` plus the scene length scale. The spawn side (`spawn/mesh_instance.rs:1585-1600`) parents the light to `placement_root` and computes the global via `GlobalTransform::compose_translation(ref_pos, ref_rot, ref_scale, local)`, so REFR rotation and scale are composed. The coordinate path is single-source and correct.

## Dimension 2 — Legacy subsystem coverage (skimmed, no findings)

22 commits touch the listed paths. All are owned elsewhere:
- Gameplay: Eat/Sleep, `#5495`, `#5500`, `#5391`.
- CHARAL: `#5412`, `#5508`.
- Water / EXAL: `7cf2bd086`, `46e41fc79`, `cff1f9c42`, `c140cb48e`.
- Physics surface material: `886edcded`, `441d90c2f`, `c173147da`, `993d52f3a`.
- NIFAL light attach: `c1a99c25e`.
- Walker hardening: `cf8730adc`.

**Spot-checks:**
- **Walker revisit guard (`cf8730adc`, `#5485`).** All five scene-graph walkers now take `revisit_guard`, so a block is walked at most once. Gamebryo scene graphs are trees by format (an `NiAVObject` has one parent). The doc cites a 128,377-file vanilla census with zero multi-parent and zero self-referenced children, so no vanilla import changes. Not a fidelity loss.
- **Property sinks, string interning, scene-graph field mapping.** No change in range. The 10-08 spot-checks stand.
- **`#4128`** (`NiControllerManager` / KFM sequence state machine) is open. No commit on the animation paths in range.

## Dimension 3 — Cross-game translation-pattern spot-check

**First-step greps:**
- **Pattern A.** The only literal `bsver` thresholds in `crates/nif/src` outside comments are still the three test asserts at `blocks/base.rs:530,550,551`.
- **`GameKind` branches in `cell_loader/` and `scene/`.** The added lines are one pass-through (`record_index.game` into the climate resolver), the keyed Oblivion child-worldspace decision (`#5505`) and the FO3/FNV-only corpse rule (`#5005`). Each is a single `GameKind`-keyed decision, an acceptable shape. No scattered `if game ==` ladder appeared.

**Decode commits verified against xEdit dev-4.1.6** (the cross-game shape, not record depth, which belongs to `/audit-esm`):
- **`#5504` Oblivion `PKDT`/`PTDT`.** The 4-byte `PKDT` (u16 flags, u8 type, 1 unused) and 12-byte `PTDT` (type, target, count) match `wbDefinitionsTES4.pas` (the `wbPACKPKDTDecider` union, `PTDT` struct). Real-data pins exist.
- **`#5492` `PKDD` Dialogue Type.** `u32` at offset 16, optional from member 3, as in the FO3 and FNV definitions.
- **`#5358` Skyrim `BODT`.** The first `u32` is the biped mask, matching `wbBODTBOD2`. It is gated `is_skyrim_or_later` in both parsers, so the FNV `BMDT` path is untouched (as AUDIT_FNV_2026-10-09 recorded).
- **`#5345` FO76 `LSCR`.** Joins the FO4 wire shape, with a test leg per game.
- **`#5521` voice file name.** The length-test rule and the plugin-local id are as documented.

### LC-2026-10-12-D3-01: Oblivion `ARMO` `BMDT` is read as a `u32` biped mask, so the General Flags byte (Heavy armor, Non-Playable, Hide Rings, Hide Amulets) lands in `biped_flags` bits 16–23 on 661 of 996 vanilla armors
- **Severity**: LOW. A wrong decode of an authored input that is parsed into the wrong sink, with no consumer visibly wrong today (analysis below).
- **Dimension**: 3 — Pattern C (one wire structure decoded two ways within one game: the `ARMO` and `CLOT` parsers disagree on the same `BMDT`).
- **Location**: `crates/plugin/src/esm/records/items.rs:530-537` (`parse_armo` `BMDT` arm and its comment); correct sibling at `:848-850` (`parse_clot`); misleading test fixtures at `:2273-2335`.
- **Status**: NEW. `AUDIT_OBLIVION_2026-08-20` and `-04-25` judged the Oblivion `BMDT` "already correct" on length alone (4 bytes). `AUDIT_FNV_2026-10-09` only confirms `#5358` leaves `BMDT` untouched.
- **Description**: xEdit `wbDefinitionsTES4.pas` defines Oblivion's `ARMO` and `CLOT` `BMDT` as `Biped Flags` `u16`, `General Flags` `u8` and 1 unused byte. The General Flags are 0 Hide Rings, 1 Hide Amulets, 6 Non-Playable, 7 Heavy armor (`ARMO` only). `parse_armo` reads the whole sub-record as one `u32` (`biped_flags = SubReader::new(&sub.data).u32_or_default()`), with a comment calling it "just biped_flags u32". The upper half of that `u32` is therefore the General Flags byte shifted into bits 16–23. `slot_mask` (`& 0xFFFF`) stays right, but `ItemKind::Armor.biped_flags` is wrong. `parse_clot` for the same game, same sub-record, reads `u16` correctly.
- **Evidence**: census of `Oblivion.esm` (`scripts/obl_bmdt.py`): ARMO 996 records, all 4-byte `BMDT`, with the General Flags byte nonzero on 661 (`0x80` on 321, `0xC0` on 55, `0x81` on 56, `0x82` on 53, and others). Pollution per bit: 23 on 524 records, 22 on 114, 16 on 125, 17 on 153. CLOT: 604 records, 83 nonzero, correctly dropped by the `u16` read. The real `SE32CirionsHelmet4` `BMDT` is `03 00 80 00` (Head+Hair, Heavy), so it decodes to `biped_flags = 0x0080_0003`. The unit test `oblivion_armo_data_is_14_bytes_…` uses that EDID but a zeroed `BMDT`, and `oblivion_armo_4byte_bmdt_no_longer_drops_biped_flags` calls bit 2 "Hair" (it is UpperBody in Oblivion). Neither fixture can see the pollution.
- **Impact**: Today none visible, which is why this is LOW:
  - `EquipmentSlots::equip` (`crates/core/src/ecs/components/inventory.rs:226`) walks all set bits, so every heavy piece claims bit 23 and each later heavy piece "displaces" the earlier one. But the displaced piece keeps its real bits (head, upper body, foot, and so on), so `npc_spawn.rs`'s `occupants.contains(...)` retain pass and the player-equip `is_equipped` check both still see it.
  - No Oblivion ARMO has a zero real mask (census: 0), so the zero-mask exemption (`#3408`) is not mis-triggered.
  - `unequip(mask)` has no production caller with an ARMO mask.
  - The latent cost: the Heavy / Non-Playable / Hide Rings / Hide Amulets flags are authored data that no sink receives, and any future per-slot logic (a bit-23 query, a "hide rings under gauntlets" rule, a catalog `EquipTarget::BipedSlots` display) inherits foreign bits.
- **Related**: `#5487` (Oblivion NPC body assembly, which uses Oblivion's slot bits 2/3/4/5), `#3408`, `#5358`.
- **Suggested Fix**: Read Oblivion `BMDT` with the `CLOT` shape — `u16` slots into `biped_flags`, and keep the `u8` General Flags in a separate field if a consumer is wanted. Correct the comment. Change the two fixtures to the real `03 00 80 00` bytes and assert `biped_flags == 0x3`. Leave FO3/FNV (`u32` + `u8` + pad) as is.

---

## Scratch-file reconciliation

`/tmp/audit/legacy-compat/dim_1.md`, `dim_2.md` and `dim_3.md` were checked against this report. `dim_1` and `dim_2` have no findings. `dim_3` has LC-2026-10-12-D3-01 plus the verified-commit notes. Everything appears above and nothing was dropped.

## Suggested Next Step

```
/audit-publish docs/audits/AUDIT_LEGACY_COMPAT_2026-10-12.md
```
Labels:
- **LC-2026-10-12-D3-01:** `low` `bug` `legacy-compat` `esm-plugin` `inventory` `game:oblivion`.
