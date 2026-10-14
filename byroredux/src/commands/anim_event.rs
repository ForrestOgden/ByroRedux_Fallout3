//! `anim.event` - `ObjectReference.PlayAnimation` from the console.

use super::shared::*;

/// `anim.event <entity|.> <event>`: send an animation event to the reference's behavior
/// graph (a placed prop's, or - on Fallout 4 - an actor's); prints whether the graph
/// accepted it and the animation now playing.
pub(crate) struct AnimEventCommand;

impl ConsoleCommand for AnimEventCommand {
    fn name(&self) -> &str {
        "anim.event"
    }

    fn description(&self) -> &str {
        "PlayAnimation from the console: anim.event <entity|.> <event>"
    }

    fn execute(&self, world: &World, args: &str) -> CommandOutput {
        let mut tokens = args.split_whitespace();
        let (Some(entity_token), Some(event)) = (tokens.next(), tokens.next()) else {
            return CommandOutput::error("usage: anim.event <entity|.> <event>");
        };
        let entity = match resolve_console_entity(world, entity_token) {
            Ok(entity) => entity,
            Err(message) => return CommandOutput::error(format!("anim.event: {message}")),
        };
        if !byroredux_scripting::animation_graph::has_graph(world, entity) {
            return CommandOutput::error("anim.event: the reference runs no behavior graph");
        }
        let accepted = byroredux_scripting::animation_graph::play_animation(world, entity, event);
        let clip = world
            .get::<byroredux_core::animation::AnimationPlayer>(entity)
            .map(|player| player.clip_handle);
        CommandOutput::line(format!("{event}: {} (clip {clip:?})", if accepted { "accepted" } else { "rejected" }))
    }
}
