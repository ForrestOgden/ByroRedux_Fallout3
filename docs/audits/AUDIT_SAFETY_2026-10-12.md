**HEAD**: 5d31fd637 · **Baseline**: `docs/audits/AUDIT_SAFETY_2026-10-08.md` (@ `00f580e09`, 345 commits ago) · **Audited**: Dims 1–8 (every dimension's `Paths:` had commits since the baseline; Dims 1, 6 and 8 were small deltas and got a focused diff read, Dims 2–5 and 7 a full delta read) · **Unchanged since baseline (skimmed)**: none. The unchanged parts inside the dimensions are `crates/fsr3-sys`, `crates/cxx-bridge`, `tools/byro-launcher/src/preflight.rs`, `crates/core/src/ecs/query.rs`, `crates/nif/src/{stream,header}.rs`, `crates/pex/src/opcode.rs`, `GpuMaterial`/`bindings.glsl`, and the `crates/mod-runtime` capability layer (guards spot-checked only).

# Safety Audit — ByroRedux — 2026-10-12

**Command**: `/audit-safety` (default delta scope), one leg of `/audit-suite --preset comprehensive`
**Severity scale**: `.claude/commands/_audit-severity.md`

## Method

- **Scoping.** `git log 00f580e09..HEAD -- <Paths>` was run for each dimension. Counts of commits touching each dimension's paths (the root `Cargo.toml` dependency refresh counts in Dim 2):

  | Dim | 1 | 2 | 3 | 4 / 5 | 6 | 7 | 8 |
  |---|---|---|---|---|---|---|---|
  | Commits | 8 | 14 | 36 | 28 | 9 | 14 | 9 |

- **No sub-agents.** Every dimension was analysed synchronously. Per-dimension notes are in `/tmp/audit/safety/dim_{0,1,3}.md`; `dim_0` is the census and holds the findings ledger.
- **Commands run.** No engine or GPU process was launched and no source was edited. Cargo ran through the rustc 1.96.0 toolchain binary.
  - `cargo test -p byroredux-renderer --lib -- gpu_material gpu_instance gpu_camera gpu_light gpu_terrain volume_far depth_capture shader_constants triangle_frag_scales bindings_glsl the_pin_test scene_descriptor_reflection allocator_lock teardown_path frames_in_flight one_time scratch every_shader_struct no_source_scan`: **171 passed, 0 failed**.
  - `cargo test -p byroredux --bin byroredux -- rapier_release bone_palette_overflow allocator_teardown_order release_ no_source_scan`: **46 passed, 0 failed**.
  - `cargo test -p byroredux-mod-runtime`: **74 passed, 0 failed**.
  - `cargo clippy -p byroredux-renderer --no-deps -- -D clippy::undocumented_unsafe_blocks`: **exit 0** (the `deny` gate holds; the only output is two non-unsafe `doc list item` lints, see "Already tracked").
  - `cargo clippy -p byroredux-fsr3-sys -p byroredux-nif -p byroredux-core -p byroredux-pex -p byroredux -p byro-launcher -p byroredux-ui --no-deps -- -A clippy::all -W clippy::undocumented_unsafe_blocks -W clippy::missing_safety_doc`: exit 0; only the 3 known `unsafe impl AnyBitPattern` adjacency false positives (`crates/nif/src/blocks/bs_geometry.rs:364-366`).
  - `cargo tree -p byroredux-mod-runtime | grep -ic wasi`: **0**. `grep -c require_ crates/mod-runtime/src/runtime/capabilities.rs`: 22 (unchanged).
- **CI evidence.** HEAD run `38275966981` @ `5d31fd637`. Job `114901403217` (Vulkan validation layers, lavapipe) is **success**: `Selected GPU: "llvmpipe …" (… ray query: true, sync2: true)`, `rt-integrity: … rt_supported=1 rt_flag=1 tlas_build=1 … verdict=PASS`, 0 `[Vulkan]` emissions outside the script's own echoed lines, 0 `VUID-` / `Validation Error` lines. Miri (ECS cached-pointer), dhat-heap, shader source/artifact parity and the ABBA lock-order lane are also green. `Test + Check + Clippy` is red on the workspace clippy step only (see "Already tracked"); the dedicated renderer unsafe-block gate step in the same job is green. Log: `/tmp/audit/safety/vk_ci.log`.
- **Dedup.** `/tmp/audit/issues.json` (122 open); `gh` closed/all searches for `hostcall`; the state of every issue the baseline cited (#5270, #5272, #5273, #5456, #5457, #4119, #5368, #5308); today's sibling reports (`AUDIT_RENDERER`, `AUDIT_PERFORMANCE`, `AUDIT_ECS`, `AUDIT_PARSERS`) and `/tmp/audit/REPORTED_THIS_SUITE.md` (re-read just before finalizing; no overlap with the three findings below).

## Census (re-measured at HEAD)

| Crate | Word tokens | `unsafe {` blocks | Δ vs baseline |
|---|---|---|---|
| `crates/renderer/src` | 935 | 715 | **−4 tokens, −5 blocks**: the five `CStr::from_ptr` device/extension-name blocks in `vulkan/device.rs` went to ash's bounded accessors (#5273). No block added. |
| `crates/fsr3-sys/src` | 12 | 10 | none |
| `crates/nif/src` | 13 | 1 (`read_pod_vec_from`) + 3 `unsafe impl AnyBitPattern` | none |
| `crates/core/src` | 7 | 6 | none |
| `crates/pex/src` | 4 | 1 (`transmute`) | none |
| `byroredux/src` | 5 | 2 | none |
| `tools/byro-launcher/src` | 3 | 1 + 1 `unsafe fn` | none |
| `crates/ui/src` | 1 | 0 (prose) | none |
| `crates/cxx-bridge` | — | 1 `unsafe extern "C++"` | still only `native_hello() -> String` |

- **No unsafe newcomer.** Every diff line over 345 commits that carries `unsafe`, `transmute`, `from_raw_parts`, `set_len`, `from_utf8_unchecked` or a raw-pointer cast is either a #5273 removal or a re-indent of an existing `destroy_framebuffer` block in `egui_pass.rs`. The 23 other crates and 3 tools listed as unsafe-free are still unsafe-free (only `crates/sdk` is `forbid(unsafe_code)`; the rest is unenforced, so the sweep was run by hand).
- **Raw byte views.** Every `slice::from_raw_parts` cast in the renderer was re-read (`geometry_ssbo.rs`, `presentation.rs`, `skin_compute.rs`, `water.rs`, `scene_buffer/{upload,descriptors}.rs`, `morph_compute.rs`, `svgf.rs`, `groundcover_bench.rs`, `groundcover/frame.rs`, `mesh.rs`). Each carries a SAFETY text whose layout claim is true (`Vertex` 104 B `repr(C)`, `SkinPushConstants` 32 B, `WaterPush` with explicit `_reserved`, `[[f32;4];4]`). None is new in this window.

## Findings summary

| ID | Severity | Dim | Status | Title |
|---|---|---|---|---|
| SAFE-D4-2026-10-12-01 | LOW | 4 | NEW | #5273 converted `device.rs` to ash's bounded name accessors but left the same unbounded `CStr::from_ptr` at `instance.rs:128` (`layer_name`) and `byro-launcher/src/preflight.rs:230,237` (`extension_name`, `device_name`) |
| SAFE-D8-2026-10-12-01 | LOW | 8 | NEW | `set_hostcall_fuel(max_log_message_bytes + 1024)` rests on a false premise ("largest guest→host payload any interface accepts") and has no pin test |
| SAFE-D2-2026-10-12-01 | LOW | 2 | NEW | The #5456 CWS inflate bound is the file's own `u32` length field, i.e. up to 4 GiB for a hostile header |

Counts: **0 CRITICAL · 0 HIGH · 0 MEDIUM · 3 LOW** (3 NEW, 0 regressions). **Already tracked and still open** (not re-filed): #4119 (LOW, Dim 8), #5621 (CI workspace clippy red, tech-debt). **Closed since the baseline and verified fixed**: #5456, #5457, #5272, #5273 (partially, see SAFE-D4-2026-10-12-01), #5355/#5356.

## Findings

### SAFE-D4-2026-10-12-01: #5273 converted `device.rs` to bounded name accessors but the same unbounded `CStr::from_ptr` survives at `instance.rs:128` and in the launcher preflight
- **Severity**: LOW. Hardening and consistency. Each block is commented and true for a conforming driver. The Vulkan spec guarantees `layerName`, `extensionName` and `deviceName` are NUL-terminated `char[256]`.
- **Dimension**: Unsafe-Block Discipline (Dim 4); the launcher sites are Dim 1 surface.
- **Location**:
  - `crates/renderer/src/vulkan/instance.rs:124-130` (`CStr::from_ptr(layer.layer_name.as_ptr())` in the validation-layer presence check).
  - `tools/byro-launcher/src/preflight.rs:230` (`extension.extension_name`) and `:237` (`properties.device_name`), both inside `unsafe fn describe`.
- **Status**: NEW. #5273 (CLOSED, `74ed66f0a`) was filed from SAFE-D4-2026-10-05-01 and its completeness check asked for the SIBLING pattern to be swept. The fix replaced the four renderer `device_name` blocks and the `extension_name` comparison in `device.rs` only. The guard `device_names_read_through_bounded_accessors` scans `device.rs` alone.
- **Description**: The `CStr::from_ptr` form scans for a NUL with no length bound, so soundness rests on driver conformance. The same driver-supplied fixed arrays are read the same way at three more sites. The launcher probe is the more exposed one: it runs on arbitrary user machines, before the engine and its own checks, and exists precisely to open on setups where the Vulkan stack is unusual. ash 0.38 provides `layer_name_as_c_str()`, `extension_name_as_c_str()` and `device_name_as_c_str()` (the bounded `from_bytes_until_nul` over the fixed array, no `unsafe`), which `device.rs` and `app_events.rs` already use.
- **Evidence**:
  ```rust
  // crates/renderer/src/vulkan/instance.rs:124-130
  // SAFETY: VkLayerProperties::layerName is a null-terminated [c_char; 256]
  // array per the Vulkan spec. ...
  let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
  name == *required
  ```
  ```rust
  // tools/byro-launcher/src/preflight.rs:229-238
  CStr::from_ptr(extension.extension_name.as_ptr()).to_str().is_ok_and(|name| name == wanted)
  ...
  device_name: CStr::from_ptr(properties.device_name.as_ptr()).to_string_lossy().into_owned(),
  ```
  Also present and acceptable: `vulkan/debug.rs:63` (`p_message` is a variable-length driver message with no fixed array to bound) and `fsr3-sys/src/lib.rs:160` (`byro_fsr3_error_string`, a static C string from the vendored shim).
- **Impact**: None on a conforming driver. A non-terminated name would read past the properties struct while formatting a log line or comparing a name. Three avoidable `unsafe` blocks remain, and the codebase spells one conversion two ways.
- **Related**: #5273, #5120, SAFE-D4-2026-10-05-01.
- **Suggested Fix**: Use `layer.layer_name_as_c_str().unwrap_or_default()` at `instance.rs:128` and the two accessors in `preflight.rs`, delete the three `unsafe` spellings, and widen `device_names_read_through_bounded_accessors` (or add a sibling) to scan `instance.rs`. The launcher test cannot reach `crates/renderer`'s `source_scan`, so use a small local scan there.

### SAFE-D8-2026-10-12-01: `set_hostcall_fuel(max_log_message_bytes + 1024)` is justified by a false premise and pinned by no test
- **Severity**: LOW. The error direction is fail-closed (the guest traps and is quarantined), so no memory-safety or isolation property is lost. The defect is that a documented invariant is wrong and unpinned, and that SDK-valid payloads can be refused.
- **Dimension**: Sandboxed Mod Runtime (Dim 8).
- **Location**: `crates/mod-runtime/src/runtime/sandbox.rs:405-417` (added by `0781077b8`).
- **Status**: NEW. Searched `hostcall` in open and closed issues and in all audit reports: no match.
- **Description**:
  - The comment states `max_log_message_bytes` "is the largest guest→host payload any interface accepts (console lines cap at 4 KiB)". That is not true of the WIT surface in `crates/mod-runtime/wit/host.wit`.
  - `set-result-string-array: func(value: list<string>)` is validated by the SDK to `MAX_SCRIPT_ARRAY_ELEMENTS` (512) strings of up to `MAX_SCRIPT_STRING_BYTES` (4 KiB) each (`crates/sdk/src/script_function.rs:21,23`), i.e. up to ~2 MiB.
  - Wasmtime 49 charges `len × size_of::<T>()` for a list plus each string's byte length (`wasmtime-49.0.2/src/runtime/component/func/typed.rs:1530,1777`), and the budget is per host call. At the default `max_log_message_bytes` of 16 KiB the budget is 17,408 B, so a script function returning ~512 forty-byte names (~24 KiB charged) exceeds it although the SDK accepts it.
  - `SandboxConfig::validate()` only rejects `max_log_message_bytes == 0` (`limits.rs:192`), so a config that lowers the log cap to 1 KiB drops the budget to 2 KiB, below the 4 KiB console-line and script-string caps.
  - The commit added no test: `grep hostcall crates/mod-runtime/src` finds only the `sandbox.rs` lines. A later edit that drops or mis-orders the call, restoring the 128 MiB default the commit meant to remove, would pass every test.
- **Evidence**:
  ```rust
  // crates/mod-runtime/src/runtime/sandbox.rs:408-417
  // ... max_log_message_bytes is the largest guest→host payload any
  // interface accepts (console lines cap at 4 KiB); +1 KiB covers
  // the rest of a call's argument-lifting overhead.
  store.set_hostcall_fuel(self.config.max_log_message_bytes + 1024);
  ```
- **Impact**: A legitimate mod returning a large `list<string>` (or any combination whose lifted size exceeds ~17 KiB) hits `HostcallFuelExhausted`, traps, and is quarantined as faulted. The sandbox stays safe. Nothing else is affected.
- **Related**: `0781077b8`, `9046b5ac5` (the sibling wasmtime-48/49 fuel pins, which do have tests).
- **Suggested Fix**: Derive the budget from the largest legitimate payload (the SDK's `MAX_SCRIPT_ARRAY_ELEMENTS × (MAX_SCRIPT_STRING_BYTES + 8)` or an explicit constant next to the other limits), floor it above the console and script-string caps in `validate()`, and add a test that a `string-array` result at the SDK cap succeeds while a 1 MiB `write-line` string is refused before allocation. Correct the comment.

### SAFE-D2-2026-10-12-01: The #5456 CWS inflate is bounded by the file's own `u32` length field, which a hostile header can set to ~4 GiB
- **Severity**: LOW. A decompression-bomb hardening gap on an archive-sourced input. It is not a regression: the stock Ruffle `decompress_swf` has the same property, and before #5479 the shim's `read_to_end` was fully uncapped.
- **Dimension**: Memory Corruption / UB (resource exhaustion on untrusted input; `/audit-parsers` and `/audit-ui` co-own the reader).
- **Location**: `crates/ui/src/prepare.rs:115-130` (`decompress_zlib_after_header`).
- **Status**: NEW. The depth half of #5456 is correct and tested; this is the residual of its "bound the inflate" half.
- **Description**: The fix reads the header's declared uncompressed length (`u32` at `[4..8]`) and caps the inflate with `.take(declared_body + 1)`, declining a body that overruns it. The declared value is attacker-controlled and can be `0xFFFF_FFFF`, so a ~4 MB zlib stream (about 1000:1) can make `read_to_end` grow `body` toward 4 GiB. The bound defends against a stream that lies about a small length, not against a stream that claims a large one. Genuine Scaleform movies are small (the corpus census in `AUDIT_PARSERS_2026-10-12.md` inflates every archived `.swf` in five games without trouble).
- **Evidence**:
  ```rust
  let declared_body =
      u64::from(u32::from_le_bytes(swf_data[4..8].try_into().ok()?).saturating_sub(8));
  ...
  let decoder = flate2::read::ZlibDecoder::new(&swf_data[8..]).take(declared_body + 1);
  let mut body = Vec::new();
  { decoder }.read_to_end(&mut body).ok()?;
  ```
- **Impact**: A crafted mod archive can make the engine allocate up to ~4 GiB while preparing a menu or HUD movie, risking an OOM kill on smaller machines. Retail content is unaffected.
- **Related**: #5456, #5479. Related observation recorded by the parsers audit (not mine, no action here): Skyrim SE `interface\fonts_pl.swf` inflates one byte past its CWS header length, so `prepare.rs` correctly declines it to the stock loader.
- **Suggested Fix**: Apply an absolute ceiling (for example `declared_body.min(MAX_SWF_BODY_BYTES)` with a few tens of MiB, well above any shipped movie) before `.take`, and add a test with a header claiming `u32::MAX` over a small bomb stream.

## Already-tracked items touched by this window (not re-filed)

- **#5621 (OPEN, TD8-2026-10-11-01).** The workspace `cargo clippy` step is red at HEAD on three lints: `crates/physics/src/world/mod.rs:935` (`map` on an `Option` whose closure returns `()`), and `crates/renderer/src/texture_registry/dynamic_rgba.rs:38,39` (`doc list item without indentation`, introduced by #5528). Safety impact: none. The dedicated renderer unsafe-block gate step (`ci.yml:205`) ran and passed in the same job, so the #5121 design is working as intended. `AUDIT_ECS_2026-10-12.md` noted the same lints.
- **#4119 (OPEN).** `queue_increment_own_i64` still lacks the visibility check (Dim 8). No commit touched it.

## Prior-report findings: status at HEAD

| Prior ID | Issue | Status at HEAD |
|---|---|---|
| SAFE-D2-2026-10-08-01 (`DefineSprite` recursion, no depth cap) | #5456 CLOSED | **Fixed** (`8872f3725`). `MAX_SPRITE_DEPTH = 1`, the walker takes a `depth` argument, and the regression test builds 100,000 nested levels. See SAFE-D2-2026-10-12-01 for the inflate residual. |
| SAFE-D4-2026-10-08-01 (`load_shader_module` SAFETY names `chunks_exact(4)`) | #5457 CLOSED | **Fixed** (`cc8a5df8f`). The text now cites `as_chunks::<4>()` and the `assert!` on the length. |
| SAFE-D3-2026-10-05-01 (`remove_body` prunes 1 of 3 maps) | #5272 CLOSED | **Fixed** (`26a1277bc`). `remove_body` (`crates/physics/src/world/mod.rs:571`) now removes `body_labels`, `explosion_offences` and `keyframe_refusals_logged`; pinned by `remove_body_prunes_the_offence_ladder_and_refusal_set`. The sibling #5355 (articulation joint index) is also closed. |
| SAFE-D4-2026-10-05-01 (renderer `CStr::from_ptr(device_name)`) | #5273 CLOSED | **Fixed in `device.rs`** (5 blocks removed, pinned by `device_names_read_through_bounded_accessors`). Siblings remain: SAFE-D4-2026-10-12-01. |
| #4119 | OPEN | Unchanged. |

## Per-dimension results

### Dimension 1 — FFI lifetime safety: PASS, 0 new
- **`fsr3-sys`, `cxx-bridge`, launcher `preflight.rs`.** No commits. `Context::create` / `dispatch` keep their `# Safety` sections. `probe()` still destroys the `VkInstance` on every path (the `enumerate_physical_devices` error arm and the normal exit). `cxx-bridge` still exposes only `native_hello() -> String`, so there is no raw-pointer, borrowed-slice or `Box` crossing.
- **`frame_upscaler.rs`.** A 2-line test-only change.
- **`crates/ui`.** Safe Rust throughout. The Ruffle nightly-2026-10-10 / wgpu 27→30 bump (`b91e82160`) changed constructor signatures (`create_wgpu_instance(…, None)`, `SwfMovieData::from_data`) with no `unsafe`. `86f5a1a0a` queries Ruffle's callback registry before invoking and drops the player lock before `prune_callback`, which is lock-order safe. #5456 is verified above.
- **Launcher.** `app.rs` / `main.rs` / `state.rs` changes are UI and argument handling (`--profiles` absolutised, #5476). The only residual is the unbounded name reads: SAFE-D4-2026-10-12-01.

### Dimension 2 — Memory corruption / UB: 1 LOW
- **LZ4 pin intact.** `Cargo.toml:168` is `lz4_flex 0.14` with `default-features = false` and the explicit list `std, safe-encode, safe-decode, frame, checked-decode`, byte-identical to 0.14.0's own `default`. `crates/bsa` is the only dependent (`workspace = true`).
- **pex opcode.** The compile-time assert (`crates/pex/src/opcode.rs:74`) and the runtime bound (`:136`) are intact; the file has no commit since the baseline.
- **NIF POD read.** `stream.rs` and `header.rs` have no commit. The single `read_pod_vec_from` site and the big-endian gate are unchanged.
- **Dependency refresh.** glam 0.29→0.33, nalgebra 0.35, rapier 0.36, wasmtime 49, kira 0.12, egui 0.36, image 0.25 added no `unsafe` and no byte-cast of a glam or rapier type: every `NoUninit` implementor and every `from_raw_parts` target is a `repr(C)` scalar struct, not a glam type.
- **`NoUninit`.** 37 explicit `unsafe impl NoUninit for` sites plus the primitive macro and the `[T; N]` blanket; no new one. The layout pins pass.
- **New safe byte helper.** `byroredux_core::bytes::{le_u16_at, le_u32_at, le_f32_at}` (#5596) use checked offset arithmetic. `dds::read_u32` now returns an error on a truncated header instead of indexing. `parse_dds`'s new 32-bpp / `DDPF_LUMINANCE` branches (#5378, #5402) route through `uncompressed_expand_meta`, which keeps `validate_expand_masks` and the #4835 payload-length check; `expand_uncompressed_rgb` re-checks the source size and reads through `data.get(..)`. The zero-copy 32-bpp arms are length-checked at upload as before.
- **Stack depth.** Walkers: ECS hierarchy via `collect_subtree`'s guard (#5513 centralised it; `AUDIT_ECS_2026-10-12.md` ECS-D3-01 covers the remaining hand-rolled walks), ui sprite walker now capped.
- **Finding:** SAFE-D2-2026-10-12-01.

### Dimension 3 — Leaks and drop ordering: PASS, 0 new
- **Rapier.** `release_sweeps_both_ragdoll_and_rapier_handles` and `rapier_release*` are green. `remove_body` now prunes all per-body maps (#5272). The rapier 0.36 additions (`water_force_targets` cleared at the top of each `apply_buoyancy`, `water_lift` cleared per frame and tolerant of a removed body, `pending_force_events` drained per substep and cleared per step, `pending_query_leaves` drained) do not grow across frames.
- **#5490 (`99cd9fd6c`).** The session-replace purge now hands the pending convoy's whole `Children` closure (minus the player rig) to `release_entities` instead of a bare `despawn_batch`, so the mesh / texture / BLAS, item-instance and Rapier release run. A dangling-`Parent` path that made `validate_world` refuse every save is closed. Pinned by `pending_readoption_victims_are_the_whole_subtree_closure` and `…_spare_the_player_rig`.
- **#5528 (`e588e5e67`).** `GrowthBackoff` retries staging-arena growth every 120 frames or on a smaller request, so a persistent host-visible shortage no longer pays one failing `vkAllocateMemory` per frame. The gate precedes the allocation; the degrade-not-die contract is intact.
- **egui.** The 0.36 `drop_without_applying_deltas` discard is applied at the four sites (`app_frame.rs`, `context/mod.rs`, `teardown.rs`, `DebugUiState::discard_last_output`). `dispatch` now takes `textures_set` / `textures_free` out of the `FullOutput` first so an early `?` cannot trip the assert. Note: on that early-return path the taken `textures_free` is dropped without reaching `pending_free`, leaking those texture ids until teardown; the pre-change code behaved the same (`output` dropped before step 5), so this is not a regression and the path only fires on an upload error. The one-frame deferral and the all-slots fence wait (rider 14, `sync.rs`) still cover the full-delta retirement; `frames_in_flight_contract_names_every_dependent_resource` is green.
- **`AllocatorResource` drop order.** `app_events.rs:133` removes it before `VulkanContext` drops; `allocator_teardown_order_tests` is green. No commit touched `allocator.rs` production code (test-scan helpers only).
- **Deferred destroy.** Still four production `DeferredDestroyQueue<T>` instantiations; textures still drain through `tick_deferred_destroy`.
- **#5583 (`b7d40dc81`).** One `MAX_SKIN_SLOTS` constant (1364) now feeds the slot pool, `SKIN_MAX_SLOTS` and the per-frame bind-inverse upload cap, with a const assert `cap >= pool`. The palette stays within `MAX_TOTAL_BONES` (`(1364 + 1) × 144 = 196,560 ≤ 196,608`), and the over-capacity test still asserts a bounded palette.

### Dimension 4 — Unsafe-block discipline: 1 LOW
- **Guard intact.** `#![deny(clippy::undocumented_unsafe_blocks)]` at `crates/renderer/src/lib.rs:21`, no `allow(clippy::undocumented…)` anywhere, and both CI steps survive (`ci.yml:190` `--keep-going`, `:205` dedicated renderer gate). Locally the gate command exits 0.
- **Invariant truth on changed blocks.** The only `unsafe` the window touched in the renderer is the `device.rs` removal and the re-indented `destroy_framebuffer` blocks in `egui_pass.rs` (their SAFETY text, "created by this `device` just above, never recorded", is true: the arm runs on `Renderer` init failure before any use). The #5457 SAFETY rewrite in `pipeline.rs` is accurate.
- **Finding:** SAFE-D4-2026-10-12-01.

### Dimension 5 — Vulkan spec compliance: PASS, 0 new
- **Live validation.** HEAD lavapipe job is green, with RT on, a selected device line and zero validation messages (see Method). It boots only the 5-frame default scene, so it is not coverage of any game-data path.
- **Changes read.** `egui_pass.rs` (0.13 per-texture API; each `set_texture` is a synchronous one-shot submit under the queue mutex), `post_passes.rs` (volumetrics cluster-buffer sizes now come from `ClusterCullPipeline::{grid,index}_buffer_size()`, #5619, instead of a re-typed formula, which removes a descriptor-range drift risk), `context/mod.rs`, `build_and_upload_instances.rs` / `restir.rs` (a pure CPU rig key, #5369/#5507), `exposure_meter.rs` and the compute workgroup widths (#5620: shaders and host now read one generated constant, and the exposure shader's shared arrays are sized from it, closing a latent over-read on a retune), `svgf.rs` (#5211 source-level pins for the finite-value guards).
- **TLAS / BLAS / scratch.** No production change in `acceleration/` (`blas_static.rs` is a log-string reflow). Scratch tests green.
- **Depth capture, compute layouts, SPIR-V reflection.** Unchanged; the reflection and pin tests are in the green set.

### Dimension 6 — GPU struct layout: PASS, 0 new
- **Pins green.** `gpu_material_size_is_432_bytes`, `gpu_instance_is_160_bytes_std430_compatible`, `gpu_camera_is_368_bytes`, `gpu_light_is_64_bytes`, `gpu_terrain_tile_is_176_bytes`, the per-field offset test, `every_shader_struct_is_classified`, the `bindings_glsl` and `gpu_material_size_claims` guards all pass in the 171-test run.
- **Window changes.** `gpu_types.rs` +11 lines (docs), `material.rs` and `bindings.glsl` untouched, `scene_buffer/constants.rs` the #5583 constant. The `GpuInstance` standalone shader copies did not move.

### Dimension 7 — GPU-fed data and loop bounds: PASS, 0 new
- **Glass loop.** `MAX_REFRACT_PASSTHRUS` and its pin are green. The #5565 / #5249 transmission skip walk in `include/shadow_transport.glsl` is bounded by `MAX_TRANSMISSION_SELF_SKIPS` (8) at `:227`; every exit now calls `traceShadowGlassLegRemaining`, which is a single non-recursive call into the already-bounded glass loop (`MAX_GLASS_INTERFACES`). No unbounded recursion was introduced.
- **#3991 latch.** `byroredux/src/app_frame.rs:727` still reads `!ctx.skin_state_submitted || ctx.bind_inverse_upload_failed`; its source pins are green.
- **Bone palette.** `SkinSlotPool` overflow tests are green (46-test run).
- **NaN / Inf.** `material_translate.rs` and `resolve_pbr` are unchanged (only a boundary-twin collapse, #5281). The #5211 SVGF guards are pinned at source level. The water-wave tuning is now a single core-owned constant block shared by `water.vert`, the GPU upload and the CPU crest sampler (#5582); the physics-side mirror cannot drift.

### Dimension 8 — Sandboxed mod runtime: 1 LOW
- **Absence.** `cargo tree -p byroredux-mod-runtime` prints no `wasi` entry. The wasmtime feature list is unchanged at 49.0.2 (`anyhow, component-model, cranelift, runtime, std`).
- **Gating.** 22 `require_` guards in `capabilities.rs`, unchanged. No change under `crates/sdk` or `byroredux/src/extensions/` beyond `d8a6e58bf` / `627149b62` / `c85d50356` (CLI flag lookups, `ActorValue::undamaged` composition, KILL raising); none touches a host function.
- **New pins.** `0781077b8` pins `wasm_wide_arithmetic(false)` and `wasm_relaxed_simd(false)` and bounds hostcall data; `9046b5ac5` pins the wasmtime-48/49 per-byte `memory.fill` / `copy` / `init` fuel charge and the `call_ref` fuel fix (GHSA-m63x-6p34-q65x). The full crate suite is green (74 passed).
- **Finding:** SAFE-D8-2026-10-12-01 (the hostcall-fuel premise and its missing pin). #4119 is still open.

## Skill-sync notes (for the next `/audit-safety` edit, not findings)
- **Census line.** Renderer is now 935 word tokens / 715 blocks; all five `CStr::from_ptr` device/extension-name blocks are gone from `vulkan/device.rs`. Residual unbounded name reads are `instance.rs:128` and the two in `byro-launcher/src/preflight.rs`.
- **Dim 3 first step** can drop the #5272 caveat: it is fixed and pinned.
- **Dim 8** should add the hostcall-fuel budget as a checked limit alongside `SandboxConfig::validate()`, and note that the capability-versus-host-function enumeration test the skill asks for still does not exist.
- **Dim 2** could name `crates/ui/src/prepare.rs` as a bounded-recursion site (depth cap 1) and its inflate bound.
- Aside, not safety: `crates/debug-ui/src/lib.rs` (~:229) has `DebugUiState::set_visible`'s doc line "Show or hide the overlay explicitly" displaced above the new `discard_last_output` helper by `500fcb8ca`, the same inserted-between-function-and-doc-comment pattern `REN-D6-2026-10-12-01` reported in `merge.rs`.

Publish with: `/audit-publish docs/audits/AUDIT_SAFETY_2026-10-12.md`
