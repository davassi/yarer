//! Evaluation of the built-in mathematical functions, together with the
//! numeric conversions they rely on.
//!
//! Split out of [`crate::expression`], which owns the evaluation loop; this
//! module owns what happens when that loop meets a [`MathFunction`].

use crate::error::EvalError;
use crate::token::{MathFunction, Number};
use num::{Integer, Signed};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};
use statrs::distribution::{Continuous, ContinuousCDF, Normal};
use std::collections::VecDeque;

/// Evaluates `fun` against `value`, the operand already popped by the caller.
///
/// For a two-argument function that operand is the *second* argument, since the
/// caller popped the top of the stack and postfix order puts the second argument
/// there. What this function pops from `result_stack` is therefore the *first*
/// argument, keeping `var_stack` in step. Both stacks belong to the evaluation
/// loop in [`crate::expression::Expression::eval_with`].
pub(crate) fn eval(
    fun: MathFunction,
    value: Number,
    result_stack: &mut VecDeque<Number>,
    var_stack: &mut VecDeque<Option<String>>,
) -> Result<Number, EvalError> {
    let result = match fun {
        MathFunction::Sin => decimal_from_f64(
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.sin(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Cos => decimal_from_f64(
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.cos(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Tan => decimal_from_f64(
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.tan(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::ASin => decimal_from_f64(
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.asin(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::ACos => decimal_from_f64(
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.acos(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::ATan => decimal_from_f64(
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.atan(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Ln => decimal_from_f64(
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.ln(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::Log => decimal_from_f64(
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.log10(),
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
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.sqrt(),
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
                normal.pdf(number_to_f64(
                    &value,
                    EvalError::OperandTooLargeForFloat { span: None },
                )?),
                EvalError::NotARealNumber { span: None },
            )?
        }
        MathFunction::Cdf => {
            let normal = Normal::new(0.0, 1.0).expect("valid normal dist");
            decimal_from_f64(
                normal.cdf(number_to_f64(
                    &value,
                    EvalError::OperandTooLargeForFloat { span: None },
                )?),
                EvalError::NotARealNumber { span: None },
            )?
        }
        MathFunction::Exp => decimal_from_f64(
            number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?.exp(),
            EvalError::NotARealNumber { span: None },
        )?,
        MathFunction::None => return Err(EvalError::Malformed { span: None }),
    };
    Ok(result)
}

pub(crate) fn number_to_f64(value: &Number, on_error: EvalError) -> Result<f64, EvalError> {
    match value {
        Number::NaturalNumber(v) => v.to_f64().ok_or(on_error),
        Number::DecimalNumber(v) => v.to_f64().ok_or(on_error),
    }
}

pub(crate) fn decimal_from_f64(value: f64, on_error: EvalError) -> Result<Number, EvalError> {
    if !value.is_finite() {
        return Err(on_error);
    }

    BigRational::from_float(value)
        .map(Number::decimal)
        .ok_or(on_error)
}

pub(crate) fn number_to_rational(value: Number) -> BigRational {
    match value {
        Number::NaturalNumber(v) => BigRational::from_integer(v),
        Number::DecimalNumber(v) => v,
    }
}
