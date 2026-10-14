//! Print how far each of the first bones of a clip travels over its length: where an
//! animation keeps its locomotion (the bone to bind as the accumulation root).
//!
//! ```text
//! cargo run -p byroredux-hkx --example root_travel -- <skeleton.hkx> <clip.hkx> [bones]
//! ```
fn main() {
    let mut args = std::env::args().skip(1);
    let skeleton = byroredux_hkx::decode_skeleton(&std::fs::read(args.next().expect("skeleton")).expect("read")).expect("skeleton");
    let animation = byroredux_hkx::decode_spline_animation(&std::fs::read(args.next().expect("clip")).expect("read")).expect("clip");
    let bones: usize = args.next().map_or(4, |n| n.parse().expect("a count"));
    println!("duration {:.3}s, {} frames of {:.4}s", animation.duration, animation.tracks.first().map_or(0, Vec::len), animation.frame_duration);
    if let Some(frame) = &animation.reference_frame {
        println!("reference frame: {} samples over {:.3}s", frame.samples.len(), frame.duration);
        for (index, sample) in frame.samples.iter().enumerate().filter(|(i, _)| i % 6 == 0 || *i + 1 == frame.samples.len()) {
            println!("  {index:3}: {sample:?}");
        }
    }
    for (track, samples) in animation.tracks.iter().enumerate() {
        let bone = animation.track_to_bone[track] as usize;
        if bone >= bones {
            continue;
        }
        let (first, last) = (samples.first().unwrap().translation, samples.last().unwrap().translation);
        println!(
            "{} {}: first {:?} last {:?} travel {:?}",
            bone,
            skeleton.bones[bone].name,
            first,
            last,
            [last[0] - first[0], last[1] - first[1], last[2] - first[2]]
        );
    }
}
