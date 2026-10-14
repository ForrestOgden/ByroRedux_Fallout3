//! Print the idle clip an actor standing in a Fallout 4 behavior graph starts in.
use byroredux_bsa::Ba2Archive;
fn main() {
    let mut args = std::env::args().skip(1);
    let archive = Ba2Archive::open(args.next().expect("archive")).expect("open");
    let path = args.next().expect("a behavior path");
    let graph = byroredux_hkx::decode_behavior_graph(&archive.extract(&path).expect("extract")).expect("decode");
    let mut loader = |reference: &str| {
        let path = format!("meshes\\actors\\character\\{}", reference.trim_start_matches(['\\', '/']).to_ascii_lowercase());
        let bytes = archive.extract(&path).ok()?;
        byroredux_hkx::decode_behavior_graph(&bytes).ok()
    };
    println!("{:?}", graph.start_clip(&mut loader).map(|clip| clip.animation));
}
