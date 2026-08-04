//! Bounds on how large a value an evaluation may produce.
//!
//! The strategy is to predict the size of a result and refuse before computing
//! it, rather than computing under a timeout: no threads, no interruption, and
//! a decision that is deterministic and instantaneous.

use crate::token::Number;
use anyhow::anyhow;

/// Resource bounds applied while evaluating an expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The largest value any intermediate or final result may occupy, in bits.
    pub max_value_bits: u64,
}

impl Default for Limits {
    /// 1 Mibit, roughly `315_000` decimal digits.
    fn default() -> Self {
        Limits {
            max_value_bits: 1 << 20,
        }
    }
}

/// The size of a value in bits: for a rational, numerator plus denominator.
#[must_use]
pub fn size_in_bits(value: &Number) -> u64 {
    match value {
        Number::NaturalNumber(v) => v.bits(),
        Number::DecimalNumber(v) => v.numer().bits() + v.denom().bits(),
    }
}

/// Rejects a value that has already been computed and turned out too large.
///
/// # Errors
/// When the value exceeds `limits.max_value_bits`.
pub fn check_size(value: &Number, limits: Limits) -> anyhow::Result<()> {
    check_predicted_size(u128::from(size_in_bits(value)), limits)
}

/// Rejects a computation whose result was predicted to be too large, before it runs.
///
/// # Errors
/// When `predicted_bits` exceeds `limits.max_value_bits`.
pub fn check_predicted_size(predicted_bits: u128, limits: Limits) -> anyhow::Result<()> {
    if predicted_bits > u128::from(limits.max_value_bits) {
        return Err(anyhow!(
            "Runtime error: the result would need about {predicted_bits} bits, over the size limit of {} bits.",
            limits.max_value_bits
        ));
    }
    Ok(())
}

/// Predicts the bit length of `n!` without computing it, via Stirling:
/// `log2(n!) ≈ n·log2(n) − 1.44·n`.
///
/// This is an estimate, not an exact count, so the precision lost converting `n`
/// to `f64` and the truncation converting the rounded-up estimate back to `u128`
/// are both intentional: `.max(1.0)` rules out a negative or sub-one result before
/// the cast, and a saturating cast is the correct outcome for an `n` so large that
/// the estimate would not fit anyway.
#[must_use]
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn predicted_factorial_bits(n: u64) -> u128 {
    if n < 2 {
        return 1;
    }
    let n_f = n as f64;
    // -1.442_695_040_888_963_4 is bit-for-bit -LOG2_E; clippy's approx_constant
    // lint (deny-by-default) requires the named constant instead of the literal.
    let bits = n_f.mul_add(n_f.log2(), -std::f64::consts::LOG2_E * n_f);
    // Round up and never report less than one bit.
    bits.max(1.0).ceil() as u128
}

/// An upper bound on the bit length of `base^exponent` for an integral exponent.
///
/// It uses `bits(base)` where `log2(base)` would be exact, so it overestimates by
/// up to a factor of two for small bases — `2^100` is predicted at 200 bits and
/// occupies 101. A guard that errs toward refusing is the right direction to err in,
/// and the discrepancy only matters within a factor of two of the budget.
#[must_use]
pub fn predicted_power_bits(base: &Number, exponent_magnitude: u64) -> u128 {
    let base_bits = size_in_bits(base).max(1);
    u128::from(base_bits) * u128::from(exponent_magnitude)
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    #[test]
    fn test_size_of_a_rational_counts_both_halves() {
        let third = Number::DecimalNumber(BigRational::new(BigInt::from(1), BigInt::from(3)));
        assert_eq!(size_in_bits(&third), 1 + 2);
    }

    #[test]
    fn test_factorial_prediction_is_in_the_right_ballpark() {
        // 1000! is 8529 bits; Stirling must land close and never far under.
        let predicted = predicted_factorial_bits(1000);
        assert!(
            (8000..=9200).contains(&predicted),
            "predicted {predicted} bits for 1000!"
        );
    }

    #[test]
    fn test_power_prediction_multiplies_base_size_by_exponent() {
        let ten = Number::NaturalNumber(BigInt::from(10));
        assert_eq!(predicted_power_bits(&ten, 100), 400);
    }

    #[test]
    fn test_check_rejects_above_the_budget_and_accepts_at_it() {
        let limits = Limits { max_value_bits: 64 };
        assert!(check_predicted_size(64, limits).is_ok());
        assert!(check_predicted_size(65, limits).is_err());
    }
}
