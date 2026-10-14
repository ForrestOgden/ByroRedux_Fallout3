//! Dump the 8-byte slots of every object of a class in a behavior packfile:
//! the evidence for a Havok Behavior class layout.
//!
//! ```text
//! cargo run -p byroredux-hkx --example dump_objects -- <file.hkx> <class> [bytes-per-object]
//! ```

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("a packfile");
    let class = args.next().expect("a class name");
    let size: usize = args.next().map_or(0xA8, |n| n.parse().expect("a byte count"));
    let bytes = std::fs::read(&path).expect("read");
    for line in byroredux_hkx::dump_objects(&bytes, &class, size).expect("dump") {
        println!("{line}");
    }
}
