use wasm_bindgen::prelude::*;
//use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use base64;
use std::io::Write;
use std::collections::{HashMap, HashSet};

// Old import that causes the error:
// use image::{DynamicImage, ImageFormat, GenericImageView, imageops::resize, FilterType};

// Updated imports:
use image::{DynamicImage, GenericImageView};
//use image::{DynamicImage, ImageFormat, GenericImageView};
use image::imageops::{FilterType};
//use image::imageops::{resize, FilterType};
use image::AnimationDecoder;

// Helper: encode the complete blueprint (JSON) as a Factorio blueprint string.
fn encode_blueprint(blueprint: &Value) -> Result<String, JsValue> {
    let json_str = serde_json::to_string(blueprint)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(json_str.as_bytes())
        .map_err(|e| JsValue::from_str(&format!("Compression error: {}", e)))?;
    let compressed = encoder.finish()
        .map_err(|e| JsValue::from_str(&format!("Compression finish error: {}", e)))?;
    let b64_encoded = base64::encode(&compressed);
    Ok(format!("0{}", b64_encoded))
}

// Load and downscale a GIF from a byte slice. Returns a vector of (frame, duration_ms)
fn downscale_gif(gif_data: &[u8], max_size: u32) -> Result<Vec<(DynamicImage, u32)>, JsValue> {
    let cursor = std::io::Cursor::new(gif_data);
    let decoder = image::codecs::gif::GifDecoder::new(cursor)
        .map_err(|e| JsValue::from_str(&format!("GIF decode error: {}", e)))?;
    let frames = decoder.into_frames();
    let frame_vec = frames.collect_frames()
        .map_err(|e| JsValue::from_str(&format!("Frame collection error: {}", e)))?;
    let mut result = Vec::new();
    for frame in frame_vec {
        // Get delay in ms; if unavailable, default to 100
        // Instead of this (which causes an error):
        // let delay = frame.delay().numer_denom_ms().map(|(ms, _)| ms).unwrap_or(100);

        // Use this approach:
        let (ms, _) = frame.delay().numer_denom_ms();
        let delay = if ms == 0 { 100 } else { ms };
        let frame_buffer = frame.into_buffer();
        let img = DynamicImage::ImageRgba8(frame_buffer);
        let (width, height) = img.dimensions();
        let scale_factor = (max_size as f64 / width as f64)
            .min(max_size as f64 / height as f64)
            .min(1.0);
        let new_width = (width as f64 * scale_factor).round() as u32;
        let new_height = (height as f64 * scale_factor).round() as u32;
        let resized = img.resize(new_width, new_height, FilterType::Triangle);
        result.push((resized, delay));
    }
    Ok(result)
}

// Sample frames evenly to reach the target FPS.
fn sample_frames(frames: &[(DynamicImage, u32)], target_fps: u32) -> Vec<DynamicImage> {
    let total_ms: u32 = frames.iter().map(|(_, ms)| ms).sum();
    let total_frames = frames.len();
    let avg_frame_duration = total_ms as f64 / total_frames as f64;
    let target_total_frames = ((total_ms as f64 / 1000.0) * target_fps as f64).round() as u32;
    let mut sampled = Vec::new();
    for i in 0..target_total_frames {
        let target_time = i as f64 * (1000.0 / target_fps as f64);
        let mut orig_index = (target_time / avg_frame_duration).round() as usize;
        if orig_index >= total_frames {
            orig_index = total_frames - 1;
        }
        sampled.push(frames[orig_index].0.clone());
    }
    sampled
}

// Convert an (RGB) pixel to an integer.
fn rgb_to_int(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

// Given a frame and a subset of available signals, convert each pixel to a filter JSON object.
fn frame_to_filters(frame: &DynamicImage, signals_subset: &[Value]) -> Result<Vec<Value>, JsValue> {
    let (width, height) = frame.dimensions();
    let num_pixels = (width * height) as usize;
    if num_pixels > signals_subset.len() {
        return Err(JsValue::from_str(&format!(
            "Frame pixel count ({}) exceeds available signals ({}).",
            num_pixels, signals_subset.len()
        )));
    }
    // Convert the frame to RGB8 for consistent pixel data.
    let rgb_image = frame.to_rgb8();
    let pixels = rgb_image.into_raw();
    let mut filters = Vec::new();
    for (i, chunk) in pixels.chunks(3).enumerate() {
        if chunk.len() < 3 {
            continue;
        }
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let value = rgb_to_int(r, g, b);
        // Build a filter entry.
        let mut filter = serde_json::Map::new();
        filter.insert("index".to_string(), json!(i + 1));
        filter.insert("comparator".to_string(), json!("="));
        filter.insert("count".to_string(), json!(value));
        filter.insert("quality".to_string(), json!("normal"));
        // Merge in the corresponding signal data.
        if let Value::Object(map) = &signals_subset[i] {
            for (k, v) in map {
                filter.insert(k.clone(), v.clone());
            }
        }
        filters.push(Value::Object(filter));
    }
    Ok(filters)
}

// Generate the timer combinators and associated wires.
fn generate_timer(stop: u32) -> (Vec<Value>, Vec<Value>) {
    let timer_entity1 = json!({
        "entity_number": 1,
        "name": "constant-combinator",
        "position": {"x": -2.5, "y": -4.0},
        "direction": 4,
        "control_behavior": {
            "sections": {
                "sections": [
                    {
                        "index": 1,
                        "filters": [
                            {
                                "index": 1,
                                "type": "virtual",
                                "name": "signal-T",
                                "quality": "normal",
                                "comparator": "=",
                                "count": 1
                            },
                            {
                                "index": 2,
                                "type": "virtual",
                                "name": "signal-S",
                                "quality": "normal",
                                "comparator": "=",
                                "count": stop
                            }
                        ]
                    }
                ]
            }
        }
    });
    let timer_entity2 = json!({
        "entity_number": 2,
        "name": "decider-combinator",
        "position": {"x": -1.5, "y": -4.0},
        "direction": 4,
        "control_behavior": {
            "decider_conditions": {
                "conditions": [
                    {
                        "first_signal": {"type": "virtual", "name": "signal-T"},
                        "second_signal": {"type": "virtual", "name": "signal-S"},
                        "comparator": "<"
                    }
                ],
                "outputs": [
                    {"signal": {"type": "virtual", "name": "signal-T"}}
                ]
            }
        }
    });
    let timer_entity3 = json!({
        "entity_number": 3,
        "name": "arithmetic-combinator",
        "position": {"x": -1.5, "y": -3.0},
        "direction": 12,
        "control_behavior": {
            "arithmetic_conditions": {
                "first_signal": {"type": "virtual", "name": "signal-T"},
                "second_constant": 1,
                "operation": "+",
                "output_signal": {"type": "virtual", "name": "signal-T"}
            }
        }
    });
    let entities = vec![timer_entity1, timer_entity2, timer_entity3];
    let wires = vec![
        json!([1, 1, 2, 1]),
        json!([2, 2, 3, 4]),
        json!([2, 4, 3, 2]),
    ];
    (entities, wires)
}

// Generate substations to power the blueprint.
fn generate_substations(
    substation_quality: &str,
    lamp_width: u32,
    lamp_height: u32,
    frame_count: u32,
    start_entity_number: u32,
) -> (Vec<Value>, Vec<Value>, HashSet<(i32, i32)>, u32) {
    let coverage = match substation_quality {
        "normal" => 18,
        "uncommon" => 20,
        "rare" => 22,
        "epic" => 24,
        "legendary" => 28,
        _ => 18,
    };
    let mut substation_entities = Vec::new();
    let mut substation_wires = Vec::new();
    let mut occupied_cells = HashSet::new();
    let mut current_entity = start_entity_number;
    let half_coverage = ((coverage as f64) - 2.0) / 2.0;
    // Compute how many substations are needed above the grid.
    let mut frame_coverage_count = (((frame_count as f64) - half_coverage) / (coverage as f64)).ceil() as u32;
    while ((frame_count as f64) - half_coverage + (frame_coverage_count as f64 * 2.0))
        > (frame_coverage_count as f64 * coverage as f64)
    {
        frame_coverage_count += 1;
    }
    let num_substations_width = (((lamp_width as f64) - half_coverage) / (coverage as f64)).ceil() as u32 + 1;
    let num_substations_height =
        (((lamp_height as f64) - half_coverage) / (coverage as f64)).ceil() as u32 + 1 + frame_coverage_count;
    let start_x = -1;
    let start_y = -1 - (frame_coverage_count as i32 * coverage as i32);
    for i in 0..num_substations_height {
        for j in 0..num_substations_width {
            let x = start_x + (j as i32 * coverage as i32);
            let y = start_y + (i as i32 * coverage as i32);
            let mut substation = json!({
                "entity_number": current_entity,
                "name": "substation",
                "position": {"x": x, "y": y}
            });
            if substation_quality != "normal" {
                substation["quality"] = substation_quality.into();
            }
            occupied_cells.insert((x - 1, y - 1));
            occupied_cells.insert((x - 1, y));
            occupied_cells.insert((x, y - 1));
            occupied_cells.insert((x, y));
            substation_entities.push(substation);
            if i > 0 {
                substation_wires.push(json!([current_entity, 5, current_entity - num_substations_width, 5]));
            }
            if j > 0 {
                substation_wires.push(json!([current_entity, 5, current_entity - 1, 5]));
            }
            current_entity += 1;
        }
    }
    (substation_entities, substation_wires, occupied_cells, current_entity)
}

// Generate combinator pairs for each frame.
fn generate_frame_combinators(
    frames_filters: &Vec<Vec<Value>>,
    occupied_y: &HashSet<i32>,
    ticks_per_frame: f64,
    base_entity_number: u32,
    base_constant_x: f64,
    base_decider_x: f64,
    base_y: f64,
) -> (Vec<Value>, Vec<Value>, u32) {
    let mut new_entities = Vec::new();
    let mut wires = Vec::new();
    let mut current_entity_number = base_entity_number;
    let mut first_decider = true;
    let mut y_offset = 0.0;
    for (i, filters) in frames_filters.iter().enumerate() {
        let mut current_y = base_y - i as f64 - y_offset;
        if occupied_y.contains(&(current_y.floor() as i32)) {
            y_offset += 2.0;
            current_y -= 2.0;
        }
        let constant_num = current_entity_number;
        let decider_num = current_entity_number + 1;
        let constant_entity = json!({
            "entity_number": constant_num,
            "name": "constant-combinator",
            "position": {"x": base_constant_x, "y": current_y},
            "direction": 4,
            "control_behavior": {
                "sections": {
                    "sections": [
                        {"index": 1, "filters": filters}
                    ]
                }
            }
        });
        let lower_bound = i as u32 * ticks_per_frame as u32;
        let upper_bound = (i as u32 + 1) * ticks_per_frame as u32;
        let decider_entity = json!({
            "entity_number": decider_num,
            "name": "decider-combinator",
            "position": {"x": base_decider_x, "y": current_y},
            "direction": 4,
            "control_behavior": {
                "decider_conditions": {
                    "conditions": [
                        {
                            "first_signal": {"type": "virtual", "name": "signal-T"},
                            "constant": lower_bound,
                            "comparator": ">="
                        },
                        {
                            "first_signal": {"type": "virtual", "name": "signal-T"},
                            "constant": upper_bound,
                            "comparator": "<",
                            "compare_type": "and"
                        }
                    ],
                    "outputs": [
                        {"signal": {"type": "virtual", "name": "signal-everything"}}
                    ]
                }
            }
        });
        new_entities.push(constant_entity);
        new_entities.push(decider_entity);
        wires.push(json!([constant_num, 1, decider_num, 1]));
        if !first_decider {
            let previous_decider_id = decider_num - 2;
            wires.push(json!([previous_decider_id, 2, decider_num, 2]));
            wires.push(json!([previous_decider_id, 3, decider_num, 3]));
        }
        first_decider = false;
        current_entity_number += 2;
    }
    (new_entities, wires, current_entity_number)
}

// Generate a grid of lamps.
fn generate_lamps(
    lamp_signals: &[Value],
    grid_width: u32,
    grid_height: u32,
    occupied_cells: &HashSet<(i32, i32)>,
    start_entity_number: u32,
    start_x: i32,
    start_y: i32,
) -> (Vec<Value>, Vec<Value>, u32) {
    let mut lamp_entities = Vec::new();
    let mut lamp_wires = Vec::new();
    let mut current_entity = start_entity_number;
    let mut previous_entities: HashMap<i32, u32> = HashMap::new();
    for r in 0..grid_height as i32 {
        for c in 0..grid_width as i32 {
            let x = start_x + c;
            let y = start_y + r;
            if occupied_cells.contains(&(x, y)) {
                continue;
            }
            let index = (r as u32 * grid_width + c as u32) as usize;
            let lamp = json!({
                "entity_number": current_entity,
                "name": "small-lamp",
                "position": {"x": x, "y": y},
                "control_behavior": {
                    "use_colors": true,
                    "rgb_signal": lamp_signals.get(index).cloned().unwrap_or(json!({})),
                    "color_mode": 2
                },
                "always_on": true
            });
            lamp_entities.push(lamp);
            if r == 0 && c > 0 {
                lamp_wires.push(json!([current_entity, 1, current_entity - 1, 1]));
            }
            if r > 0 {
                if let Some(&prev_entity) = previous_entities.get(&x) {
                    lamp_wires.push(json!([current_entity, 1, prev_entity, 1]));
                }
            }
            previous_entities.insert(x, current_entity);
            current_entity += 1;
        }
    }
    (lamp_entities, lamp_wires, current_entity)
}

// Build the complete blueprint JSON.
fn update_full_blueprint(
    target_fps: u32,
    sampled_frames: Vec<DynamicImage>,
    signals: Vec<Value>,
    substation_quality: &str,
) -> Result<Value, JsValue> {
    let mut blueprint = json!({
        "blueprint": {
            "icons": [{
                "signal": {"name": "decider-combinator"},
                "index": 1
            }],
            "entities": [],
            "wires": [],
            "item": "blueprint",
            "version": 562949955518464u64
        }
    });
    if sampled_frames.is_empty() {
        return Err(JsValue::from_str("No sampled frames"));
    }
    let (full_width, full_height) = sampled_frames[0].dimensions();
    let max_columns_per_group = (signals.len() as u32) / full_height;
    if max_columns_per_group < 1 {
        return Err(JsValue::from_str(
            "Not enough signals for even one column of lamps!",
        ));
    }
    let num_groups = (full_width as f64 / max_columns_per_group as f64).ceil() as u32;
    let ticks_per_frame = 60.0 / target_fps as f64;
    let total_frames = sampled_frames.len() as u32;
    let stop = total_frames * ticks_per_frame as u32;
    let (timer_entities, timer_wires) = generate_timer(stop);
    let mut all_entities = timer_entities;
    let mut all_wires: Vec<Value> = timer_wires;
    let mut next_entity = all_entities
        .iter()
        .map(|e| e.get("entity_number").and_then(|v| v.as_u64()).unwrap_or(0) as u32)
        .max()
        .unwrap_or(0)
        + 1;
    let (substation_entities, substation_wires, occupied_cells, next_entity_new) =
        generate_substations(substation_quality, full_width, full_height, total_frames, next_entity);
    next_entity = next_entity_new;
    all_entities.extend(substation_entities);
    all_wires.extend(substation_wires);
    let substation_occupied_y: HashSet<i32> =
        occupied_cells.iter().map(|(_, y)| *y).collect();
    let mut previous_first_decider_entity: Option<u32> = None;
    for group_index in 0..num_groups {
        let group_left = group_index * max_columns_per_group;
        let group_right = ((group_index + 1) * max_columns_per_group).min(full_width);
        let group_width = group_right - group_left;
        let mut group_frames_filters = Vec::new();
        for frame in &sampled_frames {
            let cropped = frame.crop_imm(group_left, 0, group_width, full_height);
            let signals_subset: Vec<Value> = signals
                .iter()
                .cloned()
                .take((group_width * full_height) as usize)
                .collect();
            let filters = frame_to_filters(&cropped, &signals_subset)?;
            group_frames_filters.push(filters);
        }
        let group_offset_x = group_index * max_columns_per_group;
        let (group_combinators, mut group_comb_wires, new_next_entity) = generate_frame_combinators(
            &group_frames_filters,
            &substation_occupied_y,
            ticks_per_frame,
            next_entity,
            group_offset_x as f64 + 0.5,
            group_offset_x as f64 + 1.5,
            -3.0,
        );
        next_entity = new_next_entity;
        let group_lamp_signals: Vec<Value> = signals
            .iter()
            .cloned()
            .take((group_width * full_height) as usize)
            .collect();
        let (group_lamps, group_lamp_wires, new_next_entity2) =
            generate_lamps(&group_lamp_signals, group_width, full_height, &occupied_cells, next_entity, group_offset_x as i32, 0);
        next_entity = new_next_entity2;
        let first_decider_entity = group_combinators
            .get(1)
            .and_then(|e| e.get("entity_number"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| JsValue::from_str("Missing first decider entity"))? as u32;
        let first_lamp_entity = group_lamps
            .get(0)
            .and_then(|e| e.get("entity_number"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| JsValue::from_str("Missing first lamp entity"))? as u32;
        group_comb_wires.push(json!([first_lamp_entity, 1, first_decider_entity, 3]));
        if group_index == 0 {
            group_comb_wires.push(json!([2, 4, first_decider_entity, 2]));
        }
        if let Some(prev) = previous_first_decider_entity {
            group_comb_wires.push(json!([first_decider_entity, 2, prev, 2]));
        }
        previous_first_decider_entity = Some(first_decider_entity);
        all_entities.extend(group_combinators);
        all_entities.extend(group_lamps);
        all_wires.extend(group_comb_wires);
        all_wires.extend(group_lamp_wires);
    }
    if let Some(bp) = blueprint.get_mut("blueprint") {
        bp.as_object_mut()
            .unwrap()
            .insert("entities".to_string(), Value::Array(all_entities));
        bp.as_object_mut()
            .unwrap()
            .insert("wires".to_string(), Value::Array(all_wires));
    }
    Ok(blueprint)
}

/// Exposed function for WebAssembly.
/// 
/// Parameters:
/// • gif_data: A byte array (e.g. a Uint8Array from JavaScript) containing the GIF.
/// • signals_json: A JSON string containing the available signals.
/// • target_fps: The desired frames per second.
/// • max_size: Maximum dimension (width/height) for downscaling.
/// • substation_quality: The quality of substations to use.
/// 
/// Returns a Factorio blueprint string.
#[wasm_bindgen]
pub fn run_blueprint(
    gif_data: &[u8],
    signals_json: &str,
    target_fps: u32,
    max_size: u32,
    substation_quality: &str,
) -> Result<String, JsValue> {
    // Parse available signals.
    let signals: Vec<Value> = serde_json::from_str(signals_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse signals JSON: {}", e)))?;
    // Downscale the input GIF.
    let frames_with_duration = downscale_gif(gif_data, max_size)?;
    // Sample the frames to achieve the target FPS.
    let sampled_frames = sample_frames(&frames_with_duration, target_fps);
    if sampled_frames.is_empty() {
        return Err(JsValue::from_str("No frames sampled!"));
    }
    // Build the complete blueprint.
    let blueprint_json = update_full_blueprint(target_fps, sampled_frames, signals, substation_quality)?;
    // Encode the blueprint as a Factorio blueprint string.
    let blueprint_str = encode_blueprint(&blueprint_json)?;
    Ok(blueprint_str)
}
