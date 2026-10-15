#![allow(dead_code)]

use crate::blueprint::{constants::*, dvec2_xy::*, util::*};
pub use crate::models::*;
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};

#[derive(Default, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArithmeticCombinatorParameterOperation {
    #[default]
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
impl ArithmeticCombinatorParameterOperation {
    pub const fn default() -> Self {
        Self::Mul
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ArithmeticCombinatorParameters {
    /// First signal to use in an operation. If not specified, the second argument will be the value of first_constant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_signal: Option<SignalID>,

    /// When not specified, defaults to "*".
    #[serde(default, skip_serializing_if = "is_default")]
    pub operation: ArithmeticCombinatorParameterOperation,

    /// Second signal to use in an operation. If not specified, the second argument will be the value of second_constant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second_signal: Option<SignalID>,

    /// Constant to use as the second argument of the operation. Has no effect when second_signal is set. Defaults to 0.
    #[serde(default, skip_serializing_if = "is_default")]
    pub second_constant: i32,

    /// Specifies the signal to output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "is_none_or_default")]
    pub direction: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_behavior: Option<ControlBehavior>,

    #[serde(default, skip_serializing_if = "is_none_or_default")]
    pub player_description: Option<String>,

    /// The prototype name of the entity's quality.
    #[serde(default, skip_serializing_if = "SignalID::is_none_or_default_quality")]
    pub quality: Option<Arc<str>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub always_on: Option<bool>,

    #[serde(skip)]
    pub custom_tag: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}
impl BlueprintEntity {
    /// Create a new entity with default None values for optional fields
    pub fn new(entity_number: u32, name: Arc<str>, position: DVec2) -> Self {
        BlueprintEntity {
            entity_number,
            name,
            position,
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

    pub const fn with_always_on(mut self, always_on: bool) -> Self {
        self.always_on = Some(always_on);
        self
    }
    pub const fn with_color(mut self, color: Color) -> Self {
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
    pub const fn with_direction(mut self, direction: u32) -> Self {
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
pub struct BlueprintInner {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub icons: Option<Vec<Icon>>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<BlueprintEntity>,

    #[serde(default, skip_serializing_if = "is_none_or_default")]
    pub wires: Option<Vec<Wire>>,

    pub item: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<Arc<str>>, // Note: MUST use Option::is_none, NOT is_none_or_normal_quality

    /// The comparator for quality. nil if any quality.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparator: Option<ComparatorString>,
    pub count: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LogisticSections {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<LogisticSection>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Color {
    #[serde(default, skip_serializing_if = "is_default")]
    pub r: f64,

    #[serde(default, skip_serializing_if = "is_default")]
    pub g: f64,

    #[serde(default, skip_serializing_if = "is_default")]
    pub b: f64,

    #[serde(
        default = "Color::default_a",
        skip_serializing_if = "Color::is_default_a"
    )]
    pub a: f64,
}
impl Color {
    pub const fn default_a() -> f64 {
        1.0
    }

    pub const fn is_default_a(a: &f64) -> bool {
        *a == Color::default_a()
    }

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

/// A string that specifies how the inputs should be compared.
///
/// While the API accepts both versions for "less/greater than or equal to" and "not equal",
/// it'll always return "≥", "≤" or "≠" respectively when reading them back.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparatorString {
    /// "equal to"
    #[serde(rename = "=")]
    Eq,

    /// "greater than"
    #[serde(rename = ">")]
    Gt,

    /// "lesser than"
    #[default]
    #[serde(rename = "<")]
    Lt,

    /// "greater than or equal to"
    #[serde(rename = "≥", alias = ">=")]
    Ge,

    /// "lesser than or equal to"
    #[serde(rename = "≤", alias = "<=")]
    Le,

    /// "not equal to"
    #[serde(rename = "≠", alias = "!=")]
    Ne,
}
impl ComparatorString {
    const fn default() -> Self {
        Self::Lt
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ControlBehavior {
    ArithmeticCombinatorBlueprintControlBehavior {
        arithmetic_conditions: ArithmeticCombinatorParameters,
    },
    ConstantCombinatorBlueprintControlBehavior {
        sections: LogisticSections,
    },
    DeciderCombinatorBlueprintControlBehavior {
        decider_conditions: DeciderCombinatorParameters,
    },
    LampBlueprintControlBehavior {
        /// Defaults to false.
        #[serde(default, skip_serializing_if = "is_default")]
        use_colors: bool,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        red_signal: Option<Arc<SignalID>>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        green_signal: Option<Arc<SignalID>>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        blue_signal: Option<Arc<SignalID>>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        rgb_signal: Option<Arc<SignalID>>,

        /// Defaults to defines.control_behavior.lamp.color_mapping.
        color_mode: i8,
    },
}
impl ControlBehavior {
    pub const fn arithmetic(arithmetic_conditions: ArithmeticCombinatorParameters) -> Self {
        Self::ArithmeticCombinatorBlueprintControlBehavior {
            arithmetic_conditions,
        }
    }
    pub const fn decider(decider_conditions: DeciderCombinatorParameters) -> Self {
        Self::DeciderCombinatorBlueprintControlBehavior { decider_conditions }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CircuitNetworkSelection {
    pub red: bool,
    pub green: bool,
}
impl Default for CircuitNetworkSelection {
    fn default() -> Self {
        Self::both()
    }
}
impl CircuitNetworkSelection {
    pub const fn both() -> Self {
        Self {
            red: true,
            green: true,
        }
    }
    pub const fn default() -> Self {
        Self::both()
    }
    pub const fn green() -> Self {
        Self {
            red: false,
            green: true,
        }
    }
    pub const fn red() -> Self {
        Self {
            red: true,
            green: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompareTypeString {
    #[serde(rename = "and")]
    And,

    #[serde(rename = "or")]
    Or,
}
impl CompareTypeString {
    pub const fn default() -> Self {
        Self::Or
    }
}
impl Default for CompareTypeString {
    fn default() -> Self {
        Self::Or
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeciderCombinatorCondition {
    /// Defaults to blank
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_signal: Option<SignalID>,

    /// Which circuit networks (red/green) to read first_signal from. Defaults to both.
    #[serde(default, skip_serializing_if = "is_default")]
    pub first_signal_networks: CircuitNetworkSelection,

    /// Constant to compare first_signal to. Has no effect when second_signal is set.
    /// When neither second_signal nor constant are specified, the effect is as though
    /// constant were specified with the value 0.
    #[serde(default, skip_serializing_if = "is_default")]
    pub constant: Option<i32>,

    /// Specifies how the inputs should be compared. If not specified, defaults to "<".
    #[serde(default, skip_serializing_if = "is_default")]
    pub comparator: ComparatorString,

    /// Tells how this condition is compared with the preceding conditions in the
    /// corresponding conditions array. Defaults to "or".
    #[serde(default, skip_serializing_if = "is_default")]
    pub compare_type: CompareTypeString,
}
impl DeciderCombinatorCondition {
    pub const fn new(first_signal: SignalID, constant: i32, comparator: ComparatorString) -> Self {
        Self {
            first_signal: Some(first_signal),
            constant: Some(constant),
            comparator,
            compare_type: const { CompareTypeString::default() },
            first_signal_networks: const { CircuitNetworkSelection::default() },
        }
    }

    pub const fn with_compare_type(mut self, compare_type: CompareTypeString) -> Self {
        self.compare_type = compare_type;
        self
    }
    pub const fn with_first_signal_networks(mut self, networks: CircuitNetworkSelection) -> Self {
        self.first_signal_networks = networks;
        self
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeciderCombinatorOutput {
    /// Specifies a signal to output.
    pub signal: Arc<SignalID>,

    /// Defaults to true. When false, will output the value from constant for the given output_signal.
    #[serde(
        default = "DeciderCombinatorOutput::default_copy_count_from_input",
        skip_serializing_if = "DeciderCombinatorOutput::is_default_copy_count_from_input"
    )]
    pub copy_count_from_input: bool,

    /// The value to output when not copying input. Defaults to 1.
    #[serde(
        default = "DeciderCombinatorOutput::default_constant",
        skip_serializing_if = "DeciderCombinatorOutput::is_default_constant"
    )]
    pub constant: i32,

    /// Sets which input network to read the value of signal from if copy_count_from_input is true. Defaults to both.
    #[serde(default, skip_serializing_if = "is_default")]
    pub networks: CircuitNetworkSelection,
}
impl DeciderCombinatorOutput {
    pub fn new(signal: Arc<SignalID>, constant: Option<i32>) -> Self {
        Self {
            copy_count_from_input: constant.is_none(),
            constant: constant.unwrap_or(Self::default_constant()),
            signal,
            networks: const { CircuitNetworkSelection::default() },
        }
    }

    pub const fn default_constant() -> i32 {
        1
    }
    pub const fn default_copy_count_from_input() -> bool {
        true
    }

    pub const fn is_default_constant(constant: &i32) -> bool {
        *constant == DeciderCombinatorOutput::default_constant()
    }
    pub const fn is_default_copy_count_from_input(copy_count_from_input: &bool) -> bool {
        *copy_count_from_input == DeciderCombinatorOutput::default_copy_count_from_input()
    }

    pub const fn with_networks(mut self, networks: CircuitNetworkSelection) -> Self {
        self.networks = networks;
        self
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeciderCombinatorParameters {
    /// List of conditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<DeciderCombinatorCondition>,

    /// List of outputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<DeciderCombinatorOutput>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Icon {
    pub signal: SignalID,
    pub index: u32,
}

pub type LogisticFilterIndex = u16;

#[derive(Clone, Serialize, Deserialize)]
pub struct LogisticSection {
    pub index: u8,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<BlueprintLogisticFilter>,
}

pub trait NetworkSwapper {
    fn swap_networks(&mut self);
}

impl NetworkSwapper for DeciderCombinatorParameters {
    fn swap_networks(&mut self) {
        self.outputs.iter_mut().for_each(|o| o.swap_networks());
        for c in self.conditions.iter_mut() {
            c.first_signal_networks.red ^= true;
            c.first_signal_networks.green ^= true;
        }
    }
}
impl NetworkSwapper for DeciderCombinatorOutput {
    fn swap_networks(&mut self) {
        self.networks.red ^= true;
        self.networks.green ^= true;
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

    /// The prototype name of the signal's quality. Defaults to normal.
    #[serde(default, skip_serializing_if = "SignalID::is_none_or_default_quality")]
    pub quality: Option<Arc<str>>,

    /// Name of the prototype.
    pub name: Arc<str>,
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

    pub fn default_signal_type() -> Arc<str> {
        Arc::clone(&Self::DEFAULT_SIGNAL_TYPE)
    }
    pub fn default_quality() -> Option<Arc<str>> {
        Some(Arc::from("normal"))
    }

    pub fn is_default_signal_type(value: &Arc<str>) -> bool {
        *value == SignalID::default_signal_type()
    }
    pub fn is_none_or_default_quality(value: &Option<Arc<str>>) -> bool {
        *value == None || *value == SignalID::default_quality()
    }
}

pub type Wire = [u32; 4]; // TODO: could use an unpacked (u32, u8, u32, u8)-type data structure for better memory
