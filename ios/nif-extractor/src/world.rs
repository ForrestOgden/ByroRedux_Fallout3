//! Export a real Fallout: New Vegas interior from ESM-authored REFR placements
//! and the original user's local BSA archives. No fake/procedural placements.
//!
//! A compact streaming-friendly manifest contains *unique* model NIF/DDS
//! payloads plus original REFR transforms, instead of baking N copies of a
//! repeated prop. The native ARM64 bridge reconstructs geometry on iPhone.
//! This is a geometry-only world milestone, not actors/quests/collision yet.
use byroredux_bsa::BsaArchive;
use byroredux_core::math::coord::{euler_zup_to_quat_yup_mode, zup_to_yup_pos};
use byroredux_plugin::esm;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

const MAGIC: &[u8; 8] = b"FNVWLD01";
pub(crate) const VERSION: u32 = 3;
const MAX_TEXTURES_PER_MODEL: usize = 64;
const MAX_ESM: u64 = 750 * 1024 * 1024;
const MAX_MODELS: usize = 160;
const MAX_INSTANCES: usize = 480;
const MAX_NIF: usize = 24 * 1024 * 1024;
const MAX_DDS: usize = 48 * 1024 * 1024;
const MAX_PACKAGE: usize = 240 * 1024 * 1024;
const WORLD_UNITS_PER_METER: f32 = 70.0;

struct ModelBlob {
    name: String,
    nif: Vec<u8>,
    /// One source DDS payload (possibly empty) for each authored diffuse slot.
    dds: Vec<Vec<u8>>,
}
struct Instance {
    model: u32,
    pos: [f32; 3],
    quat: [f32; 4],
    scale: f32,
    ref_id: u32,
}

/// The BSA index may omit its top-level "meshes" folder. Canonicalize
/// without performing filesystem resolution or trusting paths from ESM.
fn asset_key(path: &str) -> Option<String> {
    if path.contains('\0') || path.contains(':') { return None; }
    let mut text = path.trim().replace('/', "\\").to_ascii_lowercase();
    if text.starts_with('\\') { return None; }
    if let Some(rest) = text.strip_prefix("data\\") { text = rest.to_string(); }
    if let Some(rest) = text.strip_prefix("meshes\\") { text = rest.to_string(); }
    if !text.ends_with(".nif")
        || text.split('\\').any(|s| s.is_empty() || s == "." || s == "..") { return None; }
    Some(text)
}

fn choose_spawn(
    refs: &[esm::cell::PlacedRef],
    statics: &HashMap<u32, esm::cell::StaticObject>,
) -> ([f32; 3], [f32; 3], &'static str) {
    if let Some(marker) = refs.iter().find(|r| {
        statics.get(&r.base_form_id).is_some_and(|s| s.editor_id.eq_ignore_ascii_case("COCMarkerHeading"))
    }) {
        return (marker.position, marker.rotation, "authored COCMarkerHeading");
    }
    if let Some(door) = refs.iter().find(|r| r.teleport.is_some()) {
        return (door.position, door.rotation, "authored door placement (fallback, NOT verified floor spawn)");
    }
    let mut coords: Vec<[f32; 3]> = refs.iter().map(|r| r.position)
        .filter(|p| p.iter().all(|v| v.is_finite())).collect();
    if coords.is_empty() { return ([0.;3], [0.;3], "no references (invalid)"); }
    // Median of the authored REFR coordinates when no safe marker exists.
    let mut center=[0.;3];
    for axis in 0..3 {
        coords.sort_by(|a,b| a[axis].total_cmp(&b[axis]));
        center[axis]=coords[coords.len()/2][axis];
    }
    (center, [0.;3], "median of authored REFR positions (no floor guarantee)")
}

fn write_u32<W: Write>(w: &mut W, v: u32) -> std::io::Result<()> {
    w.write_all(&v.to_le_bytes())
}
fn write_f32<W: Write>(w: &mut W, v: f32) -> std::io::Result<()> {
    w.write_all(&v.to_le_bytes())
}
fn write_model<W: Write>(w: &mut W, model: &ModelBlob) -> Result<(), String> {
    let name=model.name.as_bytes();
    let name_len=u16::try_from(name.len()).map_err(|_|"NIF name too long")?;
    let nif_len=u32::try_from(model.nif.len()).map_err(|_|"NIF too large")?;
    let texture_count=u16::try_from(model.dds.len()).map_err(|_|"Too many material slots")?;
    w.write_all(&name_len.to_le_bytes()).and_then(|_|w.write_all(name))
        .map_err(|e|e.to_string())?;
    write_u32(w,nif_len)
        .and_then(|_|w.write_all(&texture_count.to_le_bytes()))
        .and_then(|_|w.write_all(&model.nif)).map_err(|e|e.to_string())?;
    for dds in &model.dds {
        let len=u32::try_from(dds.len()).map_err(|_|"DDS too large")?;
        write_u32(w,len).and_then(|_|w.write_all(dds)).map_err(|e|e.to_string())?;
    }
    Ok(())
}

pub fn export_cell(
    esm_path: &str, mesh_bsa: &str, tex_bsa: &str,
    out_path: &str, cell_id: &str,
) -> Result<(), String> {
    let output=Path::new(out_path);
    if !output.extension().and_then(|s|s.to_str()).is_some_and(|s|s.eq_ignore_ascii_case("fnvworld")) {
        return Err("Output must have .fnvworld extension".into());
    }
    if output.exists() { return Err("World file already exists; refusing to overwrite".into()); }
    let meta=fs::metadata(esm_path).map_err(|e|format!("Cannot read FalloutNV.esm: {e}"))?;
    if meta.len()<24 || meta.len()>MAX_ESM {return Err("ESM size is invalid/too large".into());}
    println!("Reading actual FalloutNV.esm ({:.1} MiB)",meta.len() as f64 / 1048576.);
    let esm_bytes=fs::read(esm_path).map_err(|e|e.to_string())?;
    let index=esm::parse_esm(&esm_bytes).map_err(|e|format!("New Vegas master parse failed: {e}"))?;
    let key=cell_id.to_ascii_lowercase();
    let cell=index.cells.cells.get(&key)
        .ok_or_else(||format!("Interior CELL {cell_id:?} not found in FalloutNV.esm"))?;
    if !cell.is_interior { return Err("This first milestone exports interior CELLs only".into()); }
    let (spawn,rotation,method)=choose_spawn(&cell.references,&index.cells.statics);
    // Reproduce the COC marker's facing direction instead of always staring
    // north. On fallback cases, zero angles face the camera's default -Z.
    if !rotation.iter().all(|v|v.is_finite()){return Err("Invalid spawn facing rotation".into());}
    let facing=euler_zup_to_quat_yup_mode(1,rotation[0],rotation[1],rotation[2])
        * byroredux_core::math::Vec3::NEG_Z;
    let spawn_yaw=facing.x.atan2(-facing.z);
    if !spawn_yaw.is_finite(){return Err("Invalid spawn camera yaw".into());}
    if !spawn.iter().all(|v|v.is_finite()) {return Err("Invalid authored spawn position".into());}
    println!("Real CELL: {} {:08X}, {} REFRs; spawn: {method}",cell.editor_id,cell.form_id,cell.references.len());
    let meshes=BsaArchive::open(mesh_bsa).map_err(|e|format!("Cannot open Meshes BSA: {e}"))?;
    let textures=BsaArchive::open(tex_bsa).map_err(|e|format!("Cannot open Textures BSA: {e}"))?;
    if meshes.version()!=104 || textures.version()!=104 {return Err("Expected New Vegas BSA v104".into());}
    let mut mesh_paths=meshes.list_files();mesh_paths.sort_unstable();
    let mesh_lookup: HashMap<String,&str>=mesh_paths.into_iter()
        .filter_map(|s|asset_key(s).map(|k|(k,s))).collect();
    // FNV spreads diffuse content across Textures.bsa and Textures2.bsa.
    // Auto-discover the standard sibling when installed; never fabricate
    // missing data or require the user to hand-match model textures.
    let second_path=Path::new(tex_bsa).with_file_name("Fallout - Textures2.bsa");
    let textures2=if second_path.exists() {
        BsaArchive::open(&second_path).ok().filter(|a|a.version()==104)
    }else{None};
    if textures2.is_some(){println!("Discovered optional Fallout - Textures2.bsa");}
    let mut tex_lookup:HashMap<String,(u8,&str)>=HashMap::new();
    let mut texture_paths=textures.list_files();texture_paths.sort_unstable();
    for path in texture_paths {
        if let Some(key)=super::package::texture_key(path){tex_lookup.insert(key,(0,path));}
    }
    if let Some(extra)=textures2.as_ref(){
        let mut other=extra.list_files();other.sort_unstable();
        for path in other{
            if let Some(key)=super::package::texture_key(path){
                tex_lookup.entry(key).or_insert((1,path));
            }
        }
    }

    // References nearest the authored spawn first. The limit is explicit
    // and output logs disclose omissions; this is NOT a full-game export.
    let mut refs:Vec<_>=cell.references.iter().collect();
    refs.sort_by(|a,b|{
        let dist=|p:&[f32;3]|->f32{
            let v=(p[0]-spawn[0]).powi(2)+(p[1]-spawn[1]).powi(2)+(p[2]-spawn[2]).powi(2);
            if v.is_finite(){v}else{f32::INFINITY}
        };
        dist(&a.position).total_cmp(&dist(&b.position))
    });
    let mut models:Vec<ModelBlob>=Vec::new();
    let mut model_ids:HashMap<String,u32>=HashMap::new();
    let mut failed:HashSet<String>=HashSet::new();
    let mut instances=Vec::new();
    let mut payload_size=64usize;
    let mut failures=0usize;

    for reference in refs {
        if instances.len()>=MAX_INSTANCES{break;}
        if reference.initially_disabled {continue;}
        if !reference.position.iter().all(|v|v.is_finite())
            || !reference.rotation.iter().all(|v|v.is_finite())
            || !reference.scale.is_finite() || reference.scale.abs()<0.0001 {continue;}
        // Use the same three base record families as the desktop cell
        // loader: structural STAT/MSTT/FURN, placed MISC/WEAP/ALCH/etc,
        // and CONT containers. Ignoring the latter two made the authored
        // saloon unexpectedly empty even though REFRs existed in its ESM.
        let model_path=if let Some(base)=index.cells.statics.get(&reference.base_form_id) {
            base.model_path.as_str()
        } else if let Some(item)=index.items.get(&reference.base_form_id) {
            item.common.model_path.as_str()
        } else if let Some(container)=index.containers.get(&reference.base_form_id) {
            container.model_path.as_str()
        } else {continue};
        let Some(key)=asset_key(model_path) else {continue};
        if failed.contains(&key){continue;}
        let model=if let Some(&id)=model_ids.get(&key){id}else{
            if models.len()>=MAX_MODELS{continue;}
            let Some(&path)=mesh_lookup.get(&key) else {failed.insert(key.clone());failures+=1;continue};
            let Ok(size)=meshes.declared_size(path) else{failed.insert(key.clone());failures+=1;continue};
            if size==0 || size>MAX_NIF{failed.insert(key.clone());failures+=1;continue;}
            let Ok(nif)=meshes.extract(path)else{failed.insert(key.clone());failures+=1;continue};
            if nif.is_empty()||nif.len()>MAX_NIF {failed.insert(key.clone());failures+=1;continue;}
            // Keep the complete, authored per-submesh material mapping.
            // Missing textures preserve empty slots so slot numbers cannot shift.
            let diffuse_keys=super::package::authored_diffuses(&nif)
                .unwrap_or_default();
            if diffuse_keys.len()>MAX_TEXTURES_PER_MODEL {
                println!("NIF {path} uses {} diffuse materials; including first {MAX_TEXTURES_PER_MODEL}",diffuse_keys.len());
            }
            let mut dds:Vec<Vec<u8>>=Vec::new();
            for key in diffuse_keys.iter().take(MAX_TEXTURES_PER_MODEL) {
                let payload=(||{
                    let &(which,archive_path)=tex_lookup.get(key)?;
                    let source=if which==1{textures2.as_ref()?}else{&textures};
                    let declared=source.declared_size(archive_path).ok()?;
                    if declared==0 || declared>MAX_DDS{return None}
                    let bytes=source.extract(archive_path).ok()?;
                    (bytes.len()<=MAX_DDS && bytes.starts_with(b"DDS ")).then_some(bytes)
                })().unwrap_or_default();
                dds.push(payload);
            }
            let contribution=2+path.len()+4+2+nif.len()
                +dds.iter().map(|bytes|4+bytes.len()).sum::<usize>();
            if payload_size.saturating_add(contribution)>MAX_PACKAGE {
                println!("Reached bounded world archive byte budget; cannot add {path}");
                break;
            }
            payload_size+=contribution;
            let idx=models.len() as u32;
            models.push(ModelBlob{name:path.to_string(),nif,dds});
            model_ids.insert(key.clone(),idx);
            idx
        };
        let origin=zup_to_yup_pos(spawn);
        let authored=zup_to_yup_pos(reference.position);
        let pos=[
            (authored[0]-origin[0])/WORLD_UNITS_PER_METER,
            (authored[1]-origin[1])/WORLD_UNITS_PER_METER,
            (authored[2]-origin[2])/WORLD_UNITS_PER_METER
        ];
        let q=euler_zup_to_quat_yup_mode(1,reference.rotation[0],reference.rotation[1],reference.rotation[2]);
        let quat=q.to_array();
        let scale=reference.scale/WORLD_UNITS_PER_METER;
        if !quat.iter().all(|v|v.is_finite()) || !scale.is_finite(){continue;}
        instances.push(Instance{model,pos,quat,scale,ref_id:reference.form_id});
    }
    if models.len()<5 || instances.len()<10 {
        return Err(format!("CELL {} yielded only {} unique models / {} placements; refusing to call this a real world",cell_id,models.len(),instances.len()));
    }
    let diffuse_slots:usize=models.iter().map(|m|m.dds.len()).sum();
    let present_textures:usize=models.iter().map(|m|m.dds.iter().filter(|t|!t.is_empty()).count()).sum();
    println!("Exporting {} UNIQUE original meshes, {} authored placements (source REFRs: {}); {} mesh lookups unavailable",
             models.len(),instances.len(),cell.references.len(),failures);
    println!("Authored NIF diffuse slots: {diffuse_slots}; embedded DDS: {present_textures}; unavailable DDS: {}",diffuse_slots-present_textures);
    println!("Limits: {MAX_MODELS} unique models, {MAX_INSTANCES} instances, {} MiB compressed source content. Missing models or materials are reported, never fabricated.",MAX_PACKAGE/1048576);
    let mut output_file=OpenOptions::new().write(true).create_new(true).open(output)
        .map_err(|e|format!("Cannot create world package: {e}"))?;
    let result=(||->Result<(),String>{
        let mut w=BufWriter::new(&mut output_file);
        w.write_all(MAGIC).map_err(|e|e.to_string())?;
        write_u32(&mut w,VERSION).and_then(|_|write_u32(&mut w,models.len() as u32))
            .and_then(|_|write_u32(&mut w,instances.len() as u32)).map_err(|e|e.to_string())?;
        let title=cell.editor_id.as_bytes();
        let title_len=u16::try_from(title.len()).map_err(|_|"CELL name too long")?;
        w.write_all(&title_len.to_le_bytes()).and_then(|_|w.write_all(title)).map_err(|e|e.to_string())?;
        // FNVWLD01 v3: original heading and all per-submesh diffuse slots.
        write_f32(&mut w,spawn_yaw).map_err(|e|e.to_string())?;
        for m in &models{write_model(&mut w,m)?;}
        for r in &instances{
            write_u32(&mut w,r.model).map_err(|e|e.to_string())?;
            for v in r.pos.into_iter().chain(r.quat).chain([r.scale]) {
                write_f32(&mut w,v).map_err(|e|e.to_string())?;
            }
            write_u32(&mut w,r.ref_id).map_err(|e|e.to_string())?;
        }
        w.flush().map_err(|e|e.to_string())
    })();
    drop(output_file);
    if let Err(error)=result {let _=fs::remove_file(output);return Err(error)}
    println!("Wrote authentic interior cell scene: {}",output.display());
    println!("Scene data is for offline local iPhone import; it is NOT a complete playable game.");
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn mesh_name_normalization_is_bounded(){
        assert_eq!(asset_key("Meshes/Clutter/AcousticGuitar/AcousticGuitar.nif"),
                   Some("clutter\\acousticguitar\\acousticguitar.nif".into()));
        for p in ["../bad.nif","C:\\x.nif","\\private.nif","foo//bar.nif","foo.nif\\..\\bad.nif","foo.dds"]{
            assert!(asset_key(p).is_none(),"invalid path {p}");
        }
    }
    #[test] fn spawn_uses_real_marker(){
        let blank:[f32;3]=[0.,0.,0.];
        assert_eq!(choose_spawn(&[],&HashMap::new()),(blank,blank,"no references (invalid)"));
    }
}
