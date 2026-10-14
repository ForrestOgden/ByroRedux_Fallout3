//! Print the model path of base records by form id.
//!
//! Usage: cargo run --release -p byroredux-plugin --example base_model -- <ESM> <form hex>...

use byroredux_plugin::esm::parse_esm;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("an ESM path");
    let index = parse_esm(&std::fs::read(&path)?)?;
    for arg in args {
        let form = u32::from_str_radix(arg.trim_start_matches("0x"), 16).expect("hex");
        match index.cells.statics.get(&form) {
            Some(object) => println!("{form:08X} {} {}", object.editor_id, object.model_path),
            None => println!("{form:08X} not a model record"),
        }
    }
    Ok(())
}
