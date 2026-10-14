use crate::blueprint::{constants::*, models::*};
use std::io;
use wasm_bindgen::*;

pub fn write_to(writer: &mut dyn io::Write, buf: &[u8]) -> Result<(), JsValue> {
    writer.write_all(buf).map_js_err("Write error")
}
pub fn write_json_trimmed_to<T>(writer: &mut dyn io::Write, value: &T) -> Result<(), JsValue>
where
    T: ?Sized + serde::Serialize,
{
    let json_bytes = serde_json::to_vec(value).map_js_err("Write error")?;
    write_to(writer, &json_bytes[1..json_bytes.len() - 1]) // Remove braces
}

pub fn invert_wires(ents: &mut Vec<Entity>, wires: &mut Vec<Wire>) {
    #[inline(always)]
    fn get_swap(wire_id: u32) -> u32 {
        match wire_id {
            1 => 2,
            2 => 1,
            3 => 4,
            4 => 3,
            _ => wire_id,
        }
    }
    for e in ents.iter_mut() {
        if let Some(ControlBehavior::Decider { decider_conditions }) = e.control_behavior.as_mut() {
            decider_conditions.swap_networks();
        }
    }
    for wire in wires.iter_mut() {
        wire[1] = get_swap(wire[1]);
        wire[3] = get_swap(wire[3]);
    }
}

/// Converts an RGB pixel to an integer using a utility function.
///
/// # Arguments
///
/// * `r` - Red channel.
/// * `g` - Green channel.
/// * `b` - Blue channel.
///
/// # Returns
///
/// A vector of CombinatorOutputs for the frame
pub fn color_frame_to_outputs(frame: &image::DynamicImage) -> Result<Vec<i32>, JsValue> {
    #[cfg(target_arch = "wasm32")]
    {
        let rgba = frame.to_rgba8();
        Ok(unsafe { rgba_to_rgb_simd(rgba.as_raw()) })
    }

    #[cfg(not(target_arch = "wasm32"))] // scalar fallback
    {
        use crate::image_processing::rgb_to_int;

        let rgb = frame.to_rgb8();
        let mut outputs = Vec::with_capacity((frame.width() * frame.height()) as usize);

        for chunk in rgb.as_raw().chunks_exact(3) {
            outputs.push(rgb_to_int(chunk[0], chunk[1], chunk[2]) as i32);
        }

        Ok(outputs)
    }
}

/// Packs grayscale frames into output signals by bit-packing pixel values.
///
/// A single output represents the value of a single pixel across multiple frames.
///
/// # Arguments
///
/// * `frames` - A slice of grayscale image frames.
/// * `prev_outputs` - Previous output values, for delta compression.
/// * `grayscale_bits` - Number of bits for grayscale conversion.
///
/// # Returns
///
/// A vector of JSON objects representing output filters.
#[inline(always)]
pub fn grayscale_frames_to_outputs(
    frames: &[image::DynamicImage],
    grayscale_bits: u32,
) -> Result<Vec<i32>, JsValue> {
    if frames.is_empty() {
        return Err(JsValue::from_str("No frames provided for packing"));
    }
    let num_pixels = (frames[0].width() * frames[0].height()) as usize;
    let luma_images: Vec<_> = frames.iter().map(|x| x.to_luma8()).collect();
    let mut outputs: Vec<i32> = vec![0i32; num_pixels];

    match grayscale_bits {
        1 => {
            for i in 0..num_pixels {
                let mut packed: u32 = 0;
                for (j, img) in luma_images.iter().enumerate() {
                    packed |= ((img.as_raw()[i] >= GRAYSCALE_THRESH) as u32) << j;
                }
                outputs[i] = packed as i32;
            }
        }
        4 => {
            for i in 0..num_pixels {
                let mut packed: u32 = 0;
                for (j, img) in luma_images.iter().enumerate() {
                    packed |= ((img.as_raw()[i] >> 4) as u32) << (j * 4);
                }
                outputs[i] = packed as i32;
            }
        }
        8 => {
            for i in 0..num_pixels {
                let mut packed: u32 = 0;
                for (j, img) in luma_images.iter().enumerate() {
                    packed |= (img.as_raw()[i] as u32) << (j * 8);
                }
                outputs[i] = packed as i32;
            }
        }
        _ => return Err(JsValue::from_str("Unsupported grayscale bit depth")),
    }
    Ok(outputs)
}
#[cfg(target_arch = "wasm32")]
use core::arch::wasm32::*;

#[cfg(target_arch = "wasm32")]
#[target_feature(enable = "simd128")]
pub unsafe fn rgba_to_rgb_simd(rgba: &[u8]) -> Vec<i32> {
    debug_assert_eq!(rgba.len() % 4, 0);

    let pixel_count = rgba.len() / 4;
    let zero = i8x16_splat(0);
    let mut output = Vec::<i32>::with_capacity(pixel_count);
    let mut i = 0;

    while i + 16 <= rgba.len() {
        // Load 4 RGBA pixels:
        //
        // [R0 G0 B0 A0 R1 G1 B1 A1 R2 G2 B2 A2 R3 G3 B3 A3]
        let input = v128_load(rgba.as_ptr().add(i) as *const v128);

        // Little-endian output needs:
        //
        // [B0 G0 R0 00 B1 G1 R1 00 B2 G2 R2 00 B3 G3 R3 00]
        //
        // Indices 0..15 = input
        // Indices 16..31 = zero
        let packed =
            i8x16_shuffle::<2, 1, 0, 16, 6, 5, 4, 16, 10, 9, 8, 16, 14, 13, 12, 16>(input, zero);

        // Write the 4 resulting i32s directly.
        v128_store(output.as_mut_ptr().add(i / 4) as *mut v128, packed);

        i += 16;
    }

    output.set_len(pixel_count);

    // Handle remaining pixels.
    while i < rgba.len() {
        let r = rgba[i];
        let g = rgba[i + 1];
        let b = rgba[i + 2];

        output[i / 4] = ((r as i32) << 16) | ((g as i32) << 8) | b as i32;
        i += 4;
    }

    output
}

pub trait ResultJsExt<T, E> {
    fn map_js_err(self, prefix: &str) -> Result<T, JsValue>;
}

impl<T, E: std::fmt::Display> ResultJsExt<T, E> for Result<T, E> {
    fn map_js_err(self, prefix: &str) -> Result<T, JsValue> {
        self.map_err(|e| JsValue::from_str(&format!("{prefix}: {e}")))
    }
}

pub fn is_none_or_empty_string(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|v| v.is_empty())
}
pub fn is_none_or_empty_vec<T>(value: &Option<Vec<T>>) -> bool {
    value.as_ref().is_none_or(|v| v.is_empty())
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}
impl<T: Clone> OneOrMany<T> {
    /// Returns a new clone of the contained value.
    pub fn to_vec(&self) -> Vec<T> {
        match self {
            Self::One(v) => vec![v.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}

pub fn format_duration(ms: u32) -> String {
    let total_seconds = ms / 1000;

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let milliseconds = ms % 1000;

    return format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}");
}
