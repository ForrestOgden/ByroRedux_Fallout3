# NIFAL-D6-2026-10-11-02: The raw per-game Havok material number is carried on the canonical `RigidBodyData` with no translate step, no sourced mapping, and no spec row

Labels: low,nifal,physics,bug

- **Severity**: LOW (dormant: both consumer maps ship empty)
- **Dimension**: Collision
- **Tier Violated**: no-leak
- **Game Affected**: Oblivion, FO3, FNV, Skyrim LE/SE (FO4+/Starfield bodies get the `0` default; no material is read from the `BhkNP` blob)
- **Location**: `crates/core/src/ecs/components/collision.rs:118-145` (`surface_material: u32` at :131, `part_surface_materials` at :144); `crates/nif/src/import/collision/mod.rs:444-466`; consumers `byroredux/src/components.rs:1826-1900` (`surface_sounds: BTreeMap<u32, Arc<Sound>>`), `byroredux/src/systems/audio.rs`
- **Status**: NEW. Adjacent: #5542 (CLOSED) listed "translate to a canonical engine surface enum at the NIFAL boundary, per game" as the alternative to widening; the fix widened instead, which leaves this seam untracked. #5549 (OPEN) covers only the `0` sentinel half.
- **Description**: This is the same class as `hkMotionType` → `MotionType` (collapsed at translate), but nothing collapses here. Oblivion's `0=STONE,1=CLOTH,…`, FO3/FNV's different 0–127 table and Skyrim's `SKY_HAV_MAT_*` CRC32s sit side by side in one `u32` and mean something only relative to the loaded game. `0` is STONE on Oblivion/FO3/FNV but `NONE` on Skyrim. A populated `surface_sounds` map must either be game-specific (a per-game branch downstream of the boundary) or rely on one game per process. `nifal.md` §2 "Collision" and `physal.md` do not mention the channel, so it is neither a documented translation nor a parked row.
- **Evidence**: The `collision.rs` doc says "the per-game material enum's numeric value (Oblivion/Fallout3/Skyrim HavokMaterial …)"; `grep -n "surface_material\|HavokMaterial" docs/engine/nifal.md docs/engine/physal.md` returns nothing. The map-building step is deferred in 886edcded's message.
- **Impact**: None audible today. When the first map entry lands the consumer must key on a game-dependent number or add a per-game branch. A saved column already persists the raw number, so a later switch to a canonical id costs another `FORMAT_MAJOR` bump.
- **Related**: #5542 (closed), #5549 (open), #5550 (open), #4256 (analogous "discriminator never crosses" shape).
- **Suggested Fix**: Before any map is populated, decide the canonical form (a game-agnostic surface id derived at the import boundary) and add a parked row to `nifal.md` "Collision". Otherwise record explicitly that raw-per-game is the intended canonical form and name the game key.

_Source: `docs/audits/AUDIT_NIFAL_2026-10-11.md` finding D6-02 (HEAD c6e48cbc6)._

## Completeness Checks
- [ ] **SIBLING**: Same pattern checked in related files
- [ ] **CANONICAL-BOUNDARY**: per-game logic stays at the NIFAL parser→canonical boundary — never pushed into shaders/renderer or re-derived downstream. See `/audit-nifal`.
