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
//! and settles in the "Pulled" state. A state whose generator is a *parametric
//! blender* (an actor's locomotion: walk and run by `Speed`, a heading by
//! `Direction`) plays every clip of its blend tree at once as an `AnimationStack`,
//! weighted from the graph's variables each frame, and a transition with a blending
//! effect fades the old pose out as the new one fades in, and a transition gated on a
//! condition (`iSyncIdleLocomotion == 1`, `!IsPlayer`) fires only while it holds over
//! the variables. Not run: layered (non-parametric, bone-masked) blenders, which keep
//! their first child; modifiers; root motion; and the sound events (`SoundPlay.*`) the
//! triggers carry.
//!
//! The CK's results apply: `PlayAnimation` returns true when the graph accepted
//! the event; `PlayAnimationAndWait` suspends until the named event is raised
//! (the schedule is deterministic, so the wait is computed up front) and "if the
//! event you send it doesn't exist, the function will never return" — here it
//! returns false at once instead; a reference with no graph answers the way the
//! CK documents a graph that is not loaded (`false`, no wait).
//! `SetAnimationVariable*` / `GetAnimationVariable*` write and read the graph's
//! variables (decoded with their authored initial values); a blender state's weights, a
//! clip's playback speed and a transition's blend duration follow them.

use std::collections::HashMap;
use std::sync::Arc;

use byroredux_core::animation::{AnimationLayer, AnimationPlayer, AnimationStack};
use byroredux_core::ecs::sparse_set::SparseSetStorage;
use byroredux_core::ecs::storage::{Component, EntityId};
use byroredux_core::ecs::world::World;
use byroredux_hkx::{
    BehaviorGraph, BlendNode, Blender, ClipMode, StateClip, StateMachine, Transition, VariableKind, TRANSITION_FLAG_DISABLED,
};

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
    /// Clips resolved on demand (an actor's graph names hundreds of clips, and which
    /// file each is depends on the actor's animation folders): the host provides what
    /// the graph asked for. `None` for a graph whose clips are all registered.
    pub lazy: Option<Arc<LazyClips>>,
}

/// Clips a graph asked for and the host has resolved (or could not).
#[derive(Debug, Default)]
pub struct LazyClips {
    ready: std::sync::Mutex<HashMap<(String, bool), GraphClip>>,
    wanted: std::sync::Mutex<Vec<(String, bool)>>,
    missing: std::sync::Mutex<std::collections::HashSet<(String, bool)>>,
}

impl LazyClips {
    fn lookup(&self, key: &(String, bool)) -> Option<GraphClip> {
        if let Some(clip) = self.ready.lock().ok()?.get(key) {
            return Some(*clip);
        }
        let missing = self.missing.lock().ok()?.contains(key);
        if !missing {
            let mut wanted = self.wanted.lock().ok()?;
            if !wanted.contains(key) {
                wanted.push(key.clone());
            }
        }
        None
    }

    /// Whether the graph asked for a clip not yet taken.
    pub fn peek_wanted(&self) -> bool {
        self.wanted.lock().map(|wanted| !wanted.is_empty()).unwrap_or(false)
    }

    /// The `(lower-cased animation path, looping)` pairs the graph asked for since the
    /// last call.
    pub fn take_wanted(&self) -> Vec<(String, bool)> {
        self.wanted.lock().map(|mut wanted| std::mem::take(&mut *wanted)).unwrap_or_default()
    }

    /// The host resolved `key` to `clip`.
    pub fn provide(&self, key: (String, bool), clip: GraphClip) {
        if let Ok(mut ready) = self.ready.lock() {
            ready.insert(key, clip);
        }
    }

    /// The host could not resolve `key`: it is not asked for again.
    pub fn fail(&self, key: (String, bool)) {
        if let Ok(mut missing) = self.missing.lock() {
            missing.insert(key);
        }
    }
}

impl LoadedGraph {
    fn clip_for<'a>(&self, clip: &'a StateClip) -> Option<(&'a StateClip, GraphClip)> {
        let looping = clip.mode == ClipMode::Looping;
        let key = (clip.animation.to_ascii_lowercase(), looping);
        let registered = match self.clips.get(&key) {
            Some(registered) => *registered,
            None => self.lazy.as_ref()?.lookup(&key)?,
        };
        Some((clip, registered))
    }

    fn sequence(&self, name: &str) -> Option<GraphClip> {
        self.sequences.get(&name.to_ascii_lowercase()).copied()
    }
}

/// One clip of a blended pose: how much of the pose it is and how fast it runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseLayer {
    pub handle: u32,
    pub weight: f32,
    /// Playback speed multiplier (the clip's, scaled so every layer of a synced blend
    /// advances through its own length at the same rate).
    pub speed: f32,
}

/// How the animation player must run after a state change.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Playback {
    pub handle: u32,
    pub speed: f32,
    /// Clip time the playback starts at.
    pub start_time: f32,
    /// A user-controlled clip: held at this clip time instead of playing.
    pub hold_at: Option<f32>,
    /// Seconds the previous pose fades out over while this one fades in (the
    /// transition's blending effect); 0 switches at once.
    pub fade: f32,
    /// A blended pose (a state whose generator is a blender): one layer per clip of
    /// the blend tree, which replaces `handle`/`speed`. Empty for a single clip.
    pub pose: Vec<PoseLayer>,
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
    /// The entity that owns the graph and its `AnimationPlayer` (the placement root).
    pub root: EntityId,
    /// The entity whose subtree the clips animate, when it is not `root` (an actor's
    /// skeleton root).
    pub animated_root: Option<EntityId>,
    run: MachineRun,
    /// The graph's own clock, seconds.
    now: f32,
    /// Trigger events waiting for their time, in no particular order.
    scheduled: Vec<Scheduled>,
    /// The graph's variables (`Speed`, `Direction`, ...), by index into
    /// [`BehaviorGraph::variables`]: bool as 0/1, int as its value.
    variables: Vec<f32>,
    /// The longest blend duration among the transitions the last event(s) took.
    pending_fade: f32,
    /// Whether the state the graph is in plays a blended pose (the weights follow the
    /// variables every frame).
    blend_active: bool,
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
        let variables = graph.graph.variables.iter().map(|variable| variable.initial).collect();
        Some(Self {
            graph,
            root,
            animated_root: None,
            run,
            now: 0.0,
            scheduled: Vec::new(),
            raised: Vec::new(),
            variables,
            pending_fade: 0.0,
            blend_active: false,
        })
    }

    /// The graph animating `animated` (an actor's skeleton root) instead of its owner.
    pub fn with_animated_root(mut self, animated: EntityId) -> Self {
        self.animated_root = Some(animated);
        self
    }

    /// The entity the graph's clips animate.
    pub fn animated(&self) -> EntityId {
        self.animated_root.unwrap_or(self.root)
    }

    /// The root machine's current state.
    pub fn state(&self) -> i32 {
        self.run.state
    }

    pub fn graph(&self) -> &LoadedGraph {
        &self.graph
    }

    /// The value of the graph variable named `name` (case-insensitive): bool as 0/1,
    /// int as its value.
    pub fn variable(&self, name: &str) -> Option<f32> {
        self.variables.get(self.graph.graph.variable_id(name)?).copied()
    }

    /// Every variable with its current value, in graph order.
    pub fn variables(&self) -> impl Iterator<Item = (&str, f32)> {
        self.graph.graph.variables.iter().zip(&self.variables).map(|(variable, value)| (variable.name.as_str(), *value))
    }

    /// Set the graph variable named `name` (`SetAnimationVariable*`); whether the graph
    /// has it. A bool variable keeps 0/1 and an int keeps whole numbers; a variable
    /// of a kind the graph does not evaluate (a vector) is left alone.
    pub fn set_variable(&mut self, name: &str, value: f32) -> bool {
        let Some(index) = self.graph.graph.variable_id(name) else { return false };
        if !value.is_finite() {
            return false;
        }
        let value = match self.graph.graph.variables[index].kind {
            VariableKind::Bool => f32::from(value != 0.0),
            VariableKind::Int => value.trunc(),
            VariableKind::Real => value,
            VariableKind::Other => return false,
        };
        self.variables[index] = value;
        true
    }

    /// The clip's playback speed: its bound variable's value, else the authored speed.
    fn clip_speed(&self, clip: &StateClip) -> f32 {
        clip_speed_of(&self.variables, clip)
    }

    /// The seconds a taken transition fades the old pose out over: the blending
    /// effect's duration, from its bound variable when it has one.
    fn transition_fade(&self, transition: &Transition) -> f32 {
        let duration = match transition.blend_duration_variable.and_then(|index| self.variables.get(index)) {
            Some(value) => *value,
            None => transition.blend_duration,
        };
        if duration.is_finite() && duration > 0.0 {
            duration
        } else {
            0.0
        }
    }

    /// The blend tree of the state the machine at `path` is in.
    fn state_blend_at(&self, path: &[u16]) -> Option<&BlendNode> {
        let root = self.graph.graph.machine.as_ref()?;
        let (machine, run) = locate(root, &self.run, path)?;
        machine.states.iter().find(|state| state.id == run.state)?.content.blend.as_ref()
    }

    /// The layers of `blend` under the current variables, in tree order.
    fn pose_of(&self, blend: &BlendNode) -> Vec<PoseLayer> {
        pose_layers(&self.graph, &self.variables, blend)
    }

    /// The pose the current state plays right now when it is a blend (the weights move
    /// with the variables), else `None`.
    pub fn pose_now(&self) -> Option<Vec<PoseLayer>> {
        if !self.blend_active {
            return None;
        }
        let (path, _) = self.motion_leaf()?;
        let pose = self.pose_of(self.state_blend_at(&path)?);
        (!pose.is_empty()).then_some(pose)
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
        let fade = self.pending_fade;
        let blended = self.state_blend_at(&path).map(|blend| self.pose_of(blend)).filter(|pose| !pose.is_empty());
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
                Some(Playback { handle: registered.handle, speed: 1.0, start_time: 0.0, fade, ..Default::default() })
            }
            Leaf::Clip(clip) => {
                let (clip, registered) = self.graph.clip_for(clip)?;
                if let Some(pose) = blended {
                    return Some(Playback { handle: registered.handle, speed: 1.0, fade, pose, ..Default::default() });
                }
                let speed = self.clip_speed(clip);
                if clip.mode == ClipMode::UserControlled {
                    return Some(Playback {
                        handle: registered.handle,
                        speed,
                        start_time: 0.0,
                        hold_at: Some(clip.user_fraction.clamp(0.0, 1.0) * registered.duration),
                        fade,
                        ..Default::default()
                    });
                }
                Some(Playback { handle: registered.handle, speed, start_time: clip.start_time, fade, ..Default::default() })
            }
        }
    }

    /// The playback of the current configuration, started fresh (every trigger
    /// of every clip in it scheduled from now).
    pub fn start(&mut self) -> Option<Playback> {
        self.scheduled.clear();
        self.schedule_from(&[]);
        self.pending_fade = 0.0;
        let playback = self.playback(None);
        self.blend_active = playback.as_ref().is_some_and(|playback| !playback.pose.is_empty());
        playback
    }

    /// The transition `event` takes in the machine at `machine`/`run`: the current
    /// state's own first, then the machine's wildcard; disabled ones never fire, and one
    /// gated on a condition fires only while the condition holds over the variables.
    fn transition_of(&self, machine: &StateMachine, run: &MachineRun, event: i32) -> Option<Transition> {
        if event < 0 {
            return None;
        }
        let state = machine.states.iter().find(|state| state.id == run.state)?;
        let usable = |t: &&Transition| {
            t.event == event
                && t.flags & TRANSITION_FLAG_DISABLED == 0
                && t.condition.as_ref().is_none_or(|condition| condition.holds(&|name| self.variable(name)))
        };
        state.transitions.iter().find(usable).or_else(|| machine.wildcard.iter().find(usable)).cloned()
    }

    /// Give `event` to the machine at `path`: its own transition if it has one,
    /// else to the nested machines of its current state. The paths of the machines
    /// that changed state are appended to `changed`.
    fn dispatch(&mut self, path: &mut Vec<u16>, event: i32, changed: &mut Vec<Vec<u16>>) {
        let graph = Arc::clone(&self.graph);
        let Some(root) = graph.graph.machine.as_ref() else { return };
        let Some((machine, run)) = locate(root, &self.run, path) else { return };
        if let Some(transition) = self.transition_of(machine, run, event) {
            self.pending_fade = self.pending_fade.max(self.transition_fade(&transition));
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
        let playback = self.playback(Some((&changed, before)));
        self.pending_fade = 0.0;
        if let Some(playback) = &playback {
            self.blend_active = !playback.pose.is_empty();
        }
        Some(playback)
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

/// The clip's playback speed: its bound variable's value (when it is a usable speed),
/// else the authored one.
fn clip_speed_of(variables: &[f32], clip: &StateClip) -> f32 {
    let bound = clip.playback_speed_variable.and_then(|index| variables.get(index)).copied();
    let speed = bound.unwrap_or(clip.playback_speed);
    if speed.is_finite() && speed > 0.0 {
        speed
    } else {
        1.0
    }
}

/// How much of each child of a blender is in the pose.
///
/// A *parametric* blender's child weights are positions on one axis and the blend
/// parameter (the authored one, or its bound variable) falls between two of them: those
/// two children share the pose in proportion to how near the parameter is, and beyond
/// the ends the end child has all of it. A *cyclic* one treats the axis as a circle
/// of `min..max` (a heading), so the last child blends back round into the first. Any
/// other blender's weights are per-child layer amounts over bone masks, which are not
/// modelled: its first child has the pose.
pub(crate) fn blender_weights(blender: &Blender, variables: &[f32]) -> Vec<f32> {
    let count = blender.children.len();
    let mut weights = vec![0.0; count];
    if count == 0 {
        return weights;
    }
    if !blender.parametric || count == 1 {
        weights[0] = 1.0;
        return weights;
    }
    let value = |variable: Option<usize>, authored: f32| variable.and_then(|index| variables.get(index)).copied().unwrap_or(authored);
    let parameter = value(blender.parameter_variable, blender.parameter);
    let parameter = if parameter.is_finite() { parameter } else { 0.0 };
    let mut order: Vec<(f32, usize)> = blender
        .children
        .iter()
        .enumerate()
        .map(|(index, child)| (value(child.weight_variable, child.weight), index))
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let first = order[0];
    let last = order[count - 1];
    let range = blender.max_cyclic - blender.min_cyclic;
    if blender.cyclic && range > 0.0 {
        let mut at = blender.min_cyclic + (parameter - blender.min_cyclic).rem_euclid(range);
        if at < first.0 {
            at += range;
        }
        // The last child at or before `at`, and the one after it (round the circle).
        let before = order.iter().rposition(|(position, _)| *position <= at).unwrap_or(count - 1);
        let (from, to, to_position) = if before + 1 < count {
            (order[before], order[before + 1].1, order[before + 1].0)
        } else {
            (order[before], first.1, first.0 + range)
        };
        let span = to_position - from.0;
        let t = if span > 0.0 { ((at - from.0) / span).clamp(0.0, 1.0) } else { 0.0 };
        weights[from.1] += 1.0 - t;
        weights[to] += t;
        return weights;
    }
    if parameter <= first.0 {
        weights[first.1] = 1.0;
    } else if parameter >= last.0 {
        weights[last.1] = 1.0;
    } else {
        let before = order.iter().rposition(|(position, _)| *position <= parameter).unwrap_or(0);
        let (low, high) = (order[before], order[before + 1]);
        let span = high.0 - low.0;
        let t = if span > 0.0 { ((parameter - low.0) / span).clamp(0.0, 1.0) } else { 0.0 };
        weights[low.1] += 1.0 - t;
        weights[high.1] += t;
    }
    weights
}

/// A blend tree's clips as a pose: every clip the graph has (the zero-weight ones too,
/// so a clip that comes into the blend is already in step), weighted by the product of
/// the blenders above it. The clips advance through their own lengths at one common
/// rate - the weighted mean of their cycles per second - so a walk and a run
/// blended together stay in phase (an approximation of Havok's sync points).
fn pose_layers(graph: &LoadedGraph, variables: &[f32], blend: &BlendNode) -> Vec<PoseLayer> {
    struct Raw {
        handle: u32,
        weight: f32,
        speed: f32,
        duration: f32,
    }
    fn collect(graph: &LoadedGraph, variables: &[f32], node: &BlendNode, weight: f32, out: &mut Vec<Raw>) {
        match node {
            BlendNode::Clip(clip) => {
                if let Some((clip, registered)) = graph.clip_for(clip) {
                    out.push(Raw {
                        handle: registered.handle,
                        weight,
                        speed: clip_speed_of(variables, clip),
                        duration: registered.duration,
                    });
                }
            }
            BlendNode::Blender(blender) => {
                let weights = blender_weights(blender, variables);
                for (child, share) in blender.children.iter().zip(weights) {
                    collect(graph, variables, &child.node, weight * share, out);
                }
            }
        }
    }
    let mut raw = Vec::new();
    collect(graph, variables, blend, 1.0, &mut raw);
    let total: f32 = raw.iter().map(|layer| layer.weight).sum();
    let cycles = |layer: &Raw| if layer.duration > 0.0 { layer.speed / layer.duration } else { 0.0 };
    let rate = if total > 0.0 {
        raw.iter().map(|layer| layer.weight * cycles(layer)).sum::<f32>() / total
    } else {
        raw.first().map_or(0.0, cycles)
    };
    raw.into_iter()
        .map(|layer| PoseLayer {
            handle: layer.handle,
            weight: layer.weight,
            speed: if layer.duration > 0.0 { rate * layer.duration } else { layer.speed },
        })
        .collect()
}

/// Start `playback` on the graph's root: the object's `AnimationPlayer`.
pub fn apply_playback(world: &World, root: EntityId, playback: Playback) {
    apply_playback_for(world, root, root, playback);
}

/// [`apply_playback`] for a graph owned by `owner` whose clips animate `animated`.
///
/// A single clip that switches at once is the object's `AnimationPlayer`. A blended pose,
/// or a transition that fades (the old layers fade out as the new ones fade in), is an
/// `AnimationStack` on the owner instead, which the animation system blends by weight;
/// the two never coexist on the owner.
pub fn apply_playback_for(world: &World, owner: EntityId, animated: EntityId, playback: Playback) {
    let root = owner;
    let stacked = !playback.pose.is_empty() || playback.fade > 0.0;
    if !stacked {
        let mut player = AnimationPlayer::new(playback.handle).with_root(animated);
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
        if let Some(mut stacks) = world.query_mut::<AnimationStack>() {
            stacks.remove(root);
        }
        if let Some(mut players) = world.query_mut::<AnimationPlayer>() {
            players.insert(root, player);
        }
        return;
    }

    let mut stack = world
        .query_mut::<AnimationStack>()
        .and_then(|mut stacks| stacks.remove(root))
        .unwrap_or_default();
    stack.root_entity = Some(animated);
    // A plain playing clip is the pose that fades out.
    if let Some(player) = world.query_mut::<AnimationPlayer>().and_then(|mut players| players.remove(root)) {
        let mut layer = AnimationLayer::new(player.clip_handle);
        layer.local_time = player.local_time;
        layer.prev_time = player.local_time;
        layer.speed = player.speed;
        layer.playing = player.playing;
        stack.layers.push(layer);
    }
    if playback.fade > 0.0 {
        for layer in &mut stack.layers {
            if layer.blend_out_remaining <= 0.0 {
                layer.blend_out_remaining = playback.fade;
                layer.blend_out_total = playback.fade;
            }
        }
    } else {
        stack.layers.clear();
    }
    let fresh: Vec<AnimationLayer> = if playback.pose.is_empty() {
        let mut layer = AnimationLayer::new(playback.handle);
        layer.speed = playback.speed;
        match playback.hold_at {
            Some(at) => {
                layer.local_time = at;
                layer.prev_time = at;
                layer.playing = false;
            }
            None => {
                layer.local_time = playback.start_time;
                layer.prev_time = playback.start_time;
            }
        }
        vec![layer]
    } else {
        playback
            .pose
            .iter()
            .map(|pose| {
                let mut layer = AnimationLayer::new(pose.handle);
                layer.speed = pose.speed;
                layer.weight = pose.weight;
                layer
            })
            .collect()
    };
    for layer in fresh {
        stack.layers.push(if playback.fade > 0.0 { layer.with_blend_in(playback.fade) } else { layer });
    }
    if let Some(mut stacks) = world.query_mut::<AnimationStack>() {
        stacks.insert(root, stack);
    }
}

/// A graph that has faded into one plain clip goes back to the object's
/// `AnimationPlayer`: a stack with a single settled layer is a player in all but name,
/// and the systems that look for a player (a seat, a walk) must find it.
fn settle_stack(world: &World, root: EntityId) {
    let layer = {
        let Some(stacks) = world.query::<AnimationStack>() else { return };
        let Some(stack) = stacks.get(root) else { return };
        let [layer] = stack.layers.as_slice() else { return };
        if layer.blend_in_remaining > 0.0 || layer.blend_out_remaining > 0.0 {
            return;
        }
        (layer.clone(), stack.root_entity)
    };
    if world.query::<AnimationPlayer>().is_some_and(|players| players.get(root).is_some()) {
        return;
    }
    let (layer, animated) = layer;
    let mut player = AnimationPlayer::new(layer.clip_handle).with_root(animated.unwrap_or(root));
    player.local_time = layer.local_time;
    player.prev_time = layer.prev_time;
    player.speed = layer.speed;
    player.playing = layer.playing;
    if let Some(mut stacks) = world.query_mut::<AnimationStack>() {
        stacks.remove(root);
    }
    if let Some(mut players) = world.query_mut::<AnimationPlayer>() {
        players.insert(root, player);
    }
}

/// Move the weights and speeds of the blended pose on `root`'s stack to `pose`: the
/// pose is the stack's newest layers (older ones are fading out).
fn update_pose(stack: &mut AnimationStack, pose: &[PoseLayer]) {
    let Some(first) = stack.layers.len().checked_sub(pose.len()) else { return };
    for (layer, target) in stack.layers[first..].iter_mut().zip(pose) {
        if layer.clip_handle != target.handle {
            return;
        }
    }
    for (layer, target) in stack.layers[first..].iter_mut().zip(pose) {
        layer.speed = target.speed;
        layer.blend_in_target = target.weight;
        if layer.blend_in_remaining <= 0.0 {
            layer.weight = target.weight;
        }
    }
}

/// `reference.SetAnimationVariable*(name, value)`: write the variable of the
/// reference's graph; whether it has such a variable.
pub fn set_variable_value(world: &World, entity: EntityId, name: &str, value: f32) -> bool {
    let Some(mut graphs) = world.query_mut::<ObjectAnimationGraph>() else { return false };
    graphs.get_mut(entity).is_some_and(|graph| graph.set_variable(name, value))
}

/// `reference.GetAnimationVariable*(name)`: the variable's current value.
pub fn variable_value(world: &World, entity: EntityId, name: &str) -> Option<f32> {
    world.query::<ObjectAnimationGraph>()?.get(entity)?.variable(name)
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
    let (root, animated, playback) = {
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
        (graph.root, graph.animated(), sent?)
    };
    if let Some(playback) = playback {
        apply_playback_for(world, root, animated, playback);
    }
    Some(())
}

/// Step every object graph's trigger schedule (a clip's end raises its
/// completion event, which moves the machine on). Run each frame.
pub fn object_graph_step_system(world: &World, dt: f32) {
    let (started, poses, plain): (Vec<(EntityId, EntityId, Playback)>, Vec<(EntityId, Vec<PoseLayer>)>, Vec<EntityId>) = {
        let Some(mut graphs) = world.query_mut::<ObjectAnimationGraph>() else {
            return;
        };
        let mut started = Vec::new();
        let mut poses = Vec::new();
        let mut plain = Vec::new();
        for (_, graph) in graphs.iter_mut() {
            if let Some(playback) = graph.step(dt).pop() {
                started.push((graph.root, graph.animated(), playback));
            } else if let Some(pose) = graph.pose_now() {
                // The variables move the weights of a blended pose between events.
                poses.push((graph.root, pose));
            } else {
                plain.push(graph.root);
            }
        }
        (started, poses, plain)
    };
    for (root, animated, playback) in started {
        apply_playback_for(world, root, animated, playback);
    }
    for root in plain {
        settle_stack(world, root);
    }
    if poses.is_empty() {
        return;
    }
    let Some(mut stacks) = world.query_mut::<AnimationStack>() else { return };
    let taken_over: Vec<EntityId> = match world.query::<AnimationPlayer>() {
        Some(players) => poses.iter().map(|(root, _)| *root).filter(|root| players.get(*root).is_some()).collect(),
        None => Vec::new(),
    };
    for (root, pose) in &poses {
        if taken_over.contains(root) {
            // Another system gave the owner a clip of its own (a seat, a walk): the
            // graph's stack must not fight it.
            stacks.remove(*root);
        } else if let Some(stack) = stacks.get_mut(*root) {
            update_pose(stack, pose);
        }
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
        Transition { event, to_state, priority: 0, flags: 0, ..Default::default() }
    }

    fn clip(animation: &str, mode: ClipMode, speed: f32, triggers: Vec<ClipTrigger>) -> StateClip {
        StateClip {
            animation: animation.into(),
            mode,
            playback_speed: speed,
            start_time: 0.0,
            user_fraction: 0.0,
            triggers,
            playback_speed_variable: None,
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
            variables: Vec::new(),
            events: vec!["Down".into(), "Pulled".into(), "Up".into(), "Rest".into()],
            machine: Some(machine),
        };
        let mut clips = HashMap::new();
        clips.insert(("a/pose.hkx".to_owned(), false), GraphClip { handle: 7, duration: 0.8 });
        clips.insert(("a/pull.hkx".to_owned(), false), GraphClip { handle: 9, duration: 0.8 });
        ObjectAnimationGraph::new(Arc::new(LoadedGraph { graph, clips, sequences: HashMap::new(), lazy: None }), 5).unwrap()
    }

    // ── variables, blenders and transition fades ──────────────────────

    use byroredux_hkx::{BlendChild, Variable};

    fn blend_clip(animation: &str) -> BlendNode {
        BlendNode::Clip(clip(animation, ClipMode::Looping, 1.0, Vec::new()))
    }

    /// The vanilla locomotion shape: a cyclic direction blender (children at 0, 0.25, 0.5,
    /// 0.75 of a turn) over speed blenders (walk at 80, run at 370 units/s).
    fn locomotion_blend() -> BlendNode {
        let speed = |walk: &str, run: &str| {
            BlendNode::Blender(Blender {
                parametric: true,
                cyclic: false,
                sync: true,
                parameter: 0.0,
                parameter_variable: Some(0),
                min_cyclic: 0.0,
                max_cyclic: 1.0,
                children: vec![
                    BlendChild { weight: 80.0, weight_variable: None, node: blend_clip(walk) },
                    BlendChild { weight: 370.0, weight_variable: None, node: blend_clip(run) },
                ],
            })
        };
        BlendNode::Blender(Blender {
            parametric: true,
            cyclic: true,
            sync: true,
            parameter: 0.0,
            parameter_variable: Some(1),
            min_cyclic: 0.0,
            max_cyclic: 1.0,
            children: ["forward", "right", "back", "left"]
                .iter()
                .enumerate()
                .map(|(index, name)| BlendChild {
                    weight: index as f32 * 0.25,
                    weight_variable: None,
                    node: speed(&format!("walk_{name}"), &format!("run_{name}")),
                })
                .collect(),
        })
    }

    /// An idle (Idle, state 0) that `Move` (event 0) takes to the locomotion blend
    /// (Move, state 1) over a quarter second, and `Stop` (event 1) back over 0.1 s.
    /// Variables: `Speed` (0), `Direction` (1), `bFlag` (2, bool), `iMode` (3, int).
    fn locomotion() -> ObjectAnimationGraph {
        let fading = |event, to_state, blend_duration| Transition { blend_duration, ..transition(event, to_state) };
        let machine = StateMachine {
            start_state: 0,
            states: vec![
                State {
                    id: 0,
                    name: "Idle".into(),
                    transitions: vec![fading(0, 1, 0.25)],
                    clip: Some(clip("idle", ClipMode::Looping, 1.0, Vec::new())),
                    content: Default::default(),
                },
                State {
                    id: 1,
                    name: "Move".into(),
                    transitions: vec![fading(1, 0, 0.1)],
                    clip: Some(clip("walk_forward", ClipMode::Looping, 1.0, Vec::new())),
                    content: byroredux_hkx::StateContent { blend: Some(locomotion_blend()), ..Default::default() },
                },
            ],
            wildcard: Vec::new(),
        };
        let real = |name: &str, initial| Variable { name: name.into(), kind: byroredux_hkx::VariableKind::Real, initial };
        let graph = BehaviorGraph {
            variables: vec![
                real("Speed", 0.0),
                real("Direction", 0.0),
                Variable { name: "bFlag".into(), kind: byroredux_hkx::VariableKind::Bool, initial: 0.0 },
                Variable { name: "iMode".into(), kind: byroredux_hkx::VariableKind::Int, initial: 2.0 },
            ],
            events: vec!["Move".into(), "Stop".into()],
            machine: Some(machine),
        };
        let mut clips = HashMap::new();
        clips.insert(("idle".to_owned(), true), GraphClip { handle: 1, duration: 2.0 });
        for (index, name) in ["forward", "right", "back", "left"].iter().enumerate() {
            clips.insert((format!("walk_{name}"), true), GraphClip { handle: 10 + index as u32, duration: 1.0 });
            clips.insert((format!("run_{name}"), true), GraphClip { handle: 20 + index as u32, duration: 0.5 });
        }
        ObjectAnimationGraph::new(Arc::new(LoadedGraph { graph, clips, sequences: HashMap::new(), lazy: None }), 3).unwrap()
    }

    fn weight_of(pose: &[PoseLayer], handle: u32) -> f32 {
        pose.iter().find(|layer| layer.handle == handle).map_or(0.0, |layer| layer.weight)
    }

    /// A parametric blender mixes the two children its parameter falls between and
    /// holds the end child beyond the ends; a cyclic one wraps round from the last
    /// child to the first.
    #[test]
    fn parametric_blenders_mix_the_neighbours_and_cyclic_ones_wrap() {
        let BlendNode::Blender(direction) = locomotion_blend() else { unreachable!() };
        let BlendNode::Blender(speed) = &direction.children[0].node else { unreachable!() };
        let speeds = |value: f32| blender_weights(speed, &[value, 0.0]);
        assert_eq!(speeds(0.0), vec![1.0, 0.0], "slower than the walk: the walk");
        assert_eq!(speeds(80.0), vec![1.0, 0.0]);
        assert_eq!(speeds(225.0), vec![0.5, 0.5], "half way between walk and run");
        assert_eq!(speeds(370.0), vec![0.0, 1.0]);
        assert_eq!(speeds(900.0), vec![0.0, 1.0], "faster than the run: the run");

        let headings = |value: f32| blender_weights(&direction, &[0.0, value]);
        assert_eq!(headings(0.0), vec![1.0, 0.0, 0.0, 0.0]);
        assert_eq!(headings(0.125), vec![0.5, 0.5, 0.0, 0.0]);
        assert_eq!(headings(0.5), vec![0.0, 0.0, 1.0, 0.0]);
        let round = headings(0.875);
        assert!((round[3] - 0.5).abs() < 1e-6 && (round[0] - 0.5).abs() < 1e-6, "between the last child and the first: {round:?}");
        assert_eq!(headings(1.0), headings(0.0), "a full turn is no turn");
        assert_eq!(headings(-0.25), headings(0.75), "negative headings wrap");

        // Not parametric: the first child has the pose.
        let layered = Blender { parametric: false, ..direction.clone() };
        assert_eq!(blender_weights(&layered, &[0.0, 0.5]), vec![1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn variables_have_the_graphs_initial_values_and_keep_their_kind() {
        let mut graph = locomotion();
        assert_eq!(graph.variable("speed"), Some(0.0), "names are case-insensitive");
        assert_eq!(graph.variable("iMode"), Some(2.0), "the authored initial value");
        assert!(graph.set_variable("Speed", 225.0));
        assert_eq!(graph.variable("Speed"), Some(225.0));
        assert!(graph.set_variable("bFlag", 7.0));
        assert_eq!(graph.variable("bFlag"), Some(1.0), "a bool is 0 or 1");
        assert!(graph.set_variable("iMode", 3.9));
        assert_eq!(graph.variable("iMode"), Some(3.0), "an int is whole");
        assert!(!graph.set_variable("NoSuchVariable", 1.0));
        assert!(!graph.set_variable("Speed", f32::NAN));
        assert_eq!(graph.variable("Speed"), Some(225.0));
    }

    /// Entering the locomotion state yields a blended pose (every clip of the tree,
    /// weighted by its variables), faded in over the transition's duration; the
    /// variables then move the weights without another event, and the clips advance
    /// at one common rate so they stay in phase.
    #[test]
    fn a_blend_state_plays_a_weighted_pose_that_follows_its_variables() {
        let mut graph = locomotion();
        graph.start();
        assert!(graph.pose_now().is_none(), "the idle is a single clip");
        graph.set_variable("Speed", 225.0);
        graph.set_variable("Direction", 0.125);
        let playback = graph.send_event(0).expect("Move is taken").expect("the state plays");
        assert_eq!(playback.fade, 0.25, "the transition's blending effect");
        assert_eq!(playback.pose.len(), 8, "every clip of the blend tree: 4 headings x walk and run");
        // Heading 0.125 is half forward, half right; 225 is half walk, half run.
        for handle in [10, 11, 20, 21] {
            assert!((weight_of(&playback.pose, handle) - 0.25).abs() < 1e-6, "handle {handle}");
        }
        assert_eq!(weight_of(&playback.pose, 12), 0.0);
        let total: f32 = playback.pose.iter().map(|layer| layer.weight).sum();
        assert!((total - 1.0).abs() < 1e-5);
        // Every layer crosses its own length at the same rate (walk 1 s, run 0.5 s).
        let rate = |handle: u32, duration: f32| playback.pose.iter().find(|l| l.handle == handle).unwrap().speed / duration;
        assert!((rate(10, 1.0) - rate(20, 0.5)).abs() < 1e-5, "walk and run stay in phase");
        // The mean of one cycle per second (walk) and two (run), half and half.
        assert!((rate(10, 1.0) - 1.5).abs() < 1e-5);

        graph.set_variable("Speed", 370.0);
        graph.set_variable("Direction", 0.5);
        let moved = graph.pose_now().expect("still blending");
        assert_eq!(weight_of(&moved, 22), 1.0, "running backwards");
        assert_eq!(weight_of(&moved, 10), 0.0);
    }

    /// A transition with a duration fades: the old pose's layers fade out and the new
    /// pose's fade in on an `AnimationStack`; an instant one into a single clip goes
    /// back to a plain `AnimationPlayer`.
    #[test]
    fn a_fading_transition_stacks_the_poses_and_an_instant_one_unstacks_them() {
        let mut world = World::new();
        world.register::<AnimationPlayer>();
        world.register::<AnimationStack>();
        let mut graph = locomotion();
        let idle = graph.start().expect("the idle plays");
        apply_playback_for(&world, 3, 3, idle);
        assert!(world.get::<AnimationPlayer>(3).is_some() && world.get::<AnimationStack>(3).is_none());

        graph.set_variable("Speed", 80.0);
        let moving = graph.send_event(0).unwrap().unwrap();
        apply_playback_for(&world, 3, 3, moving);
        assert!(world.get::<AnimationPlayer>(3).is_none(), "the stack replaces the player");
        {
            let stack = world.get::<AnimationStack>(3).expect("a stack");
            assert_eq!(stack.layers.len(), 1 + 8, "the idle fading out, then the pose");
            assert_eq!(stack.layers[0].clip_handle, 1);
            assert_eq!((stack.layers[0].blend_out_total, stack.layers[1].blend_in_total), (0.25, 0.25));
            // The walk-forward layer ramps to a full weight, the others to none.
            assert_eq!(stack.layers[1].blend_in_target, 1.0);
            assert_eq!(stack.layers[2].blend_in_target, 0.0);
        }

        // Between events, the variables retarget the newest layers.
        graph.set_variable("Speed", 370.0);
        let pose = graph.pose_now().unwrap();
        update_pose(&mut world.get_mut::<AnimationStack>(3).unwrap(), &pose);
        let stack = world.get::<AnimationStack>(3).unwrap();
        assert_eq!(stack.layers[1].blend_in_target, 0.0, "no longer the walk");
        assert_eq!(stack.layers[2].clip_handle, 20, "run_forward");
        assert_eq!(stack.layers[2].blend_in_target, 1.0, "the run, ramping in");
        drop(stack);

        // Back to the idle over 0.1 s: still a fade, so still a stack...
        let stopping = graph.send_event(1).unwrap().unwrap();
        assert_eq!((stopping.fade, stopping.pose.len()), (0.1, 0));
        apply_playback_for(&world, 3, 3, stopping);
        assert!(world.get::<AnimationStack>(3).is_some());
        // ... and an instant playback with no pose is a plain player again.
        apply_playback_for(&world, 3, 3, Playback { handle: 1, speed: 1.0, ..Default::default() });
        assert!(world.get::<AnimationStack>(3).is_none());
        assert_eq!(world.get::<AnimationPlayer>(3).unwrap().clip_handle, 1);
    }

    /// Once a fade has finished and one plain clip is left, the stack goes back to a
    /// plain player (so the systems that look for a player find the clip).
    #[test]
    fn a_settled_single_layer_stack_becomes_a_player_again() {
        let mut world = World::new();
        world.register::<AnimationPlayer>();
        world.register::<AnimationStack>();
        let mut graph = locomotion();
        apply_playback_for(&world, 3, 3, graph.start().unwrap());
        graph.set_variable("Speed", 80.0);
        apply_playback_for(&world, 3, 3, graph.send_event(0).unwrap().unwrap());
        apply_playback_for(&world, 3, 3, graph.send_event(1).unwrap().unwrap());
        // Mid-fade: nothing settles.
        settle_stack(&world, 3);
        assert!(world.get::<AnimationStack>(3).is_some());
        // The fade completes: the old layers are gone and the new one is at full weight.
        {
            let stack = &mut *world.get_mut::<AnimationStack>(3).unwrap();
            stack.layers.retain(|layer| layer.blend_in_total > 0.0 && layer.clip_handle == 1);
            let layer = &mut stack.layers[0];
            layer.blend_in_remaining = 0.0;
            layer.weight = 1.0;
            layer.local_time = 0.75;
        }
        settle_stack(&world, 3);
        assert!(world.get::<AnimationStack>(3).is_none());
        let player = world.get::<AnimationPlayer>(3).expect("a player");
        assert_eq!((player.clip_handle, player.local_time, player.root_entity), (1, 0.75, Some(3)));
    }

    /// A transition gated on a condition fires only while the expression holds over the
    /// graph's variables; one without a condition (or whose text is unreadable) always
    /// does.
    #[test]
    fn a_conditioned_transition_waits_for_its_condition() {
        let mut graph = locomotion();
        let mut shared = (*graph.graph).clone();
        let machine = shared.graph.machine.as_mut().unwrap();
        machine.states[0].transitions[0].condition = Some(Arc::new(byroredux_hkx::Condition::parse("Speed > 50 && !bFlag")));
        graph.graph = Arc::new(shared);
        graph.start();
        assert_eq!(graph.send_event(0), None, "Speed is 0: Move is refused");
        graph.set_variable("Speed", 80.0);
        graph.set_variable("bFlag", 1.0);
        assert_eq!(graph.send_event(0), None, "bFlag blocks it");
        graph.set_variable("bFlag", 0.0);
        assert!(graph.send_event(0).is_some(), "now it holds");
        assert_eq!(graph.state(), 1);

        // Text the grammar cannot read does not block.
        let mut graph = locomotion();
        let mut shared = (*graph.graph).clone();
        shared.graph.machine.as_mut().unwrap().states[0].transitions[0].condition =
            Some(Arc::new(byroredux_hkx::Condition::parse("iMode=18")));
        graph.graph = Arc::new(shared);
        graph.start();
        assert!(graph.send_event(0).is_some());
    }

    /// A transition's duration comes from its bound variable when it has one.
    #[test]
    fn a_bound_duration_reads_its_variable() {
        let mut graph = locomotion();
        let mut shared = (*graph.graph).clone();
        let machine = shared.graph.machine.as_mut().unwrap();
        machine.states[0].transitions[0].blend_duration_variable = Some(0);
        graph.graph = Arc::new(shared);
        graph.start();
        graph.set_variable("Speed", 0.6);
        let playback = graph.send_event(0).unwrap().unwrap();
        assert_eq!(playback.fade, 0.6, "the variable, not the authored 0.25");
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
        let graph = BehaviorGraph { variables: Vec::new(), events: vec!["Single".into(), "Reset".into(), "Apex".into()], machine: Some(root) };
        let mut clips = HashMap::new();
        clips.insert(("a/trigger.hkx".to_owned(), true), GraphClip { handle: 11, duration: 1.0 });
        let mut sequences = HashMap::new();
        sequences.insert("playanim01".to_owned(), GraphClip { handle: 21, duration: 2.0 });
        let mut graph = ObjectAnimationGraph::new(Arc::new(LoadedGraph { graph, clips, sequences, lazy: None }), 5).unwrap();
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
            Playback { handle: 9, speed: 2.0, start_time: 0.25, hold_at: None, ..Default::default() },
        );
        {
            let player = world.get::<AnimationPlayer>(root).unwrap();
            assert_eq!((player.clip_handle, player.speed, player.local_time, player.playing), (9, 2.0, 0.25, true));
            assert_eq!(player.root_entity, Some(root));
        }
        apply_playback(&world, root, Playback { handle: 7, speed: 1.0, start_time: 0.0, hold_at: Some(0.4), ..Default::default() });
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

    /// An actor's graph resolves clips on demand: a state whose clip is not yet provided
    /// plays nothing and asks for it; once the host provides it, restarting the graph
    /// starts the clip, and a clip the host cannot resolve is not asked for again.
    #[test]
    fn a_lazy_clip_is_requested_then_played_once_provided() {
        let graph = BehaviorGraph {
            variables: Vec::new(),
            events: vec!["go".into()],
            machine: Some(StateMachine {
                start_state: 0,
                states: vec![
                    State {
                        id: 0,
                        name: "A".into(),
                        transitions: vec![transition(0, 1)],
                        clip: Some(clip("Animations\\A.hkt", ClipMode::Looping, 1.0, Vec::new())),
                        content: Default::default(),
                    },
                    State {
                        id: 1,
                        name: "B".into(),
                        transitions: Vec::new(),
                        clip: Some(clip("Animations\\B.hkt", ClipMode::SinglePlay, 1.0, Vec::new())),
                        content: Default::default(),
                    },
                ],
                wildcard: Vec::new(),
            }),
        };
        let lazy = Arc::new(LazyClips::default());
        let loaded = Arc::new(LoadedGraph { graph, clips: HashMap::new(), sequences: HashMap::new(), lazy: Some(Arc::clone(&lazy)) });
        let mut run = ObjectAnimationGraph::new(loaded, 5).unwrap().with_animated_root(9);
        assert_eq!(run.animated(), 9);
        assert_eq!(run.start(), None, "nothing to play until the clip is provided");
        assert_eq!(lazy.take_wanted(), [("animations\\a.hkt".to_owned(), true)]);
        assert!(lazy.take_wanted().is_empty(), "asked once");
        lazy.provide(("animations\\a.hkt".to_owned(), true), GraphClip { handle: 3, duration: 2.0 });
        assert_eq!(run.start().map(|playback| playback.handle), Some(3));
        // The event moves to a clip nobody can resolve: no playback, and it is not re-asked.
        run.send_event(0);
        assert_eq!(lazy.take_wanted(), [("animations\\b.hkt".to_owned(), false)]);
        lazy.fail(("animations\\b.hkt".to_owned(), false));
        run.start();
        assert!(lazy.take_wanted().is_empty());
    }
}
