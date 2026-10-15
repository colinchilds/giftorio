use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    static PROGRESS_CALLBACK: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
}

/// Sets the progress callback to be used during blueprint generation.
///
/// # Arguments
///
/// * `callback` - A JavaScript function accepting a percentage and a status message.
#[wasm_bindgen]
pub fn set_progress_callback(callback: js_sys::Function) {
    PROGRESS_CALLBACK.with_borrow_mut(|progress| *progress = Some(callback));
}

/// Reports progress by calling the registered progress callback.
///
/// # Arguments
///
/// * `percentage` - A value between 0 and 1 representing the progress.
/// * `status` - A status message indicating the current stage.
pub fn report_progress<T>(percentage: T, status: &str)
where
    T: Into<f64>,
{
    PROGRESS_CALLBACK.with_borrow(|progress| {
        if let Some(ref callback) = *progress {
            let pct = 100.0 * percentage.into();
            let _ = callback.call2(&JsValue::NULL, &JsValue::from(pct), &JsValue::from(status));
        }
    });
}

pub fn set_progress(start: f64, end: f64, progress: f64, msg: &str) {
    report_progress(progress * (end - start) + start, &msg);
}
