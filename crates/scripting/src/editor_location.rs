//! A placed reference's authored ("editor") location — where the plugin put it.
//!
//! CK *MoveToMyEditorLocation - ObjectReference*: "Moves this reference to its
//! original editor location and orientation." The cell loader stamps the
//! component when it spawns the reference, so it is the authored pose however
//! far a script has since moved the reference; it is re-derived at every spawn,
//! never saved.

use byroredux_core::ecs::sparse_set::SparseSetStorage;
use byroredux_core::ecs::storage::Component;
use byroredux_core::math::{Quat, Vec3};

/// The pose a reference was placed at, in the renderer's Y-up space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorLocation {
    pub translation: Vec3,
    pub rotation: Quat,
}

impl Component for EditorLocation {
    type Storage = SparseSetStorage<Self>;
}
