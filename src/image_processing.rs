use image::AnimationDecoder;
use image::{DynamicImage, Frames, RgbaImage};
use image::imageops::FilterType;
use rayon::prelude::*;
use std::io::Cursor;
use wasm_bindgen::prelude::*;
use crate::constants::{DEFAULT_FRAME_DELAY_MS, MS_PER_SECOND};


/// Creates a streaming frame decoder for the provided image data.
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
/// An iterator that decodes one frame at a time, or a JavaScript error.
pub fn get_frames<'a>(image_data: &'a [u8], image_type: &str) -> Result<Frames<'a>, JsValue> {
    let cursor = Cursor::new(image_data);
    let decoder_result = match image_type {
        "gif" => image::codecs::gif::GifDecoder::new(cursor).map(|d| d.into_frames()),
        "webp" => image::codecs::webp::WebPDecoder::new(cursor).map(|d| d.into_frames()),
        _ => return Err(JsValue::from_str("Unsupported image type. Only 'gif' and 'webp' are allowed.")),
    };

    decoder_result
        .map_err(|e| JsValue::from_str(&format!("{} decode error: {}", image_type.to_uppercase(), e)))
}

/// Reads the delay of every frame in milliseconds.
///
/// # Arguments
///
/// * `image_data` - Raw image data.
/// * `image_type` - Image type (e.g., "gif" or "webp").
///
/// # Returns
///
/// A vector with one delay per frame, or a JavaScript error.
fn get_frame_delays(image_data: &[u8], image_type: &str) -> Result<Vec<u32>, JsValue> {
    if image_type != "gif" {
        return get_frames(image_data, image_type)?
            .map(|frame| frame.map(|f| f.delay().numer_denom_ms().0))
            .collect::<Result<_, _>>()
            .map_err(|e| JsValue::from_str(&format!("Frame decode error: {}", e)));
    }

    let mut options = gif::DecodeOptions::new();
    options.skip_frame_decoding(true);
    let mut decoder = options
        .read_info(Cursor::new(image_data))
        .map_err(|e| JsValue::from_str(&format!("GIF decode error: {}", e)))?;
    let mut delays = Vec::new();
    while let Some(frame) = decoder
        .read_next_frame()
        .map_err(|e| JsValue::from_str(&format!("GIF decode error: {}", e)))?
    {
        // GIF delays are stored in units of 10ms.
        delays.push(u32::from(frame.delay) * 10);
    }
    Ok(delays)
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
    // First pass: gather frame durations.
    let durations: Vec<u32> = get_frame_delays(image_data, image_type)?
        .into_iter()
        .map(|ms| if ms == 0 { DEFAULT_FRAME_DELAY_MS } else { ms })
        .collect();
    if durations.is_empty() {
        return Err(JsValue::from_str("Image contains no frames"));
    }
    let total_ms: u32 = durations.iter().sum();

    // Compute average frame duration and derive FPS.
    let avg_frame_duration = total_ms as f64 / durations.len() as f64;
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

    let mut sample_counts = vec![0usize; durations.len()];
    for &i in &sampled_indices {
        sample_counts[i] += 1;
    }
    let last_sampled = *sampled_indices.last().unwrap();

    // Second pass: decode frames one at a time, keeping only the sampled ones.
    let mut processed: Vec<DynamicImage> = Vec::with_capacity(sampled_indices.len());
    // Bounds how many full-size frames are held in memory at once.
    let batch_size = rayon::current_num_threads() * 2;
    let mut batch: Vec<(RgbaImage, usize)> = Vec::with_capacity(batch_size);
    let mut target_size: Option<(u32, u32)> = None;
    for (i, frame) in get_frames(image_data, image_type)?.enumerate().take(last_sampled + 1) {
        let count = sample_counts.get(i).copied().unwrap_or(0);
        if count == 0 {
            continue;
        }
        let buffer = frame
            .map_err(|e| JsValue::from_str(&format!("Frame decode error: {}", e)))?
            .into_buffer();
        if target_size.is_none() {
            let (width, height) = buffer.dimensions();
            let scale_factor = (max_size as f64 / width as f64)
                .min(max_size as f64 / height as f64)
                .min(1.0);
            target_size = Some((
                (width as f64 * scale_factor).round() as u32,
                (height as f64 * scale_factor).round() as u32,
            ));
        }
        batch.push((buffer, count));
        if batch.len() == batch_size {
            resize_batch(&mut batch, target_size.unwrap(), grayscale_bits, &mut processed);
        }
    }
    if let Some(size) = target_size {
        resize_batch(&mut batch, size, grayscale_bits, &mut processed);
    }
    Ok((processed, effective_fps))
}

/// Resizes (and optionally grayscales) a batch of frames in parallel, appending each result
/// to `processed` as many times as that frame was sampled. Empties the batch.
fn resize_batch(
    batch: &mut Vec<(RgbaImage, usize)>,
    (new_width, new_height): (u32, u32),
    grayscale_bits: u32,
    processed: &mut Vec<DynamicImage>,
) {
    let resized: Vec<(DynamicImage, usize)> = batch
        .par_drain(..)
        .map(|(buffer, count)| {
            let mut img = DynamicImage::ImageRgba8(buffer);

            // Convert to grayscale if requested.
            if grayscale_bits > 0 {
                img = DynamicImage::ImageLuma8(img.to_luma8());
            }
            (img.resize(new_width, new_height, FilterType::Triangle), count)
        })
        .collect();
    for (img, count) in resized {
        for _ in 1..count {
            processed.push(img.clone());
        }
        processed.push(img);
    }
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
