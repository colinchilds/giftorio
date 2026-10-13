use crate::blueprint::models::*;
use std::{cell::RefCell, sync::Arc};

thread_local! {
    pub static QUALITIES: RefCell<Vec<Arc<str>>> = const {
        RefCell::new(Vec::new())
    };
    pub static SIGNALS: RefCell<Vec<Arc<crate::blueprint::models::Signal>>> = const {
        RefCell::new(Vec::new())
    };
}

/// Enhances the provided signals by associating quality levels based on DLC usage.
///
/// # Arguments
///
/// * `signals_csv` - The raw CSV string of signals, as a Uint8Array.
/// * `qualities_csv` - Raw CSV string of qualities, as a Uint8Array.
///
/// # Returns
///
/// A new vector of signal JSON objects with added quality attributes.
pub fn load_signal_data(data_json: &[u8]) {
    #[derive(serde::Deserialize)]
    struct SignalData {
        pub signals: Vec<Arc<Signal>>,
        pub qualities: Vec<Arc<str>>,
    }
    let data: SignalData = serde_json::from_slice(data_json).unwrap();

    let mut sig_types: Vec<Arc<str>> = Vec::with_capacity(data.qualities.len()); // Allow "type" field to reference the same strings
    let mut signals: Vec<Arc<Signal>> =
        Vec::with_capacity(data.signals.len() * data.qualities.len());
    let mut qualities: Vec<Option<Arc<str>>> =
        data.qualities.iter().map(|q| Some(Arc::clone(q))).collect();

    if qualities.is_empty() {
        qualities = vec![None];
    }

    for sig in data.signals.iter() {
        let type_: Arc<str>;
        if let Some(t) = sig_types.iter().rfind(|&x| *x == sig.type_) {
            type_ = Arc::clone(t);
        } else {
            type_ = Arc::clone(&sig.type_);
            sig_types.push(Arc::clone(&sig.type_));
        }

        for q in qualities.iter() {
            signals.push(Arc::from(Signal {
                type_: Arc::clone(&type_),
                name: Arc::clone(&sig.name),
                quality: q.clone(),
            }));
        }
    }

    QUALITIES.with_borrow_mut(|q| *q = data.qualities);
    SIGNALS.with_borrow_mut(|s| *s = signals);
}
