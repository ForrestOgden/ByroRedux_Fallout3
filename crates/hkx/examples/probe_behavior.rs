//! Print the decoded behavior graph of a Skyrim SE `Behaviors\*.hkx` packfile
//! (and, for a `Characters\*.hkx` or project file, what it points at).
//!
//! ```text
//! cargo run -p byroredux-hkx --example probe_behavior -- <file.hkx> [more.hkx ...]
//! ```

use byroredux_hkx::{decode_behavior_graph, decode_character, decode_project};

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read");
        println!("== {path}");
        if let Ok(graph) = decode_behavior_graph(&bytes) {
            println!("  {} events: {:?}", graph.events.len(), graph.events);
            if let Some(machine) = &graph.machine {
                println!("  machine: start state {}", machine.start_state);
                for state in &machine.states {
                    println!("   state {} '{}'", state.id, state.name);
                    for t in &state.transitions {
                        println!(
                            "     on {:?} -> {} (flags {:#x})",
                            graph.event_name(t.event),
                            t.to_state,
                            t.flags
                        );
                    }
                    if let Some(clip) = &state.clip {
                        println!(
                            "     clip {} {:?} speed {} start {} fraction {}",
                            clip.animation, clip.mode, clip.playback_speed, clip.start_time, clip.user_fraction
                        );
                        for trigger in &clip.triggers {
                            println!(
                                "       trigger {:?} at {} (end-relative {}, annotation {})",
                                graph.event_name(trigger.event),
                                trigger.local_time,
                                trigger.relative_to_end,
                                trigger.annotation
                            );
                        }
                    }
                }
                for t in &machine.wildcard {
                    println!("   wildcard on {:?} -> {}", graph.event_name(t.event), t.to_state);
                }
            }
            continue;
        }
        if let Ok(character) = decode_character(&bytes) {
            println!("  character: {character:?}");
            continue;
        }
        match decode_project(&bytes) {
            Ok(characters) => println!("  project characters: {characters:?}"),
            Err(error) => println!("  not behavior data ({error})"),
        }
    }
}
