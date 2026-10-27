use crate::constants::*;
use crate::models::Signal;
use serde_json::Value;
use std::collections::HashMap;
use std::rc::Rc;

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
pub fn get_signals_with_quality(use_dlc: bool) -> Rc<[Rc<Signal>]> {
    let signals_json = if use_dlc {
        include_str!("data/signals-dlc.json")
    } else {
        include_str!("data/signals.json")
    };
    let signals_json: Vec<Value> = serde_json::from_str(signals_json).unwrap();

    let mut type_cache: HashMap<String, Rc<String>> = HashMap::new();
    let mut name_cache: HashMap<String, Rc<String>> = HashMap::new();

    let mut signals_with_quality: Vec<Rc<Signal>> = Vec::with_capacity(signals_json.len() * if use_dlc { 6 } else { 2 });
    for signal in signals_json {
        let type_str = signal["type"].as_str().unwrap();
        let name_str = signal["name"].as_str().unwrap();

        let type_rc = type_cache
            .entry(type_str.to_string())
            .or_insert_with(|| Rc::new(type_str.to_string()))
            .clone();
        let name_rc = name_cache
            .entry(name_str.to_string())
            .or_insert_with(|| Rc::new(name_str.to_string()))
            .clone();

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

        for quality in qualities {
            signals_with_quality.push(Rc::new(Signal {
                type_: Rc::clone(&type_rc),
                name: Rc::clone(&name_rc),
                quality: Some(quality),
            }));
        }
    }

    signals_with_quality.into()
}
