# #5666 — PAR-D5-2026-10-12-02: the prefetch worker takes `PREFETCH_GATE` before creating its slot, so a lookup for a still-queued CDB builds on the main thread, ungated, alongside the gated worker

**Labels**: `low`, `bug`, `import-pipeline`, `performance`, `game:starfield`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D5-2026-10-12-02) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Decode-Consumer Wiring
- **Location**:
  - `byroredux/src/asset_provider/material/cdb.rs:202-223`: the `contains_key` check, the spawn, `PREFETCH_GATE.lock()`, and only then the slot creation inside `cdb_material_index`.
  - `:229-241`: `lookup_cdb_material` walks sources last-registered-first.
- **Status**: NEW (dd759b762, #5291).
- **Trigger Input**:
  - Two or more valid CDBs, with the first `.mat` lookup arriving before the serial prefetch queue drains.
  - Not reachable with the shipped `starfield` profile, which has one CDB. It needs DLC or Creation `* - Main.ba2` archives named manually.
- **Description**:
  - The doc says a mid-build lookup "parks on that build" and that N CDBs "must not peak at N× that at once".
  - A queued worker has no slot yet, so the lookup builds that CDB itself on the main thread, outside the gate and concurrently with the gated worker.
  - When the queued worker later takes the gate, it parks on the main-thread build while still holding the gate, which stalls the queue behind it.
  - There is no deadlock: the main thread never takes the gate, and the map locks are leaves.
- **Evidence**: probe _probe-b/oncelock/src/bin/queue.rs_:
  ```
  build base on thread 'sf-cdb-index[base]' (concurrent builds now 1)
  build dlc on thread 'main' (concurrent builds now 2)
  main-thread lookup walk blocked 400.069539ms; peak concurrent builds = 2
  ```
- **Impact**: performance and doc accuracy only. On multi-CDB runs the ~1.6 s main-thread build that #5291 removed returns inside a streamed apply slice, with a doubled memory peak.
- **Related**: #5291, PAR-D2-2026-10-12-03.
- **Suggested Fix**:
  - Create the slot synchronously in `prefetch_cdb_material_index` and take the gate *inside* the `get_or_init` closure, so a lookup always parks on an owned slot. Alternatively, have lookups take the same gate.
  - Correct the doc to match.

## Completeness Checks
- [ ] **SIBLING**: Any other gate-then-slot prefetch in `byroredux/src/asset_provider/` is checked for the same ordering
- [ ] **LOCK_ORDER**: If a lock scope changes (registry / `PREFETCH_GATE` / CDB cache maps), the leaf-lock discipline and the ABBA lock-order lane (#5535) stay green
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
