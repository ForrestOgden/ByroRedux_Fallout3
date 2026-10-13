//! `LCTN` location records (Skyrim+) — the hierarchy and keywords the
//! location runtime reads (M47.2): `GetInCurrentLoc`, `LocationHasKeyword`,
//! the location aliases and the editor location of a persistent reference.
//!
//! Layout per xEdit's `wbDefinitionsTES5` `LCTN`: `FULL`, `KSIZ` + `KWDA`
//! keywords, `PNAM` parent location, and the reference tables. The `A*` and
//! `L*` spellings carry the same content (UESP *Skyrim Mod:Mod File
//! Format/LCTN*: the names changed with Dawnguard):
//!
//! - `ACPR` / `LCPR` population refs `{ref, cell-or-world, grid y, grid x}`
//!   (12 bytes) and `RCPR` bare refs — editor location: this location.
//! - `ACUN` / `LCUN` unique actors `{actor base, ref, location}` (12 bytes) —
//!   editor location: the struct's own location ("points to itself in most
//!   cases"), this one when null.
//! - `ACSR` / `LCSR` static refs `{location ref type, ref, cell-or-world,
//!   grid}` (16 bytes) — the location's `LCRT`-tagged references (boss,
//!   map marker, …).

use super::super::common::{remap_fid, CommonNamedFields};
use crate::esm::reader::{FormIdRemap, SubRecord};
use crate::esm::sub_reader::SubReader;

/// One parsed `LCTN`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LctnRecord {
    pub form_id: u32,
    pub editor_id: String,
    pub full_name: String,
    /// `KWDA` keyword FormIDs (`LocTypeDungeon`, `LocTypeCity`, …).
    pub keywords: Vec<u32>,
    /// `PNAM` parent location, `None` at the top of the hierarchy.
    pub parent: Option<u32>,
    /// `(reference, editor location)` for the `ACPR` / `LCPR` / `RCPR`
    /// population refs and the `ACUN` / `LCUN` unique actors.
    pub editor_location_refs: Vec<(u32, u32)>,
    /// `(location ref type, reference)` from `ACSR` / `LCSR`.
    pub ref_type_refs: Vec<(u32, u32)>,
}

pub fn parse_lctn(form_id: u32, subs: &[SubRecord], remap: &Option<FormIdRemap>) -> LctnRecord {
    let common = CommonNamedFields::from_subs_with_remap(subs, remap);
    let mut out = LctnRecord {
        form_id,
        editor_id: common.editor_id,
        full_name: common.full_name,
        ..Default::default()
    };
    for sub in subs {
        let mut r = SubReader::new(&sub.data);
        match &sub.sub_type {
            b"KWDA" => {
                while r.remaining() >= 4 {
                    out.keywords.push(remap_fid(r.u32_or_default(), remap));
                }
            }
            b"PNAM" if sub.data.len() >= 4 => {
                let parent = remap_fid(r.u32_or_default(), remap);
                out.parent = (parent != 0).then_some(parent);
            }
            // {ref, cell/world, grid y i16, grid x i16}
            b"ACPR" | b"LCPR" => {
                while r.remaining() >= 12 {
                    let reference = r.u32_or_default();
                    let _cell = r.u32_or_default();
                    let _grid = r.u32_or_default();
                    if reference != 0 {
                        out.editor_location_refs
                            .push((remap_fid(reference, remap), form_id));
                    }
                }
            }
            b"RCPR" => {
                while r.remaining() >= 4 {
                    let reference = r.u32_or_default();
                    if reference != 0 {
                        out.editor_location_refs
                            .push((remap_fid(reference, remap), form_id));
                    }
                }
            }
            // {actor base, ref, location}
            b"ACUN" | b"LCUN" => {
                while r.remaining() >= 12 {
                    let _actor = r.u32_or_default();
                    let reference = r.u32_or_default();
                    let location = r.u32_or_default();
                    if reference != 0 {
                        let location = match location {
                            0 => form_id,
                            location => remap_fid(location, remap),
                        };
                        out.editor_location_refs
                            .push((remap_fid(reference, remap), location));
                    }
                }
            }
            // {location ref type, ref, cell/world, grid y i16, grid x i16}
            b"ACSR" | b"LCSR" => {
                while r.remaining() >= 16 {
                    let ref_type = r.u32_or_default();
                    let reference = r.u32_or_default();
                    let _cell = r.u32_or_default();
                    let _grid = r.u32_or_default();
                    if ref_type != 0 && reference != 0 {
                        out.ref_type_refs
                            .push((remap_fid(ref_type, remap), remap_fid(reference, remap)));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::esm::records::test_support::{edid, sub};

    fn words(values: &[u32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn parses_keywords_parent_and_editor_location_refs() {
        let subs = vec![
            edid("WhiterunLocation"),
            sub(b"LCPR", words(&[0xA1, 0x3C, 0x0002_FFFC, 0xA2, 0x3C, 0])),
            sub(b"RCPR", words(&[0xA3])),
            sub(b"LCUN", words(&[0x0001_3BBF, 0xA4, 0x1_8A56, 0x0001_3BC0, 0xA5, 0])),
            sub(b"LCSR", words(&[0x0001_30F8, 0xA6, 0x3C, 0x0002_FFFC])),
            sub(b"KSIZ", words(&[2])),
            sub(b"KWDA", words(&[0x13168, 0x13166])),
            sub(b"PNAM", words(&[0x1_6772])),
        ];
        let record = parse_lctn(0x1_8A56, &subs, &None);
        assert_eq!(record.editor_id, "WhiterunLocation");
        assert_eq!(record.keywords, vec![0x13168, 0x13166]);
        assert_eq!(record.parent, Some(0x1_6772));
        let own = 0x1_8A56;
        assert_eq!(
            record.editor_location_refs,
            vec![(0xA1, own), (0xA2, own), (0xA3, own), (0xA4, 0x1_8A56), (0xA5, own)]
        );
        assert_eq!(record.ref_type_refs, vec![(0x0001_30F8, 0xA6)]);
    }

    #[test]
    fn a_null_parent_is_none_and_short_tables_stop_cleanly() {
        let subs = vec![
            sub(b"PNAM", words(&[0])),
            sub(b"LCPR", [1, 2, 3]),
        ];
        let record = parse_lctn(1, &subs, &None);
        assert_eq!(record.parent, None);
        assert!(record.editor_location_refs.is_empty());
    }
}
