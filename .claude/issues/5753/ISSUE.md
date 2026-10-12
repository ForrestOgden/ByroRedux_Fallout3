# OBL-2026-10-12-D3-01: Argonian and Khajiit actors spawn tailless — the RACE body-section tail `MODL` is parsed into `body_models` and never consumed, and the beast skeleton that carries the tail bones is never selected

- **GitHub**: #5753 (https://github.com/matiaszanolli/ByroRedux/issues/5753)
- **Labels**: medium,game:oblivion,legacy-compat,ai,import-pipeline,bug
- **Source**: docs/audits/AUDIT_OBLIVION_2026-10-12.md (`OBL-2026-10-12-D3-01`)

**Audit finding** `OBL-2026-10-12-D3-01` — severity MEDIUM — from `docs/audits/AUDIT_OBLIVION_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `byroredux/src/npc_spawn.rs` (`humanoid_body_paths`: four parts, no tail arm; `humanoid_body_path_icon_index`: INDX 4 has no mesh in the arm; `humanoid_skeleton_path`); `byroredux/src/npc_spawn/resumable/runtime.rs` (`skeleton_path: humanoid_skeleton_path(game)`); `crates/plugin/src/esm/records/actor/race.rs` (the `MODL` arm pushes every `MODL`, tail included, onto `body_models`)

## Description
`Oblivion.esm` authors a tail mesh in the RACE body section: Argonian under both `MNAM` and `FNAM` (`Characters\Argonian\Tail.NIF`), Khajiit under `MNAM` only (`Characters\Khajiit\KhajiitTail.NIF`), each followed by an INDX 4 `ICON` (`...\tail.dds`). The archive ships both (`meshes\characters\argonian\tail.nif`, `meshes\characters\khajiit\khajiittail.nif`). #5487 decoded the INDX 4 `ICON` into `RaceRecord::body_part_textures` but deliberately wired only INDX 0..3, and the `MODL` is pushed onto `body_models`, whose only reader is the head fallback. Both tail NIFs skin to `Bip01 Tail01` .. `Bip01 Tail08`; those bones exist in `skeletonbeast.nif` (8 `Tail0*` names) and not in `skeleton.nif` (0). `humanoid_skeleton_path` returns `skeleton.nif` for every Oblivion humanoid, and `skeletonbeast.nif` is referenced nowhere in `byroredux/src` (validated at publish time: only a doc comment mentions the name).

## Evidence
Walker over `Oblivion.esm` (`/tmp/audit/oblivion/tools`): Argonian 68 male + 43 female bases (59 + 33 ACHR placements); Khajiit male 58 bases / 53 placements; Khajiit female 37 bases / 35 placements, no authored tail (not counted). Tail-bearing total: 169 NPC_ bases, 145 placements in the base game alone, before leveled spawns and the DLC. `strings tail.nif | grep Tail` shows `Bip01 Tail01..08`; `strings skeleton.nif | grep -c Tail0` is 0; `skeletonbeast.nif` is 8.

## Impact
Every Argonian and male Khajiit NPC, including the quest NPCs of both races, renders without a tail. The skeleton choice also means beast-race actors are bound to the humanoid rig rather than the beast rig the game authors them against. Same class as #5487 (HIGH: handless humanoids) at smaller scale.

## Suggested Fix
Translate the RACE tail into the body table at the parser boundary (a `tail_model` per gender on `RaceRecord`, skipping an empty `MODL`), spawn it as a fifth body part with the INDX 4 skin, and select `skeletonbeast.nif` for races whose body section authors a tail (or via the Oblivion beast flag, `race_flags & 0x04`, which `is_child_race` currently only has to avoid). Pin it with the same archive-presence test #5487 added.

## Related
#5487 (the baseline D3-01 this extends), #3419 (head/body INDX split).

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files / sibling sites
- [ ] **CANONICAL-BOUNDARY**: Per-game logic stays at the NIFAL parser->`Material` boundary (never pushed into shaders/renderer or re-derived at render time). See `/audit-nifal`.
- [ ] **TESTS**: A regression test pins this specific fix
