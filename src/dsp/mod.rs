//! Fixed-point spectral decoding and excitation generation.
mod effects;
mod filter;
mod schedule;
mod tables;

pub(crate) use effects::Effects;
pub(crate) use schedule::{FramePolicy, Hold, schedule};

pub(crate) use filter::{coefficients_with_order, excitation, noise, shift_spectrum};

/// Round halves away from zero, as required by the normalization and window stages.
pub(crate) fn rounded_div(numerator: i64, denominator: i64) -> i64 {
    if denominator == 0 {
        return 0;
    }
    let quotient = (numerator.abs() + denominator.abs() / 2) / denominator.abs();
    if (numerator >= 0) == (denominator >= 0) {
        quotient
    } else {
        -quotient
    }
}

/// Pitch interpolation uses a different half-negative rule and saturates to 15 bits.
pub(crate) fn pitch_div(numerator: i32, denominator: i32) -> i32 {
    if denominator <= 0 {
        return 0;
    }
    let bias = if numerator >= 0 {
        denominator / 2
    } else {
        (denominator - 1) / 2
    };
    let magnitude =
        ((i64::from(numerator).abs() + i64::from(bias)) / i64::from(denominator)).min(32767) as i32;
    if numerator >= 0 {
        magnitude
    } else {
        -magnitude
    }
}
