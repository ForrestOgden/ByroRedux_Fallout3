# SCR-D5-2026-10-12-01: dialogue selection evaluates every INFO's conditions twice per open/click, so a `GetRandomPercent` (fn 77) gate can pass the menu filter and fail the selection

Labels: low,scripting,dialogue,bug
Issue: https://github.com/matiaszanolli/ByroRedux/issues/5682

Filed from `docs/audits/AUDIT_SCRIPTING_2026-10-12.md` (audit at HEAD 5d31fd637; finding re-validated against current code before filing).

- **Severity**: LOW
- **Dimension**: Scene/Package/Cinematic (dialogue selection)
- **Untrusted-Input**: No
- **Location**: `byroredux/src/systems/npc_dialogue.rs:191-203` (`top_level_menu`), `:239-247` (`select_on_topic`), `:314-336` (`open_conversation`: blocking probe then `select_on_topic`), `:768-805` (`select_topic_by_form_id` recomputes the whole menu, then selects)
- **Status**: NEW — latent since #5380 `395d2d831` catalogued fn 77 (before it, fn 77 evaluated as the fixed Unknown default 0.0, so the evaluation was deterministic)
- **Description**: `select_first_info(record, ..)` is the INFO gate, and it is called once to decide whether a topic belongs in the list/qualifies as Blocking, and again inside `select_on_topic` to pick the line. `select_first_info` evaluates the INFO CTDA set through the shared evaluator, where fn 77 draws from `StoryManagerRng` (installed by `install_story_manager`). A topic whose INFOs are all fn-77-gated passes the first evaluation with probability p and the second independently. When the second fails, `select_on_topic` returns `None` and `open_conversation` returns it directly (no fall-through to the generic greeting), so an NPC that listed a topic opens nothing; `select_topic_by_form_id` reports "no INFO … passes its conditions right now" for a topic the player just clicked in the list. The list shown after a line (`top_level_menu` fallback) also re-rolls on every click.
- **Evidence**: census of INFO CTDA function 77 (read-only script over the installed masters, GRUP walk, CTDA function at subrecord offset 8): Skyrim.esm 522 of 31 465 INFOs in 47 of 15 037 DIALs, 12 DIALs with *every* INFO gated (named ones: `DCETIdle`, `DCETTaunt`, `FreeformFrostRiverDeliverRepeat`, the rest EDID-less); FalloutNV.esm 9 INFOs in 3 DIALs, none all-gated. The all-gated Skyrim set is bark/freeform topics, not owned-quest menu topics.
- **Impact**: Intermittent "NPC opens nothing / clicked topic does nothing" on any plugin that gates a menu topic's INFOs on `GetRandomPercent`. Vanilla reach is near zero (the topics above are not Player-category menu entries), so LOW; the design hazard is that any other stochastic or stateful catalog function would behave the same.
- **Related**: #5645 (the pre-filter facet of the same stochastic-evaluator problem), #5380
- **Suggested Fix**: evaluate once per open — carry the chosen `&InfoRecord` from the filtering pass into the selection (`select_on_topic` takes the already-selected INFO), or draw the fn-77 value once per selection pass.

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files/sites
- [ ] **TESTS**: A regression test pins this specific fix

