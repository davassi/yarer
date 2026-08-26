//! Bounds on how large a value an evaluation may produce.
//!
//! Two checks, with different jobs. **Predict to avoid the work, verify to be
//! correct.**
//!
//! `check_predicted_size` runs before an expensive computation and refuses it
//! on an estimate, which is what lets `999999999!` be declined in milliseconds
//! instead of running until it exhausts memory. That is an optimisation, and an
//! estimate is all it can be: it is beaten by any path that has no prediction
//! (`2^0.5` goes through `f64`) and by any prediction that measures something
//! other than the value finally returned (`2^-1` predicts the magnitude of
//! `2^1` and returns its reciprocal).
//!
//! `check_size` runs on the value that was actually built, and is the
//! guarantee. It is exact by construction, because it measures rather than
//! estimates. Predictions may be tightened or added freely; correctness does not
//! rest on them.
//!
//! Refusing rather than computing under a timeout keeps the decision
//! deterministic and instantaneous: no threads, no interruption.

use crate::error::EvalError;
use crate::token::Number;
use num_bigint::BigUint;
use num_traits::ToPrimitive;

/// Resource bounds applied while evaluating an expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits {
    /// The largest value any intermediate or final result may occupy, in bits.
    /// Every value pushed onto the evaluation stack is measured against it,
    /// whatever produced it — literal, variable, arithmetic result or function
    /// result — with no exceptions.
    ///
    /// This bounds memory directly and worst-case running time only indirectly,
    /// and the second relationship is superlinear, so raise it with the measured
    /// ratios below in mind rather than in the expectation that time scales with
    /// it.
    ///
    /// A factorial is a loop of `n` bignum multiplications, so quadrupling this
    /// budget costs roughly twelve times the worst-case factorial time: at the
    /// 1 Mibit default the largest factorial admitted is `71421!`, about 0.43 s
    /// in a release build. That is no longer the worst case, though. A base of
    /// magnitude 1 short-circuits the power prediction — `1^n` is `1` under any
    /// limit — so it goes on to run a repeated-squaring loop over every bit of
    /// `n`, and `n` is bounded only by this budget applied to the literal. At
    /// the default the largest such exponent is 315,652 digits, and evaluating
    /// it takes about 1.57 s.
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

impl Limits {
    /// The same limits with a different size budget.
    ///
    /// ```
    /// # use yarer::Limits;
    /// let tight = Limits::default().with_max_value_bits(4096);
    /// ```
    #[must_use]
    pub fn with_max_value_bits(mut self, bits: u64) -> Limits {
        self.max_value_bits = bits;
        self
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

/// Rejects a value that has already been computed and turned out too large.
///
/// This is the budget's guarantee: it measures the value in hand, so it holds
/// regardless of whether a prediction ran first, or ran accurately. Every path
/// that produces a value applies it.
///
/// # Errors
/// When the value exceeds `limits.max_value_bits`.
pub(crate) fn check_size(value: &Number, limits: Limits) -> Result<(), EvalError> {
    let bits = u128::from(size_in_bits(value));
    if bits > u128::from(limits.max_value_bits) {
        return Err(EvalError::ValueTooLarge {
            bits,
            limit: limits.max_value_bits,
            span: None,
        });
    }
    Ok(())
}

/// Rejects a computation whose result was predicted to be too large, before it runs.
///
/// A pre-filter, not the guarantee — see the module docs. Its purpose is to make
/// a hopeless computation cheap to refuse; being approximate is acceptable
/// because [`check_size`] measures the result afterwards either way.
///
/// # Errors
/// When `predicted_bits` exceeds `limits.max_value_bits`.
pub(crate) fn check_predicted_size(predicted_bits: u128, limits: Limits) -> Result<(), EvalError> {
    if predicted_bits > u128::from(limits.max_value_bits) {
        return Err(EvalError::ComputationTooLarge {
            predicted_bits,
            limit: limits.max_value_bits,
            span: None,
        });
    }
    Ok(())
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
/// That exposure is a performance detail rather than a correctness one, because
/// this is a pre-filter and [`check_size`] measures the factorial afterwards. An
/// over-estimate refuses a computation that would have fitted, by about a bit; an
/// under-estimate lets one start that is then refused on measurement. Neither can
/// admit an oversized value. `n = 2` is the only under-estimate for any `n` up to
/// 60000, verified by computing `n!` and comparing bit lengths.
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

/// An estimate of the bit length of `base` raised to the *magnitude* of an
/// integral exponent, or [`None`] when that magnitude is too large for the
/// estimate to be made at all.
///
/// Note the "magnitude": a negative exponent returns the reciprocal, and
/// [`size_in_bits`] counts a rational's denominator as well as its numerator, so
/// this is **not** an upper bound on what the caller ends up holding. `2^-1`
/// estimates 2 bits and produces `1/2`, which measures `1 + 2 = 3`. Correcting
/// that here would only narrow the gap, not close it, since the `powf` path has
/// no prediction at all; [`check_size`] on the finished value is what makes the
/// budget exact, and this stays a pre-filter.
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
/// at 200 bits and occupies 101. For a pre-filter, erring toward refusing is the
/// right direction to err in: the cost is a computation declined that would have
/// fitted, within a factor of two of the budget, and never an oversized value
/// admitted.
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
    fn test_power_prediction_is_not_an_upper_bound_for_a_negative_exponent() {
        // The prediction is made on the magnitude, so it describes 2^1 and not
        // the 1/2 a negative exponent actually returns. Pinning the gap here
        // documents why check_size on the finished value is load-bearing rather
        // than belt-and-braces: at a 2-bit budget the prediction admits, and the
        // value measures 3.
        let two = Number::NaturalNumber(BigInt::from(2));
        assert_eq!(predicted_power_bits(&two, &BigUint::from(1_u32)), Some(2));
        let half = Number::DecimalNumber(BigRational::new(BigInt::from(1), BigInt::from(2)));
        assert_eq!(size_in_bits(&half), 3);
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
    fn test_the_two_checks_report_two_different_conditions() {
        let limits = Limits { max_value_bits: 64 };
        assert!(check_predicted_size(64, limits).is_ok());
        assert!(matches!(
            check_predicted_size(65, limits),
            Err(EvalError::ComputationTooLarge {
                predicted_bits: 65,
                limit: 64,
                ..
            })
        ));

        let big = Number::NaturalNumber(BigInt::from(1u8) << 65);
        assert!(matches!(
            check_size(&big, limits),
            Err(EvalError::ValueTooLarge { limit: 64, .. })
        ));
    }
}
