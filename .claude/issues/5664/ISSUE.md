# #5664 — PAR-D6-2026-10-12-02: reused handles skip the #3637 shadow count, so an archive first opened in another pool never logs its override count for this pool

**Labels**: `low`, `bug`, `import-pipeline`, `game:oblivion`, `game:skyrim`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D6-2026-10-12-02) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: I/O & Paths
- **Location**:
  - `byroredux/src/asset_provider/archive.rs:699-712`: `log_opened_archive` returns early when `reused`.
  - `byroredux/src/boot/cli.rs:230-253`: `push_optional_archive_args` emits `--bsa P --textures-bsa P --sounds-bsa P`.
- **Status**: Regression of #3637 (diagnostic only; introduced by 5402b2530 / #5288).
- **Trigger Input**: every `--game oblivion` (GOTY) and `--game skyrim_se` (AE / Creation Club) boot.
- **Description**:
  - The early return assumes the count "was already reported… the answer for a repeat open in the same order is the same". That is true within one pool.
  - The registry is keyed by path, not by pool. The `--bsa P` open computes the count against the mesh chain, and the `--textures-bsa P` open is then a registry hit that logs "(shared handle)" and never computes the textures-chain count.
- **Evidence**: probe _probe-b/stale/src/bin/shadow.rs_ (`count_shadowed_entries` copied verbatim, real installs):

  | Archive | Mesh-pool count (logged) | Textures-pool count (now suppressed) |
  |---|---|---|
  | Oblivion `DLCShiveringIsles - Textures.bsa` | 0 | 5 |
  | Oblivion `DLCFrostcrag.bsa` | 3 | 3 |
  | Skyrim `ccBGSSSE037-Curios.bsa` | 3 | 3 |

- **Impact**: diagnostic only. Precedence is unchanged, because it follows push order and `every_content_provider_resolves_collisions_last_wins` still passes. The silent texture overrides #3637 exists to surface are no longer logged.
- **Related**: #3637, #3924, #2584.
- **Suggested Fix**: memoise the count on the shared handle per pool kind, or keep a static set of `(kind, path)` pairs already reported. Skip only on a repeat for the same pool.

## Completeness Checks
- [ ] **SIBLING**: Every pool that reaches `log_opened_archive` with a reused handle is covered: mesh, textures, materials, and HUD `open_with_numeric_siblings` chains
- [ ] **LOCK_ORDER**: If a lock scope changes (registry / `PREFETCH_GATE` / CDB cache maps), the leaf-lock discipline and the ABBA lock-order lane (#5535) stay green
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
