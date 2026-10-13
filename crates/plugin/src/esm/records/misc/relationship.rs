//! `RELA` relationship records (Skyrim+, M47.2) — the authored rank between
//! two NPCs that `GetRelationshipRank` reads.
//!
//! Layout per xEdit's `wbDefinitionsTES5` `RELA`: `DATA` = `{Parent NPC_,
//! Child NPC_, Rank u16, unknown u8, flags u8, Association Type}`. The rank
//! enum runs Lover (0) … Archnemesis (8); the CK *GetRelationshipRank* page
//! numbers the same ranks +4 Lover … −4 Archnemesis, so a wire rank `r` is
//! `4 − r`.

use super::super::common::remap_fid;
use crate::esm::reader::{FormIdRemap, SubRecord};
use crate::esm::sub_reader::SubReader;

/// One parsed `RELA`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RelaRecord {
    pub form_id: u32,
    /// The "Parent NPC" `NPC_`.
    pub parent: u32,
    /// The "Child NPC" `NPC_`.
    pub child: u32,
    /// The rank as `GetRelationshipRank` reports it, +4 Lover … −4
    /// Archnemesis.
    pub rank: i8,
    /// The `ASTP` association type, `0` for none.
    pub association: u32,
}

pub fn parse_rela(form_id: u32, subs: &[SubRecord], remap: &Option<FormIdRemap>) -> RelaRecord {
    let mut out = RelaRecord {
        form_id,
        ..Default::default()
    };
    for sub in subs {
        if &sub.sub_type == b"DATA" && sub.data.len() >= 10 {
            let mut r = SubReader::new(&sub.data);
            out.parent = remap_fid(r.u32_or_default(), remap);
            out.child = remap_fid(r.u32_or_default(), remap);
            let wire = r.u16_or_default();
            out.rank = 4 - wire.min(8) as i8;
            if sub.data.len() >= 16 {
                r.skip_or_eof(2);
                out.association = remap_fid(r.u32_or_default(), remap);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rela_data_decodes_the_rank_as_the_ck_numbers_it() {
        let mut data = Vec::new();
        data.extend_from_slice(&0x0001_3478u32.to_le_bytes());
        data.extend_from_slice(&0x0001_3BBAu32.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes()); // Friend
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&0x0001_2345u32.to_le_bytes());
        let subs = vec![SubRecord {
            sub_type: *b"DATA",
            data,
        }];
        let rela = parse_rela(0x10, &subs, &None);
        assert_eq!(
            rela,
            RelaRecord {
                form_id: 0x10,
                parent: 0x0001_3478,
                child: 0x0001_3BBA,
                rank: 1,
                association: 0x0001_2345,
            }
        );
    }
}
