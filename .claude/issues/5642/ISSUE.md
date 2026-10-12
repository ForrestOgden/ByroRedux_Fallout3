# SCR-D5-2026-10-11-05: Force-greet pass 2 opens every in-radius greeter against a stale `open_partner` snapshot — the second open strips the first's line mid-sentence

Labels: low,scripting,dialogue,ai,game:fnv,game:fo3,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 5
- **Location**: `systems/forcegreet.rs:119-122, 200-208, 225-243`
- **Status**: NEW

Pass 2 opens every in-radius greeter against a stale `open_partner` snapshot. Two same-frame greeters open A then B, which strips A's line mid-sentence and spends both directives. Re-read `DialogueSurfaceState::npc` per open

