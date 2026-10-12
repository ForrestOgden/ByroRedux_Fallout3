# #5660 — PAR-D2-2026-10-12-01: the flate2 1.1.10 lock refresh (317d4b7a9) makes #5073 live and the strict BSA real-data lane red; no default-lane test pins empty/truncated-stream semantics, and the CI job's missing `--no-fail-fast` masks `csg_real`

**Labels**: `medium`, `bug`, `import-pipeline`, `dependencies`, `test-gap`, `game:oblivion`, `game:fo3`, `game:fnv`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-2026-10-12-01) · audited at HEAD c1a99c25e

- **Severity**: MEDIUM
- **Dimension**: Error Semantics / Corpus Gates
- **Location**:
  - `Cargo.lock` at `317d4b7a9` ("chore(deps): refresh Cargo.lock within existing semver ranges"): flate2 1.1.9 → 1.1.10.
  - `crates/bsa/src/safety.rs` (`inflate_bounded_zlib`, about :157-200). Its doc says "A *short* decode is deliberately still `Ok`".
  - `crates/bsa/src/archive/extract.rs`: the compressed branch, which has no empty-payload short-circuit.
  - `crates/bsa/tests/bsa_real.rs:160-245`: `oblivion_all_bsas_v103_brute_force_extract_zero_errors`.
  - `.github/workflows/real-data-gates.yml:207-208`: `cargo test --release -p "byroredux-${crate}" -- --ignored …`, with no `--no-fail-fast`.
- **Status**: NEW. The defect class is #5073 (open, LOW, filed 2026-09-29 as a hard error). What is new:
  1. At the repo's own lock it was **not** an error until 2026-10-10. Under 1.1.9 these entries extract as `Ok(empty)`. The 09-29 Oblivion brute force recorded 147,629 files and 0 errors on 1.1.9. #5073's error was most likely observed on an unlocked probe crate that resolved a newer flate2.
  2. It is now live in the engine.
  3. The strict lane is red at HEAD.
- **Trigger Input**: a BSA v103/v104 entry flagged compressed whose record size is 4: a 4-byte original-size header of 0 and no zlib stream.
- **Description**:
  - flate2 1.1.10 changed `read::ZlibDecoder` on an empty or truncated input from `Ok(0)` to `Err(UnexpectedEof, "incomplete deflate stream")`.
  - `inflate_bounded_zlib` returns that error. Its raw-DEFLATE retry needs `len >= 2`.
  - So every empty-payload compressed entry now fails `extract`, while `declared_size` still returns `Ok(0)`.
  - Nothing in the default lane pins this behaviour. `short_decode_stays_ok_for_the_shipped_padding_deltas` uses a *complete* stream with a larger declared size, so it stays green under both versions.
  - The documented "short decode stays `Ok`" contract therefore depends silently on the decoder version for truncated streams.
  - The gate that caught the change is the `--ignored` Oblivion brute force. Its CI lane has no registered runner (`gh api …/actions/runners` reports `total_count: 0`), and the scheduled runs on 10-08 through 10-11 were all cancelled. So the change landed unseen.
  - When the bsa suite does run, `bsa_real` failing stops cargo before the `csg_real` binary.
- **Evidence**:
  - `flprobe`, `ZlibDecoder::new(&[]).take(1).read_to_end`:
    - 1.1.9: `Ok(0)`
    - 1.1.10: `Err(Custom { kind: UnexpectedEof, error: "incomplete deflate stream" })`
  - `zsweep` (flate2 1.1.10, all 8 installs, 3,928,089 entries): 37 errors, every one `incomplete deflate stream` with `declared=Some(0)`.
    - Oblivion: 35 × `textures\menus*\…\*.txt` in `Oblivion - Textures - Compressed.bsa`.
    - FO3: `Fallout - Meshes.bsa : meshes\dungeons\vaultruined\placeholder.txt`.
    - FNV: `Fallout - Misc.bsa : menus\do_not_delete.txt`.
  - `zsweep119` (the same 37 entries, flate2 pinned to 1.1.9): 0 errors.
  - Blast radius checked:
    - No vanilla archive has a truncated stream; the only errors are the 37 empty ones.
    - ESM compressed records: 597,097, identical under both versions.
    - Archived SWFs: identical under both versions.
  - Strict lane at HEAD: `bsa_real … FAILED. 7 passed; 1 failed`, after which `csg_real` never ran. Run alone, `csg_real` passes 1/1.
- **Impact**:
  - **Engine:** none visible. No content loads these `.txt` files.
  - **Extract-all tooling and corpus sweeps:** they see 37 hard errors on vanilla data.
  - **Gates:** the BSA strict lane stays red until #5073 is fixed. Any other regression in `bsa_real` hides behind the known failure, and `csg_real` does not run at all.
  - **Future bumps:** the next semver-compatible refresh can change decoder semantics again with no default-lane signal.
- **Related**: #5073, #3812, #3720, #622/#812, #4659 (nightly lane), `20ad630a6` (lz4_flex 0.14, cleared by the same sweep).
- **Suggested Fix**:
  - Fix #5073: in the compressed branch, `original_size == 0 && compressed_len == 0` → `Ok(Vec::new())`.
  - Add default-lane unit tests in `safety.rs` that pin the empty-stream and truncated-stream outcomes, so a decoder bump fails `cargo test --workspace` rather than only the unrunnable nightly.
  - Add `--no-fail-fast` to the `parsers` job's `cargo test` call.

## Completeness Checks
- [ ] **SIBLING**: The same empty/truncated-stream edge is pinned on the v105 LZ4-frame arm, the BA2 GNRL/DX10 zlib paths, `CsgArchive`, and the ESM reader (`crates/plugin/src/esm/reader.rs`), which uses the same zlib pattern
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
