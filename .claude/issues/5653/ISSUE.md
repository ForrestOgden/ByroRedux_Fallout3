# SCR-D1-2026-10-11-10: `prim_lock` can never match a compiled `Lock` call (declared 2 parameters, bounded at 1)

Labels: low,scripting,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 1
- **Location**: `effects.rs:1207-1217`
- **Status**: NEW

`prim_lock` bounds at 1 argument, but compiled `Lock(abLock, abAsOwner)` materializes 2, so it never fires on real content. Decline-safe (#3487 class)

