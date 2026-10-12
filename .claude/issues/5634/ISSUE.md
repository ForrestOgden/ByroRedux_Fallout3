# SCR-D1-2026-10-11-06: `quest_via` maps `<any receiver>.GetOwningQuest()` to `QuestRef::OwningQuest`

Labels: high,scripting,bug

- **Severity**: HIGH (binding resolves to the wrong quest) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/translate/compose.rs:121-124` (`method_call` returns the receiver but `quest_via` ignores it); consumers `effects.rs:1702-1713`, `quest_stage_gate.rs:79-80`, `fragment/effects.rs:17`
- **Status**: NEW
- **Description**: the doc says `Self.GetOwningQuest()`, but any receiver is accepted. `OtherAlias.GetOwningQuest().SetStage(10)`, where `OtherAlias` is another quest's alias property, lowers to `SetStage` on the *context* quest. Guard atoms `X.GetOwningQuest().GetStageDone(n)` classify the same way, so mixed-quest guards pass the same-quest check.
- **Impact**: stage written to or read from the wrong quest.
- **Related**: #1905 (same family, different hole).
- **Suggested Fix**: require the receiver to be `Expr::Ident("self")`; decline any property or local alias.
