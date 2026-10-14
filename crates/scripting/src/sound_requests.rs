//! `Sound.Play(akSource)` requests (M47.2).
//!
//! CK *Play - Sound*: "Plays the sound from the location of the specified
//! object reference." A script run records the request; the host's
//! `scripted_sound_system` resolves the sound descriptor to a file, decodes it
//! and plays it from the source. Transient: a request is played the frame it is
//! made or dropped.

use byroredux_core::ecs::resource::Resource;

/// One `Sound.Play` call: the sound's form and the reference it plays from
/// (`0x14` is the player).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundPlay {
    pub sound: u32,
    pub source: u32,
}

/// The `Sound.Play` calls awaiting the host.
#[derive(Debug, Default)]
pub struct PendingSoundPlays(pub Vec<SoundPlay>);

impl Resource for PendingSoundPlays {}

/// Sound descriptor editor ids (lowercase) to their forms, for the
/// `SoundPlay.<name>` notifies an animation graph raises. `NOT_SAVED_BY_DESIGN`:
/// rebuilt from the records every boot.
#[derive(Debug, Default, Clone)]
pub struct SoundEditorIds(std::collections::HashMap<String, u32>);

impl Resource for SoundEditorIds {}

impl SoundEditorIds {
    pub fn from_index(index: &byroredux_plugin::esm::records::EsmIndex) -> Self {
        Self(
            index
                .sounds
                .values()
                .filter(|sound| !sound.editor_id.is_empty())
                .map(|sound| (sound.editor_id.to_ascii_lowercase(), sound.form_id))
                .collect(),
        )
    }

    /// The sound a `SoundPlay.<name>` graph event names (the prefix matched
    /// ignoring case; the node-attached `SoundPlayAt.<name>.<node>` form is not
    /// one).
    pub fn of_graph_event(&self, event: &str) -> Option<u32> {
        let lower = event.to_ascii_lowercase();
        let name = lower.strip_prefix("soundplay.")?;
        self.0.get(name).copied()
    }
}

/// Install the load order's [`SoundEditorIds`]; returns how many sounds are named.
pub fn install_sound_editor_ids(world: &mut byroredux_core::ecs::World, index: &byroredux_plugin::esm::records::EsmIndex) -> usize {
    let ids = SoundEditorIds::from_index(index);
    let count = ids.0.len();
    world.insert_resource(ids);
    count
}
