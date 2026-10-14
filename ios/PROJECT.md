# Fallout 3 Native iOS — engineering project

Status: **research and platform bring-up; not a playable iOS game**.

## Objective

Run legally obtained Fallout 3 (PC) game data using a **native ARM64 iOS application**, with a platform-specific renderer, asset loading, runtime systems, controls, and saves. No Windows, x86, PS3, Xbox 360, game streaming, or dynamic console emulation layer is the execution path.

## Source and provenance

- Git upstream: https://github.com/matiaszanolli/ByroRedux
- This fork: https://github.com/ForrestOgden/ByroRedux_Fallout3
- Initial upstream baseline: `ea2d94a1921592ccd3f18d991950a5b868458405` (2026-10-14)
- Toolchain pinned by upstream: `rust-toolchain.toml`, Rust 1.96.0.
- Reuse applicable upstream copyright, package licenses, third-party notices, tests, and history. Upstream `Cargo.toml` declares `license = "MIT"` for workspace packages; an independent repository-wide root license has not been established. Audit before distribution.

## Boundaries

- The PC Fallout 3 original files are user-provided inputs, not repository content or CI artifacts.
- Neither this fork nor its CI may contain Bethesda ESM/BSA/ESP/BA2 data.
- Keep upstream `main` intact until baseline checks and changes are reviewed. Develop through topic branches and pull requests.
- Every milestone must be validated by evidence. A build, file parser, render screenshot, and playable game are *different* levels of proof.
- Run without JIT/dynamically generated native code in the iOS runtime unless allowed by the actual platform and demonstrated.
- Do not remove upstream smoke tests, mark failures successful, or silently skip unsupported parts of the game.

## Work phases

1. Reproducible source baseline and CI (this milestone).
2. Validate independently supplied Fallout 3 data, parsing and gameplay on supported Linux hardware.
3. Native iOS ARM64 launcher and Rust engine boundary.
4. Metal/MoltenVK feature census on the real iPhone.
5. Non-ray-query mobile renderer and actual Fallout 3 scene.
6. User-controlled asset import, integrity checking and caching.
7. Real world traversal and persisted game state.
8. Touch and Game Controller support.
9. Full story/systems compatibility.
10. Performance, thermal and memory optimization.
11. Full-game and DLC regression testing.
12. Signing, packaging and release.

**Stop/go:** An iOS renderer is currently unproven. Do not claim subsequent phases completed merely because a Phase 1 runner turns green.

See [PHASE-1.md](PHASE-1.md), [PORTABILITY-AUDIT.md](PORTABILITY-AUDIT.md), and [ASSET-POLICY.md](ASSET-POLICY.md).
