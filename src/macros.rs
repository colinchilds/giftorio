/// Logs a message to the web console.
#[allow(unused_macros)]
macro_rules! log {
    ($($arg:tt)*) => {{
        let value = format!($($arg)*);
        web_sys::console::log_1(&value.clone().into());
        value
    }};
}

/// Creates a constant value that is lazily evaluated once.
macro_rules! lazy_const {
    (pub $name:ident: $type:ident = $value:expr) => {
        pub const $name: std::sync::LazyLock<std::sync::Arc<$type>> =
            std::sync::LazyLock::new(|| std::sync::Arc::from($value));
    };
    ($name:ident: $type:ident = $value:expr) => {
        const $name: std::sync::LazyLock<std::sync::Arc<$type>> =
            std::sync::LazyLock::new(|| std::sync::Arc::from($value));
    };
}

pub(crate) use lazy_const;
#[allow(unused_imports)]
pub(crate) use log;
