/// Default delay in milliseconds when a frame’s delay is zero.
pub const DEFAULT_FRAME_DELAY_MS: u32 = 100;

/// Number of milliseconds per second.
pub const MS_PER_SECOND: f64 = 1000.0;

/// Timer entity positions.
pub const TIMER_ENTITY1_POSITION: (f64, f64) = (-2.5, -3.0);
pub const TIMER_ENTITY2_POSITION: (f64, f64) = (-1.5, -3.0);
pub const TIMER_ENTITY3_POSITION: (f64, f64) = (-1.5, -4.0);
pub const TIMER_ENTITY4_POSITION: (f64, f64) = (-1.5, -5.0);
pub const TIMER_ENTITY5_POSITION: (f64, f64) = (-2.5, -5.5);
pub const TIMER_ENTITY6_POSITION: (f64, f64) = (-1.5, -6.0);

/// Direction constants.
pub const DIRECTION_RIGHT: u32 = 4;
pub const DIRECTION_LEFT: u32 = 12;

/// Blueprint version constant.
pub const BLUEPRINT_VERSION: u64 = 562949955518464;

/// Threshold used for binary grayscale conversion.
pub const GRAYSCALE_THRESHOLD: u8 = 128;

/// Quality constants.
pub const QUALITY_NORMAL: &str = "normal";
pub const QUALITY_UNCOMMON: &'static str = "uncommon";
pub const QUALITY_RARE: &'static str = "rare";
pub const QUALITY_EPIC: &'static str = "epic";
pub const QUALITY_LEGENDARY: &'static str = "legendary";
pub const QUALITY_UNKNOWN: &'static str = "quality-unknown";
pub const QUALITY_NONE: &'static str = "none";

/// Entity types
pub const DECIDER_COMBINATOR: &'static str = "decider-combinator";
pub const ARITHMETIC_COMBINATOR: &'static str = "arithmetic-combinator";
pub const CONSTANT_COMBINATOR: &'static str = "constant-combinator";
pub const SUBSTATION: &'static str = "substation";
pub const LAMP: &'static str = "small-lamp";
pub const BLUEPRINT: &'static str = "blueprint";

/// Signal types
pub const SIGNAL_TYPE_VIRTUAL: &'static str = "virtual";

/// Signals
pub const SIGNAL_F: &'static str = "signal-F";
pub const SIGNAL_S: &'static str = "signal-S";
pub const SIGNAL_T: &'static str = "signal-T";
pub const SIGNAL_EACH: &'static str = "signal-each";

/// Comparators
pub const COMPARATOR_EQUAL: &'static str = "equal";
pub const COMPARATOR_GREATER_EQUAL: &'static str = ">=";
pub const COMPARATOR_LESS: &'static str = "<";

/// Operations
pub const OPERATION_MOD: &'static str = "%";
pub const OPERATION_MUL: &'static str = "*";
pub const OPERATION_DIV: &'static str = "/";
pub const OPERATION_SUB: &'static str = "-";
pub const OPERATION_SHIFT_R: &'static str = ">>";
pub const OPERATION_AND: &'static str = "AND";