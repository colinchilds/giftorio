use wasm_bindgen::prelude::*;

pub fn js_num<N: Into<f64>>(num: N) -> js_sys::Number {
    js_sys::Number::from(num.into())
}

pub fn format_duration<T: Into<u128>>(ms: T) -> String {
    let ms = ms.into();

    let total_seconds = ms / 1000;

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let milliseconds = ms % 1000;

    format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}")
}

/// Checks if the value is the default value for the type.
pub fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &T::default()
}

/// Checks if an Option is None or the default value for the type.
pub fn is_none_or_default<T: Default + PartialEq>(value: &Option<T>) -> bool {
    value.as_ref().is_none_or(|v| v == &T::default())
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

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}
impl<T: Clone> OneOrMany<T> {
    /// Returns a new clone of the contained value.
    pub fn to_vec(&self) -> Vec<T> {
        match self {
            Self::One(v) => vec![v.clone()],
            Self::Many(v) => v.clone(),
        }
    }
}

pub trait JsReflectGetSet: Into<JsValue> + Clone {
    fn reflect_get<T: JsCast>(&self, k: impl Into<JsValue>) -> Result<T, JsValue> {
        js_sys::Reflect::get(&self.clone().into(), &k.into())?.dyn_into()
    }
    fn reflect_set(&self, k: impl Into<JsValue>, v: impl Into<JsValue>) -> Result<bool, JsValue> {
        js_sys::Reflect::set(&self.clone().into(), &k.into(), &v.into())
    }
}
impl<T: Into<JsValue> + Clone> JsReflectGetSet for T {}

pub trait ResultJsExt<T, E> {
    fn map_js_err(self, prefix: &str) -> Result<T, JsValue>;
}
impl<T, E: std::fmt::Display> ResultJsExt<T, E> for Result<T, E> {
    fn map_js_err(self, prefix: &str) -> Result<T, JsValue> {
        self.map_err(|e| JsValue::from_str(&format!("{prefix}: {e}")))
    }
}
