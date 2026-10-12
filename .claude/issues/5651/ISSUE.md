# SCR-D1-2026-10-11-08: `PlayerControlSelection::PAPYRUS_DEFAULT` is a hybrid that matches neither declared default set

Labels: low,scripting,bug

- **Severity**: LOW
- **Dimension**: Scripting Dim 1
- **Location**: `player_control.rs:30-42`; `effects.rs:1283-1294`
- **Status**: NEW

`PlayerControlSelection::PAPYRUS_DEFAULT` is one hybrid table. The CK wiki's Enable and Disable defaults differ (camera/looking/sneaking, journal tabs). Two constants. Only `.psc` bodies see the fill-in

