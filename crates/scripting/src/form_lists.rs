//! Form lists scripts can read (M47.2): `GetSize`, `GetAt`, `HasForm` and `Find`
//! on a `FLST` record.
//!
//! CK *FormList Script*: `GetSize` "the number of forms in the list", `GetAt`
//! "the form at index aiIndex ... none in case of error (such as wrong index
//! value)", `HasForm` "whether the passed-in form exists in this form list",
//! `Find` "the index ... or -1 if it's not a member". The lists are the
//! records' own; forms scripts add (`AddForm`) are not modelled. A list's null
//! entries (`0xFFFFFFFF`, the sentinel the parser keeps for slot positions)
//! are no forms and are dropped here. Derived from the records, never saved.

use std::collections::HashMap;
use std::sync::Arc;

use byroredux_core::ecs::resource::Resource;
use byroredux_core::ecs::world::World;
use byroredux_plugin::esm::records::EsmIndex;

/// Every `FLST` record's forms, by FormID.
#[derive(Debug, Default, Clone)]
pub struct FormLists(Arc<HashMap<u32, Vec<u32>>>);

impl Resource for FormLists {}

impl FormLists {
    /// The forms of the list `list`, or `None` when no such list is loaded.
    pub fn get(&self, list: u32) -> Option<&[u32]> {
        self.0.get(&list).map(Vec::as_slice)
    }
}

/// Install the load order's form lists; returns how many.
pub fn install_form_lists(world: &mut World, index: &EsmIndex) -> usize {
    let lists: HashMap<u32, Vec<u32>> = index
        .form_lists
        .iter()
        .map(|(&form_id, list)| {
            let forms = list.entries.iter().copied().filter(|&form| form != 0 && form != 0xFFFF_FFFF).collect();
            (form_id, forms)
        })
        .collect();
    let count = lists.len();
    world.insert_resource(FormLists(Arc::new(lists)));
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use byroredux_plugin::esm::records::FlstRecord;

    #[test]
    fn null_entries_are_not_forms() {
        let mut index = EsmIndex::default();
        index.form_lists.insert(
            0x10,
            FlstRecord {
                form_id: 0x10,
                entries: vec![0xA, 0xFFFF_FFFF, 0xB],
                ..Default::default()
            },
        );
        let mut world = World::new();
        assert_eq!(install_form_lists(&mut world, &index), 1);
        let lists = world.resource::<FormLists>();
        assert_eq!(lists.get(0x10), Some(&[0xA, 0xB][..]));
        assert_eq!(lists.get(0x11), None);
    }
}
