use wasm_bindgen::prelude::*;

use crate::macros::log;

mod blueprint;
mod constants;
mod image_processing;
mod image_utils;
mod macros;
mod models;
mod progress;
mod streaming_writer;
mod util;

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
    use image::GenericImageView;
    use image_processing::{FrameCallback, FrameData, JsVideoFrameCallback};
    use js_sys as js;
    use models::*;
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
        rc::Rc,
    };
    use streaming_writer::StreamingWriter;
    use util::{js_num, now_ms};
    use wasm_bindgen_futures::JsFuture;

    const SYNC_TIME_MS: f64 = 1000.0;

    let on_chunk = get_cb::<fn(js::Uint8Array) -> js::Promise>(&callbacks, "onChunk")?;
    let on_frame = get_cb::<
        fn(JsValue, js::Number, js::Number, js::Number, js::Number) -> js::Undefined,
    >(&callbacks, "onFrame")?;
    let should_show_preview = get_cb::<fn() -> js::Boolean>(&callbacks, &"shouldShowPreview")?;
    let yield_to_js = get_cb::<fn() -> js::Promise>(&callbacks, &"yieldToJs")?;

    let mut args: BlueprintArgs = serde_wasm_bindgen::from_value(options)?;
    let mut frame_data: Option<FrameData> = None;
    let promises = Rc::new(const { RefCell::new(VecDeque::<Vec<js::Promise>>::new()) });
    let promise_q = Rc::clone(&promises);

    let on_frame = Rc::new(on_frame);
    let should_preview = Rc::new(should_show_preview);
    let yield_to_js = Rc::new(yield_to_js);
    let next_yield_time = Rc::new(Cell::new(now_ms() + SYNC_TIME_MS));

    // timestamp, pointer, len, width, height -> Promise<JsVideoFrame>
    let js_next_frame = get_cb::<JsVideoFrameCallback>(&callbacks, "getFrameData")?;

    if matches!(args.mode, Mode::Static { .. }) {
        args.signal_compression = None;
        args.target_fps = 1;
    }
    if matches!(args.mode, Mode::Full | Mode::Static { .. }) {
        let on_frame: FrameCallback = Box::new(move |result| {
            let on_frame = on_frame.clone();
            let should_show_preview = Rc::clone(&should_preview);
            let yield_to_js = Rc::clone(&yield_to_js);
            let next_yield_time = Rc::clone(&next_yield_time);

            Box::pin(async move {
                let show_preview = should_show_preview.call0(&JsValue::NULL).unwrap_throw();
                let show_preview = show_preview.as_bool() == Some(true);
                let now = now_ms();

                // yield to js so preview can be updated
                if show_preview || (now > next_yield_time.get()) {
                    next_yield_time.set(now + SYNC_TIME_MS);
                    JsFuture::from(yield_to_js.call0(&JsValue::NULL).unwrap_throw())
                        .await
                        .unwrap_throw();
                }
                if !show_preview {
                    return;
                }

                let img = result.unwrap_throw();
                let rgba = match img.as_rgba8() {
                    Some(rgba) => rgba,
                    _ => &img.to_rgba8(),
                };
                let (ptr, len) = (rgba.as_ptr() as u32, rgba.len() as u32);

                let (w, h) = img.dimensions();
                let mem = wasm_bindgen::memory();
                let args = (&mem, &js_num(ptr), &js_num(len), &js_num(w), &js_num(h));

                on_frame.call(&JsValue::NULL, args).unwrap_throw();
            })
        });

        let js_next_frame = args
            .image_metadata
            .image_type
            .as_ref()
            .is_some_and(|v| v.starts_with("video/"))
            .then(|| js_next_frame.clone());

        log!("DASASASASASASASHERE");

        // Create iterator over frames.
        frame_data =
            Some(FrameData::new(image_data, &args, Some(on_frame), js_next_frame).unwrap_throw());
    }

    // Custom writer that invokes a callback whenever chunk size is reached.
    let writer = StreamingWriter::new(1 * 1024 * 1024, |chunk| {
        let data = js::Uint8Array::from(chunk.as_slice());
        let promise = on_chunk.call(&JsValue::NULL, (&data,)).unwrap_throw();
        promise_q.borrow_mut().back_mut().map(|q| q.push(promise));
    });

    // Yields JSON or compressed blueprint in chunks
    let generator = BlueprintGenerator::new(frame_data, &args)
        .await
        .unwrap_throw();
    let mut encoder = BlueprintEncoder::new(generator, &args, writer).unwrap_throw();

    let flush = async || -> Result<(), JsValue> {
        // Get the earliest list of promises from the queue
        if let Some(mut q) = std::mem::take(&mut *promises.borrow_mut()).pop_front() {
            for promise in q.drain(..) {
                JsFuture::from(promise).await?;
            }
        }
        Ok(())
    };

    promises.borrow_mut().push_back(vec![]);

    // `encoder.next()` may generate a series of chunks
    while let Some(_) = encoder.next().await.unwrap_throw() {
        flush().await?; // Wait for all those chunks to be sent
        promises.borrow_mut().push_back(vec![]); // Repopulate
    }

    let mut stream_writer = encoder.finish().unwrap_throw();
    stream_writer.finish();

    flush().await?; // Flush tail

    let obj = js::Object::new();
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
