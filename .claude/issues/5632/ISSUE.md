# SCR-D1-2026-10-11-03: branch-local rebinding is lost — `If` branches lower against cloned `Scope`s that are never merged back

Labels: high,scripting,bug

- **Severity**: HIGH (receiver resolved to the wrong target) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/translate/effects.rs:548-553`, `:508-513`
- **Status**: NEW
- **Description**: cloned scopes are conservative for *declarations* inside a branch. Papyrus locals are function-scoped, though, so `k = OtherQuestProp` inside a branch changes what `k` means after `EndIf`. `Assign` goes through `bind_local` on the clone, which is dropped. Later statements lower against the pre-branch binding.
- **Evidence**: `Quest k = Self / If Self.GetStageDone(10) / k = OtherQuestProp / EndIf / k.SetStage(20)` lowers to an unconditional `SetStage { SelfRef, 20 }`. No test constructs a rebind inside a branch.
- **Impact**: effect applied to the wrong quest, actor or reference.
- **Suggested Fix**: after lowering both branches, decline when any pre-existing binding differs between a branch scope and the outer scope.
