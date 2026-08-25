use crate::error::EvalError;
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
use anyhow::anyhow;
use log::debug;
use num::Integer;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use num::{BigInt, BigUint, One, Zero};
use num_rational::BigRational;
use num_traits::ToPrimitive;

/// The main [`RpnResolver`] contains the core logic of Yarer
/// for parsing and evaluating a math expression.
///
/// It holds the tokenised expression (by the [`Parser`]) and
/// a heap of local variables borrowed from a [`Session`]
///
pub struct RpnResolver<'a> {
    rpn_expr: VecDeque<Spanned<Token<'a>>>,
    local_heap: Rc<RefCell<HashMap<String, Number>>>,
    build_error: Option<String>,
    limits: Limits,
}

impl RpnResolver<'_> {
    /// Generates a new [`RpnResolver`] instance with borrowed heap
    ///
    pub fn parse_with_borrowed_heap<'a>(
        exp: &'a str,
        borrowed_heap: Rc<RefCell<HashMap<String, Number>>>,
        limits: Limits,
    ) -> RpnResolver<'a> {
        match Parser::parse(exp)
            .and_then(|tokens| validate::validate(&tokens, exp))
            .and_then(|validated| shunting::to_rpn(&validated))
        {
            Ok(rpn_expr) => RpnResolver {
                rpn_expr,
                local_heap: borrowed_heap,
                build_error: None,
                limits,
            },
            Err(err) => RpnResolver {
                rpn_expr: VecDeque::new(),
                local_heap: borrowed_heap,
                build_error: Some(err.to_string()),
                limits,
            },
        }
    }

    /// This method evaluates the rpn expression stack
    ///
    pub fn resolve(&mut self) -> anyhow::Result<Number> {
        if let Some(build_error) = &self.build_error {
            return Err(anyhow!(build_error.clone()));
        }

        self.eval_inner().map_err(anyhow::Error::from)
    }

    /// The evaluation loop. Kept separate from [`Self::resolve`] so its errors
    /// can be typed as [`EvalError`] rather than boxed into `anyhow` from the
    /// first line: `resolve` is the only place that still needs the wider,
    /// stringly-typed boundary.
    fn eval_inner(&mut self) -> Result<Number, EvalError> {
        let zero: Number = Number::NaturalNumber(Zero::zero());
        let minus_one: Number = Number::NaturalNumber(BigInt::from(-1));
        let limits = self.limits;

        let mut result_stack: VecDeque<Number> = VecDeque::new();
        let mut var_stack: VecDeque<Option<String>> = VecDeque::new();
        let mut last_result: Option<Number> = None;

        for t in &self.rpn_expr {
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

                    let left_value = if op != &Operator::Une && op != &Operator::Fac {
                        result_stack
                            .pop_back()
                            .ok_or(EvalError::Malformed { span: Some(t.span) })?
                    } else {
                        zero.clone()
                    };
                    let left_var = if op != &Operator::Une && op != &Operator::Fac {
                        var_stack.pop_back().unwrap_or(None)
                    } else {
                        None
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
                            if right_value == zero {
                                return Err(EvalError::DivisionByZero { span: Some(t.span) });
                            }
                            let value = left_value / right_value;
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
                        Operator::Eql => {
                            if let Some(var) = left_var {
                                if Session::is_constant_name(&var) {
                                    return Err(EvalError::ReadOnlyConstant {
                                        name: var,
                                        span: Some(t.span),
                                    });
                                }
                                self.local_heap
                                    .borrow_mut()
                                    .insert(var.clone(), right_value.clone());

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
                    }
                }
                Token::Variable(v) => {
                    let var_name = v.to_lowercase();
                    debug!("Heap {:?}", self.local_heap);
                    let heap = self.local_heap.borrow();
                    // An undefined variable reads as zero, deliberately.
                    let n = heap
                        .get(&var_name)
                        .cloned()
                        .unwrap_or_else(|| Number::NaturalNumber(BigInt::zero()));
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
                if is_negative {
                    if base.is_zero() {
                        return Err(EvalError::DivisionByZero { span: None });
                    }

                    let value = Self::pow_big_int(base, exponent);
                    Ok(Number::decimal(BigRational::new(BigInt::one(), value)))
                } else {
                    Ok(Number::NaturalNumber(Self::pow_big_int(base, exponent)))
                }
            }
            Number::DecimalNumber(base) => {
                if is_negative && base.is_zero() {
                    return Err(EvalError::DivisionByZero { span: None });
                }

                let value = Self::pow_big_rational(base, exponent);
                if is_negative {
                    Ok(Number::decimal(value.recip()))
                } else {
                    Ok(Number::decimal(value))
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
            RpnResolver::factorial_helper(BigUint::from(5u8)),
            BigUint::from(120u16)
        );
    }

    #[test]
    fn test_resolve() {
        // The spans on the rpn expression are irrelevant to this test — it
        // exercises evaluation, not span propagation — so an arbitrary
        // placeholder span is used throughout.
        let no_span = Span::new(0, 0);
        let mut resolver = RpnResolver {
            rpn_expr: VecDeque::from(vec![
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
            local_heap: Rc::new(RefCell::new(HashMap::new())),
            build_error: None,
            limits: Limits::default(),
        };
        assert_eq!(
            resolver.resolve().unwrap(),
            Number::NaturalNumber(BigInt::from(3u8))
        );
    }

    #[test]
    fn test_invalid_factorial() {
        let session = Session::init();
        let mut resolver = session.process("(-1)!");
        assert!(resolver.resolve().is_err());
        let mut resolver2 = session.process("1.5!");
        assert!(resolver2.resolve().is_err());
    }

    /// `max` and `min` of integers return integers, and the enum tag has to say
    /// so. Asserting the value alone is not enough: cross-variant equality makes
    /// `NaturalNumber(2) == DecimalNumber(2/1)`, so a value-only assertion passes
    /// whichever variant comes back. That is exactly how this test used to read
    /// as "max returns a decimal" and stay green under either behaviour.
    #[test]
    fn test_max_min() {
        let session = Session::init();
        for (expr, expected) in [("max(1,2)", 2), ("min(1,2)", 1), ("min(max(1,2),3)", 2)] {
            let mut resolver = session.process(expr);
            let result = resolver.resolve().unwrap();
            assert_eq!(result, Number::NaturalNumber(BigInt::from(expected)));
            assert!(
                matches!(result, Number::NaturalNumber(_)),
                "{expr} produced {result:?}, expected a NaturalNumber"
            );
        }
    }
}
