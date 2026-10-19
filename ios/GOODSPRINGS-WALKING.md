# Goodsprings milestone: grounded native walking (Phase 14)

## Scope
This change continues the actual FalloutNV.esm Prospector Saloon interior instead of returning to model demonstrations. It layers an original-geometry spatial collision grid and a deterministic first-person movement motor onto the Phase 13 original-Esm-ref / multi-material Metal renderer.

- The original user-local `.fnvworld` package remains the input. Both version 2 and 3 are accepted; prefer the latest version 3 materials.
- Native Rust NIF parsing yields the actual authored mesh triangles. Swift converts triangles using the same quaternion, original NIF scale and ESM REFR placement as the Metal scene (70 Bethesda units per meter).
- On local, source-only data, a bounded 2-meter X/Z grid indexes up to 120,000 triangles within 75 meters of the authored spawn. It ignores degenerate and unreasonably long edges, but **does not fabricate** a floor or wall.
- Player feet are grounded on nearby nearly-horizontal source triangles, with 0.34 meter step-up, fall-down, 9.81 m/s² gravity, basic horizontal wall blocking and axis-wise sliding.
- Player camera is at 1.65 meters above the feet; touch LOOK and MOVE remain; touch JUMP and gamepad A work, while gamepad triggers allow faster walking.
- The original native offline-on-phone private world cache is unchanged. Running the installed app does not require a connected PC or streaming.
- No proprietary assets are committed or uploaded. All tests use synthetic triangles.

## Limitations / do not overclaim
This is a collision approximation from **visible triangles**, not the game's original NIF Havok/ bhk collision layer or Rapier physics. It can block decorative objects, leave invisible physics barriers missing, and handle ramps or stairs less faithfully. The world is capped at 160 models, 480 placements, 192 MiB of source assets, and the collision grid indexes no more than 120,000 nearby visible triangles. Large building pieces omitted by these caps can still cause missing collision. Door transitions, cell-to-cell loading, AI, combat, full audio, quests, saves and the actual Goodsprings exterior remain missing.

## CI
The Mac workflow compiles and runs a standalone Swift movement test using synthetic floor/wall triangle data. It verifies that the player starts grounded, is blocked by a vertical wall, can jump and lands on the same authored floor, and does not invent a floor where there is no geometry. Then it compiles the iPhone ARM64 Rust bridge, Xcode/Metal app and unsigned IPA as before.

## Next engineering targets
1. Verify a genuine v3 Goodsprings export includes the main floor and architectural collision surfaces, comparing against source engine Havok collision; address missing floor/dropped NIFs.
2. Port actual physics colliders (bhk/Havok equivalents) and integrate rigid movement and stairs with authored navmesh.
3. Open original New Vegas door REF links from the saloon to Goodsprings exterior. Build outdoor terrain streaming from LAND/WRLD.
4. Bring over player interactions, inventory, audio, dialogue, NPCs, quest state and saves from the desktop ByroRedux engine.
5. Profile actual phone GPU/CPU memory and frame times; game completeness requires broad play-through testing.

**Current status:** source-only implementation; cannot claim hardware-proven collision until actual scene navigation is run on-device.
