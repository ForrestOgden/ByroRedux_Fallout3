# SCR-D1-2026-10-11-05: `SetMotionType` integer literals use the NIF `hkMotionType` table, not the Papyrus `Motion_*` constants

Labels: high,scripting,physics,game:skyrim,game:fo4,bug

- **Severity**: HIGH (wrong lowering accepted) · **Dimension**: Recognizer Chain · **Untrusted-Input**: Yes
- **Location**: `crates/scripting/src/translate/effects.rs:1549-1577` (literal arm `:1553-1562`); test pin `:3311-3339` (`lowers_literal_havok_motion_types`)
- **Status**: NEW — premise error in closed #2286, which moved the table to the NIF enum on an unverified claim
- **Description**: the literal arm maps `1..=5|8 → Dynamic, 6 → Keyframed, 7 → Static, 9 → CharacterKinematic`, copied from the NIF importer. The CK wiki (`SetMotionType - ObjectReference.wiki`) gives Skyrim `Motion_Dynamic=1, SphereInertia=2, BoxInertia=3, Keyframed=4, Fixed=5, ThinBoxInertia=6, Character=7`. FO4 (`falloutck-uesp-wiki`) gives `Fixed=0, Dynamic=1, Keyframed=2`. **Confirmed by the orchestrator against the wiki and the code.**
- **Evidence**: Skyrim literal `4` (Keyframed) → `Dynamic`; `5` (Fixed) → `Dynamic`; `6` → `Keyframed`; `7` (Character) → `Static`. FO4 `2` (Keyframed) → `Dynamic`; `0` (Fixed) declines. The named-member arm is correct for Skyrim.
- **Impact**: a keyframed prop is released to physics, or a dynamic one frozen, by a literal-argument call.
- **Related**: #2286, #1652, *no-guessing policy*.
- **Suggested Fix**: the integer arm cannot be game-agnostic. Decline every literal except `1`, or take the game from the attach context and use the per-game Papyrus table. Re-pin the test with the wiki values.
