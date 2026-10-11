# TD2-2026-10-11-07: "clone the clip's channels, then `attach_animation_sinks`, then read the phase" is pasted at three NIF animation spawn sites

URL: https://github.com/matiaszanolli/ByroRedux/issues/5590
Labels: low, animation, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 2 - Logic Duplication
- **Location**: `byroredux/src/scene.rs:769-802`, `byroredux/src/scene/nif_loader.rs:607-637`, `byroredux/src/cell_loader/spawn.rs:869-896`; callee `byroredux/src/anim_convert.rs:129`
- **Status**: NEW
- **Age**: all three predate 2026-10-08 (#3345 phase wiring).
- **Effort**: trivial
- **Description**: each site scopes a registry read, clones four channel vectors (`bool/float/color/texture_flip`) into a tuple, drops the guard, calls `attach_animation_sinks(world, &b, &f, &c, &t, Some(ctx), Some(provider), root)`, then separately does `registry.get(handle).map(|c| c.phase).unwrap_or(0.0)` into `AnimationPlayer::new(h).with_phase(phase)`. `anim_convert.rs:123-127` already says every production caller passes both `Some`.
- **Evidence**: `grep -rn 'texture_flip_channels.clone()' byroredux/src` -> exactly those three sites.
- **Impact**: a fifth channel kind (the sink signature already takes four slices) is a 3-site edit plus the helper.
- **Related**: #3345.
- **Suggested Fix**: add `pub(crate) fn attach_clip_sinks_and_phase(world, clip_handle, ctx, tex_provider, root) -> f32` in `anim_convert.rs` that does the registry read, clone, `attach_animation_sinks`, and returns the phase; the three sites call it and build the `AnimationPlayer` from the returned phase.
