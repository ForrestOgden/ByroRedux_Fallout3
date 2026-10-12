# SCR-D5-2026-10-11-01: `GetRandomPercent` re-enters the `StoryManagerRng` write lock the Story Manager dispatcher holds across `evaluate` — panic on a vanilla Skyrim CLOC node

Labels: high,scripting,quests,game:skyrim,bug

- **Severity**: HIGH · **Dimension**: Scene/Package/Dialogue (Story Manager); evaluator side Dim 3 · **Untrusted-Input**: Yes (any plugin can author fn-77 nodes; the vanilla trigger below needs none) · independently found as SCR-D3-2026-10-11-01
- **Location**: `crates/scripting/src/story_manager.rs:582` (guard taken), `:598` / `:850` (walk + `evaluate` under it); `crates/scripting/src/condition.rs:746-762` (re-acquire at `:753`); panic source `crates/core/src/ecs/world.rs:865-875` and `lock_tracker.rs:200-218`
- **Status**: NEW — introduced by #5380 (`395d2d831`), which catalogued fn 77 and routed it to this resource
- **Description**: `story_manager_dispatch_system` binds `let mut rng = world.try_resource_mut::<StoryManagerRng>()` and keeps it for the entire phase-A block. That block calls `evaluate(&node.conditions, ..)` for every node whose conditions are all catalogued. Since #5380, `evaluate_function`'s `GetRandomPercent` arm calls `world.try_resource_mut::<StoryManagerRng>()` again. `try_resource_mut` returns `None` only for a missing resource. A write held on the same thread trips the lock tracker's re-entrancy check, which is always on in debug **and** release, so the first fn-77 evaluation panics the Update-stage exclusive system.
- **Evidence**:
  - Skyrim.esm: `DA09ChangeLocationSharesEvent` (SMQN `0x00081C5A`) has CTDAs `[GetQuestRunning(DA09) == 1, GetRandomPercent <= <GLOB>]`. Both functions are catalogued, so the `modeled` gate passes. It is the first quest node walked on every CLOC. The AND list short-circuits while DA09 is un-started, which hides the defect on a fresh game.
  - `WIChangeLocationThiefNode` (`0x23F1B`, CLOC; `[GetLevel >= 5, GetStageDone(TG00), 77, 74, 74]`) is also fully modeled.
  - FO4 `testDogmeatInteractions` has fn 77 as its first condition (ADIA, not raised today).
  - Twelve more fn-77 nodes are masked only by an uncatalogued sibling term.
  - No test installs the RNG together with a fn-77 node (`setup_world`, `story_manager.rs:1015`, omits it).
- **Impact**: process-level panic in normal play on the first location change after DA09 starts, and (via SCR-D5-03) ever after. Any mod SM node gated on `GetRandomPercent` alone crashes at its first event.
- **Related**: #5380, #5372/#5414 (same lock-nest family), SCR-D5-03, SCR-D2-01.
- **Suggested Fix**: do not hold the RNG guard across the walk. Take it only inside the two pick sites, or clone the small `StoryManagerRng` at phase-A entry and write the state back after. Add a dispatch test that installs the RNG with a lone `[77]` node and a `[56, 77]` node.
