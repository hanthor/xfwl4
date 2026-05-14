use pixman_sys as ffi;
use thiserror::Error;

/// Defines the possible operations for composition
#[derive(Debug, Clone, Copy)]
pub enum Operation {
    /// Clear
    Clear,
    /// Src
    Src,
    /// Dst
    Dst,
    /// Over
    Over,
    /// Over-reverse
    OverReverse,
    /// In
    In,
    /// In-reverse
    InReverse,
    /// Out
    Out,
    /// Out-reverse
    OutReverse,
    /// A-top
    Atop,
    /// A-top-reverse
    AtopReverse,
    /// Xor
    Xor,
    /// Add
    Add,
    /// Saturate
    Saturate,
    /// Disjoint-clear
    DisjointClear,
    /// Disjoint-src
    DisjointSrc,
    /// Disjoint-dst
    DisjointDst,
    /// Disjoint-over
    DisjointOver,
    /// Disjoint-over-reverse
    DisjointOverReverse,
    /// Disjoint-in
    DisjointIn,
    /// Disjoint-in-reverse
    DisjointInReverse,
    /// Disjoint-out
    DisjointOut,
    /// Disjoint-out-reverse
    DisjointOutReverse,
    /// Disjoint-a-top
    DisjointAtop,
    /// Disjoint-a-top-reverse
    DisjointAtopReverse,
    /// Disjoint-xor
    DisjointXor,
    /// Conjoint-clear
    ConjointClear,
    /// Conjoint-src
    ConjointSrc,
    /// Conjoint-dst
    ConjointDst,
    /// Conjoint-over
    ConjointOver,
    /// Conjoint-over-reverse
    ConjointOverReverse,
    /// Conjoint-in
    ConjointIn,
    /// Conjoint-in-reverse
    ConjointInReverse,
    /// Conjoint-out
    ConjointOut,
    /// Conjoint-out-reverse
    ConjointOutReverse,
    /// Conjoint-a-top
    ConjointAtop,
    /// Conjoint-a-top-reverse
    ConjointAtopReverse,
    /// Conjoint-xor
    ConjointXor,
    /// Multiply
    Multiply,
    /// Screen
    Screen,
    /// Overlay
    Overlay,
    /// Darken
    Darken,
    /// Lighten
    Lighten,
    /// Color dodge
    ColorDodge,
    /// Color burn
    ColorBurn,
    /// Hard-light
    HardLight,
    /// Soft-light
    SoftLight,
    /// Difference
    Difference,
    /// Exclustion
    Exclustion,
    /// Hls hue
    HslHue,
    /// Hls saturation
    HslSaturation,
    /// Hls color
    HslColor,
    /// Hls luminosity
    HslLuminosity,
}

/// The operation is unknown
#[derive(Debug, Error)]
#[error("Unknown operation {0}")]
pub struct UnknownOperation(ffi::pixman_op_t);

impl TryFrom<ffi::pixman_op_t> for Operation {
    type Error = UnknownOperation;

    fn try_from(value: ffi::pixman_op_t) -> Result<Self, Self::Error> {
        let format = match value {
            ffi::pixman_op_t_PIXMAN_OP_CLEAR => Operation::Clear,
            ffi::pixman_op_t_PIXMAN_OP_SRC => Operation::Src,
            ffi::pixman_op_t_PIXMAN_OP_DST => Operation::Dst,
            ffi::pixman_op_t_PIXMAN_OP_OVER => Operation::Over,
            ffi::pixman_op_t_PIXMAN_OP_OVER_REVERSE => Operation::OverReverse,
            ffi::pixman_op_t_PIXMAN_OP_IN => Operation::In,
            ffi::pixman_op_t_PIXMAN_OP_IN_REVERSE => Operation::InReverse,
            ffi::pixman_op_t_PIXMAN_OP_OUT => Operation::Out,
            ffi::pixman_op_t_PIXMAN_OP_OUT_REVERSE => Operation::OutReverse,
            ffi::pixman_op_t_PIXMAN_OP_ATOP => Operation::Atop,
            ffi::pixman_op_t_PIXMAN_OP_ATOP_REVERSE => Operation::AtopReverse,
            ffi::pixman_op_t_PIXMAN_OP_XOR => Operation::Xor,
            ffi::pixman_op_t_PIXMAN_OP_ADD => Operation::Add,
            ffi::pixman_op_t_PIXMAN_OP_SATURATE => Operation::Saturate,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_CLEAR => Operation::DisjointClear,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_SRC => Operation::DisjointSrc,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_DST => Operation::DisjointDst,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_OVER => Operation::DisjointOver,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_OVER_REVERSE => Operation::DisjointOverReverse,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_IN => Operation::DisjointIn,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_IN_REVERSE => Operation::DisjointInReverse,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_OUT => Operation::DisjointOut,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_OUT_REVERSE => Operation::DisjointOutReverse,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_ATOP => Operation::DisjointAtop,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_ATOP_REVERSE => Operation::DisjointAtopReverse,
            ffi::pixman_op_t_PIXMAN_OP_DISJOINT_XOR => Operation::DisjointXor,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_CLEAR => Operation::ConjointClear,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_SRC => Operation::ConjointSrc,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_DST => Operation::ConjointDst,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_OVER => Operation::ConjointOver,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_OVER_REVERSE => Operation::ConjointOverReverse,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_IN => Operation::ConjointIn,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_IN_REVERSE => Operation::ConjointInReverse,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_OUT => Operation::ConjointOut,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_OUT_REVERSE => Operation::ConjointOutReverse,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_ATOP => Operation::ConjointAtop,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_ATOP_REVERSE => Operation::ConjointAtopReverse,
            ffi::pixman_op_t_PIXMAN_OP_CONJOINT_XOR => Operation::ConjointXor,
            ffi::pixman_op_t_PIXMAN_OP_MULTIPLY => Operation::Multiply,
            ffi::pixman_op_t_PIXMAN_OP_SCREEN => Operation::Screen,
            ffi::pixman_op_t_PIXMAN_OP_OVERLAY => Operation::Overlay,
            ffi::pixman_op_t_PIXMAN_OP_DARKEN => Operation::Darken,
            ffi::pixman_op_t_PIXMAN_OP_LIGHTEN => Operation::Lighten,
            ffi::pixman_op_t_PIXMAN_OP_COLOR_DODGE => Operation::ColorDodge,
            ffi::pixman_op_t_PIXMAN_OP_COLOR_BURN => Operation::ColorBurn,
            ffi::pixman_op_t_PIXMAN_OP_HARD_LIGHT => Operation::HardLight,
            ffi::pixman_op_t_PIXMAN_OP_SOFT_LIGHT => Operation::SoftLight,
            ffi::pixman_op_t_PIXMAN_OP_DIFFERENCE => Operation::Difference,
            ffi::pixman_op_t_PIXMAN_OP_EXCLUSION => Operation::Exclustion,
            ffi::pixman_op_t_PIXMAN_OP_HSL_HUE => Operation::HslHue,
            ffi::pixman_op_t_PIXMAN_OP_HSL_SATURATION => Operation::HslSaturation,
            ffi::pixman_op_t_PIXMAN_OP_HSL_COLOR => Operation::HslColor,
            ffi::pixman_op_t_PIXMAN_OP_HSL_LUMINOSITY => Operation::HslLuminosity,
            _ => return Err(UnknownOperation(value)),
        };
        Ok(format)
    }
}

impl From<Operation> for ffi::pixman_op_t {
    fn from(value: Operation) -> Self {
        match value {
            Operation::Clear => ffi::pixman_op_t_PIXMAN_OP_CLEAR,
            Operation::Src => ffi::pixman_op_t_PIXMAN_OP_SRC,
            Operation::Dst => ffi::pixman_op_t_PIXMAN_OP_DST,
            Operation::Over => ffi::pixman_op_t_PIXMAN_OP_OVER,
            Operation::OverReverse => ffi::pixman_op_t_PIXMAN_OP_OVER_REVERSE,
            Operation::In => ffi::pixman_op_t_PIXMAN_OP_IN,
            Operation::InReverse => ffi::pixman_op_t_PIXMAN_OP_IN_REVERSE,
            Operation::Out => ffi::pixman_op_t_PIXMAN_OP_OUT,
            Operation::OutReverse => ffi::pixman_op_t_PIXMAN_OP_OUT_REVERSE,
            Operation::Atop => ffi::pixman_op_t_PIXMAN_OP_ATOP,
            Operation::AtopReverse => ffi::pixman_op_t_PIXMAN_OP_ATOP_REVERSE,
            Operation::Xor => ffi::pixman_op_t_PIXMAN_OP_XOR,
            Operation::Add => ffi::pixman_op_t_PIXMAN_OP_ADD,
            Operation::Saturate => ffi::pixman_op_t_PIXMAN_OP_SATURATE,
            Operation::DisjointClear => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_CLEAR,
            Operation::DisjointSrc => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_SRC,
            Operation::DisjointDst => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_DST,
            Operation::DisjointOver => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_OVER,
            Operation::DisjointOverReverse => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_OVER_REVERSE,
            Operation::DisjointIn => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_IN,
            Operation::DisjointInReverse => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_IN_REVERSE,
            Operation::DisjointOut => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_OUT,
            Operation::DisjointOutReverse => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_OUT_REVERSE,
            Operation::DisjointAtop => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_ATOP,
            Operation::DisjointAtopReverse => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_ATOP_REVERSE,
            Operation::DisjointXor => ffi::pixman_op_t_PIXMAN_OP_DISJOINT_XOR,
            Operation::ConjointClear => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_CLEAR,
            Operation::ConjointSrc => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_SRC,
            Operation::ConjointDst => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_DST,
            Operation::ConjointOver => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_OVER,
            Operation::ConjointOverReverse => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_OVER_REVERSE,
            Operation::ConjointIn => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_IN,
            Operation::ConjointInReverse => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_IN_REVERSE,
            Operation::ConjointOut => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_OUT,
            Operation::ConjointOutReverse => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_OUT_REVERSE,
            Operation::ConjointAtop => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_ATOP,
            Operation::ConjointAtopReverse => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_ATOP_REVERSE,
            Operation::ConjointXor => ffi::pixman_op_t_PIXMAN_OP_CONJOINT_XOR,
            Operation::Multiply => ffi::pixman_op_t_PIXMAN_OP_MULTIPLY,
            Operation::Screen => ffi::pixman_op_t_PIXMAN_OP_SCREEN,
            Operation::Overlay => ffi::pixman_op_t_PIXMAN_OP_OVERLAY,
            Operation::Darken => ffi::pixman_op_t_PIXMAN_OP_DARKEN,
            Operation::Lighten => ffi::pixman_op_t_PIXMAN_OP_LIGHTEN,
            Operation::ColorDodge => ffi::pixman_op_t_PIXMAN_OP_COLOR_DODGE,
            Operation::ColorBurn => ffi::pixman_op_t_PIXMAN_OP_COLOR_BURN,
            Operation::HardLight => ffi::pixman_op_t_PIXMAN_OP_HARD_LIGHT,
            Operation::SoftLight => ffi::pixman_op_t_PIXMAN_OP_SOFT_LIGHT,
            Operation::Difference => ffi::pixman_op_t_PIXMAN_OP_DIFFERENCE,
            Operation::Exclustion => ffi::pixman_op_t_PIXMAN_OP_EXCLUSION,
            Operation::HslHue => ffi::pixman_op_t_PIXMAN_OP_HSL_HUE,
            Operation::HslSaturation => ffi::pixman_op_t_PIXMAN_OP_HSL_SATURATION,
            Operation::HslColor => ffi::pixman_op_t_PIXMAN_OP_HSL_COLOR,
            Operation::HslLuminosity => ffi::pixman_op_t_PIXMAN_OP_HSL_LUMINOSITY,
        }
    }
}
