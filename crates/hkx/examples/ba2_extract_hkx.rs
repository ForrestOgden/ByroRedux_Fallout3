//! List the `.hkx` files of a BA2 whose path contains a substring, and optionally
//! write them (flattened names) into a directory: the raw material for deriving a
//! Havok class layout.
//!
//! ```text
//! cargo run --release -p byroredux-hkx --example ba2_extract_hkx -- \
//!     "<Fallout 4>/Data/Fallout4 - Animations.ba2" <path-substring> [save-dir]
//! ```

use byroredux_bsa::Ba2Archive;
fn main() {
    let mut a = std::env::args().skip(1);
    let ar = Ba2Archive::open(a.next().unwrap()).unwrap();
    let filter = a.next().unwrap().to_ascii_lowercase();
    let dir = a.next();
    for p in ar.list_files() {
        if p.to_ascii_lowercase().contains(&filter) && p.ends_with(".hkx") {
            let b = ar.extract(p).unwrap();
            println!("{p} {}", b.len());
            if let Some(dir) = &dir {
                std::fs::write(format!("{dir}/{}", p.replace('\\', "__")), &b).unwrap();
            }
        }
    }
}
