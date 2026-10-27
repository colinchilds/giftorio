use std::rc::Rc;
use serde::{Serialize, Serializer};
#[derive(Serialize)]
pub struct Blueprint {
    pub blueprint: BlueprintInner,
}

#[derive(Serialize)]
pub struct BlueprintInner {
    pub icons: Vec<Icon>,
    pub entities: Vec<Entity>,
    pub wires: Vec<Wire>,
    pub item: &'static str,
    pub version: u64,
}

#[derive(Serialize)]
pub struct Icon {
    pub signal: Signal,
    pub index: u32,
}

/// `type` and `quality` are left out when they're the game's defaults ("item" and "normal").
#[derive(Serialize, Clone)]
pub struct Signal {
    #[serde(rename = "type", skip_serializing_if = "is_default_type")]
    pub type_: Rc<String>,
    pub name: Rc<String>,
    #[serde(skip_serializing_if = "is_default_quality")]
    pub quality: Option<&'static str>,
}

fn is_default_type(type_: &Rc<String>) -> bool {
    type_.as_str() == "item"
}

fn is_default_quality(quality: &Option<&'static str>) -> bool {
    matches!(quality, None | Some("normal"))
}

pub type Wire = [u32; 4];

#[derive(Serialize)]
pub struct Entity {
    pub entity_number: u32,
    pub name: &'static str,
    pub position: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_behavior: Option<ControlBehavior>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_description: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_on: Option<bool>,
}

impl Entity {
    /// Create a new entity with default None values for optional fields
    pub fn new(entity_number: u32, name: &'static str, position: Position) -> Self {
        Entity {
            entity_number,
            name,
            position,
            direction: None,
            control_behavior: None,
            player_description: None,
            quality: None,
            always_on: None,
        }
    }

    pub fn with_direction(mut self, direction: u32) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn with_control_behavior(mut self, behavior: ControlBehavior) -> Self {
        self.control_behavior = Some(behavior);
        self
    }

    pub fn with_description(mut self, desc: &'static str) -> Self {
        self.player_description = Some(desc);
        self
    }

    pub fn with_always_on(mut self, always_on: bool) -> Self {
        self.always_on = Some(always_on);
        self
    }
}

#[derive(Serialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize)]
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
        rgb_signal: Rc<Signal>,
    },
    GrayLamp {
        use_colors: bool,
        color_mode: i8,
        red_signal: Rc<Signal>,
        green_signal: Rc<Signal>,
        blue_signal: Rc<Signal>,
    },
}

#[derive(Serialize)]
pub struct Sections {
    pub sections: Vec<Section>,
}

#[derive(Serialize)]
pub struct Section {
    pub index: u32,
    pub filters: Vec<Filter>,
}

#[derive(Serialize)]
pub struct Filter {
    pub index: u32,
    #[serde(rename = "type")]
    pub type_: &'static str,
    pub name: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparator: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

#[derive(Serialize)]
pub struct DeciderConditions {
    pub conditions: Vec<Condition>,
    pub outputs: DeciderOutputs,
}

pub enum DeciderOutputs {
    List(Vec<CombinatorOutput>),
    /// `values[i]` is output on `signals[i]`. Stored as bare values to keep frame data small.
    /// Zero values are left out, since a constant-0 output emits nothing.
    Pixels {
        signals: Rc<[Rc<Signal>]>,
        values: Vec<i32>,
    },
}

/// Must serialize identically to `CombinatorOutput`.
#[derive(Serialize)]
struct PixelOutput<'a> {
    copy_count_from_input: bool,
    constant: i32,
    signal: &'a Signal,
}

impl Serialize for DeciderOutputs {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            DeciderOutputs::List(outputs) => outputs.serialize(serializer),
            DeciderOutputs::Pixels { signals, values } => serializer.collect_seq(
                values.iter().zip(signals.iter()).filter(|(&constant, _)| constant != 0).map(|(&constant, signal)| PixelOutput {
                    copy_count_from_input: false,
                    constant,
                    signal,
                }),
            ),
        }
    }
}

#[derive(Serialize)]
pub struct Condition {
    pub first_signal: Signal,
    pub constant: i32,
    pub comparator: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compare_type: Option<&'static str>,
}

#[derive(Serialize, Clone)]
pub struct CombinatorOutput {
    pub copy_count_from_input: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant: Option<i32>,
    pub signal: Rc<Signal>,
}

#[derive(Serialize)]
pub struct ArithmeticConditions {
    pub first_signal: Signal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_signal: Option<Signal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub second_constant: Option<i32>,
    pub operation: &'static str,
    pub output_signal: Signal,
}