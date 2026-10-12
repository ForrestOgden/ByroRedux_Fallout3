# #5672 — PAR-D2-2026-10-12-06: the CDB `Objects` table decode and the Parent-chain walk degrade silently (shape mismatch drops a row or an edge; a chain past `MAX_PARENT_DEPTH` or a non-start cycle ends without a warning)

**Labels**: `low`, `bug`, `import-pipeline`, `nifal`, `game:starfield`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-2026-10-12-06) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `crates/sfmaterial/src/index.rs`:
  - `:686-692`: `id_named(o, "DBID")` / `id_named(o, "Parent")` returning `None` silently skips the row or the edge.
  - `:743-751`: `id_named`.
  - `:378-387`: `for _ in 0..=MAX_PARENT_DEPTH`, with only `parent != obj` as a cycle check.
- **Status**: NEW. #5578 added the default-lane decode test but not a signal. The #5320 strictness covers `Components` only.
- **Trigger Input**: a non-vanilla CDB whose `ObjectInfo` declares `DBID` or `Parent` in a shape other than `{ Value: u32 }`; a chain deeper than 64; or a cycle that does not pass back through the start.
- **Description**:
  - Contract (c) requires a degraded decode to be `Ok` plus a warning, or an error. These cases are neither.
  - A Parent shape drift would silently disable all #5502 inheritance (IsGlass 217 → 4, alpha-test −585).
  - The walk is bounded, so it terminates, but cycles and truncation are not detected.
- **Evidence**: `cdbprobe` on vanilla:
  - 500,403 rows: DBID misses 0, Parent-shape misses 0, `Parent = 0` on 6 rows.
  - Maximum chain depth 24 (2.7× headroom under the 64 cap), 0 cycles.
  - Vanilla is clean. The finding is the missing signal on non-vanilla input.
- **Impact**: schema drift in a mod or DLC CDB degrades Starfield materials wholesale with only an info-level key count to show for it.
- **Related**: #5320, #5578, #5502, #5465.
- **Suggested Fix**:
  - Count `Objects` rows whose DBID or Parent fails `id_named`, and error or `warn!` once, mirroring `RowInstanceMismatch`.
  - Detect revisits and depth-cap hits in `collect_settings` and warn once per lookup.

## Completeness Checks
- [ ] **SIBLING**: The `Components` join and the `Objects` table use one degradation policy (typed error or a single warning)
- [ ] **CANONICAL-BOUNDARY**: The fix stays at the CDB → `ImportedMaterial` merge (Phase-2 resolver) feeding NIFAL `translate_material`; nothing is re-derived per game at render time (see `/audit-nifal`)
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
