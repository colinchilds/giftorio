use std::sync::Arc;
use crate::constants::*;
use crate::models::Signal;
use serde_json::Value;

/// Enhances the provided signals by associating quality levels based on DLC usage.
///
/// # Arguments
///
/// * `use_dlc` - Whether to include additional quality levels.
/// * `signals` - A vector of signal JSON objects.
///
/// # Returns
///
/// A new vector of signal JSON objects with added quality attributes.
pub fn get_signals_with_quality(use_dlc: bool) -> Vec<Arc<Signal>> {
    get_signal_list(use_dlc)
        .into_iter()
        .flat_map(|signal| {
            let mut signals_vec = Vec::new();
            let qualities = if use_dlc {
                vec![
                    QUALITY_NORMAL,
                    QUALITY_UNCOMMON,
                    QUALITY_RARE,
                    QUALITY_EPIC,
                    QUALITY_LEGENDARY,
                    QUALITY_UNKNOWN,
                ]
            } else {
                vec![QUALITY_NORMAL, QUALITY_UNKNOWN]
            };
            for quality in qualities.iter() {
                let signal = Arc::from(Signal {
                    type_: Arc::new(signal["type"].as_str().unwrap().to_string()),
                    name: Arc::new(signal["name"].as_str().unwrap().to_string()),
                    quality: Some(quality),
                });
                signals_vec.push(signal);
            }
            signals_vec
        })
        .collect()
}

/// Retrieves the list of signals from the embedded JSON file.
///
/// # Arguments
///
/// * `use_dlc` - Whether to include additional signals from the Space Age DLC.
///
/// # Returns
///
/// A vector of signal JSON objects.
fn get_signal_list(use_dlc: bool) -> Vec<Value> {
    let signals_json = if use_dlc {
        include_str!("data/signals-dlc.json")
    } else {
        include_str!("data/signals.json")
    };
    serde_json::from_str(signals_json).unwrap()
}
