//! Engine-side Story Manager event producers — #5366 Phases 1–2.
//!
//! The scripting crate owns dispatch (`story_manager_dispatch_system`)
//! and the marker; this module owns the engine surfaces that raise it.
//! Phase 1 wires `CLOC` here (the session's location identity from the
//! cell-loader contexts) — `KILL` is raised by [`raise_kill_story_event`] at
//! each live death producer (combat carries the aggressor; drowning and
//! the SDK batch have none).
//!
//! Phase 2 tightens the CLOC key to Skyrim's location granularity: when
//! the current cell resolves an LCTN (`XLCN`, already decoded into
//! `CellData::location_form`), the key is that LCTN — moving between
//! two cells of one location no longer fires, moving within one hold
//! between authored locations does. Cells without an LCTN (wilderness
//! grids) keep the Phase-1 worldspace+grid key, so cadence there is
//! unchanged. The resolved LCTN rides on the event as `L2` (new) with
//! the previous fire's as `L1` (old).

use crate::cell_loader::{CurrentCellContext, CurrentExteriorContext, LoadedCellIndex};
use crate::systems::character::PlayerEntity;
use byroredux_core::ecs::storage::EntityId;
use byroredux_core::ecs::world::World;
use std::hash::{Hash, Hasher};

/// System: raise a `CLOC` story event when the session's location
/// identity changes.
///
/// No player entity (loose-NIF demo, spawn ladders): no event — a
/// location change nobody occupies is not a story event.
pub fn story_change_location_system(world: &World) {
    let Some(player) = world
        .try_resource::<PlayerEntity>()
        .and_then(|player| player.0)
    else {
        return;
    };
    // LCTN first (Phase 2): interior cells by editor-id, exterior grid
    // cells by worldspace+grid. The loaded index is installed on every
    // load path; without it (or without an `XLCN`) the location stays
    // `None` and the key falls back to the Phase-1 identity.
    let location = resolve_current_lctn(world);
    let key = if let Some(lctn) = location {
        let parts: [&str; 2] = ["lctn", &format!("{:08X}", lctn)];
        Some(location_hash(&parts))
    } else if let Some(cell) = world.try_resource::<CurrentCellContext>() {
        let parts: [&str; 2] = [&cell.esm_path, &cell.cell_editor_id];
        Some(location_hash(&parts))
    } else if let Some(exterior) = world.try_resource::<CurrentExteriorContext>() {
        let grid = format!("grid:{:?}", exterior.grid);
        let parts: [&str; 3] = [&exterior.esm_path, &exterior.worldspace_key, &grid];
        Some(location_hash(&parts))
    } else {
        // Neither context: mid-transition window or loose-NIF mode. The
        // cursor follows so the post-transition frame counts as a change.
        None
    };
    byroredux_scripting::story_manager::emit_change_location_on_key_change(
        world, key, player, location,
    );
}

/// The current cell's `XLCN` LCTN, interior or exterior grid cell, from
/// the loaded plugin index. `None` when the index is absent, the cell
/// isn't found, or the record authors no location (wilderness grids).
pub(crate) fn resolve_current_lctn(world: &World) -> Option<u32> {
    let index = world.try_resource::<LoadedCellIndex>()?.0.clone();
    if let Some(cell) = world.try_resource::<CurrentCellContext>() {
        return index
            .cells
            .cells
            .get(&cell.cell_editor_id.to_ascii_lowercase())
            .and_then(|data| data.location_form);
    }
    if let Some(exterior) = world.try_resource::<CurrentExteriorContext>() {
        return index
            .cells
            .exterior_cells
            .get(&exterior.worldspace_key)
            .and_then(|grid| grid.get(&exterior.grid))
            .and_then(|data| data.location_form);
    }
    None
}

/// Process-local identity hash for change detection only — never
/// persisted, so `DefaultHasher`'s unspecified seeding is fine.
fn location_hash(parts: &[&str]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    hasher.finish()
}

/// #5366 Phase 4 — `AHEL` (Actor Hello): raised when a conversation
/// opens (activation greeting or force-greet — the #5367 surfaces).
/// `OnStoryHello(akLocation, akActor1, akActor2)`: L1 = the session's
/// current LCTN, R1 = the greeter, R2 = the greeted party. On
/// pre-Creation titles this is a no-op (no SM tree installed).
pub(crate) fn raise_hello_story_event(world: &World, greeter: EntityId, greeted: EntityId) {
    use byroredux_scripting::story_manager::StoryEvent;
    // #5372 — resolve the location BEFORE taking the StoryEvent write
    // guard: `resolve_current_lctn` reads `LoadedCellIndex`, and every
    // dialogue-open call site (forcegreet_open, the selection system's
    // Pass 2) already holds a `LoadedCellIndex` read of its own. As a
    // struct-field expression it evaluated under the marker guard and
    // closed the cycle #5066 predicted (LoadedCellIndex -> StoryEvent
    // -> LoadedCellIndex).
    let location_1 = resolve_current_lctn(world);
    let Some(mut events) = world.query_mut::<StoryEvent>() else {
        return;
    };
    events.insert(
        greeter,
        StoryEvent {
            mnemonic: *b"AHEL",
            reference_1: greeter,
            reference_2: Some(greeted),
            location_1,
            location_2: None,
        },
    );
}

/// #5434 — `KILL` (Actor Kill), raised from every LIVE death producer
/// (combat damage, NPC/player drowning, the SDK actor-value batch) — never
/// from corpse restores, which re-assert a death that already fired.
/// Slots: R1 = victim, R2 = killer (`None` for environmental deaths such as
/// drowning), L1 = the session's current LCTN (`WIKill06` authors a
/// KILL→L1 alias fill). On pre-Creation titles this is a no-op (no SM tree
/// installed).
pub(crate) fn raise_kill_story_event(world: &World, victim: EntityId, killer: Option<EntityId>) {
    use byroredux_scripting::story_manager::StoryEvent;
    // #5372 — resolve the location BEFORE taking the StoryEvent write
    // guard (`resolve_current_lctn` reads `LoadedCellIndex`).
    let location_1 = resolve_current_lctn(world);
    let Some(mut events) = world.query_mut::<StoryEvent>() else {
        return;
    };
    events.insert(
        victim,
        StoryEvent {
            mnemonic: *b"KILL",
            reference_1: victim,
            reference_2: killer,
            location_1,
            location_2: None,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #5366 Phase 4 — the AHEL producer stamps the hello shape the CK
    /// documents (`OnStoryHello(akLocation, akActor1, akActor2)`):
    /// greeter → R1, greeted → R2, session LCTN → L1 (absent index →
    /// none, the loose-NIF/no-context shape).
    #[test]
    fn hello_event_carries_greeter_greeted_and_session_location() {
        let mut world = World::new();
        byroredux_scripting::register(&mut world);
        let greeter = world.spawn();
        let greeted = world.spawn();
        raise_hello_story_event(&world, greeter, greeted);
        let event = world
            .query::<byroredux_scripting::story_manager::StoryEvent>()
            .map(|events| events.iter().next().map(|(_, event)| *event))
            .unwrap_or_default()
            .expect("the hello marker is raised on the greeter");
        assert_eq!(event.mnemonic, *b"AHEL");
        assert_eq!(event.reference_1, greeter);
        assert_eq!(event.reference_2, Some(greeted));
        assert_eq!(event.location_1, None, "no loaded index — no session LCTN");
        assert_eq!(event.location_2, None);
    }

    /// #5434 — a world whose session sits in a cell that authors LCTN
    /// `0x0001_0A0B`.
    fn world_in_located_cell() -> World {
        use byroredux_plugin::esm::cell::CellData;
        let mut world = World::new();
        byroredux_scripting::register(&mut world);
        let cell = CellData {
            form_id: 0x0001_2345,
            editor_id: "WhiterunBanneredMare".to_string(),
            display_name: None,
            references: Vec::new(),
            is_interior: true,
            show_sky: None,
            grid: None,
            lighting: None,
            landscape: None,
            water_height: None,
            water_height_is_explicit: false,
            image_space_form: None,
            water_type_form: None,
            acoustic_space_form: None,
            music_type_form: None,
            music_type_enum: None,
            climate_override: None,
            location_form: Some(0x0001_0A0B),
            encounter_zone_form: None,
            regions: Vec::new(),
            lighting_template_form: None,
            ownership: None,
            regional_color_override: None,
            precombined_mesh_hashes: Vec::new(),
            absorbed_ref_bakes: Vec::new(),
            navmeshes: Vec::new(),
            pathgrids: Vec::new(),
            deleted_refs: Vec::new(),
        };
        let mut loaded = byroredux_plugin::esm::records::EsmIndex::default();
        loaded.cells.cells.insert("whiterunbanneredmare".to_string(), cell);
        world.insert_resource(LoadedCellIndex(std::sync::Arc::new(loaded)));
        world.insert_resource(CurrentCellContext {
            cell_editor_id: "WhiterunBanneredMare".to_string(),
            esm_path: String::new(),
            masters: Vec::new(),
        });
        world
    }

    /// #5434 — KILL carries victim → R1, killer → R2 and the session LCTN
    /// → L1, the same location the AHEL producer resolves (`WIKill06`
    /// authors a KILL→L1 alias fill).
    #[test]
    fn kill_event_carries_victim_killer_and_session_location() {
        let mut world = world_in_located_cell();
        let victim = world.spawn();
        let killer = world.spawn();
        raise_kill_story_event(&world, victim, Some(killer));
        let event = *world
            .query::<byroredux_scripting::story_manager::StoryEvent>()
            .expect("StoryEvent storage")
            .get(victim)
            .expect("the KILL marker is raised on the victim");
        assert_eq!(event.mnemonic, *b"KILL");
        assert_eq!(event.reference_1, victim);
        assert_eq!(event.reference_2, Some(killer));
        assert_eq!(event.location_1, Some(0x0001_0A0B));
        assert_eq!(event.location_2, None);
    }

    /// #5434 — an environmental death (drowning, SDK batch) has no killer.
    #[test]
    fn kill_event_without_a_killer_leaves_r2_empty() {
        let mut world = world_in_located_cell();
        let victim = world.spawn();
        raise_kill_story_event(&world, victim, None);
        let event = *world
            .query::<byroredux_scripting::story_manager::StoryEvent>()
            .expect("StoryEvent storage")
            .get(victim)
            .expect("the KILL marker is raised on the victim");
        assert_eq!(event.reference_2, None);
        assert_eq!(event.location_1, Some(0x0001_0A0B));
    }
}
