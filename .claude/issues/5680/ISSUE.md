# SPT-2026-10-12-D3-01: SpeedTree placeholder's `attach_light_offset` comment still says "flame markers / Phase 18"

Labels: low,speedtree,documentation,doc-rot
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5680

Filed from `docs/audits/AUDIT_SPEEDTREE_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: TREE→Billboard Wiring
- **Location**: `byroredux/src/cell_loader/references/import.rs:530-533`
- **Status**: NEW (same doc-rot class as NIFAL-D3-2026-10-12-01, but a different site; that finding covers `attach.rs:866-868`)
- **Description**: `c1a99c25e` renamed `CachedNifImport.flame_attach_offset` to `attach_light_offset` and redefined it as the model's exact-name `AttachLight` node offset (the Phase 18 substring "flame marker" rule was retired). The `.spt` constructor's comment was not updated.
- **Evidence**:
  ```rust
  // SpeedTree placeholders carry no flame markers — they're
  // pure billboard quads. Phase 18.
  attach_light_offset: None,
  ```
- **Impact**: Documentation only. The value `None` is correct (a billboard quad has no `AttachLight` node), but the comment describes a mechanism that no longer exists.
- **Related**: NIFAL-D3-2026-10-12-01, `c1a99c25e`.
- **Suggested Fix**: Reword to "SpeedTree placeholders have no `AttachLight` node — they are pure billboard quads."

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites

