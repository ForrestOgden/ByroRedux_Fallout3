use byroredux_bsa::Ba2Archive;
use byroredux_hkx::{decode_behavior_graph, StateMachine};
fn walk(m: &StateMachine, depth: usize) {
    let pad = " ".repeat(depth * 2);
    let state = m.states.iter().find(|s| s.id == m.start_state).or(m.states.first());
    let Some(s) = state else { println!("{pad}(no states)"); return };
    println!("{pad}start state {} '{}' ({} states)", s.id, s.name, m.states.len());
    if let Some(c) = &s.clip { println!("{pad}  clip {} mode {:?}", c.animation, c.mode); }
    for c in &s.content.blended_clips { println!("{pad}  blended clip {}", c.animation); }
    for r in &s.content.behavior_references { println!("{pad}  ref {r}"); }
    for inner in &s.content.machines { walk(inner, depth + 1); }
}
fn main() {
    let mut a = std::env::args().skip(1);
    let ar = Ba2Archive::open(a.next().unwrap()).unwrap();
    let p = a.next().unwrap();
    let g = decode_behavior_graph(&ar.extract(&p).unwrap()).unwrap();
    println!("{} events", g.events.len());
    walk(g.machine.as_ref().unwrap(), 0);
}
