//! Evaluation of the built-in mathematical functions, together with the
//! numeric conversions they rely on.
//!
//! Split out of [`crate::expression`], which owns the evaluation loop; this
//! module owns what happens when that loop meets a [`MathFunction`].

use crate::error::EvalError;
use crate::token::{narrow_to_f64, MathFunction, Narrowing, Number};
use num::{Integer, Signed};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;
use statrs::distribution::{Continuous, ContinuousCDF, Normal};
use std::collections::VecDeque;

/// Evaluates `fun` against `value`, the operand already popped by the caller.
///
/// For a two-argument function that operand is the *second* argument, since the
/// caller popped the top of the stack and postfix order puts the second argument
/// there. What this function pops from `result_stack` is therefore the *first*
/// argument, keeping `var_stack` in step. Both stacks belong to the evaluation
/// loop in [`crate::Expression::eval_with`].
pub(crate) fn eval(
    fun: MathFunction,
    value: Number,
    result_stack: &mut VecDeque<Number>,
    var_stack: &mut VecDeque<Option<String>>,
) -> Result<Number, EvalError> {
    let result = match fun {
        MathFunction::Sin => decimal_from_f64(
            number_to_f64(&value)?.sin(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Cos => decimal_from_f64(
            number_to_f64(&value)?.cos(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Tan => decimal_from_f64(
            number_to_f64(&value)?.tan(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::ASin => decimal_from_f64(
            number_to_f64(&value)?.asin(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::ACos => decimal_from_f64(
            number_to_f64(&value)?.acos(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::ATan => decimal_from_f64(
            number_to_f64(&value)?.atan(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Ln => decimal_from_f64(
            number_to_f64(&value)?.ln(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Log => decimal_from_f64(
            number_to_f64(&value)?.log10(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Abs => match value {
            Number::NaturalNumber(v) => Number::NaturalNumber(v.abs()),
            Number::DecimalNumber(v) => Number::decimal(v.abs()),
        },
        MathFunction::Max => {
            let value2: Number = result_stack
                .pop_back()
                .ok_or(EvalError::Malformed { span: None })?;
            var_stack.pop_back();
            if value >= value2 {
                value
            } else {
                value2
            }
        }
        MathFunction::Min => {
            let value2: Number = result_stack
                .pop_back()
                .ok_or(EvalError::Malformed { span: None })?;
            var_stack.pop_back();
            if value <= value2 {
                value
            } else {
                value2
            }
        }
        MathFunction::Sqrt => decimal_from_f64(
            number_to_f64(&value)?.sqrt(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Floor => {
            let value = number_to_rational(value);
            Number::NaturalNumber(value.numer().div_floor(value.denom()))
        }
        MathFunction::Ceil => {
            let value = number_to_rational(value);
            Number::NaturalNumber(value.numer().div_ceil(value.denom()))
        }
        MathFunction::Round => {
            let value = number_to_rational(value);
            let denom = value.denom().clone();
            let doubled_numer = value.numer().clone() * BigInt::from(2_u8);
            let doubled_denom = denom.clone() * BigInt::from(2_u8);
            let rounded = if doubled_numer >= BigInt::zero() {
                (doubled_numer + denom).div_floor(&doubled_denom)
            } else {
                (doubled_numer - denom).div_ceil(&doubled_denom)
            };
            Number::NaturalNumber(rounded)
        }
        MathFunction::Pdf => {
            let normal = Normal::new(0.0, 1.0).expect("valid normal dist");
            decimal_from_f64(
                normal.pdf(number_to_f64(&value)?),
                EvalError::NotARealNumber { span: None },
            )?
        }
        MathFunction::Cdf => {
            let normal = Normal::new(0.0, 1.0).expect("valid normal dist");
            decimal_from_f64(
                normal.cdf(number_to_f64(&value)?),
                EvalError::NotARealNumber { span: None },
            )?
        }
        MathFunction::Exp => decimal_from_f64(
            number_to_f64(&value)?.exp(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::None => return Err(EvalError::Malformed { span: None }),
    };
    Ok(result)
}

/// Narrows an operand to an [`f64`] for the built-ins that are defined in terms
/// of one, answering the error that says which end of the range it fell off.
///
/// `BigInt::to_f64` and `BigRational::to_f64` answer `Some(±inf)` on overflow
/// and `Some(0.0)` on underflow, never `None`, so both losses arrive looking
/// like successes. Stage 2 caught the overflow half: before it, the infinity
/// flowed on and was caught downstream by [`decimal_from_f64`]'s own finiteness
/// test under a different name, and `sqrt(2^5000)` reported "function result is
/// not a real number" about a number that is perfectly real.
///
/// The underflow half was missed, and cost more, because a zeroed operand is
/// not obviously wrong. A function that shrinks toward its input does not care
/// — `sin x ≈ x` — but one that expands small values is wrecked: `log(1/(10^400))`
/// is exactly -400 and was refused as not a real number, and `sqrt(1/(10^400))`
/// is 1e-200 and answered 0.
///
/// The `on_error` parameter this used to take was passed
/// [`EvalError::OperandTooLargeForFloat`] at all twelve call sites and nothing
/// else was ever possible — which is precisely why the underflow case could not
/// be reported. Choosing the variant is this function's job now, and
/// [`narrow_to_f64`] is what tells it which.
///
/// # Errors
/// [`EvalError::OperandTooLargeForFloat`] or
/// [`EvalError::OperandTooSmallForFloat`].
pub(crate) fn number_to_f64(value: &Number) -> Result<f64, EvalError> {
    narrow_to_f64(value).map_err(|why| match why {
        Narrowing::TooLarge => EvalError::OperandTooLargeForFloat { span: None },
        Narrowing::TooSmall => EvalError::OperandTooSmallForFloat { span: None },
    })
}

pub(crate) fn decimal_from_f64(value: f64, on_error: EvalError) -> Result<Number, EvalError> {
    if !value.is_finite() {
        return Err(on_error);
    }

    // `BigRational::from_float` builds its result through `Ratio::new`
    // (or `Ratio::from_integer` when the exponent is non-negative), both of
    // which reduce before returning, so the value is already reduced.
    BigRational::from_float(value)
        .map(Number::decimal_unchecked)
        .ok_or(on_error)
}

pub(crate) fn number_to_rational(value: Number) -> BigRational {
    match value {
        Number::NaturalNumber(v) => BigRational::from_integer(v),
        Number::DecimalNumber(v) => v,
    }
}
