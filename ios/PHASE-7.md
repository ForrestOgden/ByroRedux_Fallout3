# Phase 7 — Lit, touch-controlled native NIF preview

The owner's 6.7-second A19 Pro recording confirms authentic FNV `fnv-preview.nif` (142 triangles) is parsed with native ByroRedux and rendered in Metal. The material appears flat, and the model cannot be inspected interactively.

This stage retains the verified 32-byte Rust-to-Metal vertex ABI and the private local Files import:
- Geometry-derived two-sided flat Lambert-style shading per triangle, bounded between 0.28 and 1.0, with guardrails for degenerate/nonfinite triangles. This is an **untextured diagnostic**, not the original New Vegas materials or full dynamic lighting.
- One-finger orbit drag, pinch zoom (clamped), double-tap camera reset, and slower automatic yaw.
- Larger 340-point preview and visible `lit orbit v4` marker.

Success requires GitHub's pinned Rust tests, native ARM64 build, Swift/Metal build, unsigned IPA artifact, and an actual iPhone gesture test. No Bethesda files are added to the repository. Textures, cell loading, collision, animation, audio and gameplay remain future work.
