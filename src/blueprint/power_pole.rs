use crate::blueprint::{constants::*, models::*};
use crate::models::ImageRotation::*;
use glam::{dvec2, DVec2};
use std::collections::BTreeMap;
use std::{collections::HashMap, sync::Arc};

#[derive(serde::Deserialize)]
struct PowerPoleInfoEntry {
    bounding_box: DVec2,
    coverage: HashMap<String, f64>,
}

/// Tracks cells occupied by power poles.
pub struct PowerPoleOccupied {
    bounding_box: DVec2,
    coverage: f64,
    ref_entity: BlueprintEntity,
    requests: Vec<DVec2>,
}
impl PowerPoleOccupied {
    pub fn new(mut start_pos: DVec2, args: &BlueprintArgs) -> Self {
        let quality = args.power_pole_quality.clone();
        let quality = quality.filter(|q| *q != *DEFAULT_QUAL_KEY);

        const COVERAGE: &'static str = include_str!("powerPoleCoverage.json");

        let name = args.power_pole_name.clone();
        let name = name.unwrap_or(DEFAULT_POWER_POLE_NAME.to_owned());
        let map: HashMap<String, PowerPoleInfoEntry> = serde_json::from_str(COVERAGE).unwrap();
        let entry = &map[&name];
        let cov = entry.coverage.clone();
        let default_cov = *cov.get("_").unwrap();

        start_pos.x -= entry.bounding_box.x / 2.0;
        start_pos.y += entry.bounding_box.y / 2.0;

        Self {
            bounding_box: entry.bounding_box,
            ref_entity: BlueprintEntity::new(0, Arc::from(name), start_pos)
                .with_quality(quality.clone()),
            coverage: quality.map_or(default_cov, |q| *cov.get(&*q).unwrap_or(&default_cov))
                * args.include_power_poles as i32 as f64,
            requests: Vec::new(),
        }
    }

    /// Generates power pole entities and wires for powering the blueprint.
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
    /// A tuple with power pole entities, their wires, occupied grid cells, and the next entity number.
    pub fn generate_power_poles(
        &mut self,
        base_ent_n: u32,
        args: &BlueprintArgs,
    ) -> (Vec<BlueprintEntity>, Vec<Wire>, u32) {
        let mut poles: Vec<BlueprintEntity> = Vec::new();
        let mut wires: Vec<[u32; 4]> = Vec::new();
        let mut curr_ent_n = base_ent_n;
        let cov = self.coverage;

        if cov == 0.0 {
            return (poles, wires, base_ent_n);
        }

        use std::cmp::Ordering;

        struct OrderedDVec2(DVec2);
        impl Ord for OrderedDVec2 {
            fn cmp(&self, other: &Self) -> Ordering {
                let (a, b) = (&self.0, &other.0);
                a.y.total_cmp(&b.y).then_with(|| a.x.total_cmp(&b.x))
            }
        }
        impl PartialOrd for OrderedDVec2 {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl PartialEq for OrderedDVec2 {
            fn eq(&self, other: &Self) -> bool {
                self.cmp(other) == Ordering::Equal
            }
        }
        impl Eq for OrderedDVec2 {}

        let horiz = true ^ matches!(args.image_rotation, Deg90 | Deg270);
        let mut pole_map = BTreeMap::<OrderedDVec2, u32>::new();

        self.requests
            .sort_by(|a, b| a.y.total_cmp(&b.y).then_with(|| a.x.total_cmp(&b.x)));

        // Create all power poles first. This lets us determine the topology
        // from the actual generated positions without making assumptions
        // about the shape of the layout.
        for &req in &self.requests {
            let mut en = self.ref_entity.clone();
            en.entity_number = curr_ent_n;
            en.position += req;

            pole_map.insert(OrderedDVec2(en.position), curr_ent_n);
            poles.push(en);
            curr_ent_n += 1;
        }

        for pole in &poles {
            let pos = pole.position;
            if let Some(&prev) = pole_map.get(&OrderedDVec2(pos + dvec2(-cov, 0.0))) {
                wires.push([prev, WIRE_C, pole.entity_number, WIRE_C]);
            }
            if let Some(&prev) = pole_map.get(&OrderedDVec2(pos + dvec2(0.0, cov))) {
                wires.push([prev, WIRE_C, pole.entity_number, WIRE_C]);
            }
        }

        return (poles, wires, curr_ent_n);
    }

    /// Tests if it is possible to place an entity at the given point, without overlapping with a power pole.
    ///
    /// # Returns
    ///
    /// A `bool` indicating whether the power pole would NOT occupy the given point.
    pub fn test(&mut self, point: DVec2) -> bool {
        self.test_center(point).0
    }

    fn test_center(&mut self, mut point: DVec2) -> (bool, DVec2) {
        if self.coverage == 0.0 {
            return (true, DVec2::ZERO);
        }
        point -= self.ref_entity.position;

        let center = (point / self.coverage).round() * self.coverage;
        let delta = (point - center).abs();
        let did_hit = delta.x >= self.bounding_box.x / 2.0 || delta.y >= self.bounding_box.y / 2.0;

        (did_hit, center)
    }

    /// Requests that a power pole be placed to power the given point.
    ///
    /// # Returns
    ///
    /// A `bool` indicating whether the power pole would NOT occupy the given point.
    pub fn request(&mut self, point: DVec2) -> bool {
        let (hit, center) = self.test_center(point);
        if !self.requests.contains(&center) {
            self.requests.push(center);
        }
        hit
    }
}
