=== ISSUE #5684 ===
PEX-D4-2026-10-12-01: `papyrus-parser.md` and two code docs still describe the pre-#5398 flag sets

Filed from `docs/audits/AUDIT_PAPYRUS_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Papyrus Lexer & Parser
- **Untrusted-Input**: No
- **Location**: `docs/engine/papyrus-parser.md:78` (grammar line lists only `[Native] [Const] [DebugOnly]
  [Hidden]`), `:128` (`Variable` fields omit `is_hidden`), `:148` (`ScriptFlags` lists four flags;
  now seven), `:151` (`GroupFlags` omits `COLLAPSED`); `crates/papyrus/src/parser/script.rs:680`
  (`parse_struct` doc: "no flag annotations beyond what `Variable` carries", but a member may now be
  `Hidden`)
- **Status**: NEW
- **Description**: `ce90b481c` changed `ast.rs` and the parser but did not touch the reference doc.
  The doc's own flag table (the only place a reader looks up what the AST can express) now omits
  `CONDITIONAL`, `BETA_ONLY`, `DEFAULT`, `GroupFlags::COLLAPSED` and `Variable.is_hidden`, and the
  grammar summary rejects scripts the parser accepts. `docs/engine/papyrus-parser.md` was last
  touched in `3b7ffa670` (#5021).
- **Impact**: Documentation only.
- **Suggested Fix**: Update the four table rows and the grammar line, note that `Default` and
  `Collapsed` are matched contextually (not lexer keywords), and fix the `parse_struct` doc.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites


=== ISSUE #5685 ===
PEX-D3-2026-10-12-02: `.pex` lowering maps script and variable user flags but not property user flags, so the two frontends still disagree on `PropertyFlags`

Filed from `docs/audits/AUDIT_PAPYRUS_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Decompiler Boolean/Control-Flow/Lower
- **Untrusted-Input**: Yes (it is `.pex` input, but only a silent omission)
- **Location**: `crates/pex/src/decompile/lower.rs:360-397` (`lower_property`: `flags` is only ever
  `AUTO` or `AUTO_READ_ONLY`; `prop.user_flags` is never read), against `crates/papyrus/src/ast.rs:108-117`
  (`PropertyFlags` also has `CONST`, `MANDATORY`, `HIDDEN`, `CONDITIONAL`, all of which the `.psc`
  frontend parses)
- **Status**: NEW (the property half of the gap #5398 closed for scripts and variables)
- **Description**: #5398's stated aim is "both frontends agree". It maps the object's `conditional`,
  `hidden` and `default` bits and a variable's `conditional` bit. `model::Property` carries
  `user_flags` just the same, but `lower_property` drops them. A property declared
  `Int Property P Auto Conditional Hidden Mandatory` lowers from `.psc` with `CONDITIONAL | HIDDEN |
  MANDATORY` and from `.pex` with `AUTO` only.
- **Evidence**: `.psc` probe: `Int Property P Auto Conditional Hidden` parses with those flags set.
  `lower_property` builds `flags` solely from `has_auto_var()` / readable / writable. A grep for
  consumers of `PropertyFlags` / `ScriptFlags::CONDITIONAL` finds none outside `crates/papyrus` and
  `crates/pex`, so nothing misbehaves today.
- **Impact**: None today. It would bite the first consumer that keys on a property's `Conditional` (the
  property half of what `GetVMQuestVariable`-style conditions read) or on `Mandatory`, and it makes
  `.psc`-vs-`.pex` AST equality checks impossible for any script with flagged properties. Also note the
  `Variable.is_hidden` flag is never set from `.pex` (struct definitions are not lowered at all).
- **Related**: #5398, #5443 (the other cross-frontend divergence, `is` lowering to `Cast`)
- **Suggested Fix**: Resolve the `hidden`/`conditional`/`mandatory`/`const` bits through the same
  per-`Pex` mask table proposed for finding 01 and OR them into `lower_property`'s `flags`.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix


=== ISSUE #5686 ===
SAVE-D1-2026-10-12-01: the live-load overlay replaces a freshly imported `RigidBodyData` wholesale, wiping its `serde(skip)` `part_surface_materials`; the "re-derived at import, a snapshot never carries it" invariant does not hold on the overlay path

Filed from `docs/audits/AUDIT_SAVE_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW. It is dormant today, and the severity rises to MEDIUM when the per-surface sound tables land.
- **Dimension**: Snapshot Completeness & the Two Lists
- **Data-Loss Class**: silent-drop (derived render/audio data, not player progress)
- **Location**:
  - `crates/core/src/ecs/components/collision.rs:134-150` (`part_surface_materials`, `#[cfg_attr(feature = "inspect", serde(skip))]`, with the justification comment).
  - `byroredux/src/save_io.rs:146` (`"RigidBodyData"` in `MUTABLE_DELTA_COLUMNS`).
  - `crates/save/src/registry.rs:157-196` (the apply closure ends in `world.insert_batch::<T, _>(remapped)`).
  - `crates/physics/src/sync.rs:1042-1054` (the consumer).
  - `byroredux/src/save_io/serde_default_guard_tests.rs` (the 2026-10-10 "refreshed WITHOUT a bump" note) and `round_trip_tests.rs:657-658` (the round-trip test writes an empty slice).
- **Status**: NEW. Closed #5550 (the leaf-alignment walker) and #5542 do not mention save, and no closed issue covers the overlay.
- **Description**:
  1. The Wave 4 follow-through added `part_surface_materials`, the per-collider-part material sequence the NIF import derives from the collision tree.
  2. It was marked `serde(skip)` on the claim that a snapshot never needs it because "the tree is re-parsed from content on load and re-derived at import" (the #3470 class). The fingerprint was refreshed without a bump on that basis.
  3. That reasoning holds for the `restore_world` path, where the value is the only source. It does not hold for the live-load path.
  4. `RigidBodyData` is an overlaid column, and `apply_deltas` replaces each matched entity's component with the deserialized row. The reload has just spawned the entity with a correct `part_surface_materials` from the re-imported NIF, and the overlay overwrites it with `Box::new([])`.
  5. The overlay runs before any collider exists. The only callers of `register_newcomers_and_refresh_queries` are `scene.rs` `select_and_spawn_player_mode` (boot-only), the transition `ground_character_body_at`, a console command and tests. Neither reload arm calls it, so colliders register in the first scheduler tick after the drain.
  6. `physics_sync_system` then builds the colliders from the overlaid data. `collision_shape_to_parts(.., body_data.surface_material, &body_data.part_surface_materials)` sees an empty slice and gives every part the body-level material.
- **Evidence**:
  - Overlay path: `execute_pending_save_loads` → `apply_deltas(world, &registry, &snapshot, &remap, MUTABLE_DELTA_COLUMNS)` → the apply closure → `insert_batch`.
  - `insert_batch` → `insert_bulk` → `insert`, which overwrites.
  - It is the only `serde(skip)` field on a component in `MUTABLE_DELTA_COLUMNS`. The other skip fields in the tree (`quest_stages.rs`, `story_manager.rs`, `quest_alias.rs`) are resources, which are restored wholesale and are not affected the same way.
  - No test exercises it: `rigid_body_data_survives_save_load_round_trip` writes an empty slice, and nothing asserts that a non-empty value survives or is re-derived across a live load.
  - I did not run a probe. The result follows from the code path above.
- **Impact**:
  - After any in-process save load (interior or exterior), every FormID-matched collider whose NIF authored heterogeneous per-child Havok materials loses its per-part split. Footstep and impact lookups see one material for the whole body.
  - Today the per-surface sound tables are empty ("until IPDS decoding lands"), so nothing audible changes.
  - Once the tables are populated, the first load in a session silently degrades surface selection on compound statics, and a fresh boot behaves differently from a loaded game.
  - Savegame data is not lost.
- **Related**: #5550, #5542, #5574, #3470 (the skip-field precedent), Dim 1's "Replacing columns" and overlay notes.
- **Suggested Fix**:
  - Preferred: make the `RigidBodyData` apply keep the live `part_surface_materials` when the live row exists, for example a `register_*` apply hook that merges non-saved fields. Alternatively, drop `RigidBodyData` from `MUTABLE_DELTA_COLUMNS`, since the static and dynamic bodies it describes re-import identically.
  - Add a live-load test that spawns a body with a non-empty slice, overlays an identical saved row, and asserts the slice is intact.
  - Note: `crates/physics/src/sync.rs:1172` still says "the material keeps the low byte". After #5542 the user-data carries the full `u32`; the comment is stale. This is physics-owned and not filed separately.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix


=== ISSUE #5687 ===
ESM-2026-10-12-D2-01: the `WTHS` `DIFF` value walker recurses with no depth bound, so a hostile `RDIF` blob aborts the process with a stack overflow

- **Severity**: HIGH (untrusted-input reader that can be made to abort; a stack overflow is not a catchable panic)
- **Dimension**: Sub-Record Byte Accounting
- **Record / Sub-record**: `WTHS` / `RDIF` (also `REFL`)
- **Location**: `crates/plugin/src/esm/records/weather_settings.rs` — `skip_diff_value` (recursion on the struct arm and on the `Ref` arm), `collect_diff_indices` (entry point)
- **Status**: NEW. Introduced by #5514 (`f0f8dceef`). #5643 is a different recursion (Story Manager tree build); #5627 is the unrelated `WTHS` value-reader tracker.

## Description
#5514 replaced the flat index read with a recursive value skip that mirrors `crates/sfmaterial/src/reader.rs`. Two arms recurse on file-controlled data:
- A non-user class field ("struct") opens a nested diff-pair stream and recurses into each field's own type.
- A `Ref` field reads a target type id and recurses into it.

The class table is decoded from the same blob (`CLAS` chunks with arbitrary `type_ref`s), so a class whose field 0 names the class itself is legal to the parser. Each nesting level consumes only 2 bytes (field index) or 4 bytes (a `Ref` target), so recursion depth is bounded by payload length, not by any cap. The sibling reader this code claims to mirror carries `MAX_NESTING_DEPTH = 64` (`sfmaterial/src/reader.rs`); the port dropped the guard.

## Evidence
Release build, 1.96 toolchain, the real `parse_bgs_reflection`. Blob: `STRT "C\0"`, one `CLAS` with `flags=0` and one field whose `type_ref` is the class's own name offset (0), and a `DIFF` payload of `type_ref = 0` followed by N repetitions of u16 `0x0000`.
- N=1,000 and N=30,000 on a 2 MiB thread: ok, `diff_field_indices = None`.
- **N=100,000** (200 KB payload) on a 2 MiB thread (the default stack of the rayon walk workers, #3813): `thread has overflowed its stack / fatal runtime error: stack overflow, aborting`.
- N=1,000,000 on an 8 MiB thread (the main thread's stack): the same abort.
- A sub-record payload over 64 KiB needs the `XXXX` extended-size override, which `read_sub_records` supports.

## Impact
Any Starfield plugin (or any plugin carrying a `WTHS` record) can kill the engine at load with an abort, before any cell loads. Vanilla data never trips it (163/163 real `RDIF`s decode), so this is a hostile-mod / corrupted-file hardening gap. Blast radius is one record kind, but the consequence is process death.

## Related
#5514 (introduced), #5364, #5627 (the value-reader follow-up will reuse these helpers and inherit the problem — this should land first), #5643, `MAX_GRUP_NESTING_DEPTH` (same class guarded elsewhere).

## Suggested Fix
Thread a `depth: usize` through `skip_diff_value` / `collect_diff_indices` and return `false` (already treated as "decline, leave `diff_field_indices` unset") past a cap equal to `sfmaterial`'s 64. Pin with a unit test that builds the self-referential class and asserts `diff_field_indices == None` without overflowing; a `Ref` chain test should cover the other recursion edge. (The cap constant lives in `sfmaterial`, which `byroredux-plugin` does not depend on, #5615 — share or duplicate with a comment.)

## Completeness Checks
- [ ] **SIBLING**: Every recursive value skipper in the reflection-container readers (`sfmaterial` reader, `weather_settings.rs` helpers, any `REFL` consumer) carries the same depth cap
- [ ] **TESTS**: A regression test pins both recursion edges (self-referential struct, `Ref` chain) and asserts no overflow


=== ISSUE #5688 ===
SAVE-D2-2026-10-12-01: the v38 reshaping of `DialogueSpokenInfoForms` to a `(speaker, INFO)` pair has no save round-trip test, although it is a saved resource whose shape just changed

Filed from `docs/audits/AUDIT_SAVE_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Format & Schema Discipline
- **Data-Loss Class**: none (test-gap; the shape is very likely correct)
- **Location**:
  - `crates/scripting/src/dialogue.rs:54` (`pub struct DialogueSpokenInfoForms(pub std::collections::HashSet<(u32, u32)>)`).
  - `byroredux/src/save_io.rs:543-548` (registration).
  - Searched with `grep -rln DialogueSpokenInfoForms`. The only test-tree hits are the shape-baseline comments in `serde_default_guard_tests.rs`.
- **Status**: NEW. It sits beside the open #5459, which covers only the missing v32→v33 ledger entry.
- **Description**:
  1. Most saved gameplay resources have a `*_survives_save_load*` test (`QuestStageState`, `StoryEventAliasFill`, `StoryManagerNodeState`, the lock and enable ledgers, `Perks`, and so on).
  2. `DialogueSpokenInfoForms` has none. It was registered at v33 (#5367) and reshaped at v38 (#5300), and no test in the repo serializes or restores it.
  3. The only guarantee is the type-level one: `HashSet<(u32, u32)>` serializes as an array of 2-arrays under `serde_json` and decodes back. That is correct by inspection, and the typed preflight would catch a decode failure before teardown.
  4. What nothing pins is the contract: a Say-Once line spoken by speaker A stays disqualified for A, and remains available to speaker B, after an F5/F9 cycle.
- **Evidence**: the grep above. The v38 commit touched `snapshot.rs` and `serde_default_guard_tests.rs` (bump and baseline) and no round-trip test.
- **Impact**: a future change to the pair key (for example mapping `0` speakers, or switching to a map keyed by tuple, which `serde_json` rejects) would not be caught by the guards. The shape baseline and `FORMAT_MAJOR` pin the shape, not the semantics.
- **Related**: #5300, #5367, #5459.
- **Suggested Fix**: add `dialogue_spoken_info_forms_survive_save_load` to `round_trip_tests.rs`. Save a world with two `(speaker, info)` pairs, restore it, and assert the set and the per-speaker `select_info` result.

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix


=== ISSUE #5689 ===
ESM-2026-10-12-D5-01: docs still describe the pre-#5484 / #5493 / #5497 semantics (`XPRI` as the absorbed-ref list, `FLTR` as a FormID filter, `CNAM` as an intensity)

- **Severity**: LOW (doc rot; one of the claims is the exact one that produced #5484, every baked STAT/SCOL drawn twice)
- **Dimension**: CELL / WRLD Walkers & Placement Data (docs)
- **Record / Sub-record**: `CELL` / `XCRI`, `XPRI`; `PKIN` / `FLTR`; `MSWP` / `CNAM`
- **Status**: NEW. The fix commits touched code only (`777351ec1` has no `docs/` entry; `0b2ffed89`/`136048a95` did not edit the module docs below).

## Locations
- `docs/engine/esm-records.md:101` and `:680`: `pkin.rs` described as reading "the FLTR workshop build-mode filter". It is the CK object-window filter path (a zstring, never a FormID; #5493).
- `docs/engine/esm-records.md:303`: `CellData` documented with `pub absorbed_refs: HashSet<u32>, // XCRI/XPRI absorbed REFRs`. The live field is `absorbed_ref_bakes: Vec<(u32, u32)>` (XCRI (ref, bake-hash) pairs; `XPRI` is validated but not stored).
- `docs/engine/esm-records.md:699-700`: "reads the cell's `XCRI` / `XPRI` sub-records into `precombined_mesh_hashes` + `absorbed_refs`".
- `docs/engine/game-compatibility.md:229`: "`XPRI` (absorbed-REFR form-IDs)".
- `docs/engine/asset-pipeline.md:120`: names the `absorbed_refs` set.
- `crates/plugin/src/esm/records/mswp.rs:21`: module doc still says "`CNAM` = optional 4-byte f32 intensity / colour multiplier" while the field doc at `:55-67` says that reading was wrong and the value is the palette row (`color_remap_index`).

## Description
The code and field docs are right, but the reference docs and one module header still carry the old model. The XPRI claim is the dangerous one: a reader or future decoder author following `esm-records.md` will re-derive the inverted gate. `the_documented_esm_cell_index_lists_every_field` pins the `EsmCellIndex` listing only, not the `CellData` block at `:303`.

## Evidence
`crates/plugin/src/esm/cell/mod.rs` (`pub absorbed_ref_bakes: Vec<(u32, u32)>`), `crates/plugin/src/esm/cell/helpers.rs` (XCRI decoded, XPRI arm `{}`), `crates/plugin/src/esm/records/pkin.rs` (the corrected module doc).

## Impact
Documentation only; misleads whoever next touches the precombine, PKIN or MSWP code.

## Related
#5484, #5493, #5497, #2699.

## Suggested Fix
Update the six sites to the current field names and meanings (`absorbed_ref_bakes`, XCRI pairs, XPRI = disjoint previs list not stored; `FLTR` = CK filter path; `CNAM` = Color Remapping Index) and add the `CellData` block to a doc-pin test the way the `EsmCellIndex` one works.

## Completeness Checks
- [ ] **SIBLING**: grep `docs/engine` and module headers for any other `absorbed_refs` / "FLTR … FormID" / "CNAM … intensity" mention
- [ ] **TESTS**: A doc-pin test covers the `CellData` block like the `EsmCellIndex` one


=== ISSUE #5690 ===
SAVE-D5-2026-10-12-01: `docs/engine/save-load-roundtrip.md` §6 (the authoritative load-apply trace) lists 8 steps and "two reconcilers"; the code has since grown about ten further steps, and every line cite is wrong

Filed from `docs/audits/AUDIT_SAVE_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Live Load-Apply & Frame Boundary
- **Data-Loss Class**: none (doc-rot)
- **Location**: `docs/engine/save-load-roundtrip.md:143-250`. Last edited 2026-09-26 (`2f8538334`).
- **Status**: NEW. The earlier doc-rot issues #3028, #3794 and #4145 are closed and fixed other drift. This is new drift.
- **Description**:
  1. Cites that are stale:
     - `execute_pending_save_loads` is cited at `save_io.rs:1203`; it is at `:1747`.
     - `step_save_loads` is cited at `app_step.rs:642`.
     - `apply_deltas` is cited at `driver.rs:318`; it is at `:380`.
     - `build_form_id_remap` is cited at `driver.rs:226`; it is at `:262`.
     - `capture_player_pose` and `apply_player_pose` are cited at `save_io.rs:447` / `:493`.
  2. The numbered sequence omits steps the code now runs:
     - `PRE_RELOAD_RESOURCES` is described as "the first call", but the list of resources and the `ItemInstancePool` exclusion (#4135) are not stated.
     - `purge_cinematic_retention_state` (#5056, #5379, #5490).
     - `without_parked_state` around the reload.
     - `reset_story_location_cursor` and `discard_pending_story_events` (#5435).
     - `restore_resident` (#4695).
     - `drop_entity_bound_continuations` (#4139).
     - `reseat_ambient_packages_after_restore` (#4815).
     - `unresolved_form_id_pairs` → `park_unresolved_snapshot_rows` (#5054).
     - `reconcile_worn_gear` (#5034) and `reset_player_factions_to_record` (#5058), both after the overlay.
     - The diagnostic `validate_world` before the pose restore.
  3. Step 7 says "two reconcilers now". The code runs four, counting `reconcile_worn_gear` and `reset_player_factions_to_record`.
  4. Step 6 calls the `MUTABLE_DELTA_COLUMNS` list "~20" and gives a partial inventory; the real list is 24.
  5. The skill's strict-sequence paragraph and `save_io.rs` are now the more accurate description.
- **Evidence**: compare the `execute_pending_save_loads` body (`save_io.rs:1747-2074`) with the doc's steps 1-8.
- **Impact**: the reference a new contributor reads first under-describes the order-critical load sequence. Several of the omitted steps are order-sensitive (#5060 is open on exactly that).
- **Related**: #5060, #3028, #3794, #4145.
- **Suggested Fix**: regenerate §6 from the current function, and add the file and line cites by function name instead of line number.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites


=== ISSUE #5691 ===
PAR-D2-2026-10-12-07: #5662's policy ("a non-finite scalar drops its track") is bypassed by finite bounds whose span overflows, so `decode_spline_animation` returns `Ok` with NaN/inf translation or scale samples

- **Severity**: MEDIUM
- **Dimension**: Error Semantics
- **Location**: `crates/hkx/src/animation.rs` — `read_vector_curve` (`values.push(min + quantized * (max - min))`); the finite test covers only the stored `min` and `max`. Consumer: `byroredux/src/asset_provider/animation.rs` `convert_hkx_clip` copies samples into `AnimationClip` keys without a finiteness check.
- **Status**: NEW. Residual of closed #5662, whose title's second half (finite garbage on the same bones "passes unchecked") was left undecided by the fix. Searches of open and closed issues for "HKX non-finite" and "spline overflow" found nothing else.
- **Trigger Input**: a dynamic translation or scale component with finite bounds `min = -3.0e38`, `max = +3.0e38` (both pass `is_finite()` and `min <= max`). `max - min` overflows to `+inf`, so a control point with quantized value 0 evaluates `0 * inf = NaN` and one with value 1 evaluates `inf`.

## Description
- #5662 made non-finite stored scalars drop the track instead of failing the clip. The sampled value is never checked, so the same artifact reaches the clip by arithmetic.
- The original report counted 12 *accepted* vanilla clips with finite garbage up to `|T| = 1.41e22` on the same prop-attachment bones. They are still accepted and nothing downstream sanitises a key value. The only `is_finite` guards in `crates/core/src/animation/` are on phase and time.
- This is a mod-file hazard: vanilla bounds do not overflow.

## Evidence
Probe crate (copy of `crates/hkx/src`, real-data tests cut) with one added test built on the repo's `synthetic_clip` / `dynamic_x_translation` fixtures:
```
let curves = dynamic_x_translation((-3.0e38, 3.0e38));
decode_spline_animation(&synthetic_clip(0, 4, &[[0, 0x10, 0, 0]], &curves))
-> Ok, dropped=[], translation=[NaN, 0.0, 0.0] on both frames
```

## Impact
A hostile or corrupt mod clip puts NaN translation or scale into `AnimationClip`, then `Transform`, then the palette and TLAS instance. It affects only that clip while it plays. Downstream GPU behaviour of a NaN bone was not exercised, so it is not rated higher.

## Related
#5662, #3013, #4396 (same class: finite components overflowing to a bad result), PAR-D1-2026-10-12-02.

## Suggested Fix
After the `Dynamic` control points are pushed, mark the track `non_finite` if any value is non-finite (or compute `max - min` in `f64`). Optionally clamp `|T|` to a sane bound for the 12 vanilla finite-garbage clips. Add a fixture using the `dynamic_x_translation((-3.0e38, 3.0e38))` shape, since the existing `non_finite_*` tests only use NaN and inf stored scalars.

## Completeness Checks
- [ ] **SIBLING**: The rotation/scale paths of the spline decoder and the non-spline HKX channel decoders get the same sampled-value finiteness check
- [ ] **TESTS**: A regression test pins the overflowing-span fixture (`-3.0e38 .. 3.0e38`)


=== ISSUE #5692 ===
TOOL-D1-2026-10-12-01: Two #5147/egui-0.36 helpers were inserted between a doc comment and its function, so `write_screenshot` and `DebugUiState::set_visible` lost their docs

Filed from `docs/audits/AUDIT_TOOLING_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Debug Trust Boundary (and the debug-ui panel host)
- **Exposure**: developers only (rustdoc and reader confusion)
- **Location**: `crates/debug-server/src/system.rs:223-243` (`write_screenshot`, `72a63616b`); `crates/debug-ui/src/lib.rs:229-248` (`set_visible`, `500fcb8ca` "egui: discard dropped frames")
- **Status**: NEW (same class as REN-D6-2026-10-12-01 and NIFAL-D9-2026-10-12-01, different files)
- **Description**: Both commits added a new function directly under an existing `///` block.
  - `system.rs`: the original docs "Save debug screenshots under the working directory's `screenshots/` folder. Only a single relative filename is accepted ..." now sit on top of the #5147 text and attach to `unique_screenshot_name`, which does not confine anything. `write_screenshot`, the function that actually enforces the filename-only rule and the `starts_with(&cwd)` check, has no doc.
  - `lib.rs`: "Show or hide the overlay explicitly (scripted captures)." now documents `discard_last_output`. The public `set_visible` has none.
- **Evidence**: `sed -n 223,230p crates/debug-server/src/system.rs` shows five consecutive `///` lines followed by `fn unique_screenshot_name`; `sed -n 229,247p crates/debug-ui/src/lib.rs` shows the `set_visible` doc followed directly by "Drop a stashed `FullOutput` ..." and `fn discard_last_output`.
- **Impact**: rustdoc for `set_visible` is empty and `discard_last_output` (private) carries a public-API sentence; the security-relevant confinement contract of `write_screenshot` is no longer attached to it. No behaviour change.
- **Related**: #5147, #5165
- **Suggested Fix**: Move each original doc block back above its function and give the helper only its own comment.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites


=== ISSUE #5693 ===
PAR-D1-2026-10-12-04: an over-budget menu texture is refused after the archive fetch, is not remembered by resolved path, and the warning overstates what is refused

- **Severity**: LOW
- **Dimension**: Size Discipline
- **Location**: `crates/menuxml/src/menu.rs` — `MenuRenderer::texture`, the #5658 budget arm
- **Status**: NEW (residual of the #5658 fix).
- **Trigger Input**: a menu naming K spellings (`Menus\x.dds`, `Menus80\x.dds`, `textures\menus\x.dds`, …) of one 8192² archive file after the budget is spent.

## Description
- `assets.texture(path)` runs before the budget check, so every refusal pays a full archive extract of the offending file. The refusal is cached only under the authored spelling (`tex_cache`), and the `decoded` map is keyed only on successful decodes. Each additional spelling of a *refused* file therefore re-extracts it (about 33 MB inflated per 8192² DXT1). The cost is transient and bounded by `MAX_DOCUMENT_TILES` (16,384) once per spelling, with no retained memory. It is a first-render CPU stall.
- The log text says "it and any further new textures are not drawn". The check is per texture (`decoded_bytes + len > budget`), so a later, smaller texture that still fits is decoded and drawn. Which textures draw depends on first-visible order. The tests use equal-size textures and cannot tell.

## Evidence
Code reading, `menu.rs` budget arm, with `self.decoded` populated only after a successful `Rgba8::decode_dds`.

## Impact
First-render hitch on a hostile mod menu, plus a misleading operator message. No memory growth.

## Related
#5658, #5667 (BC paths still allocate before checking the payload).

## Suggested Fix
Record refused resolved paths in a small `HashSet<String>` consulted before the fetch, and reword the warning to "this texture is not drawn; later textures that fit the budget still are". Add a test with unequal-size textures.

## Completeness Checks
- [ ] **SIBLING**: Font atlas / `.tex` atlas paths checked for the same fetch-before-budget order
- [ ] **TESTS**: A regression test pins refused-path memoisation and the mixed-size budget behaviour


