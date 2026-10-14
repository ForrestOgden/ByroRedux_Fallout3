//! Print the decoded behavior graph(s) of an animation archive whose path
//! contains the given substring: events, states, transitions and clips.
//!
//! ```text
//! cargo run --release -p byroredux-hkx --example graph_dump -- \
//!     "<Skyrim SE>/Data/Skyrim - Animations.bsa" <path-substring> [save-dir]
//! ```

use byroredux_bsa::BsaArchive;
use byroredux_hkx::decode_behavior_graph;

fn main() {
    let mut args = std::env::args().skip(1);
    let archive = BsaArchive::open(args.next().expect("archive path")).expect("open");
    let filter = args.next().expect("a path substring").to_ascii_lowercase();
    // Optional third argument: also write each matching file's bytes here.
    let save_dir = args.next();
    for path in archive.list_files() {
        let lower = path.to_ascii_lowercase();
        if !lower.contains(&filter) || !lower.ends_with(".hkx") {
            continue;
        }
        let Ok(bytes) = archive.extract(&path) else { continue };
        if let Some(dir) = &save_dir {
            let name = path.rsplit('\\').next().unwrap_or(&path);
            std::fs::write(std::path::Path::new(dir).join(name), &bytes).expect("write");
        }
        let Ok(graph) = decode_behavior_graph(&bytes) else { continue };
        println!("== {path}");
        println!("events: {:?}", graph.events);
        let Some(machine) = &graph.machine else {
            println!("(no state machine)");
            continue;
        };
        println!("start {} ; wildcard {:?}", machine.start_state, machine.wildcard);
        for state in &machine.states {
            println!("state {} '{}'", state.id, state.name);
            for t in &state.transitions {
                println!("    on {:?} -> {} (flags {:#x}, priority {})", graph.event_name(t.event), t.to_state, t.flags, t.priority);
            }
            match &state.clip {
                Some(clip) => println!("    clip {:?} mode {:?} speed {} triggers {}", clip.animation, clip.mode, clip.playback_speed, clip.triggers.len()),
                None => println!("    (no clip)"),
            }
        }
    }
}
