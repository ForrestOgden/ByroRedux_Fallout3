//! Print a skeleton packfile's bone names and parents.
//!
//! ```text
//! cargo run -p byroredux-hkx --example skeleton_bones -- <skeleton.hkx> [max]
//! ```
fn main() {
    let mut args = std::env::args().skip(1);
    let bytes = std::fs::read(args.next().expect("a skeleton packfile")).expect("read");
    let max: usize = args.next().map_or(12, |n| n.parse().expect("a count"));
    let skeleton = byroredux_hkx::decode_skeleton(&bytes).expect("decode");
    println!("{} bones", skeleton.bones.len());
    for (index, bone) in skeleton.bones.iter().take(max).enumerate() {
        println!("{index:3} {} (parent {:?})", bone.name, bone.parent_index);
    }
}
