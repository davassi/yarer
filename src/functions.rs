//! Evaluation of the built-in mathematical functions, together with the
//! numeric conversions they rely on.
//!
//! Split out of [`crate::rpn_resolver`], which owns the shunting-yard
//! translation and the evaluation loop; this module owns what happens when
//! that loop meets a [`MathFunction`].

use crate::rpn_resolver::{FLOAT_EVAL_TOO_LARGE_ERR, INVALID_FUNCTION_RESULT_ERR, MALFORMED_ERR};
use crate::token::{MathFunction, Number};
use anyhow::anyhow;
use num::{Integer, Signed};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};
use statrs::distribution::{Continuous, ContinuousCDF, Normal};
use std::collections::VecDeque;

/// Evaluates `fun` against `value`, the operand already popped by the caller.
///
/// A two-argument function pops its second operand from `result_stack` itself,
/// keeping `var_stack` in step. Both stacks belong to the evaluation loop in
/// [`crate::rpn_resolver::RpnResolver::resolve`].
pub(crate) fn eval(
    fun: MathFunction,
    value: Number,
    result_stack: &mut VecDeque<Number>,
    var_stack: &mut VecDeque<Option<String>>,
) -> anyhow::Result<Number> {
    let result = match fun {
        MathFunction::Sin => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.sin(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::Cos => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.cos(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::Tan => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.tan(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::ASin => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.asin(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::ACos => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.acos(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::ATan => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.atan(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::Ln => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.ln(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::Log => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.log10(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::Abs => to_decimal_number(match value {
            Number::NaturalNumber(v) => Number::NaturalNumber(v.abs()),
            Number::DecimalNumber(v) => Number::DecimalNumber(v.abs()),
        }),
        MathFunction::Max => {
            let value2: Number = result_stack.pop_back().ok_or(anyhow!(
                "{} {}",
                MALFORMED_ERR,
                "Wrong number of parameters for function Max"
            ))?;
            var_stack.pop_back();
            to_decimal_number(if value >= value2 { value } else { value2 })
        }
        MathFunction::Min => {
            let value2: Number = result_stack.pop_back().ok_or(anyhow!(
                "{} {}",
                MALFORMED_ERR,
                "Wrong number of parameters for function Min"
            ))?;
            var_stack.pop_back();
            to_decimal_number(if value <= value2 { value } else { value2 })
        }
        MathFunction::Sqrt => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.sqrt(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::Floor => {
            let value = number_to_rational(value);
            to_decimal_number(Number::NaturalNumber(
                value.numer().div_floor(value.denom()),
            ))
        }
        MathFunction::Ceil => {
            let value = number_to_rational(value);
            to_decimal_number(Number::NaturalNumber(value.numer().div_ceil(value.denom())))
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
            to_decimal_number(Number::NaturalNumber(rounded))
        }
        MathFunction::Pdf => {
            let normal = Normal::new(0.0, 1.0).expect("valid normal dist");
            decimal_from_f64(
                normal.pdf(number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?),
                INVALID_FUNCTION_RESULT_ERR,
            )?
        }
        MathFunction::Cdf => {
            let normal = Normal::new(0.0, 1.0).expect("valid normal dist");
            decimal_from_f64(
                normal.cdf(number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?),
                INVALID_FUNCTION_RESULT_ERR,
            )?
        }
        MathFunction::Exp => decimal_from_f64(
            number_to_f64(&value, FLOAT_EVAL_TOO_LARGE_ERR)?.exp(),
            INVALID_FUNCTION_RESULT_ERR,
        )?,
        MathFunction::None => return Err(anyhow!("This should never happen!")),
    };
    Ok(result)
}

pub(crate) fn number_to_f64(value: &Number, error_message: &'static str) -> anyhow::Result<f64> {
    match value {
        Number::NaturalNumber(v) => v.to_f64().ok_or_else(|| anyhow!(error_message)),
        Number::DecimalNumber(v) => v.to_f64().ok_or_else(|| anyhow!(error_message)),
    }
}

pub(crate) fn decimal_from_f64(value: f64, error_message: &'static str) -> anyhow::Result<Number> {
    if !value.is_finite() {
        return Err(anyhow!(error_message));
    }

    BigRational::from_float(value)
        .map(Number::DecimalNumber)
        .ok_or_else(|| anyhow!(error_message))
}

pub(crate) fn number_to_rational(value: Number) -> BigRational {
    match value {
        Number::NaturalNumber(v) => BigRational::from_integer(v),
        Number::DecimalNumber(v) => v,
    }
}

pub(crate) fn to_decimal_number(value: Number) -> Number {
    match value {
        Number::NaturalNumber(v) => Number::DecimalNumber(BigRational::from_integer(v)),
        Number::DecimalNumber(v) => Number::DecimalNumber(v),
    }
}
