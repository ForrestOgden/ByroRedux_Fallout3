# Initial portability inventory

This is a *source/design review*, not evidence of a successful iOS compilation.

| Area | Present upstream | Native iOS work |
|---|---|---|
| Rust workspace + C++ bridge | Yes | Identify and isolate platform-specific build dependencies |
| BSA, NIF, plugin loaders | Yes | Compile parsers for `aarch64-apple-ios`, validate with original owned data |
| World/ECS/scripting | Partial implementation | Port runtime lifecycle and complete Fallout 3 semantics |
| Renderer | Vulkan 1.3 and ray queries required | Replace mandatory ray-query paths, validate formats and shaders |
| Window/input | Linux/desktop-first | UIKit + CAMetalLayer and touch/Game Controller |
| Audio | Desktop-oriented | iOS-compatible audio backend + interruption handling |
| Save/config | Linux-oriented paths | iOS sandbox Documents/Application Support |
| Code-generating mod runtime | Wasmtime + Cranelift dependencies | Audit for iOS restrictions; disable or replace unsupported JIT |
| Distribution | Rust executable | Xcode app bundle, signing, IPA, legal user-supplied data |

## First technical dependency tests

1. Compile minimal `aarch64-apple-ios` Rust staticlib with an exported C ABI function.
2. Load and call it from a Swift/UIKit iOS app *on hardware*, without private APIs.
3. Enumerate actual Metal GPU capabilities; do not infer extension support from macOS.
4. Establish a native graphics triangle; only then port shaders and engine state.
5. Port the data-only crates before engine-wide cross compilation. Separate native CPU/GPU limitations and game-specific incompleteness in every report.

Never represent MoltenVK as an automatic solution for Vulkan ray queries. Do not bypass Vulkan capability checks or fake supported extensions.
