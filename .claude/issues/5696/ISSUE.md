# LC-2026-10-12-D3-01: Oblivion `ARMO` `BMDT` is read as a `u32` biped mask, so the General Flags byte (Heavy armor, Non-Playable, Hide Rings, Hide Amulets) lands in `biped_flags` bits 16–23 on 661 of 996 vanilla armors

Labels: low,legacy-compat,esm-plugin,inventory,game:oblivion,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5696

Filed from `docs/audits/AUDIT_LEGACY_COMPAT_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

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

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix

