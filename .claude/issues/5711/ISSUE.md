# PERF-D1-2026-10-12-03: `impact_audio_system` clones its sound table every frame, and impact events are armed with no consumer

- **GitHub**: #5711 (https://github.com/matiaszanolli/ByroRedux/issues/5711)
- **Labels**: low,performance,physics,audio,bug
- **Report**: docs/audits/AUDIT_PERFORMANCE_2026-10-12.md

**Severity**: LOW

## Location
- `byroredux/src/systems/audio.rs` (`impact_audio_system`, the `ImpactConfig` snapshot before the `default.is_none() && surface_sounds.is_empty()` early return)
- `crates/physics/src/sync.rs:1155-1161` (`ActiveEvents::CONTACT_FORCE_EVENTS` for every dynamic collider)
- `crates/physics/src/world/mod.rs:1369-1377` (per-substep channel drain)

## Description
- **(a)** The system clones `config.surface_sounds` (a `BTreeMap<u32, Arc<Sound>>`) and `default_sound` every frame to satisfy a lock-order rule, then checks emptiness *after* the clone. Today the map is empty (IPDS/IPCT decode has not landed), so the clone is free. Once content populates it, the clone allocates O(entries) nodes every frame for a system that returns at once when no impact fired.
- **(b)** `register_newcomers` arms `CONTACT_FORCE_EVENTS` on every dynamic collider with rapier's 0.0 threshold. Every awake resting contact pair then pushes an event each substep, and `run_substep` drains and filters the channel each substep behind a `Mutex`. All of it is discarded, because the consumer returns early while `ImpactConfig` has no default sound and no entries (the shipped state).

## Evidence
`world.try_resource::<ImpactConfig>().map(|config| (config.surface_sounds.clone(), config.default_sound.clone(), …))` precedes the `if default.is_none() && surface_sounds.is_empty() { return; }` check. `ActiveEvents::CONTACT_FORCE_EVENTS` is set whenever `motion_type == Dynamic`.

## Impact
- Today only (b), and it is small: rapier already computes the contact forces, and the channel send is cheap.
- Latent (a) once a table ships.
- Not measured. A `phys.stats` plus `cpu_ms:` A/B with `ImpactConfig` unregistered would size (b).

## Suggested Fix
- (a) Hold the table as an `Arc<BTreeMap<…>>` (clone is a refcount bump), or check a cheap `has_content` flag first.
- (b) Arm `CONTACT_FORCE_EVENTS` only while `ImpactConfig` carries content, or gate the drain on the same flag.

**Related**: #5549, #5554, #5556 (closed, same function), #5348 / #5347 (open, footstep emitter docs)

## Completeness Checks
- [ ] **LOCK_ORDER**: The snapshot exists to avoid holding `ImpactConfig` across the `AudioWorld` write guard; any change keeps one-resource-lock-at-a-time
- [ ] **SIBLING**: `footstep_system` snapshots `FootstepConfig.surface_sounds` the same way (`audio.rs`, Arc-cloned per #5556)

Fix with: `/fix-issue 5711`
