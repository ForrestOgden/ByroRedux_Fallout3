//! Native iPhone FFI for genuine Bethesda Gamebryo NIF geometry, parsed by ByroRedux.
//! Rendering remains a diagnostic: no game world, textures, physics, or scripting.

use byroredux_bsa::BsaArchive;
use byroredux_core::string::StringPool;
use byroredux_nif::import::{import_nif_scene, ImportedNode};
use glam::{Quat, Vec3};
use std::ffi::{c_char, CStr};
use std::fs;
use std::ptr;
mod dds;

const MAX_NIF_BYTES: u64 = 24 * 1024 * 1024;
const MAX_RENDER_VERTICES: usize = 180_000;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FnvVertex {
    /// float4 in Metal; entire vertex is exactly 32 bytes.
    pub position: [f32; 4],
    pub color: [f32; 4],
    /// U, V, per-triangle light intensity, reserved: 16 bytes.
    pub uv_light: [f32; 4],
}

#[repr(C)]
#[derive(Debug)]
pub struct FnvMesh {
    pub vertices: *mut FnvVertex,
    pub count: usize,
}

#[derive(Clone, Copy)]
struct Rigid {
    position: Vec3,
    rotation: Quat,
    scale: f32,
}

impl Default for Rigid {
    fn default() -> Self {
        Self { position: Vec3::ZERO, rotation: Quat::IDENTITY, scale: 1.0 }
    }
}

impl Rigid {
    fn extend(self, p: [f32; 3], r: [f32; 4], scale: f32) -> Self {
        let rot = Quat::from_xyzw(r[0], r[1], r[2], r[3]);
        let rot = if rot.is_finite() && rot.length_squared() > 0.01 {
            rot.normalize()
        } else {
            Quat::IDENTITY
        };
        Self {
            position: self.position + self.rotation * (Vec3::from_array(p) * self.scale),
            rotation: (self.rotation * rot).normalize(),
            scale: self.scale * scale,
        }
    }

    fn point(&self, p: [f32; 3]) -> Vec3 {
        self.position + self.rotation * (Vec3::from_array(p) * self.scale)
    }
}

/// Compose the parent chain root-to-leaf, with a hard limit against malformed graphs.
fn world_of(nodes: &[ImportedNode], start: Option<usize>) -> Option<Rigid> {
    let mut chain = Vec::new();
    let mut current = start;
    while let Some(i) = current {
        let n = nodes.get(i)?;
        if chain.len() >= 128 || chain.contains(&i) {
            return None;
        }
        chain.push(i);
        current = n.parent_node;
    }
    let mut x = Rigid::default();
    for &i in chain.iter().rev() {
        let n = &nodes[i];
        x = x.extend(n.translation, n.rotation, n.scale);
    }
    Some(x)
}

/// Two-sided, geometry-derived flat shading. This is not Bethesda material lighting.
fn diagnostic_face_light(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    let cross = (b - a).cross(c - a);
    let squared = cross.length_squared();
    if !squared.is_finite() || squared < 1.0e-12 { return 0.35; }
    let normal = cross / squared.sqrt();
    let light = Vec3::new(0.45, 0.78, 0.44).normalize();
    (0.28 + 0.72 * normal.dot(light).abs()).clamp(0.28, 1.0)
}

fn load_vertices(path: &str) -> Result<Vec<FnvVertex>, &'static str> {
    let meta = fs::metadata(path).map_err(|_| "NIF file is not readable")?;
    if meta.len() == 0 || meta.len() > MAX_NIF_BYTES {
        return Err("NIF size is zero or exceeds 24 MiB");
    }
    let bytes = fs::read(path).map_err(|_| "NIF read failed")?;
    let scene = byroredux_nif::parse_nif(&bytes).map_err(|_| "NIF parse failed")?;
    if scene.truncated {
        return Err("NIF scene is incomplete");
    }
    let mut string_pool = StringPool::new();
    let imported = import_nif_scene(&scene, &mut string_pool);
    let mut vertices = Vec::new();
    for (mesh_index, mesh) in imported.meshes.iter().enumerate() {
        let Some(parent) = world_of(&imported.nodes, mesh.parent_node) else { continue };
        let transform = parent.extend(mesh.translation, mesh.rotation, mesh.scale);
        if !transform.position.is_finite() || !transform.scale.is_finite() {
            continue;
        }
        // Diagnostic palette differentiates submeshes; actual textures are not yet used.
        let palette = [[0.90, 0.70, 0.37], [0.45, 0.78, 0.73],
                       [0.81, 0.52, 0.45], [0.62, 0.68, 0.88]];
        let base = palette[mesh_index % palette.len()];
        for tri in mesh.indices.chunks_exact(3) {
            if vertices.len() + 3 > MAX_RENDER_VERTICES { break; }
            if tri.iter().any(|&index| index as usize >= mesh.positions.len()) { continue; }
            let points = [
                transform.point(mesh.positions[tri[0] as usize]),
                transform.point(mesh.positions[tri[1] as usize]),
                transform.point(mesh.positions[tri[2] as usize]),
            ];
            // Reject an entire corrupt triangle, never partial vertices.
            if points.iter().any(|p| !p.is_finite()) { continue; }
            let brightness = diagnostic_face_light(points[0], points[1], points[2]);
            for pos in points {
                let vertex_idx = tri[vertices.len() % 3] as usize;
                let uv = mesh.uvs.get(vertex_idx).copied().unwrap_or([0.0, 0.0]);
                let uv = if uv[0].is_finite() && uv[1].is_finite() { uv } else { [0.0, 0.0] };
                vertices.push(FnvVertex {
                    position: [pos.x, pos.y, pos.z, 1.0],
                    color: [base[0] * brightness, base[1] * brightness, base[2] * brightness, 1.0],
                    uv_light: [uv[0], uv[1], brightness, 0.0],
                });
            }
        }
        if vertices.len() >= MAX_RENDER_VERTICES { break; }
    }
    if vertices.len() < 3 || vertices.len() % 3 != 0 {
        return Err("NIF has no usable triangle geometry");
    }

    // Center and normalize geometry for the fixed camera; preserve the original shape.
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for v in &vertices {
        let p = Vec3::new(v.position[0], v.position[1], v.position[2]);
        lo = lo.min(p);
        hi = hi.max(p);
    }
    let span = (hi - lo).max_element();
    if !span.is_finite() || span < 0.00001 { return Err("Degenerate mesh bounds"); }
    let center = (lo + hi) * 0.5;
    let factor = 2.7 / span;
    for v in &mut vertices {
        let p = (Vec3::new(v.position[0], v.position[1], v.position[2]) - center) * factor;
        v.position = [p.x, p.y, p.z, 1.0];
    }
    Ok(vertices)
}

#[unsafe(no_mangle)]
pub extern "C" fn fnv_ios_abi_version() -> u32 { 3 }

/// Probe a local BSA using the existing ByroRedux archive parser.
/// Caller must provide a live NUL-terminated UTF-8 C string, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fnv_ios_probe_bsa(path: *const c_char) -> i32 {
    if path.is_null() { return -1; }
    let path = unsafe { CStr::from_ptr(path) };
    let Ok(path) = path.to_str() else { return -2 };
    match BsaArchive::open(path) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

/// Parse/import a user-supplied, locally stored NIF using native ARM64 Rust.
///
/// Return codes: 0 success, -1 invalid pointers, -2 non-UTF8,
/// 1 unreadable/oversized file, 2 unsupported/invalid geometry.
/// On success, out.vertices must be released with fnv_ios_release_mesh.
///
/// # Safety
/// path must be valid and NUL-terminated for the call. out must be a writable
/// FnvMesh pointer; the caller must not use vertices after release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fnv_ios_load_nif_mesh(path: *const c_char, out: *mut FnvMesh) -> i32 {
    if path.is_null() || out.is_null() { return -1; }
    unsafe { *out = FnvMesh { vertices: ptr::null_mut(), count: 0 } };
    let cpath = unsafe { CStr::from_ptr(path) };
    let Ok(path) = cpath.to_str() else { return -2 };
    let Ok(vertices) = load_vertices(path) else {
        return if fs::metadata(path).map_or(true, |m| m.len() > MAX_NIF_BYTES) { 1 } else { 2 };
    };
    let boxed = vertices.into_boxed_slice();
    let count = boxed.len();
    let data = Box::into_raw(boxed) as *mut FnvVertex;
    unsafe { *out = FnvMesh { vertices: data, count } };
    0
}

/// # Safety
/// ptr/count must be exactly the pair returned once by fnv_ios_load_nif_mesh.
/// Double release or a different count is invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fnv_ios_release_mesh(ptr: *mut FnvVertex, count: usize) {
    if ptr.is_null() || count == 0 { return; }
    let slice = ptr::slice_from_raw_parts_mut(ptr, count);
    unsafe { drop(Box::from_raw(slice)); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    #[test]
    fn layout_is_exactly_thirty_two_bytes() {
        assert_eq!(std::mem::size_of::<FnvVertex>(), 48);
    }
    #[test]
    fn abi_3() { assert_eq!(fnv_ios_abi_version(), 3); }
    #[test]
    fn null_pointer_is_rejected() {
        let mut out = FnvMesh { vertices: ptr::null_mut(), count: 0 };
        assert_eq!(unsafe { fnv_ios_load_nif_mesh(ptr::null(), &mut out) }, -1);
        assert_eq!(unsafe { fnv_ios_load_nif_mesh(ptr::null(), ptr::null_mut()) }, -1);
        assert_eq!(unsafe { fnv_ios_probe_bsa(ptr::null()) }, -1);
    }
    #[test]
    fn nonexistent_file_fails_without_allocations() {
        let p = CString::new("/__falloutnv_ios_uninstalled__/absent.nif").unwrap();
        let mut out = FnvMesh { vertices: ptr::null_mut(), count: 0 };
        assert_eq!(unsafe { fnv_ios_load_nif_mesh(p.as_ptr(), &mut out) }, 1);
        assert!(out.vertices.is_null());
        assert_eq!(out.count, 0);
    }
    #[test]
    fn faceted_light_is_bounded_and_winding_invariant() {
        let a = Vec3::ZERO;
        let b = Vec3::X;
        let c = Vec3::Y;
        let f = diagnostic_face_light(a,b,c);
        assert!((0.28..=1.0).contains(&f));
        assert!((f - diagnostic_face_light(a,c,b)).abs() < 1e-6);
        assert_eq!(diagnostic_face_light(a,a,a), 0.35);
    }
    #[test]
    fn local_transform_composes() {
        let a = Rigid::default().extend([2.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 2.0);
        let b = a.extend([1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 1.0);
        assert!((b.point([1.0, 0.0, 0.0]).x - 6.0).abs() < 0.0001);
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct FnvTexture {
    pub pixels: *mut u8,
    pub len: usize,
    pub width: u32,
    pub height: u32,
}

/// Decode a locally selected DDS to CPU RGBA8, suitable for Apple Metal.
/// BC1/DXT1, BC2/DXT3, BC3/DXT5 and 32-bit RGB masks only.
/// On success call fnv_ios_release_texture exactly once with returned pointer and len.
///
/// # Safety
/// path must be a valid NUL-terminated string and out a writable FnvTexture.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fnv_ios_load_dds_rgba(path: *const c_char, out: *mut FnvTexture) -> i32 {
    if path.is_null() || out.is_null() { return -1; }
    unsafe { *out = FnvTexture { pixels: ptr::null_mut(), len: 0, width: 0, height: 0 } };
    let path = unsafe { CStr::from_ptr(path) };
    let Ok(path) = path.to_str() else { return -2 };
    let Ok(meta) = fs::metadata(path) else { return 1 };
    if meta.len() < 128 || meta.len() > 48 * 1024 * 1024 { return 1; }
    let Ok(bytes) = fs::read(path) else { return 1 };
    let Ok(decoded) = dds::decode_dds(&bytes) else { return 2 };
    let len = decoded.rgba.len();
    let pixels = Box::into_raw(decoded.rgba.into_boxed_slice()) as *mut u8;
    unsafe {
        *out = FnvTexture { pixels, len, width: decoded.width, height: decoded.height };
    }
    0
}

/// # Safety
/// pointer/length must exactly match a successful fnv_ios_load_dds_rgba
/// allocation; each allocation may only be released once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fnv_ios_release_texture(pixels: *mut u8, len: usize) {
    if pixels.is_null() || len == 0 { return; }
    unsafe { drop(Box::from_raw(ptr::slice_from_raw_parts_mut(pixels, len))); }
}

#[cfg(test)]
mod texture_ffi_tests {
    use super::*;
    #[test]
    fn null_rejected() {
        let mut dst = FnvTexture { pixels: ptr::null_mut(), len: 0, width: 0, height: 0 };
        assert_eq!(unsafe { fnv_ios_load_dds_rgba(ptr::null(), &mut dst) }, -1);
        assert_eq!(unsafe { fnv_ios_load_dds_rgba(ptr::null(), ptr::null_mut()) }, -1);
    }
}
