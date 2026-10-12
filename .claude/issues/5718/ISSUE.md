# SAFE-D8-2026-10-12-01: `set_hostcall_fuel(max_log_message_bytes + 1024)` is justified by a false premise and pinned by no test

- **GitHub**: #5718 (https://github.com/matiaszanolli/ByroRedux/issues/5718)
- **Labels**: low,safety,test-gap,bug
- **Report**: docs/audits/AUDIT_SAFETY_2026-10-12.md

**Severity**: LOW. The error direction is fail-closed (the guest traps and is quarantined), so no memory-safety or isolation property is lost. The defect is that a documented invariant is wrong and unpinned, and that SDK-valid payloads can be refused.

## Location
`crates/mod-runtime/src/runtime/sandbox.rs:405-417` (added by `0781077b8`).

## Description
- The comment states `max_log_message_bytes` "is the largest guest→host payload any interface accepts (console lines cap at 4 KiB)". That is not true of the WIT surface in `crates/mod-runtime/wit/host.wit`.
- `set-result-string-array: func(value: list<string>)` is validated by the SDK to `MAX_SCRIPT_ARRAY_ELEMENTS` (512) strings of up to `MAX_SCRIPT_STRING_BYTES` (4 KiB) each (`crates/sdk/src/script_function.rs:21,23`), i.e. up to ~2 MiB.
- Wasmtime 49 charges `len × size_of::<T>()` for a list plus each string's byte length (`wasmtime-49.0.2/src/runtime/component/func/typed.rs:1530,1777`), and the budget is per host call. At the default `max_log_message_bytes` of 16 KiB the budget is 17,408 B, so a script function returning ~512 forty-byte names (~24 KiB charged) exceeds it although the SDK accepts it.
- `SandboxConfig::validate()` only rejects `max_log_message_bytes == 0` (`limits.rs:192`), so a config that lowers the log cap to 1 KiB drops the budget to 2 KiB, below the 4 KiB console-line and script-string caps.
- The commit added no test: `grep hostcall crates/mod-runtime/src` finds only the `sandbox.rs` lines. A later edit that drops or mis-orders the call, restoring the 128 MiB default the commit meant to remove, would pass every test.

## Evidence
```rust
// crates/mod-runtime/src/runtime/sandbox.rs:408-417
// ... max_log_message_bytes is the largest guest→host payload any
// interface accepts (console lines cap at 4 KiB); +1 KiB covers
// the rest of a call's argument-lifting overhead.
store.set_hostcall_fuel(self.config.max_log_message_bytes + 1024);
```

## Impact
A legitimate mod returning a large `list<string>` (or any combination whose lifted size exceeds ~17 KiB) hits `HostcallFuelExhausted`, traps, and is quarantined as faulted. The sandbox stays safe. Nothing else is affected.

## Suggested Fix
Derive the budget from the largest legitimate payload (the SDK's `MAX_SCRIPT_ARRAY_ELEMENTS × (MAX_SCRIPT_STRING_BYTES + 8)` or an explicit constant next to the other limits), floor it above the console and script-string caps in `validate()`, and add a test that a `string-array` result at the SDK cap succeeds while a 1 MiB `write-line` string is refused before allocation. Correct the comment.

**Related**: `0781077b8`, `9046b5ac5`. Label gap: the sandboxed mod runtime (`crates/mod-runtime`) has no domain label; filed with `safety` + `test-gap`.

## Completeness Checks
- [ ] **SIBLING**: The sibling wasmtime-48/49 fuel pins (`9046b5ac5`) already have tests; keep this one consistent with them
- [ ] **TESTS**: `string-array` result at the SDK cap succeeds; 1 MiB `write-line` is refused; dropping `set_hostcall_fuel` fails a test

Fix with: `/fix-issue 5718`
