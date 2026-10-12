# SCR-D6-2026-10-11-02: a tag-`r` `Set` is a remote-variable assignment, but `exec_set` treats it as a ref-variable assignment and shadows the target object

Labels: high,scripting,game:oblivion,game:fnv,game:fo3,bug

- **Severity**: HIGH · **Dimension**: Legacy ObScript · **Untrusted-Input**: No for vanilla; Yes in general
- **Location**: `crates/scripting/src/obscript_vm.rs:416-419` (doc), `:428-432`, `:458-461` (`ref_vars.insert(idx, form)`), `:672-674` (`ref_vars` shadows SCRO)
- **Status**: NEW
- **Description**: in all 1646 (Oblivion) / 4240 (FNV) / 1290 (FO3) vanilla tag-`r` `Set` statements, bytes 3–5 of the target are `s`/`f` + a variable ordinal, not zero as the doc comment claims. The escape is `<ref>.<remote variable>`, the target of `set Quest.var to x`. `exec_set` stores `ref_vars[idx] = value as u32`, and `resolve_ref_var` consults `ref_vars` before the SCRO table. After `set MS51Quest.iFlag to 1`, every later reference to that slot in the same block resolves to form `1`.
- **Evidence** (agent census; textual lower bound): a stage or query op follows such a `Set` on the same slot in 5 Oblivion (`SE07BScript`, `TGCastOut`, `MS51script`, `MQ01Script`, `RufioDieScript`), 9 FNV and 6 FO3 quest scripts. The remote-variable write itself is silently lost and is not counted as an unknown command.
- **Impact**: the later stage op runs on `QuestFormId(<small integer>)`, advancing the wrong quest or creating a phantom entry.
- **Related**: SCR-D6-01. Fixing 01 alone does not fix this.
- **Suggested Fix**: parse the 6-byte target as `ref + variable token`. Do not write `ref_vars` for it. Count it under `unknown_commands` ("remote variable write") or decline the block. Correct the doc comment with the corpus numbers.
