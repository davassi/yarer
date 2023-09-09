use crate::{
    parser::Parser,
    token::{self, MathFunction, Number, Operator, Token, ZERO},
};
use anyhow::anyhow;
use log::debug;
use std::{collections::{HashMap, VecDeque}, rc::Rc, cell::RefCell, fmt::Display};

static MALFORMED_ERR: &str = "Runtime Error: The mathematical expression is malformed.";
static DIVISION_ZERO_ERR: &str = "Runtime error: Divide by zero.";
static NO_VARIABLE_ERR: &str = "Runtime error: No variable has been defined for assignent.";

/// The main [`RpnResolver`]: contains the core logic of Yarer
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
        let mut result_stack: VecDeque<Number> = VecDeque::new();

        let mut last_var_ref: Option<&str> = None;

        for t in &self.rpn_expr {
            match t {
                Token::Operand(n) => {
                    result_stack.push_back(*n);
                }
                Token::Operator(op) => {
                    let right_value: Number = result_stack
                        .pop_back()
                        .ok_or_else(|| anyhow!("{} {}", MALFORMED_ERR, "Invalid Right Operand."))?;

                    let mut left_value = if op != &Operator::Une && op != &Operator::Fac {
                        result_stack
                            .pop_back()
                            .ok_or_else(|| anyhow!("{} {}", MALFORMED_ERR, "Invalid Left Operand."))?
                    } else {
                        ZERO
                    };

                    match op {
                        Operator::Add => result_stack.push_back(left_value + right_value),
                        Operator::Sub => result_stack.push_back(left_value - right_value),
                        Operator::Mul => result_stack.push_back(left_value * right_value),
                        Operator::Div => {
                            if right_value == ZERO {
                                return Err(anyhow!(DIVISION_ZERO_ERR));
                            }
                            left_value = Number::DecimalNumber(left_value.into());
                            result_stack.push_back(left_value / right_value);
                        }
                        Operator::Pow => {
                            if right_value < ZERO {
                                if left_value == ZERO {
                                    return Err(anyhow!(DIVISION_ZERO_ERR));
                                }
                                left_value = Number::DecimalNumber(left_value.into());
                            }
                            result_stack.push_back(left_value ^ right_value);
                        }
                        Operator::Eql => {
                            if let Some(var) = last_var_ref {
                                self.local_heap.borrow_mut()
                                    .insert(var.to_string(), right_value);
                                debug!("Heap {:?}", self.local_heap);
                                result_stack.push_back(right_value);
                            } else {
                                return Err(anyhow!(NO_VARIABLE_ERR));
                            }
                        }
                        Operator::Fac => {
                            // factorial. Only for natural numbers
                            let v = i32::from(right_value);
                            if v<0 {
                                println!("Warning: Factorial of a Negative or Decimal number has not been yet implemented.");
                            }
                            result_stack.push_back(Number::NaturalNumber((1..=v).product()));
                        }
                        Operator::Une => {
                            //# unary neg
                            result_stack.push_back(right_value * token::MINUS_ONE);
                        }
                    }
                }
                Token::Variable(v) => {
                    let _ = last_var_ref.insert(*v);
                    debug!("Heap {:?}", self.local_heap);
                    let heap = self.local_heap.borrow_mut();
                    let n = heap
                        .get(*v)
                        .unwrap_or(&Number::DecimalNumber(0.));
                    result_stack.push_back(*n);
                }
                Token::Function(fun) => {
                    let value: Number = result_stack
                        .pop_back()
                        .ok_or(anyhow!("{} {}", MALFORMED_ERR, "Wrong use of function"))?;

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
                            let value2: Number = result_stack.pop_back().unwrap();
                            f64::max(value.into(), value2.into())
                        }
                        MathFunction::Min => {
                            let value2: Number = result_stack.pop_back().unwrap();
                            f64::min(value.into(), value2.into())
                        }
                        MathFunction::Sqrt => f64::sqrt(value.into()),
                        MathFunction::None => return Err(anyhow!("This should never happen!")),
                    };
                    result_stack.push_back(Number::DecimalNumber(res));
                }
                _ => return Err(anyhow!("{} Internal Error at line: {}.", MALFORMED_ERR, line!())),
            }
        }
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
        //infix_stack.iter().for_each(|t: &Token| {
        for t in infix_stack {
            match *t {
                /* If the token is an operand, add it to the output list. */
                Token::Operand(_) => postfix_stack.push_back(*t),

                /* If the token is a left parenthesis, push it on the stack. */
                Token::Bracket(token::Bracket::Open) => operators_stack.push(*t),

                /* If the token is a right parenthesis:
                    Pop the stack and add operators to the output list until you encounter a left parenthesis.
                    Pop the left parenthesis from the stack but do not add it to the output list.*/
                Token::Bracket(token::Bracket::Close) => {

                    while let Some(token) = operators_stack.pop() {
                        match token {
                            Token::Bracket(token::Bracket::Open) => break, // discards left parenthesis
                            _ => postfix_stack.push_back(token),
                        }
                    }
                },

                /* If the token is an operator, op1, then:
                   while there is an operator, op2, at the top of the stack, and op1 is left-associative 
                   and its precedence is less than or equal to that of op2, 
                   or op1 is right-associative and its precedence is less than that of op2:
                      pop op2 off the stack, onto the output list;
                    push op1 on the stack.*/
                Token::Operator(_op) => {
                    let op1: Token<'_> = *t;

                    while !operators_stack.is_empty() {
                        let op2: &Token = operators_stack.last().unwrap();
                        match op2 {
                            Token::Operator(_) => {
                                if Token::compare_operator_priority(op1, *op2) {
                                    postfix_stack.push_back(operators_stack.pop().unwrap());
                                } else {
                                    break;
                                }
                            },
                            Token::Function(_) => {
                                postfix_stack.push_back(operators_stack.pop().unwrap());
                            }
                            _ => break,
                        }
                    }
                    operators_stack.push(op1);
                },

                Token::Function(_) => {
                    operators_stack.push(*t);
                },

                /* If the token is a variable, add it to the output list and to the local_heap with a default value*/
                Token::Variable(s) => {
                    postfix_stack.push_back(*t);
                    let s = s.to_lowercase();
                    local_heap.borrow_mut().entry(s) // let's not override consts
                        .or_insert(token::ZERO);
                },
            }
            debug!("Inspecting... {} - OUT {} - OP - {}", *t, DisplayThisDeque(&postfix_stack), DisplayThatVec(&operators_stack));
        };

        /* After all tokens are read, pop remaining operators from the stack and add them to the list. */
        operators_stack.reverse();
        postfix_stack.extend(operators_stack.iter());

        debug!(
            "Inspecting... EOF - OUT {} - OP - {}", DisplayThisDeque(&postfix_stack), DisplayThatVec(&operators_stack)
        );

        (postfix_stack, local_heap)
    }
}

struct DisplayThatVec<'a>(&'a Vec<Token<'a>>);
struct DisplayThisDeque<'a>(&'a VecDeque<Token<'a>>);

impl Display for DisplayThatVec<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.iter().map(ToString::to_string).collect::<String>())
    }
}

impl Display for DisplayThisDeque<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.iter().map(ToString::to_string).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{Number, Operator};

    #[test]
    fn test_reverse_polish_notation() {
        let a: Vec<Token> = vec![
            Token::Operand(Number::NaturalNumber(1)),
            Token::Operator(Operator::Add),
            Token::Operand(Number::NaturalNumber(2)),
        ];
        let b: Vec<Token> = vec![
            Token::Operand(Number::NaturalNumber(1)),
            Token::Operand(Number::NaturalNumber(2)),
            Token::Operator(Operator::Add),
        ];
        assert_eq!(
            RpnResolver::reverse_polish_notation(&a, Rc::new(RefCell::new(HashMap::new()))).0,
            b
        );
    }
}
