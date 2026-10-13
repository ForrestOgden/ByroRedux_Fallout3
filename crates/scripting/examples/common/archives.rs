//! The script archives the coverage examples read, and the
//! [`ScriptLibrary`] over them — the coverage twin of the app's
//! `ScriptProvider`-backed library.

use byroredux_bsa::{Ba2Archive, BsaArchive};
use byroredux_pex::{decompile::decompile_script, parse};
use byroredux_scripting::translate::effects::ScriptLibrary;

pub enum Archive {
    Bsa(BsaArchive),
    Ba2(Ba2Archive),
}

impl Archive {
    pub fn open(path: &str) -> std::io::Result<Self> {
        if path.to_ascii_lowercase().ends_with(".ba2") {
            Ok(Archive::Ba2(Ba2Archive::open(path)?))
        } else {
            Ok(Archive::Bsa(BsaArchive::open(path)?))
        }
    }
    pub fn list_files(&self) -> Vec<&str> {
        match self {
            Archive::Bsa(a) => a.list_files(),
            Archive::Ba2(a) => a.list_files(),
        }
    }
    pub fn extract(&self, path: &str) -> std::io::Result<Vec<u8>> {
        match self {
            Archive::Bsa(a) => a.extract(path),
            Archive::Ba2(a) => a.extract(path),
        }
    }
}

/// Every opened archive's `.pex`, decompiled on demand — the coverage twin of
/// the app's `ScriptProvider`-backed library, so quest fragments inline calls
/// into their quest's own script exactly as production does.
pub struct ArchiveLibrary<'a> {
    archives: &'a [Archive],
    /// `scripts\a\b.pex` (lowercased, `:` namespaces as folders) →
    /// (archive, path). First archive wins, like `--scripts-bsa` order.
    index: std::collections::HashMap<String, (usize, String)>,
    cache: std::cell::RefCell<
        std::collections::HashMap<String, Option<std::sync::Arc<byroredux_papyrus::ast::Script>>>,
    >,
}

impl<'a> ArchiveLibrary<'a> {
    pub fn new(archives: &'a [Archive]) -> Self {
        let mut index = std::collections::HashMap::new();
        for (i, archive) in archives.iter().enumerate() {
            for file in archive.list_files() {
                let key = file.to_ascii_lowercase().replace('/', "\\");
                if key.ends_with(".pex") {
                    index.entry(key).or_insert((i, file.to_string()));
                }
            }
        }
        Self {
            archives,
            index,
            cache: Default::default(),
        }
    }
}

impl ScriptLibrary for ArchiveLibrary<'_> {
    fn script(&self, name: &str) -> Option<std::sync::Arc<byroredux_papyrus::ast::Script>> {
        let key = format!("scripts\\{}.pex", name.to_ascii_lowercase().replace(':', "\\"));
        if let Some(hit) = self.cache.borrow().get(&key) {
            return hit.clone();
        }
        let decoded = self.index.get(&key).and_then(|(archive, path)| {
            let data = self.archives[*archive].extract(path).ok()?;
            let pex = parse(&data).ok()?;
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decompile_script(&pex)))
                .ok()?
                .ok()
                .map(std::sync::Arc::new)
        });
        self.cache.borrow_mut().insert(key, decoded.clone());
        decoded
    }
}
