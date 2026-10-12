=== ISSUE #5668 ===
PAR-D2-2026-10-12-04: once an include budget is spent, every further distinct include path still logs its own warning, and the byte-denominated fetch budget admits about 131k fetches of 2-byte fragments

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-2026-10-12-04) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `crates/menuxml/src/parse.rs`:
  - `:675-705`: `warn_once`, keyed `"{cause}:{path}"`.
  - `:788-805`: the budget, bytes and fetched early returns.
  - `:845`: `bytes_fetched += bytes.len()`, with no per-fetch floor.
- **Status**: NEW. The "fetched" cause comes from 6a132a967 (#5399); "budget" and "bytes" predate it.
- **Trigger Input**: a menu XML whose includes exhaust a budget and then name K further distinct prefabs. Variant: after splicing to 262,143 B, K distinct 2-byte fragments.
- **Description**:
  - The "exhausted" notices are per path, not per document.
  - The fetch budget counts bytes, so tiny refused fragments each still cost one archive extract.
  - Both effects are linear in document size: a log flood and a main-thread stall, not an amplification.
- **Evidence**: probe `includes`, real `parse_document`, release:

  | Case | Root | Probes | Fetched | Warn lines |
  |---|---|---|---|---|
  | 100k distinct missing includes | 2.7 MB | 300,000 | 0 B | 100,000 |
  | Splice to the cap, then 200k distinct 2-byte refusals | 5.5 MB | 131,077 | 524,289 B | 200,000 |

- **Impact**: a 5.5 MB mod menu XML emits 200,000 warn lines and up to 131k main-thread archive extracts at `--hud` load. Memory stays bounded.
- **Related**: #5399, #5314, #5007.
- **Suggested Fix**:
  - Warn once per document for the exhausted-budget causes.
  - Charge each fetch a floor, for example `max(len, 1 KiB)`, so the fetch budget also bounds the fetch count.

## Completeness Checks
- [ ] **SIBLING**: Every exhausted-budget cause ("budget", "bytes", "fetched", "tiles") follows the same once-per-document rule
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it


=== ISSUE #5669 ===
PAR-D4-2026-10-12-01: the #5400 header census gates only Oblivion `textures\menus\`, not `menus80`/`menus50` (which the loader also serves) or FO3/FNV `textures\interface\`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D4-2026-10-12-01) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Corpus Gates
- **Location**: `crates/menuxml/tests/vanilla_corpus.rs:204-212`, the `starts_with("textures\\menus\\")` filter in `menu_texture_header_census_decodes`.
- **Status**: NEW. It is a scope gap in #5400's census.
- **Trigger Input**: n/a.
- **Description**:
  - The census covers 1,975 of Oblivion's 5,476 base-game menu-set DDS. It skips `menus80` (1,712) and `menus50` (1,789), both of which `MenuRenderer::texture` resolves.
  - FO3/FNV HUD art lives in `textures\interface\`, which no test decodes. That includes 24-bpp `hud_comp_direction_vertical.dds` and the 32-bpp `target_*` and `shared\line\*` files.
  - Nothing asserts a vanilla count, so the census can pass vacuously.
- **Evidence**: `ddscensus` decodes all of them today: Oblivion (all three sets plus SI) 6,023/6,023, FO3 1,157/1,157, FNV 1,767/1,767.
- **Impact**: no current failure; this is lost regression coverage for the FO3/FNV HUD driver's art.
- **Related**: #5400, #4665.
- **Suggested Fix**:
  - Widen the Oblivion filter to `textures\menus*`.
  - Add the census over `textures\interface\` to `fo3_corpus.rs` and `fnv_corpus.rs`.
  - Assert the vanilla counts.

## Completeness Checks
- [ ] **SIBLING**: The FO3 and FNV corpus files (`crates/menuxml/tests/fo3_corpus.rs`, `fnv_corpus.rs`) gain the same census
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it


=== ISSUE #5670 ===
PAR-D2-2026-10-12-05: a present-but-undecodable texture in the preferred resolution set is silently replaced by another set's copy

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-2026-10-12-05) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `crates/menuxml/src/menu.rs:461-482`. `undecodable` is logged only when *no* candidate decodes.
- **Status**: NEW (5c53ad9c7, #5400).
- **Trigger Input**: with the preferred set `menus`, `textures\menus\x.dds` is present but undecodable while `textures\menus80\x.dds` decodes.
- **Description**:
  - Keeping the search going past an undecodable candidate is correct.
  - But when a lower-priority set then succeeds, the substitution is silent, and the result is cached.
- **Evidence**: probe `fallback` (real `MenuRenderer`, counting logger, 3 frames): probes `["textures\\menus\\hud\\a.dds", "textures\\menus80\\hud\\a.dds"]`, warnings `[]`.
  - The same probe confirms that priority order is preserved, each candidate is probed once, and nothing is retried per frame.
- **Impact**: a degraded result (another set's art) with no log line. Cosmetic.
- **Related**: #5400.
- **Suggested Fix**: when `decoded.is_some() && !undecodable.is_empty()`, warn once per cache key.

## Completeness Checks
- [ ] **SIBLING**: The font-atlas resolution path is checked for the same silent substitution
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it


=== ISSUE #5671 ===
PAR-D4-2026-10-12-02: the HKX census asserts `rejects <= 288` while claiming to catch "drift in either direction", pins a count rather than a set, and its block-consistency clause is inert

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D4-2026-10-12-02) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Corpus Gates
- **Location**: `crates/hkx/src/animation.rs`:
  - `:1326-1342`: `assert!(rejects.len() <= KNOWN_VANILLA_REJECTS)`; the comment at `:1332` says "catches drift in either direction".
  - `:1347-1350`: `div_ceil(MAX_FRAMES_PER_BLOCK) >= max_blocks || max_blocks <= 64`.
- **Status**: NEW (d4352d477, the #5326 fix). Distinct from #5446, which covers the unit fixtures.
- **Trigger Input**: n/a.
- **Description**:
  - A one-sided `<=` catches only growth. Once PAR-D1-2026-10-12-02 lands (288 → 59), a regression re-rejecting up to 229 clips stays green unless the constant is lowered too.
  - A swap of which clips are rejected passes, because only the count is pinned.
  - The block clause holds only through its `|| max_blocks <= 64` arm: measured `1471.div_ceil(256) = 6 < 15`. Its "consistency" half never constrains anything.
- **Evidence**: see the location above. The strict run printed `census: 288 clips rejected by the real decoder` and passed.
- **Impact**: the real-data gate can bless a regression of the decoder finding it was written to track.
- **Related**: #5326, #5446, PAR-D1-2026-10-12-02, PAR-D2-2026-10-12-02.
- **Suggested Fix**:
  - Use `assert_eq!` against the baseline, or better, pin the sorted set of rejected paths split by error variant.
  - Drop or correct the inert block clause.

## Completeness Checks
- [ ] **SIBLING**: Every other real-data census in the parser crates that pins a count is checked for a one-sided or inert assert
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it


=== ISSUE #5672 ===
PAR-D2-2026-10-12-06: the CDB `Objects` table decode and the Parent-chain walk degrade silently (shape mismatch drops a row or an edge; a chain past `MAX_PARENT_DEPTH` or a non-start cycle ends without a warning)

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-2026-10-12-06) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `crates/sfmaterial/src/index.rs`:
  - `:686-692`: `id_named(o, "DBID")` / `id_named(o, "Parent")` returning `None` silently skips the row or the edge.
  - `:743-751`: `id_named`.
  - `:378-387`: `for _ in 0..=MAX_PARENT_DEPTH`, with only `parent != obj` as a cycle check.
- **Status**: NEW. #5578 added the default-lane decode test but not a signal. The #5320 strictness covers `Components` only.
- **Trigger Input**: a non-vanilla CDB whose `ObjectInfo` declares `DBID` or `Parent` in a shape other than `{ Value: u32 }`; a chain deeper than 64; or a cycle that does not pass back through the start.
- **Description**:
  - Contract (c) requires a degraded decode to be `Ok` plus a warning, or an error. These cases are neither.
  - A Parent shape drift would silently disable all #5502 inheritance (IsGlass 217 → 4, alpha-test −585).
  - The walk is bounded, so it terminates, but cycles and truncation are not detected.
- **Evidence**: `cdbprobe` on vanilla:
  - 500,403 rows: DBID misses 0, Parent-shape misses 0, `Parent = 0` on 6 rows.
  - Maximum chain depth 24 (2.7× headroom under the 64 cap), 0 cycles.
  - Vanilla is clean. The finding is the missing signal on non-vanilla input.
- **Impact**: schema drift in a mod or DLC CDB degrades Starfield materials wholesale with only an info-level key count to show for it.
- **Related**: #5320, #5578, #5502, #5465.
- **Suggested Fix**:
  - Count `Objects` rows whose DBID or Parent fails `id_named`, and error or `warn!` once, mirroring `RowInstanceMismatch`.
  - Detect revisits and depth-cap hits in `collect_settings` and warn once per lookup.

## Completeness Checks
- [ ] **SIBLING**: The `Components` join and the `Objects` table use one degradation policy (typed error or a single warning)
- [ ] **CANONICAL-BOUNDARY**: The fix stays at the CDB → `ImportedMaterial` merge (Phase-2 resolver) feeding NIFAL `translate_material`; nothing is re-derived per game at render time (see `/audit-nifal`)
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it


=== ISSUE #5673 ===
PAR-D5-2026-10-12-03: the CDB translucency capture keys on `BSMaterial::TranslucencySettings`, which never occurs as a top-level component, so `use_sss` and `transmissive_scale` are always `None`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D5-2026-10-12-03) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Decode-Consumer Wiring
- **Location**:
  - `crates/sfmaterial/src/index.rs:571-591`: the match arm.
  - `:99-102`: field docs.
  - `crates/sfmaterial/examples/cdb_join_probe.rs:130`.
  - Source of the error: `docs/audits/SF_CDB_PHASE2_SPIKE_2026-08-29.md:183`.
- **Status**: NEW. It predates this delta (224a19372) and was found while auditing the 5c14ebdf8 settings pass.
- **Trigger Input**: every vanilla `BSMaterial::TranslucencySettingsComponent`: 19,768 instances, with fields `Enabled` plus an inline `Settings: BSMaterial::TranslucencySettings { …, TransmissiveScale, …, UseSSS }`.
- **Description**:
  - The arm matches the nested struct's class name as if it were the component, so it never fires.
  - The emissive arm (#5283) already shows the correct shape: match the `…Component` and read the inline `Settings`.
  - The fields are parked downstream on purpose (#5196), so nothing renders differently today. But un-parking them would be inert, the same "wired but cannot fire" shape #5502 was filed for.
- **Evidence**: `cdbprobe` class census: `CLASS 19768 BSMaterial::TranslucencySettingsComponent`, no top-level `BSMaterial::TranslucencySettings`, `translucency captures with data: 0`.
- **Impact**: dead capture, and the docs overstate the capture set. No rendering impact while the field stays parked.
- **Related**: #5196, #3398, #5502, #5525 (the loose `.mat` arm probably misses the nested `Settings` too; unverified).
- **Suggested Fix**:
  - Match `TranslucencySettingsComponent`, read `Enabled` and `Settings.{UseSSS, TransmissiveScale}`, and add a synthetic nested fixture.
  - Correct the spike doc's class table.

## Completeness Checks
- [ ] **SIBLING**: Every other component arm in `capture_instance` is checked for matching a nested struct name instead of its `…Component` class
- [ ] **CANONICAL-BOUNDARY**: The fix stays at the CDB → `ImportedMaterial` merge (Phase-2 resolver) feeding NIFAL `translate_material`; nothing is re-derived per game at render time (see `/audit-nifal`)
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it


=== ISSUE #5674 ===
p5-soak on the default game (skyrim_se) runs an unmeasured route and fails at cycle 2 — skyrim_se.env declares no FIXTURE_GATES

Found during the #5155 soak rerun (2026-10-12).

`p5-soak.sh` with no positional defaults to `skyrim_se` (`fixture.sh`'s default game), but `docs/smoke-tests/fixtures/skyrim_se.env` declares **no `FIXTURE_GATES`** — the gate-support check is skipped when the variable is unset — and its `P0_*` fields come from the p0-door route, never measured for the 30-min F5→door→F9 soak. Result on this machine (RTX 4070 Ti, HEAD past 274ba13c5):

```
smoke[p5-soak]: 30-minute transition/save soak (Skyrim Special Edition)
smoke[p5-soak]: cycle 1 @ 25s — rss=3481MiB quicksaves=1 transitions=1
smoke[p5-soak]: FAIL -- cycle 2: door prompt missing (world state drifted?)
artifacts: /tmp/byro-p5-soak.nzYWlg
```

Cycle 1 completes (F5, E-door, exterior arrival, F9 restore, back-walk all pass); cycle 2 fails the door-prompt check (`grep -Fq "$P0_PROMPT"` over `interaction.status`). The player status samples show the body grounded and responsive, so this is the route/fixture mismatch, not necessarily an engine regression — but nothing distinguishes that for the next runner.

Suggested fix (test hygiene, LOW): either declare `FIXTURE_GATES` in `skyrim_se.env` without `p5-soak` so the default invocation fails fast with the existing "no measured route for this gate" message, or measure the Skyrim soak route and add it. Note a confounder for any rerun: a concurrent session was running renderer GPU tests on the same device during this leg.

=== ISSUE #5675 ===
AUD-2026-10-12-D5-01: Clutter-impact sounds play at the midpoint of the two parent *body origins*, not the contact. Every exterior terrain tile's body origin is the world origin

Filed from `docs/audits/AUDIT_AUDIO_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: MEDIUM (dormant today: `ImpactConfig` ships with no `default_sound` and an empty map, so `impact_audio_system` returns before it computes a position. It is wrong the moment IPDS/IPCT content lands.)
- **Dimension**: Engine Consumers
- **Location**:
  - `crates/physics/src/world/mod.rs:706-716` (`contact_event_position`)
  - `byroredux/src/systems/audio.rs:405` (the consumer)
  - `byroredux/src/cell_loader/terrain.rs:1276-1285` (terrain ghost placed at `Vec3::ZERO`)
  - `byroredux/src/cell_loader/spawn.rs:398-418` (static trimesh ghosts placed at the REFR origin)
- **Status**: NEW (introduced by `1291f370a`)
- **Description**: The impact position is `(body_a.translation + body_b.translation) / 2`. That is the midpoint of the two *rigid-body origins*, not of the contact manifold.
  - For a dynamic clutter body hitting a static one, the second origin is wherever the static's body sits.
  - Static architecture ghosts are placed at the placement origin (`spawn_trimesh_collider_ghost(world, positions, indices, pos, rot, scale, …)`), which for a large kit piece can be many metres from the point of contact.
  - Exterior terrain is worse. The LAND tile's vertices are already world-space and the ghost is spawned with `Vec3::ZERO`, so its body translation is exactly the world origin.
  - The `ContactForceEvent` carries no contact point. The point has to come from the narrow phase (`contact_pair(c1, c2)` manifold points) or from the dynamic body's own position.
- **Evidence**:
  ```rust
  // physics/src/world/mod.rs:708-715
  let pa = self.bodies.get(a)?.translation();
  let pb = self.bodies.get(b)?.translation();
  Some(Vec3::new((pa.x + pb.x) * 0.5, (pa.y + pb.y) * 0.5, (pa.z + pb.z) * 0.5))
  // cell_loader/terrain.rs:1276-1284
  spawn_trimesh_collider_ghost(world, &positions, &indices, Vec3::ZERO, Quat::IDENTITY, 1.0, None)
  ```
  The only test is `contact_force_started_event_carries_surface_materials` (`physics/src/world/mod.rs:4102`), which asserts `contact_event_position(&event).is_some()`. Nothing pins the value, and `systems/audio.rs` tests cover selection only (`impact_selects_the_surface_sound_when_authored`, `an_unmapped_material_skips_only_its_own_impact`, `impact_material_resolution_keeps_both_zero_pairs`), never the position handed to `play_oneshot`.
- **Impact**: Once impact content is authored, a clutter item landing on exterior terrain at (x, z) plays at (x/2, z/2). At 300 m from the origin that is 150 m from the item, past the 24 m attenuation range, so exterior terrain impacts are inaudible. Interior statics shift the sound by half the kit piece's origin offset. No effect on the current build.
- **Related**: #5562 (the same "body origin where a contact point belongs" family on the water side), PERF-D1-2026-10-12-03 (same system, perf aspect).
- **Suggested Fix**: Resolve the position from the narrow-phase contact pair (deepest manifold point, converted to world space), falling back to the dynamic collider's body translation when the manifold is empty. Add a test that drops a body onto an off-origin static and asserts the resolved point is within the contact's extent.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix


=== ISSUE #5676 ===
AUD-2026-10-12-D5-02: `--sounds-bsa` is documented as "first hit wins, list overrides first" in three places; the provider has been last-listed-wins since #3917

Filed from `docs/audits/AUDIT_AUDIO_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Engine Consumers (archive precedence)
- **Location**:
  - `docs/engine/game-loop.md:56` (the user-facing CLI table)
  - `byroredux/src/asset_provider/audio.rs:83`
  - `byroredux/src/asset_provider/texture.rs:297` (and `try_load_default_water_splash`'s "first archive/candidate hit wins")
- **Status**: NEW
- **Description**: The #3776 wording ("override/mod archives listed before the vanilla one, first hit wins") predates #3917, which flipped `SoundArchiveProvider::extract` to `extract_first(self.archives.iter().rev(), …)`. The same file's `extract` doc, `assets/debug_profiles.toml` (which lists `Update.bsa` *after* `Fallout - Sound.bsa` for exactly this reason) and the CLI-profile comments all say last-wins. The three drifted sites were not updated.
- **Evidence**:
  ```text
  game-loop.md:56   Repeatable — override/mod archives listed before the vanilla one, first hit wins (#3776)
  audio.rs:141      extract_first(self.archives.iter().rev(), archive_path)
  ```
- **Impact**: A user following the CLI documentation lists their override first; the vanilla archive then shadows it, and the footstep, splash, REGN and voice lookups serve unmodded audio with no log line. Workaround: list overrides last.
- **Related**: #3776 (closed), #3917 (closed). `script.rs:935` documents `ScriptProvider`'s deliberate first-wins correctly.
- **Suggested Fix**: Change all three wordings to "later `--sounds-bsa` archives override earlier ones (last-listed wins; `ScriptProvider` is the first-wins exception)".

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites


