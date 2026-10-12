# SCR-D7-2026-10-11-02: ModEvent delivery is O(programs × events × registrations) and the registration map is cloned every frame

Labels: medium,scripting,performance,bug

- **Severity**: MEDIUM · **Dimension**: Provider/Extender Layer · **Untrusted-Input**: Yes
- **Location**: `papyrus_provider/execute.rs:34-40` (unconditional `registrations.clone()`), `:162-252`
- **Status**: NEW
- **Description**: the innermost loop filters by entity after iterating the whole map. The caps (`MAX_PENDING_PAPYRUS_MOD_EVENTS` 256 × `MAX_PAPYRUS_MOD_EVENT_REGISTRATIONS` 4096) allow about 10^6 iterations per program entity per frame, and about 10^9 for a script on ~1 000 references.
- **Impact**: mod-controlled multi-second hitches.
- **Suggested Fix**: clone only when events are pending, and key the lookup by entity with a `range` scan.
