//! Bounded, CPU-only decoder for the subset of classic Fallout: New Vegas DDS
//! formats this native iOS *preview* supports. Output is top mip RGBA8.
//! No GPU BC format support assumed (iOS Metal does not guarantee it).

pub struct DecodedDds {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

const MAX_DIM: u32 = 2048;
const DDS_LEN: usize = 128;

fn le16(x: &[u8]) -> u16 { u16::from_le_bytes([x[0], x[1]]) }
fn le32(x: &[u8]) -> u32 { u32::from_le_bytes([x[0], x[1], x[2], x[3]]) }

fn rgb565(v: u16) -> [u8; 3] {
    let r = ((v >> 11) & 31) as u32;
    let g = ((v >> 5) & 63) as u32;
    let b = (v & 31) as u32;
    [((r * 255 + 15) / 31) as u8,
     ((g * 255 + 31) / 63) as u8,
     ((b * 255 + 15) / 31) as u8]
}

fn block_colors(bytes: &[u8], bc1_alpha: bool) -> [[u8; 4]; 4] {
    let a = le16(&bytes[0..2]);
    let b = le16(&bytes[2..4]);
    let pa = rgb565(a);
    let pb = rgb565(b);
    let mut colors = [[0u8; 4]; 4];
    colors[0] = [pa[0], pa[1], pa[2], 255];
    colors[1] = [pb[0], pb[1], pb[2], 255];
    if a > b || !bc1_alpha {
        for ch in 0..3 {
            colors[2][ch] = ((2 * pa[ch] as u16 + pb[ch] as u16) / 3) as u8;
            colors[3][ch] = ((pa[ch] as u16 + 2 * pb[ch] as u16) / 3) as u8;
        }
        colors[2][3] = 255;
        colors[3][3] = 255;
    } else {
        for ch in 0..3 {
            colors[2][ch] = ((pa[ch] as u16 + pb[ch] as u16) / 2) as u8;
        }
        colors[2][3] = 255;
    }
    colors
}

fn bc3_alpha(block: &[u8], pixel: u32) -> u8 {
    let a0 = block[0];
    let a1 = block[1];
    let mut indices: u64 = 0;
    for (i, byte) in block[2..8].iter().enumerate() {
        indices |= (*byte as u64) << (8*i);
    }
    let id = ((indices >> (3 * pixel)) & 7) as u8;
    match id {
        0 => a0,
        1 => a1,
        2..=7 if a0 > a1 => {
            let k = (id - 1) as u16;
            (((7-k) * a0 as u16 + k * a1 as u16) / 7) as u8
        }
        2..=5 => {
            let k = (id - 1) as u16;
            (((5-k) * a0 as u16 + k * a1 as u16) / 5) as u8
        }
        6 => 0,
        _ => 255,
    }
}

fn scale_mask(v: u32, mask: u32, opaque: u8) -> u8 {
    if mask == 0 { return opaque; }
    let shifted = ((v & mask) >> mask.trailing_zeros()) as u64;
    let max = (mask >> mask.trailing_zeros()) as u64;
    ((shifted * 255 + max / 2) / max) as u8
}

pub fn decode_dds(bytes: &[u8]) -> Result<DecodedDds, &'static str> {
    if bytes.len() < DDS_LEN || &bytes[0..4] != b"DDS " || le32(&bytes[4..8]) != 124 {
        return Err("Invalid DDS header");
    }
    if le32(&bytes[76..80]) != 32 { return Err("Invalid DDS pixel format"); }
    let height = le32(&bytes[12..16]);
    let width = le32(&bytes[16..20]);
    if width == 0 || height == 0 || width > MAX_DIM || height > MAX_DIM {
        return Err("DDS dimensions outside safe limits");
    }
    // 2D textures only, not cube maps or volumes.
    if le32(&bytes[112..116]) & 0x200 != 0 || le32(&bytes[24..28]) > 1 {
        return Err("Cubemaps and volume textures unsupported");
    }
    let count = (width as usize).checked_mul(height as usize).ok_or("Texture dimensions overflow")?;
    let out_bytes = count.checked_mul(4).ok_or("Texture bytes overflow")?;
    let mut rgba = vec![0u8; out_bytes];
    let fourcc = &bytes[84..88];
    let kind = match fourcc {
        b"DXT1" => 1,
        b"DXT3" => 2,
        b"DXT5" => 3,
        _ => 0,
    };
    let pixel_flags = le32(&bytes[80..84]);
    if kind == 0 && (pixel_flags & 0x40 == 0 || le32(&bytes[88..92]) != 32) {
        return Err("Only BC1/BC2/BC3 and 32-bit uncompressed RGB DDS supported");
    }
    if kind == 0 {
        let n = out_bytes;
        if bytes.len() < DDS_LEN + n { return Err("Uncompressed texture truncated"); }
        let masks = [
            le32(&bytes[92..96]), le32(&bytes[96..100]),
            le32(&bytes[100..104]), le32(&bytes[104..108])
        ];
        if masks[0] == 0 || masks[1] == 0 || masks[2] == 0 { return Err("Invalid RGB channel masks"); }
        for i in 0..count {
            let p = le32(&bytes[DDS_LEN+i*4..DDS_LEN+i*4+4]);
            for c in 0..4 {
                rgba[i*4+c] = scale_mask(p, masks[c], if c == 3 { 255 } else { 0 });
            }
        }
    } else {
        let tile_bytes = if kind == 1 { 8usize } else { 16usize };
        let bw = width.div_ceil(4) as usize;
        let bh = height.div_ceil(4) as usize;
        let block_count = bw.checked_mul(bh).ok_or("DDS block count overflow")?;
        let data_size = block_count.checked_mul(tile_bytes).ok_or("DDS block bytes overflow")?;
        if bytes.len() < DDS_LEN + data_size { return Err("Compressed texture truncated"); }
        for by in 0..bh {
            for bx in 0..bw {
                let start = DDS_LEN + (by * bw + bx) * tile_bytes;
                let block = &bytes[start..start+tile_bytes];
                let color_start = if kind == 1 { 0 } else { 8 };
                let cbytes = &block[color_start..color_start+8];
                let colors = block_colors(cbytes, kind == 1);
                let indices = le32(&cbytes[4..8]);
                for py in 0..4 {
                    for px in 0..4 {
                        let x = bx*4 + px;
                        let y = by*4 + py;
                        if x >= width as usize || y >= height as usize { continue; }
                        let pi = (py*4+px) as u32;
                        let color_index = ((indices >> (2*pi)) & 3) as usize;
                        let mut c = colors[color_index];
                        if kind == 2 {
                            let nibble = ((u64::from_le_bytes(block[0..8].try_into().unwrap()) >> (4*pi)) & 15) as u8;
                            c[3] = nibble * 17;
                        } else if kind == 3 {
                            c[3] = bc3_alpha(block, pi);
                        }
                        rgba[(y * width as usize+x)*4..(y * width as usize+x)*4+4].copy_from_slice(&c);
                    }
                }
            }
        }
    }
    Ok(DecodedDds {width, height, rgba})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(w:u32,h:u32,format:&[u8;4],blocks:&[u8]) -> Vec<u8> {
        let mut x = vec![0u8; 128];
        x[0..4].copy_from_slice(b"DDS ");
        x[4..8].copy_from_slice(&124u32.to_le_bytes());
        x[12..16].copy_from_slice(&h.to_le_bytes());
        x[16..20].copy_from_slice(&w.to_le_bytes());
        x[76..80].copy_from_slice(&32u32.to_le_bytes());
        x[80..84].copy_from_slice(&4u32.to_le_bytes());
        x[84..88].copy_from_slice(format);
        x.extend_from_slice(blocks);
        x
    }

    #[test]
    fn bc1_red_four_by_four() {
        let bytes = fixture(4,4,b"DXT1",&[0x00,0xF8,0xE0,0x07,0,0,0,0]);
        let d = decode_dds(&bytes).unwrap();
        assert_eq!((d.width,d.height),(4,4));
        assert_eq!(&d.rgba[..4], &[255,0,0,255]);
        assert_eq!(d.rgba.len(),64);
    }
    #[test]
    fn bc3_red_with_opaque_alpha() {
        let block = [255,0,0,0,0,0,0,0, 0,0xF8,0xE0,0x07,0,0,0,0];
        let d = decode_dds(&fixture(4,4,b"DXT5",&block)).unwrap();
        assert_eq!(&d.rgba[..4], &[255,0,0,255]);
    }
    #[test]
    fn bc2_red_with_half_alpha() {
        let block = [0x88,0x88,0x88,0x88,0x88,0x88,0x88,0x88, 0,0xF8,0xE0,0x07,0,0,0,0];
        let d = decode_dds(&fixture(4,4,b"DXT3",&block)).unwrap();
        assert_eq!(&d.rgba[..4], &[255,0,0,136]);
    }
    #[test]
    fn bc3_interpolation_is_bounded() {
        let mut block = [255, 0, 0, 0, 0, 0, 0, 0];
        let expected = [255, 0, 218, 182, 145, 109, 72, 36];
        for (i, &value) in expected.iter().enumerate() {
            block[2] = i as u8;
            assert_eq!(bc3_alpha(&block, 0), value);
        }
        // 6-alpha-value variant includes explicit 0 and 255.
        block[0] = 0;
        block[1] = 255;
        block[2] = 6;
        assert_eq!(bc3_alpha(&block, 0), 0);
        block[2] = 7;
        assert_eq!(bc3_alpha(&block, 0), 255);
    }
    #[test]
    fn rejects_oversized_and_bad_header() {
        let mut bytes = fixture(4,4,b"DXT1",&[0u8;8]);
        bytes[0] = 0;
        assert!(decode_dds(&bytes).is_err());
    }
    #[test]
    fn uncompressed_rgba32_uses_dds_channel_masks() {
        let mut bytes = fixture(1, 1, b"NONE", &[0x20, 0x40, 0x80, 0xFF]);
        bytes[80..84].copy_from_slice(&0x40u32.to_le_bytes());
        bytes[88..92].copy_from_slice(&32u32.to_le_bytes());
        bytes[92..96].copy_from_slice(&0x00ff0000u32.to_le_bytes());
        bytes[96..100].copy_from_slice(&0x0000ff00u32.to_le_bytes());
        bytes[100..104].copy_from_slice(&0x000000ffu32.to_le_bytes());
        bytes[104..108].copy_from_slice(&0xff000000u32.to_le_bytes());
        let decoded = decode_dds(&bytes).unwrap();
        assert_eq!(decoded.rgba, [128, 64, 32, 255]);
    }
    #[test]
    fn large_channel_mask_does_not_overflow() {
        assert_eq!(scale_mask(0xFFFF_FFFF, 0xFFFF_FFFF, 0), 255);
        assert_eq!(scale_mask(0, 0xFFFF_FFFF, 0), 0);
    }
    #[test]
    fn truncation_and_limit_are_rejected() {
        assert!(decode_dds(&fixture(4,4,b"DXT1",&[0])).is_err());
        assert!(decode_dds(&fixture(4096,4,b"DXT1",&[0;8])).is_err());
        assert!(decode_dds(&fixture(4,4,b"DX10",&[0;16])).is_err());
    }
    #[test]
    fn bc1_alpha_and_cropped_blocks() {
        let bytes = fixture(2,2,b"DXT1",&[0,0,0,0,0xFF,0xFF,0xFF,0xFF]);
        let d = decode_dds(&bytes).unwrap();
        assert_eq!(d.rgba.len(),16);
        assert_eq!(d.rgba[3],0);
    }
}
