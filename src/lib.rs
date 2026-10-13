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
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};
    use streaming_writer::StreamingWriter;

    let mut args: BlueprintArgs = serde_wasm_bindgen::from_value(options)?;
    let mut frame_data: Option<FrameData> = None;
    let promises: Rc<RefCell<VecDeque<Vec<js_sys::Promise>>>> =
        Rc::new(RefCell::new(VecDeque::new()));
    let promise_q = Rc::clone(&promises);

    if matches!(args.mode, Mode::Static { .. }) {
        args.signal_compression = None;
        args.target_fps = 1;
    }
    if matches!(args.mode, Mode::Full | Mode::Static { .. }) {
        // Create iterator over frames. Auto-detects the total number of frames + total duration
        // using custom byte scanning.
        frame_data = Some(FrameData::new(image_data, &args)?);
    }

    let writer = StreamingWriter::new(1 * 1000 * 1000, move |chunk| {
        let data = js_sys::Uint8Array::from(chunk.as_slice());
        let promise = send_chunk.call(&JsValue::NULL, (&data,)).unwrap_throw();
        let promise = js_sys::Promise::from(promise);
        promise_q.borrow_mut().back_mut().map(|q| q.push(promise));
    });

    // Yields JSON or compressed blueprint in chunks
    let gen = BlueprintGenerator::new(frame_data, &args)?;
    let mut encoder = BlueprintEncoder::new(gen, &args, writer)?;

    let flush = async || -> Result<(), JsValue> {
        // Get the earliest list of promises from the queue
        if let Some(mut q) = std::mem::take(&mut *promises.borrow_mut()).pop_front() {
            for promise in q.drain(..) {
                wasm_bindgen_futures::JsFuture::from(promise).await?;
            }
        }
        Ok(())
    };

    promises.borrow_mut().push_back(vec![]);

    // `encoder.next()` may generate a series of chunks
    while let Some(_) = encoder.next()? {
        flush().await?; // Wait for all those chunks to be sent
        promises.borrow_mut().push_back(vec![]); // Repopulate
    }

    let mut stream_writer = encoder.finish()?;
    stream_writer.finish();

    flush().await?; // Flush tail

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
