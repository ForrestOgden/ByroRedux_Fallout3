# TD4-2026-10-11-02: 12 skills carry "known-open / open #N / cite, don't re-file" callouts for issues that are now CLOSED, and some describe code defects that are fixed

URL: https://github.com/matiaszanolli/ByroRedux/issues/5606
Labels: low, tech-debt, documentation, doc-rot
Source: docs/audits/AUDIT_TECH_DEBT_2026-10-11.md

- **Severity**: LOW
- **Dimension**: 4, Audit-Finding Rot ("Existing:/open issue" callouts)
- **Location**: only the issue refs that are CLOSED yet presented as open are listed. Each site's other refs were checked (the OPEN ones such as #5505, #5523, #5520, #5073, #5351, #5526, #5439, #5011, #5459, #5285, #5534 are correct).
  - `audit-ecs:116`: #5513 ("Known-open … no visited set"). #5513 was closed 10-11 04:20Z (`9da80470d`).
  - `audit-ecs:256` and `audit-save:286`: #5490 ("since #5384 that list holds roots only, so the bare `despawn_batch` orphans their subtrees"). #5490 was closed 10-11 19:35Z (`99cd9fd6c`).
  - `audit-esm:262`: #5515 ("per-quest FNAM/UNAM … is open"), #5514 ("DIFF mis-decode open"). Both closed 10-11 (`0edd1dede`, `f0f8dceef`).
  - `audit-exterior:33`: #5519 (open doc rot), #5496 (LOD rings ignore PNAM 0x02), #5516. Also `:34` #5517 and `:84` #5518. All closed (`65cadde2f`, `2ca57c4bc`, `5a549e664`).
  - `audit-fnv:61` #5492, `:62` #5494, `:100` #5499, `:102` #5500 and #5495. All closed 10-10/10-11 (`dae08000d`, `5356c8780`, `f7ca4a60f`, `bcaf63aa8`, `0ba1dd3af`).
  - `audit-fo3:29`: #5428, #5429, #5433, #5521, #5494, #5499, #5496, #5518. All closed; the open ones in that list, #5520 and #5073, are right.
  - `audit-fo4:89`: #5493 and #5498 (#5523 is open, correct).
  - `audit-oblivion:28`: #5504, #5350, #5516, #5335, #5519 (the open ones #5505, #5351, #5526, #5439, #5011 and #5073 are correct).
  - `audit-performance:28` #5451 ("open"), `:81` #5447/#5448/#5449 ("Known open, do not re-file"), `:105` #5291 and #5528 ("open"), `:129` "#5249's transmission shadow trace … is unmeasured … (#5401)".
    - #5401 was closed 10-11 16:28Z with a comment "Measured, not material", so the "unmeasured" claim is false.
    - #5451/#5291/#5528 were closed 10-11 (`4b9472909`, `dd759b762`, `e588e5e67`).
  - `audit-renderer:133`: #5528 ("open"). `:214`: #5506 and #5532 ("Known open: the toe still crosses zero …; engine docs still show the pre-#5482 order"). Both closed 10-11 15:23Z (`38f711a93`, `b27e52c71`).
  - `audit-save:111`: #5458 ("Known-open (cite, don't re-file)"), closed 10-11 19:23Z (`0aa844dbb`).
  - `audit-gameplay:17` and `audit-audio:19`: #5432 ("nothing stops a voice line on close / topic change / death"; "no … path calls `stop_sounds_for`"). Fixed by `40b77828b`, and `dialogue_voice.rs:332` now calls `stop_sounds_for`.
- **Status**: NEW (the prior report's TD4-01 was a different class). The `d84e7f163` re-sync claims "verified 2026-10-10" for lists that the 10-10 and 10-11 fix commits then invalidated.
- **Age**: closures dated 2026-10-10 to 10-11 (commits listed above).
- **Effort**: small
- **Description**: These callouts tell an auditor to "cite, don't re-file" a gap. `_audit-common.md` § Deduplication step 3 says a CLOSED hit means "verify the fix is in place; if regressed, Regression of #N". A reader who trusts the callout would skip a regression check, or cite a closed issue as the open owner.
- **Evidence**: a script over `.claude/commands/**/*.md`:
  - It reads the state of every `#NNN` from `gh issue list --state all --limit 6000` (5,467 issues).
  - It looks at segments that start at `known-open | still open | open: | Existing` and end at "Fixed since" or end-of-line.
  - It lists CLOSED numbers in the "open" part (29 raw hits). I then hand-triaged the hits: those that really claim "open" came to 34 distinct issues across the 12 skills above, and the rest are historical or "closed" mentions.
  - Spot-checks: `gh issue view 5490/5513/5432/5401` all CLOSED, each with a matching `Fix #N` commit in `git log --grep`.
- **Impact**: audit time and dedup quality. No wrong issue has been filed from it yet. It is a mid-wave snapshot problem: the re-sync is hours old and already behind.
- **Related**: TD4-2026-10-11-01; #5579; `_audit-common.md` line 51 ("a rotting fact is checkable and correctable").
- **Suggested Fix**:
  - Move each closed ref from "Known-open" to the sibling "Fixed since 2026-10-08 — verify, don't re-file" list. `audit-fo3:29` and `audit-oblivion:28` already have that list.
  - Rewrite the code-defect clauses to their post-fix wording (`audit-ecs:116`, `audit-ecs:256`, `audit-save:286`, `audit-gameplay:17`, `audit-audio:19`, `audit-performance:129`).
  - Make `audit-sync/prep.sh` print every `#N` in a "known-open | open" segment whose state is CLOSED (the script above is the prototype). `audit-sync` then has a mechanical close-out check instead of relying on a hand read.
