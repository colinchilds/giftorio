use js_sys::{Function, Uint8Array};
use std::io::{self, Write};
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
/// - `palette_size`: Number of colors or gray levels (0 means full color, or 256 gray levels).
/// - `grayscale`: Whether to use evenly spaced gray levels instead of an adaptive palette.
/// - `split_frames`: Whether to split frames into more deciders for a much smaller blueprint string.
/// - `on_chunk`: Called with each consecutive piece of the blueprint string, as a `Uint8Array`.
#[wasm_bindgen]
pub fn run_blueprint(
    image_data: &[u8],
    image_type: &str,
    use_dlc: bool,
    target_fps: u32,
    max_size: u32,
    substation_quality: String,
    palette_size: u32,
    grayscale: bool,
    split_frames: bool,
    on_chunk: &Function,
) -> Result<(), JsValue> {
    // Process the image to extract frames and determine the effective FPS.
    let (frames, fps) = image_processing::process_image(image_data, image_type, max_size, target_fps, grayscale)?;
    if frames.is_empty() {
        return Err(JsValue::from_str("No frames sampled!"));
    }

    // Build the complete blueprint JSON.
    let blueprint_json = blueprint::update_full_blueprint(fps, frames, use_dlc, palette_size, grayscale, split_frames, substation_quality)?;

    // Encode the blueprint into a Factorio blueprint string.
    blueprint::encode_blueprint(&blueprint_json, ChunkWriter::new(on_chunk))?;
    Ok(())
}

const CHUNK_SIZE: usize = 1 << 20;

/// Hands output to JavaScript in fixed-size chunks so the full blueprint string,
/// which can be hundreds of MB, never has to fit in wasm memory.
struct ChunkWriter<'a> {
    callback: &'a Function,
    buffer: Vec<u8>,
}

impl<'a> ChunkWriter<'a> {
    fn new(callback: &'a Function) -> Self {
        ChunkWriter { callback, buffer: Vec::with_capacity(CHUNK_SIZE) }
    }
}

impl Write for ChunkWriter<'_> {
    // Always consumes all of `data`: base64's EncoderWriter reports a bogus zero-length
    // write after a partial write, which zlib then treats as an error.
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        for piece in data.chunks(CHUNK_SIZE) {
            let len = piece.len().min(CHUNK_SIZE - self.buffer.len());
            self.buffer.extend_from_slice(&piece[..len]);
            if self.buffer.len() == CHUNK_SIZE {
                self.flush()?;
                self.buffer.extend_from_slice(&piece[len..]);
            }
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        self.callback
            .call1(&JsValue::NULL, &Uint8Array::from(&self.buffer[..]))
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "chunk callback failed"))?;
        self.buffer.clear();
        Ok(())
    }
}
