# PEX-D3-2026-10-12-01: `has_user_flag` (#5398) rescans the whole user-flag table for every variable: a 852 KB hostile `.pex` stalls `decompile_script` for ~18 s

Labels: medium,scripting,performance,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5683

Filed from `docs/audits/AUDIT_PAPYRUS_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: MEDIUM
- **Dimension**: Decompiler Boolean/Control-Flow/Lower
- **Untrusted-Input**: Yes
- **Location**: `crates/pex/src/decompile/lower.rs:430-436` (`has_user_flag`), called per variable at
  `lower.rs:452` and per flag name at `lower.rs:530`
- **Status**: NEW (introduced by `ce90b481c`, #5398)
- **Description**: `has_user_flag(pex, user_flags, name)` does `pex.user_flags.iter().any(...)` with a
  case-insensitive name compare. `decompile_script` calls it once for every entry of
  `object.variables`. Both tables are bounded only by their wire `u16` counts (65,535 each). The parse
  budget does not help: each `UserFlag` name is one 2-byte string index, charged by length, and
  a short name x 65,535 is far below the 16 MiB budget. Cost is therefore O(F x V), with no early exit
  for an attacker who names every flag `"conditionaX"` (same length as `conditional`, so the compare
  runs to the last byte) or gives it `flag_index >= 32` (the name matches but the bit test is skipped).
  The reader and every other lowering pass are linear; this is the only quadratic site in `lower.rs`
  (I grepped it for `iter().find/any/position/contains`).
- **Evidence**: A hand-built FO4-format `.pex` (scratch probe `hostile`, release build, one object with
  N user flags and N variables):

  | flags = vars | file size | `parse` | `decompile_script` |
  |---|---|---|---|
  | 100 | 1.4 KB | 22 us | 53 us |
  | 8,000 | 104 KB | 0.9 ms | 0.26 s |
  | 16,000 | 208 KB | 1.4 ms | 1.12 s |
  | 65,535 | **852 KB** | 5.6 ms | **17.7 s** |

  Quadratic growth (2x input = 4.3x time). `decompile_script` is reached from
  `crates/scripting/src/translate/mod.rs:162` (inside `decompile_catching_panics`, which cannot
  interrupt a CPU loop) and `crates/scripting/src/fragment/populate.rs:190,405,551`, i.e. on any
  `.pex` pulled from a game or mod archive when a script is attached.
- **Impact**: A mod-supplied script stalls its loader thread for tens of seconds with no crash, no
  allocation spike and no error, so no existing net catches it. Vanilla content is unaffected (real
  tables hold 2-6 flags). Not a memory or stack problem, hence MEDIUM rather than HIGH.
- **Related**: closed #3938 (the same O(n·m) class in `call_sites`), #5398 (the introducing change)
- **Suggested Fix**: Resolve the name-to-bit mapping once per `Pex`: one pass over `pex.user_flags` to
  build `{conditional, hidden, default}` bit masks (ignoring `flag_index >= 32`), then test
  `user_flags & mask` per variable and per object. That makes it O(F + V). While there, add a test
  with a `flag_index` of 32 or more and a hostile-size table, since the `< 32` shift guard on
  untrusted bytes currently has no pin.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix

