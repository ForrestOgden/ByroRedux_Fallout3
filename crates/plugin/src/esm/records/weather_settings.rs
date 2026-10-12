//! WTHS (Weather Settings) record parser — Starfield's replacement for
//! the WTHR weather type (#5364).
//!
//! A WTHS is an `EDID` + BGS-parameter-blob pair, not the WTHR
//! sub-record schema. The blob rides the `REFL` sub-record on root
//! ("template") records and the `RDIF` (Reflection Diff) sub-record on
//! child records that point up an `RFDP` (Reflection Parent) chain:
//! vanilla `Starfield.esm` ships 181 WTHS — 18 roots with a full REFL
//! (`WeatherClearTemplate`, `DefaultWeatherSettings`, …) and 163
//! children carrying an RFDP + RDIF (e.g. Akila's
//! `WeatherUniqueAkila_Clear_C0_Clear` → `Weather_Clear_C0_Clear` →
//! `WeatherClearTemplate`).
//!
//! The blob's grammar is the BGS **reflection** container, recovered
//! from xEdit's `wbDefinitionsSF1.pas` (`wbReflectionChunk`, the
//! reflection-block draft — still commented out upstream at
//! dev-4.1.5) and validated byte-for-byte against `Starfield.esm` with
//! the `dump_wths_refl` example:
//!
//! ```text
//! container  := chunk*
//! chunk      := sig: char[4], size: u32, payload: byte[size]
//! BETH       := version: u32, chunk_count: u32          // first chunk
//! STRT       := (nul-terminated string)*                 // string table;
//!                                                        // name refs are
//!                                                        // i32 byte offsets
//! TYPE       := class_count: u32
//! CLAS       := name: i32, type: i32, flags: u16, field_count: u16,
//!               field_count × { name: i32, type: i32, offset: u16, size: u16 }
//! DIFF       := type: u32, (field_index: u16, value)*, 0xFFFF   // RDIF only
//! OBJT/LIST/USER/USRD/MAPC                           // the instance values
//! ```
//!
//! A `DIFF` is an object diff, not a flat index list (#5514): the type
//! ref names the class (through its string-table name offset), then
//! each overridden field follows as `(u16 index, inline value)` until
//! the `0xFFFF` terminator. Inline value sizes follow the CDB reader's
//! skip rules (`crates/sfmaterial/src/reader.rs`): built-ins by size,
//! `String` by its `u16` length prefix, `Ref` by recursing its target,
//! and a non-user class (struct) as a nested diff-pair stream — diff
//! semantics propagate into overridden structs. Chunk-typed fields
//! (List/Map built-ins, user-flagged classes) carry no inline bytes:
//! their values ride the side chunks after the terminator, which the
//! container census keeps counting as instance chunks.
//!
//! `name`/`type` refs are i32 string-table offsets, or the *negative*
//! built-in type ids (`0xFFFFFF02` String, `0xFFFFFF0D` UInt32,
//! `0xFFFFFF11` Float, … — the full table in
//! [`builtin_type_label`]). CLAS flags: bit 2 `User`, bit 3 `Struct`.
//!
//! **Scope of this decode.** The container walk plus the schema half —
//! string table, class list, every field's name/type/offset/size — is
//! decoded here. The instance half (`OBJT`/`LIST`/`USER` chunks, the
//! actual parameter values: fog distances, colors, wind) is censused
//! but not interpreted; xEdit has no shipped reader for it either, and
//! grounding those layouts is the tracked follow-up (#5627; #5364 landed
//! the schema half only).
//! Until a value reader lands, exteriors on WTHS-only climates keep
//! resolving weather through the authored `DefaultWeather` WTHR
//! (#5363's fallback).

use super::common::{read_zstring, remap_fid};
use crate::esm::reader::{FormIdRemap, SubRecord};
use crate::esm::sub_reader::SubReader;

/// Built-in reflection type ids as the raw i32 stored on disk
/// (`0xFFFFFF02` = -254 = String, `0xFFFFFF0D` = -243 = UInt32,
/// `0xFFFFFF11` = -239 = Float, …). The same values
/// `byroredux-sfmaterial`'s `BuiltinType` names as `u32`; this crate does
/// not depend on that one, so the ids live here once and
/// [`BUILTINS`] is the only table that maps them.
mod builtin {
    pub const NULL: i32 = -255;
    pub const STRING: i32 = -254;
    pub const LIST: i32 = -253;
    pub const MAP: i32 = -252;
    pub const REF: i32 = -251;
    pub const INT8: i32 = -248;
    pub const UINT8: i32 = -247;
    pub const INT16: i32 = -246;
    pub const UINT16: i32 = -245;
    pub const INT32: i32 = -244;
    pub const UINT32: i32 = -243;
    pub const INT64: i32 = -242;
    pub const UINT64: i32 = -241;
    pub const BOOL: i32 = -240;
    pub const FLOAT: i32 = -239;
    pub const DOUBLE: i32 = -238;
}

/// CLAS flag bit 2 `User`: the class's values ride side chunks, nothing
/// inline in a diff stream (the CDB reader's `ClassFlags::IS_USER`).
const CLASS_FLAG_USER: u16 = 1 << 2;

/// A DIFF stream's field-index terminator.
const DIFF_TERMINATOR: u16 = 0xFFFF;

/// #5687 — ceiling on struct / `Ref` nesting while skipping a diff
/// value. `skip_diff_value` recurses on file-controlled data (a struct
/// field may name its own class; a `Ref` target may be `Ref` again), each
/// level costing only 2–4 payload bytes, so a hostile `RDIF` blob recursed
/// ~payload/2 deep and aborted the process with a stack overflow. Mirrors
/// `crates/sfmaterial/src/reader.rs`'s `MAX_NESTING_DEPTH` (duplicated
/// because this crate does not depend on that one, #5615). Vanilla
/// `RDIF`s are shallow — 163/163 decode — so 64 is ample.
const MAX_NESTING_DEPTH: usize = 64;

/// How a built-in's value is laid out inline in a diff stream.
#[derive(Clone, Copy)]
enum InlineValue {
    /// Nothing inline: `Null`, and `List` / `Map` (chunk-typed side chunks).
    Nothing,
    /// A fixed number of bytes.
    Fixed(usize),
    /// `u16` length prefix, then that many bytes.
    LengthPrefixedString,
    /// `i32` target type, then the target's own value (a List/Map target is
    /// the null sentinel — nothing inline).
    Ref,
}

/// Every built-in: raw id, label, inline layout. One table so the label
/// and the value-skip walk cannot disagree about which ids exist.
const BUILTINS: [(i32, &str, InlineValue); 16] = [
    (builtin::NULL, "Null", InlineValue::Nothing),
    (builtin::STRING, "String", InlineValue::LengthPrefixedString),
    (builtin::LIST, "List", InlineValue::Nothing),
    (builtin::MAP, "Map", InlineValue::Nothing),
    (builtin::REF, "Ref", InlineValue::Ref),
    (builtin::INT8, "Int8", InlineValue::Fixed(1)),
    (builtin::UINT8, "UInt8", InlineValue::Fixed(1)),
    (builtin::INT16, "Int16", InlineValue::Fixed(2)),
    (builtin::UINT16, "UInt16", InlineValue::Fixed(2)),
    (builtin::INT32, "Int32", InlineValue::Fixed(4)),
    (builtin::UINT32, "UInt32", InlineValue::Fixed(4)),
    (builtin::INT64, "Int64", InlineValue::Fixed(8)),
    (builtin::UINT64, "UInt64", InlineValue::Fixed(8)),
    (builtin::BOOL, "Bool", InlineValue::Fixed(1)),
    (builtin::FLOAT, "Float", InlineValue::Fixed(4)),
    (builtin::DOUBLE, "Double", InlineValue::Fixed(8)),
];

fn builtin_entry(raw: i32) -> Option<&'static (i32, &'static str, InlineValue)> {
    BUILTINS.iter().find(|(id, _, _)| *id == raw)
}

pub fn builtin_type_label(raw: i32) -> Option<&'static str> {
    builtin_entry(raw).map(|(_, label, _)| *label)
}

/// One field of a reflection class: name and type resolved through the
/// string table (or the built-in label), plus the authored offset/size
/// into the class's instance layout. `type_ref` keeps the raw i32 —
/// negative = built-in id, non-negative = the referenced class's
/// string-table name offset — because inline value sizes (a DIFF walk,
/// the future values reader) derive from the raw ref, not the label.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReflectionField {
    pub name: String,
    pub type_name: String,
    pub type_ref: i32,
    pub offset: u16,
    pub size: u16,
}

/// One CLAS chunk: a named class with its field list.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReflectionClass {
    /// The class's name string-table offset — the key a `DIFF` type ref
    /// resolves through (Gibbed's `typeMap` is indexed by `NameOffset`).
    pub name_offset: i32,
    pub name: String,
    /// Bit 2 `User`, bit 3 `Struct` (xEdit's flag table; vanilla roots
    /// mark `XMFLOAT4`-style value types as Struct).
    pub flags: u16,
    pub fields: Vec<ReflectionField>,
}

/// A decoded BGS reflection container (the `REFL`/`RDIF` payload).
#[derive(Debug, Clone, Default)]
pub struct BgsReflection {
    /// From the leading `BETH` chunk.
    pub version: u32,
    /// Chunk count declared by `BETH` (sanity: the walk's chunk tally
    /// must reach it — a mismatch means an unknown chunk kind).
    pub chunk_count: u32,
    /// The `STRT` string table, in authored order.
    pub strings: Vec<String>,
    /// `TYPE`'s class count (must equal `classes.len()`).
    pub class_count: u32,
    /// Every `CLAS` chunk, in authored order.
    pub classes: Vec<ReflectionClass>,
    /// `DIFF` field indices (RDIF blobs only): the overridden fields the
    /// diff pairs name, with each pair's inline value skipped by its
    /// field type up to the `0xFFFF` terminator (#5514 — value bytes
    /// and the terminator no longer pollute the list). `None` when the
    /// blob carries no DIFF chunk, or the diff walk could not resolve
    /// the class / ran past the chunk (vanilla never trips either).
    pub diff_field_indices: Option<Vec<u16>>,
    /// The instance chunks (`OBJT`/`LIST`/`USER`/`USRD`/`MAPC`),
    /// censused as `(signature, payload size)` — the value half is the
    /// tracked follow-up (#5627), and this census is the evidence base for it.
    pub instance_chunks: Vec<([u8; 4], usize)>,
    /// Signatures of chunks the walker did not recognize (skipped by
    /// size). Empty on vanilla data.
    pub unknown_chunks: Vec<[u8; 4]>,
}

impl BgsReflection {
    /// The class named exactly `BGSWeatherSettingsForm` — the root of
    /// every weather-settings blob (its field list is the weather's
    /// parameter schema: fog, colors, wind, precipitation, …).
    pub fn weather_root_class(&self) -> Option<&ReflectionClass> {
        self.classes.iter().find(|c| c.name == "BGSWeatherSettingsForm")
    }
}

/// Parsed WTHS record.
#[derive(Debug, Clone, Default)]
pub struct WeatherSettingsRecord {
    pub form_id: u32,
    pub editor_id: String,
    /// The `REFL` container — full reflection, on root (template)
    /// records. `None` on children, which carry `reflection_diff` +
    /// `reflection_parent` instead.
    pub reflection: Option<BgsReflection>,
    /// The `RDIF` container — same grammar, plus a `DIFF` chunk naming
    /// the overridden fields. `None` on roots.
    pub reflection_diff: Option<BgsReflection>,
    /// `RFDP` — the parent WTHS this record diffs against. Load-order
    /// remapped like every cross-record FormID. `None` on roots.
    pub reflection_parent: Option<u32>,
}

/// Resolve a name/type reference: negative = built-in type id,
/// non-negative = byte offset into the decoded string table.
fn resolve_ref(raw: i32, strings: &[(usize, String)]) -> String {
    if raw < 0 {
        return builtin_type_label(raw)
            .map(str::to_string)
            .unwrap_or_else(|| format!("<builtin {raw:#010x}>"));
    }
    let off = raw as usize;
    // The table is sorted by construction (offsets ascend with the
    // string list); binary search the last entry whose offset <= off.
    match strings.binary_search_by(|(o, _)| o.cmp(&off)) {
        Ok(i) => strings[i].1.clone(),
        Err(0) => format!("<offset {off}>"),
        Err(i) => strings[i - 1].1.clone(),
    }
}

/// Walk the reflection container in `blob` (the REFL/RDIF payload).
/// Tolerant: a chunk whose declared size overflows the blob, or a CLAS
/// whose field walk runs past its chunk, ends the walk with everything
/// decoded so far (vanilla data never trips either).
pub fn parse_bgs_reflection(blob: &[u8]) -> BgsReflection {
    let mut out = BgsReflection::default();
    // (byte offset, string) pairs — the i32 refs index this by offset.
    let mut table: Vec<(usize, String)> = Vec::new();
    let mut off = 0usize;
    while off + 8 <= blob.len() {
        let sig: [u8; 4] = blob[off..off + 4].try_into().unwrap();
        let size = u32::from_le_bytes(blob[off + 4..off + 8].try_into().unwrap()) as usize;
        if off + 8 + size > blob.len() {
            // Overflowing chunk — record nothing further; the tally
            // mismatch against `chunk_count` is the diagnostic.
            break;
        }
        let payload = &blob[off + 8..off + 8 + size];
        match &sig {
            b"BETH" if payload.len() >= 8 => {
                let mut r = SubReader::new(payload);
                out.version = r.u32_or_default();
                out.chunk_count = r.u32_or_default();
            }
            b"STRT" => {
                let mut p = 0usize;
                while p < payload.len() {
                    let end = payload[p..]
                        .iter()
                        .position(|&b| b == 0)
                        .map(|e| p + e)
                        .unwrap_or(payload.len());
                    table.push((p, read_zstring(&payload[p..end])));
                    p = end + 1;
                }
            }
            b"TYPE" if payload.len() >= 4 => {
                out.class_count = u32::from_le_bytes(payload[0..4].try_into().unwrap());
            }
            b"CLAS" if payload.len() >= 12 => {
                let name = i32::from_le_bytes(payload[0..4].try_into().unwrap());
                let flags = u16::from_le_bytes(payload[8..10].try_into().unwrap());
                let field_count = u16::from_le_bytes(payload[10..12].try_into().unwrap()) as usize;
                let mut class = ReflectionClass {
                    name_offset: name,
                    name: resolve_ref(name, &table),
                    flags,
                    ..ReflectionClass::default()
                };
                let mut p = 12usize;
                for _ in 0..field_count {
                    let Some(field) = payload.get(p..p + 12) else { break };
                    let type_ref = i32::from_le_bytes(field[4..8].try_into().unwrap());
                    class.fields.push(ReflectionField {
                        name: resolve_ref(i32::from_le_bytes(field[0..4].try_into().unwrap()), &table),
                        type_ref,
                        type_name: resolve_ref(type_ref, &table),
                        offset: u16::from_le_bytes(field[8..10].try_into().unwrap()),
                        size: u16::from_le_bytes(field[10..12].try_into().unwrap()),
                    });
                    p += 12;
                }
                out.classes.push(class);
            }
            b"DIFF" if payload.len() >= 4 => {
                // #5514 — object-diff walk, not a flat u16 list: the
                // leading type ref names the class (by string-table
                // name offset), then (u16 field index, inline value)
                // pairs until the 0xFFFF terminator. Values are skipped
                // by field type; chunk-typed fields defer to the side
                // chunks the container census keeps counting.
                let type_ref = i32::from_le_bytes(payload[0..4].try_into().unwrap());
                let Some(class) = out.classes.iter().find(|c| c.name_offset == type_ref) else {
                    // Unresolvable class — the diff layout is unknowable.
                    // Leave `diff_field_indices` unset rather than guess.
                    continue;
                };
                let class = class.clone();
                if let Some(indices) = collect_diff_indices(payload, &class, &out.classes) {
                    out.diff_field_indices = Some(indices);
                }
            }
            b"OBJT" | b"LIST" | b"USER" | b"USRD" | b"MAPC" => {
                out.instance_chunks.push((sig, size));
            }
            _ => {
                out.unknown_chunks.push(sig);
            }
        }
        off += 8 + size;
    }
    out.strings = table.into_iter().map(|(_, s)| s).collect();
    out
}

/// Walk a `DIFF` chunk payload (after its leading type ref) as the
/// CDB reader does (`reader.rs`'s diff loop): collect each overridden
/// field index, skip its inline value by field type, stop at `0xFFFF`.
/// `None` when the stream truncates or names an out-of-range field —
/// callers keep the unset state rather than a half-guessed list.
fn collect_diff_indices(
    payload: &[u8],
    class: &ReflectionClass,
    classes: &[ReflectionClass],
) -> Option<Vec<u16>> {
    let mut pos = 4usize;
    let mut indices = Vec::new();
    loop {
        let idx = read_i16(payload, &mut pos)?;
        if idx == DIFF_TERMINATOR {
            return Some(indices);
        }
        let field = class.fields.get(idx as usize)?;
        indices.push(idx);
        if !skip_diff_value(payload, &mut pos, field.type_ref, classes, 0) {
            return None;
        }
    }
}

/// Skip one inline diff value of `type_ref`, the CDB reader's rules
/// (`skip_value`): built-ins by size, `String` by its `u16` length
/// prefix, `Ref` by recursing its target (a List/Map ref is the null
/// sentinel — nothing inline), user-flagged classes ride side chunks,
/// and a non-user class (struct) carries a nested diff-pair stream —
/// diff semantics propagate into overridden structs. Returns `false`
/// on truncation or when nesting exceeds [`MAX_NESTING_DEPTH`]
/// (both struct and `Ref` edges recurse on file-controlled types,
/// #5687).
fn skip_diff_value(
    src: &[u8],
    pos: &mut usize,
    type_ref: i32,
    classes: &[ReflectionClass],
    depth: usize,
) -> bool {
    if depth >= MAX_NESTING_DEPTH {
        return false;
    }
    if type_ref >= 0 {
        let Some(class) = classes.iter().find(|c| c.name_offset == type_ref) else {
            return false;
        };
        if class.flags & CLASS_FLAG_USER != 0 {
            return true; // User class — side chunk, nothing inline.
        }
        // Struct — nested diff-pair stream (reader.rs' diff loop recurses
        // with the same is_diff flag).
        let mut nested_pos = *pos;
        loop {
            let Some(idx) = read_i16(src, &mut nested_pos) else {
                return false;
            };
            if idx == DIFF_TERMINATOR {
                *pos = nested_pos;
                return true;
            }
            let Some(field) = class.fields.get(idx as usize) else {
                return false;
            };
            if !skip_diff_value(src, &mut nested_pos, field.type_ref, classes, depth + 1) {
                return false;
            }
        }
    }
    let Some(&(_, _, inline)) = builtin_entry(type_ref) else {
        return false; // unknown built-in — refuse to guess
    };
    match inline {
        InlineValue::Nothing => true,
        InlineValue::LengthPrefixedString => match src.get(*pos..*pos + 2) {
            Some(len) => {
                let len = u16::from_le_bytes(len.try_into().unwrap()) as usize;
                *pos += 2 + len;
                *pos <= src.len()
            }
            None => false,
        },
        InlineValue::Ref => {
            let Some(target) = src.get(*pos..*pos + 4) else {
                return false;
            };
            *pos += 4;
            let target = i32::from_le_bytes(target.try_into().unwrap());
            if target == builtin::LIST || target == builtin::MAP {
                return true; // List/Map ref → null sentinel, nothing inline.
            }
            skip_diff_value(src, pos, target, classes, depth + 1)
        }
        InlineValue::Fixed(n) => {
            *pos += n;
            *pos <= src.len()
        }
    }
}

/// Read one little-endian u16 at `pos`, advancing it. `None` on
/// truncation.
fn read_i16(src: &[u8], pos: &mut usize) -> Option<u16> {
    let bytes = src.get(*pos..*pos + 2)?;
    *pos += 2;
    Some(u16::from_le_bytes(bytes.try_into().unwrap()))
}

/// Parse a WTHS record from its sub-records. Starfield-only; the label
/// is not routed here for other games (their `WTHR` arm stays the
/// weather path).
pub fn parse_wths(
    form_id: u32,
    subs: &[SubRecord],
    remap: &Option<FormIdRemap>,
) -> WeatherSettingsRecord {
    let mut record = WeatherSettingsRecord {
        form_id,
        ..WeatherSettingsRecord::default()
    };
    for sub in subs {
        match &sub.sub_type {
            b"EDID" => record.editor_id = read_zstring(&sub.data),
            // RFDP — the parent WTHS this record diffs against.
            // Cross-record FormID: load-order remapped (#4066 pattern)
            // so `index.weather_settings.get(parent)` resolves.
            b"RFDP" if sub.data.len() >= 4 => {
                record.reflection_parent = SubReader::new(&sub.data)
                    .u32()
                    .ok()
                    .map(|raw| remap_fid(raw, remap));
            }
            b"REFL" => record.reflection = Some(parse_bgs_reflection(&sub.data)),
            b"RDIF" => record.reflection_diff = Some(parse_bgs_reflection(&sub.data)),
            _ => {}
        }
    }
    record
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::esm::records::test_support::sub;

    /// The builtin ids are the on-disk `0xFFFFFF01..=0xFFFFFF12` tags that
    /// `byroredux-sfmaterial::BuiltinType` names (that crate is not a
    /// dependency of this one, #5615), as the same bit patterns viewed as
    /// i32. Pins the single [`BUILTINS`] table against the documented set.
    #[test]
    fn builtin_table_is_the_documented_tag_set() {
        let tags: Vec<u32> = BUILTINS.iter().map(|(id, _, _)| *id as u32).collect();
        assert_eq!(
            tags,
            [
                0xFFFFFF01, 0xFFFFFF02, 0xFFFFFF03, 0xFFFFFF04, 0xFFFFFF05, 0xFFFFFF08,
                0xFFFFFF09, 0xFFFFFF0A, 0xFFFFFF0B, 0xFFFFFF0C, 0xFFFFFF0D, 0xFFFFFF0E,
                0xFFFFFF0F, 0xFFFFFF10, 0xFFFFFF11, 0xFFFFFF12,
            ]
        );
        assert_eq!(builtin_type_label(-243), Some("UInt32"));
        assert_eq!(builtin_type_label(-249), None, "gap ids stay unknown");
        let mut labels: Vec<&str> = BUILTINS.iter().map(|(_, l, _)| *l).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), BUILTINS.len(), "labels are unique");
        // An unknown built-in refuses to guess rather than skipping bytes.
        let mut pos = 0;
        assert!(!skip_diff_value(&[0; 8], &mut pos, -249, &[], 0));
    }

    /// Hand-built reflection container exercising every decoded chunk
    /// kind, in authored order: BETH → STRT → TYPE → CLAS → OBJT.
    #[test]
    fn reflection_container_decodes_schema_and_census() {
        let mut blob: Vec<u8> = Vec::new();
        // BETH: version 4, 5 chunks.
        blob.extend_from_slice(b"BETH");
        blob.extend_from_slice(&8u32.to_le_bytes());
        blob.extend_from_slice(&4u32.to_le_bytes());
        blob.extend_from_slice(&5u32.to_le_bytes());
        // STRT: "BGSWeatherSettingsForm\0FogNear\0FloatValue\0"
        let strs = b"BGSWeatherSettingsForm\0FogNear\0FloatValue\0";
        let fog_near_off: i32 = 24; // after the root name + nul
        let float_value_off: i32 = fog_near_off + 8;
        blob.extend_from_slice(b"STRT");
        blob.extend_from_slice(&(strs.len() as u32).to_le_bytes());
        blob.extend_from_slice(strs);
        // TYPE: one class.
        blob.extend_from_slice(b"TYPE");
        blob.extend_from_slice(&4u32.to_le_bytes());
        blob.extend_from_slice(&1u32.to_le_bytes());
        // CLAS: root class, one FogNear: Float field at offset 0 size 4.
        let mut clas = Vec::new();
        clas.extend_from_slice(&0i32.to_le_bytes()); // name → strtab 0
        clas.extend_from_slice(&0i32.to_le_bytes()); // type
        clas.extend_from_slice(&0u16.to_le_bytes()); // flags
        clas.extend_from_slice(&1u16.to_le_bytes()); // field count
        clas.extend_from_slice(&fog_near_off.to_le_bytes());
        clas.extend_from_slice(&float_value_off.to_le_bytes());
        clas.extend_from_slice(&0u16.to_le_bytes()); // offset
        clas.extend_from_slice(&4u16.to_le_bytes()); // size
        blob.extend_from_slice(b"CLAS");
        blob.extend_from_slice(&(clas.len() as u32).to_le_bytes());
        blob.extend_from_slice(&clas);
        // OBJT: instance chunk, censused not interpreted.
        blob.extend_from_slice(b"OBJT");
        blob.extend_from_slice(&4u32.to_le_bytes());
        blob.extend_from_slice(&[0, 0, 0, 0]);

        let refl = parse_bgs_reflection(&blob);
        assert_eq!(refl.version, 4);
        assert_eq!(refl.chunk_count, 5);
        assert_eq!(refl.strings.len(), 3);
        assert_eq!(refl.class_count, 1);
        assert_eq!(refl.classes.len(), 1);
        let root = refl.weather_root_class().expect("root class resolved");
        assert_eq!(root.name, "BGSWeatherSettingsForm");
        assert_eq!(root.fields.len(), 1);
        assert_eq!(root.fields[0].name, "FogNear");
        assert_eq!(root.fields[0].type_name, "FloatValue");
        assert_eq!((root.fields[0].offset, root.fields[0].size), (0, 4));
        assert_eq!(refl.instance_chunks, vec![(*b"OBJT", 4)]);
        assert!(refl.unknown_chunks.is_empty());
        assert!(refl.diff_field_indices.is_none());
    }

    /// #5514 — a `DIFF` chunk is an object diff, not a flat index list:
    /// each overridden field carries its inline value (skipped by field
    /// type — `String` by length prefix, built-ins by size, structs as
    /// nested diff-pair streams), and the walk stops at `0xFFFF`. The
    /// pre-fix reader treated every `u16` as an index, so value bytes
    /// and the terminator polluted `diff_field_indices`.
    #[test]
    fn diff_chunk_decodes_index_value_pairs_to_the_terminator() {
        // Class layout (strtab offsets): "Root" @0, "Struct" @5, plus
        // field names/types. Fields:
        //   0 Name:  String   (-254)
        //   1 Count: UInt32   (-243)
        //   2 Nested: Struct  (class ref = name offset 5, non-user)
        //   3 Bits:  Bool     (-240)
        let strs = b"Root\0Struct\0Name\0Count\0Nested\0Bits\0";
        let name_off = 0i32;
        let struct_off = 5i32;
        let field = |n: i32, t: i32, o: u16, s: u16| {
            let mut v = Vec::new();
            v.extend_from_slice(&n.to_le_bytes());
            v.extend_from_slice(&t.to_le_bytes());
            v.extend_from_slice(&o.to_le_bytes());
            v.extend_from_slice(&s.to_le_bytes());
            v
        };
        let mut clas = Vec::new();
        clas.extend_from_slice(&name_off.to_le_bytes()); // Root's name offset
        clas.extend_from_slice(&0i32.to_le_bytes()); // type
        clas.extend_from_slice(&0u16.to_le_bytes()); // flags (not User)
        clas.extend_from_slice(&4u16.to_le_bytes()); // field count
        clas.extend(field(11, -254, 0, 0)); // Name: String
        clas.extend(field(16, -243, 0, 4)); // Count: UInt32
        clas.extend(field(22, struct_off, 0, 0)); // Nested: Struct
        clas.extend(field(29, -240, 4, 1)); // Bits: Bool
        let mut struct_clas = Vec::new();
        struct_clas.extend_from_slice(&struct_off.to_le_bytes()); // Struct's name offset
        struct_clas.extend_from_slice(&0i32.to_le_bytes());
        struct_clas.extend_from_slice(&0u16.to_le_bytes()); // not User → inline
        struct_clas.extend_from_slice(&1u16.to_le_bytes());
        struct_clas.extend(field(11, -243, 0, 4)); // Name (index 0): UInt32

        // DIFF body: type ref = Root's name offset, then pairs:
        //   0 → String len 3 "abc"
        //   1 → u32 0x11223344 (a value whose low u16 bytes would have
        //       been misread as indices pre-fix)
        //   2 → nested struct diff: 0 → u32, 0xFFFF
        //   3 → bool 1
        //   0xFFFF terminator
        let mut diff = Vec::new();
        diff.extend_from_slice(&name_off.to_le_bytes());
        diff.extend_from_slice(&0u16.to_le_bytes());
        diff.extend_from_slice(&3u16.to_le_bytes());
        diff.extend_from_slice(b"abc");
        diff.extend_from_slice(&1u16.to_le_bytes());
        diff.extend_from_slice(&0x1122_3344u32.to_le_bytes());
        diff.extend_from_slice(&2u16.to_le_bytes());
        diff.extend_from_slice(&0u16.to_le_bytes()); // nested pair index
        diff.extend_from_slice(&0x5566_7788u32.to_le_bytes()); // nested value
        diff.extend_from_slice(&0xFFFFu16.to_le_bytes()); // nested terminator
        diff.extend_from_slice(&3u16.to_le_bytes());
        diff.push(1u8); // bool
        diff.extend_from_slice(&0xFFFFu16.to_le_bytes()); // terminator

        let mut blob: Vec<u8> = Vec::new();
        let chunk = |sig: &[u8; 4], payload: &[u8], out: &mut Vec<u8>| {
            out.extend_from_slice(sig);
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            out.extend_from_slice(payload);
        };
        chunk(b"BETH", &4u32.to_le_bytes().iter().chain(&5u32.to_le_bytes()).copied().collect::<Vec<u8>>(), &mut blob);
        chunk(b"STRT", strs, &mut blob);
        chunk(
            b"TYPE",
            1u32.to_le_bytes().as_ref(),
            &mut blob,
        );
        chunk(b"CLAS", &clas, &mut blob);
        chunk(b"CLAS", &struct_clas, &mut blob);
        chunk(b"DIFF", &diff, &mut blob);
        // A side chunk the DIFF's chunk-typed fields would own — censused.
        chunk(b"OBJT", &[0; 4], &mut blob);

        let refl = parse_bgs_reflection(&blob);
        assert_eq!(
            refl.diff_field_indices,
            Some(vec![0, 1, 2, 3]),
            "only the authored field indices — value bytes and the \
             terminator must not pollute the list"
        );
        assert!(refl.unknown_chunks.is_empty());
        // USRD/MAPC are recognized instance chunks, not unknowns (#5514).
        let refl_usrd = parse_bgs_reflection(
            &{
                let mut b = blob.clone();
                b.extend_from_slice(b"USRD");
                b.extend_from_slice(&8u32.to_le_bytes());
                b.extend_from_slice(&[0u8; 8]);
                b
            },
        );
        assert_eq!(refl_usrd.instance_chunks.len(), 2);
        assert!(refl_usrd.unknown_chunks.is_empty());
    }

    /// #5687 — hostile `RDIF` shapes: a class whose field names the class
    /// itself (the struct edge) and a `Ref` whose target is `Ref` again
    /// (the Ref edge). Each nesting level consumes only 2–4 payload bytes,
    /// so an unbounded walk recurses ~payload/2 deep and aborts the process
    /// with a stack overflow; both edges now decline at
    /// [`MAX_NESTING_DEPTH`], leaving `diff_field_indices` unset. Parsed on
    /// a deliberately small thread so a regression aborts here, not during
    /// a production load.
    #[test]
    fn hostile_self_referential_diffs_decline_at_the_depth_cap() {
        let parse = |field_type: i32, value_bytes: Vec<u8>| {
            // One class named at strtab offset 0 ("C"), not user-flagged,
            // with the single field the DIFF payload exercises.
            let mut clas = Vec::new();
            clas.extend_from_slice(&0i32.to_le_bytes()); // name → strtab 0
            clas.extend_from_slice(&0i32.to_le_bytes()); // type
            clas.extend_from_slice(&0u16.to_le_bytes()); // flags (not User)
            clas.extend_from_slice(&1u16.to_le_bytes()); // field count
            clas.extend_from_slice(&0i32.to_le_bytes()); // field name → 0
            clas.extend_from_slice(&field_type.to_le_bytes());
            clas.extend_from_slice(&0u16.to_le_bytes()); // offset
            clas.extend_from_slice(&0u16.to_le_bytes()); // size
            let mut diff = Vec::new();
            diff.extend_from_slice(&0i32.to_le_bytes()); // DIFF names the class
            diff.extend_from_slice(&value_bytes);

            let mut blob: Vec<u8> = Vec::new();
            let chunk = |sig: &[u8; 4], payload: &[u8], out: &mut Vec<u8>| {
                out.extend_from_slice(sig);
                out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                out.extend_from_slice(payload);
            };
            chunk(
                b"BETH",
                &4u32.to_le_bytes().iter().chain(&5u32.to_le_bytes()).copied().collect::<Vec<u8>>(),
                &mut blob,
            );
            chunk(b"STRT", b"C\0", &mut blob);
            chunk(b"TYPE", 1u32.to_le_bytes().as_ref(), &mut blob);
            chunk(b"CLAS", &clas, &mut blob);
            chunk(b"DIFF", &diff, &mut blob);

            std::thread::Builder::new()
                .stack_size(256 * 1024)
                .spawn(move || parse_bgs_reflection(&blob))
                .expect("spawn")
                .join()
                .expect("no panic")
        };

        // Struct edge: field 0 names the class itself; every nested level
        // re-enters the struct arm consuming one u16 field index.
        let mut struct_body = Vec::new();
        for _ in 0..100_000 {
            struct_body.extend_from_slice(&0u16.to_le_bytes());
        }
        assert!(
            parse(0, struct_body).diff_field_indices.is_none(),
            "self-referential struct declines at the cap, not a stack overflow"
        );

        // Ref edge: a Ref field whose inline target is Ref again; every
        // level consumes one i32 target.
        let mut ref_body = vec![0u8, 0]; // DIFF pair index 0
        for _ in 0..100_000 {
            ref_body.extend_from_slice(&builtin::REF.to_le_bytes());
        }
        assert!(
            parse(builtin::REF, ref_body).diff_field_indices.is_none(),
            "self-referential Ref chain declines at the cap, not a stack overflow"
        );
    }

    /// Built-in type refs (negative i32) resolve to their labels; the
    /// String id is `0xFFFFFF02` = -254 as authored, and an unlisted
    /// negative keeps its hex for diagnostics.
    #[test]
    fn builtin_refs_label_and_unlisted_negatives_kept_as_hex() {
        let table = vec![(0usize, "Named".to_string())];
        assert_eq!(resolve_ref(-254, &table), "String");
        assert_eq!(resolve_ref(-243, &table), "UInt32");
        assert_eq!(resolve_ref(-239, &table), "Float");
        assert_eq!(resolve_ref(-240, &table), "Bool");
        assert_eq!(resolve_ref(-100, &table), "<builtin 0xffffff9c>");
        assert_eq!(resolve_ref(0, &table), "Named");
        assert_eq!(resolve_ref(3, &table), "Named", "mid-string offsets fall back to the containing entry");
    }

    /// An overflowing chunk size ends the walk with the decoded prefix
    /// intact — no panic, no silent wraparound.
    #[test]
    fn overflowing_chunk_stops_the_walk() {
        let mut blob: Vec<u8> = Vec::new();
        blob.extend_from_slice(b"BETH");
        blob.extend_from_slice(&8u32.to_le_bytes());
        blob.extend_from_slice(&4u32.to_le_bytes());
        blob.extend_from_slice(&2u32.to_le_bytes());
        blob.extend_from_slice(b"OBJT");
        blob.extend_from_slice(&0xFFFF_0000u32.to_le_bytes()); // overflows
        let refl = parse_bgs_reflection(&blob);
        assert_eq!(refl.version, 4);
        assert!(refl.instance_chunks.is_empty());
    }

    /// The WTHS shell: EDID + REFL root shape and the RFDP remap.
    #[test]
    fn wths_shell_parses_refl_and_remaps_rfdp() {
        let remap = crate::esm::reader::FormIdRemap::regular(2, vec![0]);
        let self_ref = (1u32 << 24) | 0x000E_8397; // parent template, own plugin
        let subs = vec![
            sub(b"EDID", b"WeatherUniqueProbe\0"),
            sub(b"RFDP", self_ref.to_le_bytes()),
            sub(
                b"RDIF",
                // BETH chunk: size 8, then version 4 / chunk count 1.
                b"BETH\x08\x00\x00\x00\x04\x00\x00\x00\x01\x00\x00\x00",
            ),
        ];
        let rec = parse_wths(0x000A_0001, &subs, &Some(remap));
        assert_eq!(rec.editor_id, "WeatherUniqueProbe");
        assert_eq!(rec.reflection_parent, Some((2u32 << 24) | 0x000E_8397));
        let diff = rec.reflection_diff.expect("RDIF decoded");
        assert_eq!(diff.version, 4);
        assert!(rec.reflection.is_none());
    }
}
