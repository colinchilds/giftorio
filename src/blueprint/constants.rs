#![allow(dead_code)]

use crate::macros::lazy_const;

/// Timer entity positions.
pub const TIMER1_POS: (f64, f64) = (-2.5, -3.0);
pub const TIMER2_POS: (f64, f64) = (-1.5, -3.0);
pub const TIMER3_POS: (f64, f64) = (-1.5, -4.0);
pub const TIMER4_POS: (f64, f64) = (-1.5, -5.0);
pub const TIMER5_POS: (f64, f64) = (-2.5, -5.5);
pub const TIMER6_POS: (f64, f64) = (-1.5, -6.0);

/// Direction constants.
pub const DIR_U: u32 = 0;
pub const DIR_R: u32 = 4;
pub const DIR_D: u32 = 8;
pub const DIR_L: u32 = 12;

/// Blueprint version constant.
pub const BLUEPRINT_VERSION: u64 = 562949955518464;

/// Threshold used for binary grayscale conversion. (out of 256)
pub const GRAYSCALE_THRESH: u8 = 128;

lazy_const!(pub DEC_CB          : str     = "decider-combinator"   );
lazy_const!(pub ARI_CB          : str     = "arithmetic-combinator");
lazy_const!(pub CONSTANT_COMB   : str     = "constant-combinator"  );
lazy_const!(pub SUBSTATION      : str     = "substation"           );
lazy_const!(pub LAMP            : str     = "small-lamp"           );
lazy_const!(pub BLUEPRINT       : str     = "blueprint"            );

lazy_const!(pub DEFAULT_QUAL_KEY: str     = "<default>"            );
lazy_const!(pub SIG_TYPE_VIRTUAL: str     = "virtual"              );

pub const SIG_EACH: std::sync::LazyLock<crate::blueprint::models::Signal> =
    std::sync::LazyLock::new(|| {
        crate::blueprint::models::Signal::new_virtual(std::sync::Arc::from("signal-each"))
    });

/// Comparators
pub const COMP_GT: &'static str = ">";
pub const COMP_LT: &'static str = "<";
pub const COMP_EQ: &'static str = "=";
pub const COMP_GE: &'static str = ">=";
pub const COMP_LE: &'static str = "<=";
pub const COMP_NE: &'static str = "!=";

/// Operations
pub const OP_MUL: &'static str = "*";
pub const OP_DIV: &'static str = "/";
pub const OP_ADD: &'static str = "+";
pub const OP_SUB: &'static str = "-";
pub const OP_MOD: &'static str = "%";
pub const OP_POW: &'static str = "^";
pub const OP_LSHIFT: &'static str = "<<";
pub const OP_RSHIFT: &'static str = ">>";
pub const OP_AND: &'static str = "AND";
pub const OP_OR: &'static str = "OR";
pub const OP_XOR: &'static str = "XOR";

/// Compare types
pub const COMP_AND: &'static str = "and";

pub const WIRE_R: u32 = 1;
pub const WIRE_G: u32 = 2;
pub const WIRE_OUT_R: u32 = 3;
pub const WIRE_OUT_G: u32 = 4;
pub const WIRE_C: u32 = 5;
