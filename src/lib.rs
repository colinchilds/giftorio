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
    let mut args: models::BlueprintArgs = serde_wasm_bindgen::from_value(options)?;
    let mut frame_data: Option<image_processing::FrameData> = None;

    if matches!(args.mode, models::Mode::Static { .. }) {
        args.signal_compression = None;
        args.target_fps = 1;
    }
    if matches!(args.mode, models::Mode::Full | models::Mode::Static { .. }) {
        // Create iterator over frames. Auto-detects the total number of frames + total duration
        // using custom byte scanning.
        frame_data = Some(image_processing::FrameData::new(image_data, &args)?);
    }

    // Yields JSON or compressed blueprint in chunks
    let mut encoder = blueprint::encoder::BlueprintEncoder::new_from_frame_data(frame_data, &args)?;

    while let Some(chunk) = encoder.next_chunk()? {
        // Directly write the chunk to the disk (no storing a full buffer in RAM)
        let promise = send_chunk.call1(&JsValue::NULL, &js_sys::Uint8Array::from(&chunk[..]))?;
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await?;
    }
    let obj = js_sys::Object::new(); // Technically redundant but allows adding metadata
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
