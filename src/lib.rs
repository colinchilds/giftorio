use serde_wasm_bindgen::{from_value as from_js_value, to_value as to_js_value};
use wasm_bindgen::prelude::*;

mod blueprint;
mod constants;
mod image_processing;
mod image_utils;
mod macros;
mod models;
mod progress;
mod streaming_writer;
mod util;

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
    image_data: &[u8], // note: may be empty
    args: JsValue,
    callbacks: JsValue,
    // on_chunk: &js_sys::Function<fn(js_sys::Uint8Array) -> js_sys::Promise>,
    // on_preview: &js_sys::Function<
    //     fn(js_sys::Uint8Array, js_sys::Number, js_sys::Number) -> js_sys::Undefined,
    // >,
) -> Result<JsValue, JsValue> {
    use blueprint::{encoder::BlueprintEncoder, generator::BlueprintGenerator};
    use image::GenericImageView;
    use image_processing::{FrameCallback, FrameData, JsVideoFrameCallback};
    use js_sys::{self as js, Function as JsFn};
    use models::*;
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
        rc::Rc,
    };
    use streaming_writer::StreamingWriter;
    use util::*;
    use wasm_bindgen_futures::JsFuture;

    const SYNC_TIME_MS: f64 = 1000.0;

    let get_next_frame = callbacks.reflect_get::<JsFn<JsVideoFrameCallback>>("getFrameData")?;
    let on_chunk = callbacks.reflect_get::<JsFn<fn(js::Uint8Array) -> js::Promise>>("onChunk")?;
    let on_frame = callbacks.reflect_get::<JsFn<fn(JsValue) -> js::Undefined>>("onFrame")?;
    let should_show_preview =
        callbacks.reflect_get::<JsFn<fn() -> js::Boolean>>("shouldShowPreview")?;
    let yield_to_js = callbacks.reflect_get::<JsFn<fn() -> js::Promise>>("yieldToJs")?;

    let on_frame = Rc::new(on_frame);
    let should_show_preview = Rc::new(should_show_preview);
    let yield_to_js = Rc::new(yield_to_js);

    let mut args: BlueprintArgs = from_js_value(args)?;
    let mut frame_data: Option<FrameData> = None;
    let promises = Rc::new(const { RefCell::new(VecDeque::<Vec<js::Promise>>::new()) });
    let promise_q = Rc::clone(&promises);
    let next_yield_time = Rc::new(Cell::new(now_ms() + SYNC_TIME_MS));

    if matches!(args.mode, Mode::Static { .. }) {
        args.signal_compression = None;
        args.target_fps = 1;
    }
    if matches!(args.mode, Mode::Full | Mode::Static { .. }) {
        let on_frame: FrameCallback = Box::new(move |result| {
            let on_frame = on_frame.clone();
            let should_show_preview = Rc::clone(&should_show_preview);
            let yield_to_js = Rc::clone(&yield_to_js);
            let next_yield_time = Rc::clone(&next_yield_time);

            Box::pin(async move {
                let show_preview = should_show_preview.call0(&JsValue::NULL)?;
                let show_preview = show_preview.as_bool() == Some(true);
                let now = now_ms();

                // yield to js so preview can be updated
                if show_preview || (now > next_yield_time.get()) {
                    next_yield_time.set(now + SYNC_TIME_MS);
                    JsFuture::from(yield_to_js.call0(&JsValue::NULL)?).await?;
                }
                if !show_preview {
                    return Ok(());
                }

                let img = result?;
                let rgba = match img.as_rgba8() {
                    Some(rgba) => rgba,
                    _ => &img.to_rgba8(),
                };

                let args_obj = js::Object::new();
                args_obj.reflect_set("outBuffer", wasm_shared_memory(rgba)?)?;
                args_obj.reflect_set("dimensions", to_js_value(&img.dimensions())?)?;

                on_frame.call(&JsValue::NULL, (&args_obj.into(),))?;

                Ok(())
            })
        });

        let js_next_frame = (args.image_metadata.mime_type)
            .starts_with("video/")
            .then(|| get_next_frame.clone());

        // Create iterator over frames.
        frame_data = Some(FrameData::new(
            image_data,
            &args,
            Some(on_frame),
            js_next_frame,
        )?);
    }

    // Custom writer that invokes a callback whenever chunk size is reached.
    let writer = StreamingWriter::new(1024 * 1024, |chunk| {
        let data = js::Uint8Array::from(chunk.as_slice());
        (promise_q.borrow_mut().back_mut().unwrap()).push(on_chunk.call(&JsValue::NULL, (&data,))?);
        Ok(())
    });

    // Yields JSON or compressed blueprint in chunks
    let generator = BlueprintGenerator::new(frame_data, &args).await?;
    let mut encoder = BlueprintEncoder::new(generator, &args, writer)?;

    let flush = async || -> Result<(), JsValue> {
        if let Some(mut q) = { std::mem::take(&mut *promises.borrow_mut()).pop_front() } {
            for promise in q.drain(..) {
                JsFuture::from(promise).await?;
            }
        }
        Ok(())
    };

    promises.borrow_mut().push_back(vec![]);

    // `encoder.next()` may generate a series of chunks
    while encoder.next().await?.is_some() {
        flush().await?; // Wait for all those chunks to be sent
        promises.borrow_mut().push_back(vec![]); // Repopulate
    }

    let mut stream_writer = encoder.finish()?;
    stream_writer.finish().map_js_err("Stream finish err")?;

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

#[wasm_bindgen]
pub async fn encode_gif(args: JsValue, callbacks: JsValue) -> Result<(), JsValue> {
    use image_processing::{JsVideoFrameCallback, encode_gif};
    use js_sys::{self as js, Function as JsFn};
    use util::JsReflectGetSet;

    let args: models::EncodeGifArgs = serde_wasm_bindgen::from_value(args)?;
    let get_next_frame = callbacks.reflect_get::<JsFn<JsVideoFrameCallback>>("getFrameData")?;
    let on_chunk = callbacks.reflect_get::<JsFn<fn(js::Uint8Array) -> js::Promise>>("onChunk")?;

    encode_gif(get_next_frame, on_chunk, &args).await?;

    Ok(())
}
