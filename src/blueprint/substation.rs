use crate::blueprint::{constants::*, macros::*, models::*};
use crate::constants::FACTORIO_EPSILON;
use crate::models::ImageRotation::*;
use glam::{dvec2, DVec2};
use std::sync::Arc;

/// Tracks cells occupied by substations.
pub struct SubstationOccupied {
    pub start_pos: DVec2,
    include_substations: bool,
    quality: Option<Arc<str>>,
    requests: Vec<DVec2>,
}
impl SubstationOccupied {
    pub fn new(start_pos: DVec2, args: &BlueprintArgs) -> Self {
        let quality = args.substation_quality.clone();
        let quality = quality.filter(|q| *q != *DEFAULT_QUAL_KEY);

        Self {
            start_pos,
            requests: Vec::new(),
            include_substations: args.include_substations,
            quality,
        }
    }

    pub fn coverage(&self) -> f64 {
        const DEFAULT: i32 = 9;
        2.0 * self.quality.as_ref().map_or(DEFAULT, |q| match q.as_ref() {
            "normal" => DEFAULT,
            "uncommon" => 10,
            "rare" => 11,
            "epic" => 12,
            "legendary" => 14,
            _ => DEFAULT,
        }) as f64
    }

    /// Tests if it is possible to place an entity at the given point, without overlapping with a substation.
    ///
    /// # Returns
    ///
    /// A `bool` indicating whether the substation would NOT occupy the given point.
    pub fn test(&mut self, mut point: DVec2) -> bool {
        if !self.include_substations || self.coverage() == 0.0 {
            return true;
        }
        point -= self.start_pos;

        let center = (point / self.coverage()).round() * self.coverage();
        let delta = (point - center).abs();

        delta.x >= 1.0 || delta.y >= 1.0
    }

    /// Requests that a substation be placed to power the given point.
    ///
    /// # Returns
    ///
    /// A `bool` indicating whether the substation would NOT occupy the given point.
    pub fn request(&mut self, mut point: DVec2) -> bool {
        if self.coverage() == 0.0 {
            return true;
        }
        point -= self.start_pos;

        let center = (point / self.coverage()).round() * self.coverage();
        let delta = (point - center).abs();

        if !self.requests.contains(&center) {
            self.requests.push(center);
        }

        delta.x >= 1.0 || delta.y >= 1.0
    }
}

/// Generates substation entities and wires for powering the blueprint.
///
/// # Arguments
///
/// * `lamp_dim` - Width and height of the lamp grid.
/// * `n_frames` - Number of frames (affects vertical coverage).
/// * `base_ent_n` - Starting entity number.
/// * `args` - Args for generating the blueprint.
///
/// # Returns
///
/// A tuple with substation entities, their wires, occupied grid cells, and the next entity number.
pub fn generate_substations(
    occupied: &SubstationOccupied,
    base_ent_n: u32,
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>, u32) {
    let mut subs: Vec<Entity> = Vec::new();
    let mut wires: Vec<[u32; 4]> = Vec::new();
    let mut curr_ent_n = base_ent_n;
    let cov = occupied.coverage();

    if !occupied.include_substations || cov == 0.0 {
        return (subs, wires, base_ent_n);
    }

    // f64 doesn't implement Ord, so use total_cmp.
    // Sort by Y first, then X.
    let mut requests = occupied.requests.clone();
    requests.sort_by(|a, b| a.y.total_cmp(&b.y).then_with(|| a.x.total_cmp(&b.x)));

    let get_tag = |pos: DVec2| format!("sub-({},{})", pos.x, pos.y);
    let horiz = true ^ matches!(args.image_rotation, Deg90 | Deg270);

    // Create all substations first. This lets us determine the topology
    // from the actual generated positions without making assumptions
    // about the shape of the layout.
    for req in &requests {
        let pos = occupied.start_pos + *req;
        let en = Entity::new(curr_ent_n, Arc::clone(&SUBSTATION), pos).with_tag(&get_tag(pos));
        subs.push(en.with_quality(occupied.quality.clone()));
        curr_ent_n += 1;
    }

    // Find an actual neighbouring substation at the given offset.
    //
    // This deliberately searches by position rather than assuming that
    // the layout is rectangular or that every row has the same length.
    let find_nb = |pos: DVec2, delta: DVec2| -> Option<&Entity> {
        let target = pos + delta;
        subs.iter()
            .find(|e| e.position.abs_diff_eq(target, FACTORIO_EPSILON))
    };

    let (preferred_delta, cross_neg, cross_pos) = if horiz {
        (dvec2(-cov, 0.0), dvec2(0.0, -cov), dvec2(0.0, cov))
    } else {
        (dvec2(0.0, -cov), dvec2(-cov, 0.0), dvec2(cov, 0.0))
    };

    let find_best = |a: &&Entity, b: &&Entity| {
        let (a, b) = (a.position, b.position);
        if horiz {
            a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y))
        } else {
            a.y.total_cmp(&b.y).then_with(|| a.x.total_cmp(&b.x))
        }
    };

    let anchor = subs
        .iter()
        .filter(|s| {
            find_nb(s.position, cross_neg).is_some() && find_nb(s.position, cross_pos).is_some()
        })
        .min_by(find_best);

    let crossing_coord = anchor
        .or_else(|| {
            subs.iter()
                .filter(|s| find_nb(s.position, cross_pos).is_some())
                .min_by(find_best)
        })
        .map(|s| if horiz { s.position.x } else { s.position.y });

    for sub in &subs {
        let pos = sub.position;
        let tag = get_tag(pos);

        if let Some(prev) = find_nb(pos, preferred_delta) {
            wires.push(mkwires!(C subs; IN get_tag(prev.position) => IN tag));
        }

        if let Some(coord) = crossing_coord {
            if (coord - if horiz { pos.x } else { pos.y }).abs() <= FACTORIO_EPSILON {
                if let Some(prev) = find_nb(pos, cross_neg) {
                    wires.push(mkwires!(C subs; IN get_tag(prev.position) => IN tag));
                }
            }
        }
    }

    return (subs, wires, curr_ent_n);
}
