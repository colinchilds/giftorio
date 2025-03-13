use js_sys::Function;
use std::cell::RefCell;
use std::thread_local;
use wasm_bindgen::prelude::*;

thread_local! {
    static PROGRESS_CALLBACK: RefCell<Option<Function>> = RefCell::new(None);
}

/// Sets the progress callback to be used during blueprint generation.
///
/// # Arguments
///
/// * `callback` - A JavaScript function accepting a percentage and a status message.
#[wasm_bindgen]
pub fn set_progress_callback(callback: Function) {
    PROGRESS_CALLBACK.with(|progress| {
        *progress.borrow_mut() = Some(callback);
    });
}

/// Reports progress by calling the registered progress callback.
///
/// # Arguments
///
/// * `percentage` - A value between 0 and 100 representing the progress.
/// * `status` - A status message indicating the current stage.
pub fn report_progress(percentage: u32, status: &str) {
    PROGRESS_CALLBACK.with(|progress| {
        if let Some(ref callback) = *progress.borrow() {
            let _ = callback.call2(&JsValue::NULL, &JsValue::from(percentage), &JsValue::from(status));
        }
    });
}
