use image::GenericImageView;
use wasm_bindgen::prelude::*;

mod blueprint;
mod constants;
mod image_processing;
mod image_utils;
mod macros;
mod models;
mod progress;
mod streaming_writer;

fn get_cb<T: js_sys::JsFunction>(
    callbacks: &JsValue,
    name: &str,
) -> Result<js_sys::Function<T>, JsValue> {
    js_sys::Reflect::get(callbacks, &name.into())?
        .dyn_into::<js_sys::Function<T>>()
        .map_err(|_| JsValue::from_str(&format!("{name} must be a function")))
}

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
    callbacks: JsValue,
    // on_chunk: &js_sys::Function<fn(js_sys::Uint8Array) -> js_sys::Promise>,
    // on_preview: &js_sys::Function<
    //     fn(js_sys::Uint8Array, js_sys::Number, js_sys::Number) -> js_sys::Undefined,
    // >,
) -> Result<JsValue, JsValue> {
    use blueprint::{blueprint::BlueprintGenerator, encoder::BlueprintEncoder};
    use image_processing::{FrameCallback, FrameData};
    use models::*;
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};
    use streaming_writer::StreamingWriter;

    let on_chunk = get_cb::<fn(js_sys::Uint8Array) -> js_sys::Promise>(&callbacks, "onChunk")?;
    let on_frame = get_cb::<
        fn(js_sys::Uint8Array, js_sys::Number, js_sys::Number) -> js_sys::Undefined,
    >(&callbacks, "onFrame")?;
    let should_show_preview = get_cb::<fn() -> js_sys::Boolean>(&callbacks, &"shouldShowPreview")?;

    let mut args: BlueprintArgs = serde_wasm_bindgen::from_value(options)?;
    let mut frame_data: Option<FrameData> = None;
    let promises = Rc::new(const { RefCell::new(VecDeque::<Vec<js_sys::Promise>>::new()) });
    let promise_q = Rc::clone(&promises);

    if matches!(args.mode, Mode::Static { .. }) {
        args.signal_compression = None;
        args.target_fps = 1;
    }
    if matches!(args.mode, Mode::Full | Mode::Static { .. }) {
        let on_frame = on_frame.clone();
        let on_frame: FrameCallback = Box::new(move |result| {
            if should_show_preview.call0(&JsValue::NULL).unwrap().as_bool() == Some(false) {
                return;
            }
            let img = result.unwrap_throw();
            let buf = js_sys::Uint8Array::from(img.to_rgba8().as_raw().as_slice());
            let (w, h) = img.dimensions();
            let (w, h) = (js_sys::Number::from(w), js_sys::Number::from(h));
            on_frame.call(&JsValue::NULL, (&buf, &w, &h)).unwrap_throw();
        });

        // Create iterator over frames.
        frame_data = Some(FrameData::new(image_data, &args, Some(on_frame)).unwrap_throw());
    }

    // Custom writer that invokes a callback whenever chunk size is reached.
    let writer = StreamingWriter::new(1 * 1000 * 1000, |chunk| {
        let data = js_sys::Uint8Array::from(chunk.as_slice());
        let promise = on_chunk.call(&JsValue::NULL, (&data,)).unwrap_throw();
        let promise = js_sys::Promise::from(promise);
        promise_q.borrow_mut().back_mut().map(|q| q.push(promise));
    });

    // Yields JSON or compressed blueprint in chunks
    let generator = BlueprintGenerator::new(frame_data, &args).unwrap_throw();
    let mut encoder = BlueprintEncoder::new(generator, &args, writer).unwrap_throw();

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
    while let Some(_) = encoder.next().unwrap_throw() {
        flush().await?; // Wait for all those chunks to be sent
        promises.borrow_mut().push_back(vec![]); // Repopulate
    }

    let mut stream_writer = encoder.finish().unwrap_throw();
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
    blueprint::data::load_data(data_json);
    Ok(())
}

#[wasm_bindgen]
pub async fn set_comb_pos_data(data_json: &[u8]) -> Result<(), JsValue> {
    blueprint::combinator::load_comb_pos_json(data_json)?;
    Ok(())
}
