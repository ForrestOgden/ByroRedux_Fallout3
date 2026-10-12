# TD9-2026-10-12-01: The skeever isolation probe's run recipe has a typo, and the pin for closed #5161 is hand-run only

- **GitHub**: #5746 (https://github.com/matiaszanolli/ByroRedux/issues/5746)
- **Labels**: low,physics,test-gap,tech-debt,bug
- **Source**: docs/audits/AUDIT_TECH_DEBT_2026-10-12.md (`TD9-2026-10-12-01`)

**Audit finding** `TD9-2026-10-12-01` — severity LOW — from `docs/audits/AUDIT_TECH_DEBT_2026-10-12.md` (HEAD 5d31fd637, 2026-10-12).

**Location**: `byroredux/src/ragdoll_skeever_probe_tests.rs:21-24, 90`

## Description
The module doc extracts to `/tmp/skeever_skeleton.nif` and then runs `BYRO_SKEEVER_NIF=/tmp/skeever_skeleton.nfa`, a file that does not exist; the test's own default is `/tmp/byro-5161-probe3/skeever_skeleton.nif`. The probe `#[ignore]`s on "needs BYRO_SKEEVER_NIF" and, per its own doc, "now pins the containment" of an explosion that previously poisoned the broad phase, but it needs a hand-extracted NIF plus a hand-set env var, so no lane ever runs it (the sibling `ragdoll_installed_tests` do run in the default lane).

## Evidence
`grep -n 'nfa\|skeever_skeleton' byroredux/src/ragdoll_skeever_probe_tests.rs` (validated at publish time: line 23 still says `.nfa`).

## Impact
The only copy-pasteable command for the #5161 containment check fails with "missing: /tmp/skeever_skeleton.nfa".

## Suggested Fix
Correct `.nfa` to `.nif`; if the containment claim is meant to hold, read the NIF from the Skyrim archive the way the other real-data tests do (`BYROREDUX_SKYRIMLE_DATA` fallback) so the `--ignored` corpus run exercises it.

## Related
#5161, #5488, `docs/smoke-tests/README.md`.

## Completeness Checks
- [ ] **TESTS**: A regression test pins this specific fix
