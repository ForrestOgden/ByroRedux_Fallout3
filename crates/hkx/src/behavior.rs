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

// ── 64-bit field offsets ──────────────────────────────────────────────

/// One Havok version's class layouts. Skyrim SE's are `hk_2010.2.0-r1`, Fallout 4's
/// `hk_2014.1.0-r1`; both are 64-bit MSVC. The 2014 `hkbNode` grew by 0x40 bytes, which
/// moves every member of the generator classes (graph, state machine, blender) by
/// that much; `hkbBindable`-derived classes (state info, blender child) and the
/// plain data classes did not move. The clip generator moved by 0x48, then 0x4C
/// after its trigger array (one more 4-byte member before the start time).
struct Layout {
    /// `hkbBehaviorGraphStringData::m_eventNames` (`hkArray<hkStringPtr>`).
    string_data_event_names: usize,
    /// `hkbBehaviorGraph::m_rootGenerator` / `m_data`.
    graph_root_generator: usize,
    graph_data: usize,
    /// `hkbBehaviorGraphData::m_stringData`.
    graph_data_string_data: usize,
    /// `hkbStateMachine::m_startStateId`, `m_states`, `m_wildcardTransitions`.
    machine_start_state: usize,
    machine_states: usize,
    machine_wildcard: usize,
    /// `hkbStateMachine::StateInfo` fields.
    state_transitions: usize,
    state_generator: usize,
    state_name: usize,
    state_id: usize,
    /// `hkbStateMachineTransitionInfoArray::m_transitions` and its element layout.
    transitions_array: usize,
    transition_stride: usize,
    transition_event_id: usize,
    transition_to_state: usize,
    transition_priority: usize,
    transition_flags: usize,
    /// `hkbBlenderGenerator::m_children` and `hkbBlenderGeneratorChild::m_generator`.
    blender_children: usize,
    blender_child_generator: usize,
    /// `BGSGamebryoSequenceGenerator::m_pSequence` (`hkStringPtr`).
    sequence_name: usize,
    /// The inner generator of the wrapper generators (`hkbModifierGenerator`,
    /// `BSiStateTaggingGenerator`, `DynamicAnimationTaggingGenerator`,
    /// `BSOffsetAnimationGenerator`, `BSCyclicBlendTransitionGenerator`). Only
    /// measured for Havok 2014; `None` leaves them unfollowed.
    wrapped_generator: Option<usize>,
    /// `hkbManualSelectorGenerator::m_generators` and
    /// `hkbBehaviorReferenceGenerator::m_behaviorName` (a sub-behavior packfile).
    selector_generators: Option<usize>,
    reference_behavior_name: Option<usize>,
    /// `hkbClipGenerator` fields.
    clip_animation_name: usize,
    clip_triggers: usize,
    clip_start_time: usize,
    clip_playback_speed: usize,
    clip_user_fraction: usize,
    clip_mode: usize,
}

const LAYOUT_2010: Layout = Layout {
    string_data_event_names: 0x10,
    graph_root_generator: 0x80,
    graph_data: 0x88,
    graph_data_string_data: 0x78,
    machine_start_state: 0x68,
    machine_states: 0x90,
    machine_wildcard: 0xA0,
    state_transitions: 0x50,
    state_generator: 0x58,
    state_name: 0x60,
    state_id: 0x68,
    transitions_array: 0x10,
    transition_stride: 72,
    transition_event_id: 0x30,
    transition_to_state: 0x34,
    transition_priority: 0x40,
    transition_flags: 0x42,
    blender_children: 0x60,
    blender_child_generator: 0x30,
    sequence_name: 0x48,
    wrapped_generator: None,
    selector_generators: None,
    reference_behavior_name: None,
    clip_animation_name: 0x48,
    clip_triggers: 0x50,
    clip_start_time: 0x60,
    clip_playback_speed: 0x64,
    clip_user_fraction: 0x6C,
    clip_mode: 0x72,
};

/// Derived from the vanilla Fallout 4 graphs (`Fallout4 - Animations.ba2`) the same
/// way as the 2010 table (`examples/probe_behavior.rs`): `StateInfo` and
/// `BlenderGeneratorChild` kept their offsets, the generator classes moved by
/// 0x40, the transition element kept its 72 bytes (the array object just starts its
/// elements 0x10 later), and the clip generator moved by 0x48 up to its trigger array and
/// 0x4C after it. Fallout 4 has no `BGSGamebryoSequenceGenerator`.
const LAYOUT_2014: Layout = Layout {
    string_data_event_names: 0x10,
    graph_root_generator: 0xC0,
    graph_data: 0xC8,
    graph_data_string_data: 0x68,
    machine_start_state: 0xA8,
    machine_states: 0xD0,
    machine_wildcard: 0xE0,
    state_transitions: 0x50,
    state_generator: 0x58,
    state_name: 0x60,
    state_id: 0x68,
    transitions_array: 0x10,
    transition_stride: 72,
    transition_event_id: 0x30,
    transition_to_state: 0x34,
    transition_priority: 0x40,
    transition_flags: 0x42,
    blender_children: 0xA0,
    blender_child_generator: 0x30,
    sequence_name: 0,
    wrapped_generator: Some(0x90),
    selector_generators: Some(0x88),
    reference_behavior_name: Some(0x88),
    clip_animation_name: 0x90,
    clip_triggers: 0x98,
    clip_start_time: 0xAC,
    clip_playback_speed: 0xB0,
    clip_user_fraction: 0xB8,
    clip_mode: 0xBE,
};

fn layout(file: &Packfile) -> &'static Layout {
    if file.version() == 11 {
        &LAYOUT_2014
    } else {
        &LAYOUT_2010
    }
}

/// Nested generators are followed this deep (a state machine in a blender in a
/// state machine ...).
const MAX_GENERATOR_DEPTH: usize = 32;
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
    /// The clip when the state's generator is a clip generator; `None` for any
    /// other kind (see [`Self::content`] for what a blender or nested machine holds).
    pub clip: Option<StateClip>,
    /// What a blender or nested-machine generator holds, flattened.
    pub content: StateContent,
}

/// What a state's non-clip generator holds: a blender's children and nested state
/// machines are flattened into these lists (the blender's weights are not kept).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StateContent {
    /// `BGSGamebryoSequenceGenerator` sequence names: `NiControllerSequence`s in
    /// the object's own NIF.
    pub sequences: Vec<String>,
    /// Nested state machines. Events are answered inside them.
    pub machines: Vec<StateMachine>,
    /// Further clips of a blender (the first is the state's own `clip`).
    pub blended_clips: Vec<StateClip>,
    /// Sub-behavior packfiles an `hkbBehaviorReferenceGenerator` names (Fallout 4:
    /// a state that runs another graph, e.g. `Behaviors\\DialogueBehavior.hkx`).
    pub behavior_references: Vec<String>,
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

/// How many behavior references [`BehaviorGraph::start_clip`] follows before it gives
/// up (a mod graph that refers to itself cannot hang it).
const MAX_REFERENCE_HOPS: usize = 8;

impl StateMachine {
    /// The clip this machine plays on entry: the start state's own clip, else the
    /// first of its nested machines that yields one, else the first sub-behavior it
    /// refers to. `resolve` loads a referenced behavior by the path it names.
    fn start_clip(&self, resolve: &mut dyn FnMut(&str) -> Option<BehaviorGraph>, hops: &mut usize) -> Option<StateClip> {
        let state = self.states.iter().find(|s| s.id == self.start_state).or(self.states.first())?;
        if let Some(clip) = &state.clip {
            return Some(clip.clone());
        }
        for machine in &state.content.machines {
            if let Some(clip) = machine.start_clip(resolve, hops) {
                return Some(clip);
            }
        }
        for reference in &state.content.behavior_references {
            if *hops >= MAX_REFERENCE_HOPS {
                return None;
            }
            *hops += 1;
            if let Some(clip) = resolve(reference).and_then(|graph| graph.machine?.start_clip(resolve, hops)) {
                return Some(clip);
            }
        }
        state.content.blended_clips.first().cloned()
    }
}

/// How many sub-behaviors [`BehaviorGraph::inline_references`] pulls in before it stops
/// (a graph can refer to the same few behaviors from many states).
const MAX_INLINED_REFERENCES: usize = 64;

/// Renumber the events `machine` names (transitions and clip triggers) through `map`.
fn remap_machine_events(machine: &mut StateMachine, map: &[i32]) {
    let remap = |event: i32| usize::try_from(event).ok().and_then(|index| map.get(index)).copied().unwrap_or(event);
    for transition in machine.wildcard.iter_mut() {
        transition.event = remap(transition.event);
    }
    for state in &mut machine.states {
        for transition in &mut state.transitions {
            transition.event = remap(transition.event);
        }
        for clip in state.clip.iter_mut().chain(state.content.blended_clips.iter_mut()) {
            for trigger in &mut clip.triggers {
                trigger.event = remap(trigger.event);
            }
        }
        for inner in &mut state.content.machines {
            remap_machine_events(inner, map);
        }
    }
}

impl BehaviorGraph {
    /// The id of `name` in this graph's events, adding it when it has none: behaviors
    /// share event *names*, so a referenced graph's events join its parent's table.
    fn intern_event(&mut self, name: &str) -> i32 {
        match self.event_id(name) {
            Some(id) => id,
            None => {
                self.events.push(name.to_owned());
                (self.events.len() - 1) as i32
            }
        }
    }

    /// Replace every state's sub-behavior reference with the referenced graph's root
    /// machine, nested in that state (a state that refers to a behavior runs it as a
    /// machine of its own): the machine's events are renumbered into this graph's
    /// table by name, so an event sent to the parent reaches the sub-behavior's
    /// transitions. `resolve` loads a referenced behavior by the path it names; a
    /// reference it cannot load is left as it was.
    pub fn inline_references(&mut self, resolve: &mut dyn FnMut(&str) -> Option<BehaviorGraph>) {
        let Some(mut machine) = self.machine.take() else { return };
        let mut budget = MAX_INLINED_REFERENCES;
        self.inline_machine(&mut machine, resolve, &mut budget);
        self.machine = Some(machine);
    }

    fn inline_machine(
        &mut self,
        machine: &mut StateMachine,
        resolve: &mut dyn FnMut(&str) -> Option<BehaviorGraph>,
        budget: &mut usize,
    ) {
        for state in &mut machine.states {
            let references = std::mem::take(&mut state.content.behavior_references);
            for reference in references {
                if *budget == 0 {
                    state.content.behavior_references.push(reference);
                    continue;
                }
                *budget -= 1;
                let Some(mut inner) = resolve(&reference) else {
                    state.content.behavior_references.push(reference);
                    continue;
                };
                // The referenced graph's own references first (its events are its own).
                inner.inline_references_with(resolve, budget);
                let Some(mut inner_machine) = inner.machine.take() else { continue };
                let map: Vec<i32> = inner.events.iter().map(|name| self.intern_event(name)).collect();
                remap_machine_events(&mut inner_machine, &map);
                state.content.machines.push(inner_machine);
            }
            for inner in &mut state.content.machines {
                self.inline_machine(inner, resolve, budget);
            }
        }
    }

    fn inline_references_with(&mut self, resolve: &mut dyn FnMut(&str) -> Option<BehaviorGraph>, budget: &mut usize) {
        let Some(mut machine) = self.machine.take() else { return };
        self.inline_machine(&mut machine, resolve, budget);
        self.machine = Some(machine);
    }

    /// The clip an actor standing idle in this graph plays: following the start
    /// state of every machine down (and into the sub-behaviors a state refers to,
    /// loaded through `resolve`) to the first clip. Fallout 4's `MTBehavior` reaches
    /// `Animations\PoseA_Idle1.hkt` through its `MTIdles` state's reference to
    /// `DialogueBehavior.hkx`. Variables and events are not evaluated: this is the
    /// pose the graph starts in.
    pub fn start_clip(&self, resolve: &mut dyn FnMut(&str) -> Option<BehaviorGraph>) -> Option<StateClip> {
        self.machine.as_ref()?.start_clip(resolve, &mut 0)
    }

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
    let l = layout(file);
    let (data, len) = array(file, array_object + l.transitions_array, MAX_TRANSITIONS, "transitions")?;
    (0..len)
        .map(|index| {
            let element = data + index * l.transition_stride;
            Ok(Transition {
                event: file.u32(element + l.transition_event_id, "transition event")? as i32,
                to_state: file.u32(element + l.transition_to_state, "transition target")? as i32,
                priority: file.u16(element + l.transition_priority, "transition priority")? as i16,
                flags: file.u16(element + l.transition_flags, "transition flags")?,
            })
        })
        .collect()
}

fn read_clip(file: &Packfile, clip: usize) -> Result<StateClip> {
    let l = layout(file);
    let mut triggers = Vec::new();
    if let Some(array_object) = file.object_pointer(clip + l.clip_triggers) {
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
    let mode = match file.u8(clip + l.clip_mode, "clip mode")? {
        0 => ClipMode::SinglePlay,
        1 => ClipMode::Looping,
        2 => ClipMode::UserControlled,
        3 => ClipMode::PingPong,
        _ => return Err(HkxError::InvalidData("unknown clip playback mode")),
    };
    Ok(StateClip {
        animation: string(file, clip + l.clip_animation_name, "clip animation name")?,
        mode,
        playback_speed: file.f32(clip + l.clip_playback_speed, "clip playback speed")?,
        start_time: file.f32(clip + l.clip_start_time, "clip start time")?,
        user_fraction: file.f32(clip + l.clip_user_fraction, "clip user fraction")?,
        triggers,
    })
}

/// Read the generator at `generator` into `state`: a clip, a Gamebryo sequence, a
/// nested machine, or a blender over such children.
fn read_generator(
    file: &Packfile,
    generator: usize,
    clip: &mut Option<StateClip>,
    content: &mut StateContent,
    depth: usize,
) -> Result<()> {
    if depth > MAX_GENERATOR_DEPTH {
        return Ok(());
    }
    let l = layout(file);
    match file.class_at(generator) {
        Some("hkbClipGenerator") => {
            let read = read_clip(file, generator)?;
            if clip.is_none() {
                *clip = Some(read);
            } else {
                content.blended_clips.push(read);
            }
        }
        Some("BGSGamebryoSequenceGenerator") => {
            let name = string(file, generator + l.sequence_name, "sequence name")?;
            if !name.is_empty() {
                content.sequences.push(name);
            }
        }
        Some("hkbStateMachine") => content.machines.push(read_machine_at(file, generator, depth + 1)?),
        Some(
            "hkbModifierGenerator"
            | "BSiStateTaggingGenerator"
            | "DynamicAnimationTaggingGenerator"
            | "BSOffsetAnimationGenerator"
            | "BSCyclicBlendTransitionGenerator",
        ) => {
            if let Some(field) = l.wrapped_generator {
                if let Some(inner) = file.object_pointer(generator + field) {
                    read_generator(file, inner, clip, content, depth + 1)?;
                }
            }
        }
        Some("hkbManualSelectorGenerator") => {
            if let Some(field) = l.selector_generators {
                let (data, len) = array(file, generator + field, MAX_STATES, "selector generators")?;
                for index in 0..len {
                    if let Some(inner) = file.object_pointer(data + index * 8) {
                        read_generator(file, inner, clip, content, depth + 1)?;
                    }
                }
            }
        }
        Some("hkbBehaviorReferenceGenerator") => {
            if let Some(field) = l.reference_behavior_name {
                let name = string(file, generator + field, "behavior reference")?;
                if !name.is_empty() {
                    content.behavior_references.push(name);
                }
            }
        }
        Some("hkbBlenderGenerator") => {
            let (data, len) = array(file, generator + l.blender_children, MAX_STATES, "blender children")?;
            for index in 0..len {
                let Some(child) = file.object_pointer(data + index * 8) else { continue };
                let Some(inner) = file.object_pointer(child + l.blender_child_generator) else { continue };
                read_generator(file, inner, clip, content, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn read_state(file: &Packfile, state: usize, depth: usize) -> Result<State> {
    let l = layout(file);
    let transitions = match file.object_pointer(state + l.state_transitions) {
        Some(array_object) => read_transitions(file, array_object)?,
        None => Vec::new(),
    };
    let mut clip = None;
    let mut content = StateContent::default();
    if let Some(generator) = file.object_pointer(state + l.state_generator) {
        read_generator(file, generator, &mut clip, &mut content, depth)?;
    }
    Ok(State {
        id: file.u32(state + l.state_id, "state id")? as i32,
        name: string(file, state + l.state_name, "state name")?,
        transitions,
        clip,
        content,
    })
}

fn read_machine(file: &Packfile, machine: usize) -> Result<StateMachine> {
    read_machine_at(file, machine, 0)
}

fn read_machine_at(file: &Packfile, machine: usize, depth: usize) -> Result<StateMachine> {
    let l = layout(file);
    let (data, len) = array(file, machine + l.machine_states, MAX_STATES, "states")?;
    let mut states = Vec::with_capacity(len);
    for index in 0..len {
        let state = file
            .object_pointer(data + index * 8)
            .ok_or(HkxError::InvalidData("state pointer without a target"))?;
        states.push(read_state(file, state, depth)?);
    }
    let wildcard = match file.object_pointer(machine + l.machine_wildcard) {
        Some(array_object) => read_transitions(file, array_object)?,
        None => Vec::new(),
    };
    Ok(StateMachine {
        start_state: file.u32(machine + l.machine_start_state, "start state")? as i32,
        states,
        wildcard,
    })
}

/// Decode a behavior-graph packfile (`Behaviors\\*.hkx`).
pub fn decode_behavior_graph(bytes: &[u8]) -> Result<BehaviorGraph> {
    let file = Packfile::parse(bytes)?;
    require_64_bit(&file)?;
    let l = layout(&file);
    let graph = file.object("hkbBehaviorGraph")?;
    let string_data = file
        .object_pointer(graph + l.graph_data)
        .and_then(|data| file.object_pointer(data + l.graph_data_string_data))
        .or_else(|| file.objects_of("hkbBehaviorGraphStringData").first().copied())
        .ok_or(HkxError::MissingClass("hkbBehaviorGraphStringData"))?;
    let events = string_array(&file, string_data + l.string_data_event_names, MAX_EVENTS, "event names")?;
    let machine = match file.object_pointer(graph + l.graph_root_generator) {
        Some(root) if file.class_at(root) == Some("hkbStateMachine") => Some(read_machine(&file, root)?),
        _ => match file.objects_of("hkbStateMachine").first() {
            Some(&machine) => Some(read_machine(&file, machine)?),
            None => None,
        },
    };
    Ok(BehaviorGraph { events, machine })
}

/// A line-per-slot dump of every object of `class_name` in a packfile: each
/// 8-byte slot as raw hex, plus what it points at (an object's class, or a local
/// string). The evidence for a class layout, where the Havok SDK on disk has none
/// (`examples/graph_dump.rs --objects`).
#[doc(hidden)]
pub fn dump_objects(bytes: &[u8], class_name: &str, bytes_per_object: usize) -> Result<Vec<String>> {
    let file = Packfile::parse(bytes)?;
    let mut lines = Vec::new();
    for object in file.objects_of(class_name) {
        lines.push(format!("{class_name} @ {object:#x}"));
        for slot in (0..bytes_per_object).step_by(8) {
            let at = object + slot;
            let raw = file.u32(at, "slot")? as u64 | (file.u32(at + 4, "slot")? as u64) << 32;
            let mut note = String::new();
            if let Some(target) = file.object_pointer(at) {
                note = format!("-> object {} @ {target:#x}", file.class_at(target).unwrap_or("?"));
            } else if let Some(target) = file.local_target(at) {
                note = match file.cstr(target, "dump string") {
                    Ok(text) if !text.is_empty() && text.len() < 80 && text.is_ascii() => format!("-> local {target:#x} {text:?}"),
                    _ => format!("-> local {target:#x}"),
                };
            }
            lines.push(format!("  +{slot:#04x}  {raw:#018x}  {note}"));
        }
    }
    Ok(lines)
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

    /// The swinging blade: its single root state holds a blender over two nested
    /// machines - one swinging the `Trigger` clip (held, looped or played once,
    /// raising `Apex` / `InWin*` / a sound), one naming Gamebryo sequences.
    /// `cargo test -p byroredux-hkx -- --ignored`
    #[test]
    #[ignore = "needs Skyrim SE game data on disk"]
    fn the_swinging_blade_graph_decodes_its_nested_machines() {
        let Some(archive) = animations_archive() else { return };
        let bytes = archive.extract(r"meshes\traps\swingingblade\behaviors\behavior00.hkx").expect("extract");
        let graph = decode_behavior_graph(&bytes).unwrap();
        let root = graph.machine.as_ref().expect("machine");
        assert_eq!(root.states.len(), 1);
        let state = &root.states[0];
        assert!(state.clip.is_none(), "the root state holds a blender, not a clip");
        assert_eq!(state.content.machines.len(), 2);
        let (swing, sequences) = (&state.content.machines[0], &state.content.machines[1]);
        assert_eq!(swing.states.len(), 3);
        let modes: Vec<_> = swing.states.iter().map(|s| s.clip.as_ref().unwrap().mode).collect();
        assert_eq!(modes, [ClipMode::UserControlled, ClipMode::Looping, ClipMode::SinglePlay]);
        let single = graph.event_id("Single").unwrap();
        assert!(swing.states[0].transitions.iter().any(|t| t.event == single && t.to_state == 2));
        let names: Vec<_> = sequences.states.iter().flat_map(|s| s.content.sequences.iter().map(String::as_str)).collect();
        assert_eq!(names, ["playanim02", "playanim01", "playanim02"]);
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

    fn fo4_animations_archive() -> Option<byroredux_bsa::Ba2Archive> {
        let env = byroredux_plugin::esm::test_paths::FO4_ENV;
        let dir = std::env::var(env)
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| byroredux_plugin::esm::test_paths::FO4_DEFAULT.to_owned());
        let path = std::path::Path::new(&dir).join("Fallout4 - Animations.ba2");
        if !path.is_file() {
            if std::env::var("BYROREDUX_REQUIRE_GAME_DATA").is_ok_and(|v| v != "0") {
                panic!("BYROREDUX_REQUIRE_GAME_DATA is set, but {path:?} is not a file");
            }
            eprintln!("SKIP: {} not found (no game data?)", path.display());
            return None;
        }
        Some(byroredux_bsa::Ba2Archive::open(path).unwrap())
    }

    /// Fallout 4 ships Havok 2014 packfiles (file version 11): the LED display's
    /// two-state graph and the player's eight-way walk blend decode with the
    /// `LAYOUT_2014` offsets - a state machine on named events, a clip per state,
    /// a blender over clips.
    /// `cargo test -p byroredux-hkx -- --ignored`
    #[test]
    #[ignore = "needs Fallout 4 game data on disk"]
    fn fallout_4_havok_2014_graphs_decode() {
        let Some(archive) = fo4_animations_archive() else { return };
        let led = archive
            .extract(r"meshes\setdressing\animated\leddisplay01\behaviors\behavior00.hkx")
            .expect("extract");
        let graph = decode_behavior_graph(&led).unwrap();
        let machine = graph.machine.as_ref().expect("state machine");
        assert_eq!(machine.states.len(), 2);
        let first = &machine.states[0];
        assert_eq!((first.id, first.name.as_str()), (5, "StartLevel"));
        assert!(machine.states.iter().any(|s| s.id == machine.start_state));
        let clip = machine
            .states
            .iter()
            .find_map(|state| state.clip.as_ref().or(state.content.blended_clips.first()))
            .expect("a state clip");
        assert!(clip.animation.starts_with("Animations\\"), "{}", clip.animation);
        assert!(clip.playback_speed.is_finite() && clip.playback_speed > 0.0);

        let walk = archive
            .extract(r"meshes\actors\character\behaviors\locomotion_8wayblend.hkx")
            .expect("extract");
        let graph = decode_behavior_graph(&walk).unwrap();
        assert_eq!(graph.events.len(), 26);
        assert_eq!(graph.event_id("FootLeft"), Some(0));
        let blended: usize = graph
            .machine
            .iter()
            .flat_map(|m| &m.states)
            .map(|s| s.content.blended_clips.len() + usize::from(s.clip.is_some()))
            .sum();
        assert!(blended >= 8, "the eight walk directions are clips of one blender: {blended}");

        // The humanoid MT graph idles in `PoseA_Idle1`, reached through its
        // `MTIdles` state's reference to `DialogueBehavior.hkx`.
        let mt = decode_behavior_graph(&archive.extract(r"meshes\actors\character\behaviors\mtbehavior.hkx").unwrap()).unwrap();
        let clip = mt
            .start_clip(&mut |reference| {
                let path = format!(r"meshes\actors\character\{}", reference.to_ascii_lowercase());
                decode_behavior_graph(&archive.extract(&path).ok()?).ok()
            })
            .expect("the MT idle clip");
        assert_eq!(clip.animation, "Animations\\PoseA_Idle1.hkt");
        assert_eq!(clip.mode, ClipMode::Looping);
    }

    fn machine_of(start: i32, states: Vec<State>) -> StateMachine {
        StateMachine { start_state: start, states, wildcard: Vec::new() }
    }

    fn state_of(id: i32, clip: Option<&str>, content: StateContent) -> State {
        State {
            id,
            name: format!("S{id}"),
            transitions: Vec::new(),
            clip: clip.map(|animation| StateClip {
                animation: animation.to_owned(),
                mode: ClipMode::Looping,
                playback_speed: 1.0,
                start_time: 0.0,
                user_fraction: 0.0,
                triggers: Vec::new(),
            }),
            content,
        }
    }

    /// The idle pose is found by following start states down, then nested machines,
    /// then referenced sub-behaviors; a self-referencing graph ends instead of hanging.
    #[test]
    fn start_clip_follows_start_states_nested_machines_and_references() {
        let leaf = BehaviorGraph {
            events: Vec::new(),
            machine: Some(machine_of(
                1,
                vec![state_of(0, Some("Animations\\Other.hkt"), StateContent::default()), state_of(1, Some("Animations\\Idle.hkt"), StateContent::default())],
            )),
        };
        let root = BehaviorGraph {
            events: Vec::new(),
            machine: Some(machine_of(
                0,
                vec![state_of(
                    0,
                    None,
                    StateContent {
                        // The first nested machine has nothing; the reference is next.
                        machines: vec![machine_of(0, vec![state_of(0, None, StateContent::default())])],
                        behavior_references: vec!["Behaviors\\Leaf.hkx".into()],
                        ..Default::default()
                    },
                )],
            )),
        };
        let mut loaded = Vec::new();
        let clip = root
            .start_clip(&mut |path| {
                loaded.push(path.to_owned());
                Some(leaf.clone())
            })
            .expect("a clip");
        assert_eq!(clip.animation, "Animations\\Idle.hkt", "the start state, not the first state");
        assert_eq!(loaded, ["Behaviors\\Leaf.hkx"]);

        let looping = BehaviorGraph {
            events: Vec::new(),
            machine: Some(machine_of(
                0,
                vec![state_of(0, None, StateContent { behavior_references: vec!["Self.hkx".into()], ..Default::default() })],
            )),
        };
        let mut hops = 0;
        assert!(looping
            .start_clip(&mut |_| {
                hops += 1;
                Some(looping.clone())
            })
            .is_none());
        assert!(hops <= MAX_REFERENCE_HOPS);
    }

    /// A referenced behavior becomes a machine of the referring state; its events join
    /// the parent's table by name, so an event of the parent reaches its transitions.
    #[test]
    fn inline_references_nests_the_sub_behavior_and_renumbers_its_events_by_name() {
        let transition = |event, to_state| Transition { event, to_state, priority: 0, flags: 0 };
        let mut child_state = state_of(0, Some("Animations\\Idle.hkt"), StateContent::default());
        child_state.transitions = vec![transition(1, 1)]; // "C" in the child's own table
        let mut child_wildcard = Vec::new();
        child_wildcard.push(transition(0, 0)); // "B"
        let child = BehaviorGraph {
            events: vec!["B".into(), "C".into()],
            machine: Some(StateMachine {
                start_state: 0,
                states: vec![child_state, state_of(1, Some("Animations\\Other.hkt"), StateContent::default())],
                wildcard: child_wildcard,
            }),
        };
        let mut parent = BehaviorGraph {
            events: vec!["A".into(), "B".into()],
            machine: Some(machine_of(
                0,
                vec![state_of(0, None, StateContent { behavior_references: vec!["Child.hkx".into()], ..Default::default() })],
            )),
        };
        parent.inline_references(&mut |path| (path == "Child.hkx").then(|| child.clone()));
        assert_eq!(parent.events, ["A", "B", "C"], "the child's new event joins the table");
        let state = &parent.machine.as_ref().unwrap().states[0];
        assert!(state.content.behavior_references.is_empty());
        let inner = &state.content.machines[0];
        assert_eq!(inner.states[0].transitions[0].event, 2, "C is the parent's event 2");
        assert_eq!(inner.wildcard[0].event, 1, "B is the parent's existing event 1");
        // An unloadable reference stays a reference.
        let mut orphan = BehaviorGraph {
            events: Vec::new(),
            machine: Some(machine_of(
                0,
                vec![state_of(0, None, StateContent { behavior_references: vec!["Gone.hkx".into()], ..Default::default() })],
            )),
        };
        orphan.inline_references(&mut |_| None);
        assert_eq!(orphan.machine.unwrap().states[0].content.behavior_references, ["Gone.hkx"]);
    }
}
