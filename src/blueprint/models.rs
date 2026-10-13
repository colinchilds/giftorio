use crate::blueprint::{constants::*, util::*};
pub use crate::models::*;
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};

#[derive(Clone, Serialize, Deserialize)]
pub struct BlueprintBook {
    pub icons: Option<Vec<Icon>>,
    pub active_index: u32,
    pub blueprint_book: BlueprintBookInner,
    pub item: String,
    pub label: Option<String>,
    pub version: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BlueprintBookInner {
    pub blueprints: Vec<Blueprint>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub blueprint: BlueprintInner,
}
impl Blueprint {
    pub fn from_json(json: String) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Self>(&json)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BlueprintInner {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub icons: Option<Vec<Icon>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<Entity>,
    #[serde(skip_serializing_if = "is_none_or_empty_vec")]
    pub wires: Option<Vec<Wire>>,
    pub item: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_color: Option<Color>,
    pub version: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Icon {
    pub signal: Signal,
    pub index: u32,
}

pub fn is_none_or_normal_quality(value: &Option<Arc<str>>) -> bool {
    value.as_ref().is_none_or(|v| **v == *"normal")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signal {
    #[serde(
        rename = "type",
        default = "Signal::default_signal_type",
        skip_serializing_if = "Signal::is_default_signal_type"
    )]
    pub type_: Arc<str>,
    pub name: Arc<str>,
    #[serde(skip_serializing_if = "is_none_or_normal_quality")]
    pub quality: Option<Arc<str>>,
}
impl Signal {
    pub fn new_virtual(name: Arc<str>) -> Self {
        Self {
            type_: Arc::clone(&SIG_TYPE_VIRTUAL),
            name: Arc::clone(&name),
            quality: None,
        }
    }

    const DEFAULT_SIGNAL_TYPE: LazyLock<Arc<str>> = LazyLock::new(|| Arc::from("item"));

    fn default_signal_type() -> Arc<str> {
        Arc::clone(&Self::DEFAULT_SIGNAL_TYPE)
    }
    fn is_default_signal_type(value: &Arc<str>) -> bool {
        value.to_string() == "item"
    }
}

pub type Wire = [u32; 4]; // TODO: could use an unpacked (u32, u8, u32, u8)-type data structure for better memory

mod dvec2_xy {
    use glam::DVec2;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct XY {
        x: f64,
        y: f64,
    }

    pub fn serialize<S>(v: &DVec2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        XY { x: v.x, y: v.y }.serialize(serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DVec2, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let xy = XY::deserialize(deserializer)?;
        Ok(DVec2::new(xy.x, xy.y))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_number: u32,
    pub name: Arc<str>,
    #[serde(with = "dvec2_xy")]
    pub position: DVec2,
    #[serde(skip_serializing_if = "Entity::is_none_or_0")]
    pub direction: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_behavior: Option<ControlBehavior>,
    #[serde(skip_serializing_if = "is_none_or_empty_string")]
    pub player_description: Option<String>,
    #[serde(skip_serializing_if = "is_none_or_normal_quality")]
    pub quality: Option<Arc<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_on: Option<bool>,
    #[serde(skip)]
    pub custom_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}
impl Entity {
    /// Create a new entity with default None values for optional fields
    pub fn new(entity_number: u32, name: Arc<str>, position: impl Into<DVec2>) -> Self {
        Entity {
            entity_number,
            name,
            position: position.into(),
            direction: None,
            control_behavior: None,
            player_description: None,
            quality: None,
            always_on: None,
            custom_tag: None,
            color: None,
        }
    }

    pub fn has_tag(&self, tag: String) -> bool {
        self.custom_tag == Some(tag)
    }

    pub fn with_always_on(mut self, always_on: bool) -> Self {
        self.always_on = Some(always_on);
        self
    }
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    pub fn with_control_behavior(mut self, behavior: ControlBehavior) -> Self {
        self.control_behavior = Some(behavior);
        self
    }
    pub fn with_description(mut self, desc: &str) -> Self {
        self.player_description = Some(desc.to_owned());
        self
    }
    pub fn with_direction(mut self, direction: u32) -> Self {
        self.direction = Some(direction);
        self
    }
    pub fn with_quality(mut self, quality: Option<Arc<str>>) -> Self {
        self.quality = quality;
        self
    }

    /// Custom tag for the entity. This does not get serialized.
    ///
    /// Utility to help identify entities for wire connections.
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.custom_tag = Some(tag.to_owned());
        self
    }

    fn is_none_or_0(value: &Option<u32>) -> bool {
        return matches!(value, None | Some(0));
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}
impl Color {
    pub fn from_hex(hex: &str) -> Option<Color> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;

        Some(Color { r, g, b, a: 1.0 })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ControlBehavior {
    Constant {
        sections: Sections,
    },
    Decider {
        decider_conditions: DeciderConditions,
    },
    Arithmetic {
        arithmetic_conditions: ArithmeticConditions,
    },
    ColorLamp {
        use_colors: bool,
        color_mode: i8,
        rgb_signal: Arc<Signal>,
    },
    GrayLamp {
        use_colors: bool,
        color_mode: i8,
        red_signal: Arc<Signal>,
        green_signal: Arc<Signal>,
        blue_signal: Arc<Signal>,
    },
}
impl ControlBehavior {
    pub fn from_arithmetic_conditions(arithmetic: ArithmeticConditions) -> Self {
        Self::Arithmetic {
            arithmetic_conditions: arithmetic,
        }
    }
    pub fn from_decider_conditions(decider: DeciderConditions) -> Self {
        Self::Decider {
            decider_conditions: decider,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Sections {
    pub sections: Vec<Section>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Section {
    pub index: u32,
    pub filters: Vec<Filter>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ArithmeticConditions {
    pub first_signal: Option<Signal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_signal: Option<Signal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_constant: Option<i32>,
    pub operation: Option<String>,
    pub output_signal: Option<Signal>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DeciderConditions {
    pub conditions: Option<Vec<Condition>>,
    pub outputs: Option<Vec<CombinatorOutput>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Filter {
    pub index: u32,
    #[serde(
        rename = "type",
        default = "Signal::default_signal_type",
        skip_serializing_if = "Signal::is_default_signal_type"
    )]
    pub type_: Arc<str>,
    pub name: Arc<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Arc<str>>, // Note: MUST use Option::is_none, NOT is_none_or_normal_quality
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
}

pub trait NetworkSwapper {
    fn swap_networks(&mut self);
}

impl NetworkSwapper for DeciderConditions {
    fn swap_networks(&mut self) {
        if let Some(conds) = &mut self.conditions {
            for c in conds.iter_mut() {
                if let Some(n) = &mut c.first_signal_networks {
                    n.red ^= true;
                    n.green ^= true;
                }
            }
            if let Some(outs) = &mut self.outputs {
                outs.iter_mut().for_each(|o| o.swap_networks());
            }
        }
    }
}
impl NetworkSwapper for CombinatorOutput {
    fn swap_networks(&mut self) {
        if let Some(n) = &mut self.networks {
            n.red ^= true;
            n.green ^= true;
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Condition {
    pub first_signal: Option<Signal>,
    pub constant: Option<i32>,
    pub comparator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare_type: Option<String>,
    pub first_signal_networks: Option<NetworkFilters>,
}
impl Condition {
    pub fn new(first_signal: Signal, constant: i32, comparator: &str) -> Self {
        Self {
            first_signal: Some(first_signal),
            constant: Some(constant),
            comparator: Some(comparator.to_owned()),
            compare_type: None,
            first_signal_networks: None,
        }
    }

    pub fn with_compare_type(mut self, compare_type: &str) -> Self {
        self.compare_type = Some(compare_type.to_owned());
        self
    }
    pub fn with_first_signal_networks(mut self, networks: NetworkFilters) -> Self {
        self.first_signal_networks = Some(networks);
        self
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CombinatorOutput {
    pub copy_count_from_input: Option<bool>,
    pub signal: Option<Arc<Signal>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<NetworkFilters>,
}
impl CombinatorOutput {
    pub fn new(signal: Arc<Signal>, constant: Option<i32>) -> Self {
        Self {
            copy_count_from_input: Some(constant.is_none()),
            constant,
            signal: Some(signal),
            networks: None,
        }
    }

    pub fn with_networks(mut self, networks: NetworkFilters) -> Self {
        self.networks = Some(networks);
        self
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NetworkFilters {
    pub red: bool,
    pub green: bool,
}

#[allow(dead_code)]
impl NetworkFilters {
    #[inline(always)]
    pub fn new(modes: &str) -> Self {
        let red = modes.contains('r');
        let green = modes.contains('g');
        Self { red, green }
    }
}
