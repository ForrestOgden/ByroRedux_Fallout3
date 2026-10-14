# Phase 1 — inherited fixed baseline and test gates

## Known fixed baseline

```text
Upstream: matiaszanolli/ByroRedux
Commit: ea2d94a1921592ccd3f18d991950a5b868458405
Branch under development: fnv/native-ios-transition
Rust: 1.96.0 (rust-toolchain.toml)
Environment: Linux / Ubuntu 24.04 or GitHub Actions ubuntu-latest
Game data: not required for unit tests
GPU: not required for unit tests
```

## Required evidence

- The baseline commit is a real ancestor of the fork branch.
- Source-only assets policy passes.
- GitHub Actions runs Rust 1.96.0 with the committed Cargo.lock.
- `cargo check --workspace --locked` passes.
- `cargo test --workspace --locked --no-fail-fast` passes.
- Record the actual workflow run URL, failing commands and relevant logs; never claim green before GitHub shows success.
- The upstream `CI` workflow may run additional checks on the pull request. Failures count; do not hide them.

## Local Ubuntu / WSL2 verification

```bash
sudo apt-get update
sudo apt-get install -y git curl build-essential g++ libasound2-dev libvulkan-dev
git clone https://github.com/ForrestOgden/ByroRedux_FalloutNV.git
cd ByroRedux_FalloutNV
git switch fnv/native-ios-transition
rustup toolchain install 1.96.0 --profile minimal
cargo check --workspace --locked
cargo test --workspace --locked --no-fail-fast
```

Command meanings:
- `apt-get update`: refresh Ubuntu package indexes; `install -y`: install dependencies while automatically accepting package prompts.
- `git clone`: copy the repository and its history; `git switch`: select the named development branch.
- `rustup toolchain install`: install compiler version 1.96.0; `--profile minimal`: install only the base Rust components.
- `cargo check`: type-check without producing a final executable; `--workspace`: include all Rust workspace crates; `--locked`: do not modify `Cargo.lock` for dependency resolution.
- `cargo test`: compile and execute unit tests; `--no-fail-fast`: continue through test binaries after one fails to expose more problems.
- Use the rustup-managed Cargo toolchain, not an older distribution Cargo that may shadow it.

## Phase 1 exit

**Not complete until the first GitHub Actions run reports success and its log is reviewed.** A missing run is a **blocked/unexecuted** gate, not a passing gate. No Fallout: New Vegas gameplay claims derive from these generic tests.
