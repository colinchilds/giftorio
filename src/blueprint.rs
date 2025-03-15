use crate::constants::*;
use crate::image_processing::rgb_to_int;
use crate::models::*;
use crate::progress::report_progress;
use crate::signals::get_signals_with_quality;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::GenericImageView;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use wasm_bindgen::JsValue;
use std::sync::Arc;

/// Encodes the blueprint JSON as a Factorio blueprint string.
///
/// # Arguments
///
/// * `blueprint` - The blueprint JSON object.
///
/// # Returns
///
/// A Factorio blueprint string on success.
pub fn encode_blueprint(blueprint: &Blueprint) -> Result<String, JsValue> {
    report_progress(80, "Encoding blueprint...");
    let json_bytes = serde_json::to_vec(blueprint)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

    report_progress(85, "Compressing blueprint...");
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(&json_bytes)
        .map_err(|e| JsValue::from_str(&format!("Compression error: {}", e)))?;
    let compressed = encoder
        .finish()
        .map_err(|e| JsValue::from_str(&format!("Compression finish error: {}", e)))?;

    let b64_encoded = base64::encode(&compressed);
    report_progress(100, "Blueprint generation complete. Loading to browser...");
    Ok(format!("0{}", b64_encoded))
}

/// Generates timer entities and wires for the blueprint's timing mechanism.
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
pub fn generate_timer(
    stop: u32,
    grayscale_bits: u32,
    ticks_per_frame: u32,
    frames_per_combinator: u32,
) -> (Vec<Entity>, Vec<Wire>) {
    let mut entities = Vec::new();
    let mut wires = Vec::new();

    entities.push(
        Entity::new(1,
            CONSTANT_COMBINATOR,
            Position {
                x: TIMER_ENTITY1_POSITION.0,
                y: TIMER_ENTITY1_POSITION.1,
            },
        )
        .with_direction(DIRECTION_RIGHT)
        .with_control_behavior(ControlBehavior::Constant {
            sections: Sections {
                sections: vec![Section {
                    index: 1,
                    filters: vec![Filter {
                        index: 1,
                        type_: SIGNAL_TYPE_VIRTUAL,
                        name: SIGNAL_T,
                        quality: Some(QUALITY_NORMAL),
                        comparator: Some(COMPARATOR_EQUAL),
                        count: Some(1),
                    }],
                }],
            },
        }),
    );

    entities.push(
        Entity::new(2,
            DECIDER_COMBINATOR,
            Position {
                x: TIMER_ENTITY2_POSITION.0,
                y: TIMER_ENTITY2_POSITION.1
            }
        )
        .with_direction(DIRECTION_RIGHT)
        .with_control_behavior(ControlBehavior::Decider {
            decider_conditions: DeciderConditions {
                conditions: vec![Condition {
                    first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_T.to_string()), quality: None },
                    constant: stop as i32,
                    comparator: COMPARATOR_LESS,
                    compare_type: None,
                }],
                outputs: vec![CombinatorOutput {
                    copy_count_from_input: true,
                    constant: None,
                    signal: Arc::from(Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_T.to_string()), quality: None }),
                }],
            },
        })
        .with_description("[virtual-signal=signal-T] is our timer that ticks up 60 times per second up to the max ticks for the entire gif. \
        When it reaches the max, it will start over, resetting the gif. This timer is used to know which frames to render."));

    entities.push(
        Entity::new(
            3,
            ARITHMETIC_COMBINATOR,
            Position {
                x: TIMER_ENTITY3_POSITION.0,
                y: TIMER_ENTITY3_POSITION.1,
            },
        )
        .with_direction(DIRECTION_RIGHT)
        .with_control_behavior(ControlBehavior::Arithmetic {
            arithmetic_conditions: ArithmeticConditions {
                first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_T.to_string()), quality: None },
                second_signal: None,
                second_constant: Some(1),
                operation: OPERATION_SUB,
                output_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_T.to_string()), quality: None },
            },
        }),
    );

    wires.push([1, 2, 2, 2]);
    wires.push([2, 2, 2, 4]);
    wires.push([2, 2, 3, 2]);

    if grayscale_bits > 0 {
        entities.push(
            Entity::new(
                4,
                ARITHMETIC_COMBINATOR,
                Position {
                    x: TIMER_ENTITY4_POSITION.0,
                    y: TIMER_ENTITY4_POSITION.1,
                },
            )
            .with_direction(DIRECTION_LEFT)
            .with_control_behavior(ControlBehavior::Arithmetic {
                arithmetic_conditions: ArithmeticConditions {
                    first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_T.to_string()), quality: None },
                    second_signal: None,
                    second_constant: Some((ticks_per_frame * frames_per_combinator) as i32),
                    operation: OPERATION_MOD,
                    output_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_S.to_string()), quality: None },
                },
            }),
        );

        entities.push(
            Entity::new(
                5,
                ARITHMETIC_COMBINATOR,
                Position {
                    x: TIMER_ENTITY5_POSITION.0,
                    y: TIMER_ENTITY5_POSITION.1,
                },
            )
            .with_control_behavior(ControlBehavior::Arithmetic {
                arithmetic_conditions: ArithmeticConditions {
                    first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_S.to_string()), quality: None },
                    second_signal: None,
                    second_constant: Some(ticks_per_frame as i32),
                    operation: OPERATION_DIV,
                    output_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_F.to_string()), quality: None },
                },
            }),
        );

        entities.push(
            Entity::new(
                6,
                ARITHMETIC_COMBINATOR,
                Position {
                    x: TIMER_ENTITY6_POSITION.0,
                    y: TIMER_ENTITY6_POSITION.1,
                },
            )
            .with_direction(DIRECTION_RIGHT)
            .with_control_behavior(ControlBehavior::Arithmetic {
                arithmetic_conditions: ArithmeticConditions {
                    first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_EACH.to_string()), quality: None },
                    second_signal: None,
                    second_constant: Some(grayscale_bits as i32),
                    operation: OPERATION_MUL,
                    output_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_EACH.to_string()), quality: None },
                },
            })
            .with_description("Calculates the bit shift necessary for the frame we should be rendering."),
        );

        wires.push([3, 2, 4, 2]);
        wires.push([4, 4, 5, 2]);
        wires.push([5, 4, 6, 2]);
        wires.push([6, 4, 3, 4]);
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
    substation_quality: String,
    lamp_width: u32,
    lamp_height: u32,
    frame_count: u32,
    start_entity_number: u32,
) -> (Vec<Entity>, Vec<Wire>, HashSet<(i32, i32)>, u32) {
    if substation_quality == QUALITY_NONE {
        return (Vec::new(), Vec::new(), HashSet::new(), start_entity_number);
    }
    let coverage = match substation_quality.as_str() {
        QUALITY_NORMAL => 18,
        QUALITY_UNCOMMON => 20,
        QUALITY_RARE => 22,
        QUALITY_EPIC => 24,
        QUALITY_LEGENDARY => 28,
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
            let mut entity = Entity::new(
                current_entity,
                SUBSTATION,
                Position {
                    x: x as f64,
                    y: y as f64,
                },
            );
            entity.quality = if substation_quality.as_str() != QUALITY_NORMAL {
                Some(substation_quality.clone())
            } else {
                None
            };
            substation_entities.push(entity);

            // Mark occupied cells.
            occupied_cells.insert((x - 1, y - 1));
            occupied_cells.insert((x - 1, y));
            occupied_cells.insert((x, y - 1));
            occupied_cells.insert((x, y));
            if i > 0 {
                substation_wires.push([current_entity, 5, current_entity - num_substations_width, 5]);
            }
            if j > 0 {
                substation_wires.push([current_entity, 5, current_entity - 1, 5]);
            }
            current_entity += 1;
        }
    }
    (
        substation_entities,
        substation_wires,
        occupied_cells,
        current_entity,
    )
}

/// Generates combinator entities and wiring for each frame group.
///
/// # Arguments
///
/// * `frame_outputs` - A vector of all the outputs for a frame.
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
    frame_outputs: &[Vec<CombinatorOutput>],
    occupied_y: &HashSet<i32>,
    ticks_per_group: u32,
    base_entity_number: u32,
    base_decider_x: f64,
    base_y: f64,
    max_rows_per_group: u32,
    grayscale_bits: u32,
) -> (Vec<Entity>, Vec<Wire>, u32) {
    let mut current_entity_number = base_entity_number;
    let num_frames = frame_outputs.len();
    let mut new_entities = Vec::with_capacity(num_frames * 2 + 3);
    let mut wires = Vec::with_capacity(num_frames * 3 + 4);

    if grayscale_bits > 0 {
        let shifter1_x = base_decider_x;
        let shifter2_x = shifter1_x + 2.0;
        new_entities.push(
            Entity::new(
                current_entity_number,
                ARITHMETIC_COMBINATOR,
                Position {
                    x: shifter1_x,
                    y: base_y + 1.0,
                },
            )
            .with_direction(DIRECTION_RIGHT)
            .with_control_behavior(ControlBehavior::Arithmetic {
                arithmetic_conditions: ArithmeticConditions {
                    first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_EACH.to_string()), quality: None },
                    second_signal: Some(Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_F.to_string()), quality: None }),
                    second_constant: None,
                    operation: OPERATION_SHIFT_R,
                    output_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_EACH.to_string()), quality: None },
                },
            }),
        );

        let first_decider_id = current_entity_number + if grayscale_bits == 1 || grayscale_bits == 4 { 4 } else { 3 };
        wires.push([current_entity_number, 2, first_decider_id, 2]);
        wires.push([current_entity_number, 1, first_decider_id, 3]);
        current_entity_number += 1;

        new_entities.push(
            Entity::new(
                current_entity_number,
                ARITHMETIC_COMBINATOR,
                Position {
                    x: shifter2_x,
                    y: base_y + 1.0,
                },
            )
            .with_direction(DIRECTION_RIGHT)
            .with_control_behavior(ControlBehavior::Arithmetic {
                arithmetic_conditions: ArithmeticConditions {
                    first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_EACH.to_string()), quality: None },
                    second_signal: None,
                    second_constant: Some(if grayscale_bits == 1 { 1 } else if grayscale_bits == 4 { 15 } else { 255 }),
                    operation: OPERATION_AND,
                    output_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_EACH.to_string()), quality: None },
                },
            }),
        );
        wires.push([current_entity_number - 1, 4, current_entity_number, 2]);
        current_entity_number += 1;
        if grayscale_bits == 1 || grayscale_bits == 4 {
            new_entities.push(
                Entity::new(
                    current_entity_number,
                    ARITHMETIC_COMBINATOR,
                    Position {
                        x: shifter2_x + 1.0,
                        y: base_y + 2.0,
                    },
                )
                .with_direction(DIRECTION_LEFT)
                .with_control_behavior(ControlBehavior::Arithmetic {
                    arithmetic_conditions: ArithmeticConditions {
                        first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_EACH.to_string()), quality: None },
                        second_signal: None,
                        second_constant: Some(if grayscale_bits == 1 { 255 } else { 17 }),
                        operation: OPERATION_MUL,
                        output_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_EACH.to_string()), quality: None },
                    },
                }),
            );
            wires.push([current_entity_number - 1, 4, current_entity_number, 2]);
            current_entity_number += 1;
        }
    }

    let mut first_decider = true;
    let mut x_offset = 0.0;
    let mut y_offset = 0.0;
    let mut row_in_this_column = 0;
    let mut previous_first_decider: Option<u32> = None;
    for (i, outputs) in frame_outputs.iter().enumerate() {
        let mut current_y = base_y - row_in_this_column as f64 - y_offset;
        if occupied_y.contains(&(current_y.floor() as i32)) {
            y_offset += 2.0;
            current_y -= 2.0;
        }
        let decider_num = current_entity_number + 1;
        let lower_bound = (i as u32 * ticks_per_group) as i32;
        let upper_bound = ((i as u32 + 1) * ticks_per_group) as i32;
        let decider_entity = Entity::new(
            decider_num,
            DECIDER_COMBINATOR,
            Position {
                x: base_decider_x + x_offset,
                y: current_y,
            },
        )
        .with_direction(DIRECTION_RIGHT)
        .with_control_behavior(ControlBehavior::Decider {
            decider_conditions: DeciderConditions {
                conditions: vec![
                    Condition {
                        first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_T.to_string()), quality: None },
                        constant: lower_bound,
                        comparator: COMPARATOR_GREATER_EQUAL,
                        compare_type: None,
                    },
                    Condition {
                        first_signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(SIGNAL_T.to_string()), quality: None },
                        constant: upper_bound,
                        comparator: COMPARATOR_LESS,
                        compare_type: Some(COMPARE_AND),
                    },
                ],
                outputs: outputs.clone(), // Cloning the outputs once per entity.
            },
        });
        new_entities.push(decider_entity);

        if !first_decider {
            let previous_decider_id = decider_num - 2;
            wires.push([previous_decider_id, 2, decider_num, 2]);
            wires.push([previous_decider_id, 3, decider_num, 3]);
        } else {
            if let Some(prev) = previous_first_decider {
                wires.push([prev, 2, decider_num, 2]);
                wires.push([prev, 3, decider_num, 3]);
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
/// * `lamp_signals` - Signals to assign to each lamp.
/// * `grid_width` - Number of lamps horizontally.
/// * `grid_height` - Number of lamps vertically.
/// * `occupied_cells` - Set of grid cells already occupied by substations.
/// * `start_entity_number` - Starting entity number for lamps.
/// * `start_x` - Starting X coordinate.
/// * `start_y` - Starting Y coordinate.
/// * `use_grayscale` - If true, configure lamps for grayscale mode.
///
/// # Returns
///
/// A tuple with lamp entities, lamp wires, the next entity number, and the top-right lamp entity.
pub fn generate_lamps(
    signals: Vec<Arc<Signal>>,
    grid_width: u32,
    grid_height: u32,
    occupied_cells: &HashSet<(i32, i32)>,
    start_entity_number: u32,
    start_x: i32,
    start_y: i32,
    use_grayscale: bool,
) -> (Vec<Entity>, Vec<Wire>, u32, u32) {
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
            let signal = Arc::clone(&signals[index]);
            let colors = if use_grayscale {
                ControlBehavior::GrayLamp {
                    use_colors: true,
                    color_mode: 1,
                    red_signal: signal.clone(),
                    green_signal: signal.clone(),
                    blue_signal: signal.clone(),
                }
            } else {
                ControlBehavior::ColorLamp {
                    use_colors: true,
                    color_mode: 2,
                    rgb_signal: signal,
                }
            };
            let lamp = Entity::new(
                current_entity,
                LAMP,
                Position {
                    x: x as f64,
                    y: y as f64,
                },
            )
            .with_control_behavior(colors)
            .with_always_on(true);
            lamp_entities.push(lamp);
            if r == 0 && c > 0 {
                lamp_wires.push([current_entity, 1, current_entity - 1, 1]);
                lamp_wires.push([current_entity, 2, current_entity - 1, 2]);
                top_right_lamp = current_entity;
            }
            if r > 0 {
                if let Some(&prev_entity) = previous_entities.get(&x) {
                    lamp_wires.push([current_entity, 1, prev_entity, 1]);
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
    substation_quality: String,
) -> Result<Blueprint, JsValue> {
    report_progress(0, "Starting blueprint update");

    // Get signals internally.
    let signals: Vec<Arc<Signal>> = get_signals_with_quality(use_dlc);

    if sampled_frames.is_empty() {
        return Err(JsValue::from_str("No sampled frames"));
    }

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
    let mut all_wires: Vec<Wire> = timer_wires;
    let mut next_entity = all_entities
        .iter()
        .map(|e| e.entity_number)
        .max()
        .unwrap_or(0)
        + 1;
    report_progress(10, "Generating power grid");
    let (substation_entities, substation_wires, occupied_cells, next_entity_new) =
        generate_substations(
            substation_quality,
            full_width,
            full_height,
            max_rows_per_group + if grayscale_bits == 1 || grayscale_bits == 4 { 2 } else if grayscale_bits == 8 { 1 } else { 0 },
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

        let group_frames_outputs = if use_grayscale {
            sampled_frames
                .chunks(frames_per_combinator as usize)
                .map(|chunk| {
                    let cropped_frames: Vec<image::DynamicImage> = chunk
                        .iter()
                        .map(|frame| frame.crop_imm(group_left, 0, group_width, full_height))
                        .collect();
                    pack_grayscale_frames_to_outputs(&cropped_frames, signals.clone(), grayscale_bits)
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut outputs = Vec::new();
            for frame in &sampled_frames {
                let cropped = frame.crop_imm(group_left, 0, group_width, full_height);
                outputs.push(frame_to_outputs(&cropped, signals.clone())?);
            }
            outputs
        };

        let group_offset_x = group_index * max_columns_per_group;
        let first_connection_entity = if use_grayscale { next_entity } else { next_entity + 1 };
        let (group_combinators, mut group_comb_wires, new_next_entity) = generate_frame_combinators(
            &group_frames_outputs,
            &substation_occupied_y,
            ticks_per_frame * frames_per_combinator,
            next_entity,
            group_offset_x as f64 + 0.5,
            if grayscale_bits == 1 || grayscale_bits == 4 { -5.0 } else if grayscale_bits == 8 { -4.0 } else { -3.0 },
            max_rows_per_group,
            grayscale_bits,
        );
        if group_index == 0 {
            group_comb_wires.push([3, 4, first_connection_entity, 2]);
        }
        next_entity = new_next_entity;

        let (group_lamps, mut group_lamp_wires, new_next_entity, top_right_lamp) = generate_lamps(
            signals.clone(),
            group_width,
            full_height,
            &occupied_cells,
            next_entity,
            group_offset_x as i32,
            0,
            use_grayscale,
        );
        next_entity = new_next_entity;

        let first_lamp_entity = group_lamps[0].entity_number;
        if use_grayscale {
            group_comb_wires.push([first_lamp_entity, 2, first_connection_entity, 2]);
            let last_shifter = if grayscale_bits == 1 || grayscale_bits == 4 { first_connection_entity + 2 } else { first_connection_entity + 1 };
            group_comb_wires.push([first_lamp_entity, 1, last_shifter, 3]);
        } else {
            group_comb_wires.push([first_lamp_entity, 1, first_connection_entity, 3]);
            group_comb_wires.push([first_lamp_entity, 2, first_connection_entity, 2]);
        }

        if let Some(prev) = previous_top_right_lamp {
            group_lamp_wires.push([group_lamps[0].entity_number, 2, prev, 2]);
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

    let blueprint = Blueprint {
        blueprint: BlueprintInner {
            icons: vec![Icon {
                signal: Signal { type_: Arc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Arc::new(DECIDER_COMBINATOR.to_string()), quality: None, },
                index: 1,
            }],
            entities: all_entities,
            wires: all_wires,
            item: BLUEPRINT,
            version: BLUEPRINT_VERSION,
        },
    };

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
/// A vector of CombinatorOutputs for the frame
pub fn frame_to_outputs(
    frame: &image::DynamicImage,
    signals: Vec<Arc<Signal>>,
) -> Result<Vec<CombinatorOutput>, JsValue> {
    let (width, height) = frame.dimensions();
    let num_pixels = (width * height) as usize;
    if num_pixels > signals.len() {
        return Err(JsValue::from_str(&format!(
            "Frame pixel count ({}) exceeds available signals ({}).",
            num_pixels,
            signals.len()
        )));
    }
    let rgb_image = frame.to_rgb8();
    let pixels = rgb_image.into_raw();
    let mut outputs = Vec::with_capacity(num_pixels);
    for (i, chunk) in pixels.chunks(3).enumerate() {
        if chunk.len() < 3 {
            continue;
        }
        let value = rgb_to_int(chunk[0], chunk[1], chunk[2]) as i32;
        let signal = Arc::clone(&signals[i]);
        outputs.push(CombinatorOutput {
            copy_count_from_input: false,
            constant: Some(value),
            signal
        });
    }
    Ok(outputs)
}

/// Packs grayscale frames into output signals by bit-packing pixel values.
///
/// # Arguments
///
/// * `frames` - A slice of grayscale image frames.
/// * `signals` - The signals to map to each pixel.
/// * `grayscale_bits` - Number of bits for grayscale conversion.
///
/// # Returns
///
/// A vector of JSON objects representing output filters.
pub fn pack_grayscale_frames_to_outputs(
    frames: &[image::DynamicImage],
    signals: Vec<Arc<Signal>>,
    grayscale_bits: u32,
) -> Result<Vec<CombinatorOutput>, JsValue> {
    if frames.is_empty() {
        return Err(JsValue::from_str("No frames provided for packing"));
    }
    let (width, height) = frames[0].dimensions();
    let num_pixels = (width * height) as usize;
    if num_pixels > signals.len() {
        return Err(JsValue::from_str(&format!(
            "Frame pixel count ({}) exceeds available signals ({}).",
            num_pixels,
            signals.len()
        )));
    }
    let luma_images: Vec<_> = frames.iter().map(|frame| frame.to_luma8()).collect();
    let mut outputs = Vec::with_capacity(num_pixels);
    for i in 0..num_pixels {
        let mut packed_value = 0;
        for (j, img) in luma_images.iter().enumerate() {
            let pixel_value = img.as_raw()[i];
            if grayscale_bits == 1 {
                let binary_value = if pixel_value >= GRAYSCALE_THRESHOLD {
                    1
                } else {
                    0
                };
                packed_value |= (binary_value as u32) << j;
            } else if grayscale_bits == 4 {
                let four_bit = pixel_value >> 4;
                packed_value |= (four_bit as u32) << (4 * j);
            } else if grayscale_bits == 8 {
                packed_value |= (pixel_value as u32) << (8 * j);
            }
        }
        let signal = Arc::clone(&signals[i]);
        outputs.push(CombinatorOutput {
            copy_count_from_input: false,
            constant: Some(packed_value as i32),
            signal,
        });
    }
    Ok(outputs)
}
