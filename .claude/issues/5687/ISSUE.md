# #5687 — ESM-2026-10-12-D2-01: the `WTHS` `DIFF` value walker recurses with no depth bound, so a hostile `RDIF` blob aborts the process with a stack overflow

**Labels**: `high`, `esm-plugin`, `safety`, `bug`, `game:starfield`

**Source**: `docs/audits/AUDIT_ESM_2026-10-12.md` (ESM-D2-01) · audited at HEAD 5d31fd637

- **Severity**: HIGH (untrusted-input reader that can be made to abort; a stack overflow is not a catchable panic)
- **Dimension**: Sub-Record Byte Accounting
- **Record / Sub-record**: `WTHS` / `RDIF` (also `REFL`)
- **Location**: `crates/plugin/src/esm/records/weather_settings.rs` — `skip_diff_value` (recursion on the struct arm and on the `Ref` arm), `collect_diff_indices` (entry point)
- **Status**: NEW. Introduced by #5514 (`f0f8dceef`). #5643 is a different recursion (Story Manager tree build); #5627 is the unrelated `WTHS` value-reader tracker.

## Description
#5514 replaced the flat index read with a recursive value skip that mirrors `crates/sfmaterial/src/reader.rs`. Two arms recurse on file-controlled data:
- A non-user class field ("struct") opens a nested diff-pair stream and recurses into each field's own type.
- A `Ref` field reads a target type id and recurses into it.

The class table is decoded from the same blob (`CLAS` chunks with arbitrary `type_ref`s), so a class whose field 0 names the class itself is legal to the parser. Each nesting level consumes only 2 bytes (field index) or 4 bytes (a `Ref` target), so recursion depth is bounded by payload length, not by any cap. The sibling reader this code claims to mirror carries `MAX_NESTING_DEPTH = 64` (`sfmaterial/src/reader.rs`); the port dropped the guard.

## Evidence
Release build, 1.96 toolchain, the real `parse_bgs_reflection`. Blob: `STRT "C\0"`, one `CLAS` with `flags=0` and one field whose `type_ref` is the class's own name offset (0), and a `DIFF` payload of `type_ref = 0` followed by N repetitions of u16 `0x0000`.
- N=1,000 and N=30,000 on a 2 MiB thread: ok, `diff_field_indices = None`.
- **N=100,000** (200 KB payload) on a 2 MiB thread (the default stack of the rayon walk workers, #3813): `thread has overflowed its stack / fatal runtime error: stack overflow, aborting`.
- N=1,000,000 on an 8 MiB thread (the main thread's stack): the same abort.
- A sub-record payload over 64 KiB needs the `XXXX` extended-size override, which `read_sub_records` supports.

## Impact
Any Starfield plugin (or any plugin carrying a `WTHS` record) can kill the engine at load with an abort, before any cell loads. Vanilla data never trips it (163/163 real `RDIF`s decode), so this is a hostile-mod / corrupted-file hardening gap. Blast radius is one record kind, but the consequence is process death.

## Related
#5514 (introduced), #5364, #5627 (the value-reader follow-up will reuse these helpers and inherit the problem — this should land first), #5643, `MAX_GRUP_NESTING_DEPTH` (same class guarded elsewhere).

## Suggested Fix
Thread a `depth: usize` through `skip_diff_value` / `collect_diff_indices` and return `false` (already treated as "decline, leave `diff_field_indices` unset") past a cap equal to `sfmaterial`'s 64. Pin with a unit test that builds the self-referential class and asserts `diff_field_indices == None` without overflowing; a `Ref` chain test should cover the other recursion edge. (The cap constant lives in `sfmaterial`, which `byroredux-plugin` does not depend on, #5615 — share or duplicate with a comment.)

## Completeness Checks
- [ ] **SIBLING**: Every recursive value skipper in the reflection-container readers (`sfmaterial` reader, `weather_settings.rs` helpers, any `REFL` consumer) carries the same depth cap
- [ ] **TESTS**: A regression test pins both recursion edges (self-referential struct, `Ref` chain) and asserts no overflow
