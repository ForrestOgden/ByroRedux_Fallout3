//! Locomotion inputs for actors that run a behavior graph (M47.2).
//!
//! A Fallout 4 actor's `MTBehavior` walks, jogs and runs from the *variables* the engine
//! feeds it: `Speed` (world units a second), `Direction` (the heading of the motion
//! relative to the body, a fraction of a turn clockwise from straight ahead) and the
//! `moveStart` / `moveStop` events (CK: a graph "state machine whose transitions fire on
//! named events"). This system derives them from the actor's own displacement, the way
//! `npc_walk_animation_system` classifies the same motion for the Skyrim walk clip: the
//! locomotion procedures and combat chase move the root, and whatever moved it, the graph
//! sees the speed and heading that resulted. The graph's blenders (`Speed` between its walk
//! and run clips, `Direction` round its eight-way cycle) and its transition fades then do
//! the rest.
//!
//! The hysteresis windows are the walk-clip system's (a collision slide must not cut the
//! gait), and an actor that system already drives is left to it.

use super::walk_anim::{WALK_ANIM_MIN_SPEED, WALK_ANIM_RELEASE_DWELL, WALK_ANIM_TAKE_DWELL};
use crate::components::WalkAnimation;
use byroredux_core::ecs::components::{Dead, Seated, Transform};
use byroredux_core::ecs::sparse_set::SparseSetStorage;
use byroredux_core::ecs::storage::{Component, EntityId};
use byroredux_core::ecs::World;
use byroredux_core::math::{Quat, Vec3};
use byroredux_scripting::animation_graph::{play_animation, set_variable_value};
use byroredux_scripting::ObjectAnimationGraph;

/// A displacement faster than this in one tick is a teleport (a cell change, a script's
/// `MoveTo`), not walking: the baseline moves and no speed is read from it. A sprinting
/// actor is well under it.
const TELEPORT_SPEED: f32 = 1500.0;

/// Time constant of the speed smoothing, seconds: the per-frame displacement is noisy
/// (a fixed-step mover, collision slides) and the graph blends on it.
const SPEED_SMOOTHING_SECS: f32 = 0.1;

/// One actor's locomotion state. `NOT_SAVED_BY_DESIGN`: re-derived from the next frames'
/// displacement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GraphLocomotion {
    last_pos: Vec3,
    /// Smoothed speed, world units a second.
    speed: f32,
    /// Whether `moveStart` has been sent (and `moveStop` has not).
    moving: bool,
    /// How long the motion has disagreed with `moving`.
    dwell: f32,
}

impl Component for GraphLocomotion {
    type Storage = SparseSetStorage<Self>;
}

/// The heading of `moved` relative to a body with `rotation`, as a fraction of a turn
/// clockwise (seen from above) from straight ahead: 0 forward, 0.25 right, 0.5 back, 0.75
/// left - the positions of the eight-way blenders' children. A body faces +Z at zero yaw
/// and its right-hand side is -X (the opposite of the camera that looks down -Z).
pub(crate) fn heading_fraction(moved: Vec3, rotation: Quat) -> f32 {
    let forward = rotation * Vec3::Z;
    let right = rotation * Vec3::NEG_X;
    let angle = moved.dot(right).atan2(moved.dot(forward));
    (angle / std::f32::consts::TAU).rem_euclid(1.0)
}

/// An actor the console is walking in a straight line (`actor.walk`): a stand-in mover
/// for exercising what the locomotion systems feed a graph on actors no package moves
/// yet. `NOT_SAVED_BY_DESIGN`: a debugging aid with a lifetime of seconds.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ConsoleWalk {
    /// World-space velocity, units a second.
    pub(crate) velocity: Vec3,
    pub(crate) remaining: f32,
}

impl Component for ConsoleWalk {
    type Storage = SparseSetStorage<Self>;
}

/// Move every [`ConsoleWalk`] actor along its velocity, facing it. Runs before the
/// graph's inputs are read.
pub(crate) fn console_walk_system(world: &World, dt: f32) {
    let Some(mut walks) = world.query_mut::<ConsoleWalk>() else { return };
    if walks.iter().next().is_none() {
        return;
    }
    let Some(mut transforms) = world.query_mut::<Transform>() else { return };
    let mut done = Vec::new();
    for (entity, walk) in walks.iter_mut() {
        let step = dt.min(walk.remaining).max(0.0);
        if let Some(transform) = transforms.get_mut(entity) {
            transform.translation += walk.velocity * step;
            if walk.velocity.length_squared() > 1e-6 {
                transform.rotation = Quat::from_rotation_y(walk.velocity.x.atan2(walk.velocity.z));
            }
        }
        walk.remaining -= step;
        if walk.remaining <= 0.0 {
            done.push(entity);
        }
    }
    for entity in done {
        walks.remove(entity);
    }
}

struct Observed {
    entity: EntityId,
    position: Vec3,
    rotation: Quat,
    parked: bool,
}

/// Feed every graph-driven actor's `Speed` / `Direction` variables and `moveStart` /
/// `moveStop` events from its motion; see the module docs. Exclusive, after the movers.
pub(crate) fn graph_locomotion_system(world: &World, dt: f32) {
    let observed: Vec<Observed> = {
        let Some(graphs) = world.query::<ObjectAnimationGraph>() else { return };
        let Some(transforms) = world.query::<Transform>() else { return };
        let walkers = world.query::<WalkAnimation>();
        let seated = world.query::<Seated>();
        let dead = world.query::<Dead>();
        let cinematic = world.query::<byroredux_scripting::ActorCinematicState>();
        graphs
            .iter()
            .filter(|(entity, _)| !walkers.as_ref().is_some_and(|walkers| walkers.contains(*entity)))
            .filter_map(|(entity, _)| {
                let transform = transforms.get(entity)?;
                Some(Observed {
                    entity,
                    position: transform.translation,
                    rotation: transform.rotation,
                    parked: seated.as_ref().is_some_and(|q| q.contains(entity))
                        || dead.as_ref().is_some_and(|q| q.contains(entity))
                        || cinematic.as_ref().is_some_and(|q| q.contains(entity)),
                })
            })
            .collect()
    };
    if observed.is_empty() {
        return;
    }
    let known = |entity| world.query::<GraphLocomotion>().and_then(|q| q.get(entity).map(|state| *state));
    let mut next: Vec<(EntityId, GraphLocomotion)> = Vec::with_capacity(observed.len());
    for actor in observed {
        let Some(mut state) = known(actor.entity) else {
            next.push((actor.entity, GraphLocomotion { last_pos: actor.position, speed: 0.0, moving: false, dwell: 0.0 }));
            continue;
        };
        let moved = Vec3::new(actor.position.x - state.last_pos.x, 0.0, actor.position.z - state.last_pos.z);
        state.last_pos = actor.position;
        let raw = if dt > f32::MIN_POSITIVE && dt < 0.25 { moved.length() / dt } else { 0.0 };
        let raw = if raw > TELEPORT_SPEED || actor.parked { 0.0 } else { raw };
        let blend = 1.0 - (-dt.max(0.0) / SPEED_SMOOTHING_SECS).exp();
        state.speed += (raw - state.speed) * blend;
        let in_motion = state.speed > WALK_ANIM_MIN_SPEED;
        if in_motion != state.moving {
            state.dwell += dt;
            let window = if in_motion { WALK_ANIM_TAKE_DWELL } else { WALK_ANIM_RELEASE_DWELL };
            if state.dwell >= window {
                state.moving = in_motion;
                state.dwell = 0.0;
                let event = if in_motion { "moveStart" } else { "moveStop" };
                let accepted = play_animation(world, actor.entity, event);
                log::debug!("graph locomotion: {event} on entity {} - {}", actor.entity, if accepted { "taken" } else { "no transition" });
            }
        } else {
            state.dwell = 0.0;
        }
        let speed = if state.moving { state.speed } else { 0.0 };
        set_variable_value(world, actor.entity, "Speed", speed);
        set_variable_value(world, actor.entity, "SpeedSmoothed", speed);
        if raw > WALK_ANIM_MIN_SPEED {
            set_variable_value(world, actor.entity, "Direction", heading_fraction(moved, actor.rotation));
        }
        next.push((actor.entity, state));
    }
    if let Some(mut states) = world.query_mut::<GraphLocomotion>() {
        for (entity, state) in next {
            states.insert(entity, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byroredux_core::animation::{AnimationPlayer, AnimationStack};
    use byroredux_hkx::{BehaviorGraph, ClipMode, State, StateClip, StateMachine, Transition, Variable, VariableKind};
    use byroredux_scripting::{GraphClip, LoadedGraph};
    use std::collections::HashMap;
    use std::sync::Arc;

    fn clip(name: &str) -> StateClip {
        StateClip {
            animation: name.into(),
            mode: ClipMode::Looping,
            playback_speed: 1.0,
            start_time: 0.0,
            user_fraction: 0.0,
            triggers: Vec::new(),
            playback_speed_variable: None,
        }
    }

    /// Idle (state 0) --moveStart--> Move (state 1) --moveStop--> Idle.
    fn graph() -> ObjectAnimationGraph {
        let go = |event, to_state| Transition { event, to_state, priority: 0, flags: 0, ..Default::default() };
        let state = |id, name: &str, transitions, clip_name| State {
            id,
            name: name.into(),
            transitions,
            clip: Some(clip(clip_name)),
            content: Default::default(),
        };
        let real = |name: &str| Variable { name: name.into(), kind: VariableKind::Real, initial: 0.0 };
        let loaded = BehaviorGraph {
            variables: vec![real("Speed"), real("SpeedSmoothed"), real("Direction")],
            events: vec!["moveStart".into(), "moveStop".into()],
            machine: Some(StateMachine {
                start_state: 0,
                states: vec![state(0, "Idle", vec![go(0, 1)], "idle"), state(1, "Move", vec![go(1, 0)], "walk")],
                wildcard: Vec::new(),
            }),
        };
        let mut clips = HashMap::new();
        clips.insert(("idle".to_owned(), true), GraphClip { handle: 1, duration: 2.0 });
        clips.insert(("walk".to_owned(), true), GraphClip { handle: 2, duration: 1.0 });
        ObjectAnimationGraph::new(Arc::new(LoadedGraph { graph: loaded, clips, sequences: HashMap::new(), lazy: None }), 0).unwrap()
    }

    fn world_with_actor() -> (World, EntityId) {
        let mut world = World::new();
        world.register::<ObjectAnimationGraph>();
        world.register::<GraphLocomotion>();
        world.register::<Transform>();
        world.register::<AnimationPlayer>();
        world.register::<AnimationStack>();
        let actor = world.spawn();
        let mut graph = graph();
        graph.root = actor;
        world.insert(actor, graph);
        world.insert(actor, Transform::from_translation(Vec3::ZERO));
        (world, actor)
    }

    fn walk(world: &World, actor: EntityId, ticks: usize, dt: f32, step: Vec3) {
        for _ in 0..ticks {
            let mut transforms = world.query_mut::<Transform>().unwrap();
            transforms.get_mut(actor).unwrap().translation += step;
            drop(transforms);
            graph_locomotion_system(world, dt);
        }
    }

    fn variable(world: &World, actor: EntityId, name: &str) -> f32 {
        byroredux_scripting::animation_graph::variable_value(world, actor, name).unwrap()
    }

    /// Straight ahead is 0, a quarter turn right 0.25, backwards 0.5, left 0.75 -
    /// wrapping, whatever way the body is turned.
    #[test]
    fn the_heading_is_a_fraction_of_a_turn_from_straight_ahead() {
        let facing = |yaw: f32| Quat::from_rotation_y(yaw);
        let ahead = facing(0.0) * Vec3::Z;
        assert!(heading_fraction(ahead, facing(0.0)).abs() < 1e-5);
        assert!((heading_fraction(Vec3::NEG_X, facing(0.0)) - 0.25).abs() < 1e-5, "its right is -X");
        assert!((heading_fraction(-Vec3::Z, facing(0.0)) - 0.5).abs() < 1e-5);
        assert!((heading_fraction(Vec3::X, facing(0.0)) - 0.75).abs() < 1e-5);
        // Turned 90 degrees (yaw), forward is +X: walking along +X is straight ahead.
        let turned = facing(std::f32::consts::FRAC_PI_2);
        assert!(heading_fraction(Vec3::X, turned).abs() < 1e-5 || (heading_fraction(Vec3::X, turned) - 1.0).abs() < 1e-5);
    }

    /// A moving actor feeds its graph the speed it walks and starts the move state; a
    /// stopped one zeroes the speed and takes `moveStop` after the release window.
    #[test]
    fn an_actor_that_walks_feeds_the_graph_its_speed_and_events() {
        let (world, actor) = world_with_actor();
        graph_locomotion_system(&world, 0.016); // baseline
        // 100 units a second, forward.
        walk(&world, actor, 40, 0.016, Vec3::new(0.0, 0.0, 1.6));
        let speed = variable(&world, actor, "Speed");
        assert!((speed - 100.0).abs() < 2.0, "smoothed to the walked speed, got {speed}");
        assert_eq!(variable(&world, actor, "SpeedSmoothed"), speed);
        let direction = variable(&world, actor, "Direction");
        assert!(direction < 0.01 || direction > 0.99, "walking along its facing: {direction}");
        let graphs = world.query::<ObjectAnimationGraph>().unwrap();
        assert_eq!(graphs.get(actor).unwrap().state(), 1, "moveStart moved the graph to its move state");
        drop(graphs);

        walk(&world, actor, 80, 0.016, Vec3::ZERO);
        assert_eq!(variable(&world, actor, "Speed"), 0.0);
        let graphs = world.query::<ObjectAnimationGraph>().unwrap();
        assert_eq!(graphs.get(actor).unwrap().state(), 0, "moveStop returned it to the idle");
    }

    /// A one-tick jump (a teleport) and a seated actor are not walking.
    #[test]
    fn teleports_and_parked_actors_do_not_read_as_walking() {
        let (world, actor) = world_with_actor();
        graph_locomotion_system(&world, 0.016);
        walk(&world, actor, 1, 0.016, Vec3::new(0.0, 0.0, 5000.0));
        walk(&world, actor, 20, 0.016, Vec3::ZERO);
        assert_eq!(variable(&world, actor, "Speed"), 0.0);
        let graphs = world.query::<ObjectAnimationGraph>().unwrap();
        assert_eq!(graphs.get(actor).unwrap().state(), 0, "never started moving");
    }
}
