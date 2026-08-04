//! Bounds on how large a value an evaluation may produce.
//!
//! The strategy is to predict the size of a result and refuse before computing
//! it, rather than computing under a timeout: no threads, no interruption, and
//! a decision that is deterministic and instantaneous.

use crate::token::Number;
use anyhow::anyhow;
use num_bigint::BigUint;
use num_traits::ToPrimitive;

/// Resource bounds applied while evaluating an expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The largest value any intermediate or final result may occupy, in bits.
    ///
    /// This bounds memory directly and worst-case running time only indirectly,
    /// and the second relationship is superlinear: a factorial is a loop of `n`
    /// bignum multiplications, so quadrupling this budget costs roughly twelve
    /// times the worst-case factorial time. At the 1 Mibit default the largest
    /// factorial admitted is `71421!`, measured at about 0.43 s in a release
    /// build. Raise the budget with that ratio in mind rather than in the
    /// expectation that time scales with it.
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
pub(crate) fn size_in_bits(value: &Number) -> u64 {
    match value {
        Number::NaturalNumber(v) => v.bits(),
        Number::DecimalNumber(v) => v.numer().bits() + v.denom().bits(),
    }
}

/// Compares `bits` against the budget, wording the error for whichever of the two
/// callers below is asking: a value already computed occupies a size, while a
/// computation not yet run only has a predicted one.
fn check_bits(bits: u128, limits: Limits, phrase: &str) -> anyhow::Result<()> {
    if bits > u128::from(limits.max_value_bits) {
        return Err(anyhow!(
            "Runtime error: the result {phrase} about {bits} bits, over the size limit of {} bits.",
            limits.max_value_bits
        ));
    }
    Ok(())
}

/// Rejects a value that has already been computed and turned out too large.
///
/// # Errors
/// When the value exceeds `limits.max_value_bits`.
pub(crate) fn check_size(value: &Number, limits: Limits) -> anyhow::Result<()> {
    check_bits(u128::from(size_in_bits(value)), limits, "occupies")
}

/// Rejects a computation whose result was predicted to be too large, before it runs.
///
/// # Errors
/// When `predicted_bits` exceeds `limits.max_value_bits`.
pub(crate) fn check_predicted_size(predicted_bits: u128, limits: Limits) -> anyhow::Result<()> {
    check_bits(predicted_bits, limits, "would need")
}

/// Predicts the bit length of `n!` without computing it, via Stirling's series:
/// `log2(n!) ≈ n·log2(n) − n·log2(e) + 0.5·log2(2πn)`.
///
/// The first two terms alone are optimistic — they omit the `0.5·log2(2πn)` term,
/// which for `n` in the hundreds of thousands is close to ten bits, enough to let a
/// prediction land just under a tight budget while the true value lands just over
/// it. With the correction included this matches `lgamma(n+1)/ln(2)` to within about
/// a bit, a large improvement, but not an exact lower bound: the series is still
/// asymptotic, and `.ceil()` of a value just below an integer can still round to one
/// bit less than the true bit count — at `n = 2` the raw estimate is `0.94`, which
/// ceils to `1`, while `2!` needs `2` bits. So the remaining exposure is about a bit,
/// not zero, in either direction.
///
/// This is still an estimate, not an exact count, so the precision lost converting
/// `n` to `f64` and the truncation converting the rounded-up estimate back to
/// `u128` are both intentional: `.max(1.0)` rules out a negative or sub-one result
/// before the cast, and a saturating cast is the correct outcome for an `n` so
/// large that the estimate would not fit anyway.
#[must_use]
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn predicted_factorial_bits(n: u64) -> u128 {
    if n < 2 {
        return 1;
    }
    let n_f = n as f64;
    // -1.442_695_040_888_963_4 is bit-for-bit -LOG2_E; clippy's approx_constant
    // lint (deny-by-default) requires the named constant instead of the literal.
    let leading_terms = n_f.mul_add(n_f.log2(), -std::f64::consts::LOG2_E * n_f);
    // 0.5 * log2(2 * pi * n); TAU is the named constant for 2*pi.
    let correction = 0.5 * (std::f64::consts::TAU * n_f).log2();
    let bits = leading_terms + correction;
    // Round up and never report less than one bit.
    bits.max(1.0).ceil() as u128
}

/// An upper bound on the bit length of `base^exponent` for an integral exponent,
/// or [`None`] when the exponent is too large for the prediction to be made at all.
///
/// Degenerate bases are special-cased first: when `base`'s magnitude is 0 or 1, so
/// is the magnitude of any power of it, regardless of how large the exponent is, so
/// the exponent is irrelevant and no prediction is needed. Without this, `base_bits`
/// would be clamped up to 1 by `.max(1)` and multiplied by the exponent, turning a
/// computation that is actually free — `1^10000000` is `1`, computed instantly — into
/// one refused for needing ten million bits.
///
/// That test deliberately comes *before* the exponent is narrowed to a `u64`, which
/// is why the narrowing lives here rather than at the call site: an exponent too
/// large to narrow is still irrelevant to a base of magnitude 1, and answering
/// [`None`] for it would make the caller report an exponent as unevaluable when
/// `1^n` is `1` under every conceivable limit.
///
/// For every other base it uses `bits(base)` where `log2(base)` would be exact, so
/// it overestimates by up to a factor of two for small bases — `2^100` is predicted
/// at 200 bits and occupies 101. A guard that errs toward refusing is the right
/// direction to err in, and the discrepancy only matters within a factor of two of
/// the budget.
#[must_use]
pub(crate) fn predicted_power_bits(base: &Number, exponent_magnitude: &BigUint) -> Option<u128> {
    let base_bits = size_in_bits(base);
    if base_bits <= 1 {
        return Some(1);
    }
    Some(u128::from(base_bits) * u128::from(exponent_magnitude.to_u64()?))
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
    fn test_factorial_prediction_matches_stirling_with_correction() {
        // 1000! is 8529 bits (lgamma(1001)/ln(2) = 8529.4); the three-term series
        // used here predicts 8530. A wide "ballpark" range does not pin this down:
        // dropping the 0.5*log2(2*pi*n) correction term still predicts 8524, which
        // is inside almost any range wide enough to be useful, so only an exact
        // value catches the correction term going missing.
        assert_eq!(predicted_factorial_bits(1000), 8530);
    }

    #[test]
    fn test_power_prediction_multiplies_base_size_by_exponent() {
        let ten = Number::NaturalNumber(BigInt::from(10));
        assert_eq!(
            predicted_power_bits(&ten, &BigUint::from(100_u32)),
            Some(400)
        );
    }

    #[test]
    fn test_power_prediction_ignores_the_exponent_for_degenerate_bases() {
        // 1^n, 0^n and (-1)^n all have magnitude 0 or 1 no matter how large n is,
        // so a huge exponent must not inflate the prediction: base_bits.max(1) *
        // exponent_magnitude would otherwise turn a free computation into a refusal.
        // The second exponent does not fit in a u64, which pins down that the
        // degenerate test runs before the narrowing rather than after it.
        let beyond_u64 = BigUint::from(u64::MAX) + BigUint::from(1_u32);
        for base in [
            Number::NaturalNumber(BigInt::from(1)),
            Number::NaturalNumber(BigInt::from(0)),
            Number::NaturalNumber(BigInt::from(-1)),
        ] {
            assert_eq!(
                predicted_power_bits(&base, &BigUint::from(10_000_000_u32)),
                Some(1)
            );
            assert_eq!(predicted_power_bits(&base, &beyond_u64), Some(1));
        }
    }

    #[test]
    fn test_power_prediction_declines_an_exponent_that_does_not_fit() {
        // A base that actually grows plus an exponent beyond u64 has no usable
        // prediction: the caller must refuse rather than guess.
        let two = Number::NaturalNumber(BigInt::from(2));
        let beyond_u64 = BigUint::from(u64::MAX) + BigUint::from(1_u32);
        assert_eq!(predicted_power_bits(&two, &beyond_u64), None);
    }

    #[test]
    fn test_check_rejects_above_the_budget_and_accepts_at_it() {
        let limits = Limits { max_value_bits: 64 };
        assert!(check_predicted_size(64, limits).is_ok());
        assert!(check_predicted_size(65, limits).is_err());
    }
}
