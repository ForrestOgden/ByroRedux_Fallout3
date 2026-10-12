# ECS-2026-10-12-D6-01: `footstep_system` plays no sound at all when `PhysicsWorld` is absent, although its comments promise the default sound

- **GitHub**: #5714 (https://github.com/matiaszanolli/ByroRedux/issues/5714)
- **Labels**: low,ecs,audio,bug
- **Report**: docs/audits/AUDIT_ECS_2026-10-12.md

**Severity**: LOW

## Location
`byroredux/src/systems/audio.rs:274-283` and `:306`

## Description
The surface-material lookup builds one `Option<u32>` per trigger, but the no-`PhysicsWorld` arm returns an empty `Vec`. The play loop then does `triggers.iter().zip(surface_materials)`, which stops at the shorter side, so zero one-shots are played. The comment above it says a trigger "with no solid ground in range ... reads `None` and takes the default sound". The sibling `impact_audio_system` documents the missing-`PhysicsWorld` case as a clean no-op.

## Evidence
```rust
let surface_materials: Vec<Option<u32>> =
    if let Some(pw) = world.try_resource::<PhysicsWorld>() { triggers.iter().map(..).collect() }
    else { Vec::new() };
...
for (pos, surface) in triggers.iter().zip(surface_materials) {
```

## Impact
None in the shipped engine, because `boot/world.rs:182` always inserts `PhysicsWorld`. It shows up only in a reduced `World` (a headless fixture, or any future physics-less mode), where footsteps vanish and the stride tests that count `scratch.triggers` cannot see it. It also drains `triggers` and discards them without a diagnostic.

## Suggested Fix
Use `vec![None; triggers.len()]` in the else arm so every trigger takes the default sound, and add a test that runs `footstep_system` with no `PhysicsWorld`. Owner overlap: `/audit-audio`.

**Related**: #5556 (closed), #5348 (open, a different footstep gate)

## Completeness Checks
- [ ] **SIBLING**: Check the other `try_resource::<PhysicsWorld>()` else-arms in `systems/audio.rs` for the same zip-truncation shape
- [ ] **TESTS**: `footstep_system` with no `PhysicsWorld` still plays the default sound

Fix with: `/fix-issue 5714`
