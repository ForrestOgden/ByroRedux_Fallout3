# CONC-D5-2026-10-12-01: `contact_force_rx`'s comment names a rayon-worker invocation the build cannot produce

- **GitHub**: #5716 (https://github.com/matiaszanolli/ByroRedux/issues/5716)
- **Labels**: low,concurrency,physics,documentation,doc-rot
- **Report**: docs/audits/AUDIT_CONCURRENCY_2026-10-12.md

**Severity**: LOW (comment accuracy)

## Location
`crates/physics/src/world/mod.rs:291-294`

## Description
The field is `std::sync::Mutex<mpsc::Receiver<ContactForceEvent>>`, with this note: "the narrow phase invokes the event handler from rayon workers, so `PhysicsWorld` must stay `Sync`". `cargo tree -p byroredux-physics -e features -i rapier3d` shows rapier 0.36.1 built with `alloc`, `std`, `dim3`, `f32`, `default` and no `parallel`. Rapier's narrow phase therefore does not fan out to rayon in this build, and the handler runs on the calling thread inside `physics_sync_system`.

## Evidence
```
rapier3d v0.36.1
├── rapier3d feature "alloc" / "std" / "default" / "dim3" / "f32"      (no "parallel")
```

## Impact
None on behaviour. The `Mutex` is still required, because `Receiver` is `!Sync` and `PhysicsWorld` is an ECS resource. A reader who trusts the comment concludes the physics step is multi-threaded and may add synchronization that is not needed, or skip a check that would matter if `parallel` is ever enabled.

## Suggested Fix
Reword the note to "`Receiver` is `!Sync` and `PhysicsWorld` is a `Sync` resource; with rapier's `parallel` feature the handler would also be called from rayon workers." Leave the `Mutex`.

**Related**: #5542, #5549, #5554 (the impact-audio chain on the same channel)

## Completeness Checks
- [ ] **SIBLING**: Other physics comments that claim rayon fan-out (`grep -rn rayon crates/physics`)

Fix with: `/fix-issue 5716`
