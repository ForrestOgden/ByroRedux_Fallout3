# TD3-2026-10-11-04: Three `docs/feature-matrix.md` rows under-state shipped code (UV scrolling, NPC/creature physics, "kira 0.10")

URL: https://github.com/matiaszanolli/ByroRedux/issues/5600
Labels: low, tech-debt, documentation, doc-rot
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 3 — Stale Documentation & Comments
- **Location**:
  - `docs/feature-matrix.md:128` "UV scrolling (animated UV offset) | ✗ | parsed, not rendered" — contradicts `:124` "Embedded ambient controllers (UV scroll, alpha fade, vis flicker) | ✓" in the same table. `AnimatedUvTransform` (`crates/core/src/ecs/components/animated.rs:88`) is written by `animation_system` and consumed at `byroredux/src/render/static_meshes.rs:369-378` (#525).
  - `docs/feature-matrix.md:167` "NPC / creature physics | ✗" — NPCs are driven through Rapier's KCC (`byroredux/src/systems/locomotion.rs:123,240`, M42.10/M42.11) and have ragdolls (row below it, `:169`).
  - `docs/feature-matrix.md:147` "3D spatial audio (kira 0.10)" — workspace is `kira = "0.12"` (`Cargo.toml:209`); ROADMAP crate map, dependencies.md and `crates/audio/src/lib.rs` comments already say 0.12.
  - Aside (same file, `:155`): per-material footsteps `FOOT` `✗` is plausible; not changed since the Wave-4 footstep work (`dd720f82d`) which touches Rapier surface materials, not FOOT records.
- **Status**: NEW
- **Age**: rows predate M28.5/M42.10 (2026-09-18) and the 2026-10 dependency refresh (`f7405adb7`).
- **Effort**: trivial
- **Impact**: the matrix is documented as a status floor; these rows are below the floor.
- **Related**: `docs/engine/dependencies.md:121` also says "kira 0.10's default set" (accurate: refers to 0.10's feature set; leave).
- **Suggested Fix**: flip `:128` to `✓` (or delete as a duplicate of `:124`), `:167` to `~` ("NPC locomotion via KCC; no NPC dynamics"), `:147` to "kira 0.12".
