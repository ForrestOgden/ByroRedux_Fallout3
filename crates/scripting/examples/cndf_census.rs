//! M47.2 — `ConditionForm.IsTrue` census over a Starfield master: how many
//! `CNDF` condition forms there are, which condition functions and run-on
//! targets they use, and how many the engine models end to end.
//!
//! ```bash
//! cargo run --release -p byroredux-scripting --example cndf_census -- \
//!     "<Starfield>/Data/Starfield.esm"
//! ```

use std::collections::BTreeMap;

use byroredux_plugin::esm::parse_esm;
use byroredux_scripting::condition::ConditionFunction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: cndf_census <Starfield.esm>")?;
    let index = parse_esm(&std::fs::read(&path)?)?;
    let mut functions: BTreeMap<u32, usize> = BTreeMap::new();
    let mut run_on: BTreeMap<String, usize> = BTreeMap::new();
    let (mut modeled, mut empty) = (0, 0);
    for form in index.condition_forms.values() {
        if form.conditions.is_empty() {
            empty += 1;
        }
        let mut all = true;
        for condition in &form.conditions {
            *functions.entry(condition.function_index).or_default() += 1;
            *run_on.entry(format!("{:?}", condition.run_on)).or_default() += 1;
            all &= !matches!(
                ConditionFunction::from_index(condition.function_index),
                ConditionFunction::Unknown(_)
            );
        }
        modeled += usize::from(all && !form.conditions.is_empty());
    }
    println!(
        "{} condition forms ({empty} empty); {modeled} use only catalogued functions",
        index.condition_forms.len()
    );
    println!("run-on targets: {run_on:?}");
    let mut by_count: Vec<_> = functions.into_iter().collect();
    by_count.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (function, count) in by_count.iter().take(25) {
        println!("  fn {function:>4}  {count:>6}  {:?}", ConditionFunction::from_index(*function));
    }
    Ok(())
}
