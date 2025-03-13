use image::AnimationDecoder;
use image::{DynamicImage, GenericImageView, Frame};
use image::imageops::FilterType;
use rayon::prelude::*;
use std::io::Cursor;
use wasm_bindgen::prelude::*;
use crate::constants::{DEFAULT_FRAME_DELAY_MS, MS_PER_SECOND};

/// Decodes the provided image data into a vector of frames based on the image type.
///
/// Supported types: "gif" and "webp".
///
/// # Arguments
///
/// * `image_data` - A byte slice containing the image data.
/// * `image_type` - The type of the image ("gif" or "webp").
///
/// # Returns
///
/// A vector of `Frame` objects or a JavaScript error.
pub fn get_frames(image_data: &[u8], image_type: &str) -> Result<Vec<Frame>, JsValue> {
    let cursor = Cursor::new(image_data);
    let decoder_result = match image_type {
        "gif" => image::codecs::gif::GifDecoder::new(cursor).map(|d| d.into_frames()),
        "webp" => image::codecs::webp::WebPDecoder::new(cursor).map(|d| d.into_frames()),
        _ => return Err(JsValue::from_str("Unsupported image type. Only 'gif' and 'webp' are allowed.")),
    };

    let frames = decoder_result
        .map_err(|e| JsValue::from_str(&format!("{} decode error: {}", image_type.to_uppercase(), e)))?;

    frames.collect_frames()
        .map_err(|e| JsValue::from_str(&format!("Frame collection error: {}", e)))
}

/// Processes the image by decoding frames, sampling, resizing, and optionally converting to grayscale.
///
/// # Arguments
///
/// * `image_data` - Raw image data.
/// * `image_type` - Image type (e.g., "gif" or "webp").
/// * `max_size` - Maximum width/height for downscaling.
/// * `target_fps` - Desired frames per second (limited by the original FPS).
/// * `grayscale_bits` - Number of bits for grayscale conversion (0 means full color).
///
/// # Returns
///
/// A tuple containing the processed frames (`DynamicImage`s) and the effective FPS.
pub fn process_image(
    image_data: &[u8],
    image_type: &str,
    max_size: u32,
    target_fps: u32,
    grayscale_bits: u32,
) -> Result<(Vec<DynamicImage>, u32), JsValue> {
    // First pass: decode frames and gather durations.
    let frame_vec = get_frames(image_data, image_type)?;
    let mut durations = Vec::with_capacity(frame_vec.len());
    let mut total_ms = 0u32;
    for frame in &frame_vec {
        let (ms, _) = frame.delay().numer_denom_ms();
        let delay = if ms == 0 { DEFAULT_FRAME_DELAY_MS } else { ms };
        durations.push(delay);
        total_ms += delay;
    }

    // Compute average frame duration and derive FPS.
    let avg_frame_duration = total_ms as f64 / frame_vec.len() as f64;
    let original_fps = (MS_PER_SECOND / avg_frame_duration).floor() as u32;
    let effective_fps = target_fps.min(original_fps);

    // Determine target frame count.
    let target_total_frames = ((total_ms as f64 / 1000.0) * effective_fps as f64).round() as usize;

    // Sample frames based on cumulative timing.
    let mut sampled_indices = Vec::with_capacity(target_total_frames);
    let mut next_target_time = 0.0;
    let mut accumulated_time = 0.0;
    for (i, &delay) in durations.iter().enumerate() {
        accumulated_time += delay as f64;
        while accumulated_time >= next_target_time && sampled_indices.len() < target_total_frames {
            sampled_indices.push(i);
            next_target_time += MS_PER_SECOND / effective_fps as f64;
        }
    }
    if sampled_indices.is_empty() {
        sampled_indices.push(0);
    }

    // Second pass: process the sampled frames in parallel.
    let processed: Vec<DynamicImage> = sampled_indices
        .par_iter()
        .map(|&i| {
            let frame = &frame_vec[i];
            let mut img = DynamicImage::ImageRgba8(frame.clone().into_buffer());

            // Convert to grayscale if requested.
            if grayscale_bits > 0 {
                img = DynamicImage::ImageLuma8(img.to_luma8());
            }

            let (width, height) = img.dimensions();
            let scale_factor = (max_size as f64 / width as f64)
                .min(max_size as f64 / height as f64)
                .min(1.0);
            let new_width = (width as f64 * scale_factor).round() as u32;
            let new_height = (height as f64 * scale_factor).round() as u32;
            img.resize(new_width, new_height, FilterType::Triangle)
        })
        .collect();
    Ok((processed, effective_fps))
}

/// Converts an RGB pixel to a single 24 bit integer (inside a u32, I know...).
///
/// # Arguments
///
/// * `r` - Red channel (0–255).
/// * `g` - Green channel (0–255).
/// * `b` - Blue channel (0–255).
///
/// # Returns
///
/// An integer representing the RGB value.
pub fn rgb_to_int(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
