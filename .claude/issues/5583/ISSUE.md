# TD7-2026-10-11-02: `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME = 1366` is a hand-typed copy of a formula that evaluates to 1364, and the formula is re-typed again beside it

URL: https://github.com/matiaszanolli/ByroRedux/issues/5583
Labels: medium, renderer, tech-debt, bug
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md


- **Severity**: MEDIUM (the lockstep failure it guards has already shipped twice: #1198, #1284)
- **Dimension**: 7
- **Location**:
  - `crates/renderer/src/vulkan/scene_buffer/constants.rs:69,104`
  - `crates/renderer/src/vulkan/context/mod.rs:81` (`SKIN_MAX_SLOTS`)
  - `byroredux/src/main.rs:1155`
  - 8 test files under `byroredux/src/render/` (`bone_palette_overflow_tests.rs:96`, `variant_pack_gating_tests.rs:15`, `parallax_alpha_gate_tests.rs:56`, `static_mesh_fx_skip_tests.rs:10`, `tint_alpha_gate_tests.rs:28`, `water_wave_params_tests.rs:72,174`, `fog_curve_propagation_tests.rs:13`, `render_origin_shared_tests.rs:17`)
- **Status**: NEW
- **Age**: a3c2836a6, 2026-05-28 (#1284)
- **Effort**: trivial
- **Description**: The slot-pool capacity is `MAX_TOTAL_BONES / MAX_BONES_PER_MESH - 1`, which is 1364 (slot 0 is the identity). It is computed three separate times in non-test code: `SKIN_MAX_SLOTS` (`context/mod.rs:81`), `main.rs:1155`, and the eight test copies. The per-frame first-sight upload cap `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME` is not derived from any of them.
- **Evidence**:
  - `constants.rs:104` has `= 1366`. Its doc says "(= `MAX_TOTAL_BONES (196608) / MAX_BONES_PER_MESH (144)`)". That quotient is 1365.33, floor 1365, and the real pool is 1364. 1366 matches neither.
  - `grep -rn '1366\|1364' --include='*.rs' crates byroredux` finds no test or assert tying the cap to the pool size.
  - History: the cap was 16, then 227, then 1366. Each bump was a hand edit to follow `MAX_TOTAL_BONES`. #1198 records the symptom of letting them diverge: "one-frame bind-pose glitch when more than 16 skinned NPCs first-sighted in a single frame".
  - `core/src/ecs/resources/skin_slot_pool.rs:180` tells the next person to "Bump MAX_TOTAL_BONES". Doing so leaves 1366 stale. Staging size is `1366 × 144 × 64 B` at `buffers.rs:691`. `upload.rs:526` silently defers the excess past the cap, so the failure is a one-frame glitch, not a crash.
- **Impact**: Today the cap is 2 slots over capacity, about 18 KB, so it is harmless. The risk is the documented lever: raise `MAX_TOTAL_BONES` or change `MAX_BONES_PER_MESH` and the upload cap stays at 1366 (under-cap, glitch returns) or over-allocates staging.
- **Related**: #1192, #1198, #1284, #1299 (all closed).
- **Suggested Fix**:
  - Define `pub const MAX_SKIN_SLOTS: usize = MAX_TOTAL_BONES / MAX_BONES_PER_MESH - 1;` in `scene_buffer/constants.rs`.
  - Set `MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME = MAX_SKIN_SLOTS`.
  - Define `SKIN_MAX_SLOTS` from the same constant, and have `main.rs:1155` and the eight tests import it instead of re-deriving.
  - Add a `const _: () = assert!(MAX_PENDING_BIND_INVERSE_UPLOADS_PER_FRAME >= MAX_SKIN_SLOTS);`.

### LOW
