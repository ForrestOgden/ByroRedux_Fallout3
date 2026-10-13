//! Created Object "At" quest aliases (M47.2).
//!
//! The CK *Quest Alias Tab*: "Create Reference to Object: When the quest
//! starts, a reference will be created using the selected base object. The
//! new reference can either be placed at the same location as another
//! Reference Alias (usually a marker), or in the inventory of another
//! Reference Alias". The "In" half grants an item (`quest_alias.rs`); this
//! module is the "At" half — 441 Skyrim and 1,175 FO4 aliases, 1,389 of them
//! `NPC_` spawns (`crates/plugin/examples/alias_created_object_census.rs`).
//!
//! The alias refresh records one [`CreatedReference`] per created alias of a
//! running quest once its target alias is bound, with a fresh FormID in the
//! created-object plugin slot `0xFF` (UESP *Skyrim Mod:Save File Format*:
//! "Created (ie, plugin index of 0xFF)"). The engine resolves where it stands
//! — the target reference's placement — and spawns it whenever that cell is
//! loaded; the refresh then binds the alias to the spawned entity like any
//! other reference. Records are saved, so the reference keeps its identity
//! and place across reloads. A stopped quest's records are dropped: its
//! created references no longer respawn.

use std::collections::HashMap;

use byroredux_core::ecs::resource::Resource;

use crate::quest_stages::QuestFormId;

/// The created-object plugin slot's first FormID.
pub const CREATED_FORM_ID_BASE: u32 = 0xFF00_0000;

/// The cell a created reference stands in.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub enum CreatedCell {
    /// An interior cell, by editor ID.
    Interior { editor_id: String },
    /// An exterior grid cell.
    Exterior { worldspace: String, grid: (i32, i32) },
}

/// Where a created reference stands: its cell and its pose in Bethesda Z-up
/// world units (the placement convention `PlacedRef` uses).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct CreatedPlacement {
    pub cell: CreatedCell,
    pub position_zup: [f32; 3],
    pub rotation_zup: [f32; 3],
}

/// One created reference.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct CreatedReference {
    /// Its FormID, in the `0xFF` created-object slot.
    pub form_id: u32,
    /// The base object (`ALCO`).
    pub base: u32,
    /// `ALCL`'s encounter level, as authored.
    pub level: u32,
    /// The reference it is created at — the target alias's reference.
    pub at_reference: u32,
    /// Resolved by the engine from `at_reference`; `None` until then.
    pub placement: Option<CreatedPlacement>,
}

/// Every created reference of a running quest, by quest and alias. Saved.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct CreatedReferences {
    by_alias: HashMap<QuestFormId, HashMap<i32, CreatedReference>>,
    /// The next created FormID's low 24 bits.
    next: u32,
}

impl Resource for CreatedReferences {}

impl CreatedReferences {
    pub fn get(&self, quest: QuestFormId, alias_id: i32) -> Option<&CreatedReference> {
        self.by_alias.get(&quest)?.get(&alias_id)
    }

    /// Record a new created reference for `quest`'s alias, returning its
    /// FormID. `None` once the 24-bit created slot is exhausted.
    pub fn create(
        &mut self,
        quest: QuestFormId,
        alias_id: i32,
        base: u32,
        level: u32,
        at_reference: u32,
    ) -> Option<u32> {
        if self.next > 0x00FF_FFFF {
            return None;
        }
        let form_id = CREATED_FORM_ID_BASE | self.next;
        self.next += 1;
        self.by_alias.entry(quest).or_default().insert(
            alias_id,
            CreatedReference {
                form_id,
                base,
                level,
                at_reference,
                placement: None,
            },
        );
        Some(form_id)
    }

    /// Drop the records of quests `running` rejects.
    pub fn retain_quests(&mut self, mut running: impl FnMut(QuestFormId) -> bool) {
        self.by_alias.retain(|quest, _| running(*quest));
    }

    /// Every record, in no particular order.
    pub fn iter(&self) -> impl Iterator<Item = &CreatedReference> + '_ {
        self.by_alias.values().flat_map(|aliases| aliases.values())
    }

    /// Record where `form_id` stands.
    pub fn place(&mut self, form_id: u32, placement: CreatedPlacement) {
        if let Some(record) = self
            .by_alias
            .values_mut()
            .flat_map(|aliases| aliases.values_mut())
            .find(|record| record.form_id == form_id)
        {
            record.placement = Some(placement);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_form_ids_live_in_the_0xff_slot_and_records_follow_their_quest() {
        let mut created = CreatedReferences::default();
        let first = created.create(QuestFormId(1), 3, 0xA, 0, 0x55).unwrap();
        let second = created.create(QuestFormId(2), 4, 0xB, 1, 0x56).unwrap();
        assert_eq!(first, 0xFF00_0000);
        assert_eq!(second, 0xFF00_0001);
        assert_eq!(created.get(QuestFormId(1), 3).unwrap().at_reference, 0x55);

        let placement = CreatedPlacement {
            cell: CreatedCell::Interior {
                editor_id: "MQ101".into(),
            },
            position_zup: [1.0, 2.0, 3.0],
            rotation_zup: [0.0; 3],
        };
        created.place(second, placement.clone());
        assert_eq!(created.get(QuestFormId(2), 4).unwrap().placement, Some(placement));

        created.retain_quests(|quest| quest == QuestFormId(2));
        assert!(created.get(QuestFormId(1), 3).is_none());
        // FormIDs are never reused.
        assert_eq!(created.create(QuestFormId(1), 3, 0xA, 0, 0x55), Some(0xFF00_0002));
    }
}
