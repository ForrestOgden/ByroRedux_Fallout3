# SAFE-D4-2026-10-12-01: #5273 converted `device.rs` to bounded name accessors but the same unbounded `CStr::from_ptr` survives at `instance.rs:128` and in the launcher preflight

- **GitHub**: #5717 (https://github.com/matiaszanolli/ByroRedux/issues/5717)
- **Labels**: low,safety,renderer,tech-debt,bug
- **Report**: docs/audits/AUDIT_SAFETY_2026-10-12.md

**Severity**: LOW. Hardening and consistency. Each block is commented and true for a conforming driver. The Vulkan spec guarantees `layerName`, `extensionName` and `deviceName` are NUL-terminated `char[256]`.

## Location
- `crates/renderer/src/vulkan/instance.rs:124-130` (`CStr::from_ptr(layer.layer_name.as_ptr())` in the validation-layer presence check).
- `tools/byro-launcher/src/preflight.rs:230` (`extension.extension_name`) and `:237` (`properties.device_name`), both inside `unsafe fn describe`.

## Description
The `CStr::from_ptr` form scans for a NUL with no length bound, so soundness rests on driver conformance. #5273 (CLOSED, `74ed66f0a`) was filed from SAFE-D4-2026-10-05-01 and its completeness check asked for the SIBLING pattern to be swept. The fix replaced the four renderer `device_name` blocks and the `extension_name` comparison in `device.rs` only. The guard `device_names_read_through_bounded_accessors` scans `device.rs` alone. The same driver-supplied fixed arrays are read the same way at three more sites. The launcher probe is the more exposed one: it runs on arbitrary user machines, before the engine and its own checks, and exists precisely to open on setups where the Vulkan stack is unusual. ash 0.38 provides `layer_name_as_c_str()`, `extension_name_as_c_str()` and `device_name_as_c_str()` (the bounded `from_bytes_until_nul` over the fixed array, no `unsafe`), which `device.rs` and `app_events.rs` already use.

## Evidence
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

## Impact
None on a conforming driver. A non-terminated name would read past the properties struct while formatting a log line or comparing a name. Three avoidable `unsafe` blocks remain, and the codebase spells one conversion two ways.

## Suggested Fix
Use `layer.layer_name_as_c_str().unwrap_or_default()` at `instance.rs:128` and the two accessors in `preflight.rs`, delete the three `unsafe` spellings, and widen `device_names_read_through_bounded_accessors` (or add a sibling) to scan `instance.rs`. The launcher test cannot reach `crates/renderer`'s `source_scan`, so use a small local scan there.

**Related**: #5273, #5120, SAFE-D4-2026-10-05-01

## Completeness Checks
- [ ] **UNSAFE**: The fix removes `unsafe`; confirm no new `unsafe` and that the renderer unsafe-block gate (`ci.yml`) count drops accordingly
- [ ] **SIBLING**: Re-grep `CStr::from_ptr` across `crates/` and `tools/` after the change
- [ ] **TESTS**: Widened bounded-accessor source scan covers `instance.rs` and the launcher preflight

Fix with: `/fix-issue 5717`
