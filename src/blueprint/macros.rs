macro_rules! arithmetic_conds {
    ($in_sig:ident $op:tt $second:expr => $out_sig:ident) => {
        ArithmeticConditions {
            first_signal: Some($in_sig.clone()),
            second_signal: None,
            second_constant: Some($second as i32),
            operation: Some($crate::blueprint::macros::arithmetic_combinator_op!($op).to_owned()),
            output_signal: Some($out_sig.clone()),
        }
    };
}

macro_rules! arithmetic_combinator_op {
    (*) => {
        OP_MUL
    };
    (/) => {
        OP_DIV
    };
    (+) => {
        OP_ADD
    };
    (-) => {
        OP_SUB
    };
    (%) => {
        OP_MOD
    };
    (^) => {
        OP_POW
    };
    (<<) => {
        OP_LSHIFT
    };
    (>>) => {
        OP_RSHIFT
    };
    (AND) => {
        OP_AND
    };
    (OR) => {
        OP_OR
    };
    (XOR) => {
        OP_XOR
    };
}

/// High-level utility macro for making wire connections.
macro_rules! mkwires {
    ($wire_type:ident $ents:ident; $src_type:ident $src_name:expr => $dst_type:ident $dst_name:expr) => {
        [
            entity_idx_by_tag!($ents, $src_name).expect("No entity found with tag"),
            mkwires!(@wire_type $wire_type, $src_type),
            entity_idx_by_tag!($ents, $dst_name).expect("No entity found with tag"),
            mkwires!(@wire_type $wire_type, $dst_type),
        ]
    };

    (@wire_type R, IN) => { 1 };
    (@wire_type G, IN) => { 2 };
    (@wire_type R, OUT) => { 3 };
    (@wire_type G, OUT) => { 4 };

    (@wire_type C, IN) => { 5 };
    (@wire_type C, OUT) => { 6 };
}
macro_rules! entity_idx_by_tag {
    ($ents:ident, $tag:expr) => {
        $ents
            .iter()
            .find(|e| e.has_tag($tag.to_string()))
            .map(|e| e.entity_number)
    };
}

pub(crate) use arithmetic_combinator_op;
pub(crate) use arithmetic_conds;
pub(crate) use entity_idx_by_tag;
pub(crate) use mkwires;
