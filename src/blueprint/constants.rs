#![allow(dead_code)]

use glam::{dvec2, DVec2};

use crate::macros::lazy_const;

/// Timer entity positions.
pub const TIMER1_POS: DVec2 = dvec2(-2.5, -3.0);
pub const TIMER2_POS: DVec2 = dvec2(-1.0, -3.0);
pub const TIMER3_POS: DVec2 = dvec2(-1.0, -4.0);
pub const TIMER4_POS: DVec2 = dvec2(-1.0, -5.0);
pub const TIMER5_POS: DVec2 = dvec2(-2.5, -5.5);
pub const TIMER6_POS: DVec2 = dvec2(-1.0, -6.0);

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
lazy_const!(pub LAMP            : str     = "small-lamp"           );
lazy_const!(pub BLUEPRINT       : str     = "blueprint"            );

lazy_const!(pub DEFAULT_QUAL_KEY: str     = "<default>"            );
lazy_const!(pub SIG_TYPE_VIRTUAL: str     = "virtual"              );

pub const DEFAULT_POWER_POLE_NAME: &'static str = "substation";

pub const SIG_EACH: std::sync::LazyLock<crate::blueprint::models::SignalID> =
    std::sync::LazyLock::new(|| {
        crate::blueprint::models::SignalID::new_virtual(std::sync::Arc::from("signal-each"))
    });

pub const WIRE_R: u32 = 1;
pub const WIRE_G: u32 = 2;
pub const WIRE_OUT_R: u32 = 3;
pub const WIRE_OUT_G: u32 = 4;
pub const WIRE_C: u32 = 5;
