use wasm_bindgen::prelude::*;

mod progress;
mod image_processing;
mod blueprint;
mod signals;
mod constants;
mod models;

/// Public entry point for WebAssembly.
///
/// # Parameters
///
/// - `image_data`: Byte array containing the GIF/WebP data.
/// - `image_type`: Type of the image ("gif" or "webp").
/// - `use_dlc`: Whether to use additional DLC signals.
/// - `target_fps`: Desired frames per second (won't exceed original FPS).
/// - `max_size`: Maximum dimension (width/height) for downscaling.
/// - `substation_quality`: Quality of substations to use.
/// - `grayscale_bits`: Number of bits for grayscale conversion (0 means full color).
///
/// # Returns
///
/// A Factorio blueprint string on success.
#[wasm_bindgen]
pub fn run_blueprint(
    image_data: &[u8],
    image_type: &str,
    use_dlc: bool,
    target_fps: u32,
    max_size: u32,
    substation_quality: String,
    grayscale_bits: u32,
) -> Result<String, JsValue> {
    // Process the image to extract frames and determine the effective FPS.
    let (frames, fps) = image_processing::process_image(image_data, image_type, max_size, target_fps, grayscale_bits)?;
    if frames.is_empty() {
        return Err(JsValue::from_str("No frames sampled!"));
    }

    // Build the complete blueprint JSON.
    let blueprint_json = blueprint::update_full_blueprint(fps, frames, use_dlc, grayscale_bits, substation_quality)?;

    // Encode the blueprint into a Factorio blueprint string.
    let blueprint_str = blueprint::encode_blueprint(&blueprint_json)?;
    Ok(blueprint_str)
}
