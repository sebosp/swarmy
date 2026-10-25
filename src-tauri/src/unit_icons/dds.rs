//! Decodes the first mip level of the DDS button textures of the balance data, so that they can
//! be shown as PNG in the frontend. Only the formats of the SC2 button textures are supported:
//! DXT1, DXT5 and uncompressed 32 bit BGRA.

/// The size of the DDS header, including the magic number.
const HEADER_SIZE: usize = 128;

/// An RGBA8 image.
#[derive(Debug, Clone, PartialEq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

/// Expands an RGB565 color to RGBA8.
fn rgb565(color: u16) -> [u8; 4] {
    let r = ((color >> 11) & 0x1f) as u8;
    let g = ((color >> 5) & 0x3f) as u8;
    let b = (color & 0x1f) as u8;
    [
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
        255,
    ]
}

fn mix(a: [u8; 4], b: [u8; 4], a_weight: u16, b_weight: u16) -> [u8; 4] {
    let total = a_weight + b_weight;
    let channel =
        |idx: usize| ((a[idx] as u16 * a_weight + b[idx] as u16 * b_weight) / total) as u8;
    [channel(0), channel(1), channel(2), 255]
}

/// The 16 colors of a DXT color block, `opaque` forces the 4 color mode as DXT3/5 do.
fn color_block(block: &[u8], opaque: bool) -> [[u8; 4]; 16] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let (p0, p1) = (rgb565(c0), rgb565(c1));
    let palette = if c0 > c1 || opaque {
        [p0, p1, mix(p0, p1, 2, 1), mix(p0, p1, 1, 2)]
    } else {
        [p0, p1, mix(p0, p1, 1, 1), [0, 0, 0, 0]]
    };
    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    std::array::from_fn(|pixel| palette[((indices >> (pixel * 2)) & 0b11) as usize])
}

/// The 16 alphas of a DXT5 alpha block.
fn alpha_block(block: &[u8]) -> [u8; 16] {
    let (a0, a1) = (block[0] as u16, block[1] as u16);
    let palette: [u8; 8] = if a0 > a1 {
        std::array::from_fn(|idx| match idx {
            0 => a0 as u8,
            1 => a1 as u8,
            idx => ((a0 * (8 - idx as u16) + a1 * (idx as u16 - 1)) / 7) as u8,
        })
    } else {
        std::array::from_fn(|idx| match idx {
            0 => a0 as u8,
            1 => a1 as u8,
            6 => 0,
            7 => 255,
            idx => ((a0 * (6 - idx as u16) + a1 * (idx as u16 - 1)) / 5) as u8,
        })
    };
    let mut bits = 0u64;
    for (idx, byte) in block[2..8].iter().enumerate() {
        bits |= (*byte as u64) << (8 * idx);
    }
    std::array::from_fn(|pixel| palette[((bits >> (pixel * 3)) & 0b111) as usize])
}

/// Decodes 4x4 blocks of `block_size` bytes into the image.
fn decode_blocks(
    data: &[u8],
    width: u32,
    height: u32,
    block_size: usize,
    decode: impl Fn(&[u8]) -> [[u8; 4]; 16],
) -> Option<Vec<u8>> {
    let (blocks_x, blocks_y) = (width.div_ceil(4) as usize, height.div_ceil(4) as usize);
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for block_y in 0..blocks_y {
        for block_x in 0..blocks_x {
            let offset = (block_y * blocks_x + block_x) * block_size;
            let colors = decode(data.get(offset..offset + block_size)?);
            for (pixel, color) in colors.iter().enumerate() {
                let (x, y) = (block_x * 4 + pixel % 4, block_y * 4 + pixel / 4);
                if x < width as usize && y < height as usize {
                    let idx = (y * width as usize + x) * 4;
                    pixels[idx..idx + 4].copy_from_slice(color);
                }
            }
        }
    }
    Some(pixels)
}

/// Decodes the first mip level of a DDS file, None for unsupported formats.
pub fn decode_dds(bytes: &[u8]) -> Option<RgbaImage> {
    if bytes.get(0..4)? != b"DDS " {
        return None;
    }
    let height = u32_at(bytes, 12)?;
    let width = u32_at(bytes, 16)?;
    let four_cc = bytes.get(84..88)?;
    let rgb_bits = u32_at(bytes, 88)?;
    let data = bytes.get(HEADER_SIZE..)?;
    let pixels = match four_cc {
        b"DXT1" => decode_blocks(data, width, height, 8, |block| color_block(block, false))?,
        b"DXT5" => decode_blocks(data, width, height, 16, |block| {
            let alphas = alpha_block(&block[0..8]);
            let mut colors = color_block(&block[8..16], true);
            for (color, alpha) in colors.iter_mut().zip(alphas) {
                color[3] = alpha;
            }
            colors
        })?,
        // Uncompressed, the masks of the SC2 textures are BGRA.
        [0, 0, 0, 0] if rgb_bits == 32 => data
            .get(..(width * height * 4) as usize)?
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|[b, g, r, a]| [*r, *g, *b, *a])
            .collect(),
        _ => return None,
    };
    Some(RgbaImage {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(width: u32, height: u32, four_cc: &[u8; 4], rgb_bits: u32) -> Vec<u8> {
        let mut header = vec![0u8; HEADER_SIZE];
        header[0..4].copy_from_slice(b"DDS ");
        header[12..16].copy_from_slice(&height.to_le_bytes());
        header[16..20].copy_from_slice(&width.to_le_bytes());
        header[84..88].copy_from_slice(four_cc);
        header[88..92].copy_from_slice(&rgb_bits.to_le_bytes());
        header
    }

    #[test]
    fn it_decodes_a_dxt1_block() {
        let mut dds = header(4, 4, b"DXT1", 0);
        // Red and blue, every pixel uses the first color except the second one.
        dds.extend([0x00, 0xf8, 0x1f, 0x00, 0b0000_0100, 0, 0, 0]);
        let image = decode_dds(&dds).unwrap();
        assert_eq!(&image.pixels[0..4], &[255, 0, 0, 255]);
        assert_eq!(&image.pixels[4..8], &[0, 0, 255, 255]);
        assert_eq!(image.pixels.len(), 64);
    }

    #[test]
    fn it_decodes_dxt5_alpha() {
        let mut dds = header(4, 4, b"DXT5", 0);
        // Alpha 255 and 0, the first pixel uses the second alpha.
        dds.extend([255, 0, 0b001, 0, 0, 0, 0, 0]);
        dds.extend([0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]);
        let image = decode_dds(&dds).unwrap();
        assert_eq!(&image.pixels[0..4], &[255, 255, 255, 0]);
        assert_eq!(&image.pixels[4..8], &[255, 255, 255, 255]);
    }

    #[test]
    fn it_decodes_uncompressed_bgra() {
        let mut dds = header(1, 1, &[0, 0, 0, 0], 32);
        dds.extend([1, 2, 3, 4]);
        assert_eq!(decode_dds(&dds).unwrap().pixels, vec![3, 2, 1, 4]);
    }
}
