# #5668 — PAR-D2-2026-10-12-04: once an include budget is spent, every further distinct include path still logs its own warning, and the byte-denominated fetch budget admits about 131k fetches of 2-byte fragments

**Labels**: `low`, `bug`, `import-pipeline`, `ui`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-2026-10-12-04) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `crates/menuxml/src/parse.rs`:
  - `:675-705`: `warn_once`, keyed `"{cause}:{path}"`.
  - `:788-805`: the budget, bytes and fetched early returns.
  - `:845`: `bytes_fetched += bytes.len()`, with no per-fetch floor.
- **Status**: NEW. The "fetched" cause comes from 6a132a967 (#5399); "budget" and "bytes" predate it.
- **Trigger Input**: a menu XML whose includes exhaust a budget and then name K further distinct prefabs. Variant: after splicing to 262,143 B, K distinct 2-byte fragments.
- **Description**:
  - The "exhausted" notices are per path, not per document.
  - The fetch budget counts bytes, so tiny refused fragments each still cost one archive extract.
  - Both effects are linear in document size: a log flood and a main-thread stall, not an amplification.
- **Evidence**: probe `includes`, real `parse_document`, release:

  | Case | Root | Probes | Fetched | Warn lines |
  |---|---|---|---|---|
  | 100k distinct missing includes | 2.7 MB | 300,000 | 0 B | 100,000 |
  | Splice to the cap, then 200k distinct 2-byte refusals | 5.5 MB | 131,077 | 524,289 B | 200,000 |

- **Impact**: a 5.5 MB mod menu XML emits 200,000 warn lines and up to 131k main-thread archive extracts at `--hud` load. Memory stays bounded.
- **Related**: #5399, #5314, #5007.
- **Suggested Fix**:
  - Warn once per document for the exhausted-budget causes.
  - Charge each fetch a floor, for example `max(len, 1 KiB)`, so the fetch budget also bounds the fetch count.

## Completeness Checks
- [ ] **SIBLING**: Every exhausted-budget cause ("budget", "bytes", "fetched", "tiles") follows the same once-per-document rule
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
