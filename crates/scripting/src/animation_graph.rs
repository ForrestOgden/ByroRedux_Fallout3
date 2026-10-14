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
use byroredux_hkx::{BehaviorGraph, ClipMode, State, Transition, TRANSITION_FLAG_DISABLED};

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

/// A decoded behavior graph with its clips registered. Shared by every object
/// that uses the project.
#[derive(Debug, Clone)]
pub struct LoadedGraph {
    pub graph: BehaviorGraph,
    /// `(lower-cased animation path, looping)` -> the registered clip.
    pub clips: HashMap<(String, bool), GraphClip>,
}

impl LoadedGraph {
    fn state(&self, id: i32) -> Option<&State> {
        self.graph.machine.as_ref()?.states.iter().find(|state| state.id == id)
    }

    fn clip_for<'a>(&self, state: &'a State) -> Option<(&'a byroredux_hkx::StateClip, GraphClip)> {
        let clip = state.clip.as_ref()?;
        let looping = clip.mode == ClipMode::Looping;
        let registered = *self.clips.get(&(clip.animation.to_ascii_lowercase(), looping))?;
        Some((clip, registered))
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

#[derive(Debug, Clone, Default)]
struct Active {
    elapsed: f32,
    /// `(seconds from the clip start, event id)`, ascending.
    events: Vec<(f32, i32)>,
}

/// A placed object's running behavior graph.
#[derive(Debug, Clone)]
pub struct ObjectAnimationGraph {
    graph: Arc<LoadedGraph>,
    /// The entity whose subtree the clips animate (the placement root).
    pub root: EntityId,
    state: i32,
    active: Active,
    /// Events the graph raised since [`take_raised`](Self::take_raised): the
    /// ones a script's accepted `PlayAnimation` sent and the ones a clip's
    /// triggers fired (a trigger raises its event whether or not a transition
    /// answers it, as an animation notify does). Runtime only.
    raised: Vec<String>,
}

impl Component for ObjectAnimationGraph {
    type Storage = SparseSetStorage<Self>;
}

impl ObjectAnimationGraph {
    /// The graph in its start state, with the start state's clip not yet playing.
    pub fn new(graph: Arc<LoadedGraph>, root: EntityId) -> Option<Self> {
        let state = graph.graph.machine.as_ref()?.start_state;
        Some(Self { graph, root, state, active: Active::default(), raised: Vec::new() })
    }

    pub fn state(&self) -> i32 {
        self.state
    }

    pub fn graph(&self) -> &LoadedGraph {
        &self.graph
    }

    /// The events raised since the last call, oldest first.
    pub fn take_raised(&mut self) -> Vec<String> {
        std::mem::take(&mut self.raised)
    }

    fn note_raised(&mut self, event: i32) {
        if let Some(name) = self.graph.graph.event_name(event) {
            self.raised.push(name.to_owned());
        }
    }

    /// The playback of the current state's clip, started fresh.
    pub fn start(&mut self) -> Option<Playback> {
        let state = self.state;
        self.enter(state)
    }

    fn transition(&self, event: i32) -> Option<Transition> {
        if event < 0 {
            return None;
        }
        let machine = self.graph.graph.machine.as_ref()?;
        let state = self.graph.state(self.state)?;
        let usable = |t: &&Transition| t.event == event && t.flags & TRANSITION_FLAG_DISABLED == 0;
        state
            .transitions
            .iter()
            .find(usable)
            .or_else(|| machine.wildcard.iter().find(usable))
            .copied()
    }

    /// Enter `state_id`: schedule its clip's trigger events and describe its playback.
    fn enter(&mut self, state_id: i32) -> Option<Playback> {
        self.state = state_id;
        self.active = Active::default();
        let graph = Arc::clone(&self.graph);
        let state = graph.state(state_id)?;
        let (clip, registered) = graph.clip_for(state)?;
        let speed = if clip.playback_speed.is_finite() && clip.playback_speed > 0.0 {
            clip.playback_speed
        } else {
            1.0
        };
        if clip.mode == ClipMode::UserControlled {
            return Some(Playback {
                handle: registered.handle,
                speed,
                start_time: 0.0,
                hold_at: Some(clip.user_fraction.clamp(0.0, 1.0) * registered.duration),
            });
        }
        let mut events: Vec<(f32, i32)> = clip
            .triggers
            .iter()
            .filter_map(|trigger| {
                let at = if trigger.relative_to_end {
                    registered.duration + trigger.local_time
                } else {
                    trigger.local_time
                };
                let real = (at - clip.start_time) / speed;
                (real >= 0.0 && real.is_finite()).then_some((real, trigger.event))
            })
            .collect();
        events.sort_by(|a, b| a.0.total_cmp(&b.0));
        self.active.events = events;
        Some(Playback { handle: registered.handle, speed, start_time: clip.start_time, hold_at: None })
    }

    /// Raise `event`: `None` when no transition answers it; otherwise the new
    /// state's playback (`Some(None)` when that state has no clip to play).
    pub fn send_event(&mut self, event: i32) -> Option<Option<Playback>> {
        let transition = self.transition(event)?;
        Some(self.enter(transition.to_state))
    }

    /// Seconds from now until `target` is raised, following the machine through
    /// the transitions the schedule's own events take; `None` when it never is.
    pub fn seconds_until(&self, target: i32) -> Option<f32> {
        let mut sim = self.clone();
        let mut origin = -sim.active.elapsed;
        for _ in 0..64 {
            let (at, event) = *sim.active.events.first()?;
            let total = origin + at;
            if event == target {
                return Some(total.max(0.0));
            }
            sim.active.events.remove(0);
            if sim.send_event(event).is_some() {
                origin = total;
            }
        }
        None
    }

    /// Advance the schedule by `dt`: the events now due raise in order, and each
    /// that changes the state yields the playback to start.
    pub fn step(&mut self, dt: f32) -> Vec<Playback> {
        let mut started = Vec::new();
        self.active.elapsed += dt.max(0.0);
        while let Some(&(at, event)) = self.active.events.first() {
            if at > self.active.elapsed {
                break;
            }
            let carry = self.active.elapsed - at;
            self.active.events.remove(0);
            self.note_raised(event);
            if let Some(Some(playback)) = self.send_event(event) {
                self.active.elapsed = carry;
                started.push(playback);
            }
        }
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
        let before = graph.state;
        let sent = graph.send_event(id);
        if sent.is_some() {
            graph.note_raised(id);
        }
        log::debug!(
            "PlayAnimation '{event}' on entity {entity}: {} (state {before} -> {})",
            if sent.is_some() { "accepted" } else { "rejected" },
            graph.state
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
    use byroredux_hkx::{ClipTrigger, StateClip, StateMachine};

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
                },
                State {
                    id: 1,
                    name: "Pulled".into(),
                    transitions: vec![transition(2, 0)],
                    clip: Some(clip("a/pose.hkx", ClipMode::UserControlled, 1.0, Vec::new())),
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
        ObjectAnimationGraph::new(Arc::new(LoadedGraph { graph, clips }), 5).unwrap()
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
