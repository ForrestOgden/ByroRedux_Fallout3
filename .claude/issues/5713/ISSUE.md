# ECS-2026-10-12-D3-01: five hand-rolled `Children` walks duplicate the new `collect_subtree`, and one of them was written 15 hours before it

- **GitHub**: #5713 (https://github.com/matiaszanolli/ByroRedux/issues/5713)
- **Labels**: low,ecs,tech-debt,bug
- **Report**: docs/audits/AUDIT_ECS_2026-10-12.md

**Severity**: LOW

## Location
`byroredux/src/cell_loader/unload.rs:65-78` (`player_owned_entities`), `byroredux/src/player_body.rs:441-455` (`descendant_entities`), `byroredux/src/npc_spawn/resumable/runtime.rs:1280-1305` (`subtree_has_skinned_mesh`) and `:1340-1375` (`common_shape_rotation`), `byroredux/src/cell_loader/water.rs:790-808`

## Description
#5513 (`9da80470d`) added `core::ecs::collect_subtree` as "the one shared collector every whole-subtree walk routes through", and moved its existing call sites onto it. Five whole-subtree `Children` walks still carry their own copy of the visited-set-plus-`HierarchyTraversalGuard` loop.
- `player_owned_entities` and `descendant_entities` are exactly `collect_subtree` (the second minus the root).
- `subtree_has_skinned_mesh`, `common_shape_rotation` and the water plane scan are `collect_subtree` followed by a per-entity filter.
- `player_owned_entities` was written in `5a68e983e` (2026-10-10 09:57). Its comment says "Cycle-safe subtree walk, matching `cinematic_retained_entities`", and that function now calls `collect_subtree`.

## Evidence
Each site has the same shape, for example `unload.rs:68-76`:
```rust
let mut stack = vec![root];
while let Some(entity) = stack.pop() {
    if !protected.insert(entity) { continue; }
    if let Some(row) = world.get::<Children>(entity) { stack.extend(row.0.iter().copied()); }
}
```
The ragdoll BFS walks (`ragdoll.rs:715`, `:771`) and `bounds.rs` are different: they interleave per-node work and lock-held queries, so they are not candidates.

## Impact
None today, since every copy has a visited set. The risk is that the next copy skips it, which is the failure class #3700, #4572 and #5513 each closed. The hierarchy-walk discipline lives in one place only if the walks use it.

## Suggested Fix
Route the five sites through `collect_subtree` (the water scan and the two `runtime.rs` walks filter its result). Drop `player_body.rs`'s `HashSet` import if nothing else uses it.

**Related**: #5513 (closed); the baseline's ECS-2026-10-09-D3-01

## Completeness Checks
- [ ] **LOCK_ORDER**: `collect_subtree` takes its `Children` reads one at a time; confirm the converted sites do not now hold a query across the walk
- [ ] **SIBLING**: Re-grep `world.get::<Children>` loops after the change for any remaining whole-subtree copy

Fix with: `/fix-issue 5713`
