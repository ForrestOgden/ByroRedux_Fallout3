# #5732 — UI-D1-2026-10-12-01: 86f5a1a0a shipped the callback-registry check and the empty-`ImportAssets` skip without a test, and `Drop` still bypasses the registry

**Labels**: `low`, `ui`, `test-gap`, `bug`

**Source**: `docs/audits/AUDIT_UI_2026-10-12.md` (UI-D1-01) · audited at HEAD 5d31fd637

- **Severity**: LOW (test gap; the code itself is correct on reading)
- **Dimension**: Profile & Bridge (also Resource Navigator)
- **Profile**: both Scaleform profiles
- **Location**:
  - `crates/ui/src/player.rs` `invoke_callback`
  - `crates/ui/src/host.rs` `prune_callback`
  - `crates/ui/src/navigator.rs` (empty-URL `continue` in the `ImportAssets` scan)
  - `crates/ui/src/player.rs` `Drop for SwfPlayer`
- **Status**: NEW

## Description
- 86f5a1a0a changed three files (`host.rs`, `navigator.rs`, `player.rs`) and added no test.
- `invoke_callback` now asks Ruffle's own registry (`context.external_interface.get_callback`) before invoking. It prunes a stale name from the bridge's set and returns `None`, so an AVM2 `addCallback(name, null)` unregistration no longer produces a `Null` that looks like a live callback's answer.
- Nothing exercises that path. `grep` for `prune` or an unregistration test finds none in `crates/ui/src/{player,host,host/tests}.rs`.
- Nothing exercises the empty-`ImportAssets` skip either. `navigator.rs` has no test with an empty URL.
- `Drop for SwfPlayer` still calls `player.call_internal_interface(DESTROY_CALLBACK, [])` after checking only the bridge's name set. It does not use the registry check. If a movie unregisters its destroy hook, the drop path reaches Ruffle's "Calling unknown internal interface" warn. This is inert: the return value is discarded.

## Evidence
`player.rs` `Drop` (`has_callback` then `call_internal_interface`) against `invoke_callback` (registry check, prune, `return None`).

## Impact
No functional effect today. A refactor of the lock scope (the registry check and the invoke now share one `MutexGuard` held across `mutate_with_update_context`) or a Ruffle API change would regress silently.

## Related
86f5a1a0a. The AVM2 fixtures in `crates/ui/testdata/` (Ruffle's `external_interface` SWFs) could host a test.

## Suggested Fix
- Add a host-bridge test that registers a callback in a real AVM2 movie, unregisters it with `addCallback(name, null)`, and asserts `invoke_callback` returns `None`, that `has_callback` is false afterwards, and that no `Null` leaks.
- Add an `import_asset_paths_from_tags` test with an empty `ImportAssets` URL that asserts neither a path nor an error is produced.
- Optionally route the `Drop` hook through the same registry check.

## Completeness Checks
- [ ] **SIBLING**: Both Scaleform profiles (AVM1 and AVM2) covered where the behaviour differs
- [ ] **TESTS**: The unregistration and empty-`ImportAssets` tests above exist
