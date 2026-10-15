#![allow(dead_code)]

use crate::blueprint::{constants::*, dvec2_xy::*, util::*};
pub use crate::models::*;
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArithmeticCombinatorParameterOperation {
    #[serde(rename = "*")]
    Mul,

    #[serde(rename = "/")]
    Div,

    #[serde(rename = "+")]
    Add,

    #[serde(rename = "-")]
    Sub,

    #[serde(rename = "%")]
    Mod,

    #[serde(rename = "^")]
    Pow,

    #[serde(rename = "<<")]
    Shl,

    #[serde(rename = ">>")]
    Shr,

    #[serde(rename = "AND")]
    And,

    #[serde(rename = "OR")]
    Or,

    #[serde(rename = "XOR")]
    Xor,
}
impl Default for ArithmeticCombinatorParameterOperation {
    fn default() -> Self {
        Self::Mul
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ArithmeticCombinatorParameters {
    /// First signal to use in an operation. If not specified, the second argument will be the value of first_constant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_signal: Option<SignalID>,

    /// Second signal to use in an operation. If not specified, the second argument will be the value of second_constant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_signal: Option<SignalID>,

    /// Constant to use as the second argument of the operation. Has no effect when second_signal is set. Defaults to 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second_constant: Option<i32>,

    /// When not specified, defaults to "*".
    #[serde(default, skip_serializing_if = "is_default")]
    pub operation: ArithmeticCombinatorParameterOperation,

    /// Specifies the signal to output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_signal: Option<SignalID>,
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
pub struct BlueprintInner {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub icons: Option<Vec<Icon>>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<BlueprintEntity>,

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
pub struct BlueprintLogisticFilter {
    pub index: LogisticFilterIndex,

    /// The type of the logistic filter.
    #[serde(
        rename = "type",
        default = "SignalID::default_signal_type",
        skip_serializing_if = "SignalID::is_default_signal_type"
    )]
    pub type_: Arc<str>,

    /// Name of the logistic filter.
    pub name: Arc<str>,

    /// The prototype name of the quality. nil for any quality.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Arc<str>>, // Note: MUST use Option::is_none, NOT is_none_or_normal_quality

    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparator: Option<ComparatorString>,
    pub count: i32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparatorString {
    #[serde(rename = "=")]
    Eq,

    #[serde(rename = ">")]
    Gt,

    #[serde(rename = "<")]
    Lt,

    #[serde(rename = "≥", alias = ">=")]
    Ge,

    #[serde(rename = "≤", alias = "<=")]
    Le,

    #[serde(rename = "≠", alias = "!=")]
    Ne,
}

pub fn is_none_or_normal_quality(value: &Option<Arc<str>>) -> bool {
    value.as_ref().is_none_or(|v| **v == *"normal")
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ControlBehavior {
    ArithmeticCombinatorBlueprintControlBehavior {
        arithmetic_conditions: ArithmeticCombinatorParameters,
    },
    ConstantCombinatorBlueprintControlBehavior {
        sections: BlueprintLogisticSections,
    },
    DeciderCombinatorBlueprintControlBehavior {
        decider_conditions: DeciderConditions,
    },
    LampBlueprintControlBehavior {
        /// Defaults to false.
        #[serde(default)]
        use_colors: bool,

        #[serde(default)]
        red_signal: Option<Arc<SignalID>>,

        #[serde(default)]
        green_signal: Option<Arc<SignalID>>,

        #[serde(default)]
        blue_signal: Option<Arc<SignalID>>,

        #[serde(default)]
        rgb_signal: Option<Arc<SignalID>>,

        /// Defaults to defines.control_behavior.lamp.color_mapping.
        #[serde(default)]
        color_mode: i8,
    },
}
impl ControlBehavior {
    pub fn arithmetic(arithmetic: ArithmeticCombinatorParameters) -> Self {
        Self::ArithmeticCombinatorBlueprintControlBehavior {
            arithmetic_conditions: arithmetic,
        }
    }
    pub fn decider(decider: DeciderConditions) -> Self {
        Self::DeciderCombinatorBlueprintControlBehavior {
            decider_conditions: decider,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BlueprintLogisticSections {
    pub sections: Vec<LogisticSection>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CircuitNetworkSelection {
    pub red: bool,
    pub green: bool,
}

#[allow(dead_code)]
impl CircuitNetworkSelection {
    #[inline(always)]
    pub fn new(modes: &str) -> Self {
        let (red, green) = (modes.contains('r'), modes.contains('g'));
        Self { red, green }
    }
}

pub struct CompareType(pub String);
impl Default for CompareType {
    fn default() -> Self {
        Self(String::from("or"))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeciderCombinatorCondition {
    /// Defaults to blank
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_signal: Option<SignalID>,

    /// Which circuit networks (red/green) to read first_signal from. Defaults to both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_signal_networks: Option<CircuitNetworkSelection>,

    /// Constant to compare first_signal to. Has no effect when second_signal is set.
    /// When neither second_signal nor constant are specified, the effect is as though
    /// constant were specified with the value 0.
    pub constant: Option<i32>,

    /// Specifies how the inputs should be compared. If not specified, defaults to "<".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparator: Option<ComparatorString>,

    /// Tells how this condition is compared with the preceding conditions in the
    /// corresponding conditions array. Defaults to "or".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare_type: Option<String>,
}
impl DeciderCombinatorCondition {
    pub fn new(first_signal: SignalID, constant: i32, comparator: ComparatorString) -> Self {
        Self {
            first_signal: Some(first_signal),
            constant: Some(constant),
            comparator: Some(comparator),
            compare_type: None,
            first_signal_networks: None,
        }
    }

    pub fn with_compare_type(mut self, compare_type: &str) -> Self {
        self.compare_type = Some(compare_type.to_owned());
        self
    }
    pub fn with_first_signal_networks(mut self, networks: CircuitNetworkSelection) -> Self {
        self.first_signal_networks = Some(networks);
        self
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeciderCombinatorOutput {
    /// Specifies a signal to output.
    pub signal: Option<Arc<SignalID>>,

    /// Defaults to true. When false, will output the value from constant for the given output_signal.
    pub copy_count_from_input: Option<bool>,

    /// The value to output when not copying input. Defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant: Option<i32>,

    /// Sets which input network to read the value of signal from if copy_count_from_input is true. Defaults to both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<CircuitNetworkSelection>,
}
impl DeciderCombinatorOutput {
    pub fn new(signal: Arc<SignalID>, constant: Option<i32>) -> Self {
        Self {
            copy_count_from_input: Some(constant.is_none()),
            constant,
            signal: Some(signal),
            networks: None,
        }
    }

    pub fn with_networks(mut self, networks: CircuitNetworkSelection) -> Self {
        self.networks = Some(networks);
        self
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeciderConditions {
    pub conditions: Option<Vec<DeciderCombinatorCondition>>,
    pub outputs: Option<Vec<DeciderCombinatorOutput>>,
}

/// The representation of an entity inside of a blueprint.
#[derive(Clone, Serialize, Deserialize)]
pub struct BlueprintEntity {
    /// The entity's unique identifier in the blueprint.
    pub entity_number: u32,

    /// The prototype name of the entity.
    pub name: Arc<str>,

    /// The position of the entity.
    #[serde(with = "dvec2_xy")]
    pub position: DVec2,

    /// The direction the entity is facing. Only present for entities that can face in
    /// different directions and when the entity is not facing north.
    #[serde(skip_serializing_if = "is_none_or_default")]
    pub direction: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_behavior: Option<ControlBehavior>,

    #[serde(skip_serializing_if = "is_none_or_empty_string")]
    pub player_description: Option<String>,

    /// The prototype name of the entity's quality.
    #[serde(skip_serializing_if = "is_none_or_normal_quality")]
    pub quality: Option<Arc<str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_on: Option<bool>,

    #[serde(skip)]
    pub custom_tag: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}
impl BlueprintEntity {
    /// Create a new entity with default None values for optional fields
    pub fn new(entity_number: u32, name: Arc<str>, position: impl Into<DVec2>) -> Self {
        BlueprintEntity {
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
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Icon {
    pub signal: SignalID,
    pub index: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LogisticSection {
    pub index: u8,
    pub filters: Vec<BlueprintLogisticFilter>,
}

pub type LogisticFilterIndex = u16;

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
impl NetworkSwapper for DeciderCombinatorOutput {
    fn swap_networks(&mut self) {
        if let Some(n) = &mut self.networks {
            n.red ^= true;
            n.green ^= true;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignalID {
    /// The type of the signal. If the type is "item", this will be nil when reading.
    #[serde(
        rename = "type",
        default = "SignalID::default_signal_type",
        skip_serializing_if = "SignalID::is_default_signal_type"
    )]
    pub type_: Arc<str>,

    /// Name of the prototype.
    pub name: Arc<str>,

    /// The prototype name of the signal's quality. Defaults to normal.
    #[serde(skip_serializing_if = "is_none_or_normal_quality")]
    pub quality: Option<Arc<str>>,
}
impl SignalID {
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
