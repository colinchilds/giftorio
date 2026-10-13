use wasm_bindgen::prelude::*;

mod blueprint;
mod constants;
mod image_processing;
mod image_utils;
mod macros;
mod models;
mod progress;
mod streaming_writer;

#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Public entry point for WebAssembly.
///
/// # Parameters
///
/// - `options`: Options dict passed in
/// - `image_data`: A Uint8Array passed in from JS. May be empty if `mode` (specified in `options``) is not `"full"`.
///
/// # Returns
///
/// A Factorio blueprint string on success.
#[wasm_bindgen]
pub async fn run_blueprint(
    options: JsValue,
    image_data: &[u8], // note: may be empty
    send_chunk: &js_sys::Function<fn(js_sys::Uint8Array) -> js_sys::Promise>,
) -> Result<JsValue, JsValue> {
    use blueprint::{blueprint::BlueprintGenerator, encoder::BlueprintEncoder};
    use image_processing::FrameData;
    use models::*;
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };
    use streaming_writer::{ChunkQueue, StreamingWriter};

    let mut args: BlueprintArgs = serde_wasm_bindgen::from_value(options)?;
    let mut frame_data: Option<FrameData> = None;

    if matches!(args.mode, Mode::Static { .. }) {
        args.signal_compression = None;
        args.target_fps = 1;
    }
    if matches!(args.mode, Mode::Full | Mode::Static { .. }) {
        // Create iterator over frames. Auto-detects the total number of frames + total duration
        // using custom byte scanning.
        frame_data = Some(FrameData::new(image_data, &args)?);
    }

    let chunks: ChunkQueue = Arc::new(Mutex::new(VecDeque::new()));
    let writer = StreamingWriter::new(5 * 1024 * 1024, Arc::clone(&chunks));

    // Yields JSON or compressed blueprint in chunks
    let gen = BlueprintGenerator::new(frame_data, &args)?;
    let mut encoder = BlueprintEncoder::new(gen, &args, writer)?;

    // Directly write the chunk to the disk (no storing a full buffer in RAM)

    // Generate and send full chunks as they become available.
    while let Some(_) = encoder.next()? {
        loop {
            let chunk = chunks.lock().unwrap().pop_front();
            let Some(chunk) = chunk else {
                break;
            };

            let buf = js_sys::Uint8Array::from(chunk.as_slice());
            let promise = send_chunk.call1(&JsValue::NULL, &buf)?;
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await?;
        }
    }

    // Finalize zlib/Base64, then emit the remaining partial chunk.
    let mut stream_writer = encoder.finish()?;
    stream_writer.finish();

    // IMPORTANT: send the final chunk(s), including small outputs
    // that never reached the 5 MiB threshold.
    loop {
        let chunk = chunks.lock().unwrap().pop_front();
        let Some(chunk) = chunk else {
            break;
        };

        let buf = js_sys::Uint8Array::from(chunk.as_slice());
        let promise = send_chunk.call1(&JsValue::NULL, &buf)?;
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await?;
    }

    let obj = js_sys::Object::new();
    Ok(JsValue::from(obj))
}

/// Sets the list of signals and qualities available for use in the blueprint.
///
/// # Arguments
///
/// * `signals_csv` - The raw CSV string of signals, as a Uint8Array.
/// * `qualities_csv` - Raw CSV string of qualities, as a Uint8Array.
///
/// # Returns
/// Nothing.
#[wasm_bindgen]
pub async fn set_signal_data(data_json: &[u8]) -> Result<(), JsValue> {
    blueprint::signals::load_signal_data(data_json);
    Ok(())
}
