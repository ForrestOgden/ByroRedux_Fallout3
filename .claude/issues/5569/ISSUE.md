# REN-D5-2026-10-11-01: memory-budget.md egui row describes a retained retired image and cites egui-ash-renderer 0.11

GitHub: https://github.com/matiaszanolli/ByroRedux/issues/5569
Labels: low,renderer,documentation,doc-rot
Source: docs/audits/AUDIT_RENDERER_2026-10-11.md (filed snapshot; GitHub is authoritative for state)

- **Severity**: LOW (documentation drift)
- **Dimension**: Memory/Lifecycle
- **Location**: `docs/engine/memory-budget.md`, section "egui debug overlay (`EguiPass`) — #4986" (the "Retired image awaiting `pending_free`" row and the "egui-ash-renderer 0.11" prose); code side `crates/renderer/src/vulkan/egui_pass.rs` (`promote_partial_deltas`, `dispatch`) and `sync.rs` rider 14.
- **Status**: NEW (related to closed #5026)
- **Description**: In egui-ash-renderer 0.11 and 0.13, `Renderer::set_texture`'s full-upload path creates the new image and descriptor set, then destroys the previous image and frees its descriptor set inside the same call. `pending_free` carries only egui's own `TexturesDelta.free` ids. `sync.rs` rider 14 already states this correctly.
- **Impact**: overstates steady-state VRAM by one atlas-sized image (a few MiB); the real peak is transient (old plus new resident only during the call).
- **Related**: #5026, #4986, #4988.
- **Suggested Fix**: Replace the row with "transient: old image destroyed inside `set_texture` once the replacement exists (peak = 2 images during the call)" and change "0.11" to "0.11/0.13".
