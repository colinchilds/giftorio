use crate::constants::*;
use crate::image_processing::{build_palette, nearest_palette_index, rgb_to_int};
use crate::models::*;
use crate::progress::report_progress;
use crate::signals::get_signals_with_quality;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::{DynamicImage, GenericImageView};
use std::collections::{HashMap, HashSet};
use base64::write::EncoderWriter;
use std::io::{BufWriter, Write};
use std::iter::once;
use std::ops::Range;
use std::rc::Rc;
use wasm_bindgen::JsValue;

/// Encodes the blueprint as a Factorio blueprint string or as plain JSON, writing it to `out`.
///
/// # Arguments
///
/// * `blueprint` - The blueprint to encode.
/// * `json` - Whether to write the uncompressed JSON instead of a blueprint string.
/// * `out` - Where the output is written.
///
/// # Returns
///
/// The writer, after everything has been written and flushed.
pub fn encode_blueprint<W: Write>(blueprint: &Blueprint, json: bool, mut out: W) -> Result<W, JsValue> {
    if json {
        report_progress(80, "Writing blueprint JSON...");
        serde_json::to_writer(&mut out, blueprint)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
        out.flush()
            .map_err(|e| JsValue::from_str(&format!("Output error: {}", e)))?;
        report_progress(100, "Blueprint generation complete. Loading to browser...");
        return Ok(out);
    }
    report_progress(80, "Encoding and compressing blueprint...");
    // Factorio blueprint strings are a version byte ('0') followed by the base64 data.
    out.write_all(b"0")
        .map_err(|e| JsValue::from_str(&format!("Output error: {}", e)))?;
    let base64_writer = EncoderWriter::new(out, base64::STANDARD);
    let encoder = ZlibEncoder::new(base64_writer, Compression::best());
    // Without buffering, serde's many tiny writes make zlib extremely slow.
    let mut writer = BufWriter::with_capacity(64 * 1024, encoder);
    serde_json::to_writer(&mut writer, blueprint)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
    // Not flush(): it triggers a zlib sync flush, which changes the output bytes.
    let encoder = writer
        .into_inner()
        .map_err(|e| JsValue::from_str(&format!("Compression error: {}", e)))?;
    let mut base64_writer = encoder
        .finish()
        .map_err(|e| JsValue::from_str(&format!("Compression finish error: {}", e)))?;
    let mut out = base64_writer
        .finish()
        .map_err(|e| JsValue::from_str(&format!("Base64 finish error: {}", e)))?;
    out.flush()
        .map_err(|e| JsValue::from_str(&format!("Output error: {}", e)))?;
    report_progress(100, "Blueprint generation complete. Loading to browser...");
    Ok(out)
}

/// Generates timer entities and wires for the blueprint's timing mechanism.
///
/// # Arguments
///
/// * `stop` - Maximum tick value before reset.
/// * `bits` - Bits per frame in packed modes (adds extra timer entities if > 0).
/// * `frames_per_int` - Number of frames packed into each pixel value.
/// * `ticks_per_block` - Ticks each block of `frames_per_int` frames lasts.
///
/// # Returns
///
/// A tuple with timer entities and wires.
pub fn generate_timer(
    stop: u32,
    bits: u32,
    frames_per_int: u32,
    ticks_per_block: u32,
) -> (Vec<Entity>, Vec<Wire>) {
    let mut entities = Vec::new();
    let mut wires = Vec::new();

    let mut filters = vec![Filter {
        index: 1,
        type_: SIGNAL_TYPE_VIRTUAL,
        name: SIGNAL_T,
        quality: Some(QUALITY_NORMAL),
        comparator: Some(COMPARATOR_EQUAL),
        count: Some(1),
    }];
    if bits > 0 {
        // The timer loop carries S alongside T, so S is always T * frames_per_int.
        filters.push(Filter {
            index: 2,
            type_: SIGNAL_TYPE_VIRTUAL,
            name: SIGNAL_S,
            quality: Some(QUALITY_NORMAL),
            comparator: Some(COMPARATOR_EQUAL),
            count: Some(frames_per_int),
        });
    }
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
                sections: vec![Section { index: 1, filters }],
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
                    first_signal: virtual_signal(SIGNAL_T),
                    constant: stop as i32,
                    comparator: COMPARATOR_LESS,
                    compare_type: None,
                }],
                // Everything, not Each: a decider can only output Each when its condition uses Each.
                outputs: DeciderOutputs::List(vec![CombinatorOutput {
                    copy_count_from_input: true,
                    constant: None,
                    signal: Rc::new(virtual_signal(SIGNAL_EVERYTHING)),
                }]),
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
                first_signal: virtual_signal(SIGNAL_T),
                second_signal: None,
                second_constant: Some(1),
                operation: OPERATION_SUB,
                output_signal: virtual_signal(SIGNAL_T),
            },
        }),
    );

    wires.push([1, 2, 2, 2]);
    wires.push([2, 2, 2, 4]);
    wires.push([2, 2, 3, 2]);

    if bits > 0 {
        // F = (T % ticks_per_block) * frames_per_int / ticks_per_block, computed from S as
        // (S % (frames_per_int * ticks_per_block)) / ticks_per_block. Starting from S saves a
        // multiply, which keeps F in step with the frame deciders' outputs.
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
                    first_signal: virtual_signal(SIGNAL_S),
                    second_signal: None,
                    second_constant: Some((frames_per_int * ticks_per_block) as i32),
                    operation: OPERATION_MOD,
                    output_signal: virtual_signal(SIGNAL_S),
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
                    first_signal: virtual_signal(SIGNAL_S),
                    second_signal: None,
                    second_constant: Some(ticks_per_block as i32),
                    operation: OPERATION_DIV,
                    output_signal: virtual_signal(SIGNAL_F),
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
                    first_signal: virtual_signal(SIGNAL_EACH),
                    second_signal: None,
                    second_constant: Some(bits as i32),
                    operation: OPERATION_MUL,
                    output_signal: virtual_signal(SIGNAL_EACH),
                },
            })
            .with_description("Calculates the bit shift necessary for the frame we should be rendering."),
        );

        wires.push([3, 2, 4, 2]);
        wires.push([4, 4, 5, 2]);
        wires.push([5, 4, 6, 2]);
        wires.push([6, 4, 3, 4]);

        if stop % ticks_per_block != 0 {
            // F is computed from the timer one tick earlier than the frame deciders read T, so when the
            // deciders see T wrap to 0, F is still based on T = stop. Zeroing S on that tick keeps F at
            // 0; otherwise the first block would show the wrong frame for a tick.
            entities.push(
                Entity::new(
                    7,
                    DECIDER_COMBINATOR,
                    Position {
                        x: TIMER_ENTITY7_POSITION.0,
                        y: TIMER_ENTITY7_POSITION.1,
                    },
                )
                .with_direction(DIRECTION_RIGHT)
                .with_control_behavior(ControlBehavior::Decider {
                    decider_conditions: DeciderConditions {
                        conditions: vec![Condition {
                            first_signal: virtual_signal(SIGNAL_T),
                            constant: stop as i32 - 1,
                            comparator: COMPARATOR_EQUAL,
                            compare_type: None,
                        }],
                        outputs: DeciderOutputs::List(vec![CombinatorOutput {
                            copy_count_from_input: false,
                            constant: Some(-((frames_per_int * stop) as i32)),
                            signal: Rc::new(virtual_signal(SIGNAL_S)),
                        }]),
                    },
                }),
            );
            wires.push([2, 2, 7, 2]);
            wires.push([7, 2, 7, 4]);
        }
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
/// * `combinator_rows` - Rows of combinators above the lamps (affects vertical coverage).
/// * `start_entity_number` - Starting entity number.
///
/// # Returns
///
/// A tuple with substation entities, their wires, occupied grid cells, and the next entity number.
pub fn generate_substations(
    substation_quality: String,
    lamp_width: u32,
    lamp_height: u32,
    combinator_rows: u32,
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
    let mut frame_coverage_count = (((combinator_rows as f64) - half_coverage) / (coverage as f64)).ceil() as u32;
    while ((combinator_rows as f64) - half_coverage + (frame_coverage_count as f64 * 2.0))
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

/// How pixel values are stored in the frame deciders and turned into lamp colors.
pub enum ColorMode {
    /// One packed 24-bit RGB value per pixel and frame.
    FullColor,
    /// Evenly spaced gray levels, `bits` per frame, multiplied by `multiplier` for the lamps.
    Grayscale { bits: u32, multiplier: u32 },
    /// Indices into one palette shared by all frames, `bits` per frame, decoded by one lookup
    /// decider per color. Index 0 is black.
    Palette { bits: u32, colors: Vec<[u8; 3]> },
}

impl ColorMode {
    /// Picks the color mode for the given settings.
    ///
    /// # Arguments
    ///
    /// * `palette_size` - Number of colors or gray levels (0 means full color, or 256 gray levels).
    /// * `grayscale` - Whether to use evenly spaced gray levels instead of an adaptive palette.
    /// * `frames` - The processed frames, used to build an adaptive palette.
    ///
    /// # Returns
    ///
    /// The color mode, or a JavaScript error for an unsupported palette size.
    pub fn new(palette_size: u32, grayscale: bool, frames: &[DynamicImage]) -> Result<Self, JsValue> {
        if palette_size == 0 {
            return Ok(if grayscale { ColorMode::Grayscale { bits: 8, multiplier: 1 } } else { ColorMode::FullColor });
        }
        let max_size = if grayscale { 256 } else { MAX_COLOR_PALETTE_SIZE };
        if palette_size < 2 || palette_size > max_size || !palette_size.is_power_of_two() {
            return Err(JsValue::from_str(&format!("Unsupported palette size: {}", palette_size)));
        }
        let bits = palette_size.trailing_zeros();
        if grayscale {
            // Exact for 2, 4, 16 and 256 levels; 32 and 64 levels top out at 248 and 252.
            Ok(ColorMode::Grayscale { bits, multiplier: 255 / (palette_size - 1) })
        } else {
            report_progress(15, "Building color palette");
            Ok(ColorMode::Palette { bits, colors: build_palette(frames, palette_size as usize) })
        }
    }

    /// Bits per frame in each pixel value, or 0 when each value holds a single full-color frame.
    fn bits(&self) -> u32 {
        match self {
            ColorMode::FullColor => 0,
            ColorMode::Grayscale { bits, .. } | ColorMode::Palette { bits, .. } => *bits,
        }
    }

    /// Number of frames packed into each pixel value.
    fn frames_per_int(&self) -> u32 {
        match self.bits() {
            0 => 1,
            bits => 32 / bits,
        }
    }

    /// Short description of the colors, for the blueprint description.
    fn describe(&self) -> String {
        match self {
            ColorMode::FullColor => "Full color".to_string(),
            ColorMode::Grayscale { bits: 1, .. } => "Black and white".to_string(),
            ColorMode::Grayscale { bits, .. } => format!("{} shades of gray", 1 << bits),
            ColorMode::Palette { colors, .. } => format!("{} colors", colors.len()),
        }
    }

    /// Converts a frame to one value per pixel: packed RGB, a gray level, or a palette index.
    fn pixel_values(&self, frame: &DynamicImage) -> Vec<u32> {
        self.frame_bytes(frame)
            .chunks_exact(self.channels())
            .map(|pixel| self.pixel_value(pixel))
            .collect()
    }

    /// A frame's raw pixels, `channels()` bytes each: luma in grayscale, RGB otherwise.
    fn frame_bytes(&self, frame: &DynamicImage) -> Vec<u8> {
        match self {
            ColorMode::Grayscale { .. } => frame.to_luma8().into_raw(),
            _ => frame.to_rgb8().into_raw(),
        }
    }

    fn channels(&self) -> usize {
        match self {
            ColorMode::Grayscale { .. } => 1,
            _ => 3,
        }
    }

    /// Converts one pixel's raw bytes to its value.
    fn pixel_value(&self, pixel: &[u8]) -> u32 {
        match self {
            ColorMode::FullColor => rgb_to_int(pixel[0], pixel[1], pixel[2]),
            ColorMode::Grayscale { bits, multiplier } => {
                ((pixel[0] as u32 + multiplier / 2) / multiplier).min((1 << bits) - 1)
            }
            ColorMode::Palette { colors, .. } => nearest_palette_index(colors, pixel),
        }
    }

    /// Number of `lamp_drivers`.
    fn lamp_driver_count(&self) -> usize {
        match self {
            ColorMode::FullColor | ColorMode::Grayscale { multiplier: 1, .. } => 0,
            ColorMode::Grayscale { .. } => 1,
            ColorMode::Palette { colors, .. } => colors.len() - 1,
        }
    }

    /// The combinators that turn decoded values into lamp colors: one lookup decider per non-black
    /// palette color, or a multiplier for gray levels that don't already span 0-255.
    fn lamp_drivers(&self) -> Vec<(&'static str, ControlBehavior)> {
        match self {
            ColorMode::FullColor | ColorMode::Grayscale { multiplier: 1, .. } => Vec::new(),
            ColorMode::Grayscale { multiplier, .. } => vec![(
                ARITHMETIC_COMBINATOR,
                ControlBehavior::Arithmetic {
                    arithmetic_conditions: ArithmeticConditions {
                        first_signal: virtual_signal(SIGNAL_EACH),
                        second_signal: None,
                        second_constant: Some(*multiplier as i32),
                        operation: OPERATION_MUL,
                        output_signal: virtual_signal(SIGNAL_EACH),
                    },
                },
            )],
            ColorMode::Palette { colors, .. } => colors
                .iter()
                .enumerate()
                .skip(1)
                .map(|(index, color)| {
                    (
                        DECIDER_COMBINATOR,
                        ControlBehavior::Decider {
                            decider_conditions: DeciderConditions {
                                conditions: vec![Condition {
                                    first_signal: virtual_signal(SIGNAL_EACH),
                                    constant: index as i32,
                                    comparator: COMPARATOR_EQUAL,
                                    compare_type: None,
                                }],
                                outputs: DeciderOutputs::List(vec![CombinatorOutput {
                                    copy_count_from_input: false,
                                    constant: Some(rgb_to_int(color[0], color[1], color[2]) as i32),
                                    signal: Rc::new(virtual_signal(SIGNAL_EACH)),
                                }]),
                            },
                        },
                    )
                })
                .collect(),
        }
    }

    /// Rows of combinator slots the decoder takes up in a group with `columns` slots per row.
    fn decoder_rows(&self, columns: usize) -> usize {
        match self {
            ColorMode::FullColor => 0,
            _ => {
                let driver_rows = self.lamp_driver_count().div_ceil(columns);
                // The shifter and AND share a row, or stack when there's only one column.
                driver_rows + if columns >= 2 { 1 } else { 2 }
            }
        }
    }
}

fn virtual_signal(name: &str) -> Signal {
    Signal { type_: Rc::new(SIGNAL_TYPE_VIRTUAL.to_string()), name: Rc::new(name.to_string()), quality: None }
}

/// Rounds `numerator / denominator` to the nearest integer.
fn round_div(numerator: u32, denominator: u32) -> u32 {
    ((2 * numerator as u64 + denominator as u64) / (2 * denominator as u64)) as u32
}

/// Formats a number of ticks as seconds under a minute ("8.5s"), or minutes and seconds ("2:05").
fn format_ticks(ticks: u32) -> String {
    let seconds = ticks as f64 / TICKS_PER_SECOND as f64;
    if seconds < 60.0 {
        format!("{:.1}s", seconds)
    } else {
        let whole = seconds.round() as u32;
        format!("{}:{:02}", whole / 60, whole % 60)
    }
}

/// A decider outputting one chunk of a group's pixels for blocks `start_block..end_block`.
struct FrameDecider {
    start_block: usize,
    end_block: usize,
    values: Vec<i32>,
}

/// Adds a block's values for one chunk, extending the last decider when the values are unchanged.
/// All-zero values output nothing, so they get no decider.
fn add_block(deciders: &mut Vec<FrameDecider>, block: usize, values: &[i32]) {
    if let Some(last) = deciders.last_mut() {
        if last.end_block == block && last.values == values {
            last.end_block += 1;
            return;
        }
    }
    if values.iter().any(|&v| v != 0) {
        deciders.push(FrameDecider { start_block: block, end_block: block + 1, values: values.to_vec() });
    }
}

/// Finds the pixels whose value is the same in every frame.
fn find_static_pixels(mode: &ColorMode, frames: &[DynamicImage]) -> Vec<bool> {
    let first = mode.pixel_values(&frames[0]);
    let channels = mode.channels();
    let mut candidates: Vec<usize> = (0..first.len()).collect();
    for frame in &frames[1..] {
        if candidates.is_empty() {
            break;
        }
        let bytes = mode.frame_bytes(frame);
        candidates.retain(|&pixel| mode.pixel_value(&bytes[pixel * channels..][..channels]) == first[pixel]);
    }
    let mut is_static = vec![false; first.len()];
    for pixel in candidates {
        is_static[pixel] = true;
    }
    is_static
}

/// Which signal each of a group's pixels uses, and how the signals are split into deciders.
struct GroupLayout {
    /// `pixel_order[i]` is the group pixel (row-major) that uses signal `i`. Pixels that change come
    /// first and pixels that never change last, so the static ones share deciders that stay on for
    /// the whole loop instead of repeating in every block.
    pixel_order: Vec<usize>,
    /// Signal ranges of the deciders for each block.
    chunks: Vec<Range<usize>>,
}

impl GroupLayout {
    fn new(
        is_static: &[bool],
        full_width: u32,
        (group_left, group_width): (u32, u32),
        height: u32,
        split_frames: bool,
    ) -> Self {
        let (dynamic, fixed): (Vec<usize>, Vec<usize>) = (0..(group_width * height) as usize)
            .partition(|&pixel| {
                let (row, column) = (pixel as u32 / group_width, pixel as u32 % group_width);
                !is_static[(row * full_width + group_left + column) as usize]
            });
        let mut chunks = Vec::new();
        let mut start = 0;
        for count in [dynamic.len(), fixed.len()] {
            let end = start + count;
            if count > 0 {
                let size = chunk_size(count, split_frames);
                chunks.extend((start..end).step_by(size).map(|chunk_start| chunk_start..(chunk_start + size).min(end)));
            }
            start = end;
        }
        GroupLayout { pixel_order: dynamic.into_iter().chain(fixed).collect(), chunks }
    }
}

/// Splits `count` pixels into the fewest chunks of at most `MAX_SIGNALS_PER_DECIDER`, sized evenly,
/// or keeps them in one chunk when `split_frames` is off.
fn chunk_size(count: usize, split_frames: bool) -> usize {
    if !split_frames {
        return count;
    }
    count.div_ceil(count.div_ceil(MAX_SIGNALS_PER_DECIDER))
}

/// Rows of 2-tile-wide combinator slots above a group's lamps.
struct Grid<'a> {
    left_x: f64,
    columns: usize,
    /// Y of each row, skipping rows taken by substations.
    row_ys: &'a [f64],
}

impl Grid<'_> {
    fn position(&self, row: usize, column: usize) -> Position {
        Position { x: self.left_x + 2.0 * column as f64, y: self.row_ys[row] }
    }
}

/// Wires combinators laid out row-major in a grid onto shared networks: each one in the first row
/// to its left neighbor, and every other one to the combinator below it.
fn comb_wires(ids: &[u32], columns: usize, connectors: &[u32], wires: &mut Vec<Wire>) {
    for slot in 1..ids.len() {
        let other = if slot >= columns { slot - columns } else { slot - 1 };
        for &connector in connectors {
            wires.push([ids[other], connector, ids[slot], connector]);
        }
    }
}

/// Generates a group's decoder and frame deciders.
///
/// The decoder sits in the rows nearest the lamps: the lamp drivers first, then the shifter and
/// AND in the row just below the frame deciders. In full color the frame deciders drive the lamps directly.
///
/// # Arguments
///
/// * `mode` - How pixel values are encoded.
/// * `frame_deciders` - The group's frame deciders, one list per chunk of pixels.
/// * `chunk_signals` - The signals of each chunk.
/// * `block_bounds` - Block `j` is active for ticks `block_bounds[j]..block_bounds[j + 1]`.
/// * `grid` - Where the group's combinators go.
/// * `start_entity_number` - Starting entity number.
///
/// # Returns
///
/// A tuple with the entities, their wires, the next entity number, and the connectors the group's
/// first lamp is wired to for pixel data (red) and the timer (green).
fn generate_group_combinators(
    mode: &ColorMode,
    frame_deciders: Vec<Vec<FrameDecider>>,
    chunk_signals: &[Rc<[Rc<Signal>]>],
    block_bounds: &[u32],
    grid: &Grid,
    start_entity_number: u32,
) -> (Vec<Entity>, Vec<Wire>, u32, Option<(u32, u32)>, Option<(u32, u32)>) {
    let columns = grid.columns;
    let mut entities = Vec::new();
    let mut wires = Vec::new();
    let mut next_entity = start_entity_number;
    let mut lamp_red = None;
    let mut lamp_green = None;
    let mut shifter = None;

    if !matches!(mode, ColorMode::FullColor) {
        let drivers = mode.lamp_drivers();
        let driver_rows = drivers.len().div_ceil(columns);
        let mut driver_ids = Vec::with_capacity(drivers.len());
        for (slot, (name, behavior)) in drivers.into_iter().enumerate() {
            entities.push(
                Entity::new(next_entity, name, grid.position(slot / columns, slot % columns))
                    .with_direction(DIRECTION_RIGHT)
                    .with_control_behavior(behavior),
            );
            driver_ids.push(next_entity);
            next_entity += 1;
        }
        comb_wires(&driver_ids, columns, &[RED_IN, GREEN_IN, RED_OUT], &mut wires);

        let (shifter_position, and_position) = if columns >= 2 {
            ((driver_rows, 0), (driver_rows, 1))
        } else {
            ((driver_rows + 1, 0), (driver_rows, 0))
        };
        let shifter_id = next_entity;
        let and_id = next_entity + 1;
        next_entity += 2;
        entities.push(
            Entity::new(shifter_id, ARITHMETIC_COMBINATOR, grid.position(shifter_position.0, shifter_position.1))
                .with_direction(DIRECTION_RIGHT)
                .with_control_behavior(ControlBehavior::Arithmetic {
                    arithmetic_conditions: ArithmeticConditions {
                        first_signal: virtual_signal(SIGNAL_EACH),
                        second_signal: Some(virtual_signal(SIGNAL_F)),
                        second_constant: None,
                        operation: OPERATION_SHIFT_R,
                        output_signal: virtual_signal(SIGNAL_EACH),
                    },
                }),
        );
        entities.push(
            Entity::new(and_id, ARITHMETIC_COMBINATOR, grid.position(and_position.0, and_position.1))
                .with_direction(DIRECTION_RIGHT)
                .with_control_behavior(ControlBehavior::Arithmetic {
                    arithmetic_conditions: ArithmeticConditions {
                        first_signal: virtual_signal(SIGNAL_EACH),
                        second_signal: None,
                        second_constant: Some(((1u32 << mode.bits()) - 1) as i32),
                        operation: OPERATION_AND,
                        output_signal: virtual_signal(SIGNAL_EACH),
                    },
                }),
        );
        wires.push([shifter_id, RED_OUT, and_id, RED_IN]);

        if driver_ids.is_empty() {
            lamp_red = Some((and_id, RED_OUT));
            lamp_green = Some((shifter_id, GREEN_IN));
        } else {
            // The driver below the AND, or the last driver if the top driver row doesn't reach that column.
            let below_and = ((driver_rows - 1) * columns + and_position.1).min(driver_ids.len() - 1);
            wires.push([and_id, RED_OUT, driver_ids[below_and], RED_IN]);
            // The drivers' green inputs carry the timer from the lamps up to the shifter, so it
            // stays in wire reach however tall the decoder is. They only ever see T and F on it.
            wires.push([shifter_id, GREEN_IN, driver_ids[(driver_rows - 1) * columns], GREEN_IN]);
            lamp_red = Some((driver_ids[0], RED_OUT));
            lamp_green = Some((driver_ids[0], GREEN_IN));
        }
        shifter = Some(shifter_id);
    }

    // Chunk-major, so chunk k of each block directly follows chunk k of the block before it and
    // stays inside zlib's window.
    let first_row = mode.decoder_rows(columns);
    let mut frame_ids = Vec::new();
    for (deciders, signals) in frame_deciders.into_iter().zip(chunk_signals) {
        for decider in deciders {
            let lower_bound = block_bounds[decider.start_block] as i32;
            let upper_bound = block_bounds[decider.end_block] as i32;
            // Above 60fps, some frames round to zero ticks and are never shown.
            if lower_bound == upper_bound {
                continue;
            }
            let slot = frame_ids.len();
            entities.push(
                Entity::new(
                    next_entity,
                    DECIDER_COMBINATOR,
                    grid.position(first_row + slot / columns, slot % columns),
                )
                .with_direction(DIRECTION_RIGHT)
                .with_control_behavior(ControlBehavior::Decider {
                    decider_conditions: DeciderConditions {
                        conditions: vec![
                            Condition {
                                first_signal: virtual_signal(SIGNAL_T),
                                constant: lower_bound,
                                comparator: COMPARATOR_GREATER_EQUAL,
                                compare_type: None,
                            },
                            Condition {
                                first_signal: virtual_signal(SIGNAL_T),
                                constant: upper_bound,
                                comparator: COMPARATOR_LESS,
                                compare_type: Some(COMPARE_AND),
                            },
                        ],
                        outputs: DeciderOutputs::Pixels {
                            signals: Rc::clone(signals),
                            values: decider.values,
                        },
                    },
                }),
            );
            frame_ids.push(next_entity);
            next_entity += 1;
        }
    }
    comb_wires(&frame_ids, columns, &[GREEN_IN, RED_OUT], &mut wires);

    if let Some(&first) = frame_ids.first() {
        match shifter {
            Some(shifter_id) => {
                wires.push([first, RED_OUT, shifter_id, RED_IN]);
                wires.push([first, GREEN_IN, shifter_id, GREEN_IN]);
            }
            None => {
                lamp_red = Some((first, RED_OUT));
                lamp_green = Some((first, GREEN_IN));
            }
        }
    }
    (entities, wires, next_entity, lamp_red, lamp_green)
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
    signals: &[Rc<Signal>],
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
            let signal = Rc::clone(&signals[index]);
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
/// * `palette_size` - Number of colors or gray levels (0 means full color, or 256 gray levels).
/// * `grayscale` - Whether to use evenly spaced gray levels instead of an adaptive palette.
/// * `split_frames` - Whether to split each frame into deciders of at most `MAX_SIGNALS_PER_DECIDER`
///   pixels. That makes the blueprint string much smaller, but takes several times more deciders.
/// * `substation_quality` - Quality level for substations.
/// * `name` - Name for the blueprint's label, usually the GIF's file name.
///
/// # Returns
///
/// The final blueprint as a JSON value.
pub fn update_full_blueprint(
    name: &str,
    fps: u32,
    sampled_frames: Vec<DynamicImage>,
    use_dlc: bool,
    palette_size: u32,
    grayscale: bool,
    split_frames: bool,
    substation_quality: String,
) -> Result<Blueprint, JsValue> {
    report_progress(0, "Starting blueprint update");

    // Get signals internally.
    let signals = get_signals_with_quality(use_dlc);

    if sampled_frames.is_empty() {
        return Err(JsValue::from_str("No sampled frames"));
    }

    let mode = ColorMode::new(palette_size, grayscale, &sampled_frames)?;
    let bits = mode.bits();
    let frames_per_int = mode.frames_per_int();
    let total_frames = sampled_frames.len() as u32;
    let (full_width, full_height) = sampled_frames[0].dimensions();
    let max_columns_per_group = ((signals.len() as u32) / full_height).min(full_width);
    if max_columns_per_group < 1 {
        return Err(JsValue::from_str("Not enough signals for even one column of lamps!"));
    }
    let num_groups = full_width.div_ceil(max_columns_per_group);
    let columns_per_group = full_width.div_ceil(num_groups);
    let groups: Vec<(u32, u32)> = (0..num_groups)
        .map(|group_index| {
            let group_left = group_index * columns_per_group;
            let group_right = ((group_index + 1) * columns_per_group).min(full_width);
            (group_left, group_right - group_left)
        })
        .collect();

    // Block j (one frame in full color, frames_per_int frames otherwise) plays for ticks
    // block_bounds[j]..block_bounds[j + 1], and the last bound is where the timer resets.
    let num_blocks = total_frames.div_ceil(frames_per_int);
    let ticks_per_block = round_div(frames_per_int * TICKS_PER_SECOND, fps).max(1);
    let block_bounds: Vec<u32> = if bits == 0 {
        (0..=total_frames).map(|i| round_div(i * TICKS_PER_SECOND, fps)).collect()
    } else {
        // Frames within a block advance at ticks_per_block / frames_per_int, so the last,
        // shorter block ends once its own frames have played.
        let last_block_frames = total_frames - (num_blocks - 1) * frames_per_int;
        let stop = (num_blocks - 1) * ticks_per_block
            + round_div(last_block_frames * ticks_per_block, frames_per_int);
        (0..num_blocks).map(|j| j * ticks_per_block).chain(once(stop)).collect()
    };
    let stop = *block_bounds.last().unwrap();

    report_progress(18, "Finding static pixels");
    let is_static = find_static_pixels(&mode, &sampled_frames);
    let layouts: Vec<GroupLayout> = groups
        .iter()
        .map(|&group| GroupLayout::new(&is_static, full_width, group, full_height, split_frames))
        .collect();

    // Consume the frames as they're converted so each full frame is freed once its values exist.
    report_progress(20, "Converting frames");
    let mut deciders_by_group: Vec<Vec<Vec<FrameDecider>>> = layouts
        .iter()
        .map(|layout| layout.chunks.iter().map(|_| Vec::new()).collect())
        .collect();
    let mut frames = sampled_frames.into_iter();
    for block in 0..num_blocks as usize {
        let mut frame_values: Vec<Vec<u32>> = frames
            .by_ref()
            .take(frames_per_int as usize)
            .map(|frame| mode.pixel_values(&frame))
            .collect();
        // Pad a short last block with its final frame, so static pixels pack to the same value in
        // every block. The padding is never shown.
        if frame_values.len() < frames_per_int as usize {
            let last_frame = frame_values.last().unwrap().clone();
            frame_values.resize(frames_per_int as usize, last_frame);
        }
        for (group_index, &(group_left, group_width)) in groups.iter().enumerate() {
            let values = pack_group_values(&frame_values, full_width, group_left, group_width, full_height, bits);
            let layout = &layouts[group_index];
            let ordered: Vec<i32> = layout.pixel_order.iter().map(|&pixel| values[pixel]).collect();
            for (chunk, deciders) in layout.chunks.iter().zip(deciders_by_group[group_index].iter_mut()) {
                add_block(deciders, block, &ordered[chunk.clone()]);
            }
        }
    }

    let group_columns: Vec<usize> = groups
        .iter()
        .map(|&(_, group_width)| ((group_width / 2) as usize).max(1))
        .collect();
    let total_rows = deciders_by_group
        .iter()
        .zip(&group_columns)
        .map(|(chunks, &columns)| {
            let count: usize = chunks.iter().map(Vec::len).sum();
            mode.decoder_rows(columns) + count.div_ceil(columns)
        })
        .max()
        .unwrap_or(0);

    let (timer_entities, timer_wires) = generate_timer(stop, bits, frames_per_int, ticks_per_block);

    let mut all_entities = timer_entities;
    let mut all_wires: Vec<Wire> = timer_wires;
    let mut next_entity = all_entities
        .iter()
        .map(|e| e.entity_number)
        .max()
        .unwrap_or(0)
        + 1;
    report_progress(60, "Generating power grid");
    let (substation_entities, substation_wires, occupied_cells, next_entity_new) =
        generate_substations(
            substation_quality,
            full_width,
            full_height,
            total_rows as u32,
            next_entity,
        );

    next_entity = next_entity_new;
    all_entities.extend(substation_entities);
    all_wires.extend(substation_wires);
    let substation_occupied_y: HashSet<i32> = occupied_cells.iter().map(|(_, y)| *y).collect();
    let row_ys: Vec<f64> = (0..)
        .map(|row| FIRST_COMBINATOR_ROW_Y - row)
        .filter(|y| !substation_occupied_y.contains(y))
        .take(total_rows)
        .map(|y| y as f64)
        .collect();
    let mut previous_top_right_lamp: Option<u32> = None;

    for (group_index, (group_deciders, &(group_left, group_width))) in deciders_by_group.into_iter().zip(&groups).enumerate() {
        let layout = &layouts[group_index];
        let group_signals = &signals[..(group_width * full_height) as usize];
        let chunk_signals: Vec<Rc<[Rc<Signal>]>> = layout
            .chunks
            .iter()
            .map(|chunk| Rc::from(&group_signals[chunk.clone()]))
            .collect();
        let mut lamp_signals = group_signals.to_vec();
        for (signal, &pixel) in layout.pixel_order.iter().enumerate() {
            lamp_signals[pixel] = Rc::clone(&group_signals[signal]);
        }
        let grid = Grid {
            left_x: group_left as f64 + 0.5,
            columns: group_columns[group_index],
            row_ys: &row_ys,
        };
        let (group_combinators, mut group_comb_wires, new_next_entity, lamp_red, lamp_green) =
            generate_group_combinators(&mode, group_deciders, &chunk_signals, &block_bounds, &grid, next_entity);
        next_entity = new_next_entity;

        let (group_lamps, mut group_lamp_wires, new_next_entity, top_right_lamp) = generate_lamps(
            &lamp_signals,
            group_width,
            full_height,
            &occupied_cells,
            next_entity,
            group_left as i32,
            0,
            matches!(mode, ColorMode::Grayscale { .. }),
        );
        next_entity = new_next_entity;

        let first_lamp_entity = group_lamps[0].entity_number;
        if let Some((entity, connector)) = lamp_red {
            group_comb_wires.push([first_lamp_entity, RED_IN, entity, connector]);
        }
        if let Some((entity, connector)) = lamp_green {
            group_comb_wires.push([first_lamp_entity, GREEN_IN, entity, connector]);
        }
        if group_index == 0 {
            // The timer reaches every group over the lamps' green wire.
            group_comb_wires.push([first_lamp_entity, GREEN_IN, 3, GREEN_OUT]);
        }

        if let Some(prev) = previous_top_right_lamp {
            group_lamp_wires.push([first_lamp_entity, GREEN_IN, prev, GREEN_IN]);
        }
        previous_top_right_lamp = Some(top_right_lamp);

        all_entities.extend(group_combinators);
        all_entities.extend(group_lamps);
        all_wires.extend(group_comb_wires);
        all_wires.extend(group_lamp_wires);

        let percent = 60 + ((group_index as u32 + 1) * 20 / num_groups);
        report_progress(
            percent,
            &format!("Processed chunk {}/{}", group_index + 1, num_groups),
        );
    }

    let combinators = all_entities
        .iter()
        .filter(|e| matches!(e.name, DECIDER_COMBINATOR | ARITHMETIC_COMBINATOR | CONSTANT_COMBINATOR))
        .count();
    let name = if name.is_empty() { "GIFtorio" } else { name };
    let description = format!(
        "{}x{} lamps, {} frames at {} FPS ({} loop)\n{}, {} combinators\nMade with GIFtorio: https://giftor.io",
        full_width,
        full_height,
        total_frames,
        fps,
        format_ticks(stop),
        mode.describe(),
        combinators,
    );

    let blueprint = Blueprint {
        blueprint: BlueprintInner {
            label: format!("{} ({}x{})", name, full_width, full_height),
            description,
            icons: vec![Icon {
                signal: virtual_signal(DECIDER_COMBINATOR),
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

/// Packs frames of per-pixel values into one value per pixel of a group.
///
/// # Arguments
///
/// * `frame_values` - One value per pixel of the full image, for each frame in the block.
/// * `full_width` - Width of the full image.
/// * `group_left` - Leftmost column of the group.
/// * `group_width` - Number of columns in the group.
/// * `height` - Height of the image.
/// * `bits` - Bits per frame; frame `j` goes in bits `j * bits` and up.
///
/// # Returns
///
/// One packed value per group pixel, row-major.
fn pack_group_values(
    frame_values: &[Vec<u32>],
    full_width: u32,
    group_left: u32,
    group_width: u32,
    height: u32,
    bits: u32,
) -> Vec<i32> {
    let mut outputs = Vec::with_capacity((group_width * height) as usize);
    for row in 0..height {
        for column in group_left..group_left + group_width {
            let pixel = (row * full_width + column) as usize;
            let packed = frame_values
                .iter()
                .enumerate()
                .fold(0u32, |packed, (j, values)| packed | (values[pixel] << (bits * j as u32)));
            outputs.push(packed as i32);
        }
    }
    outputs
}
