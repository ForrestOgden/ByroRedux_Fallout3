# #5663 — PAR-D6-2026-10-12-01: `open_shared`'s (length, mtime) stamp misses a same-length, mtime-preserving in-place rewrite, so a reused handle returns another entry's bytes as `Ok`

**Labels**: `low`, `bug`, `import-pipeline`

**Source**: `docs/audits/AUDIT_PARSERS_2026-10-12.md` (PAR-D6-2026-10-12-01) · audited at HEAD c1a99c25e

- **Severity**: LOW
- **Dimension**: I/O & Paths
- **Location**:
  - `byroredux/src/asset_provider/archive.rs:111-160`: `open_shared` and `ArchiveStamp { len, modified }`.
  - Test: `archive.rs:925-950`, which rewrites with a *longer* payload only.
  - Path-keyed CDB caches that are never invalidated: `byroredux/src/asset_provider/material/cdb.rs` (`sf_cdb_cache`, `sf_cdb_index_cache`).
- **Status**: NEW (5402b2530, #5288).
- **Trigger Input**: an opened BSA or BA2 is rewritten in place mid-session to a different layout of the same byte length, and either:
  - the writer restores the mtime (`cp -p`, `rsync -t --inplace`, `touch -r`, Windows `CopyFile`), or
  - `Metadata::modified()` fails, so the stamp becomes `(len, None)`.
- **Description**:
  - The cached reader keeps its fd and its parsed table. After an in-place rewrite, the fd sees the new bytes at the old offsets. The length is unchanged, so every read stays in bounds.
  - Neither format carries a checksum, so a stored entry returns `Ok` holding another entry's bytes. A compressed entry usually fails to inflate and falls through with a warning.
  - After a rename replacement, the fd still reads the old inode: the content is stale but self-consistent.
  - The skill contract says "Bounds checks must fail on a replaced archive, not return garbage". The registry now holds tables across rebuilds; before #5288 each rebuild re-parsed.
- **Evidence**: probe _probe-b/stale_ (`open_shared` copied verbatim, real `Ba2Archive`; ext4 and tmpfs gave the same result):
  ```
  S1 in-place rewrite, mtime preserved: ctime changed=true inode changed=false
    second open_shared reused=true
    stale handle extract a (want 'aaaaaaaa'): Ok("bbbbbbbb")
  S3 rename replacement, mtime preserved: reused=true → Ok("AAAAAAAA") (old inode)
  S4 same-tick same-length rewrite without mtime restore: detected 2000/2000 (multigrain timestamps)
  ```
- **Impact**: the trigger is unlikely, since an archive has to be replaced mid-session by a same-size file with the same mtime. If it happens, every later rebuild (door cover, destination apply, save-load) serves wrong bytes for stored entries.
- **Related**: #5288, #3637, #4658.
- **Suggested Fix**:
  - Add `dev`, `ino` and `ctime`/`ctime_nsec` (Unix) or the file index (Windows) to `ArchiveStamp`. `utimensat` cannot set ctime back.
  - Add a same-length, mtime-restored rewrite test.
  - Key the CDB caches on the stamp, or document that they live for the whole process.

## Completeness Checks
- [ ] **SIBLING**: The path-keyed CDB caches in `byroredux/src/asset_provider/material/cdb.rs` follow the same invalidation rule as the registry
- [ ] **LOCK_ORDER**: If a lock scope changes (registry / `PREFETCH_GATE` / CDB cache maps), the leaf-lock discipline and the ABBA lock-order lane (#5535) stay green
- [ ] **TESTS**: A regression test pins this specific fix in the default (non-`--ignored`) lane where a synthetic fixture can express it
