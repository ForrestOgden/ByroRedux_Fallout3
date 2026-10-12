# #5665 — PAR-D2-2026-10-12-03: the single-flight CDB slot does not memoise a *panicking* build, and the `sf-cdb-index` worker has no `catch_unwind`

**Labels**: `low`, `bug`, `import-pipeline`, `nifal`, `safety`, `game:starfield`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D2-2026-10-12-03) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: Error Semantics
- **Location**: `byroredux/src/asset_provider/material/cdb.rs`:
  - `:70-75`: slot doc ("`Some(None)` memoises a failed build").
  - `:136-141`: `cdb_material_index`.
  - `:144-195`: `build_cdb_material_index`.
  - `:202-223`: `prefetch_cdb_material_index`.
- **Status**: NEW. This is defence in depth: no current input is known to make `MaterialIndex::build` panic (the spot-check of `crates/sfmaterial/src/{index,reader}.rs` found none).
- **Trigger Input**: a mod or Creation `materialsbeta.cdb` that makes `MaterialIndex::build` panic, for example after a future reader regression.
- **Description**:
  - `OnceLock::get_or_init` stores only a returned value. A panic leaves the cell empty, and every waiter and later caller re-runs the closure.
  - So the worker panics, and the parked main-thread lookup re-runs the ~1.6 s, ~247 MB build on the main thread and panics there.
  - The `merge_external_materials` callers have no `catch_unwind`.
  - The sibling worker `byroredux/src/asset_provider/texture_prefetch.rs:224-229` already wraps its read in `catch_unwind`, for exactly this reason.
- **Evidence**: probe _probe-b/oncelock_, same slot type and gate:
  ```
  thread 'sf-cdb-index' panicked … build #1
  [main] build #2 starts … thread 'main' panicked … build #2
  slot initialised after panics: false
  total builds run: 3 (a memoised failure would be 1)
  ```
- **Impact**: a hypothetical panicking CDB crashes the engine on the main thread at the first `.mat` lookup, after paying the build twice.
- **Related**: #5291, #5319, #2614, `byroredux/src/asset_provider/texture_prefetch.rs`.
- **Suggested Fix**:
  - Wrap the extract and `MaterialIndex::build` in `catch_unwind(AssertUnwindSafe(..))`. Map a panic to a `warn!` and `None`, so the slot memoises it.
  - Add an injected-panic test that asserts exactly one build.

## Completeness Checks
- [ ] **SIBLING**: Every other background worker in `byroredux/src/asset_provider/` publishes through a slot and survives a panic (`texture_prefetch.rs` already does)
- [ ] **LOCK_ORDER**: If a lock scope changes (registry / `PREFETCH_GATE` / CDB cache maps), the leaf-lock discipline and the ABBA lock-order lane (#5535) stay green
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
