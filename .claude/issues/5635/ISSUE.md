# SCR-D7-2026-10-11-01: `execute_statements` recurses once per executed `If` and clones the tail — linear stack and quadratic heap in the number of sequential `If`s

Labels: high,scripting,bug

- **Severity**: HIGH (skill: unbounded alloc or recursion on mod-supplied bytes) · **Dimension**: Provider/Extender Layer · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/papyrus_provider/execute.rs:873-895` (the `If` arm builds `ordered_tail` and recurses; confirmed by the orchestrator); entry `:340-347` (`papyrus_provider_system`, Late exclusive, `boot/schedule/late.rs:366-390`)
- **Status**: NEW — the same defect as closed #3935, which was fixed only at `fragment/effects.rs:1867-1880`
- **Description**: `MAX_PROVIDER_HANDLER_NESTING` (32) bounds nesting, not the sequential count. Lowering has no statement cap, and a `.pex` function may carry 65 535 instructions. Each executed `If` clones the rest of the handler and recurses, so depth equals the number of `If`s executed on the path, and live memory is the sum of the remaining tail lengths.
- **Evidence**: a handler with one provider call and N × `If b / EndIf` is valid. The agent's **estimate (not measured)**: N=3 000 → about 1 GB and 3 000 frames; N=10 000 → OOM. `catching_panics` does not wrap this system, and could not catch OOM or stack overflow anyway. Benign scripts (N < 100) are unaffected, which is why no test sees it.
- **Impact**: a mod `.pex` can OOM or stack-overflow the process the first time its event fires (OnInit at cell attach, or every OnUpdate tick).
- **Suggested Fix**: port #3935's cursor stack (borrowed slices, no clone; build an owned tail only at a `Wait` suspension). Add a `MAX_PROVIDER_STATEMENTS` cap at lowering. Test: 20 000 sequential `If`s run in linear time.
