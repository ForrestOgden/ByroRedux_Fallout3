//! Print how many objects of each class a behavior packfile holds.
//!
//! ```text
//! cargo run -p byroredux-hkx --example class_census -- <file.hkx>
//! ```

fn main() {
    let path = std::env::args().nth(1).expect("a packfile");
    let bytes = std::fs::read(&path).expect("read");
    for (class, count) in byroredux_hkx::class_census(&bytes).expect("census") {
        println!("{count:6} {class}");
    }
}
