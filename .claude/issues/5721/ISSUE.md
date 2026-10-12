# #5721 — GAME-D1-2026-10-12-01: #5031's "hold out the failed form" chain only excludes the latest failure, so two worn items that both fail to import retry each other every frame

**Labels**: `medium`, `gameplay`, `inventory`, `bug`

**Source**: `docs/audits/AUDIT_GAMEPLAY_2026-10-12.md` (GAME-D1-01) · audited at HEAD 5d31fd637

- **Severity**: MEDIUM
- **Dimension**: Inventory & Equipment Model (mid-life gear / `GearImportLoader`)
- **Location**: `byroredux/src/npc_spawn/loot_appearance.rs` — `reconcile_worn_gear_excluding` (the `failed_form` filter), and `step_imports` (file-level miss and failed NIF import arms)
- **Status**: NEW (an incomplete part of the #5031 fix, `ec41e33e8`)
- **Trigger**: any wearer (NPC after a save-load `reconcile_worn_gear`, a looted NPC, or the player) with two or more equipped forms that have no `NpcEquipmentPart` root and whose meshes both fail, whether the mesh is missing from the loaded archives (a missing DLC archive, an unresolvable loose path) or the NIF fails to import.

## Description
`PendingGearImport` is a one-slot queue per wearer. When an import fails, the loader removes the request and chains `reconcile_worn_gear_excluding(world, wearer, Some(failed))`, which re-queues the first root-less form that is not `failed`. With forms A and B both failing:
1. A fails. The chain excludes A, so `missing = [B]` and B is queued.
2. B fails. The chain excludes only B. A still has no root and is not excluded, so `missing = [A]` and A is queued.
3. A fails again, and the loop repeats, one attempt per frame, for as long as the wearer holds both items.

With three or more failing forms the cycle is longer but equally unbounded. A single failing form is bounded. A failing form beside a succeeding one retries once per success. The two-failure case has no terminus. The unit test `a_failed_import_does_not_stall_the_forms_behind_it` checks one step and a one-form wearer, so it cannot see the cycle.

## Evidence
```rust
// step_imports, file-level miss
let Some(bytes) = textures.extract_mesh(&path) else {
    world.remove::<PendingGearImport>(wearer);
    log::warn!("mid-life gear: missing {path}; item {wearer} wears no mesh for it");
    reconcile_worn_gear_excluding(world, wearer, Some(import.form_id));   // excludes only THIS form
    return;
};
```
`missing` is `equipped_forms` minus rooted forms minus `failed_form`, so the previous failure re-enters it.

## Impact
One `log::warn!` plus a mesh extraction attempt per frame, indefinitely. For the "import failed" arm that is a full `load_nif_bytes_with_skeleton` per frame; when the failed import created partial entities on a wearer with a `CellRoot`, each cycle stamps another hidden entity range onto the cell (released only at cell teardown).

## Related
#5031 (closed), #5460 (open: `reconcile_worn_gear` called twice back-to-back in the load drain), #5266.

## Suggested Fix
Carry the failed set on the wearer (a small `Vec<u32>` on a runtime-only component, cleared when the equipment slots change) rather than a one-form parameter, and have `reconcile_worn_gear_excluding` filter against all of it. Add a two-failing-forms test that drives `step_imports`-equivalent calls three times and asserts the queue empties.

## Completeness Checks
- [ ] **SIBLING**: Both failure arms (file-level miss and failed NIF import) feed the same failed set
- [ ] **TESTS**: A two-failing-forms regression test pins that the queue empties
