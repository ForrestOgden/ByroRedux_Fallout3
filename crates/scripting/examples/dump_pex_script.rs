//! Print a decompiled `.pex` from a script archive as the Papyrus AST, one
//! statement per line — a quick look at what a recognizer sees.
//!
//! ```bash
//! cargo run --release -p byroredux-scripting --example dump_pex_script -- \
//!     "<Skyrim SE>/Data/Skyrim - Misc.bsa" defaultSetStageTrigScript
//! ```

use byroredux_bsa::BsaArchive;
use byroredux_papyrus::ast::{ScriptItem, StateItem, Stmt};
use byroredux_pex::{decompile::decompile_script, parse};

fn print_body(label: &str, body: &[byroredux_papyrus::span::Spanned<Stmt>]) {
    println!("== {label}");
    for stmt in body {
        let text = format!("{:?}", stmt.node)
            .replace("Spanned { node: ", "")
            .replace(", span: Span { start: 0, end: 0 } }", "")
            .replace("Identifier(", "")
            .replace("CallArg { name: None, value: ", "");
        println!("  {text}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let archive = BsaArchive::open(&args[0]).expect("archive opens");
    let want = format!("scripts\\{}.pex", args[1].to_ascii_lowercase());
    let path = archive
        .list_files()
        .into_iter()
        .find(|f| f.to_ascii_lowercase() == want)
        .expect("script in archive")
        .to_string();
    let script = decompile_script(&parse(&archive.extract(&path).unwrap()).unwrap()).unwrap();
    println!("ScriptName {} extends {:?}", script.name.node.0, script.parent.as_ref().map(|p| &p.node.0));
    for item in &script.body {
        match &item.node {
            ScriptItem::Property(p) => println!("  Property {} {:?} = {:?}", p.name.node.0, p.ty.node, p.initial_value.as_ref().map(|v| &v.node)),
            ScriptItem::Variable(v) => println!("  Var {} {:?} = {:?}", v.name.node.0, v.ty.node, v.initial_value.as_ref().map(|i| &i.node)),
            ScriptItem::Function(f) => print_body(&format!("Function {}", f.name.node.0), &f.body),
            ScriptItem::Event(e) => print_body(&format!("Event {}", e.name.node.0), &e.body),
            ScriptItem::State(state) => {
                for si in &state.body {
                    match &si.node {
                        StateItem::Function(f) => print_body(&format!("State {} / Function {}", state.name.node.0, f.name.node.0), &f.body),
                        StateItem::Event(e) => print_body(&format!("State {} / Event {}", state.name.node.0, e.name.node.0), &e.body),
                    }
                }
            }
            _ => {}
        }
    }
}
