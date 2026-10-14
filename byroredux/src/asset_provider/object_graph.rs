//! Placed objects' Havok behavior graphs (M47.2): resolve the project a NIF
//! names, decode its character, behavior graph, skeleton and clips from the
//! archives, register the clips, and attach the running
//! [`ObjectAnimationGraph`] — see `byroredux_scripting::animation_graph`.
//!
//! A project is `meshes\<dir>\<Name>.hkx`; its character, the character's
//! skeleton, behavior and animations are named relative to `<dir>`.
//! Skyrim Special Edition's 64-bit Havok 2010 packfiles only; anything else
//! fails to decode and the object keeps no graph (the animation natives then
//! answer the way the CK documents a missing graph).

use super::animation::convert_hkx_clip;
use super::TextureProvider;
use byroredux_core::animation::AnimationClipRegistry;
use byroredux_core::ecs::resource::Resource;
use byroredux_core::ecs::storage::EntityId;
use byroredux_core::ecs::World;
use byroredux_core::string::StringPool;
use byroredux_scripting::{BehaviorGraphRef, GraphClip, LoadedGraph, ObjectAnimationGraph};
use std::collections::HashMap;
use std::sync::Arc;

/// Graphs resolved so far, by lower-cased project path; `None` is a project
/// that did not decode (tried once). `NOT_SAVED_BY_DESIGN`: rebuilt from the
/// archives.
#[derive(Default)]
pub(crate) struct ObjectGraphCache(HashMap<String, Option<Arc<LoadedGraph>>>);

impl Resource for ObjectGraphCache {}

fn mesh_path(relative: &str) -> String {
    let relative = relative.trim_start_matches(['\\', '/']).replace('/', "\\");
    if relative.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("meshes\\")) {
        relative
    } else {
        format!("meshes\\{relative}")
    }
}

/// Decode `project` and register its clips.
fn load_graph(world: &mut World, provider: &TextureProvider, project: &str) -> Option<Arc<LoadedGraph>> {
    let project_path = mesh_path(project);
    let directory = project_path.rsplit_once('\\')?.0.to_owned();
    let relative = |name: &str| format!("{directory}\\{}", name.trim_start_matches(['\\', '/']));
    let fetch = |path: &str, what: &str| {
        let bytes = provider.extract_mesh(path);
        if bytes.is_none() {
            log::debug!("behavior graph '{project_path}': {what} '{path}' is not in the archives");
        }
        bytes
    };
    let characters = byroredux_hkx::decode_project(&fetch(&project_path, "project")?)
        .map_err(|error| log::debug!("behavior graph '{project_path}': project: {error}"))
        .ok()?;
    let character = byroredux_hkx::decode_character(&fetch(&relative(characters.first()?), "character")?)
        .map_err(|error| log::debug!("behavior graph '{project_path}': character: {error}"))
        .ok()?;
    let graph = byroredux_hkx::decode_behavior_graph(&fetch(&relative(&character.behavior), "behavior")?)
        .map_err(|error| log::debug!("behavior graph '{project_path}': behavior: {error}"))
        .ok()?;
    let skeleton = byroredux_hkx::decode_skeleton(&fetch(&relative(&character.rig), "skeleton")?)
        .map_err(|error| log::debug!("behavior graph '{project_path}': skeleton: {error}"))
        .ok()?;

    let mut clips = HashMap::new();
    for state in &graph.machine.as_ref()?.states {
        let Some(clip) = &state.clip else { continue };
        let looping = clip.mode == byroredux_hkx::ClipMode::Looping;
        let key = (clip.animation.to_ascii_lowercase(), looping);
        if clips.contains_key(&key) {
            continue;
        }
        let path = relative(&clip.animation);
        let registry_key = format!("{}#{}", path.to_ascii_lowercase(), if looping { "loop" } else { "once" });
        let existing = world.resource::<AnimationClipRegistry>().get_by_path(&registry_key);
        let (handle, duration) = match existing {
            Some(handle) => {
                let duration = world.resource::<AnimationClipRegistry>().get(handle)?.duration;
                (handle, duration)
            }
            None => {
                let animation = fetch(&path, "clip").and_then(|bytes| {
                    byroredux_hkx::decode_spline_animation(&bytes)
                        .map_err(|error| log::debug!("behavior graph '{project_path}': clip '{path}': {error}"))
                        .ok()
                })?;
                let converted = convert_hkx_clip(
                    &path,
                    &state.name,
                    &skeleton,
                    &animation,
                    &mut world.resource_mut::<StringPool>(),
                    !looping,
                    None,
                );
                let duration = converted.duration;
                let handle = world
                    .resource_mut::<AnimationClipRegistry>()
                    .get_or_insert_by_path(registry_key, || converted);
                (handle, duration)
            }
        };
        clips.insert(key, GraphClip { handle, duration });
    }
    Some(Arc::new(LoadedGraph { graph, clips }))
}

/// Attach a running behavior graph to every placed object whose NIF names one
/// and that has none yet. Idempotent and cheap when nothing is pending.
/// Returns how many objects got a graph.
pub(crate) fn populate_object_graphs(world: &mut World, provider: &TextureProvider) -> usize {
    let pending: Vec<(EntityId, Arc<str>)> = {
        let Some(refs) = world.query::<BehaviorGraphRef>() else {
            return 0;
        };
        let graphs = world.query::<ObjectAnimationGraph>();
        refs.iter()
            .filter(|(entity, _)| graphs.as_ref().is_none_or(|graphs| graphs.get(*entity).is_none()))
            .map(|(entity, reference)| (entity, Arc::clone(&reference.0)))
            .collect()
    };
    if pending.is_empty() {
        return 0;
    }
    if world.try_resource::<ObjectGraphCache>().is_none() {
        world.insert_resource(ObjectGraphCache::default());
    }
    let mut attached = 0;
    for (entity, project) in pending {
        let key = project.to_ascii_lowercase();
        let cached = world.resource::<ObjectGraphCache>().0.get(&key).cloned();
        let loaded = match cached {
            Some(loaded) => loaded,
            None => {
                let loaded = load_graph(world, provider, &project);
                match &loaded {
                    Some(graph) => log::info!(
                        "Behavior graph '{project}': {} events, {} clips",
                        graph.graph.events.len(),
                        graph.clips.len()
                    ),
                    None => log::debug!("Behavior graph '{project}' unavailable"),
                }
                world.resource_mut::<ObjectGraphCache>().0.insert(key, loaded.clone());
                loaded
            }
        };
        let Some(mut graph) = loaded.and_then(|loaded| ObjectAnimationGraph::new(loaded, entity)) else {
            continue;
        };
        let start = graph.start();
        // The graph owns the prop's animation: the NIF's own embedded controller
        // player (spawned beside the placement) would write the same node's
        // transform every frame and override it.
        let embedded: Vec<EntityId> = world
            .query::<byroredux_core::animation::AnimationPlayer>()
            .map(|players| {
                players
                    .iter()
                    .filter(|(player, state)| *player != entity && state.root_entity == Some(entity))
                    .map(|(player, _)| player)
                    .collect()
            })
            .unwrap_or_default();
        for player in embedded {
            world.despawn(player);
        }
        world.insert(entity, graph);
        log::debug!("Behavior graph '{project}' attached to entity {entity} (start playback {start:?})");
        if let Some(playback) = start {
            byroredux_scripting::animation_graph::apply_playback(world, entity, playback);
        }
        attached += 1;
    }
    attached
}
