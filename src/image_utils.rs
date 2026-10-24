use std::time::Duration;

#[allow(dead_code)]
pub struct AnimationInfo {
    pub frames: u32,
    pub duration: Duration,
    #[allow(dead_code)]
    pub timestamps_ms: Option<Vec<u64>>,
}

/// Copied from image crate
pub fn resize_dimensions(
    width: u32,
    height: u32,
    nwidth: u32,
    nheight: u32,
    fill: bool,
) -> (u32, u32) {
    let wratio = nwidth as f64 / width as f64;
    let hratio = nheight as f64 / height as f64;

    let ratio = if fill {
        f64::max(wratio, hratio)
    } else {
        f64::min(wratio, hratio)
    };

    let nw = std::cmp::max((width as f64 * ratio).round() as u64, 1);
    let nh = std::cmp::max((height as f64 * ratio).round() as u64, 1);

    if nw > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / width as f64;
        (
            u32::MAX,
            std::cmp::max((height as f64 * ratio).round() as u32, 1),
        )
    } else if nh > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / height as f64;
        (
            std::cmp::max((width as f64 * ratio).round() as u32, 1),
            u32::MAX,
        )
    } else {
        (nw as u32, nh as u32)
    }
}

/// (Optimised) Finds the number of frames and total duration of a GIF.
///
/// # Arguments
///
/// * `data` - The image data.
///
/// # Returns
///
/// The number of frames and total duration, or an error if the data is invalid.
pub fn gif_info(data: &[u8]) -> Result<AnimationInfo, &'static str> {
    if data.len() < 13 {
        return Err("GIF data is too short");
    } else if !matches!(&data[..6], b"GIF87a" | b"GIF89a") {
        return Err("Invalid GIF header");
    }

    let packed = data[10];
    let mut pos = 13;

    // Global Color Table
    if packed & 0x80 != 0 {
        let size = 3usize << ((packed & 0x07) + 1);
        if data.len().saturating_sub(pos) < size {
            return Err("Truncated global color table");
        }
        pos += size;
    }

    // Delay belonging to the next image descriptor.
    let mut pending_delay_cs: u64 = 0u64;
    let mut frames: u32 = 0u32;
    let mut duration_cs: u64 = 0u64;
    let mut timestamps_ms = Vec::new();

    #[inline(always)]
    fn skip_sub_blocks(data: &[u8], pos: &mut usize) -> Result<(), &'static str> {
        loop {
            let size = *data.get(*pos).ok_or("Truncated GIF sub-block")?;
            *pos += 1;
            if size == 0 {
                return Ok(());
            } else if data.len().saturating_sub(*pos) < size as usize {
                return Err("Truncated GIF sub-block data");
            }
            *pos += size as usize;
        }
    }

    loop {
        let b = *data.get(pos).ok_or("Missing GIF trailer")?;
        pos += 1;

        match b {
            // Image Descriptor = one frame
            0x2C => {
                frames = frames.checked_add(1).ok_or("Too many GIF frames")?;

                if data.len().saturating_sub(pos) < 9 {
                    return Err("Truncated image descriptor");
                }
                let packed = data[pos + 8];
                pos += 9;

                // Local Color Table
                if packed & 0x80 != 0 {
                    let size = 3usize << ((packed & 0x07) + 1);
                    if data.len().saturating_sub(pos) < size {
                        return Err("Truncated local color table");
                    }
                    pos += size;
                }

                // LZW minimum code size
                if pos >= data.len() {
                    return Err("Missing LZW code size");
                }
                pos += 1;

                // LZW data sub-blocks
                skip_sub_blocks(data, &mut pos)?;

                // The current frame starts after all previous frame delays.
                timestamps_ms.push(duration_cs * 10);
                duration_cs += pending_delay_cs;
                pending_delay_cs = 0;
            }

            // Extension block
            0x21 => {
                let label = *data.get(pos).ok_or("Truncated extension")?;
                pos += 1;

                if label == 0xF9 {
                    // Graphic Control Extension:
                    //
                    // 21 F9 04 [packed] [delay lo] [delay hi] [transparent] 00
                    if data.len().saturating_sub(pos) < 6 {
                        return Err("Truncated graphic control extension");
                    } else if data[pos] != 4 {
                        return Err("Invalid graphic control extension");
                    }
                    pending_delay_cs = u16::from_le_bytes([data[pos + 2], data[pos + 3]]) as u64;
                    pos += 6;
                } else {
                    // Other extension data
                    skip_sub_blocks(data, &mut pos)?;
                }
            }
            0x3B => {
                duration_cs += pending_delay_cs;
                return Ok(AnimationInfo {
                    frames,
                    duration: Duration::from_millis(duration_cs * 10),
                    timestamps_ms: Some(timestamps_ms),
                });
            }
            _ => {
                return Err("Invalid GIF block introducer");
            }
        }
    }
}
/// (Optimised) Finds the number of frames and total duration of an animated WebP.
///
/// # Arguments
///
/// * `data` - The image data.
///
/// # Returns
///
/// The number of frames and total duration, or an error if the data is invalid.
pub fn webp_info(data: &[u8]) -> Result<AnimationInfo, &'static str> {
    if data.len() < 12 {
        return Err("WebP data is too short");
    } else if &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return Err("Invalid WebP header");
    }

    let riff_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let end = 8usize.checked_add(riff_size).ok_or("Invalid RIFF size")?;

    if end > data.len() {
        return Err("Truncated WebP");
    }

    let mut pos: usize = 12;
    let mut frames: u32 = 0u32;
    let mut duration_ms: u64 = 0u64;
    let mut timestamps_ms = Vec::<u64>::new();

    while end.saturating_sub(pos) >= 8 {
        let chunk_type = &data[pos..pos + 4];
        let chunk_size =
            u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                as usize;

        pos += 8;

        if chunk_size > end.saturating_sub(pos) {
            return Err("Truncated WebP chunk");
        } else if chunk_type == b"ANMF" {
            if chunk_size < 16 {
                return Err("Invalid ANMF chunk");
            }
            frames = frames.checked_add(1).ok_or("Too many WebP frames")?;

            // ANMF:
            //   bytes 0..3   X
            //   bytes 3..6   Y
            //   bytes 6..9   width
            //   bytes 9..12  height
            //   bytes 12..15 duration (24-bit little endian)
            let frame_duration_ms = data[pos + 12] as u64
                | ((data[pos + 13] as u64) << 8)
                | ((data[pos + 14] as u64) << 16);
            timestamps_ms.push(duration_ms);
            duration_ms += frame_duration_ms;
        }

        pos += chunk_size;

        // RIFF chunks are padded to an even size.
        if chunk_size & 1 != 0 {
            pos += 1;
        }
    }

    Ok(AnimationInfo {
        frames,
        duration: Duration::from_millis(duration_ms),
        timestamps_ms: Some(timestamps_ms),
    })
}

pub fn animation_info(data: &[u8]) -> Result<AnimationInfo, &'static str> {
    if data.len() >= 6 && matches!(&data[..6], b"GIF87a" | b"GIF89a") {
        gif_info(data)
    } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        webp_info(data)
    } else {
        Err("Unsupported image format")
    }
}
