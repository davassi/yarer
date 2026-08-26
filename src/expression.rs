//! Compiled expressions, and the loop that evaluates them.

use crate::error::{EvalError, ParseError};
use crate::functions::decimal_from_f64;
use crate::limits::{self, Limits};
use crate::{
    functions,
    parser::Parser,
    session::Session,
    shunting,
    span::{Span, Spanned},
    token::{narrow_to_f64, Narrowing, Number, Operator, Token},
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

/// Truth as this crate represents it: `1` and `0`, as in GNU bc, which is the
/// value model the README already names.
///
/// There is no boolean [`Number`] variant, and this is the whole of the price:
/// `(1<2) + 5` is a legal expression worth 6. That is the accepted cost of not
/// introducing a second kind of value into a crate whose entire surface is
/// built around one.
fn boolean(truth: bool) -> Number {
    Number::NaturalNumber(BigInt::from(u8::from(truth)))
}

/// Narrows one operand of a non-integer power.
///
/// It does not go through [`functions::number_to_f64`] because the two report
/// different things: a power's operands failing to fit is about the power, and
/// `(2^2000)^0.5` says so rather than blaming `2^2000`, which is a fine value.
/// A test pins that distinction. Both share [`narrow_to_f64`], which is the
/// part that must not be duplicated — the mapping to an error is what differs.
///
/// # Errors
/// [`EvalError::PowerOperandsTooLarge`] or [`EvalError::PowerOperandsTooSmall`].
fn power_operand_to_f64(value: &Number) -> Result<f64, EvalError> {
    narrow_to_f64(value).map_err(|why| match why {
        Narrowing::TooLarge => EvalError::PowerOperandsTooLarge { span: None },
        Narrowing::TooSmall => EvalError::PowerOperandsTooSmall { span: None },
    })
}

/// Zero is false and everything else is true — including negative and
/// fractional values, which is why this asks the value rather than the variant.
///
/// [`Number`]'s [`PartialEq`] compares mathematically, so a `DecimalNumber`
/// holding zero is false too — exactly as [`Number::checked_div`] relies on
/// for its divisor test.
fn is_truthy(value: &Number) -> bool {
    value != &Number::NaturalNumber(BigInt::zero())
}

/// The two stacks the evaluation loop walks.
///
/// They are one type because they must stay in lockstep: every value pushed
/// gets a variable slot beside it, and every pop takes both. That was
/// maintained by hand at fifteen sites, and an arm that pushed a value without
/// pushing `None` beside it would have desynchronised the assignment target for
/// everything after it.
struct Stacks {
    values: VecDeque<Number>,
    vars: VecDeque<Option<String>>,
}

impl Stacks {
    fn new() -> Stacks {
        Stacks {
            values: VecDeque::new(),
            vars: VecDeque::new(),
        }
    }

    /// Pushes an operator's result: measured against the budget first, and with
    /// no variable name beside it, because an operator's result is not an
    /// assignable place.
    ///
    /// # Errors
    /// [`EvalError::ValueTooLarge`] when the result exceeds the budget.
    fn push_checked(&mut self, value: Number, limits: Limits) -> Result<(), EvalError> {
        limits::check_size(&value, limits)?;
        self.values.push_back(value);
        self.vars.push_back(None);
        Ok(())
    }

    /// Pushes a truth as this crate represents it — `1` or `0`, as in GNU bc.
    ///
    /// The size check inside is not decoration and is not unreachable: a
    /// zero-bit budget refuses `0 == 0`, whose operands cost nothing and whose
    /// answer costs one bit.
    ///
    /// # Errors
    /// As [`Stacks::push_checked`].
    fn push_truth(&mut self, truth: bool, limits: Limits) -> Result<(), EvalError> {
        self.push_checked(boolean(truth), limits)
    }
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
        let mut stacks = Stacks::new();
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
                    stacks.values.push_back(n.clone());
                    stacks.vars.push_back(None);
                }
                Token::Operator(op) => {
                    apply_operator(session, *op, t.span, limits, &mut stacks)?;
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
                    stacks.values.push_back(n);
                    stacks.vars.push_back(Some(var_name));
                }
                Token::Function(fun) => {
                    let value: Number = stacks
                        .values
                        .pop_back()
                        .ok_or(EvalError::Malformed { span: Some(t.span) })?;
                    stacks.vars.pop_back();

                    let result = functions::eval(*fun, value, &mut stacks.values, &mut stacks.vars)
                        .map_err(at)?;
                    // Every arm that pushes a value checks it. A function result
                    // is bounded by construction — the built-ins all route
                    // through `f64` — but "bounded" is not "checked", and the
                    // difference is not academic: an unchecked value goes on to
                    // feed guards that assume their input was checked, which is
                    // how `floor(exp(1))!` slipped a 2-bit result past a 1-bit
                    // budget through the factorial's predictive guard.
                    limits::check_size(&result, limits).map_err(at)?;
                    stacks.values.push_back(result);
                    stacks.vars.push_back(None);
                }
                Token::SemiColon => {
                    // A chained segment just ended. A well-formed segment leaves exactly
                    // one value on the stack; capture it as the running result, then reset
                    // for the next segment. An empty segment (e.g. a leading ';') is a no-op.
                    if !stacks.values.is_empty() {
                        if stacks.values.len() != 1 {
                            return Err(EvalError::Malformed { span: Some(t.span) });
                        }
                        last_result = stacks.values.pop_back();
                    }
                    stacks.values.clear();
                    stacks.vars.clear();
                }
                _ => {
                    return Err(EvalError::Malformed { span: Some(t.span) });
                }
            }
        }

        // A trailing ';' leaves the working stack empty: fall back to the last
        // completed segment's value rather than reporting a spurious error.
        if stacks.values.is_empty() {
            return last_result.ok_or(EvalError::Malformed { span: None });
        }

        if stacks.values.len() != 1 || stacks.vars.len() != 1 {
            return Err(EvalError::Malformed { span: None });
        }

        stacks
            .values
            .pop_back()
            .ok_or(EvalError::Malformed { span: None })
    }
}

/// Applies one operator to the top of the stacks.
///
/// Split out of [`Expression::eval_with`] because that function's job is to
/// walk the compiled sequence and dispatch by token kind, and this one's is to
/// know what each operator means. Sixteen arms of the former buried the latter.
///
/// # Errors
/// Any [`EvalError`] an operator can raise: a division by zero, a value over
/// the budget, an assignment with no target, and so on.
fn apply_operator(
    session: &Session,
    op: Operator,
    span: Span,
    limits: Limits,
    stacks: &mut Stacks,
) -> Result<(), EvalError> {
    // Errors raised inside `limits.rs` and `functions.rs` know nothing of
    // positions — this closure is how the operator stamps them with the token
    // it was holding when it called out.
    let at = |e: EvalError| e.at(span);

    let right_value: Number = stacks
        .values
        .pop_back()
        .ok_or(EvalError::Malformed { span: Some(span) })?;
    stacks.vars.pop_back();

    let (left_value, left_var) = if op.is_unary() {
        (Number::NaturalNumber(BigInt::zero()), None)
    } else {
        let value = stacks
            .values
            .pop_back()
            .ok_or(EvalError::Malformed { span: Some(span) })?;
        (value, stacks.vars.pop_back().unwrap_or(None))
    };

    match op {
        Operator::Add => stacks
            .push_checked(left_value + right_value, limits)
            .map_err(at)?,
        Operator::Sub => stacks
            .push_checked(left_value - right_value, limits)
            .map_err(at)?,
        Operator::Mul => stacks
            .push_checked(left_value * right_value, limits)
            .map_err(at)?,
        Operator::Div => {
            let value = left_value
                .checked_div(&right_value)
                .ok_or(EvalError::DivisionByZero { span: Some(span) })?;
            stacks.push_checked(value, limits).map_err(at)?;
        }
        // `power` applies the budget itself, through a prediction that refuses
        // `2^100000000` without computing it, so it does not go through
        // `push_checked`.
        Operator::Pow => {
            let value = power(left_value, right_value, limits).map_err(at)?;
            stacks.values.push_back(value);
            stacks.vars.push_back(None);
        }
        Operator::Assign => {
            let Some(var) = left_var else {
                return Err(EvalError::AssignmentTargetMissing { span: Some(span) });
            };
            // `assign` decides the refusal, here and for `set`/`setf` alike;
            // this only supplies the position it happened at.
            session.assign(&var, right_value.clone()).map_err(at)?;
            stacks.values.push_back(right_value);
            stacks.vars.push_back(None);
        }
        Operator::Fac => {
            let value = factorial(&right_value, span, limits)?;
            stacks.push_checked(value, limits).map_err(at)?;
        }
        Operator::Une => stacks
            .push_checked(
                right_value * Number::NaturalNumber(BigInt::from(-1)),
                limits,
            )
            .map_err(at)?,
        // The six comparisons ask `Number`'s own `PartialOrd`, which Stage 1
        // made agree with `PartialEq` by comparing mathematical value rather
        // than enum variant — so `2 == 6/3` is true with no code of its own.
        Operator::Less => stacks
            .push_truth(left_value < right_value, limits)
            .map_err(at)?,
        Operator::Greater => stacks
            .push_truth(left_value > right_value, limits)
            .map_err(at)?,
        Operator::LessEq => stacks
            .push_truth(left_value <= right_value, limits)
            .map_err(at)?,
        Operator::GreaterEq => stacks
            .push_truth(left_value >= right_value, limits)
            .map_err(at)?,
        Operator::Equal => stacks
            .push_truth(left_value == right_value, limits)
            .map_err(at)?,
        Operator::NotEqual => stacks
            .push_truth(left_value != right_value, limits)
            .map_err(at)?,
        // Both operands are already on the stack, so the `&&` below
        // short-circuits nothing: the right-hand expression was evaluated
        // before this arm was reached.
        Operator::And => stacks
            .push_truth(is_truthy(&left_value) && is_truthy(&right_value), limits)
            .map_err(at)?,
        Operator::Or => stacks
            .push_truth(is_truthy(&left_value) || is_truthy(&right_value), limits)
            .map_err(at)?,
        Operator::Xor => stacks
            .push_truth(is_truthy(&left_value) != is_truthy(&right_value), limits)
            .map_err(at)?,
        // Prefix, so the operand is the one `is_unary` left in `right_value`.
        Operator::Not => stacks
            .push_truth(!is_truthy(&right_value), limits)
            .map_err(at)?,
        Operator::Mod => {
            // The zero check is `checked_div`'s, the one place this crate
            // decides what a division by zero is.
            let quotient = left_value
                .checked_div(&right_value)
                .ok_or(EvalError::DivisionByZero { span: Some(span) })?;
            // `From<Number> for BigInt` truncates toward zero rather than
            // flooring, which is what makes `-7 mod 3` be -1 and not 2 — the
            // convention of C, Rust, bc and BASIC.
            let truncated = Number::NaturalNumber(BigInt::from(quotient));
            stacks
                .push_checked(left_value - right_value * truncated, limits)
                .map_err(at)?;
        }
    }
    Ok(())
}

/// Factorial, defined on non-negative integers.
///
/// Predicts the size first, so that `999999999!` is refused in microseconds
/// rather than computed, and measures the result afterwards, because the
/// prediction is an asymptotic series rounded up and is a bit short of the
/// truth at `n = 2`. The prediction buys the speed; the measurement buys the
/// exactness. Both are load-bearing, and the register records what happened
/// when one of them was missing.
///
/// # Errors
/// [`EvalError::FactorialNotNatural`], [`EvalError::FactorialOperandTooLarge`],
/// or [`EvalError::ValueTooLarge`].
fn factorial(operand: &Number, span: Span, limits: Limits) -> Result<Number, EvalError> {
    // Factorial is defined on non-negative integers. It asks the value, not the
    // enum tag: floor(2.5) and 6/3 are integers.
    let n = operand
        .as_integer()
        .ok_or(EvalError::FactorialNotNatural { span: Some(span) })?;
    if n < BigInt::zero() {
        return Err(EvalError::FactorialNotNatural { span: Some(span) });
    }
    let n = n
        .to_u64()
        .ok_or(EvalError::FactorialOperandTooLarge { span: Some(span) })?;
    limits::check_predicted_size(limits::predicted_factorial_bits(n), limits)
        .map_err(|e| e.at(span))?;
    Ok(Number::NaturalNumber(factorial_helper(n.into()).into()))
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
        power_integer(left_value, exponent, limits)?
    } else {
        let base = power_operand_to_f64(&left_value)?;
        let exponent = power_operand_to_f64(&right_value)?;
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
            let value = Number::NaturalNumber(pow_big_int(base, exponent));
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
            let value = Number::decimal_unchecked(pow_big_rational(base, exponent));
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
        assert_eq!(factorial_helper(BigUint::from(5u8)), BigUint::from(120u16));
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
