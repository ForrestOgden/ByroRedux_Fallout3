# Fix-issue batch 5648–5657 (scripting audit AUDIT_SCRIPTING_2026-10-11, HEAD 71b86091b)

All LOW severity. Domain: scripting — nearly all in the `byroredux` bin crate.

## 5648 — SCR-D3-2026-10-11-07 (doc-rot)
`condition.rs:22-50` vs `:245`. Module doc says "19 functions" (table of 19); `CATALOG` is 24.
Missing rows: 56, 71, 74, 77, 586. Related open #5610 covers only `HasPerk`.
Checks: SIBLING, TESTS.

## 5649 — SCR-D2-2026-10-11-01 (concurrency, Access rows)
`boot/schedule/late.rs:457-535`; `mod.rs:1059-1071`. `npc_dialogue_selection` Access row omits
what `evaluate_function` acquires, incl. the new `StoryManagerRng` W. The guard never scans
`condition.rs`. Exclusive row → nothing mis-scheduled today. Add `(CONDITION_SRC, "evaluate_function")`
and declare the types.
Checks: LOCK_ORDER, SIBLING, TESTS.

## 5650 — SCR-D4-2026-10-11-01 (test-gap, source pins)
`cell_loader/exterior.rs:1273-1300`; `asset_provider/script.rs:974-991`. Stub-spawner source pins
scan their own file via `include_str!` — the positive `contains` is satisfied by its own literal,
so deleting the production call passes. Negative count still works. Use `source_scan::production_text`.
Checks: SIBLING, TESTS.

## 5651 — SCR-D1-2026-10-11-08 (hybrid default table)
`player_control.rs:30-42`; `effects.rs:1283-1294`. `PlayerControlSelection::PAPYRUS_DEFAULT` is one
hybrid table. CK wiki's Enable and Disable defaults differ (camera/looking/sneaking, journal tabs).
Two constants. Only `.psc` bodies see the fill-in.
Checks: SIBLING, TESTS.

## 5652 — SCR-D1-2026-10-11-09 (dropped 3rd arg)
`effects.rs:970-982`. `SetObjectiveDisplayed` never inspects its 3rd argument (`abForce`), so a
non-literal or side-effecting expression is silently dropped. Use `bool_arg(args, 2)?`.
Checks: SIBLING, TESTS.

## 5653 — SCR-D1-2026-10-11-10 (never-matching arity)
`effects.rs:1207-1217`. `prim_lock` bounds at 1 argument, but compiled `Lock(abLock, abAsOwner)`
materializes 2, so it never fires on real content. Decline-safe (#3487 class).
Checks: SIBLING, TESTS.

## 5654 — SCR-D6-2026-10-11-03 (doc-rot)
`ROADMAP.md:687-688`, `:269`; `asset_provider/script.rs:573`; `update.rs:456`. ROADMAP says FO3/FNV
SCPTs are not executed. They are (322 FNV + 101 FO3 quest scripts), with ungated install and tick.
Command ids shared across the three games (agent census). Update ROADMAP + record census in
`docs/engine/scripting.md`.
Checks: SIBLING, TESTS.

## 5655 — SCR-D6-2026-10-11-04 (untrusted input bounds)
`obscript_vm.rs:591-614, 655-660`. Argument payload reads bounded by the buffer, not the argument
region. A straddling last argument reads the next statement's bytes instead of reporting `Malformed`.
Not memory-unsafe. Add `o + 3 > end` checks.
Checks: SIBLING, TESTS.

## 5656 — SCR-D7-2026-10-11-03 (lowering/validator mismatch)
`lower_program.rs:483-498, 363-372`; `execute.rs:294-297, 436-438`. `elseif` chains deeper than
`MAX_PROVIDER_HANDLER_NESTING` lower fine, but `validate_provider_statements` rejects them on every
dispatch with a warn — handler never runs. Count the elseif index into `depth` at lowering.
Checks: SIBLING, TESTS.

## 5657 — SCR-D7-2026-10-11-04 (unbounded Wait + drop-newest cap)
`lower_program.rs:662-690`; `execute.rs:363-368`. `Utility.Wait(3.0e38)` lowers, and the continuation
cap drops the newest, so one mod can park 4 096 immortal saved continuations and starve every other
latent handler. Cap `Wait` at lowering, or evict oldest-first.
Checks: SIBLING, TESTS.
