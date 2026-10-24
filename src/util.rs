use wasm_bindgen::prelude::*;

pub fn js_num<N: Into<f64>>(num: N) -> js_sys::Number {
    js_sys::Number::from(num.into())
}

pub trait MapJsErr<T> {
    fn map_js_err(self) -> Result<T, JsValue>;
}

impl<T, E: std::fmt::Debug> MapJsErr<T> for Result<T, E> {
    fn map_js_err(self) -> Result<T, JsValue> {
        self.map_err(|err| JsValue::from_str(&format!("{err:?}")))
    }
}

pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub fn wasm_shared_memory(data: &[u8]) -> Result<js_sys::Object, JsValue> {
    let obj = js_sys::Object::new();

    js_sys::Reflect::set(&obj, &"memory".into(), &wasm_bindgen::memory())?;
    js_sys::Reflect::set(&obj, &"ptr".into(), &js_num(data.as_ptr() as u32))?;
    js_sys::Reflect::set(&obj, &"len".into(), &js_num(data.len() as u32))?;

    Ok(obj)
}
