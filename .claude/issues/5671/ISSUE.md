# #5671 — PAR-D4-2026-10-12-02: the HKX census asserts `rejects <= 288` while claiming to catch "drift in either direction", pins a count rather than a set, and its block-consistency clause is inert

**Labels**: `low`, `bug`, `import-pipeline`, `animation`, `test-gap`, `game:skyrim`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D4-2026-10-12-02) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Corpus Gates
- **Location**: `crates/hkx/src/animation.rs`:
  - `:1326-1342`: `assert!(rejects.len() <= KNOWN_VANILLA_REJECTS)`; the comment at `:1332` says "catches drift in either direction".
  - `:1347-1350`: `div_ceil(MAX_FRAMES_PER_BLOCK) >= max_blocks || max_blocks <= 64`.
- **Status**: NEW (d4352d477, the #5326 fix). Distinct from #5446, which covers the unit fixtures.
- **Trigger Input**: n/a.
- **Description**:
  - A one-sided `<=` catches only growth. Once PAR-D1-2026-10-12-02 lands (288 → 59), a regression re-rejecting up to 229 clips stays green unless the constant is lowered too.
  - A swap of which clips are rejected passes, because only the count is pinned.
  - The block clause holds only through its `|| max_blocks <= 64` arm: measured `1471.div_ceil(256) = 6 < 15`. Its "consistency" half never constrains anything.
- **Evidence**: see the location above. The strict run printed `census: 288 clips rejected by the real decoder` and passed.
- **Impact**: the real-data gate can bless a regression of the decoder finding it was written to track.
- **Related**: #5326, #5446, PAR-D1-2026-10-12-02, PAR-D2-2026-10-12-02.
- **Suggested Fix**:
  - Use `assert_eq!` against the baseline, or better, pin the sorted set of rejected paths split by error variant.
  - Drop or correct the inert block clause.

## Completeness Checks
- [ ] **SIBLING**: Every other real-data census in the parser crates that pins a count is checked for a one-sided or inert assert
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
