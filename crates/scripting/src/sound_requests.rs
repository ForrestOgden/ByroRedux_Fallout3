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
