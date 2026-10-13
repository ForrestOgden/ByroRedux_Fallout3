//! Print one Story Manager event's tree in evaluation order — every node
//! with its kind, policies and condition function indices — as the built
//! `SmTree` walks it. A diagnostic for SM ordering / gating questions.
//!
//! ```bash
//! cargo run --release -p byroredux-scripting --example sm_tree_dump -- \
//!     "<Skyrim SE>/Data/Skyrim.esm" CLOC [depth]
//! ```

use byroredux_plugin::esm::parse_esm;
use byroredux_scripting::story_manager::{build_story_manager_tree, SmTree};

fn walk(tree: &SmTree, head: Option<usize>, depth: usize, max_depth: usize) {
    let mut cursor = head;
    while let Some(index) = cursor {
        let node = &tree.nodes[index];
        let functions: Vec<u32> = node.conditions.iter().map(|c| c.function_index).collect();
        println!(
            "{:indent$}{:08X} {:?} {} shares={} random={} conditions={functions:?}",
            "",
            node.form_id,
            node.kind,
            node.editor_id,
            node.policies.shares_event,
            node.policies.random,
            indent = depth * 2
        );
        if depth < max_depth {
            walk(tree, node.first_child, depth + 1, max_depth);
        }
        cursor = node.next_sibling;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let index = parse_esm(&std::fs::read(&args[0])?)?;
    let event: [u8; 4] = args[1].as_bytes().try_into()?;
    let max_depth = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(1);
    let tree = build_story_manager_tree(&index.story_manager_nodes);
    let root = tree.roots_by_mnemonic[&event];
    walk(&tree, tree.nodes[root].first_child, 0, max_depth);
    Ok(())
}
