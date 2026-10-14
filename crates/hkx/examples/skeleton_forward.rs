//! Which way a skeleton's feet point in its own space: the horizontal direction from a
//! foot bone to the toe bone beyond it (names containing `foot` and `toe`), composed
//! from the reference pose. The rig's forward axis, without rendering it.
//!
//! ```text
//! cargo run -p byroredux-hkx --example skeleton_forward -- <skeleton.hkx>
//! ```
fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let t = [2.0 * (y * v[2] - z * v[1]), 2.0 * (z * v[0] - x * v[2]), 2.0 * (x * v[1] - y * v[0])];
    [
        v[0] + w * t[0] + (y * t[2] - z * t[1]),
        v[1] + w * t[1] + (z * t[0] - x * t[2]),
        v[2] + w * t[2] + (x * t[1] - y * t[0]),
    ]
}

fn mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn main() {
    let bytes = std::fs::read(std::env::args().nth(1).expect("a skeleton packfile")).expect("read");
    let skeleton = byroredux_hkx::decode_skeleton(&bytes).expect("decode");
    let mut world: Vec<([f32; 3], [f32; 4])> = Vec::new();
    for bone in &skeleton.bones {
        let local = &bone.reference_pose;
        let entry = match usize::try_from(bone.parent_index).ok().and_then(|parent| world.get(parent).copied()) {
            Some((position, rotation)) => {
                let moved = rotate(rotation, local.translation);
                ([position[0] + moved[0], position[1] + moved[1], position[2] + moved[2]], mul(rotation, local.rotation))
            }
            None => (local.translation, local.rotation),
        };
        world.push(entry);
    }
    let find = |needle: &str| skeleton.bones.iter().position(|bone| bone.name.to_ascii_lowercase().contains(needle) && bone.name.to_ascii_lowercase().contains('l'));
    for (foot, toe) in [("foot", "toe")] {
        let (Some(a), Some(b)) = (find(foot), find(toe)) else {
            println!("no {foot}/{toe} bone pair");
            continue;
        };
        let d = [world[b].0[0] - world[a].0[0], world[b].0[1] - world[a].0[1], world[b].0[2] - world[a].0[2]];
        println!("{} -> {}: {:?}", skeleton.bones[a].name, skeleton.bones[b].name, d);
    }
    for (index, bone) in skeleton.bones.iter().enumerate() {
        let name = bone.name.to_ascii_lowercase();
        if name.contains("foot") || name.contains("toe") || name.contains("calf") {
            println!("  {} {}: {:?}", index, bone.name, world[index].0);
        }
    }
}
