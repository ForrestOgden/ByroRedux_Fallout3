//! Shared `SubRecord` fixture builders for record-parser tests.
//!
//! #3865 — these two lines were redeclared 33 times across 30 files, under
//! three names (`sub`, `mk_sub`, `make_sub`) and four incompatible
//! signatures (`&[u8; 4]` vs `[u8; 4]`, `&[u8]` vs `Vec<u8>` vs
//! `impl Into<Vec<u8>>`). The cost was not risk — every copy was correct —
//! but a test could not be moved between two files without editing it, and
//! a new record parser's test module opened with boilerplate instead of a
//! test.
//!
//! [`sub`] takes `impl AsRef<[u8]>` rather than the narrower
//! `impl Into<Vec<u8>>` the issue sketched, so that every existing call site
//! compiles unchanged. `Into<Vec<u8>>` looks wider but is not: the copies it
//! replaces took `&[u8]`, and a `&Vec<u8>` argument reached them by deref
//! coercion at the call site — a coercion that does not happen through a
//! generic bound. `AsRef<[u8]>` covers `Vec<u8>`, `&[u8]`, `&[u8; N]` and
//! `&Vec<u8>` alike. The copy it forces is irrelevant in a fixture builder.

use crate::esm::reader::SubRecord;

/// One sub-record from its 4-CC type and payload.
pub(crate) fn sub(typ: &[u8; 4], data: impl AsRef<[u8]>) -> SubRecord {
    SubRecord {
        sub_type: *typ,
        data: data.as_ref().to_vec(),
    }
}

/// A null-terminated `EDID` (editor id) sub-record.
pub(crate) fn edid(name: &str) -> SubRecord {
    zstring(b"EDID", name)
}

/// A null-terminated `MODL` (model path) sub-record.
pub(crate) fn modl(path: &str) -> SubRecord {
    zstring(b"MODL", path)
}

/// A null-terminated string payload — the shape every `edid` / `modl` copy
/// open-coded. `pub(crate)` so a test needing another 4-CC (`FNAM`, a
/// `MODT` path, …) reaches for this instead of re-wrapping [`sub`] (#5454).
pub(crate) fn zstring(typ: &[u8; 4], value: &str) -> SubRecord {
    let mut z = value.as_bytes().to_vec();
    z.push(0);
    sub(typ, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_accepts_every_shape_the_local_copies_took() {
        // The four signatures #3865 consolidated, exercised as arguments.
        let owned: Vec<u8> = vec![1, 2, 3];
        let borrowed: &[u8] = &owned;
        assert_eq!(sub(b"EDID", b"x\0").data, b"x\0".to_vec());
        assert_eq!(sub(b"DATA", owned.clone()).data, owned);
        assert_eq!(sub(b"DATA", borrowed).data, owned);
        // `&Vec<u8>` is the one `impl Into<Vec<u8>>` would have rejected —
        // the local copies took `&[u8]` and got here by deref coercion at
        // the call site, which a generic bound does not perform.
        assert_eq!(sub(b"DATA", &owned).data, owned);
        assert_eq!(sub(b"EDID", b"x\0").sub_type, *b"EDID");
    }

    #[test]
    fn zstring_helpers_null_terminate() {
        assert_eq!(edid("Foo").data, b"Foo\0".to_vec());
        assert_eq!(edid("Foo").sub_type, *b"EDID");
        assert_eq!(modl("a\\b.nif").data, b"a\\b.nif\0".to_vec());
        assert_eq!(modl("a\\b.nif").sub_type, *b"MODL");
    }

    /// #3865 / #5454 — the consolidation only holds if a new record parser's
    /// test module reaches for this file instead of opening with its own
    /// copy. 33 copies across 30 files is what "everyone writes their own"
    /// looks like after enough parsers, and each one was individually
    /// reasonable.
    ///
    /// Two checks, because a name allowlist alone is what let `sub_of`
    /// through (#5454): a builder is flagged when it is *called* one of the
    /// known names, OR when its body is the generic shape — a fn returning
    /// `SubRecord` that builds the struct from a **parameter** 4-CC
    /// (`sub_type: *code`). Literal-4-CC builders (`sub_type: *b"CTDA"`) are
    /// fixtures for one record kind, not copies of [`sub`], and are fine.
    ///
    /// Scans the crate source at test time rather than by `include_str!` —
    /// 30 files cannot be enumerated by hand without reintroducing exactly
    /// the maintenance burden being removed, and a new file must be covered
    /// the day it is written.
    #[test]
    fn no_module_redeclares_a_local_subrecord_builder() {
        // Assembled at run time so this test's own text is not a match.
        let needles: Vec<String> = ["sub", "mk_sub", "make_sub", "edid", "modl"]
            .iter()
            .map(|n| format!("{} {n}(", "fn"))
            .collect();
        let generic_body = format!("{}: *", "sub_type");

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        let mut scanned = 0usize;
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("crate src must be readable") {
                let path = entry.expect("readable entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs") {
                    continue;
                }
                if path.file_name().is_some_and(|n| n == "test_support.rs") {
                    continue;
                }
                scanned += 1;
                let src = std::fs::read_to_string(&path).expect("readable source");
                let lines: Vec<&str> = src.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    if !line.contains("-> SubRecord") {
                        continue;
                    }
                    let by_name = needles.iter().any(|n| line.contains(n.as_str()));
                    // The struct literal sits within the first few lines
                    // of a builder body; a lowercase `*b"…"` literal never
                    // matches because the needle ends at `*` + non-`b`.
                    let by_body = lines[i + 1..lines.len().min(i + 6)].iter().any(|body| {
                        body.contains(generic_body.as_str())
                            && !body.contains(&format!("{}b\"", generic_body))
                    });
                    if by_name || by_body {
                        offenders.push(format!("{}:{}", path.display(), i + 1));
                    }
                }
            }
        }

        assert!(
            scanned > 50,
            "the crate scan found only {scanned} files — the walk broke, not the tree",
        );
        assert!(
            offenders.is_empty(),
            "these modules declare their own `SubRecord` fixture builder instead of \
             using `esm::records::test_support` (#3865, #5454): {offenders:#?}",
        );
    }
}
