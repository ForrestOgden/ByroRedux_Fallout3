# Phase 4 (part 1) — native Metal raster pipeline proof

This is an iOS device diagnostic for Fallout: New Vegas **engine-port feasibility**, not a game renderer.

The app retains the actual ByroRedux Rust BSA reader linked as a native ARM64 static library. The added `MetalTriangleView` uses an `MTKView`, a compiled `.metal` vertex/fragment shader pair, a render pipeline state, and an actual command buffer/draw call.

## Tests and observations

- CI must compile the Rust static library with `--target aarch64-apple-ios`.
- Xcode must compile `TriangleShaders.metal` and bundle the resulting default shader library.
- The iOS Swift host and IPA must build successfully and be uploaded without requiring Apple certificates.
- On real iPhone 16 Pro Max, a colored triangle should appear. Save a screenshot showing the triangle, device name, ray-tracing family capabilities, ABI 1 and the BSA missing-file probe result 1.
- A successful IPA build is **not** hardware GPU verification; the device test is still mandatory.
- This is Metal-native rasterization, not the upstream Vulkan ray-query renderer or MoltenVK. No Fallout: New Vegas world assets are loaded.

## Future

After physical hardware passes, investigate rasterized, no-ray-query world rendering. Only then bring NIF geometry and actual game cells into the renderer. The `GSProspectorSaloonInterior` scene is the first true content target.

## Manual ownership gates

1. User signs and installs the unsigned diagnostic build using an Apple-account-supported signing workflow.
2. User launches the diagnostic on the physical iPhone.
3. User provides screenshot or reports which displayed test outputs failed.

No Bethesda game content or Apple signing credentials may be uploaded to the public repository.
