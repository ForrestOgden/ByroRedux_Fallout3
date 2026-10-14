//! Print a behavior packfile's variables and, for every state, its generator tree (clips,
//! blenders with their weights and bone masks, selectors, nested machines) and the
//! transitions that blend: the check behind the decode in `behavior.rs`.
//!
//! ```text
//! cargo run -p byroredux-hkx --example blend_dump -- <file.hkx> [state-name-substring]
//! ```

use byroredux_hkx::{decode_behavior_graph, BehaviorGraph, Generator, Modifier, ModifierNode, State, StateMachine};
use std::sync::Arc;

fn variable(graph: &BehaviorGraph, index: Option<usize>) -> String {
    match index {
        Some(index) => graph.variables.get(index).map_or(format!("#{index}?"), |v| format!("${}", v.name)),
        None => String::new(),
    }
}

fn show_modifier(graph: &BehaviorGraph, node: &Arc<ModifierNode>, pad: &str) {
    let event = |id: i32| graph.event_name(id).unwrap_or("?").to_owned();
    let var = |index: usize| graph.variables.get(index).map_or("?".to_owned(), |v| v.name.clone());
    match &node.kind {
        Modifier::AssignVariables { floats, ints } => println!(
            "{pad}assign {:?} {:?}",
            floats.iter().map(|(v, x)| format!("{}={x}", var(*v))).collect::<Vec<_>>(),
            ints.iter().map(|(v, x)| format!("{}={x}", var(*v))).collect::<Vec<_>>()
        ),
        Modifier::ModifyOnce { on_activate, on_deactivate } => {
            println!("{pad}modify-once");
            for inner in on_activate.iter().chain(on_deactivate) {
                show_modifier(graph, inner, &format!("{pad}  "));
            }
        }
        Modifier::EventsFromRange { input_variable, lower_bound, ranges, .. } => println!(
            "{pad}events-from-range input={} lower={lower_bound} {:?}",
            input_variable.map_or("-".into(), var),
            ranges.iter().map(|r| format!("<={} -> {}{}", r.upper, event(r.event), if r.when_in_range { " (while)" } else { "" })).collect::<Vec<_>>()
        ),
        Modifier::EvaluateExpression { expressions } => println!("{pad}expressions {:?}", expressions.iter().map(|e| (&e.text, e.event_mode)).collect::<Vec<_>>()),
        Modifier::TimeStep { variable } => println!("{pad}time step -> {}", variable.map_or("-".into(), var)),
        Modifier::Timer { alarm_seconds, event: e } => println!("{pad}timer {alarm_seconds}s -> {}", event(*e)),
        Modifier::RandomAlarm { min_seconds, max_seconds, event: e, reset } => {
            println!("{pad}random alarm {min_seconds}..{max_seconds}s -> {} reset={reset}", event(*e))
        }
        Modifier::EventOnDeactivate { event: e } => println!("{pad}event-on-deactivate {}", event(*e)),
        Modifier::EventEveryNEvents { check, send, count } => println!("{pad}every {count}x {} -> {}", event(*check), event(*send)),
        Modifier::EventDriven { activate, deactivate, active_by_default, inner } => {
            println!("{pad}event-driven on {} off {} default={active_by_default}", event(*activate), event(*deactivate));
            if let Some(inner) = inner {
                show_modifier(graph, inner, &format!("{pad}  "));
            }
        }
        Modifier::List(items) => {
            println!("{pad}list");
            for item in items {
                show_modifier(graph, item, &format!("{pad}  "));
            }
        }
    }
}

fn show_generator(graph: &BehaviorGraph, state: &State, generator: &Generator, pad: &str) {
    match generator {
        Generator::Clip(clip) => println!(
            "{pad}clip {} x{} {}",
            clip.animation,
            clip.playback_speed,
            variable(graph, clip.playback_speed_variable)
        ),
        Generator::Sequence(name) => println!("{pad}sequence {name}"),
        Generator::Modified { modifiers, inner } => {
            for modifier in modifiers {
                show_modifier(graph, modifier, &format!("{pad}[mod #{}] ", modifier.id));
            }
            if let Some(inner) = inner {
                show_generator(graph, state, inner, pad);
            }
        }
        Generator::Machine(index) => println!("{pad}machine #{index} ({} states)", state.content.machines[*index].states.len()),
        Generator::Reference(ordinal) => println!(
            "{pad}reference '{}' -> machine {:?}",
            state.content.behavior_references[*ordinal],
            state.content.reference_machines.get(*ordinal).copied().flatten()
        ),
        Generator::Selector { index, index_variable, children } => {
            println!("{pad}selector index={index} {}", variable(graph, *index_variable));
            for child in children {
                show_generator(graph, state, child, &format!("{pad}    "));
            }
        }
        Generator::Blender(blender) => {
            println!(
                "{pad}blender parametric={} cyclic={} sync={} param={} {} range=[{}, {}]",
                blender.parametric,
                blender.cyclic,
                blender.sync,
                blender.parameter,
                variable(graph, blender.parameter_variable),
                blender.min_cyclic,
                blender.max_cyclic
            );
            for child in &blender.children {
                let mask = child.bone_weights.as_ref().map_or(String::new(), |bones| {
                    let on = bones.weights.iter().filter(|w| **w > 0.5).count();
                    format!(" mask({} bones, {on} on, {} variable-driven)", bones.weights.len(), bones.bound.len())
                });
                println!(
                    "{pad}  weight {} {}{mask} boneWeights<-{}",
                    child.weight,
                    variable(graph, child.weight_variable),
                    variable(graph, child.bone_weights_variable)
                );
                show_generator(graph, state, &child.generator, &format!("{pad}    "));
            }
        }
    }
}

fn show_machine(graph: &BehaviorGraph, machine: &StateMachine, depth: usize, filter: &Option<String>) {
    let pad = " ".repeat(depth * 2);
    println!("{pad}machine start {} {}", machine.start_state, variable(graph, machine.start_state_variable));
    for state in &machine.states {
        let wanted = filter.as_ref().is_none_or(|f| state.name.to_ascii_lowercase().contains(f));
        if wanted {
            if let Some(generator) = &state.content.generator {
                println!("{pad}state {} '{}':", state.id, state.name);
                show_generator(graph, state, generator, &format!("{pad}  "));
            }
            for transition in &state.transitions {
                if transition.blend_duration > 0.0 || transition.blend_duration_variable.is_some() {
                    println!(
                        "{pad}state {} '{}' --{}--> {}: blend {}s {}",
                        state.id,
                        state.name,
                        graph.event_name(transition.event).unwrap_or("?"),
                        transition.to_state,
                        transition.blend_duration,
                        variable(graph, transition.blend_duration_variable)
                    );
                }
            }
        }
        for inner in &state.content.machines {
            show_machine(graph, inner, depth + 1, filter);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a behavior packfile");
    let filter = args.next().map(|f| f.to_ascii_lowercase());
    let bytes = std::fs::read(&path).expect("read");
    let graph = decode_behavior_graph(&bytes).expect("decode");
    println!("{} events, {} variables", graph.events.len(), graph.variables.len());
    if filter.is_none() {
        for (index, v) in graph.variables.iter().enumerate() {
            println!("  var {index}: {} {:?} = {}", v.name, v.kind, v.initial);
        }
    }
    if let Some(root) = &graph.machine {
        show_machine(&graph, root, 0, &filter);
    }
}
