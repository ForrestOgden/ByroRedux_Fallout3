//! `bribe.info` - the inputs of `Actor.GetBribeAmount`, read from the live actors.

use super::shared::*;

/// `bribe.info <entity|.>`: the NPC's and the player's levels, Speechcraft, and the NPC's
/// Morality as `GetBribeAmount` reads them, and the amount it computes.
pub(crate) struct BribeInfoCommand;

impl ConsoleCommand for BribeInfoCommand {
    fn name(&self) -> &str {
        "bribe.info"
    }

    fn description(&self) -> &str {
        "Inputs and result of GetBribeAmount: bribe.info <entity|.>"
    }

    fn execute(&self, world: &World, args: &str) -> CommandOutput {
        let Some(entity_token) = args.split_whitespace().next() else {
            return CommandOutput::error("usage: bribe.info <entity|.>");
        };
        let entity = match resolve_console_entity(world, entity_token) {
            Ok(entity) => entity,
            Err(message) => return CommandOutput::error(format!("bribe.info: {message}")),
        };
        match byroredux_scripting::bribe::bribe_inputs(world, entity) {
            Some(i) => CommandOutput::line(format!(
                "npc level {} speechcraft {} morality {}; player level {} speechcraft {}; amount {}",
                i.npc_level, i.npc_speechcraft, i.npc_morality, i.player_level, i.player_speechcraft, i.amount
            )),
            None => CommandOutput::error("bribe.info: no bribery rules, or the actor or the player has no level"),
        }
    }
}
