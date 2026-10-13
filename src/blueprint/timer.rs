use crate::blueprint::{constants::*, macros::*, models::*};
use std::sync::Arc;

/// Generates timer entities and wires for the blueprint's timing mechanism.
///
/// This is only run once for the entire blueprint.
///
/// # Arguments
///
/// * `stop` - Maximum tick value before reset.
/// * `ticks_per_frame` - Ticks per frame (derived from FPS).
/// * `frames_per_combinator` - Number of frames processed per combinator (derived from compression).
/// * `args` - Uses BlueprintArgs.grayscale_bits (affects extra combinators).
///
/// # Returns
///
/// A tuple with entities, wires, and the timer reset entity (which is not included in entities)
pub fn generate_timer(
    stop: u32,
    ticks_per_frame: u32,
    frames_per_comb: u32,
    args: &BlueprintArgs,
) -> (Vec<Entity>, Vec<Wire>) {
    let ts = &args.timing_sigs;
    let (sig_f, sig_s, sig_t) = (&ts.f, &ts.s, &ts.t);

    let gray_bits = args.grayscale_bits;
    let mut ents: Vec<Entity> = Vec::new();
    let mut wires: Vec<Wire> = Vec::new();
    let mut en = Entity::new(1, Arc::clone(&CONSTANT_COMB), TIMER1_POS).with_direction(DIR_R);
    let sections = Sections {
        sections: vec![Section {
            index: 1,
            filters: vec![Filter {
                index: 1,
                type_: sig_t.type_.clone(),
                name: sig_t.name.clone(),
                quality: sig_t.quality.clone(),
                comparator: Some(COMP_EQ.to_owned()),
                count: Some(1),
            }],
        }],
    };
    en = en.with_control_behavior(ControlBehavior::Constant { sections });
    ents.push(en.with_tag("incrementer"));

    let mut en = Entity::new(2, Arc::clone(&DEC_CB), TIMER2_POS).with_direction(DIR_R);
    let decider_conditions = DeciderConditions {
        conditions: Some(vec![Condition::new(sig_t.clone(), stop as i32, COMP_LT)]),
        outputs: Some(vec![CombinatorOutput::new(Arc::from(sig_t.clone()), None)]),
    };
    en = en.with_tag("base_timer").with_description("[virtual-signal=signal-T] increments 60 times per second up to the GIF's frame count, then resets to 0. It determines the current frame.");
    ents.push(en.with_control_behavior(ControlBehavior::Decider { decider_conditions }));

    let en = Entity::new(3, Arc::clone(&ARI_CB), TIMER3_POS).with_direction(DIR_R);
    let conds = arithmetic_conds!(sig_t - 1 => sig_t);
    ents.push(en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));

    wires.push([1, WIRE_R, 2, WIRE_R]);
    wires.push([2, WIRE_R, 2, WIRE_OUT_R]);
    wires.push([2, WIRE_R, 3, WIRE_R]);

    if gray_bits > 0 {
        let en = Entity::new(4, Arc::clone(&ARI_CB), TIMER4_POS).with_direction(DIR_L);
        let conds = arithmetic_conds!(sig_t % (ticks_per_frame * frames_per_comb) => sig_s);
        ents.push(en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));

        let en = Entity::new(5, Arc::clone(&ARI_CB), TIMER5_POS).with_direction(DIR_U);
        let conds = arithmetic_conds!(sig_s / ticks_per_frame => sig_f);
        ents.push(en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));

        let mut en = Entity::new(6, Arc::clone(&ARI_CB), TIMER6_POS).with_direction(DIR_R);
        let conds = arithmetic_conds!(SIG_EACH * gray_bits => SIG_EACH);
        en = en.with_description("Calculates the bit shift for current frame.");
        ents.push(en.with_control_behavior(ControlBehavior::from_arithmetic_conditions(conds)));

        wires.push([3, WIRE_R, 4, WIRE_R]);
        wires.push([4, WIRE_OUT_R, 5, WIRE_R]);
        wires.push([5, WIRE_OUT_R, 6, WIRE_R]);
        wires.push([6, WIRE_OUT_R, 3, WIRE_OUT_R]);
    }

    return (ents, wires);
}
