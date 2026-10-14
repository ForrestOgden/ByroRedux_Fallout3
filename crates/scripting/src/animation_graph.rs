//! Animation graphs of placed objects (M47.2): `PlayAnimation`,
//! `PlayAnimationAndWait`, `RegisterForAnimationEvent` and
//! `SetAnimationVariable*`.
//!
//! `ObjectReference.PlayAnimation(event)` "sends the named event to the object's
//! animation graph" (CK *PlayAnimation*). Skyrim drives levers, traps, doors and
//! rigged props that way from a Havok behavior graph: a state machine whose
//! transitions fire on named events and whose states each play a clip. The NIF
//! names the graph (`BSBehaviorGraphExtraData`, stamped as [`BehaviorGraphRef`]),
//! the binary decodes it (`byroredux_hkx::decode_behavior_graph`) and attaches an
//! [`ObjectAnimationGraph`] with its clips registered in the animation registry.
//!
//! What runs: events switch the machine's state (the state's own transitions
//! first, then the machine's wildcard transitions; disabled transitions never
//! fire), the new state's clip starts on the object's `AnimationPlayer` (single
//! play holds the last pose, looping loops, a user-controlled clip is held at
//! its authored fraction), and the clip's trigger events fire on schedule and
//! drive further transitions — a lever that finishes `PullDown` raises `Pulled`
//! and settles in the "Pulled" state. Not run: blend trees and nested machines
//! (a state whose generator is not a plain clip has no clip), transition blends
//! (a transition is instantaneous), transition conditions and variables, and the
//! sound events (`SoundPlay.*`) the triggers carry.
//!
//! The CK's results apply: `PlayAnimation` returns true when the graph accepted
//! the event; `PlayAnimationAndWait` suspends until the named event is raised
//! (the schedule is deterministic, so the wait is computed up front) and "if the
//! event you send it doesn't exist, the function will never return" — here it
//! returns false at once instead; a reference with no graph answers the way the
//! CK documents a graph that is not loaded (`false`, no wait).
//! `RegisterForAnimationEvent` still answers false: delivering `OnAnimationEvent`
//! to registered scripts is not built. `UnregisterForAnimationEvent` and
//! `SetAnimationVariable*` have no graph variables to act on and do nothing.

use std::collections::HashMap;
use std::sync::Arc;

use byroredux_core::animation::AnimationPlayer;
use byroredux_core::ecs::sparse_set::SparseSetStorage;
use byroredux_core::ecs::storage::{Component, EntityId};
use byroredux_core::ecs::world::World;
use byroredux_hkx::{BehaviorGraph, ClipMode, StateClip, StateMachine, Transition, TRANSITION_FLAG_DISABLED};

/// The project file a placed object's NIF names (`BSBehaviorGraphExtraData`),
/// relative to `meshes\`. Stamped by the cell loader; the binary resolves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorGraphRef(pub Arc<str>);

impl Component for BehaviorGraphRef {
    type Storage = SparseSetStorage<Self>;
}

/// A registered clip of a graph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphClip {
    pub handle: u32,
    /// Seconds, at the clip's authored speed of 1.
    pub duration: f32,
}

/// The `NiControllerSequence`s of a placed object's own NIF, registered in the
/// animation registry (lower-cased name -> clip): what the graph's Gamebryo
/// sequence generators play. Stamped beside [`BehaviorGraphRef`].
#[derive(Debug, Clone, Default)]
pub struct GraphSequences(pub Arc<Vec<(String, GraphClip)>>);

impl Component for GraphSequences {
    type Storage = SparseSetStorage<Self>;
}

/// A decoded behavior graph with its clips registered. Shared by every object
/// that uses the project.
#[derive(Debug, Clone)]
pub struct LoadedGraph {
    pub graph: BehaviorGraph,
    /// `(lower-cased animation path, looping)` -> the registered clip.
    pub clips: HashMap<(String, bool), GraphClip>,
    /// Lower-cased `NiControllerSequence` name -> the registered clip, for the
    /// Gamebryo sequence generators (`BGSGamebryoSequenceGenerator`) a graph
    /// plays from the object's own NIF.
    pub sequences: HashMap<String, GraphClip>,
}

impl LoadedGraph {
    fn clip_for<'a>(&self, clip: &'a StateClip) -> Option<(&'a StateClip, GraphClip)> {
        let looping = clip.mode == ClipMode::Looping;
        let registered = *self.clips.get(&(clip.animation.to_ascii_lowercase(), looping))?;
        Some((clip, registered))
    }

    fn sequence(&self, name: &str) -> Option<GraphClip> {
        self.sequences.get(&name.to_ascii_lowercase()).copied()
    }
}

/// How the animation player must run after a state change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Playback {
    pub handle: u32,
    pub speed: f32,
    /// Clip time the playback starts at.
    pub start_time: f32,
    /// A user-controlled clip: held at this clip time instead of playing.
    pub hold_at: Option<f32>,
}

/// One state machine's current state, and the runs of the nested machines of that
/// state (one per `StateContent::machines`, each in its own current state).
#[derive(Debug, Clone, PartialEq)]
struct MachineRun {
    state: i32,
    nested: Vec<MachineRun>,
}

impl MachineRun {
    /// `machine` in `state`, its nested machines at their start states.
    fn enter(machine: &StateMachine, state: i32) -> Self {
        let nested = machine
            .states
            .iter()
            .find(|candidate| candidate.id == state)
            .map(|state| state.content.machines.iter().map(|inner| Self::enter(inner, inner.start_state)).collect())
            .unwrap_or_default();
        Self { state, nested }
    }
}

/// An event waiting for its time: raised by a clip trigger of the machine at
/// `path` (indices into the current states' nested machines, from the root).
#[derive(Debug, Clone)]
struct Scheduled {
    at: f32,
    event: i32,
    path: Vec<u16>,
    /// A looping clip's loop length in seconds: the trigger comes round again
    /// that long after it fires, until the machine leaves the state.
    period: Option<f32>,
}

/// What a state plays, in the order the runtime prefers: a Gamebryo sequence
/// (the motion of a graph that blends it with trigger-only clips), else a clip.
enum Leaf<'a> {
    Sequence(&'a str),
    Clip(&'a StateClip),
}

/// A placed object's running behavior graph.
#[derive(Debug, Clone)]
pub struct ObjectAnimationGraph {
    graph: Arc<LoadedGraph>,
    /// The entity whose subtree the clips animate (the placement root).
    pub root: EntityId,
    run: MachineRun,
    /// The graph's own clock, seconds.
    now: f32,
    /// Trigger events waiting for their time, in no particular order.
    scheduled: Vec<Scheduled>,
    /// Events the graph raised since [`take_raised`](Self::take_raised): the
    /// ones a script's accepted `PlayAnimation` sent and the ones a clip's
    /// triggers fired (a trigger raises its event whether or not a transition
    /// answers it, as an animation notify does). Runtime only.
    raised: Vec<String>,
}

impl Component for ObjectAnimationGraph {
    type Storage = SparseSetStorage<Self>;
}

/// Walk `path` down from the root machine to the machine and run it names.
fn locate<'a>(
    mut machine: &'a StateMachine,
    mut run: &'a MachineRun,
    path: &[u16],
) -> Option<(&'a StateMachine, &'a MachineRun)> {
    for &index in path {
        let state = machine.states.iter().find(|state| state.id == run.state)?;
        machine = state.content.machines.get(usize::from(index))?;
        run = run.nested.get(usize::from(index))?;
    }
    Some((machine, run))
}

/// `run`'s mutable counterpart: the run `path` names.
fn locate_mut<'a>(mut run: &'a mut MachineRun, path: &[u16]) -> Option<&'a mut MachineRun> {
    for &index in path {
        run = run.nested.get_mut(usize::from(index))?;
    }
    Some(run)
}

impl ObjectAnimationGraph {
    /// The graph in its start state, with the start state's clip not yet playing.
    pub fn new(graph: Arc<LoadedGraph>, root: EntityId) -> Option<Self> {
        let machine = graph.graph.machine.as_ref()?;
        let run = MachineRun::enter(machine, machine.start_state);
        Some(Self { graph, root, run, now: 0.0, scheduled: Vec::new(), raised: Vec::new() })
    }

    /// The root machine's current state.
    pub fn state(&self) -> i32 {
        self.run.state
    }

    pub fn graph(&self) -> &LoadedGraph {
        &self.graph
    }

    /// The events raised since the last call, oldest first.
    pub fn take_raised(&mut self) -> Vec<String> {
        std::mem::take(&mut self.raised)
    }

    /// Raise an event by name without a transition (tests).
    #[cfg(test)]
    pub(crate) fn push_raised_for_test(&mut self, name: &str) {
        self.raised.push(name.to_owned());
    }

    fn note_raised(&mut self, event: i32) {
        if let Some(name) = self.graph.graph.event_name(event) {
            self.raised.push(name.to_owned());
        }
    }

    /// The root machine and the leaves of the current configuration, each with
    /// the path of the machine whose current state holds it.
    fn leaves(&self) -> Vec<(Vec<u16>, Leaf<'_>)> {
        fn walk<'a>(
            machine: &'a StateMachine,
            run: &MachineRun,
            path: &mut Vec<u16>,
            out: &mut Vec<(Vec<u16>, Leaf<'a>)>,
        ) {
            let Some(state) = machine.states.iter().find(|state| state.id == run.state) else {
                return;
            };
            if let Some(clip) = &state.clip {
                out.push((path.clone(), Leaf::Clip(clip)));
            }
            for name in &state.content.sequences {
                out.push((path.clone(), Leaf::Sequence(name)));
            }
            for clip in &state.content.blended_clips {
                out.push((path.clone(), Leaf::Clip(clip)));
            }
            for (index, inner) in state.content.machines.iter().enumerate() {
                if let Some(inner_run) = run.nested.get(index) {
                    path.push(index as u16);
                    walk(inner, inner_run, path, out);
                    path.pop();
                }
            }
        }
        let mut out = Vec::new();
        if let Some(machine) = self.graph.graph.machine.as_ref() {
            walk(machine, &self.run, &mut Vec::new(), &mut out);
        }
        out
    }

    /// Schedule the trigger events of every clip in the state(s) `path`'s machine
    /// is in, and of the nested machines below, from the graph's current time.
    fn schedule_from(&mut self, path: &[u16]) {
        let graph = Arc::clone(&self.graph);
        let Some(root) = graph.graph.machine.as_ref() else { return };
        let Some((machine, run)) = locate(root, &self.run, path) else { return };
        let mut fresh = Vec::new();
        let now = self.now;
        fn walk(
            graph: &LoadedGraph,
            machine: &StateMachine,
            run: &MachineRun,
            path: &mut Vec<u16>,
            now: f32,
            out: &mut Vec<Scheduled>,
        ) {
            let Some(state) = machine.states.iter().find(|state| state.id == run.state) else {
                return;
            };
            for clip in state.clip.iter().chain(&state.content.blended_clips) {
                let Some((clip, registered)) = graph.clip_for(clip) else { continue };
                if clip.mode == ClipMode::UserControlled {
                    continue;
                }
                let speed = if clip.playback_speed.is_finite() && clip.playback_speed > 0.0 { clip.playback_speed } else { 1.0 };
                let period = (clip.mode == ClipMode::Looping && registered.duration > 0.0)
                    .then_some(registered.duration / speed);
                for trigger in &clip.triggers {
                    let at = if trigger.relative_to_end { registered.duration + trigger.local_time } else { trigger.local_time };
                    let real = (at - clip.start_time) / speed;
                    if real >= 0.0 && real.is_finite() {
                        out.push(Scheduled { at: now + real, event: trigger.event, path: path.clone(), period });
                    }
                }
            }
            for (index, inner) in state.content.machines.iter().enumerate() {
                if let Some(inner_run) = run.nested.get(index) {
                    path.push(index as u16);
                    walk(graph, inner, inner_run, path, now, out);
                    path.pop();
                }
            }
        }
        walk(&graph, machine, run, &mut path.to_vec(), now, &mut fresh);
        self.scheduled.extend(fresh);
    }

    /// The motion leaf of the current configuration - a Gamebryo sequence if there
    /// is one, else the first clip - with its owning machine's path and a name
    /// that tells it from the others.
    fn motion_leaf(&self) -> Option<(Vec<u16>, Leaf<'_>)> {
        // Only leaves that can play: a sequence the object's NIF has, a clip that
        // was registered (a graph can name sequences its NIF does not carry).
        let mut leaves: Vec<_> = self
            .leaves()
            .into_iter()
            .filter(|(_, leaf)| match leaf {
                Leaf::Sequence(name) => self.graph.sequence(name).is_some(),
                Leaf::Clip(clip) => self.graph.clip_for(clip).is_some(),
            })
            .collect();
        let index = leaves
            .iter()
            .position(|(_, leaf)| matches!(leaf, Leaf::Sequence(_)))
            .or_else(|| leaves.iter().position(|(_, leaf)| matches!(leaf, Leaf::Clip(_))))?;
        Some(leaves.swap_remove(index))
    }

    fn leaf_name(leaf: &Leaf<'_>) -> String {
        match leaf {
            Leaf::Sequence(name) => format!("seq:{}", name.to_ascii_lowercase()),
            Leaf::Clip(clip) => format!("clip:{}", clip.animation.to_ascii_lowercase()),
        }
    }

    /// How the motion leaf plays. With `after` set (an event just changed the
    /// machines at those paths, and `before` was the motion leaf's identity), it is
    /// described only if it is a different leaf now or sits under a machine that
    /// changed state; otherwise it is already running and nothing restarts.
    fn playback(&self, after: Option<(&[Vec<u16>], Option<(Vec<u16>, String)>)>) -> Option<Playback> {
        let (path, leaf) = self.motion_leaf()?;
        if let Some((changed, before)) = after {
            let same_leaf = before.as_ref().is_some_and(|(before_path, before_name)| {
                *before_path == path && *before_name == Self::leaf_name(&leaf)
            });
            if same_leaf && !changed.iter().any(|change| path.starts_with(change)) {
                return None;
            }
        }
        match leaf {
            Leaf::Sequence(name) => {
                let registered = self.graph.sequence(name)?;
                Some(Playback { handle: registered.handle, speed: 1.0, start_time: 0.0, hold_at: None })
            }
            Leaf::Clip(clip) => {
                let (clip, registered) = self.graph.clip_for(clip)?;
                let speed = if clip.playback_speed.is_finite() && clip.playback_speed > 0.0 { clip.playback_speed } else { 1.0 };
                if clip.mode == ClipMode::UserControlled {
                    return Some(Playback {
                        handle: registered.handle,
                        speed,
                        start_time: 0.0,
                        hold_at: Some(clip.user_fraction.clamp(0.0, 1.0) * registered.duration),
                    });
                }
                Some(Playback { handle: registered.handle, speed, start_time: clip.start_time, hold_at: None })
            }
        }
    }

    /// The playback of the current configuration, started fresh (every trigger
    /// of every clip in it scheduled from now).
    pub fn start(&mut self) -> Option<Playback> {
        self.scheduled.clear();
        self.schedule_from(&[]);
        self.playback(None)
    }

    /// The transition `event` takes in the machine at `machine`/`run`: the current
    /// state's own first, then the machine's wildcard; disabled ones never fire.
    fn transition_of(machine: &StateMachine, run: &MachineRun, event: i32) -> Option<Transition> {
        if event < 0 {
            return None;
        }
        let state = machine.states.iter().find(|state| state.id == run.state)?;
        let usable = |t: &&Transition| t.event == event && t.flags & TRANSITION_FLAG_DISABLED == 0;
        state.transitions.iter().find(usable).or_else(|| machine.wildcard.iter().find(usable)).copied()
    }

    /// Give `event` to the machine at `path`: its own transition if it has one,
    /// else to the nested machines of its current state. The paths of the machines
    /// that changed state are appended to `changed`.
    fn dispatch(&mut self, path: &mut Vec<u16>, event: i32, changed: &mut Vec<Vec<u16>>) {
        let graph = Arc::clone(&self.graph);
        let Some(root) = graph.graph.machine.as_ref() else { return };
        let Some((machine, run)) = locate(root, &self.run, path) else { return };
        if let Some(transition) = Self::transition_of(machine, run, event) {
            let entered = MachineRun::enter(machine, transition.to_state);
            if let Some(slot) = locate_mut(&mut self.run, path) {
                *slot = entered;
            }
            changed.push(path.clone());
            return;
        }
        let inner = run.nested.len();
        for index in 0..inner {
            path.push(index as u16);
            self.dispatch(path, event, changed);
            path.pop();
        }
    }

    /// Raise `event`: `None` when no machine answers it; otherwise the playback to
    /// start (`Some(None)` when nothing needs restarting).
    pub fn send_event(&mut self, event: i32) -> Option<Option<Playback>> {
        let before = self.motion_leaf().map(|(path, leaf)| (path, Self::leaf_name(&leaf)));
        let mut changed = Vec::new();
        self.dispatch(&mut Vec::new(), event, &mut changed);
        if changed.is_empty() {
            return None;
        }
        for path in &changed {
            self.scheduled.retain(|scheduled| !scheduled.path.starts_with(path));
        }
        for path in &changed {
            self.schedule_from(path);
        }
        Some(self.playback(Some((&changed, before))))
    }

    /// A looping clip's trigger that just fired comes round again a loop later.
    fn rearm(&mut self, fired: &Scheduled) {
        if let Some(period) = fired.period {
            self.scheduled.push(Scheduled { at: fired.at + period, ..fired.clone() });
        }
    }

    /// The earliest scheduled event due by `limit`, removed.
    fn pop_due(&mut self, limit: f32) -> Option<Scheduled> {
        let index = self
            .scheduled
            .iter()
            .enumerate()
            .filter(|(_, scheduled)| scheduled.at <= limit)
            .min_by(|a, b| a.1.at.total_cmp(&b.1.at))
            .map(|(index, _)| index)?;
        Some(self.scheduled.remove(index))
    }

    /// Seconds from now until `target` is raised, following the machines through
    /// the transitions the schedule's own events take; `None` when it never is.
    pub fn seconds_until(&self, target: i32) -> Option<f32> {
        let mut sim = self.clone();
        let origin = sim.now;
        for _ in 0..64 {
            let due = sim.pop_due(f32::INFINITY)?;
            sim.now = sim.now.max(due.at);
            if due.event == target {
                return Some((due.at - origin).max(0.0));
            }
            sim.rearm(&due);
            sim.send_event(due.event);
        }
        None
    }

    /// Advance the schedule by `dt`: the events now due raise in order, and each
    /// that changes a state with something to restart yields the playback to start.
    pub fn step(&mut self, dt: f32) -> Vec<Playback> {
        let mut started = Vec::new();
        let limit = self.now + dt.max(0.0);
        while let Some(due) = self.pop_due(limit) {
            self.now = self.now.max(due.at);
            self.note_raised(due.event);
            self.rearm(&due);
            if let Some(Some(playback)) = self.send_event(due.event) {
                started.push(playback);
            }
        }
        self.now = limit;
        started
    }
}

/// Start `playback` on the graph's root: the object's `AnimationPlayer`.
pub fn apply_playback(world: &World, root: EntityId, playback: Playback) {
    let mut player = AnimationPlayer::new(playback.handle).with_root(root);
    player.speed = playback.speed;
    match playback.hold_at {
        Some(at) => {
            player.local_time = at;
            player.prev_time = at;
            player.playing = false;
        }
        None => {
            player.local_time = playback.start_time;
            player.prev_time = playback.start_time;
        }
    }
    if let Some(mut players) = world.query_mut::<AnimationPlayer>() {
        players.insert(root, player);
    }
}

/// `reference.PlayAnimation(event)`: whether the graph accepted the event.
pub fn play_animation(world: &World, entity: EntityId, event: &str) -> bool {
    send(world, entity, event).is_some()
}

/// `reference.PlayAnimationAndWait(event, waitEvent)`: the seconds until
/// `wait_event` is raised once `event` is accepted; `None` when the graph
/// rejects `event` or never raises `wait_event`.
pub fn play_animation_and_wait(world: &World, entity: EntityId, event: &str, wait_event: &str) -> Option<f32> {
    send(world, entity, event)?;
    let graphs = world.query::<ObjectAnimationGraph>()?;
    let graph = graphs.get(entity)?;
    let target = graph.graph.graph.event_id(wait_event)?;
    graph.seconds_until(target)
}

/// Whether `entity` runs a behavior graph.
pub fn has_graph(world: &World, entity: EntityId) -> bool {
    world.query::<ObjectAnimationGraph>().is_some_and(|graphs| graphs.get(entity).is_some())
}

/// `reference.WaitForAnimationEvent(event)`: the seconds until the graph's own
/// schedule raises `event`; `None` when the object has no graph or it never does.
pub fn wait_for_animation_event(world: &World, entity: EntityId, event: &str) -> Option<f32> {
    let graphs = world.query::<ObjectAnimationGraph>()?;
    let graph = graphs.get(entity)?;
    let target = graph.graph.graph.event_id(event)?;
    graph.seconds_until(target)
}

fn send(world: &World, entity: EntityId, event: &str) -> Option<()> {
    let (root, playback) = {
        let mut graphs = world.query_mut::<ObjectAnimationGraph>()?;
        let graph = graphs.get_mut(entity)?;
        let id = graph.graph.graph.event_id(event)?;
        let before = graph.state();
        let sent = graph.send_event(id);
        if sent.is_some() {
            graph.note_raised(id);
        }
        log::debug!(
            "PlayAnimation '{event}' on entity {entity}: {} (state {before} -> {})",
            if sent.is_some() { "accepted" } else { "rejected" },
            graph.state()
        );
        (graph.root, sent?)
    };
    if let Some(playback) = playback {
        apply_playback(world, root, playback);
    }
    Some(())
}

/// Step every object graph's trigger schedule (a clip's end raises its
/// completion event, which moves the machine on). Run each frame.
pub fn object_graph_step_system(world: &World, dt: f32) {
    let started: Vec<(EntityId, Playback)> = {
        let Some(mut graphs) = world.query_mut::<ObjectAnimationGraph>() else {
            return;
        };
        graphs
            .iter_mut()
            .filter_map(|(_, graph)| graph.step(dt).pop().map(|playback| (graph.root, playback)))
            .collect()
    };
    for (root, playback) in started {
        apply_playback(world, root, playback);
    }
}

/// `Actor.ChangeAnimArchetype(keyword)` (Fallout 4; CK: "adds the specified
/// AnimArchetype keyword to this actor", and "removes any existing AnimArchetype
/// keyword from the actor (sets actor back to 'Neutral')" when called with none).
/// The archetype picks which animation set the actor's graph runs, so it is
/// recorded per actor reference for the graph runtime to read; the authored
/// archetype is the base `NPC_`'s keyword. Saved.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "save", derive(serde::Serialize, serde::Deserialize))]
pub struct AnimArchetypeOverrides {
    set: std::collections::BTreeMap<u32, u32>,
}

impl byroredux_core::ecs::resource::Resource for AnimArchetypeOverrides {}

impl AnimArchetypeOverrides {
    /// Set `actor`'s archetype keyword; `0` is "back to Neutral".
    pub fn set(&mut self, actor: u32, keyword: u32) {
        self.set.insert(actor, keyword);
    }

    /// The archetype a script gave `actor`: `Some(0)` after a clear, `None`
    /// when no script changed it.
    pub fn get(&self, actor: u32) -> Option<u32> {
        self.set.get(&actor).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byroredux_hkx::{ClipTrigger, State};

    fn transition(event: i32, to_state: i32) -> Transition {
        Transition { event, to_state, priority: 0, flags: 0 }
    }

    fn clip(animation: &str, mode: ClipMode, speed: f32, triggers: Vec<ClipTrigger>) -> StateClip {
        StateClip {
            animation: animation.into(),
            mode,
            playback_speed: speed,
            start_time: 0.0,
            user_fraction: 0.0,
            triggers,
        }
    }

    /// A lever shaped like the Nordic one: events Down(0) / Pulled(1) / Up(2) /
    /// Rest(3, wildcard); state 0 rests (held pose), state 2 plays the 0.8 s
    /// pull-down clip at speed 2 and raises Pulled at its end, state 1 rests pulled.
    fn lever() -> ObjectAnimationGraph {
        let trigger = |event, local_time, relative_to_end| ClipTrigger { local_time, event, relative_to_end, annotation: false };
        let machine = StateMachine {
            start_state: 0,
            states: vec![
                State {
                    id: 0,
                    name: "Mid".into(),
                    transitions: vec![transition(0, 2)],
                    clip: Some(clip("a/pose.hkx", ClipMode::UserControlled, 1.0, Vec::new())),
                    content: Default::default(),
                },
                State {
                    id: 1,
                    name: "Pulled".into(),
                    transitions: vec![transition(2, 0)],
                    clip: Some(clip("a/pose.hkx", ClipMode::UserControlled, 1.0, Vec::new())),
                    content: Default::default(),
                },
                State {
                    id: 2,
                    name: "MidToPulled".into(),
                    transitions: vec![transition(1, 1), Transition { flags: TRANSITION_FLAG_DISABLED, ..transition(2, 0) }],
                    clip: Some(clip(
                        "a/pull.hkx",
                        ClipMode::SinglePlay,
                        2.0,
                        vec![trigger(1, 0.0, true)],
                    )),
                    content: Default::default(),
                },
            ],
            wildcard: vec![transition(3, 0)],
        };
        let graph = BehaviorGraph {
            events: vec!["Down".into(), "Pulled".into(), "Up".into(), "Rest".into()],
            machine: Some(machine),
        };
        let mut clips = HashMap::new();
        clips.insert(("a/pose.hkx".to_owned(), false), GraphClip { handle: 7, duration: 0.8 });
        clips.insert(("a/pull.hkx".to_owned(), false), GraphClip { handle: 9, duration: 0.8 });
        ObjectAnimationGraph::new(Arc::new(LoadedGraph { graph, clips, sequences: HashMap::new() }), 5).unwrap()
    }

    #[test]
    fn an_event_moves_the_machine_and_a_clip_end_raises_the_next() {
        let mut graph = lever();
        assert_eq!(graph.send_event(1), None, "Pulled has no transition in the start state");
        let playback = graph.send_event(0).expect("Down is taken").expect("the new state plays a clip");
        assert_eq!((playback.handle, playback.speed, playback.hold_at), (9, 2.0, None));
        assert_eq!(graph.state(), 2);
        // The clip is 0.8 s at speed 2, so 0.4 s of real time.
        assert!(graph.step(0.3).is_empty());
        assert_eq!(graph.state(), 2);
        let started = graph.step(0.2);
        assert_eq!(graph.state(), 1, "the end-of-clip Pulled event moved it on");
        assert_eq!(started[0].hold_at, Some(0.0), "settling into a held pose");
    }

    /// The swinging blade's shape: a root state holding two inner machines, one
    /// playing a looping trigger clip that raises `Apex` half a second in, the other
    /// idling until `Single` starts Gamebryo sequence `playanim01` and `Reset` ends it.
    fn blade() -> ObjectAnimationGraph {
        use byroredux_hkx::StateContent;
        let (single, reset, apex) = (0, 1, 2);
        let trigger_clip = clip(
            "a/trigger.hkx",
            ClipMode::Looping,
            1.0,
            vec![ClipTrigger { local_time: 0.5, event: apex, relative_to_end: false, annotation: true }],
        );
        let inner_trigger = StateMachine {
            start_state: 0,
            states: vec![State {
                id: 0,
                name: "State00".into(),
                transitions: vec![],
                clip: Some(trigger_clip),
                content: StateContent::default(),
            }],
            wildcard: vec![],
        };
        let swing = |id, name: &str, sequence: Option<&str>, transitions| State {
            id,
            name: name.into(),
            transitions,
            clip: None,
            content: StateContent { sequences: sequence.map(str::to_owned).into_iter().collect(), ..Default::default() },
        };
        let inner_swing = StateMachine {
            start_state: 6,
            states: vec![
                swing(6, "State06", None, vec![transition(single, 3)]),
                swing(3, "State03", Some("PlayAnim01"), vec![transition(reset, 6)]),
            ],
            wildcard: vec![],
        };
        let root = StateMachine {
            start_state: 5,
            states: vec![State {
                id: 5,
                name: "State05".into(),
                transitions: vec![],
                clip: None,
                content: StateContent { machines: vec![inner_trigger, inner_swing], ..Default::default() },
            }],
            wildcard: vec![],
        };
        let graph = BehaviorGraph { events: vec!["Single".into(), "Reset".into(), "Apex".into()], machine: Some(root) };
        let mut clips = HashMap::new();
        clips.insert(("a/trigger.hkx".to_owned(), true), GraphClip { handle: 11, duration: 1.0 });
        let mut sequences = HashMap::new();
        sequences.insert("playanim01".to_owned(), GraphClip { handle: 21, duration: 2.0 });
        let mut graph = ObjectAnimationGraph::new(Arc::new(LoadedGraph { graph, clips, sequences }), 5).unwrap();
        graph.start();
        graph
    }

    /// An event the root machine does not answer reaches the nested machines; the
    /// nested machine that answers it changes state, and the motion becomes the
    /// Gamebryo sequence (the trigger-only clip stays as the idle pose and
    /// supplies the schedule); `Reset` returns to the clip.
    #[test]
    fn events_reach_nested_machines_and_a_sequence_is_the_motion() {
        let mut graph = blade();
        assert_eq!(graph.state(), 5, "the root machine never moves");
        let swing = graph.send_event(0).expect("the swing machine answers Single").expect("a sequence starts");
        assert_eq!((swing.handle, swing.speed, swing.hold_at), (21, 1.0, None));
        assert_eq!(graph.send_event(0), None, "State03 has no Single transition");
        let back = graph.send_event(1).expect("Reset ends the swing").expect("the idle clip resumes");
        assert_eq!(back.handle, 11, "the trigger clip is the motion again");
    }

    /// A graph can name a Gamebryo sequence its NIF does not carry (the swinging
    /// blade's `playanim01`); that leaf does not play and the clip is the motion.
    #[test]
    fn a_sequence_the_nif_lacks_is_not_the_motion() {
        let mut graph = blade();
        let mut shared = (*graph.graph).clone();
        shared.sequences.clear();
        graph.graph = Arc::new(shared);
        let swing = graph.send_event(0);
        assert!(swing.is_some(), "the machine still answers Single");
        assert_eq!(graph.start().map(|p| p.handle), Some(11), "the trigger clip plays");
    }

    /// A clip trigger in one nested machine raises its event on schedule while the
    /// other machine's state changes; the events are not lost when the root idles.
    #[test]
    fn nested_clip_triggers_keep_their_schedule_across_a_sibling_transition() {
        let mut graph = blade();
        graph.send_event(0).unwrap();
        assert!(graph.step(0.4).is_empty());
        graph.take_raised();
        graph.step(0.2);
        assert_eq!(graph.take_raised(), ["Apex"], "the trigger clip's 0.5 s event, despite the swing");
        let seconds = graph.seconds_until(2).expect("the next Apex is 1 s on");
        assert!((seconds - 0.9).abs() < 1.0e-4 || (seconds - 1.0).abs() < 1.0e-4, "{seconds}");
    }

    #[test]
    fn a_disabled_transition_never_fires_and_a_wildcard_answers_from_anywhere() {
        let mut graph = lever();
        graph.send_event(0).unwrap();
        assert_eq!(graph.send_event(2), None, "state 2's Up transition is disabled");
        assert!(graph.send_event(3).is_some(), "the wildcard Rest works from any state");
        assert_eq!(graph.state(), 0);
    }

    #[test]
    fn the_wait_until_an_event_is_computed_through_the_machine() {
        let mut graph = lever();
        graph.send_event(0).unwrap();
        let seconds = graph.seconds_until(1).expect("Pulled is raised at the clip's end");
        assert!((seconds - 0.4).abs() < 1.0e-5, "{seconds}");
        assert_eq!(graph.seconds_until(2), None, "Up is never raised");
    }

    #[test]
    fn a_started_playback_drives_the_objects_animation_player() {
        let mut world = World::new();
        world.register::<AnimationPlayer>();
        let root = world.spawn();
        apply_playback(
            &world,
            root,
            Playback { handle: 9, speed: 2.0, start_time: 0.25, hold_at: None },
        );
        {
            let player = world.get::<AnimationPlayer>(root).unwrap();
            assert_eq!((player.clip_handle, player.speed, player.local_time, player.playing), (9, 2.0, 0.25, true));
            assert_eq!(player.root_entity, Some(root));
        }
        apply_playback(&world, root, Playback { handle: 7, speed: 1.0, start_time: 0.0, hold_at: Some(0.4) });
        let held = world.get::<AnimationPlayer>(root).unwrap();
        assert_eq!((held.clip_handle, held.local_time, held.playing), (7, 0.4, false));
    }

    #[test]
    fn a_script_archetype_overrides_and_zero_is_neutral() {
        let mut overrides = AnimArchetypeOverrides::default();
        assert_eq!(overrides.get(7), None);
        overrides.set(7, 0x22E49);
        assert_eq!(overrides.get(7), Some(0x22E49));
        overrides.set(7, 0);
        assert_eq!(overrides.get(7), Some(0));
    }
}
