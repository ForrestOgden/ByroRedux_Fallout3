//! Havok Behavior data for placed objects (M47.2): the project, character and
//! behavior-graph packfiles that drive an animated prop (a lever, a door, a trap).
//!
//! A NIF with a `BSBehaviorGraphExtraData` names a *project* file. The project
//! lists a *character* file; the character names its skeleton, its animation
//! clips and a *behavior* file; the behavior is a state machine whose
//! transitions fire on named events and whose states play one clip each.
//! `ObjectReference.PlayAnimation(event)` sends such an event.
//!
//! Only what that needs is decoded: the event names, the (root) state machine's
//! states with their transitions and clip, and each clip's trigger events. Blend
//! trees, variables and transition effects are not executed (a transition is
//! instantaneous). The class layouts are Havok 2010.2 (`hk_2010.2.0-r1`),
//! 64-bit: the Havok SDK on disk carries no Behavior headers, so every offset
//! below was derived from, and checked against, the strings and ids in vanilla
//! Skyrim Special Edition lever, door and trap graphs (`examples/probe_behavior.rs`).
//! A 32-bit (Skyrim LE) packfile is reported as an unsupported layout.

use crate::packfile::Packfile;
use crate::{HkxError, Result};

/// Sanity caps on array sizes read from a file.
const MAX_EVENTS: usize = 4096;
const MAX_STATES: usize = 1024;
const MAX_TRANSITIONS: usize = 4096;
const MAX_TRIGGERS: usize = 1024;

// ── 64-bit Havok 2010.2 field offsets ─────────────────────────────────

/// `hkbBehaviorGraphStringData::m_eventNames` (`hkArray<hkStringPtr>`).
const STRING_DATA_EVENT_NAMES: usize = 0x10;
/// `hkbBehaviorGraph::m_rootGenerator`.
const GRAPH_ROOT_GENERATOR: usize = 0x80;
/// `hkbBehaviorGraph::m_data` (`hkbBehaviorGraphData`).
const GRAPH_DATA: usize = 0x88;
/// `hkbBehaviorGraphData::m_stringData`.
const GRAPH_DATA_STRING_DATA: usize = 0x78;
/// `hkbStateMachine::m_startStateId`.
const MACHINE_START_STATE: usize = 0x68;
/// `hkbStateMachine::m_states` (`hkArray<hkbStateMachine::StateInfo*>`).
const MACHINE_STATES: usize = 0x90;
/// `hkbStateMachine::m_wildcardTransitions`.
const MACHINE_WILDCARD: usize = 0xA0;
/// `hkbStateMachine::StateInfo` fields.
const STATE_TRANSITIONS: usize = 0x50;
const STATE_GENERATOR: usize = 0x58;
const STATE_NAME: usize = 0x60;
const STATE_ID: usize = 0x68;
/// `hkbStateMachineTransitionInfoArray::m_transitions` and its element layout.
const TRANSITIONS_ARRAY: usize = 0x10;
const TRANSITION_STRIDE: usize = 72;
const TRANSITION_EVENT_ID: usize = 0x30;
const TRANSITION_TO_STATE: usize = 0x34;
const TRANSITION_PRIORITY: usize = 0x40;
const TRANSITION_FLAGS: usize = 0x42;
/// `hkbClipGenerator` fields.
const CLIP_ANIMATION_NAME: usize = 0x48;
const CLIP_TRIGGERS: usize = 0x50;
const CLIP_START_TIME: usize = 0x60;
const CLIP_PLAYBACK_SPEED: usize = 0x64;
const CLIP_USER_FRACTION: usize = 0x6C;
const CLIP_MODE: usize = 0x72;
/// `hkbClipTriggerArray::m_triggers` and its element layout.
const TRIGGERS_ARRAY: usize = 0x10;
const TRIGGER_STRIDE: usize = 32;
const TRIGGER_LOCAL_TIME: usize = 0x00;
const TRIGGER_EVENT_ID: usize = 0x08;
const TRIGGER_RELATIVE_TO_END: usize = 0x18;
const TRIGGER_IS_ANNOTATION: usize = 0x1A;
/// `hkbCharacterStringData` fields.
const CHARACTER_ANIMATION_NAMES: usize = 0x30;
const CHARACTER_RIG_NAME: usize = 0xA8;
const CHARACTER_BEHAVIOR_FILE: usize = 0xB8;
/// `hkbProjectStringData::m_characterFilenames`.
const PROJECT_CHARACTER_FILENAMES: usize = 0x30;

/// `hkbStateMachine::TransitionInfo::m_flags`: the transition never fires.
pub const TRANSITION_FLAG_DISABLED: u16 = 0x20;

/// How a state's clip plays (`hkbClipGenerator::PlaybackMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipMode {
    /// Plays once and holds its last pose.
    SinglePlay,
    Looping,
    /// Held at `user_fraction` of its length (a static pose).
    UserControlled,
    PingPong,
}

/// An event a clip raises while it plays.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipTrigger {
    /// Seconds from the clip start, or from its end when `relative_to_end`.
    pub local_time: f32,
    /// Index into [`BehaviorGraph::events`].
    pub event: i32,
    pub relative_to_end: bool,
    /// Authored on the clip's own annotation track rather than the behavior.
    pub annotation: bool,
}

/// The clip a state plays.
#[derive(Debug, Clone, PartialEq)]
pub struct StateClip {
    /// The clip packfile, relative to the project directory.
    pub animation: String,
    pub mode: ClipMode,
    pub playback_speed: f32,
    pub start_time: f32,
    /// Where a [`ClipMode::UserControlled`] clip is held, as a fraction of its length.
    pub user_fraction: f32,
    pub triggers: Vec<ClipTrigger>,
}

/// A state's (or the machine's wildcard) transition on an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    /// Index into [`BehaviorGraph::events`]; negative for none.
    pub event: i32,
    pub to_state: i32,
    pub priority: i16,
    pub flags: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub id: i32,
    pub name: String,
    pub transitions: Vec<Transition>,
    /// `None` when the state's generator is not a plain clip (a blend tree or a
    /// nested machine, which are not executed).
    pub clip: Option<StateClip>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateMachine {
    pub start_state: i32,
    pub states: Vec<State>,
    /// Transitions taken from any state.
    pub wildcard: Vec<Transition>,
}

/// A decoded behavior graph.
#[derive(Debug, Clone, PartialEq)]
pub struct BehaviorGraph {
    pub events: Vec<String>,
    /// The graph's root state machine, when it has one.
    pub machine: Option<StateMachine>,
}

impl BehaviorGraph {
    /// The id of the event named `name` (case-insensitive).
    pub fn event_id(&self, name: &str) -> Option<i32> {
        self.events
            .iter()
            .position(|event| event.eq_ignore_ascii_case(name))
            .map(|index| index as i32)
    }

    pub fn event_name(&self, id: i32) -> Option<&str> {
        usize::try_from(id).ok().and_then(|index| self.events.get(index)).map(String::as_str)
    }
}

/// A decoded `hkbCharacterData` + string data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    /// The skeleton packfile, relative to the project directory.
    pub rig: String,
    /// The behavior packfile, relative to the project directory.
    pub behavior: String,
    pub animations: Vec<String>,
}

fn require_64_bit(file: &Packfile) -> Result<()> {
    if file.pointer_size() != 8 {
        return Err(HkxError::UnsupportedLayout(
            "behavior data is decoded for 64-bit (Skyrim SE) packfiles only",
        ));
    }
    Ok(())
}

/// An `hkArray`'s data offset and length at `field`.
fn array(file: &Packfile, field: usize, cap: usize, label: &'static str) -> Result<(usize, usize)> {
    let len = file.u32(field + 8, label)? as usize;
    if len == 0 {
        return Ok((0, 0));
    }
    if len > cap {
        return Err(HkxError::InvalidData("implausible behavior array length"));
    }
    let data = file
        .local_target(field)
        .ok_or(HkxError::InvalidData("behavior array without data"))?;
    Ok((data, len))
}

/// The string an `hkStringPtr` at `field` points to; empty when null.
fn string(file: &Packfile, field: usize, label: &'static str) -> Result<String> {
    Ok(match file.local_target(field) {
        Some(target) => file.cstr(target, label)?.to_owned(),
        None => String::new(),
    })
}

fn string_array(file: &Packfile, field: usize, cap: usize, label: &'static str) -> Result<Vec<String>> {
    let (data, len) = array(file, field, cap, label)?;
    (0..len).map(|index| string(file, data + index * 8, label)).collect()
}

fn read_transitions(file: &Packfile, array_object: usize) -> Result<Vec<Transition>> {
    let (data, len) = array(file, array_object + TRANSITIONS_ARRAY, MAX_TRANSITIONS, "transitions")?;
    (0..len)
        .map(|index| {
            let element = data + index * TRANSITION_STRIDE;
            Ok(Transition {
                event: file.u32(element + TRANSITION_EVENT_ID, "transition event")? as i32,
                to_state: file.u32(element + TRANSITION_TO_STATE, "transition target")? as i32,
                priority: file.u16(element + TRANSITION_PRIORITY, "transition priority")? as i16,
                flags: file.u16(element + TRANSITION_FLAGS, "transition flags")?,
            })
        })
        .collect()
}

fn read_clip(file: &Packfile, clip: usize) -> Result<StateClip> {
    let mut triggers = Vec::new();
    if let Some(array_object) = file.object_pointer(clip + CLIP_TRIGGERS) {
        let (data, len) = array(file, array_object + TRIGGERS_ARRAY, MAX_TRIGGERS, "clip triggers")?;
        for index in 0..len {
            let element = data + index * TRIGGER_STRIDE;
            triggers.push(ClipTrigger {
                local_time: file.f32(element + TRIGGER_LOCAL_TIME, "trigger time")?,
                event: file.u32(element + TRIGGER_EVENT_ID, "trigger event")? as i32,
                relative_to_end: file.u8(element + TRIGGER_RELATIVE_TO_END, "trigger end flag")? != 0,
                annotation: file.u8(element + TRIGGER_IS_ANNOTATION, "trigger annotation flag")? != 0,
            });
        }
    }
    let mode = match file.u8(clip + CLIP_MODE, "clip mode")? {
        0 => ClipMode::SinglePlay,
        1 => ClipMode::Looping,
        2 => ClipMode::UserControlled,
        3 => ClipMode::PingPong,
        _ => return Err(HkxError::InvalidData("unknown clip playback mode")),
    };
    Ok(StateClip {
        animation: string(file, clip + CLIP_ANIMATION_NAME, "clip animation name")?,
        mode,
        playback_speed: file.f32(clip + CLIP_PLAYBACK_SPEED, "clip playback speed")?,
        start_time: file.f32(clip + CLIP_START_TIME, "clip start time")?,
        user_fraction: file.f32(clip + CLIP_USER_FRACTION, "clip user fraction")?,
        triggers,
    })
}

fn read_state(file: &Packfile, state: usize) -> Result<State> {
    let transitions = match file.object_pointer(state + STATE_TRANSITIONS) {
        Some(array_object) => read_transitions(file, array_object)?,
        None => Vec::new(),
    };
    let clip = match file.object_pointer(state + STATE_GENERATOR) {
        Some(generator) if file.class_at(generator) == Some("hkbClipGenerator") => {
            Some(read_clip(file, generator)?)
        }
        _ => None,
    };
    Ok(State {
        id: file.u32(state + STATE_ID, "state id")? as i32,
        name: string(file, state + STATE_NAME, "state name")?,
        transitions,
        clip,
    })
}

fn read_machine(file: &Packfile, machine: usize) -> Result<StateMachine> {
    let (data, len) = array(file, machine + MACHINE_STATES, MAX_STATES, "states")?;
    let mut states = Vec::with_capacity(len);
    for index in 0..len {
        let state = file
            .object_pointer(data + index * 8)
            .ok_or(HkxError::InvalidData("state pointer without a target"))?;
        states.push(read_state(file, state)?);
    }
    let wildcard = match file.object_pointer(machine + MACHINE_WILDCARD) {
        Some(array_object) => read_transitions(file, array_object)?,
        None => Vec::new(),
    };
    Ok(StateMachine {
        start_state: file.u32(machine + MACHINE_START_STATE, "start state")? as i32,
        states,
        wildcard,
    })
}

/// Decode a behavior-graph packfile (`Behaviors\\*.hkx`).
pub fn decode_behavior_graph(bytes: &[u8]) -> Result<BehaviorGraph> {
    let file = Packfile::parse(bytes)?;
    require_64_bit(&file)?;
    let graph = file.object("hkbBehaviorGraph")?;
    let string_data = file
        .object_pointer(graph + GRAPH_DATA)
        .and_then(|data| file.object_pointer(data + GRAPH_DATA_STRING_DATA))
        .or_else(|| file.objects_of("hkbBehaviorGraphStringData").first().copied())
        .ok_or(HkxError::MissingClass("hkbBehaviorGraphStringData"))?;
    let events = string_array(&file, string_data + STRING_DATA_EVENT_NAMES, MAX_EVENTS, "event names")?;
    let machine = match file.object_pointer(graph + GRAPH_ROOT_GENERATOR) {
        Some(root) if file.class_at(root) == Some("hkbStateMachine") => Some(read_machine(&file, root)?),
        _ => match file.objects_of("hkbStateMachine").first() {
            Some(&machine) => Some(read_machine(&file, machine)?),
            None => None,
        },
    };
    Ok(BehaviorGraph { events, machine })
}

/// Decode a character packfile (`Characters\\*.hkx`).
pub fn decode_character(bytes: &[u8]) -> Result<Character> {
    let file = Packfile::parse(bytes)?;
    require_64_bit(&file)?;
    let strings = file.object("hkbCharacterStringData")?;
    Ok(Character {
        rig: string(&file, strings + CHARACTER_RIG_NAME, "rig name")?,
        behavior: string(&file, strings + CHARACTER_BEHAVIOR_FILE, "behavior file")?,
        animations: string_array(&file, strings + CHARACTER_ANIMATION_NAMES, MAX_EVENTS, "animation names")?,
    })
}

/// The character files a project packfile lists (`hkbProjectStringData`).
pub fn decode_project(bytes: &[u8]) -> Result<Vec<String>> {
    let file = Packfile::parse(bytes)?;
    require_64_bit(&file)?;
    let strings = file.object("hkbProjectStringData")?;
    string_array(&file, strings + PROJECT_CHARACTER_FILENAMES, 64, "character filenames")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn animations_archive() -> Option<byroredux_bsa::BsaArchive> {
        let env = byroredux_plugin::esm::test_paths::SKYRIM_SE_ENV;
        let dir = std::env::var(env)
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| byroredux_plugin::esm::test_paths::SKYRIM_SE_DEFAULT.to_owned());
        let path = std::path::Path::new(&dir).join("Skyrim - Animations.bsa");
        if !path.is_file() {
            if std::env::var("BYROREDUX_REQUIRE_GAME_DATA").is_ok_and(|v| v != "0") {
                panic!("BYROREDUX_REQUIRE_GAME_DATA is set, but {path:?} is not a file");
            }
            eprintln!("SKIP: {} not found (no game data?)", path.display());
            return None;
        }
        Some(byroredux_bsa::BsaArchive::open(path).unwrap())
    }

    /// The Nordic ruins lever: a project -> character -> behavior chain whose
    /// graph is an eleven-state machine on named events, with a clip per state,
    /// end-of-clip trigger events and three wildcard "position" events.
    /// `cargo test -p byroredux-hkx -- --ignored`
    #[test]
    #[ignore = "needs Skyrim SE game data on disk"]
    fn the_nordic_lever_graph_decodes_end_to_end() {
        let Some(archive) = animations_archive() else { return };
        let base = r"meshes\dungeons\nordic\levers\ruinslever01\";
        let get = |name: &str| archive.extract(&format!("{base}{name}")).expect("extract");

        let characters = decode_project(&get("norlever01.hkx")).unwrap();
        assert_eq!(characters, [r"Characters\Character00.hkx"]);
        let character = decode_character(&get(&characters[0].to_ascii_lowercase())).unwrap();
        assert_eq!(character.behavior, r"Behaviors\Behavior00.hkx");
        assert_eq!(character.rig, r"CharacterAssets\NorLever01_Skeleton.hkx");
        assert_eq!(character.animations.len(), 6);

        let graph = decode_behavior_graph(&get(&character.behavior.to_ascii_lowercase())).unwrap();
        assert_eq!(graph.events.len(), 28);
        assert_eq!(graph.event_id("PullDown"), Some(0));
        let machine = graph.machine.as_ref().expect("state machine");
        assert_eq!(machine.start_state, 1);
        assert_eq!(machine.states.len(), 11);
        assert_eq!(machine.wildcard.len(), 3);

        let mid = machine.states.iter().find(|s| s.name == "MidPosition").unwrap();
        let pull_down = graph.event_id("PullDown").unwrap();
        let to_pulling = mid.transitions.iter().find(|t| t.event == pull_down).unwrap().to_state;
        let pulling = machine.states.iter().find(|s| s.id == to_pulling).unwrap();
        assert_eq!(pulling.name, "MidToPulled");
        let clip = pulling.clip.as_ref().unwrap();
        assert_eq!(clip.animation, r"Animations\NorLever01_PullDown.hkx");
        assert_eq!(clip.mode, ClipMode::SinglePlay);
        assert_eq!(clip.playback_speed, 2.0);
        let pulled = graph.event_id("Pulled").unwrap();
        assert!(clip.triggers.iter().any(|t| t.event == pulled && t.relative_to_end));
    }
}
