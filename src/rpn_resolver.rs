use crate::functions::{decimal_from_f64, number_to_f64};
use crate::{
    functions,
    parser::Parser,
    session::Session,
    token::{self, Number, Operator, Token},
};
use anyhow::anyhow;
use log::debug;
use num::Integer;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Display,
    rc::Rc,
};

use num::{BigInt, BigUint, One, Zero};
use num_rational::BigRational;
use num_traits::ToPrimitive;

pub(crate) static MALFORMED_ERR: &str = "Runtime Error: The mathematical expression is malformed.";
static DIVISION_ZERO_ERR: &str = "Runtime error: Divide by zero.";
static NO_VARIABLE_ERR: &str = "Runtime error: No variable has been defined for assignment.";
static FACTORIAL_NATURAL_ERR: &str =
    "Runtime error: Factorial is only defined for non-negative integers.";
static BUILTIN_CONSTANT_ERR: &str = "Runtime error: Built-in constants are read-only.";
pub(crate) static INVALID_FUNCTION_RESULT_ERR: &str =
    "Runtime error: Function result is not a real number.";
static INVALID_POWER_ERR: &str = "Runtime error: Invalid power operation.";
pub(crate) static FLOAT_EVAL_TOO_LARGE_ERR: &str =
    "Runtime error: Operand is too large for floating-point evaluation.";
static POWER_TOO_LARGE_ERR: &str =
    "Runtime error: Power operands are too large for non-integer evaluation.";

/// The main [`RpnResolver`] contains the core logic of Yarer
/// for parsing and evaluating a math expression.
///
/// It holds the tokenised expression (by the [`Parser`]) and
/// a heap of local variables borrowed from a [`Session`]
///
pub struct RpnResolver<'a> {
    rpn_expr: VecDeque<Token<'a>>,
    local_heap: Rc<RefCell<HashMap<String, Number>>>,
    build_error: Option<String>,
}

impl RpnResolver<'_> {
    /// Generates a new [`RpnResolver`] instance with borrowed heap
    ///
    pub fn parse_with_borrowed_heap<'a>(
        exp: &'a str,
        borrowed_heap: Rc<RefCell<HashMap<String, Number>>>,
    ) -> RpnResolver<'a> {
        let heap_for_parse = Rc::clone(&borrowed_heap);
        match Parser::parse(exp).and_then(|tokenised_expr| {
            RpnResolver::reverse_polish_notation(&tokenised_expr, heap_for_parse)
        }) {
            Ok((rpn_expr, local_heap)) => RpnResolver {
                rpn_expr,
                local_heap,
                build_error: None,
            },
            Err(err) => RpnResolver {
                rpn_expr: VecDeque::new(),
                local_heap: borrowed_heap,
                build_error: Some(err.to_string()),
            },
        }
    }

    /// This method evaluates the rpn expression stack
    ///
    pub fn resolve(&mut self) -> anyhow::Result<Number> {
        if let Some(build_error) = &self.build_error {
            return Err(anyhow!(build_error.clone()));
        }

        let zero: Number = Number::NaturalNumber(Zero::zero());
        let minus_one: Number = Number::NaturalNumber(BigInt::from(-1));

        let mut result_stack: VecDeque<Number> = VecDeque::new();
        let mut var_stack: VecDeque<Option<String>> = VecDeque::new();
        let mut last_result: Option<Number> = None;

        for t in &self.rpn_expr {
            match t {
                Token::Operand(n) => {
                    result_stack.push_back(n.clone());
                    var_stack.push_back(None);
                }
                Token::Operator(op) => {
                    let right_value: Number = result_stack
                        .pop_back()
                        .ok_or_else(|| anyhow!("{} {}", MALFORMED_ERR, "Invalid Right Operand."))?;

                    var_stack.pop_back();

                    let left_value = if op != &Operator::Une && op != &Operator::Fac {
                        result_stack.pop_back().ok_or_else(|| {
                            anyhow!("{} {}", MALFORMED_ERR, "Invalid Left Operand.")
                        })?
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
                            result_stack.push_back(left_value + right_value);
                            var_stack.push_back(None);
                        }
                        Operator::Sub => {
                            result_stack.push_back(left_value - right_value);
                            var_stack.push_back(None);
                        }
                        Operator::Mul => {
                            result_stack.push_back(left_value * right_value);
                            var_stack.push_back(None);
                        }
                        Operator::Div => {
                            if right_value == zero {
                                return Err(anyhow!(DIVISION_ZERO_ERR));
                            }
                            result_stack.push_back(left_value / right_value);
                            var_stack.push_back(None);
                        }
                        Operator::Pow => {
                            result_stack.push_back(Self::power(left_value, right_value)?);
                            var_stack.push_back(None);
                        }
                        Operator::Eql => {
                            if let Some(var) = left_var {
                                if Session::is_constant_name(&var) {
                                    return Err(anyhow!(BUILTIN_CONSTANT_ERR));
                                }
                                self.local_heap
                                    .borrow_mut()
                                    .insert(var.clone(), right_value.clone());

                                result_stack.push_back(right_value);
                                var_stack.push_back(None);
                            } else {
                                return Err(anyhow!(NO_VARIABLE_ERR));
                            }
                        }
                        Operator::Fac => {
                            // factorial. Only for non-negative integers
                            match right_value {
                                Number::NaturalNumber(v) => {
                                    if v < Zero::zero() {
                                        return Err(anyhow!(FACTORIAL_NATURAL_ERR));
                                    }
                                    let n = v.to_u64().ok_or_else(|| {
                                        anyhow!("Runtime Error: Factorial operand is too large")
                                    })?;
                                    let res = Self::factorial_helper(n.into());
                                    result_stack.push_back(Number::NaturalNumber(res.into()));
                                    var_stack.push_back(None);
                                }
                                Number::DecimalNumber(_) => {
                                    return Err(anyhow!(FACTORIAL_NATURAL_ERR));
                                }
                            }
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
                    let n = heap
                        .get(&var_name)
                        .cloned()
                        .unwrap_or_else(|| Number::NaturalNumber(BigInt::zero()));
                    result_stack.push_back(n);
                    var_stack.push_back(Some(var_name));
                }
                Token::Function(fun) => {
                    let value: Number = result_stack.pop_back().ok_or(anyhow!(
                        "{} {}",
                        MALFORMED_ERR,
                        "Wrong use of function"
                    ))?;
                    var_stack.pop_back();

                    let result = functions::eval(*fun, value, &mut result_stack, &mut var_stack)?;
                    result_stack.push_back(result);
                    var_stack.push_back(None);
                }
                Token::SemiColon => {
                    // A chained segment just ended. A well-formed segment leaves exactly
                    // one value on the stack; capture it as the running result, then reset
                    // for the next segment. An empty segment (e.g. a leading ';') is a no-op.
                    if !result_stack.is_empty() {
                        if result_stack.len() != 1 {
                            return Err(anyhow!(MALFORMED_ERR));
                        }
                        last_result = result_stack.pop_back();
                    }
                    result_stack.clear();
                    var_stack.clear();
                }
                _ => {
                    return Err(anyhow!(
                        "{} Internal Error at line: {}.",
                        MALFORMED_ERR,
                        line!()
                    ))
                }
            }
        }

        // A trailing ';' leaves the working stack empty: fall back to the last
        // completed segment's value rather than reporting a spurious error.
        if result_stack.is_empty() {
            return last_result.ok_or_else(|| anyhow!(MALFORMED_ERR));
        }

        if result_stack.len() != 1 || var_stack.len() != 1 {
            return Err(anyhow!(MALFORMED_ERR));
        }

        result_stack.pop_back().ok_or(anyhow!("{}", MALFORMED_ERR))
    }

    /// Transforming an infix notation to Reverse Polish Notation (RPN)
    ///
    /// Example
    /// ``
    ///     "3 * 4 + 5 * 6" becomes "3 4 * 5 6 * +"
    /// ``
    fn reverse_polish_notation<'a>(
        infix_stack: &[Token<'a>],
        local_heap: Rc<RefCell<HashMap<String, Number>>>,
    ) -> anyhow::Result<(VecDeque<Token<'a>>, Rc<RefCell<HashMap<String, Number>>>)> {
        /*  Create an empty stack for keeping operators. Create an empty list for output. */
        let mut operators_stack: Vec<Token> = Vec::new();
        let mut postfix_stack: VecDeque<Token> = VecDeque::new();
        let mut seen_variables: Vec<String> = Vec::new();

        /* Scan the infix expression from left to right. */
        for t in infix_stack {
            match *t {
                /* If the token is an operand, add it to the output list. */
                Token::Operand(_) => postfix_stack.push_back(t.clone()),

                /* If the token is a left parenthesis, push it on the stack. */
                Token::Bracket(token::Bracket::Open) => operators_stack.push(t.clone()),

                /* If the token is a right parenthesis:
                Pop the stack and add operators to the output list until you encounter a left parenthesis.
                Pop the left parenthesis from the stack but do not add it to the output list.*/
                Token::Bracket(token::Bracket::Close) => {
                    let mut found_open = false;
                    while let Some(token) = operators_stack.pop() {
                        match token {
                            Token::Bracket(token::Bracket::Open) => {
                                found_open = true;
                                // If the token is a left parenthesis, pop it from the stack
                                if let Some(Token::Function(_)) = operators_stack.last() {
                                    postfix_stack.push_back(
                                        operators_stack.pop().expect("It should not happen."),
                                    );
                                }
                                break;
                            } // discards left parenthesis
                            _ => postfix_stack.push_back(token),
                        }
                    }
                    if !found_open {
                        return Err(anyhow!(MALFORMED_ERR));
                    }
                }

                Token::Comma => {
                    let mut found_open = false;
                    while let Some(token) = operators_stack.last() {
                        if matches!(token, Token::Bracket(token::Bracket::Open)) {
                            found_open = true;
                            break;
                        }
                        postfix_stack
                            .push_back(operators_stack.pop().expect("It should not happen."));
                    }
                    if !found_open {
                        return Err(anyhow!(MALFORMED_ERR));
                    }
                }

                Token::SemiColon => {
                    while let Some(token) = operators_stack.pop() {
                        postfix_stack.push_back(token);
                    }
                    postfix_stack.push_back(Token::SemiColon);
                }

                Token::Operator(_op) => {
                    let op1: Token<'_> = t.clone();

                    while !operators_stack.is_empty() {
                        let op2: &Token = operators_stack.last().unwrap();
                        match op2 {
                            Token::Operator(_) => {
                                if Token::compare_operator_priority(op1.clone(), op2.clone()) {
                                    postfix_stack.push_back(
                                        operators_stack.pop().expect("It should not happen."),
                                    );
                                } else {
                                    break;
                                }
                            }
                            Token::Function(_) => {
                                postfix_stack.push_back(
                                    operators_stack.pop().expect("It should not happen."),
                                );
                            }
                            _ => break,
                        }
                    }
                    operators_stack.push(op1.clone());
                }

                Token::Function(_) => {
                    operators_stack.push(t.clone());
                }

                /* If the token is a variable, add it to the output list and to the local_heap with a default value*/
                Token::Variable(s) => {
                    postfix_stack.push_back(t.clone());
                    seen_variables.push(s.to_lowercase());
                }
            }
            debug!(
                "Inspecting... {} - OUT {} - OP - {}",
                *t,
                DisplayThisDeque(&postfix_stack),
                DisplayThatVec(&operators_stack)
            );
        }

        /* After all tokens are read, pop remaining operators from the stack and add them to the list. */
        operators_stack.reverse();
        for t in &operators_stack {
            if matches!(t, Token::Bracket(_)) {
                return Err(anyhow!(MALFORMED_ERR));
            }
            postfix_stack.push_back(t.clone());
        }

        let mut heap = local_heap.borrow_mut();
        for variable in seen_variables {
            heap.entry(variable)
                .or_insert(Number::NaturalNumber(Zero::zero()));
        }
        drop(heap);

        debug!(
            "DEBUG: EOF - OUT {} - OP - {}",
            DisplayThisDeque(&postfix_stack),
            DisplayThatVec(&operators_stack)
        );

        Ok((postfix_stack, local_heap))
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

    fn integer_exponent(value: &Number) -> Option<BigInt> {
        match value {
            Number::NaturalNumber(v) => Some(v.clone()),
            Number::DecimalNumber(v) if v.denom().is_one() => Some(v.to_integer()),
            Number::DecimalNumber(_) => None,
        }
    }

    fn power(left_value: Number, right_value: Number) -> anyhow::Result<Number> {
        if let Some(exponent) = Self::integer_exponent(&right_value) {
            return Self::power_integer(left_value, exponent);
        }

        let base = number_to_f64(&left_value, POWER_TOO_LARGE_ERR)?;
        let exponent = number_to_f64(&right_value, POWER_TOO_LARGE_ERR)?;
        decimal_from_f64(base.powf(exponent), INVALID_POWER_ERR)
    }

    fn power_integer(base: Number, exponent: BigInt) -> anyhow::Result<Number> {
        if exponent.is_zero() {
            return Ok(Number::NaturalNumber(BigInt::one()));
        }

        let is_negative = exponent < BigInt::zero();
        let magnitude = if is_negative { -exponent } else { exponent };
        let exponent = magnitude
            .to_biguint()
            .ok_or_else(|| anyhow!(INVALID_POWER_ERR))?;

        match base {
            Number::NaturalNumber(base) => {
                if is_negative {
                    if base.is_zero() {
                        return Err(anyhow!(DIVISION_ZERO_ERR));
                    }

                    let value = Self::pow_big_int(base, exponent);
                    Ok(Number::DecimalNumber(BigRational::new(
                        BigInt::one(),
                        value,
                    )))
                } else {
                    Ok(Number::NaturalNumber(Self::pow_big_int(base, exponent)))
                }
            }
            Number::DecimalNumber(base) => {
                if is_negative && base.is_zero() {
                    return Err(anyhow!(DIVISION_ZERO_ERR));
                }

                let value = Self::pow_big_rational(base, exponent);
                if is_negative {
                    Ok(Number::DecimalNumber(value.recip()))
                } else {
                    Ok(Number::DecimalNumber(value))
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

struct DisplayThatVec<'a>(&'a Vec<Token<'a>>);
struct DisplayThisDeque<'a>(&'a VecDeque<Token<'a>>);

impl Display for DisplayThatVec<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.iter().map(ToString::to_string).collect::<String>()
        )
    }
}

impl Display for DisplayThisDeque<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.iter().map(ToString::to_string).collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        session::Session,
        token::{Number, Operator},
    };
    use num_bigint::{BigInt, BigUint};

    #[test]
    fn test_reverse_polish_notation() {
        let a: Vec<Token> = vec![
            Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
            Token::Operator(Operator::Add),
            Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
        ];
        let b: Vec<Token> = vec![
            Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
            Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
            Token::Operator(Operator::Add),
        ];
        assert_eq!(
            RpnResolver::reverse_polish_notation(&a, Rc::new(RefCell::new(HashMap::new())))
                .unwrap()
                .0,
            b
        );
    }

    #[test]
    fn test_factorial() {
        assert_eq!(
            RpnResolver::factorial_helper(BigUint::from(5u8)),
            BigUint::from(120u16)
        );
    }

    #[test]
    fn test_resolve() {
        let mut resolver = RpnResolver {
            rpn_expr: VecDeque::from(vec![
                Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
                Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
                Token::Operator(Operator::Add),
            ]),
            local_heap: Rc::new(RefCell::new(HashMap::new())),
            build_error: None,
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

    #[test]
    fn test_max_min() {
        let session = Session::init();
        let mut resolver = session.process("max(1,2)");
        assert_eq!(
            resolver.resolve().unwrap(),
            Number::DecimalNumber(BigRational::from_float(2.0).unwrap())
        );

        let mut resolver = session.process("min(1,2)");
        assert_eq!(
            resolver.resolve().unwrap(),
            Number::DecimalNumber(BigRational::from_float(1.0).unwrap())
        );

        let mut resolver = session.process("min(max(1,2),3)");
        assert_eq!(
            resolver.resolve().unwrap(),
            Number::DecimalNumber(BigRational::from_float(2.0).unwrap())
        );
    }
}
