use crate::blueprint::models::*;
use std::{cell::RefCell, sync::Arc};

thread_local! {
    pub static SIGNAL_DATA: RefCell<Option<SignalData>> = RefCell::new(None);
}

#[derive(serde::Deserialize)]
pub struct SignalData {
    pub signals: Box<[Arc<SignalID>]>,
    pub qualities: Box<[Arc<str>]>,
}

/// Enhances the provided signals by associating quality levels based on DLC usage.
///
/// # Arguments
///
/// * `data_json` - The raw JSON string of signals, as a Uint8Array.
///
/// # Returns
///
/// A new vector of signal JSON objects with added quality attributes.
pub fn load_signal_data(data_json: &[u8]) {
    let data: SignalData = serde_json::from_slice(data_json).unwrap();

    let mut sig_types: Vec<Arc<str>> = Vec::new(); // Allow "type" field to reference the same strings
    let mut signals: Vec<Arc<SignalID>> =
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
            signals.push(Arc::from(SignalID {
                type_: Arc::clone(&type_),
                name: Arc::clone(&sig.name),
                quality: q.clone(),
            }));
        }
    }

    SIGNAL_DATA.replace(Some(SignalData {
        qualities: data.qualities,
        signals: signals.into_boxed_slice(),
    }));
}
