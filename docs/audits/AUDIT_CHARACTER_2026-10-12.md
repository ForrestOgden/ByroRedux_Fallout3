**HEAD**: 5d31fd637 · **Baseline**: [`AUDIT_CHARACTER_2026-10-08.md`](AUDIT_CHARACTER_2026-10-08.md) (HEAD `00f580e09`) · **Audited**: Dim 1 (Ruleset Seam), Dim 4 (Population Boundary), Dim 5 (Coverage & Doctrine) · **Unchanged since baseline (skimmed)**: Dim 2 (Derived Formulas), Dim 3 (Progression & Pools)

# Character / CHARAL Audit — 2026-10-12

This is `/audit-character` with the default scope (`--depth deep`), run solo as part of `/audit-suite --preset comprehensive`. No sub-agents were used and no engine was launched. The only extra process was a read-only ESM subrecord dump of the FNV and FO3 Player `NPC_`/`CLAS` records (`crates/plugin/examples/dump_record_subs.rs`, see D4 below).

The delta `00f580e09..HEAD` is 345 commits. Of those, the ones that touch CHARAL paths and matter for its numbers or boundary:
- `a614eb273` (#5412): the player `SetBase` redirect gets its own `set_override` layer.
- `212a71dba` (#5413): `RaceSpells` is stamped on the player by `attach_to_player`.
- `627149b62` (#5508): `ActorValue::undamaged()` is the one ceiling composition.
- `99c22a8db` (#5498): FO4 per-flag `TPTA` template actors in `walk_inherited_records`.
- `4957536ba` (#5338): non-auto-calc FO3/FNV NPCs seed their authored `DATA` SPECIAL.
- `5ed49ed5c` (#5340): `NpcHealthCurve.level_anchor` split out of `offset`.
- `274ba13c5` (#5342): the player Health row restored in the FO3/FNV capture.

The rest of the delta in `npc_spawn*`, `cell_loader/references/`, `systems/character.rs` and `inventory.rs` is appearance, gear, AI-package, water and physics work. None of it writes an actor value or moves a CHARAL constant.

## Tests recorded (read-only)

| Command | Result |
|---|---|
| `cargo test -p byroredux-core --features inspect -- character combat stealth actor_values` | 174 passed, 0 failed (baseline 166; the new pins are #5340's, #5412's and #5508's) |
| `cargo test -p byroredux-plugin --lib -- actor_value_derive equip consumables::tests::fn586 effective_actor_level pc_level` | 108 passed, 3 ignored (real-master pins, not run) |
| `byroredux` bin (rustc 1.96.0), filters `resolve_inherited obscript_dialect player_character_template attach_to_player vitals_snapshot melee_damage player_npc_form_id modav setav player_derived reset_player_factions is_child_race sdk_set_base racespells race_spell` | 15 passed, 0 failed (includes `resolve_inherited_call_sites_are_enumerated_and_pinned`) |
| `dump_record_subs` over `FalloutNV.esm` and `Fallout3.esm` (Player `NPC_` 0x7, `CLAS` 0x57E6A) | Read-only; result under D4 |

## Executive Summary

**4 new findings: 0 CRITICAL · 0 HIGH · 0 MEDIUM · 4 LOW. 0 regressions.**

- **All six 2026-10-08 findings are fixed and verified on HEAD:** #5338, #5340, #5342 (the baseline's open set), plus D4-01 (#5412) and D4-02 (#5413), and the #5508 follow-up that #5412 spawned. Each has a pin that survives a read of the code, not only a closing commit.
- **No constant drifted.** Dim 2/3 source is unchanged except for comments (`git diff 00f580e09..HEAD` on `derived/fallout/tes/skyrim/resistance/leveling/regen/affliction/reputation/combat/stealth` shows comment edits in `fallout.rs` only). This is the seventh run in a row with no drift. Spot re-verification of the FO3/FNV/FO4 player rows, FNV/FO3 shared rows, the NPC Health curve (all 15 infobox samples by hand arithmetic), Oblivion `ModifiedSkill`/weapon/Hand-to-Hand, and Skyrim XP all agree with their captures.
- **#5338 does not disturb the player seed.** The change reads `NPC_ DATA` SPECIAL when ACBS bit 4 is clear, and the FNV/FO3 Player record (0x7) has ACBS flags 0 (bit clear). Its `DATA` is `[100, 5,5,5,5,5,5,5]` and its class (`PlayerClass`, 0x57E6A) has `ATTR` `[5,5,5,5,5,5,5]`, so the authored-vs-class choice is invariant for the player. FO3's Player record is byte-identical on `ACBS`/`DATA`/`CNAM`.
- **New this run, all LOW:**
  - D4-01: `CharacterRuleset::actor_value` hand-sums the modifier layers and omits `set_override`. Unreachable today; it is the exact pattern #5508 removed elsewhere.
  - D5-01: the composition formula in six comments/docs still reads `base + permanent + temporary − damage`.
  - D5-02: the FO3/FNV capture's "NPC stat storage" note was not updated by #5338.
  - D5-03: `charal.md` cites `ruleset.rs:128` for `impl Resource for CharacterRuleset`; it is at `:261`.
- **Overlap check:** none of this suite's reports so far (renderer, performance, ecs, safety, nifal, nif, concurrency) touch CHARAL. No cross-reference needed.

## Constant Verification Table

Re-verified this run against the capture documents (document first, then code). Rows marked "carried" have no code change since a prior verification and were not re-derived.

| Item | Code | Document | Verdict |
|---|---|---|---|
| FO3 Health (player) | `bilinear(END, 20, LEVEL, 10, 0, 90)` = `90 + 20·END + 10·L`, `player_only` | `charal-fnv-fo3-ruleset.md` "Health (player)" row (restored by #5342): `90 + 20·END + 10·Level`; `fAVDHealthEnduranceMult` 20 / `LevelMult` 10 | PASS |
| FNV Health (player) | `bilinear(END, 20, LEVEL, 5, 0, 95)` = `95 + 20·END + 5·L` ≡ `100 + 20·END + 5·(L−1)` | same row: `100 + 20·END + 5·(Level−1)` | PASS |
| FO3/FNV Health (NPC) | `NpcHealthCurve`: `base + 5·(END−1) + 5·(L−1)` (mults 5/5, offset −1, `level_anchor` −1) | same document, "Health (NPCs, #5238)" paragraph; `fAVDNPCHealth{Endurance,Level}Mult` 5/5, exe default offset −1, engine-hardcoded level anchor −1 (#5340) | PASS. 15/15 infobox samples re-computed by hand: FNV Easy Pete 65, Sunny 90, Ringo 120, Boone 230, Benny 205, Veronica 235, Cass 210, Raul 280; FO3 Three Dog 25, Gob 35, Nova 40, Moira 80, Lucas Simms 60, Charon 240, Clover 240 |
| `level_anchor` is not overlaid by any GMST | `with_gmst` copies `self.level_anchor`; `the_level_anchor_is_engine_hardcoded_and_never_overlaid` | capture: "no level-offset GMST exists in either exe" | PASS (the capture's `strings` sweep claim is not re-measured here) |
| FO3 AP / FNV AP | `affine(AGI, 2, 65).capped(85)` / `affine(AGI, 3, 65).capped(95)`, `player_only` | `65 + 2·AGI` cap 85 / `65 + 3·AGI` cap 95; scope unsourced, conservative `player_only` (#2937, documented) | PASS (scope known-open) |
| FO3/FNV Carry Weight, Melee Damage, Crit, Unarmed | `150 + 10·STR`; `0.5·STR`; `1·Luck` cap 10; `0.05·Unarmed + 0.5` ceiled = `ceil((10+U)/20)` | table rows in the same document | PASS |
| FO4 Health | `floor(77.5 + 4.5·END + 2.5·L + 0.5·L·END)`, `player_only` | `charal-fo4-ruleset.md` Health row, LOCKED | PASS |
| FO4 AP / Carry Weight | `60 + 10·AGI` player-only; `200 + 10·STR` actor-general | AP and Carry Weight rows, LOCKED | PASS |
| Oblivion `ModifiedSkill`, weapon multiplier, Hand-to-Hand | `skill + 0.4·(Luck−50)`; `0.5·(0.75+0.005·A)·(0.2+0.015·S)`; `1 + 10.5·(STR/100)·(S/100)`, fatigue `1 + 0.5·H` | `charal-oblivion-ruleset.md` "Complete Damage Formula" | PASS |
| Skyrim XP to next | `xp_base 75`, `xp_mult 25`: 325 @ level 10 | `fXPLevelUpBase`/`Mult` overlay | PASS (carried, spot-checked) |
| FNV/FO3 skill auto-calc | `2 + 2·gov + ceil(0.5·Luck)` | `charal-fnv-fo3-ruleset.md`, geckwiki *SPECIAL* | PASS (carried; tag-skill part absent, known-open) |
| Reputation `STANDING_GRID` | `[infamy][fame]` axes, 4×4 | capture | PASS (carried, spot-checked) |
| Non-auto-calc NPC SPECIAL source | `NPC_ DATA` bytes `4..11` when `acbs_flags & 0x0010 == 0`, class `ATTR` otherwise or when `DATA` is absent | GECK *Stats Tab — NPC*: "Attributes may not be changed if Autocalc is checked" | PASS (sourced); skills for these NPCs still derive from SPECIAL via the GECK formula because the `DNAM` skill half is unparsed (disclosed in `actor_value_derive.rs`; see D5-02) |

No `UNSOURCED` rows were introduced.

## Coverage Matrix

Re-derived from the `profile.rs` arms and workspace greps. Unchanged from the baseline except the cells noted.

| Family | Ruleset impl | Wired | Derived rows | Leveling model | NPC seed | Player seed | Regen wired | Affliction wired |
|---|---|---|---|---|---|---|---|---|
| FO3 | ✓ `fallout3_ruleset` | ✓ `RulesetBuilder::Fallout3` | 8 | ✓ (model only) | ✓ auto-calc from class; non-auto-calc from authored `DATA` SPECIAL (#5338); NPC Health curve (#5238/#5340) | ✓ HP/AP refreshed per frame; `setav` into `set_override` (#5412) | ✗ registered, inert | ✗ unregistered |
| FNV | ✓ `falloutnv_ruleset` | ✓ `RulesetBuilder::FalloutNewVegas` | 8 | ✓ | as FO3 | as FO3 | ✗ | ✗ |
| FO4 | ✓ `fallout4_ruleset` | ✓ `RulesetBuilder::Fallout4` | 3 | ✓ uncapped | ✓ stored `PRPS` + `DNAM`, per-flag `TPTA` template resolution (#5498) | ✓ HP/AP refreshed per frame | ✗ | ✗ |
| Skyrim SE | ✓ `skyrim_ruleset` | ✓ `RulesetBuilder::Skyrim` | 2 | ✓ + skill XP | ✓ race + offset (class/level term deferred, disclosed) | ✓ race + offset; `RaceSpells` now stamped on the player (#5413) | ✗ | ✗ |
| Oblivion | ✓ `oblivion_ruleset` | ✗ `RulesetBuilder::None` (deliberate, pinned) | tests only | ✓ | ✗ | ✗ | ✗ | ✗ |
| FO76 | ✗ | ✗ | ✗ | ✗ | ✗ `NpcStatModel::None` (#4453) | ✗ | ✗ | ✗ |
| Starfield | ✗ | ✗ | ✗ | ✗ | ✗ `NpcStatModel::None` (#4453) | ✗ | ✗ | ✗ |

Wiring greps on HEAD: `PoolRegenConfig` has no production `insert_resource` (all in `regen.rs` tests); `pool_regen_tick_system` is registered and early-returns; `affliction_tick_system` has no scheduler registration; `level_cap()` has no consumer outside the character module; `FactionReputation` has no production insert; `CharacterLevel` is still on the save-refusal list.

## Findings

**LOW (4)**
- D4-01: `CharacterRuleset::actor_value` composes the derived branch without `set_override` (latent).
- D5-01: composition prose in six places omits the `set_override` layer.
- D5-02: the FO3/FNV capture's NPC stat storage note does not record #5338.
- D5-03: stale `ruleset.rs:128` line cite in `charal.md`.

---

### CHAR-2026-10-12-D4-01: `CharacterRuleset::actor_value` re-sums the modifier layers by hand and leaves out `set_override`, the exact pattern #5508 removed from the HUD, vitals and SDK readers
- **Severity**: LOW. Unreachable by construction today: `set_override` is written only by the two #5412 sites (`edit_av` in `byroredux/src/commands/actor_value.rs`, and the SDK `SetBase` arm in `byroredux/src/extensions/commands.rs`), and both gate on `is_player_derived_pool`, which is `PlayerOnly` + `Absolute`. Such a stat takes the `!derives_for_any_actor` early return in `actor_value`, which uses `avs.current()` and does include the override. The hand-sum is only reached for `ActorGeneral` + `Absolute` rows. It would become wrong the first time one of those rows is also writable through the override path (a second row for the same output, a future per-NPC `SetBase` redirect, or a loaded save carrying an override on such a key).
- **Dimension**: Population Boundary (derived-value composition)
- **Game**: fo3, fnv, fo4 (every family that builds a ruleset)
- **Location**: `crates/core/src/character/ruleset.rs:168-170`
- **Status**: NEW (the code path predates #5412; the omission was introduced when #5412 added the layer, and #5508 fixed only the HUD/vitals/SDK readers).
- **Source**: not numeric. The invariant is the repo's own: `ActorValue::undamaged()` is "the one composition of the ceiling" (`actor_values.rs`, #5508, pinned by `undamaged_includes_every_layer_but_damage`).
- **Description**:
  - In the derived branch, `actor_value` returns `derived_value(..) + layers`, where `layers = value.permanent_mod + value.temporary_mod - value.damage`. The base is the formula output, so `base` is correctly left out, but the other two additive layers (`set_override`) are written out by hand rather than derived from the entry (`value.current() - value.base` would compose every layer).
  - The doc comment on the same function repeats the formula: `derived_value + permanent + temporary − damage`.
  - This is the "a reader that re-sums layers by hand is the regression" case from #5508's closing note.
- **Evidence**: `ruleset.rs:156-172`; writes of `set_override` exist only at `commands/actor_value.rs:103` and `extensions/commands.rs:457`, both behind `is_player_derived_pool`. No test builds an `ActorGeneral` + `Absolute` stat carrying `set_override` through `actor_value`.
- **Impact**: none today. If it becomes reachable, `GetActorValue` and `melee_damage_charal_bonus` (the two callers) would silently ignore an override that `current()`, the HUD and the SDK all honour.
- **Related**: #5412, #5508, #5042 (the composer's origin).
- **Suggested Fix**: compute the layer term from the entry (`value.current() - value.base`), update the doc comment, and add a one-line test with `set_override` on an `ActorGeneral` row.

### CHAR-2026-10-12-D5-01: The layer-composition formula in six comments and docs still reads `base + permanent + temporary − damage`, after #5412 added `set_override`
- **Severity**: LOW (doc rot)
- **Dimension**: Coverage & Doctrine
- **Game**: all
- **Location**:
  - `crates/scripting/src/condition.rs:84` and `:718` (`GetActorValue` doc and inline comment)
  - `crates/core/src/character/ruleset.rs:148` (the `actor_value` doc)
  - `docs/engine/charal.md:86` and `:163`
- **Status**: NEW. #5412 updated `actor_values.rs`, `charal.md` §6 and the `inventory.rs` vitals doc; it did not sweep these.
- **Description**: the live composition is `base + set_override + permanent + temporary − damage` (`actor_values.rs`). The sites above state the old four-layer sum as the definition. `charal.md:86` calls it "one composition" and `:163` "layered `base + permanent + temporary − damage`", which a reader will take as exhaustive. Note: `charal-skyrim-ruleset.md:668` carries the same old sum, but inside a dated historical narrative of the Carry Weight base-read gap and is correct for the time it describes; it is not part of this finding.
- **Evidence**: `grep -rn 'base + permanent' crates byroredux docs/engine` on HEAD. `GetActorValue`'s authored-fast-path returns `avs.current()`, which does include the override, so the function behaves as the new formula says while its doc says otherwise.
- **Impact**: documentation only; it is what makes D4-01-style hand-sums look correct.
- **Related**: #5412, #5508, D4-01.
- **Suggested Fix**: point each site at `ActorValue::current`/`undamaged` ("see `actor_values.rs` for the layer list") instead of repeating the sum.

### CHAR-2026-10-12-D5-02: The FO3/FNV capture's "NPC stat storage" note was not updated for #5338
- **Severity**: LOW (doc rot — code ahead of capture)
- **Dimension**: Coverage & Doctrine
- **Game**: fo3, fnv
- **Location**: `docs/engine/charal-fnv-fo3-ruleset.md:473-480` ("NPC stat storage — NOTE (distinct from FO4)")
- **Status**: NEW
- **Description**:
  - The note still says auto-calc-OFF NPCs "store explicit skill/SPECIAL values in their `NPC_` record (DNAM-era layout)" and that the "Full byte layout" is "a follow-up when wiring FO3/FNV NPC derived stats".
  - #5338 has since wired the SPECIAL half: `NPC_ DATA` is `i32 health` + `7 × u8 SPECIAL` (11 or 25 bytes), decoded into `NpcRecord::data_attributes` and used when ACBS bit 4 is clear. The skill half (the `DNAM` block) is still unparsed, and skills for these NPCs derive from the authored SPECIAL through the GECK formula instead of the authored per-skill values.
  - `actor_value_derive.rs`'s module docs state exactly this; the capture, which is meant to be the authority for what is built, does not.
  - `docs/engine/charal.md` §8 item 2 ("FNV/FO3 population — shipped (#1663)") also does not mention the authored-SPECIAL branch.
- **Evidence**: the `DATA` arm in `crates/plugin/src/esm/records/actor/npc.rs`; `derive_autocalc_actor_values` in `actor_value_derive.rs`; `fnv_authored_data_special_reproduces_the_wiki_hit_points` (ignored real-master pin, not run here).
- **Impact**: a reader of the capture sees "unwired follow-up" for something that shipped, and no pointer to the one remaining gap (authored skills).
- **Related**: #5338 (closed). Audit-esm Dim 4 owns the `DNAM` skill-half parse.
- **Suggested Fix**: replace the "follow-up" sentence with the shipped/unparsed split and the ACBS bit-4 discriminator.

### CHAR-2026-10-12-D5-03: `charal.md` cites `ruleset.rs:128` for `impl Resource for CharacterRuleset`; it is at line 261
- **Severity**: LOW (doc rot)
- **Dimension**: Coverage & Doctrine
- **Game**: all
- **Location**: `docs/engine/charal.md:642`
- **Status**: NEW
- **Description**: the skill's convention is to cite symbols, not line ranges. This is the only `*.rs:<line>` cite left in `docs/engine/charal*.md`, and it is wrong (`impl Resource for CharacterRuleset {}` is at `crates/core/src/character/ruleset.rs:261`).
- **Impact**: documentation only.
- **Suggested Fix**: drop the line number: "`impl Resource for CharacterRuleset`, `ruleset.rs`".

---

## Dimension notes

**Dim 1, Ruleset Seam & Doctrine.**
- `GameKind` occurs nowhere under `crates/core/src/character/` or in `actor_value_derive.rs` except in comments that forbid it. Every `GameKind` use in `byroredux/src/npc_spawn*` is appearance translation (skeleton path, head parts, body tables, `is_child_race`), not stats. `player_npc_form_id(GameKind)` maps every variant to the same FormID.
- Single sink: `build_character_ruleset` (`npc_spawn.rs:335`) is the only construction site, reached from `inventory.rs:398` and `cell_loader/references/mod.rs:345`. The other `CharacterRuleset::new` calls are in tests.
- `NpcHealthCurve` gained `level_anchor`; both Fallout profile rows carry 5/5/−1/−1, and the non-`ClassAutoCalc` placeholder returns zeros.
- Profile-as-oracle consumers (`obscript_dialect_for`, `consumables.rs` fn-586 gate): their pins still pass (`obscript_dialect` filter in the bin run).

**Dim 4, Population Boundary.** Verified in code, not only by closing commit:
- #5412: both write sites call `set_override`; `permanent_mod` is untouched; the core test `set_override_survives_the_player_refresh_and_keeps_permanent_mod` and the console test cover re-`setav` plus an ability bonus. `FORMAT_MAJOR` moved 33 → 34 with the shape baseline.
- #5413: `attach_to_player` inserts `RaceSpells` from `PlayerCharacterTemplate::racial_spells`; the attach test drives `remove_race_spells`/`add_race_spells` against the production-built player.
- #5508: `ActorValue::undamaged()` is read by `hud.rs:836` and `inventory.rs:317`; the SDK capture folds `base + set_override` into the SDK `base` (`extensions/capture.rs`).
- `effective_actor_level` has one definition (`actor/mod.rs:149`, `.max(0)` on the non-multiplier branch); the other `.max(1)` in this tree (`cell_loader/references/attach.rs:222`) reads the player's `CharacterLevel` for loot expansion, not a second copy.
- #5498: `template_actors: [u32; 13]` is resolved per flag in `walk_inherited_records` with no game branch. The bit-to-slot mapping (single-bit flag → `trailing_zeros()`) matches the xEdit `wbTemplateFlags` order the field documents.
- #5338's `Use Stats` interaction: the discriminator is read from the stats-resolved record (`resolved.stats`), so a templated shell follows its template's auto-calc bit, consistent with how SPECIAL/level are already inherited.

**Dim 5, Coverage & Doctrine.** Matrix above. The `mod.rs` docstring and `feature-matrix.md` rows (265, 345, 346) are consistent with code. `ROADMAP.md`'s CHARAL bullet no longer presents the 2026-08-15 "62 constants, zero mismatch" as standing (#5342).

## Known-Open Register (re-confirmed on HEAD; none re-filed)

1. FNV/FO3 tag-skill per-level formula: still undocumented and absent. CLAS SPECIAL is still read from `ATTR`, not `DATA`.
2. FO3/FNV AP (and Crit/Melee/Unarmed) NPC scope: AP still `.player_only()` (#2937, closed as documented); Melee Damage still CONFLICTED and deliberately actor-general.
3. VATS runtime (AP pool/regen, time-pause, limb health, hit roll): absent; only the AP formulas exist.
4. Regen, affliction and level-up are inert: `PoolRegenConfig` has no production insertion; `pool_regen_tick_system` is registered and early-returns; `affliction_tick_system` is unregistered; `level_cap()` has no consumer. `FactionReputation` has no production insert; `CharacterLevel` remains on the save-refusal list (#2947 guard).
5. Oblivion `RulesetBuilder::None` is deliberate; FO76/Starfield have captures but no builders.
6. #4137 (OPEN): six `template_flags` bits have no consumer.
7. #4232 (OPEN): `effective_actor_level` returns `npc.level.max(0)` verbatim.
8. #5610 (OPEN, `character` label): `PerkList` is an orphan component. Not re-filed.
9. Closed and verified fixed this run: #5238, #5239, #5338, #5340, #5342, #5412, #5413, #5508, #4415.

## Regression checks

| Issue | Verified |
|---|---|
| #5338 | `npc.rs` `DATA` arm stores `data_attributes`; `derive_autocalc_actor_values` prefers it when ACBS bit 4 is clear and falls back to the class on a `DATA`-less shell. Unit test `authored_data_special_beats_the_class_when_auto_calc_is_off` passes. FNV/FO3 Player record is invariant (see Executive Summary). |
| #5340 | `level_anchor` is a separate field; `with_gmst` never overlays it; `the_level_anchor_is_engine_hardcoded_and_never_overlaid` passes. |
| #5342 | The `Health (player)` row is back in `charal-fnv-fo3-ruleset.md` beside `Health (NPC)`; the curve pin carries all 15 samples. |
| #5412 | See Dim 4 notes. |
| #5413 | See Dim 4 notes. |
| #5508 | See Dim 4 notes; D4-01 is its one remaining gap. |

## Cross-audit routing

Component shape of `ActorValue` → `/audit-ecs`; `NPC_ DATA`/`DNAM` parse (the remaining authored-skill half) → `/audit-esm` Dim 4; `GetActorValue` evaluation → `/audit-scripting`; gameplay writers of `ActorValues` → `/audit-gameplay`; `ActorValue` save shape (FORMAT_MAJOR 34) → `/audit-save`.

Next step: `/audit-publish docs/audits/AUDIT_CHARACTER_2026-10-12.md` (domain label `character`; the D5 findings also take `doc-rot`; game labels `game:fo3`/`game:fnv` only on D5-02).
