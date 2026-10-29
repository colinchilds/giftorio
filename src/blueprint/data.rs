use crate::blueprint::models::*;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, sync::Arc};

thread_local! {
    pub static PROTOTYPE_DATA: RefCell<Option<PrototypeData>> = const { RefCell::new(None) };
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ElectricPoleSupplyAreaDistancePrototypeData {
    pub quality: Arc<str>,
    pub supply_area_distance: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ElectricPolePrototypeData {
    pub name: Arc<str>,
    pub supply_area_distances: Box<[ElectricPoleSupplyAreaDistancePrototypeData]>,
    pub tile_width: i32,
    pub tile_height: i32,
}

#[derive(Clone, Serialize, Deserialize)]
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
    let qualities: Vec<Option<Arc<str>>> = {
        let quals = (data.qualities.iter())
            .map(|q| Some(Arc::clone(q)))
            .collect::<Vec<_>>();
        if quals.is_empty() { vec![None] } else { quals }
    };
    let mut signals: Vec<Arc<SignalID>> = Vec::with_capacity(data.signals.len() * qualities.len());

    for sig in data.signals {
        for qual in qualities.iter() {
            signals.push(Arc::from(SignalID {
                type_: if let Some(type_) = sig_types.iter().rfind(|&x| *x == sig.type_) {
                    Arc::clone(type_)
                } else {
                    sig_types.push(Arc::clone(&sig.type_));
                    Arc::clone(&sig.type_)
                },
                name: Arc::clone(&sig.name),
                quality: qual.clone(),
            }));
        }
    }
    data.signals = signals.into_boxed_slice();

    PROTOTYPE_DATA.replace(Some(data));
}
