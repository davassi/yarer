//! Compiled expressions, and the loop that evaluates them.

use crate::error::{EvalError, ParseError};
use crate::functions::{decimal_from_f64, number_to_f64};
use crate::limits::{self, Limits};
use crate::{
    functions,
    parser::Parser,
    session::Session,
    shunting,
    span::Spanned,
    token::{Number, Operator, Token},
    validate,
};
use log::debug;
use num::Integer;
use std::collections::VecDeque;

use num::{BigInt, BigUint, One, Zero};
use num_rational::BigRational;
use num_traits::ToPrimitive;

/// A compiled expression: the token sequence in postfix order, ready to be
/// evaluated as often as wanted, against any [`Session`].
///
/// The lifetime is the source text's. Compilation is a pure function of that
/// text — it consults no session and touches no variable heap — so one
/// `Expression` can be evaluated against several sessions, and under several
/// budgets.
///
/// `Expression` is [`Clone`], and cloning copies the whole compiled token
/// sequence — worth knowing before reaching for it inside a loop.
#[derive(Debug, Clone)]
pub struct Expression<'a> {
    rpn: VecDeque<Spanned<Token<'a>>>,
}

impl<'a> Expression<'a> {
    /// Compiles `source` into an expression.
    ///
    /// # Errors
    /// Any [`ParseError`]: the text does not tokenise, the token sequence is
    /// not a well-formed expression, or the brackets do not balance.
    pub fn compile(source: &'a str) -> Result<Self, ParseError> {
        let tokens = Parser::parse(source)?;
        let validated = validate::validate(&tokens, source)?;
        Ok(Self {
            rpn: shunting::to_rpn(&validated)?,
        })
    }

    /// Evaluates against `session`, under the session's own limits.
    ///
    /// # Errors
    /// Any [`EvalError`]: a division by zero, a value over the size budget, an
    /// assignment to a built-in constant, and so on.
    pub fn eval(&self, session: &Session) -> Result<Number, EvalError> {
        self.eval_with(session, session.limits())
    }

    /// Evaluates against `session`, under `limits` instead of the session's.
    ///
    /// Use this to run untrusted input under a tighter budget than trusted
    /// input, against the same variables. Mind the floor the built-in constants
    /// impose: `pi`, `e`, `tau`, `phi` and `gamma` are `f64`s held exactly as
    /// rationals and cost up to 107 bits, so a budget below that refuses a
    /// value the caller never supplied.
    ///
    /// # Errors
    /// As [`Expression::eval`].
    pub fn eval_with(&self, session: &Session, limits: Limits) -> Result<Number, EvalError> {
        let zero: Number = Number::NaturalNumber(Zero::zero());
        let minus_one: Number = Number::NaturalNumber(BigInt::from(-1));

        let mut result_stack: VecDeque<Number> = VecDeque::new();
        let mut var_stack: VecDeque<Option<String>> = VecDeque::new();
        let mut last_result: Option<Number> = None;

        for t in &self.rpn {
            // Errors raised inside `limits.rs` and `functions.rs` know nothing
            // of positions — this closure is how the loop stamps them with the
            // token it was holding when it called out.
            let at = |e: EvalError| e.at(t.span);

            match &t.node {
                Token::Operand(n) => {
                    // The budget is documented as bounding every intermediate and
                    // final result, so it has to hold for a value that arrives as a
                    // literal too — otherwise a long enough literal is returned
                    // above the limit the caller asked for.
                    limits::check_size(n, limits).map_err(at)?;
                    result_stack.push_back(n.clone());
                    var_stack.push_back(None);
                }
                Token::Operator(op) => {
                    let right_value: Number = result_stack
                        .pop_back()
                        .ok_or(EvalError::Malformed { span: Some(t.span) })?;

                    var_stack.pop_back();

                    let left_value = if op.is_unary() {
                        zero.clone()
                    } else {
                        result_stack
                            .pop_back()
                            .ok_or(EvalError::Malformed { span: Some(t.span) })?
                    };
                    let left_var = if op.is_unary() {
                        None
                    } else {
                        var_stack.pop_back().unwrap_or(None)
                    };

                    match op {
                        Operator::Add => {
                            let value = left_value + right_value;
                            limits::check_size(&value, limits).map_err(at)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Sub => {
                            let value = left_value - right_value;
                            limits::check_size(&value, limits).map_err(at)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Mul => {
                            let value = left_value * right_value;
                            limits::check_size(&value, limits).map_err(at)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Div => {
                            let value = left_value
                                .checked_div(&right_value)
                                .ok_or(EvalError::DivisionByZero { span: Some(t.span) })?;
                            limits::check_size(&value, limits).map_err(at)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Pow => {
                            result_stack.push_back(
                                Self::power(left_value, right_value, limits).map_err(at)?,
                            );
                            var_stack.push_back(None);
                        }
                        Operator::Assign => {
                            if let Some(var) = left_var {
                                // `assign` decides the refusal, here and for
                                // `set`/`setf` alike; the loop only supplies the
                                // position the refusal happened at.
                                session.assign(&var, right_value.clone()).map_err(at)?;

                                result_stack.push_back(right_value);
                                var_stack.push_back(None);
                            } else {
                                return Err(EvalError::AssignmentTargetMissing {
                                    span: Some(t.span),
                                });
                            }
                        }
                        Operator::Fac => {
                            // Factorial is defined on non-negative integers. It asks the
                            // value, not the enum tag: floor(2.5) and 6/3 are integers.
                            let n = right_value
                                .as_integer()
                                .ok_or(EvalError::FactorialNotNatural { span: Some(t.span) })?;
                            if n < BigInt::zero() {
                                return Err(EvalError::FactorialNotNatural { span: Some(t.span) });
                            }
                            let n = n.to_u64().ok_or(EvalError::FactorialOperandTooLarge {
                                span: Some(t.span),
                            })?;
                            // Predict first, to refuse `999999999!` in
                            // milliseconds rather than computing it...
                            limits::check_predicted_size(
                                limits::predicted_factorial_bits(n),
                                limits,
                            )
                            .map_err(at)?;
                            let res = Self::factorial_helper(n.into());
                            // ...then measure what was actually built, because
                            // the prediction is an asymptotic series rounded up
                            // and is a bit short of the truth at `n = 2`. The
                            // prediction buys the speed; this buys the exactness.
                            let value = Number::NaturalNumber(res.into());
                            limits::check_size(&value, limits).map_err(at)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Une => {
                            //# unary neg
                            result_stack.push_back(right_value * minus_one.clone());
                            var_stack.push_back(None);
                        }
                        Operator::Less
                        | Operator::Greater
                        | Operator::LessEq
                        | Operator::GreaterEq
                        | Operator::Equal
                        | Operator::NotEqual
                        | Operator::And
                        | Operator::Or
                        | Operator::Xor
                        | Operator::Not
                        | Operator::Mod => {
                            // Unreachable: the tokeniser cannot yet spell any of
                            // these, so no expression compiles to one. The tasks
                            // that follow replace this arm with the real ones;
                            // it exists so that this one compiles on its own.
                            return Err(EvalError::Malformed { span: Some(t.span) });
                        }
                    }
                }
                Token::Variable(v) => {
                    let var_name = v.to_lowercase();
                    // An undefined variable reads as zero, deliberately.
                    let n = session
                        .lookup(&var_name)
                        .unwrap_or_else(|| Number::NaturalNumber(BigInt::zero()));
                    debug!("Variable '{var_name}' read as {n:?}");
                    // Same reasoning as the operand arm above: a variable is a
                    // value on the stack like any other. `set`/`setf` put values
                    // into the heap without passing through any checked operator,
                    // so without this an expression that only reads a variable
                    // returns it however large it is.
                    limits::check_size(&n, limits).map_err(at)?;
                    result_stack.push_back(n);
                    var_stack.push_back(Some(var_name));
                }
                Token::Function(fun) => {
                    let value: Number = result_stack
                        .pop_back()
                        .ok_or(EvalError::Malformed { span: Some(t.span) })?;
                    var_stack.pop_back();

                    let result = functions::eval(*fun, value, &mut result_stack, &mut var_stack)
                        .map_err(at)?;
                    // Every arm that pushes a value checks it. A function result
                    // is bounded by construction — the built-ins all route
                    // through `f64` — but "bounded" is not "checked", and the
                    // difference is not academic: an unchecked value goes on to
                    // feed guards that assume their input was checked, which is
                    // how `floor(exp(1))!` slipped a 2-bit result past a 1-bit
                    // budget through the factorial's predictive guard.
                    limits::check_size(&result, limits).map_err(at)?;
                    result_stack.push_back(result);
                    var_stack.push_back(None);
                }
                Token::SemiColon => {
                    // A chained segment just ended. A well-formed segment leaves exactly
                    // one value on the stack; capture it as the running result, then reset
                    // for the next segment. An empty segment (e.g. a leading ';') is a no-op.
                    if !result_stack.is_empty() {
                        if result_stack.len() != 1 {
                            return Err(EvalError::Malformed { span: Some(t.span) });
                        }
                        last_result = result_stack.pop_back();
                    }
                    result_stack.clear();
                    var_stack.clear();
                }
                _ => {
                    return Err(EvalError::Malformed { span: Some(t.span) });
                }
            }
        }

        // A trailing ';' leaves the working stack empty: fall back to the last
        // completed segment's value rather than reporting a spurious error.
        if result_stack.is_empty() {
            return last_result.ok_or(EvalError::Malformed { span: None });
        }

        if result_stack.len() != 1 || var_stack.len() != 1 {
            return Err(EvalError::Malformed { span: None });
        }

        result_stack
            .pop_back()
            .ok_or(EvalError::Malformed { span: None })
    }

    fn factorial_helper(n: BigUint) -> BigUint {
        let mut acc = BigUint::one();
        let mut current = BigUint::one();

        while current <= n {
            acc *= &current;
            current += BigUint::one();
        }

        acc
    }

    fn power(left_value: Number, right_value: Number, limits: Limits) -> Result<Number, EvalError> {
        let value = if let Some(exponent) = right_value.as_integer() {
            Self::power_integer(left_value, exponent, limits)?
        } else {
            let base = number_to_f64(&left_value, EvalError::PowerOperandsTooLarge { span: None })?;
            let exponent = number_to_f64(
                &right_value,
                EvalError::PowerOperandsTooLarge { span: None },
            )?;
            decimal_from_f64(base.powf(exponent), EvalError::InvalidPower { span: None })?
        };

        // The prediction inside `power_integer` is an optimisation: it buys the
        // right to refuse `10^100000000` without computing it. It is not the
        // guarantee, and on two counts it cannot be. It never runs on the `powf`
        // path at all, and on the integer path it predicts the magnitude of
        // `base^|exponent|` while a negative exponent returns the reciprocal,
        // whose denominator `size_in_bits` also counts — `2^-1` predicts 2 bits
        // and yields `1/2`, which measures 3. Measuring the value we actually
        // built is what makes the budget exact, on every path.
        limits::check_size(&value, limits)?;
        Ok(value)
    }

    fn power_integer(base: Number, exponent: BigInt, limits: Limits) -> Result<Number, EvalError> {
        if exponent.is_zero() {
            return Ok(Number::NaturalNumber(BigInt::one()));
        }

        let is_negative = exponent < BigInt::zero();
        let magnitude = if is_negative { -exponent } else { exponent };
        let exponent = magnitude
            .to_biguint()
            .ok_or(EvalError::InvalidPower { span: None })?;

        // A degenerate base short-circuits inside the prediction, before the
        // exponent's own magnitude is ever consulted, so `1^n` stays evaluable for
        // an `n` no `u64` could hold. Only a base that actually grows can make the
        // exponent unrepresentable, and that is the one case this message fits.
        let predicted_bits = limits::predicted_power_bits(&base, &exponent)
            .ok_or(EvalError::ExponentTooLarge { span: None })?;
        limits::check_predicted_size(predicted_bits, limits)?;

        match base {
            Number::NaturalNumber(base) => {
                let value = Number::NaturalNumber(Self::pow_big_int(base, exponent));
                if is_negative {
                    Number::NaturalNumber(BigInt::one())
                        .checked_div(&value)
                        .ok_or(EvalError::DivisionByZero { span: None })
                } else {
                    Ok(value)
                }
            }
            Number::DecimalNumber(base) => {
                // `pow_big_rational` accumulates via `*=` on `BigRational`,
                // which reduces its own result, so `value` is already reduced.
                let value = Number::decimal_unchecked(Self::pow_big_rational(base, exponent));
                if is_negative {
                    Number::NaturalNumber(BigInt::one())
                        .checked_div(&value)
                        .ok_or(EvalError::DivisionByZero { span: None })
                } else {
                    Ok(value)
                }
            }
        }
    }

    fn pow_big_int(mut base: BigInt, mut exponent: BigUint) -> BigInt {
        let mut result = BigInt::one();

        while !exponent.is_zero() {
            if exponent.is_odd() {
                result *= &base;
            }
            exponent >>= 1_usize;
            if !exponent.is_zero() {
                base = &base * &base;
            }
        }

        result
    }

    fn pow_big_rational(mut base: BigRational, mut exponent: BigUint) -> BigRational {
        let mut result = BigRational::from_integer(BigInt::one());

        while !exponent.is_zero() {
            if exponent.is_odd() {
                result *= &base;
            }
            exponent >>= 1_usize;
            if !exponent.is_zero() {
                base = &base * &base;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        session::Session,
        span::Span,
        token::{Number, Operator},
    };
    use num_bigint::{BigInt, BigUint};

    #[test]
    fn test_factorial() {
        assert_eq!(
            Expression::factorial_helper(BigUint::from(5u8)),
            BigUint::from(120u16)
        );
    }

    #[test]
    fn test_resolve() {
        // The spans on the rpn expression are irrelevant to this test — it
        // exercises evaluation, not span propagation — so an arbitrary
        // placeholder span is used throughout.
        let no_span = Span::new(0, 0);
        let expr = Expression {
            rpn: VecDeque::from(vec![
                Spanned::new(
                    Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
                    no_span,
                ),
                Spanned::new(
                    Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
                    no_span,
                ),
                Spanned::new(Token::Operator(Operator::Add), no_span),
            ]),
        };
        assert_eq!(
            expr.eval(&Session::init()).unwrap(),
            Number::NaturalNumber(BigInt::from(3u8))
        );
    }

    #[test]
    fn test_invalid_factorial() {
        let session = Session::init();
        let expr = Expression::compile("(-1)!").unwrap();
        assert!(expr.eval(&session).is_err());
        let expr2 = Expression::compile("1.5!").unwrap();
        assert!(expr2.eval(&session).is_err());
    }

    /// `max` and `min` of integers return integers, and the enum tag has to say
    /// so. Asserting the value alone is not enough: cross-variant equality makes
    /// `NaturalNumber(2) == DecimalNumber(2/1)`, so a value-only assertion passes
    /// whichever variant comes back. That is exactly how this test used to read
    /// as "max returns a decimal" and stay green under either behaviour.
    ///
    /// Each expression used to share one loop and one failure message; a
    /// failure on the first hid whether the other two passed. Splitting them
    /// into their own tests means each runs, and reports, independently.
    #[test]
    fn test_max_of_two_naturals_returns_a_natural_number() {
        let session = Session::init();
        let result = Expression::compile("max(1,2)")
            .unwrap()
            .eval(&session)
            .unwrap();
        assert_eq!(result, Number::NaturalNumber(BigInt::from(2)));
        assert!(
            matches!(result, Number::NaturalNumber(_)),
            "max(1,2) produced {result:?}, expected a NaturalNumber"
        );
    }

    #[test]
    fn test_min_of_two_naturals_returns_a_natural_number() {
        let session = Session::init();
        let result = Expression::compile("min(1,2)")
            .unwrap()
            .eval(&session)
            .unwrap();
        assert_eq!(result, Number::NaturalNumber(BigInt::from(1)));
        assert!(
            matches!(result, Number::NaturalNumber(_)),
            "min(1,2) produced {result:?}, expected a NaturalNumber"
        );
    }

    #[test]
    fn test_nested_min_of_max_returns_a_natural_number() {
        let session = Session::init();
        let result = Expression::compile("min(max(1,2),3)")
            .unwrap()
            .eval(&session)
            .unwrap();
        assert_eq!(result, Number::NaturalNumber(BigInt::from(2)));
        assert!(
            matches!(result, Number::NaturalNumber(_)),
            "min(max(1,2),3) produced {result:?}, expected a NaturalNumber"
        );
    }
}
