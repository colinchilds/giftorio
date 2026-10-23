pub fn js_num<N: Into<f64>>(num: N) -> js_sys::Number {
    js_sys::Number::from(num.into())
}

pub trait MapJsErr<T> {
    fn map_js_err(self) -> Result<T, wasm_bindgen::JsValue>;
}

impl<T, E: std::fmt::Debug> MapJsErr<T> for Result<T, E> {
    fn map_js_err(self) -> Result<T, wasm_bindgen::JsValue> {
        self.map_err(|err| wasm_bindgen::JsValue::from_str(&format!("{err:?}")))
    }
}

pub fn now_ms() -> f64 {
    js_sys::Date::now()
}
