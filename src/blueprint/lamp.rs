use crate::blueprint::{constants::*, models::*, substation::*};
use glam::{dvec2, DVec2, UVec2};
use std::sync::Arc;

/// Generates a grid of lamp entities for the blueprint.
///
/// # Arguments
///
/// * `signals` - Signals to assign to each lamp.
/// * `grid_dim` - Number of lamps horizontally and vertically.
/// * `occupied_cells` - Set of grid cells already occupied by substations.
/// * `base_ent_n` - Starting entity number for lamps.
/// * `start_pos` - Starting X and Y coordinates.
/// * `args` - Args for generating the blueprint.
///
/// # Returns
///
/// A tuple with lamp entities, lamp wires, the next entity number, and the top-right lamp entity.
pub fn generate_lamps(
    signals: &[Arc<Signal>],
    grid_dim: UVec2,
    curr_frame: Option<image::DynamicImage>,
    occupied: &mut SubstationOccupied,
    base_ent_n: u32,
    start_pos: DVec2,
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>, u32, (Option<u32>, Option<u32>)) {
    use crate::blueprint::models::ImageRotation::*;

    let mut ents = Vec::new();
    let mut wires = Vec::new();
    let mut top_left_ent_n: Option<u32> = None;
    let mut top_right_ent_n: Option<u32> = None;

    // Auto-rotate wires based on image rotation
    let prefer_horizontal_wires =
        args.prefer_horizontal_wires ^ matches!(args.image_rotation, Deg90 | Deg270);
    let base_pos = start_pos + dvec2(0.5, 0.5); // Lamps are centered on the middle of a tile
    let mode = args.mode;
    let get_ent_num = |r: u32, c: u32| -> u32 { base_ent_n + r * grid_dim.x + c };
    let mut prev_ents_n: Vec<Option<u32>> = vec![None; grid_dim.x as usize];
    let curr_frame = curr_frame.as_ref().map(|f| f.to_rgba8());

    for r in 0..grid_dim.y as usize {
        let mut prev_ent_n: Option<u32> = None; // Used for horizontal wires
        let mut did_connect_row = false; // Track whether we've connected a row yet
        let mut was_top = false;

        for c in (0..grid_dim.x as usize).rev() {
            let pos = base_pos + dvec2(c as f64, r as f64);
            if !occupied.test(pos) {
                continue;
            }
            let curr_ent_n = get_ent_num(r as u32, c as u32);
            let mut lamp = Entity::new(curr_ent_n, Arc::clone(&LAMP), pos).with_always_on(true);
            let index = r * grid_dim.x as usize + c;

            if top_right_ent_n.is_none() {
                top_right_ent_n = Some(curr_ent_n); // Handle case where there's only one column
            }

            if matches!(args.mode, Mode::Static { combs: false, .. }) {
                let frame = curr_frame.as_ref();
                let frame = frame.expect("StaticImage without combintors requires a frame");
                let pixel = frame.get_pixel(c as u32, r as u32);

                lamp = lamp.with_color(Color {
                    r: pixel[0] as f64 / 255.0,
                    g: pixel[1] as f64 / 255.0,
                    b: pixel[2] as f64 / 255.0,
                    a: pixel[3] as f64 / 255.0,
                });
            } else {
                let signal = Arc::clone(&signals[index]);
                lamp = lamp.with_control_behavior(if args.grayscale_bits > 0 {
                    ControlBehavior::GrayLamp {
                        use_colors: true,
                        color_mode: 1,
                        red_signal: Arc::clone(&signal),
                        green_signal: Arc::clone(&signal),
                        blue_signal: Arc::clone(&signal),
                    }
                } else {
                    ControlBehavior::ColorLamp {
                        use_colors: true,
                        color_mode: 2,
                        rgb_signal: signal,
                    }
                });
            }

            if !matches!(mode, Mode::Static { combs: false, .. }) {
                let is_top = prev_ents_n[c].is_none();

                // prev_ents_n tracks the entity numbers for the rows above
                // if it's none then we never connected the top row
                if !matches!(mode, Mode::LampGrid | Mode::Static { timer: false, .. }) {
                    if is_top && (r == 0 || top_left_ent_n.is_none()) {
                        if let Some(prev) = prev_ent_n {
                            wires.push([curr_ent_n, WIRE_R, prev, WIRE_R]); // horizontal timing wire
                        }
                    }
                }
                if is_top || prefer_horizontal_wires || was_top {
                    if let Some(prev) = prev_ent_n {
                        wires.push([curr_ent_n, WIRE_G, prev, WIRE_G]); // horizontal data wire
                    }
                }
                if !did_connect_row || !prefer_horizontal_wires {
                    if let Some(prev) = prev_ents_n[c] {
                        wires.push([curr_ent_n, WIRE_G, prev, WIRE_G]); // vertical data wire
                        did_connect_row = true;
                    }
                }
                was_top = is_top;
            }

            ents.push(lamp);
            prev_ent_n = Some(curr_ent_n);
            prev_ents_n[c] = prev_ent_n;
        }

        if top_left_ent_n.is_none() {
            top_left_ent_n = prev_ent_n;
        }
    }
    let next_ent_n = get_ent_num(grid_dim.y - 1, grid_dim.x - 1) + 1;
    return (ents, wires, next_ent_n, (top_left_ent_n, top_right_ent_n));
}
