//! `script.var` - read a script variable the Papyrus VM wrote, from the console.

use super::shared::*;
use byroredux_scripting::quest_stages::QuestFormId;
use byroredux_scripting::QuestScriptVariables;

/// `script.call <entity|.> <function>`: run a function of the Papyrus scripts the VM
/// runs on a reference (the console twin of a script calling its own function, e.g.
/// `fireTrap` on a `PhysicsTrap`).
pub(crate) struct ScriptCallCommand;

impl ConsoleCommand for ScriptCallCommand {
    fn name(&self) -> &str {
        "script.call"
    }

    fn description(&self) -> &str {
        "Call a function of the Papyrus scripts on a reference: script.call <entity|.> <function>"
    }

    fn execute(&self, world: &World, args: &str) -> CommandOutput {
        let mut tokens = args.split_whitespace();
        let (Some(entity_token), Some(function)) = (tokens.next(), tokens.next()) else {
            return CommandOutput::error("usage: script.call <entity|.> <function>");
        };
        let entity = match resolve_console_entity(world, entity_token) {
            Ok(entity) => entity,
            Err(message) => return CommandOutput::error(format!("script.call: {message}")),
        };
        let queued = byroredux_scripting::raise_reference_call(world, entity, function, Vec::new());
        if queued == 0 {
            return CommandOutput::error("script.call: no Papyrus script runs on that reference");
        }
        CommandOutput::line(format!("queued {function} on {queued} script(s) of entity {entity}"))
    }
}

/// `script.qcall <script> <function> [entity]`: run a function of a quest script now,
/// with an actor argument (the console twin of a dialogue fragment calling into its quest
/// script, e.g. `FavorDialogueScript` `Bribe`).
pub(crate) struct ScriptQuestCallCommand;

impl ConsoleCommand for ScriptQuestCallCommand {
    fn name(&self) -> &str {
        "script.qcall"
    }

    fn description(&self) -> &str {
        "Run a quest script function now: script.qcall <script> <function> [actor entity]"
    }

    fn execute(&self, world: &World, args: &str) -> CommandOutput {
        let mut tokens = args.split_whitespace();
        let (Some(script), Some(function)) = (tokens.next(), tokens.next()) else {
            return CommandOutput::error("usage: script.qcall <script> <function> [actor entity]");
        };
        let mut call_args = Vec::new();
        if let Some(entity_token) = tokens.next() {
            let entity = match resolve_console_entity(world, entity_token) {
                Ok(entity) => entity,
                Err(message) => return CommandOutput::error(format!("script.qcall: {message}")),
            };
            let Some(form) = byroredux_scripting::condition::entity_global_form_id(world, entity) else {
                return CommandOutput::error("script.qcall: the entity is not a placed reference");
            };
            call_args.push(byroredux_scripting::translate::effects::VmStored::Form(form));
        }
        match byroredux_scripting::run_quest_script_function(world, script, function, call_args) {
            Ok(quest) => CommandOutput::line(format!("ran {script}.{function} on quest {quest:08X}")),
            Err(message) => CommandOutput::error(format!("script.qcall: {message}")),
        }
    }
}

/// `script.var <entity|.> <name>`: the value the VM's scripts wrote to `<name>`
/// (a property or variable) on the reference's script, else its authored value.
pub(crate) struct ScriptVarCommand;

impl ConsoleCommand for ScriptVarCommand {
    fn name(&self) -> &str {
        "script.var"
    }

    fn description(&self) -> &str {
        "Read a Papyrus script variable the VM wrote: script.var <entity|.> <name>"
    }

    fn execute(&self, world: &World, args: &str) -> CommandOutput {
        let mut tokens = args.split_whitespace();
        let (Some(entity_token), Some(name)) = (tokens.next(), tokens.next()) else {
            return CommandOutput::error("usage: script.var <entity|.> <name>");
        };
        let entity = match resolve_console_entity(world, entity_token) {
            Ok(entity) => entity,
            Err(message) => return CommandOutput::error(format!("script.var: {message}")),
        };
        let Some(form) = byroredux_scripting::condition::entity_global_form_id(world, entity) else {
            return CommandOutput::error("script.var: the entity is not a placed reference");
        };
        let key = QuestScriptVariables::property_key(name);
        let value = byroredux_scripting::quest_variable(world, QuestFormId(form), key);
        CommandOutput::line(format!("{form:08X}.{name} = {value:?}"))
    }
}
