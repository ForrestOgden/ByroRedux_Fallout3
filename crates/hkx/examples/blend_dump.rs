//! Print a behavior packfile's variables, the blend tree of every state that has
//! one, and the transitions that blend: the check behind the variable / blender /
//! transition-effect decode in `behavior.rs`.
//!
//! ```text
//! cargo run -p byroredux-hkx --example blend_dump -- <file.hkx>
//! ```

use byroredux_hkx::{decode_behavior_graph, BehaviorGraph, BlendNode, StateMachine};

fn variable(graph: &BehaviorGraph, index: Option<usize>) -> String {
    match index {
        Some(index) => graph.variables.get(index).map_or(format!("#{index}?"), |v| format!("${}", v.name)),
        None => String::new(),
    }
}

fn show_node(graph: &BehaviorGraph, node: &BlendNode, pad: &str) {
    match node {
        BlendNode::Clip(clip) => println!(
            "{pad}clip {} x{} {}",
            clip.animation,
            clip.playback_speed,
            variable(graph, clip.playback_speed_variable)
        ),
        BlendNode::Blender(blender) => {
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
                println!("{pad}  weight {} {}", child.weight, variable(graph, child.weight_variable));
                show_node(graph, &child.node, &format!("{pad}    "));
            }
        }
    }
}

fn show_machine(graph: &BehaviorGraph, machine: &StateMachine, depth: usize) {
    let pad = " ".repeat(depth * 2);
    for state in &machine.states {
        if let Some(blend) = &state.content.blend {
            println!("{pad}state {} '{}' blend:", state.id, state.name);
            show_node(graph, blend, &format!("{pad}  "));
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
        for inner in &state.content.machines {
            show_machine(graph, inner, depth + 1);
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("a behavior packfile");
    let bytes = std::fs::read(&path).expect("read");
    let graph = decode_behavior_graph(&bytes).expect("decode");
    println!("{} events, {} variables", graph.events.len(), graph.variables.len());
    for (index, v) in graph.variables.iter().enumerate() {
        println!("  var {index}: {} {:?} = {}", v.name, v.kind, v.initial);
    }
    if let Some(root) = &graph.machine {
        show_machine(&graph, root, 0);
    }
}
