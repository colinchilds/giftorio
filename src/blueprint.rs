use crate::constants::*;
use crate::progress::report_progress;
use crate::signals::get_signals_with_quality;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use image::GenericImageView;
use wasm_bindgen::JsValue;
use crate::image_processing::rgb_to_int;

/// Encodes the blueprint JSON as a Factorio blueprint string.
///
/// # Arguments
///
/// * `blueprint` - The blueprint JSON object.
///
/// # Returns
///
/// A Factorio blueprint string on success.
pub fn encode_blueprint(blueprint: &Value) -> Result<String, JsValue> {
    report_progress(80, "Encoding blueprint...");
    let json_bytes = serde_json::to_vec(blueprint)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

    report_progress(85, "Compressing blueprint...");
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&json_bytes)
        .map_err(|e| JsValue::from_str(&format!("Compression error: {}", e)))?;

    let compressed = encoder.finish()
        .map_err(|e| JsValue::from_str(&format!("Compression finish error: {}", e)))?;

    let b64_encoded = base64::encode(&compressed);
    report_progress(100, "Blueprint generation complete. Loading to browser...");
    Ok(format!("0{}", b64_encoded))
}

/// Generates timer entities and wires for the blueprint’s timing mechanism.
///
/// # Arguments
///
/// * `stop` - Maximum tick value before reset.
/// * `grayscale_bits` - Number of grayscale bits (adds extra timer entities if > 0).
/// * `ticks_per_frame` - Ticks per frame (derived from FPS).
/// * `frames_per_combinator` - Number of frames processed per combinator.
///
/// # Returns
///
/// A tuple with timer entities and wires.
pub fn generate_timer(stop: u32, grayscale_bits: u32, ticks_per_frame: u32, frames_per_combinator: u32) -> (Vec<Value>, Vec<Value>) {
    let timer_entity1 = json!({
        "entity_number": 1,
        "name": "constant-combinator",
        "position": {"x": TIMER_ENTITY1_POSITION.0, "y": TIMER_ENTITY1_POSITION.1},
        "direction": DIRECTION_RIGHT,
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
        "position": {"x": TIMER_ENTITY2_POSITION.0, "y": TIMER_ENTITY2_POSITION.1},
        "direction": DIRECTION_RIGHT,
        "control_behavior": {
            "decider_conditions": {
                "conditions": [
                    {
                        "first_signal": {"type": "virtual", "name": "signal-T"},
                        "constant": stop,
                        "comparator": "<"
                    }
                ],
                "outputs": [
                    {"signal": {"type": "virtual", "name": "signal-T"}}
                ]
            }
        },
        "player_description": "[virtual-signal=signal-T] is our timer that ticks up 60 times per second up to the max ticks for the entire gif. \
When it reaches the max, it will start over, resetting the gif. This timer is used to know which frames to render."
    });
    let timer_entity3 = json!({
        "entity_number": 3,
        "name": "arithmetic-combinator",
        "position": {"x": TIMER_ENTITY3_POSITION.0, "y": TIMER_ENTITY3_POSITION.1},
        "direction": DIRECTION_RIGHT,
         "control_behavior": {
            "arithmetic_conditions": {
                "first_signal": {"type": "virtual", "name": "signal-T"},
                "second_constant": 1,
                "operation": "-",
                "output_signal": {"type": "virtual", "name": "signal-T"}
            }
        }
    });

    let mut entities = vec![timer_entity1, timer_entity2, timer_entity3];
    let mut wires = vec![
        json!([1, 2, 2, 2]), // Constant combinator to decider combinator with green wire
        json!([2, 2, 2, 4]), // Decider combinator to itself with green wire
        json!([2, 2, 3, 2])  // Decider combinator to arithmetic combinator with green wire
    ];

    if grayscale_bits > 0 {
        let timer_entity4 = json!({
            "entity_number": 4,
            "name": "arithmetic-combinator",
            "position": {"x": TIMER_ENTITY4_POSITION.0, "y": TIMER_ENTITY4_POSITION.1},
            "direction": DIRECTION_LEFT,
            "control_behavior": {
                "arithmetic_conditions": {
                    "first_signal": {"type": "virtual", "name": "signal-T"},
                    "second_constant": ticks_per_frame * frames_per_combinator,
                    "operation": "%",
                    "output_signal": {"type": "virtual", "name": "signal-S"}
                }
            }
        });
        entities.push(timer_entity4);

        let timer_entity5 = json!({
            "entity_number": 5,
            "name": "arithmetic-combinator",
            "position": {"x": TIMER_ENTITY5_POSITION.0, "y": TIMER_ENTITY5_POSITION.1},
            "control_behavior": {
                "arithmetic_conditions": {
                    "first_signal": {"type": "virtual", "name": "signal-S"},
                    "second_constant": ticks_per_frame,
                    "operation": "/",
                    "output_signal": {"type": "virtual", "name": "signal-S"}
                }
            }
        });
        entities.push(timer_entity5);

        let timer_entity6 = json!({
            "entity_number": 6,
            "name": "arithmetic-combinator",
            "position": {"x": TIMER_ENTITY6_POSITION.0, "y": TIMER_ENTITY6_POSITION.1},
            "direction": DIRECTION_RIGHT,
            "control_behavior": {
                "arithmetic_conditions": {
                    "first_signal": {"type": "virtual", "name": "signal-S"},
                    "second_constant": grayscale_bits,
                    "operation": "*",
                    "output_signal": {"type": "virtual", "name": "signal-F"}
                }
            },
            "player_description": "Calculates the bit shift necessary for the frame we should be rendering."
        });
        entities.push(timer_entity6);

        wires.push(json!([3, 2, 4, 2]));
        wires.push(json!([4, 4, 5, 2]));
        wires.push(json!([5, 4, 6, 2]));
        wires.push(json!([6, 4, 3, 4]));
    }

    (entities, wires)
}

/// Generates substation entities and wires for powering the blueprint.
///
/// # Arguments
///
/// * `substation_quality` - Quality level ("none", "normal", "uncommon", "rare", "epic", "legendary").
/// * `lamp_width` - Width of the lamp grid.
/// * `lamp_height` - Height of the lamp grid.
/// * `frame_count` - Number of frames (affects vertical coverage).
/// * `start_entity_number` - Starting entity number.
///
/// # Returns
///
/// A tuple with substation entities, their wires, occupied grid cells, and the next entity number.
pub fn generate_substations(
    substation_quality: &str,
    lamp_width: u32,
    lamp_height: u32,
    frame_count: u32,
    start_entity_number: u32,
) -> (Vec<Value>, Vec<Value>, HashSet<(i32, i32)>, u32) {
    if substation_quality == "none" {
        return (Vec::new(), Vec::new(), HashSet::new(), start_entity_number);
    }

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
    let mut frame_coverage_count = (((frame_count as f64) - half_coverage) / (coverage as f64)).ceil() as u32;
    while ((frame_count as f64) - half_coverage + (frame_coverage_count as f64 * 2.0))
        > (frame_coverage_count as f64 * coverage as f64)
    {
        frame_coverage_count += 1;
    }
    let num_substations_width =
        (((lamp_width as f64) - half_coverage) / (coverage as f64)).ceil() as u32 + 1;
    let num_substations_height = (((lamp_height as f64) - half_coverage) / (coverage as f64)).ceil()
        as u32
        + 1
        + frame_coverage_count;
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
                substation_wires.push(json!([
                    current_entity,
                    5,
                    current_entity - num_substations_width,
                    5
                ]));
            }
            if j > 0 {
                substation_wires.push(json!([current_entity, 5, current_entity - 1, 5]));
            }
            current_entity += 1;
        }
    }
    (substation_entities, substation_wires, occupied_cells, current_entity)
}

/// Generates combinator entities and wiring for each frame group.
///
/// # Arguments
///
/// * `frame_sections` - A vector of frame signal sections.
/// * `occupied_y` - Set of Y coordinates occupied by substations.
/// * `ticks_per_group` - Ticks per group.
/// * `base_entity_number` - Starting entity number.
/// * `base_decider_x` - Base X coordinate for decider combinators.
/// * `base_y` - Base Y coordinate for placement.
/// * `max_rows_per_group` - Maximum rows per group.
/// * `grayscale_bits` - Number of grayscale bits (affects extra combinators).
///
/// # Returns
///
/// A tuple containing combinator entities, their wires, and the next entity number.
pub fn generate_frame_combinators(
    frame_sections: &Vec<Vec<Value>>,
    occupied_y: &HashSet<i32>,
    ticks_per_group: u32,
    base_entity_number: u32,
    base_decider_x: f64,
    base_y: f64,
    max_rows_per_group: u32,
    grayscale_bits: u32,
) -> (Vec<Value>, Vec<Value>, u32) {
    let mut current_entity_number = base_entity_number;
    let num_frames = frame_sections.len();
    let mut new_entities = Vec::with_capacity(num_frames * 2 + 3);
    let mut wires = Vec::with_capacity((num_frames * 3) + 4);

    // Add extra arithmetic combinators for grayscale adjustments if needed.
    if grayscale_bits > 0 {
        let shifter1_x = base_decider_x;
        let shifter2_x = shifter1_x + 2.0;
        let shifter1 = json!({
            "entity_number": current_entity_number,
            "name": "arithmetic-combinator",
            "position": {"x": shifter1_x, "y": base_y + 1.0},
            "direction": DIRECTION_RIGHT,
             "control_behavior": {
                "arithmetic_conditions": {
                    "first_signal": {"type": "virtual", "name": "signal-each"},
                    "second_signal": {"type": "virtual", "name": "signal-F"},
                    "operation": ">>",
                    "output_signal": {"type": "virtual", "name": "signal-each"},
                    "first_signal_networks": {"red": true, "green": false}
                }
            }
        });
        new_entities.push(shifter1);
        let first_decider_id = current_entity_number + if grayscale_bits == 1 || grayscale_bits == 4 { 4 } else { 3 };
        wires.push(json!([current_entity_number, 2, first_decider_id, 2]));
        wires.push(json!([current_entity_number, 1, first_decider_id, 3]));
        current_entity_number += 1;

        let shifter2 = json!({
            "entity_number": current_entity_number,
            "name": "arithmetic-combinator",
            "position": {"x": shifter2_x, "y": base_y + 1.0},
            "direction": DIRECTION_RIGHT,
             "control_behavior": {
                "arithmetic_conditions": {
                    "first_signal": {"type": "virtual", "name": "signal-each"},
                    "second_constant": if grayscale_bits == 1 { 1 } else if grayscale_bits == 4 { 15 } else { 255 },
                    "operation": "AND",
                    "output_signal": {"type": "virtual", "name": "signal-each"}
                }
            }
        });
        wires.push(json!([current_entity_number - 1, 4, current_entity_number, 2]));
        new_entities.push(shifter2);
        current_entity_number += 1;

        if grayscale_bits == 1 || grayscale_bits == 4 {
            let shifter3 = json!({
                "entity_number": current_entity_number,
                "name": "arithmetic-combinator",
                "position": {"x": shifter1_x + 1.0, "y": base_y + 2.0},
                "direction": 12,
                 "control_behavior": {
                    "arithmetic_conditions": {
                        "first_signal": {"type": "virtual", "name": "signal-each"},
                        "second_constant": if grayscale_bits == 1 { 255 } else { 17 },
                        "operation": "*",
                        "output_signal": {"type": "virtual", "name": "signal-each"}
                    }
                }
            });
            wires.push(json!([current_entity_number - 1, 4, current_entity_number, 2]));
            new_entities.push(shifter3);
            current_entity_number += 1;
        }
    }

    let mut first_decider = true;
    let mut x_offset = 0.0;
    let mut y_offset = 0.0;
    let mut row_in_this_column = 0;
    let mut previous_first_decider: Option<u32> = None;
    for (i, sections) in frame_sections.iter().enumerate() {
        let mut current_y = base_y - row_in_this_column as f64 - y_offset;
        if occupied_y.contains(&(current_y.floor() as i32)) {
            y_offset += 2.0;
            current_y -= 2.0;
        }

        let decider_num = current_entity_number + 1;
        let lower_bound = (i as u32) * ticks_per_group;
        let upper_bound = (i as u32 + 1) * ticks_per_group;
        let decider_entity = json!({
            "entity_number": decider_num,
            "name": "decider-combinator",
            "position": {"x": base_decider_x + x_offset, "y": current_y},
            "direction": DIRECTION_RIGHT,
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
                    "outputs": sections
                }
            }
        });
        new_entities.push(decider_entity);

        if !first_decider {
            let previous_decider_id = decider_num - 2;
            wires.push(json!([previous_decider_id, 2, decider_num, 2]));
            wires.push(json!([previous_decider_id, 3, decider_num, 3]));
        } else {
            if let Some(prev) = previous_first_decider {
                wires.push(json!([prev, 2, decider_num, 2]));
                wires.push(json!([prev, 3, decider_num, 3]));
            }
            previous_first_decider = Some(decider_num);
        }

        first_decider = false;
        current_entity_number += 2;
        row_in_this_column += 1;

        if row_in_this_column >= max_rows_per_group {
            row_in_this_column = 0;
            first_decider = true;
            y_offset = 0.0;
            x_offset += 2.0;
        }
    }
    (new_entities, wires, current_entity_number)
}

/// Generates a grid of lamp entities for the blueprint.
///
/// # Arguments
///
/// * `lamp_signals` - Slice of signals to assign to each lamp.
/// * `grid_width` - Number of lamps horizontally.
/// * `grid_height` - Number of lamps vertically.
/// * `occupied_cells` - Set of grid cells already occupied.
/// * `start_entity_number` - Starting entity number for lamps.
/// * `start_x` - Starting X coordinate.
/// * `start_y` - Starting Y coordinate.
/// * `use_grayscale` - If true, configure lamps for grayscale mode.
///
/// # Returns
///
/// A tuple with lamp entities, lamp wires, the next entity number, and the top-right lamp entity.
pub fn generate_lamps(
    lamp_signals: &[Value],
    grid_width: u32,
    grid_height: u32,
    occupied_cells: &HashSet<(i32, i32)>,
    start_entity_number: u32,
    start_x: i32,
    start_y: i32,
    use_grayscale: bool,
) -> (Vec<Value>, Vec<Value>, u32, u32) {
    let mut lamp_entities = Vec::new();
    let mut lamp_wires = Vec::new();
    let mut current_entity = start_entity_number;
    let mut previous_entities: HashMap<i32, u32> = HashMap::new();
    let mut top_right_lamp: u32 = 0;
    for r in 0..grid_height as i32 {
        for c in 0..grid_width as i32 {
            let x = start_x + c;
            let y = start_y + r;
            if occupied_cells.contains(&(x, y)) {
                continue;
            }
            let index = (r as u32 * grid_width + c as u32) as usize;
            let colors = if use_grayscale {
                json!({
                    "use_colors": true,
                    "color_mode": 1,
                    "red_signal": lamp_signals.get(index).cloned().unwrap_or(json!({})),
                    "green_signal": lamp_signals.get(index).cloned().unwrap_or(json!({})),
                    "blue_signal": lamp_signals.get(index).cloned().unwrap_or(json!({})),
                })
            } else {
                json!({
                    "use_colors": true,
                    "color_mode": 2,
                    "rgb_signal": lamp_signals.get(index).cloned().unwrap_or(json!({})),
                })
            };
            let lamp = json!({
                "entity_number": current_entity,
                "name": "small-lamp",
                "position": {"x": x, "y": y},
                "control_behavior": colors,
                "always_on": true
            });
            lamp_entities.push(lamp);
            if r == 0 && c > 0 {
                lamp_wires.push(json!([current_entity, 1, current_entity - 1, 1]));
                lamp_wires.push(json!([current_entity, 2, current_entity - 1, 2]));
                top_right_lamp = current_entity;
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
    (lamp_entities, lamp_wires, current_entity, top_right_lamp)
}

/// Builds the complete blueprint JSON by combining all components.
///
/// # Arguments
///
/// * `fps` - Effective frames per second.
/// * `sampled_frames` - Processed image frames.
/// * `use_dlc` - Whether to use DLC signals.
/// * `grayscale_bits` - Number of grayscale bits (0 means color mode).
/// * `signals` - Available signals vector.
/// * `substation_quality` - Quality level for substations.
///
/// # Returns
///
/// The final blueprint as a JSON value.
pub fn update_full_blueprint(
    fps: u32,
    sampled_frames: Vec<image::DynamicImage>,
    use_dlc: bool,
    grayscale_bits: u32,
    substation_quality: &str,
) -> Result<Value, JsValue> {
    report_progress(0, "Starting blueprint update");

    let signals: Vec<Value> = get_signals_with_quality(use_dlc);

    if sampled_frames.is_empty() {
        return Err(JsValue::from_str("No sampled frames"));
    }
    let mut blueprint = json!({
        "blueprint": {
            "icons": [{
                "signal": {"name": "decider-combinator"},
                "index": 1
            }],
            "entities": [],
            "wires": [],
            "item": "blueprint",
            "version": BLUEPRINT_VERSION
        }
    });

    let use_grayscale = grayscale_bits > 0;
    let total_frames = sampled_frames.len() as u32;
    let frames_per_combinator = if grayscale_bits > 0 { 32 / grayscale_bits } else { 1 };
    let (full_width, full_height) = sampled_frames[0].dimensions();
    let max_columns_per_group = ((signals.len() as u32) / full_height).min(full_width);
    let num_groups = (full_width as f64 / max_columns_per_group as f64).ceil() as u32;
    let max_columns_per_group = full_width / num_groups;
    if max_columns_per_group < 1 {
        return Err(JsValue::from_str("Not enough signals for even one column of lamps!"));
    }
    let max_rows_per_group =
        (((total_frames as f64 / ((max_columns_per_group as f64 / 2.0).floor())).ceil()) / frames_per_combinator as f64).ceil() as u32;

    let ticks_per_frame = (60.0 / fps as f64) as u32;
    let stop = total_frames * ticks_per_frame;
    let (timer_entities, timer_wires) = generate_timer(stop, grayscale_bits, ticks_per_frame, frames_per_combinator);

    let mut all_entities = timer_entities;
    let mut all_wires: Vec<Value> = timer_wires;
    let mut next_entity = all_entities
        .iter()
        .map(|e| e.get("entity_number").and_then(|v| v.as_u64()).unwrap_or(0) as u32)
        .max()
        .unwrap_or(0)
        + 1;
    report_progress(10, "Generating power grid");
    let (substation_entities, substation_wires, occupied_cells, next_entity_new) =
        generate_substations(
            substation_quality,
            full_width,
            full_height,
            max_rows_per_group + if grayscale_bits == 1 || grayscale_bits == 4 { 2 } else if grayscale_bits == 8 { 1 } else { 0},
            next_entity,
        );
    next_entity = next_entity_new;
    all_entities.extend(substation_entities);
    all_wires.extend(substation_wires);
    let substation_occupied_y: HashSet<i32> = occupied_cells.iter().map(|(_, y)| *y).collect();
    let mut previous_top_right_lamp: Option<u32> = None;
    for group_index in 0..num_groups {
        let group_left = group_index * max_columns_per_group;
        let group_right = ((group_index + 1) * max_columns_per_group).min(full_width);
        let group_width = group_right - group_left;
        let signals_subset: Vec<Value> = signals
            .iter()
            .cloned()
            .take((group_width * full_height) as usize)
            .collect();

        let group_frames_sections = if use_grayscale {
            sampled_frames
                .chunks(frames_per_combinator as usize)
                .map(|chunk| {
                    let cropped_frames: Vec<image::DynamicImage> = chunk
                        .iter()
                        .map(|frame| frame.crop_imm(group_left, 0, group_width, full_height))
                        .collect();
                    pack_grayscale_frames_to_outputs(&cropped_frames, &signals_subset, grayscale_bits)
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut sections = Vec::new();
            for frame in &sampled_frames {
                let cropped = frame.crop_imm(group_left, 0, group_width, full_height);
                sections.push(frame_to_outputs(&cropped, &signals_subset)?);
            }
            sections
        };

        let group_offset_x = group_index * max_columns_per_group;
        let first_connection_entity = if use_grayscale { next_entity } else { next_entity + 1 };
        let (group_combinators, mut group_comb_wires, new_next_entity) =
            generate_frame_combinators(
                &group_frames_sections,
                &substation_occupied_y,
                ticks_per_frame * frames_per_combinator,
                next_entity,
                group_offset_x as f64 + 0.5,
                if grayscale_bits == 1 || grayscale_bits == 4 { -5.0 } else if grayscale_bits == 8 { -4.0 } else { -3.0 },
                max_rows_per_group,
                grayscale_bits,
            );
        if group_index == 0 {
            group_comb_wires.push(json!([3, 4, first_connection_entity, 2]));
        }
        next_entity = new_next_entity;

        let first_lamp_entity = next_entity;
        let (group_lamps, mut group_lamp_wires, new_next_entity, top_right_lamp) = generate_lamps(
            &signals_subset,
            group_width,
            full_height,
            &occupied_cells,
            next_entity,
            group_offset_x as i32,
            0,
            use_grayscale,
        );
        next_entity = new_next_entity;

        if use_grayscale {
            group_comb_wires.push(json!([first_lamp_entity, 2, first_connection_entity, 2]));
            let last_shifter = if grayscale_bits == 1 || grayscale_bits == 4 { first_connection_entity + 2 } else { first_connection_entity + 1 };
            group_comb_wires.push(json!([first_lamp_entity, 1, last_shifter, 3]));
        } else {
            group_comb_wires.push(json!([first_lamp_entity, 1, first_connection_entity, 3]));
            group_comb_wires.push(json!([first_lamp_entity, 2, first_connection_entity, 2]));
        }

        if let Some(prev) = previous_top_right_lamp {
            group_lamp_wires.push(json!([first_lamp_entity, 2, prev, 2]));
        }
        previous_top_right_lamp = Some(top_right_lamp);
        all_entities.extend(group_combinators);
        all_entities.extend(group_lamps);
        all_wires.extend(group_comb_wires);
        all_wires.extend(group_lamp_wires);

        let percent = 20 + ((group_index + 1) * 50 / num_groups);
        report_progress(
            percent,
            &format!("Processed chunk {}/{}", group_index + 1, num_groups),
        );
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

/// Converts an RGB pixel to an integer using a utility function.
///
/// # Arguments
///
/// * `r` - Red channel.
/// * `g` - Green channel.
/// * `b` - Blue channel.
///
/// # Returns
///
/// An integer representing the RGB color.
pub fn frame_to_outputs(
    frame: &image::DynamicImage,
    signals_subset: &[Value],
) -> Result<Vec<Value>, JsValue> {
    let (width, height) = frame.dimensions();
    let num_pixels = (width * height) as usize;
    if num_pixels > signals_subset.len() {
        return Err(JsValue::from_str(&format!(
            "Frame pixel count ({}) exceeds available signals ({}).",
            num_pixels,
            signals_subset.len()
        )));
    }
    let rgb_image = frame.to_rgb8();
    let pixels = rgb_image.into_raw();
    let mut outputs = Vec::with_capacity(num_pixels);
    for (i, chunk) in pixels.chunks(3).enumerate() {
        if chunk.len() < 3 {
            continue;
        }
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let value = rgb_to_int(r, g, b);
        let mut filter = serde_json::Map::with_capacity(3);
        filter.insert("copy_count_from_input".to_string(), Value::Bool(false));
        filter.insert("constant".to_string(), Value::Number(value.into()));
        filter.insert("signal".to_string(), signals_subset[i].clone());
        outputs.push(Value::Object(filter));
    }
    Ok(outputs)
}

/// Packs grayscale frames into output signals by bit-packing pixel values.
///
/// # Arguments
///
/// * `frames` - A slice of grayscale image frames.
/// * `signals_subset` - The subset of signals to map to each pixel.
/// * `grayscale_bits` - Number of bits for grayscale conversion.
///
/// # Returns
///
/// A vector of JSON objects representing output filters.
pub fn pack_grayscale_frames_to_outputs(
    frames: &[image::DynamicImage],
    signals_subset: &[Value],
    grayscale_bits: u32,
) -> Result<Vec<Value>, JsValue> {
    if frames.is_empty() {
        return Err(JsValue::from_str("No frames provided for packing"));
    }
    let (width, height) = frames[0].dimensions();
    let num_pixels = (width * height) as usize;
    if num_pixels > signals_subset.len() {
        return Err(JsValue::from_str(&format!(
            "Frame pixel count ({}) exceeds available signals ({}).",
            num_pixels,
            signals_subset.len()
        )));
    }
    let luma_images: Vec<_> = frames.iter().map(|frame| frame.to_luma8()).collect();
    let mut outputs = Vec::with_capacity(num_pixels);
    for i in 0..num_pixels {
        let mut packed_value: u32 = 0;
        for (j, img) in luma_images.iter().enumerate() {
            let pixel_value = img.as_raw()[i];
            if grayscale_bits == 1 {
                let binary_value = if pixel_value >= GRAYSCALE_THRESHOLD { 1 } else { 0 };
                packed_value |= (binary_value as u32) << j;
            } else if grayscale_bits == 4 {
                let four_bit = pixel_value >> 4;
                packed_value |= (four_bit as u32) << (4 * j);
            } else if grayscale_bits == 8 {
                packed_value |= (pixel_value as u32) << (8 * j);
            }
        }
        let signed_value = packed_value as i32;
        let mut filter = serde_json::Map::with_capacity(3);
        filter.insert("copy_count_from_input".to_string(), Value::Bool(false));
        filter.insert("constant".to_string(), Value::Number(signed_value.into()));
        filter.insert("signal".to_string(), signals_subset[i].clone());
        outputs.push(Value::Object(filter));
    }
    Ok(outputs)
}
