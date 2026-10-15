use crate::blueprint::models::*;
use std::{cell::RefCell, sync::Arc};

thread_local! {
    pub static PROTOTYPE_DATA: RefCell<Option<PrototypeData>> = const { RefCell::new(None) };
}

#[derive(serde::Deserialize, Clone)]
pub struct ElectricPoleSupplyAreaDistancePrototypeData {
    pub quality: Arc<str>,
    pub supply_area_distance: f64,
}
#[derive(serde::Deserialize, Clone)]
pub struct ElectricPolePrototypeData {
    pub name: Arc<str>,
    pub supply_area_distances: Box<[ElectricPoleSupplyAreaDistancePrototypeData]>,
    pub tile_width: i32,
    pub tile_height: i32,
}
#[derive(serde::Deserialize, Clone)]
pub struct PrototypeData {
    pub signals: Box<[Arc<SignalID>]>,
    pub qualities: Box<[Arc<str>]>,
    pub electric_poles: Box<[ElectricPolePrototypeData]>,
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
pub fn load_data(data_json: &[u8]) {
    let mut data: PrototypeData = serde_json::from_slice(data_json).unwrap();
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
    data.signals = signals.into_boxed_slice();

    PROTOTYPE_DATA.replace(Some(data));
}
