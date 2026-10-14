# Phase 3 — Native iOS ARM64 host proof-of-concept

Status: **build experiment, not a tested gameplay port**.

The new Rust `fallout3-ios-bridge` package is a static library that links the real `byroredux-bsa` source crate. It exposes two stable C ABI entrypoints: one ABI version and one local BSA reader probe. The SwiftUI iOS host calls the ABI version and makes no unverified gameplay claims.

This experiment intentionally does not compile ByroRedux's Vulkan renderer or its binary, because those are not currently iPhone-compatible.

## GitHub Actions milestone

Workflow: [iOS / native ARM64 host compile](../.github/workflows/ios-arm64-host.yml).

1. Install Rust 1.96.0 and `aarch64-apple-ios` (the 64-bit Apple iOS CPU target).
2. Run Rust bridge unit tests on the macOS runner.
3. Compile the actual Rust static library for iPhone ARM64.
4. Generate a standard native iOS Xcode project with XcodeGen.
5. Build the unsigned iPhone application using the `iphoneos` SDK.

`cargo build -p fallout3-ios-bridge --target aarch64-apple-ios --release --locked`:
- `-p` selects only the bridge package (not the unsupported Vulkan renderer).
- `--target` cross-compiles for iPhone ARM64, not macOS.
- `--release` enables release optimization.
- `--locked` preserves the repository's Cargo.lock dependency versions.

`xcodebuild` flags:
- `-project` names the generated Xcode project.
- `-scheme` selects its app target.
- `-configuration Release` chooses the optimized build configuration.
- `-sdk iphoneos` selects the device SDK rather than an iOS simulator.
- `-destination 'generic/platform=iOS'` requests a generic physical-iPhone build.
- `CODE_SIGNING_ALLOWED=NO` compiles without Apple signing (not installable as-is).

## Acceptance and limitations

The CI job must pass before calling iOS cross-compilation working. After that, installing and launching on an actual iPhone is a **separate manual signing/device gate**; an unsigned CI build is not a tested device application. Neither this shell nor the archive probe plays Fallout 3.
