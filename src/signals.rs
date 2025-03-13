use serde_json::{Value, json};

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
pub fn get_signals_with_quality(use_dlc: bool) -> Vec<Value> {
    let signals = get_signal_list(use_dlc);
    signals
        .iter()
        .flat_map(|signal| {
            let mut signals_vec = Vec::new();
            let qualities = if use_dlc {
                vec![
                    "normal",
                    "uncommon",
                    "rare",
                    "epic",
                    "legendary",
                    "quality-unknown",
                ]
            } else {
                vec!["normal", "quality-unknown"]
            };
            for quality in qualities.iter() {
                let mut sig = signal.clone();
                sig["quality"] = json!(quality);
                signals_vec.push(sig);
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
