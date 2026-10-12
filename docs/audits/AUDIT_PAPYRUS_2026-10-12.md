**HEAD**: 5d31fd637 · **Baseline**: [AUDIT_PAPYRUS_2026-10-08.md](AUDIT_PAPYRUS_2026-10-08.md) (HEAD `00f580e09`) · **Audited**: Dim 3 (Boolean/Control-Flow/Lower: the #5398 `lower.rs` delta, corpus re-measure, flag census, hostile-input timing), Dim 4 (`.psc` Lexer & Pratt Parser: the #5398, logos 0.16 and #5331 deltas, fuzz and depth stress) · **Unchanged since baseline (skimmed)**: Dim 1 (PEX Reader & Opcode Decode), Dim 2 (Decompiler CFG & Lift)

# Papyrus Frontends Audit: 2026-10-12

This pass was delta-scoped and run solo, as part of `/audit-suite --preset comprehensive`. Per-dimension
scratch notes are in `/tmp/audit/papyrus/dim_N.md`. Probes were built outside the repo in
`/tmp/audit/papyrus/probe` (own `target/` dir, path dependencies on the workspace crates; no repo file was
touched apart from this report).

## Delta since baseline: 3 commits

`git log 00f580e09..HEAD -- crates/pex crates/papyrus`:

| Commit | Change | Dim | Verdict |
|---|---|---|---|
| `ce90b481c` (10-09) | #5398: `ScriptFlags` gains `CONDITIONAL`/`BETA_ONLY`/`DEFAULT`, `GroupFlags::COLLAPSED`, `Variable.is_hidden`. `Default` and `Collapsed` are matched contextually as identifiers. `.pex` lowering maps the object's `conditional`/`hidden`/`default` user flags and a variable's `conditional` | 4, 3 | Parser side SOUND. `.pex` side is correct but **quadratic on hostile input** (finding 01) |
| `15a6b1d2d` (10-10) | logos 0.15 → 0.16: 48 keyword tokens move from `ignore(ascii_case)` to `ignore(case)` | 4 | SOUND for robustness (0 panics in 200k fuzz inputs). One accepted-input drift is noted below |
| `fd5aa4ead` (10-12) | #5331: `parse_new_expr` guards the type with `expect_same_line` and the `[` with `expect_raw` | 4 | SOUND. Both probes now error. The sibling `as`/`is` case (#5445) is untouched, as expected |

`git log` shows **no commits** on `reader/opcode/model/lib/call_sites.rs` or on `decompile/{cfg,lift,boolean,control_flow,node,event_names,mod}.rs`.

## Build, test and corpus state: CLEAN

```
cargo test -p byroredux-pex -p byroredux-papyrus
  papyrus 132 unit (+3: the #5398 tests) + 4 round-trip · pex 76 unit (+1: object_user_flags_lower_into_script_flags) + 1 doc-test · 0 failed
cargo test -p byroredux-scripting   562 passed, 0 failed (5 ignored need game data)
pex_corpus_smoke  Skyrim - Misc.bsa  Fallout4 - Misc.ba2  Starfield - Misc.ba2
  decompile → AST: 26,640 / 26,641, 0 panic
  only failure: stimboxscript.pex CheckTriggeringObjectAndDoOnceStatus (the known #1732 fail-closed case)
```

## Executive Summary

**3 new findings: 0 CRITICAL, 0 HIGH, 1 MEDIUM, 2 LOW. 7 findings are already tracked and were re-verified
as still present:** #5443, #5444, #5445, #5325, #5328, #4113, #4115. (#5331 and #5398 are closed and their
fixes are verified in place.)

- **PEX-D3-2026-10-12-01 (MEDIUM):** the #5398 lowering calls `has_user_flag`, which scans the whole
  user-flag table, once per script variable. Both counts are wire `u16`s, so the cost is O(flags x
  variables). A hand-built **852 KB `.pex`** parses in 5.6 ms and then spends **17.7 s** in
  `decompile_script`. Vanilla content is unaffected.
- **PEX-D4-2026-10-12-01 (LOW):** `docs/engine/papyrus-parser.md` and two code docs still describe the
  pre-#5398 flag sets.
- **PEX-D3-2026-10-12-02 (LOW):** the `.pex` lowering maps script and variable user flags but not
  property user flags. The two frontends therefore still disagree on `PropertyFlags`.

### Untrusted-input robustness verdict: MET for crash classes (no panic, OOB, OOM or stack overflow). One CPU-time hole (finding 01)

- **`.pex`:** no reader or decompiler source changed. All bounds and OOM guards named in the skill are
  present and passing. The compile-time assert (`opcode.rs:74`) and the `>=` gate (`opcode.rs:137-142`)
  are unchanged. All 26,641 real files parse.
- **`.pex` lowering (new):** the `1 << flag.flag_index` shift is guarded by `flag_index < 32`
  (`lower.rs:433`), so it cannot panic in debug or wrap in release. Finding 01 is a time bound, not a
  crash: it cannot abort the process. The cost is bounded by the `u16` wire counts, around 18 s at the
  ceiling.
- **`.psc`:** 200,000 randomized token-soup inputs, each through both `parse_script` and `parse_expr`
  (keywords, non-ASCII fold characters, NUL, CR/CRLF, `\` continuations, unterminated strings and
  doc comments): **0 panics**. Depth stress ran on a 1 MiB-stack thread with 23 shapes of 200,000
  repetitions, up to 5 MB of input: nested parens, `!`, `-`, `[0]`, `.b`, `()`, `as`, `is`, binary
  ops, `&&`, `New Int[`, `f(`, nested `If`/`While`/`State`, 200k `ElseIf`, 200k struct members, 200k
  functions, a 5 MB identifier/string/doc comment, and 200k `\` continuations. No stack overflow, no
  Drop recursion, every case under 0.3 s.

### Decompile-rate verdict: re-measured, exactly reproduced

| Game | Decompiled | vs 10-08 |
|---|---|---|
| Skyrim SE | 14,026 / 14,026 | unchanged |
| FO4 | 7,874 / 7,875 | unchanged (the #1732 `\|\|` fail-closed case) |
| Starfield | 4,740 / 4,740 | unchanged |
| **Total** | **26,640 / 26,641**, 0 panics | unchanged |

This shows the decompiler is **robust**, not **faithful**. Fidelity evidence is the `/audit-scripting`
Dim 1 `.psc`-vs-`.pex` gate (`r5_fidelity`), which needs the game data and was not re-run: no
decompiler pass changed, and the only lowering delta is flag-only (flags are consumed by no
recognizer).

### #5398 flag lowering: independent census check

I wrote a probe that decompiles every `.pex` and compares the lowered `ScriptFlags::CONDITIONAL` with
a raw re-read of the user-flag table (a separate implementation of the same rule):

| Game | scripts | `CONDITIONAL` | raw census | mismatches | `HIDDEN` | `DEFAULT` | conditional vars |
|---|---|---|---|---|---|---|---|
| Skyrim SE | 14,026 | 761 | 761 | 0 | 10,463 | 0 | 58 |
| FO4 | 7,874 | 346 | 346 | 0 | 6,203 | 143 | 42 |
| Starfield | 4,740 | 75 | 75 | 0 | 3,240 | 209 | 8 |

Total 1,182 conditional scripts, which matches the 10-08 census that motivated #5398. The lowering is
correct on real data. Note that the raw-census comparison shares the user-flag-table interpretation
with the code under test, as `/audit-papyrus` Dim 3 warns about the corpus rate. It validates the
mapping and the counts, not the Champollion bit semantics (`1 << flagIndex`), which match the
reference.

## Findings

Deduplication used three sources:
- `/tmp/audit/issues.json` (122 open issues)
- Closed and open searches for "has_user_flag", "user flag", "quadratic decompile" and "papyrus-parser.md"
- All `AUDIT_PAPYRUS_*` reports, plus `/tmp/audit/REPORTED_THIS_SUITE.md` (no overlap: no other audit in this suite touched `crates/pex` or `crates/papyrus`)

The nearest precedent is closed #3938 (`Pex::call_sites()` O(F·D) re-scan, MEDIUM `performance`), the
same bug class in a different module.

---

### MEDIUM

#### PEX-D3-2026-10-12-01: `has_user_flag` (#5398) rescans the whole user-flag table for every variable: a 852 KB hostile `.pex` stalls `decompile_script` for ~18 s

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

---

### LOW

#### PEX-D4-2026-10-12-01: `papyrus-parser.md` and two code docs still describe the pre-#5398 flag sets

- **Severity**: LOW
- **Dimension**: Papyrus Lexer & Parser
- **Untrusted-Input**: No
- **Location**: `docs/engine/papyrus-parser.md:78` (grammar line lists only `[Native] [Const] [DebugOnly]
  [Hidden]`), `:128` (`Variable` fields omit `is_hidden`), `:148` (`ScriptFlags` lists four flags;
  now seven), `:151` (`GroupFlags` omits `COLLAPSED`); `crates/papyrus/src/parser/script.rs:680`
  (`parse_struct` doc: "no flag annotations beyond what `Variable` carries", but a member may now be
  `Hidden`)
- **Status**: NEW
- **Description**: `ce90b481c` changed `ast.rs` and the parser but did not touch the reference doc.
  The doc's own flag table (the only place a reader looks up what the AST can express) now omits
  `CONDITIONAL`, `BETA_ONLY`, `DEFAULT`, `GroupFlags::COLLAPSED` and `Variable.is_hidden`, and the
  grammar summary rejects scripts the parser accepts. `docs/engine/papyrus-parser.md` was last
  touched in `3b7ffa670` (#5021).
- **Impact**: Documentation only.
- **Suggested Fix**: Update the four table rows and the grammar line, note that `Default` and
  `Collapsed` are matched contextually (not lexer keywords), and fix the `parse_struct` doc.

#### PEX-D3-2026-10-12-02: `.pex` lowering maps script and variable user flags but not property user flags, so the two frontends still disagree on `PropertyFlags`

- **Severity**: LOW
- **Dimension**: Decompiler Boolean/Control-Flow/Lower
- **Untrusted-Input**: Yes (it is `.pex` input, but only a silent omission)
- **Location**: `crates/pex/src/decompile/lower.rs:360-397` (`lower_property`: `flags` is only ever
  `AUTO` or `AUTO_READ_ONLY`; `prop.user_flags` is never read), against `crates/papyrus/src/ast.rs:108-117`
  (`PropertyFlags` also has `CONST`, `MANDATORY`, `HIDDEN`, `CONDITIONAL`, all of which the `.psc`
  frontend parses)
- **Status**: NEW (the property half of the gap #5398 closed for scripts and variables)
- **Description**: #5398's stated aim is "both frontends agree". It maps the object's `conditional`,
  `hidden` and `default` bits and a variable's `conditional` bit. `model::Property` carries
  `user_flags` just the same, but `lower_property` drops them. A property declared
  `Int Property P Auto Conditional Hidden Mandatory` lowers from `.psc` with `CONDITIONAL | HIDDEN |
  MANDATORY` and from `.pex` with `AUTO` only.
- **Evidence**: `.psc` probe: `Int Property P Auto Conditional Hidden` parses with those flags set.
  `lower_property` builds `flags` solely from `has_auto_var()` / readable / writable. A grep for
  consumers of `PropertyFlags` / `ScriptFlags::CONDITIONAL` finds none outside `crates/papyrus` and
  `crates/pex`, so nothing misbehaves today.
- **Impact**: None today. It would bite the first consumer that keys on a property's `Conditional` (the
  property half of what `GetVMQuestVariable`-style conditions read) or on `Mandatory`, and it makes
  `.psc`-vs-`.pex` AST equality checks impossible for any script with flagged properties. Also note the
  `Variable.is_hidden` flag is never set from `.pex` (struct definitions are not lowered at all).
- **Related**: #5398, #5443 (the other cross-frontend divergence, `is` lowering to `Cast`)
- **Suggested Fix**: Resolve the `hidden`/`conditional`/`mandatory`/`const` bits through the same
  per-`Pex` mask table proposed for finding 01 and OR them into `lower_property`'s `flags`.

---

## Already tracked: re-verified this pass, not re-filed

| Issue | Severity | State this pass |
|---|---|---|
| #5443 | LOW | Still present: `lower.rs:125` lowers `op == "is"` to `Expr::Cast` while `.psc` produces `Expr::Is` |
| #5444 | LOW | Still present. `ScriptName T` / `Struct S` / `Int a = 1 EndStruct` / `Function F()`... gives 1 item and 4 errors; `F` is lost |
| #5445 | LOW | Still present: `x as` ⏎ `Actor` and `x is` ⏎ `Actor` give 0 errors. #5331's fix covers only `New`; the shared `parse_base_type` `skip_newlines` root is untouched |
| #5325 | MEDIUM | Still present: `Function F()` ⏎ `{doc}` ⏎ `Int x = 1` gives 2 errors (`F` dropped). Same root for struct members: `Quest Q` ⏎ `{doc of Q}` inside a `Struct` gives an error per doc comment, although the CK *Struct Reference* explicitly allows doc strings after members |
| #5328 | MEDIUM | Still present: `ScriptName T extends DLC01:Base` gives an error at `:` and `items=0` |
| #4113 | MEDIUM | Still present: `MAX_EXPR_DEPTH` 256 (`lift.rs:380`) and `MAX_REBUILD_DEPTH` 1024 (`control_flow.rs:44`) remain independent constants |
| #4115 | LOW | Still present: `control_flow.rs:28` still says "advanced past" |
| #5331 | (closed) | Fix verified: `Int[] a = New Int[5]` parses; `New` ⏎ `Int[5]` and `New Int` ⏎ `[5]` both error |
| #5398 | (closed) | Parser fix verified (probes below); `.pex` lowering verified against the census; finding 01 is a new cost in the same change |

## Verification of the three deltas

**#5398 parser probes** (`probe` binary, HEAD):

| Input | Result |
|---|---|
| `ScriptName DialogueFollowerScript extends Quest Conditional` | `CONDITIONAL`, 0 errors |
| `… Const Hidden Conditional`, `BetaOnly`, `default` (any case) | each accepted, 0 errors |
| `Int Default = 1` ⏎ `Default = Default + 1` (inside a function), `Int Collapsed` | stays an identifier everywhere outside the flag position, 0 errors |
| `Group G Collapsed`, struct member `Float b Hidden` | accepted; `is_hidden = [false, true]` |
| `ScriptName T extends Quest` ⏎ `Hidden` ⏎ `Int Function F()…` | the next-line `Hidden` is NOT taken as a script flag (raw peek); one error, `F` kept |
| `Float b Hidden = 2.0` | error (flags follow the initializer in Papyrus; correct) |

The struct-member `Hidden` loop uses `peek_raw`, so a flag on the next line is not taken. Papyrus also
forbids `Const` on struct members (CK *Struct Reference*); `parse_variable_body` still accepts it. That
is a lenient accept, in the same family as the existing "accepts source the compiler rejects" findings,
and not worth its own entry.

**logos 0.16 `ignore(case)`**: as the commit message says, Unicode case folding now lets `ſtate` and
`ſcriptname` (U+017F) lex as `KwState`/`KwScriptName`, where 0.15 produced a lex error. The Kelvin sign
(U+212A) and `İ` (U+0130) behave as expected (`İf` is a lex error; `Kelvin`, `Konditional`, `Iffy` and
`EndIf2` stay identifiers). This is a silent-accept of invalid Papyrus, which is harmless: no panic, spans
stay on char boundaries (0 panics across 200k mixed inputs including these characters), and it grants
nothing a plain ASCII spelling would not. Not filed.

**#5331**: `Int[] a = New Int[5]` parses; `New` ⏎ `Int[5]` fails with "type after 'New'"; both probes are pinned
in `a_mandatory_operand_on_the_next_line_is_an_error`. As with the other #5021-family fixes, an
erroring `New` inside a function body drops the whole function through recovery (3 errors, `items=0` for the
probe). That is the existing recovery behaviour for any malformed statement, not a regression.

## Decompiler Soundness Matrix

Only the rows that changed this window carry new evidence. The full matrix is in AUDIT_PAPYRUS_2026-09-19.md.

| Pass | Bounds-safe | Terminates | Total | Fidelity-tested | Change this window |
|---|---|---|---|---|---|
| Reader | Yes | Yes | Yes | 26,641 parse | none |
| CFG | Yes | Yes | Yes | JmpF/JmpT pins | none |
| Lift + copy-prop | Yes | Yes | Yes (`lift.rs:441-455` fails closed) | Yes | none |
| Boolean | Yes | Yes (`RecursionLimit`) | Yes | corpus + gate | none |
| Control-flow | Yes | Yes | Yes (fails closed, #1732; #4115 open) | corpus + gate | none |
| Lower + assembly | Yes (`flag_index < 32`) | **Linear except `has_user_flag` (finding 01)** | Yes | census (1,182/1,182) | #5398 |
| `.psc` lexer | Yes | Yes | Yes | CR/CRLF pinned; logos 0.16 fuzzed | logos 0.16 |
| `.psc` parser | Yes (depth caps, 23-shape stress) | Yes (200k fuzz) | Yes | Partial: #5325, #5328, #5444, #5445 | #5398, #5331 |

**The three Champollion departures.** The two in `boolean.rs` (no debug-line guard, and the termination
guard) are unchanged: those files have no commits and the corpus rate reproduced exactly, so the 09-19
"benign" ruling stands. The third, `is` to `Cast` in `lower.rs`, is unchanged and tracked as #5443.

## Coverage notes

- **Probes:** `probe` (`.psc` cases), `fuzz` (200k random token soups), `deep` (depth stress), `lex`
  (keyword folding), `flags` (corpus flag census with an independent cross-check) and `hostile` (crafted
  `.pex` timing). All live in `/tmp/audit/papyrus/probe` with a private cargo target dir
  (`/tmp/audit/papyrus/probe/target`).
- **`.psc` corpus:** the only one on disk is still the 29 F4SE sources plus the 4 `docs/r5` fixtures.
  No vanilla `Scripts.zip` source dump is installed.
- **Skill-text rot, for `/audit-sync`:** `audit-papyrus` SKILL.md Dim 2 still says `replace_constant_id`'s
  `debug_assert!` is debug-only, but #2666 made it a hard `ExpressionRebuildFailed` (`lift.rs:441-455`;
  noted on 10-05 and 10-08, still unfixed). Its Dim 3 should also mention `has_user_flag` once finding 01
  is fixed.

## Cross-audit routing

- `/audit-scripting`: `ScriptFlags::CONDITIONAL`, `Variable.is_conditional` and `PropertyFlags::CONDITIONAL`
  are now populated by both frontends, but no consumer reads them. #5398's rationale was that "the
  condition system's `GetVMQuestVariable` family requires" the flag. Whether that condition evaluator
  should gate on it is a recognizer/runtime question for that audit.
- `/audit-scripting` also holds the 10-08 note about `papyrus_provider/lower_program.rs`
  (`expression_mentions_provider` has a `_ => false` wildcard that would miss a `.psc`-sourced
  `Expr::Is`), which becomes live when #5443 lands.

## Summary

| Severity | NEW | Already tracked |
|---|---|---|
| CRITICAL | 0 | 0 |
| HIGH | 0 | 0 |
| MEDIUM | 1 | 3 (#5325, #5328, #4113) |
| LOW | 2 | 4 (#5443, #5444, #5445, #4115) |

Next step: `/audit-publish docs/audits/AUDIT_PAPYRUS_2026-10-12.md` (domain labels `scripting`, plus
`performance` for finding 01).
