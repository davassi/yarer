use std::{
    fmt::Display,
    ops::{Add, BitXor, Div, Mul, Sub},
};

use log::debug;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Number {
    NaturalNumber(i32),
    DecimalNumber(f64),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Une, // unary neg ('-1')
    Eql,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Associate {
    LeftAssociative,
    RightAssociative,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Bracket {
    Open,
    Close,
}

/// The [Token] enum. It represents the smallest chunk of a math expression
///
/// It can be a
/// [Token::Operand] as 1,2,3,-4,-5,6.66 ...
/// [Token::Operator] as +,-,*,/ ...
/// [Token::Bracket] as [] or ()
/// [Token::Function] as sin,cos,tan,ln ...
/// [Token::Variable] as any variable name such as x,y,ab,foo,... whatever
///
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Token<'a> {
    Operand(Number),
    Operator(Operator),
    Bracket(Bracket),
    Function(MathFunction),
    Variable(&'a str),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MathFunction {
    Sin,
    Cos,
    Tan,
    ASin,
    ACos,
    ATan,
    Ln,
    Log,
    Abs,
    Sqrt,
    Max,
    Min,
    None,
}

pub static ZERO: crate::token::Number = Number::NaturalNumber(0);
pub static MINUS_ONE: crate::token::Number = Number::NaturalNumber(-1);

impl Token<'_> {
    /// Converts a char to a [Token::Operator]
    /// or just returns [None] if nothing matches.
    ///
    const fn from_operator(c: char) -> Option<Token<'static>> {
        match c {
            '+' => Some(Token::Operator(Operator::Add)),
            '-' => Some(Token::Operator(Operator::Sub)),
            '*' => Some(Token::Operator(Operator::Mul)),
            '/' => Some(Token::Operator(Operator::Div)),
            '^' => Some(Token::Operator(Operator::Pow)),
            '#' => Some(Token::Operator(Operator::Une)),
            '=' => Some(Token::Operator(Operator::Eql)),
            _ => None,
        }
    }

    /// Converts a char to a [Token::Bracket]
    /// or just returns [None] if nothing matches.
    ///
    const fn from_bracket(c: char) -> Option<Token<'static>> {
        match c {
            '(' | '[' => Some(Token::Bracket(Bracket::Open)),
            ')' | ']' => Some(Token::Bracket(Bracket::Close)),
            _ => None,
        }
    }

    /// Converts a &str to a [Token::Function(MathFunction)]
    /// or just returns [None] if nothing matches.
    ///
    fn get_some(fun: &str) -> Option<MathFunction> {
        match fun.to_lowercase().as_str() {
            "sin" => Some(MathFunction::Sin),
            "cos" => Some(MathFunction::Cos),
            "tan" => Some(MathFunction::Tan),
            "asin" => Some(MathFunction::Sin),
            "acos" => Some(MathFunction::Cos),
            "atan" => Some(MathFunction::Tan),
            "ln" => Some(MathFunction::Ln),
            "log" => Some(MathFunction::Log),
            "abs" => Some(MathFunction::Abs),
            "sqrt" => Some(MathFunction::Sqrt),
            //   "max" => MathFunction::Max,
            //   "min" => MathFunction::Min,
            &_ => None,
        }
    }

    /// Transforms a specific chunk of chars into a specific [Token]. i.e.
    ///
    /// "+"   -> [Token::Operator]
    /// "("   -> [Token::Bracket]
    /// "42"  -> [Token::Operand(Token::NaturalNumber)]
    /// "6.6" -> [Token::Operand(Token::DecimalNumber)]
    /// "sin" -> [Token::Function]
    /// "x"   -> [Token::Variable]
    ///
    fn tokenize(t: &str) -> Token {
        match t {
            c @ ("+" | "-" | "*" | "/" | "^" | "=") => {
                return Token::from_operator(c.chars().next().unwrap()).unwrap()
            }
            b @ ("(" | ")" | "[" | "]") => {
                return Token::from_bracket(b.chars().next().unwrap()).unwrap()
            }
            _ => (),
        }

        if let Ok(v) = t.parse::<i32>() {
            return Token::Operand(Number::NaturalNumber(v));
        }

        if let Ok(v) = t.parse::<f64>() {
            return Token::Operand(Number::DecimalNumber(v));
        }

        if let Some(fun) = Token::get_some(t) {
            return Token::Function(fun);
        }

        Token::Variable(t)
    }

    /// Mapping a vec of str in a vec of Tokens
    ///
    pub fn tokenize_vec<'a>(v: &[&'a str]) -> Vec<Token<'a>> {
        v.iter().map(|t| Token::tokenize(t)).collect::<Vec<Token>>()
    }

    /// Founding out the priority and the associative precedence of an operator
    ///
    fn operator_priority(o: Token) -> (u8, Associate) {
        match o {
            Token::Operator(Operator::Add) => (1, Associate::LeftAssociative),
            Token::Operator(Operator::Sub) => (1, Associate::LeftAssociative),
            Token::Operator(Operator::Mul) => (2, Associate::LeftAssociative),
            Token::Operator(Operator::Div) => (2, Associate::LeftAssociative),
            Token::Operator(Operator::Pow) => (3, Associate::RightAssociative),
            Token::Operator(Operator::Une) => (4, Associate::RightAssociative),
            Token::Operator(Operator::Eql) => (0, Associate::LeftAssociative),
            _ => panic!("Operator '{}' not recognised. This must not happen!", o),
        }
    }

    /// Checks if an operator has priority over another one
    ///
    /// i.e.
    /// * has priority over +
    /// ^ has priority over *
    /// unary - has priority over ^
    ///
    pub fn compare_operator_priority(op1: Token, op2: Token) -> bool {
        let v_op1: (u8, Associate) = self::Token::operator_priority(op1);
        let v_op2: (u8, Associate) = self::Token::operator_priority(op2);

        v_op1.1 == Associate::LeftAssociative && v_op1.0 <= v_op2.0
            || v_op1.1 == Associate::RightAssociative && v_op1.0 < v_op2.0
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Number::NaturalNumber(v) => write!(f, "{}", v),
            Number::DecimalNumber(v) => write!(f, "{}", v),
        }
    }
}

fn apply_functional_token_operation<NF, DF>(ln: Number, rn: Number, nf: NF, df: DF) -> Number
where
    NF: Fn(i32, i32) -> i32,
    DF: Fn(f64, f64) -> f64,
{
    match (ln, rn) {
        (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => Number::NaturalNumber(nf(v1, v2)),
        (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => {
            Number::DecimalNumber(df(v1 as f64, v2))
        }
        (Number::DecimalNumber(v1), _) => Number::DecimalNumber(df(v1, rn.into())),
    }
}

impl Add for Number {
    type Output = Number;

    fn add(self, rhs: Self) -> Self::Output {
        apply_functional_token_operation(self, rhs, |a: i32, b: i32| a + b, |a: f64, b: f64| a + b)
    }
}

impl Sub for Number {
    type Output = Number;

    fn sub(self, rhs: Self) -> Self::Output {
        apply_functional_token_operation(self, rhs, |a: i32, b: i32| a - b, |a: f64, b: f64| a - b)
    }
}

impl Mul for Number {
    type Output = Number;

    fn mul(self, rhs: Self) -> Self::Output {
        apply_functional_token_operation(self, rhs, |a: i32, b: i32| a * b, |a: f64, b: f64| a * b)
    }
}

impl Div for Number {
    type Output = Number;

    fn div(self, rhs: Self) -> Self::Output {
        apply_functional_token_operation(self, rhs, |a: i32, b: i32| a / b, |a: f64, b: f64| a / b)
    }
}

impl BitXor for Number {
    type Output = Number;

    fn bitxor(self, rhs: Self) -> Self::Output {
        debug!("{} {}", self, rhs);
        apply_functional_token_operation(
            self,
            rhs,
            |a: i32, b: i32| i32::pow(a, b.try_into().unwrap()),
            f64::powf,
        )
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (*self, *other) {
            (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => v1.partial_cmp(&v2),
            (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => (v1 as f64).partial_cmp(&v2),
            (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => v1.partial_cmp(&(v2 as f64)),
            (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => v1.partial_cmp(&v2),
        }
    }
}

impl From<Number> for f64 {
    fn from(n: Number) -> f64 {
        match n {
            Number::NaturalNumber(v) => v as f64,
            Number::DecimalNumber(v) => v,
        }
    }
}

impl From<Number> for i32 {
    fn from(n: Number) -> i32 {
        match n {
            Number::NaturalNumber(v) => v,
            Number::DecimalNumber(v) => v as i32,
        }
    }
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Operator::Add => write!(f, "+"),
            Operator::Sub => write!(f, "-"),
            Operator::Mul => write!(f, "*"),
            Operator::Div => write!(f, "/"),
            Operator::Pow => write!(f, "^"),
            Operator::Une => write!(f, "#"),
            Operator::Eql => write!(f, "="),
        }
    }
}

impl Display for Bracket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Open => write!(f, "("),
            Self::Close => write!(f, ")"),
        }
    }
}

impl Display for MathFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", *self)
    }
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Token::Operand(v) => write!(f, "({})", v),
            Token::Operator(v) => write!(f, "({})", v),
            Token::Bracket(v) => write!(f, "({})", v),
            Token::Function(v) => write!(f, "({})", v),
            Token::Variable(s) => write!(f, "({})", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenise_operators() {
        let v = vec!["1", "+", "2.1"];
        assert_eq!(Token::tokenize(v[1]), Token::Operator(Operator::Add));
        assert_eq!(
            Token::tokenize(v[0]),
            Token::Operand(Number::NaturalNumber(1))
        );
        assert_eq!(
            Token::tokenize(v[2]),
            Token::Operand(Number::DecimalNumber(2.1))
        );
    }

    #[test]
    fn test_from_operator_valid() {
        assert_eq!(
            Token::from_operator('+'),
            Some(Token::Operator(Operator::Add))
        );
        assert_eq!(
            Token::from_operator('-'),
            Some(Token::Operator(Operator::Sub))
        );
        assert_eq!(
            Token::from_operator('*'),
            Some(Token::Operator(Operator::Mul))
        );
        assert_eq!(
            Token::from_operator('/'),
            Some(Token::Operator(Operator::Div))
        );
    }

    #[test]
    fn test_from_operator_invalid() {
        assert_eq!(Token::from_operator('a'), None);
        assert_eq!(Token::from_operator('1'), None);
        assert_eq!(Token::from_operator('!'), None);
    }

    #[test]
    fn test_tokenize_valid() {
        assert_eq!(Token::tokenize("+"), Token::Operator(Operator::Add));
        assert_eq!(
            Token::tokenize("100"),
            (Token::Operand(Number::NaturalNumber(100)))
        );
        assert_eq!(
            Token::tokenize("3.14"),
            (Token::Operand(Number::DecimalNumber(3.14)))
        );
        assert_eq!(Token::tokenize("("), Token::Bracket(Bracket::Open));
    }

    #[test]
    fn test_tokenize_vec_valid() {
        let input = vec!["+", "100", "3.14", "("];
        let expected = vec![
            Token::Operator(Operator::Add),
            Token::Operand(Number::NaturalNumber(100)),
            Token::Operand(Number::DecimalNumber(3.14)),
            Token::Bracket(Bracket::Open),
        ];
        assert_eq!(Token::tokenize_vec(&input), expected);
    }
}
