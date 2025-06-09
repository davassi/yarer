use crate::{
    parser::Parser,
    token::{self, MathFunction, Number, Operator, Token},
};
use anyhow::anyhow;
use log::debug;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Display,
    rc::Rc,
};

use num::{BigInt, BigUint, One, Zero};
use num_traits::ToPrimitive;

static MALFORMED_ERR: &str = "Runtime Error: The mathematical expression is malformed.";
static DIVISION_ZERO_ERR: &str = "Runtime error: Divide by zero.";
static NO_VARIABLE_ERR: &str = "Runtime error: No variable has been defined for assignment.";
static FACTORIAL_NATURAL_ERR: &str =
    "Runtime error: Factorial is only defined for non-negative integers.";

/// The main [`RpnResolver`] contains the core logic of Yarer
/// for parsing and evaluating a math expression.
///
/// It holds the tokenised expression (by the [`Parser`]) and
/// a heap of local variables borrowed from a [`Session`]
///
pub struct RpnResolver<'a> {
    rpn_expr: VecDeque<Token<'a>>,
    local_heap: Rc<RefCell<HashMap<String, Number>>>,
}

impl RpnResolver<'_> {
    /// Generates a new [`RpnResolver`] instance with borrowed heap
    ///
    pub fn parse_with_borrowed_heap<'a>(
        exp: &'a str,
        borrowed_heap: Rc<RefCell<HashMap<String, Number>>>,
    ) -> RpnResolver<'a> {
        let tokenised_expr: Vec<Token<'a>> = Parser::parse(exp);
        let (rpn_expr, local_heap) =
            RpnResolver::reverse_polish_notation(&tokenised_expr, borrowed_heap);

        RpnResolver {
            rpn_expr,
            local_heap,
        }
    }

    /// This method evaluates the rpn expression stack
    ///
    pub fn resolve(&mut self) -> anyhow::Result<Number> {
        let zero: Number = Number::NaturalNumber(Zero::zero());
        let minus_one: Number = Number::NaturalNumber(BigInt::from(-1));

        let mut result_stack: VecDeque<Number> = VecDeque::new();
        let mut var_stack: VecDeque<Option<String>> = VecDeque::new();

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

                    let mut left_value = if op != &Operator::Une && op != &Operator::Fac {
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
                            left_value = Number::DecimalNumber(left_value.into());
                            result_stack.push_back(left_value / right_value);
                            var_stack.push_back(None);
                        }
                        Operator::Pow => {
                            if right_value < zero {
                                if left_value == zero {
                                    return Err(anyhow!(DIVISION_ZERO_ERR));
                                }
                                left_value = Number::DecimalNumber(left_value.into());
                            }
                            result_stack.push_back(left_value ^ right_value);
                            var_stack.push_back(None);
                        }
                        Operator::Eql => {
                            if let Some(var) = left_var {
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
                    let n = heap.get(&var_name).unwrap_or(&Number::DecimalNumber(0.));
                    result_stack.push_back(n.clone());
                    var_stack.push_back(Some(var_name));
                }
                Token::Function(fun) => {
                    let value: Number = result_stack.pop_back().ok_or(anyhow!(
                        "{} {}",
                        MALFORMED_ERR,
                        "Wrong use of function"
                    ))?;
                    var_stack.pop_back();

                    let res = match fun {
                        MathFunction::Sin => f64::sin(value.into()),
                        MathFunction::Cos => f64::cos(value.into()),
                        MathFunction::Tan => f64::tan(value.into()),
                        MathFunction::ASin => f64::asin(value.into()),
                        MathFunction::ACos => f64::acos(value.into()),
                        MathFunction::ATan => f64::atan(value.into()),
                        MathFunction::Ln => f64::ln(value.into()),
                        MathFunction::Log => f64::log10(value.into()),
                        MathFunction::Abs => f64::abs(value.into()),
                        MathFunction::Max => {
                            let value2: Number = result_stack.pop_back().ok_or(anyhow!(
                                "{} {}",
                                MALFORMED_ERR,
                                "Wrong number of parameters for function Max"
                            ))?;
                            var_stack.pop_back();
                            f64::max(value.into(), value2.into())
                        }
                        MathFunction::Min => {
                            let value2: Number = result_stack.pop_back().ok_or(anyhow!(
                                "{} {}",
                                MALFORMED_ERR,
                                "Wrong number of parameters for function Min"
                            ))?;
                            var_stack.pop_back();
                            f64::min(value.into(), value2.into())
                        }
                        MathFunction::Sqrt => f64::sqrt(value.into()),
                        MathFunction::Floor => f64::floor(value.into()),
                        MathFunction::Ceil => f64::ceil(value.into()),
                        MathFunction::Round => f64::round(value.into()),
                        MathFunction::Exp => f64::exp(value.into()),
                        MathFunction::None => return Err(anyhow!("This should never happen!")),
                    };
                    result_stack.push_back(Number::DecimalNumber(res));
                    var_stack.push_back(None);
                }
                Token::SemiColon => {
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
        var_stack.pop_front();
        result_stack.pop_front().ok_or(anyhow!("{}", MALFORMED_ERR))
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
    ) -> (VecDeque<Token<'a>>, Rc<RefCell<HashMap<String, Number>>>) {
        /*  Create an empty stack for keeping operators. Create an empty list for output. */
        let mut operators_stack: Vec<Token> = Vec::new();
        let mut postfix_stack: VecDeque<Token> = VecDeque::new();

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
                    while let Some(token) = operators_stack.pop() {
                        match token {
                            Token::Bracket(token::Bracket::Open) => {
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
                }

                Token::Comma => {
                    while let Some(token) = operators_stack.last() {
                        if matches!(token, Token::Bracket(token::Bracket::Open)) {
                            break;
                        }
                        postfix_stack.push_back(operators_stack.pop().expect("It should not happen."));
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
                    let s = s.to_lowercase();
                    local_heap
                        .borrow_mut()
                        .entry(s) // let's not override consts
                        .or_insert(Number::NaturalNumber(Zero::zero()));
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
        operators_stack
            .iter()
            .for_each(|t| postfix_stack.push_back(t.clone()));

        debug!(
            "DEBUG: EOF - OUT {} - OP - {}",
            DisplayThisDeque(&postfix_stack),
            DisplayThatVec(&operators_stack)
        );

        (postfix_stack, local_heap)
    }

    fn factorial_helper(n: BigUint) -> BigUint {
        if n == BigUint::zero() {
            return BigUint::one();
        }

        let previous = n.clone() - BigUint::one();
        let sub_result = RpnResolver::factorial_helper(previous);
        n * sub_result
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
            RpnResolver::reverse_polish_notation(&a, Rc::new(RefCell::new(HashMap::new()))).0,
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
        assert_eq!(resolver.resolve().unwrap(), Number::DecimalNumber(2.0));

        let mut resolver = session.process("min(1,2)");
        assert_eq!(resolver.resolve().unwrap(), Number::DecimalNumber(1.0));

        let mut resolver = session.process("min(max(1,2),3)");
        assert_eq!(resolver.resolve().unwrap(), Number::DecimalNumber(2.0));
    }
}
