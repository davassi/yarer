use num_traits::ToPrimitive;
use num_rational::BigRational;
use log::debug;
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use std::{
    fmt::Display,
    ops::{Add, BitXor, Div, Mul, Sub},
};

/// Enum Type [Number]. Either an BigInt integer [`Number::NaturalNumber`]
/// or a [`BigRational`] rational number [`Number::DecimalNumber`]
///
#[derive(Debug, PartialEq, Clone)]
pub enum Number {
    /// an Integer [BigInt]
    NaturalNumber(BigInt),
    /// a Rational number [BigRational]
    DecimalNumber(BigRational),
}

/// A binary or unary Math [`Operator`]
///
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Operator {
    /// Binary Add ('1+1')
    Add,
    /// Binary Sub ('2-1')
    Sub,
    /// Binary Mul ('2*2')
    Mul,
    /// Binary Div ('3/3')
    Div,
    /// Binary Pow ('base^exponent')
    Pow,
    /// Unary Neg ('-1')
    Une,
    /// Factorial ('0!')
    Fac,
    /// Binary Assignment ('A=1')
    Eql,
}

/// The "associativity" of an operator dictates the direction
/// in which operations of equal precedence are evaluated when they appear
///
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Associate {
    /// If an operator is left-associative, then operations are evaluated from left to right.
    /// Example: -a^b, -1, -(-3)
    ///
    LeftAssociative,
    /// If an operator is right-associative, then operations are evaluated from right to left.
    /// Example: A=1
    ///
    RightAssociative,
}

/// Just [`Token::Bracket`]s. They change the order of evaluation of an expression.
///
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Bracket {
    /// either '(' or '['
    Open,
    /// either ')' or ']'
    Close,
}

/// The [Token] enum. It represents the smallest chunk of a math expression
///
/// It can be a
/// [`Token::Operand`] as 1,2,3,-4,-5,6.66 ...
/// [`Token::Operator`] as +,-,*,/ ...
/// [`Token::Bracket`] as [] or ()
/// [`Token::Function`] as sin,cos,tan,ln ...
/// [`Token::Variable`] as any variable name such as x,y,ab,foo,... whatever
///
#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a> {
    /// Natural numbers (1,2,3,4...) or their decimals (1.1, 2.3, 4.4 ...)
    Operand(Number),
    /// Operators +,-,/,*,^...
    Operator(Operator),
    /// ( ) [ ]
    Bracket(Bracket),
    /// sin cos tan ln log...
    Function(MathFunction),
    /// comma separator for function arguments
    Comma,
    /// a b c x y ...
    Variable(&'a str),
    /// Semicolon ';' separator for chained expressions
    SemiColon,
}

/// The [`MathFunction`] enum. It represents a common math function.
///
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MathFunction {
    /// Trigonometric Sine
    Sin,
    /// Trigonometric Cosine
    Cos,
    /// Trigonometric Tangent
    Tan,
    /// Arcsine
    ASin,
    /// Arccosine
    ACos,
    /// Arctangent
    ATan,
    /// Natural logarithm
    Ln,
    /// Base 10 logarithm
    Log,
    /// Absolute value
    Abs,
    /// Square root
    Sqrt,
    /// Max value
    Max,
    /// Min value
    Min,
    /// Rounds down
    Floor,
    /// Rounds up
    Ceil,
    /// Rounds to nearest integer
    Round,
    /// e^x exponentiation
    Exp,
    /// Standard Normal probability density function
    Pdf,
    /// Standard Normal cumulative distribution function
    Cdf,
    /// No function expected
    None,
}

impl Token<'_> {
    /// Converts a char to a [`Token::Operator`]
    /// or just returns [`None`] if nothing matches.
    ///
    const fn from_operator(c: char) -> Option<Token<'static>> {
        match c {
            '+' => Some(Token::Operator(Operator::Add)),
            '-' => Some(Token::Operator(Operator::Sub)),
            '*' => Some(Token::Operator(Operator::Mul)),
            '/' => Some(Token::Operator(Operator::Div)),
            '^' => Some(Token::Operator(Operator::Pow)),
            '#' => Some(Token::Operator(Operator::Une)),
            '!' => Some(Token::Operator(Operator::Fac)),
            '=' => Some(Token::Operator(Operator::Eql)),
            _ => None,
        }
    }

    /// Converts a char to a [`Token::Bracket`]
    /// or just returns [`None`] if nothing matches.
    ///
    const fn from_bracket(c: char) -> Option<Token<'static>> {
        match c {
            '(' | '[' => Some(Token::Bracket(Bracket::Open)),
            ')' | ']' => Some(Token::Bracket(Bracket::Close)),
            _ => None,
        }
    }

    /// Converts a &str to a [`Token::Function(MathFunction)`]
    /// or just returns [`None`] if nothing matches.
    ///
    fn get_some(fun: &str) -> Option<MathFunction> {
        match fun.to_lowercase().as_str() {
            "sin" => Some(MathFunction::Sin),
            "cos" => Some(MathFunction::Cos),
            "tan" => Some(MathFunction::Tan),
            "asin" => Some(MathFunction::ASin),
            "acos" => Some(MathFunction::ACos),
            "atan" => Some(MathFunction::ATan),
            "ln" => Some(MathFunction::Ln),
            "log" => Some(MathFunction::Log),
            "abs" => Some(MathFunction::Abs),
            "sqrt" => Some(MathFunction::Sqrt),
            "max" => Some(MathFunction::Max),
            "min" => Some(MathFunction::Min),
            "floor" => Some(MathFunction::Floor),
            "ceil" => Some(MathFunction::Ceil),
            "round" => Some(MathFunction::Round),
            "exp" => Some(MathFunction::Exp),
            "pdf" => Some(MathFunction::Pdf),
            "cdf" => Some(MathFunction::Cdf),
            &_ => None,
        }
    }

    /// Transforms a specific chunk of chars into a specific [Token]. i.e.
    ///
    /// "+"   -> [`Token::Operator`]
    /// "("   -> [`Token::Bracket`]
    /// "42"  -> [`Token::Operand(Token::NaturalNumber)`]
    /// "6.6" -> [`Token::Operand(Token::DecimalNumber)`]
    /// "sin" -> [`Token::Function`]
    /// "x"   -> [`Token::Variable`]
    ///
    #[must_use]
    pub fn tokenize(t: &str) -> Option<Token> {
        match t.chars().next() {
            Some(s) => match s {
                c @ ('+' | '-' | '*' | '/' | '^' | '!' | '=') => {
                    return Some(Token::from_operator(c).unwrap())
                }
                b @ ('(' | ')' | '[' | ']') => return Some(Token::from_bracket(b).unwrap()),
                ',' => return Some(Token::Comma),
                ';' => return Some(Token::SemiColon),
                _ => (), // continue the flow
            },
            None => return None,
        }

        if let Ok(v) = t.parse::<BigInt>() {
            return Some(Token::Operand(Number::NaturalNumber(v)));
        }

        if let Ok(v) = t.parse::<f64>() {
            if let Some(r) = BigRational::from_float(v) {
                return Some(Token::Operand(Number::DecimalNumber(r)));
            }
        }

        if let Some(fun) = Token::get_some(t) {
            return Some(Token::Function(fun));
        }

        Some(Token::Variable(t))
    }

    /// Founding out the priority and the associative precedence of an operator
    ///
    fn operator_priority(o: Token) -> (u8, Associate) {
        match o {
            Token::Operator(Operator::Add | Operator::Sub) => (1, Associate::LeftAssociative),
            Token::Operator(Operator::Mul | Operator::Div) => (2, Associate::LeftAssociative),
            Token::Operator(Operator::Pow) => (3, Associate::RightAssociative),
            Token::Operator(Operator::Une) => (4, Associate::RightAssociative),
            Token::Operator(Operator::Fac) => (5, Associate::LeftAssociative),
            Token::Operator(Operator::Eql) => (0, Associate::RightAssociative),
            _ => panic!("Operator '{o}' not recognised. This must not happen!"),
        }
    }

    /// Checks if an operator has priority over another one
    ///
    /// i.e.
    /// * has priority over +
    /// ^ has priority over *
    /// unary - has priority over ^
    ///
    #[must_use]
    pub fn compare_operator_priority(op1: Token, op2: Token) -> bool {
        let v_op1: (u8, Associate) = self::Token::operator_priority(op1);
        let v_op2: (u8, Associate) = self::Token::operator_priority(op2);

        v_op1.1 == Associate::LeftAssociative && v_op1.0 <= v_op2.0
            || v_op1.1 == Associate::RightAssociative && v_op1.0 < v_op2.0
    }
}

/// Let's display a [`Number::NaturalNumber`] or a [`Number::DecimalNumber`] properly
///
impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::NaturalNumber(v) => write!(f, "{v}"),
            Number::DecimalNumber(v) => {
                let fl = v.to_f64().expect("Should not happen");
                write!(f, "{fl}")
            }
        }
    }
}

/// The main operational functional closure. It handles 4 different cases:
///
/// 1. Natural (op) Natural returns Natural
/// 2. Natural (op) Decimal returns Decimal
/// 3. Decimal (op) Decimal returns Decimal
/// 4. Decimal (op) Natural returns Decimal
///
/// (op) can be [Add], [Mul], [Sub], [Div], [BitXor], ...
///
/// We define 2 closures: 1 specialised for Natural Numbers and the other one specialised for Decimals.
///
fn apply_functional_token_operation<NF, DF>(ln: Number, rn: Number, nf: NF, df: DF) -> Number
where
    NF: Fn(BigInt, BigInt) -> BigInt,
    DF: Fn(BigRational, BigRational) -> BigRational,
{
    match (ln, rn.clone()) {
        (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => Number::NaturalNumber(nf(v1, v2)),
        (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => {
            Number::DecimalNumber(df(BigRational::from(v1), v2))
        }
        (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => {
            Number::DecimalNumber(df(v1, BigRational::from(v2)))
        }
        (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => Number::DecimalNumber(df(v1, v2)),
    }
}

impl Add for Number {
    type Output = Number;

    fn add(self, rhs: Self) -> Self::Output {
        apply_functional_token_operation(self, rhs, |a, b| a + b, |a, b| a + b)
    }
}

impl Sub for Number {
    type Output = Number;

    fn sub(self, rhs: Self) -> Self::Output {
        apply_functional_token_operation(self, rhs, |a, b| a - b, |a, b| a - b)
    }
}

impl Mul for Number {
    type Output = Number;

    fn mul(self, rhs: Self) -> Self::Output {
        apply_functional_token_operation(self, rhs, |a, b| a * b, |a, b| a * b)
    }
}

impl Div for Number {
    type Output = Number;

    fn div(self, rhs: Self) -> Self::Output {
        apply_functional_token_operation(self, rhs, |a, b| a / b, |a, b| a / b)
    }
}

impl BitXor for Number {
    type Output = Number;

    fn bitxor(self, rhs: Self) -> Self::Output {
        debug!("{} {}", self, rhs);
        apply_functional_token_operation(
            self,
            rhs,
            |a, b| BigInt::pow(&a, b.try_into().unwrap()),
            |a, b| {
                let af = a.to_f64().expect("Should not happen");
                let bf = b.to_f64().expect("Should not happen");
                BigRational::from_float(f64::powf(af, bf)).expect("Should not happen")
            },
        )
    }
}

/// PartialOrd between [Number]s with the required conversions.
///
impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => v1.partial_cmp(&v2),
            (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => {
                BigRational::from(v1.clone()).partial_cmp(v2)
            }
            (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => {
                v1.partial_cmp(&BigRational::from(v2.clone()))
            }
            (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => v1.partial_cmp(&v2),
        }
    }
}

impl From<Number> for f64 {
    fn from(n: Number) -> f64 {
        match n {
            Number::NaturalNumber(v) => ToPrimitive::to_f64(&v).expect("Should not happen"),
            Number::DecimalNumber(v) => v.to_f64().expect("Should not happen"),
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
impl From<Number> for BigInt {
    fn from(n: Number) -> BigInt {
        match n {
            Number::NaturalNumber(v) => v,
            Number::DecimalNumber(v) => {
                BigInt::from_f64(v.to_f64().expect("Should not happen")).expect("Should not happen")
            }
        }
    }
}

impl From<Number> for i32 {
    fn from(n: Number) -> i32 {
        match n {
            Number::NaturalNumber(v) => ToPrimitive::to_i32(&v).expect("Should not happen"),
            Number::DecimalNumber(v) => ToPrimitive::to_i32(&BigInt::from_f64(v.to_f64().expect("Should not happen")).expect("Should not happen")).expect("Should not happen"),
        }
    }
}

impl From<Number> for i64 {
    fn from(n: Number) -> i64 {
        match n {
            Number::NaturalNumber(v) => ToPrimitive::to_i64(&v).expect("Should not happen"),
            Number::DecimalNumber(v) => ToPrimitive::to_i64(&BigInt::from_f64(v.to_f64().expect("Should not happen")).expect("Should not happen")).expect("Should not happen"),
        }
    }
}

impl From<Number> for i128 {
    fn from(n: Number) -> i128 {
        match n {
            Number::NaturalNumber(v) => ToPrimitive::to_i128(&v).expect("Should not happen"),
            Number::DecimalNumber(v) => ToPrimitive::to_i128(&BigInt::from_f64(v.to_f64().expect("Should not happen")).expect("Should not happen")).expect("Should not happen"),
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
            Operator::Fac => write!(f, "!"),
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
        match self {
            Token::Operand(v) => write!(f, "({v})"),
            Token::Operator(v) => write!(f, "({v})"),
            Token::Bracket(v) => write!(f, "({v})"),
            Token::Function(v) => write!(f, "({v})"),
            Token::Variable(v) => write!(f, "({v})"),
            Token::Comma => write!(f, "(,)") ,
            Token::SemiColon => write!(f, "(;)")
        }
    }
}

#[cfg(test)]
mod tests {
    use num::One;

    use super::*;

    #[test]
    fn test_tokenise_operators() {
        let v = vec!["1", "+", "2.1"];
        assert_eq!(Token::tokenize(v[1]), Some(Token::Operator(Operator::Add)));
        assert_eq!(
            Token::tokenize(v[0]),
            Some(Token::Operand(Number::NaturalNumber(One::one())))
        );
        assert_eq!(
            Token::tokenize(v[2]),
            Some(Token::Operand(Number::DecimalNumber(
                BigRational::from_float(2.1).unwrap()
            )))
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
        assert_eq!(
            Token::from_operator('!'),
            Some(Token::Operator(Operator::Fac))
        );
    }

    #[test]
    fn test_from_operator_invalid() {
        assert_eq!(Token::from_operator('a'), None);
        assert_eq!(Token::from_operator('1'), None);
        assert_eq!(Token::from_operator('~'), None);
    }

    #[test]
    fn test_tokenize_valid() {
        assert_eq!(Token::tokenize("+"), Some(Token::Operator(Operator::Add)));
        assert_eq!(
            Token::tokenize("100"),
            Some(Token::Operand(Number::NaturalNumber(BigInt::from(100))))
        );
        assert_eq!(
            Token::tokenize("3.14"),
            Some(Token::Operand(Number::DecimalNumber(
                BigRational::from_float(3.14).unwrap()
            )))
        );
        assert_eq!(Token::tokenize("("), Some(Token::Bracket(Bracket::Open)));
    }

    #[test]
    fn test_tokenize_vec_valid() {
        assert_eq!(Token::tokenize("+"), Some(Token::Operator(Operator::Add)));
        assert_eq!(
            Token::tokenize("100"),
            Some(Token::Operand(Number::NaturalNumber(BigInt::from(100))))
        );
        assert_eq!(
            Token::tokenize("3.14"),
            Some(Token::Operand(Number::DecimalNumber(
                BigRational::from_float(3.14).unwrap()
            )))
        );
        assert_eq!(Token::tokenize("("), Some(Token::Bracket(Bracket::Open)));
    }

    #[test]
    fn test_operator_priority() {
        assert_eq!(
            Token::operator_priority(Token::Operator(Operator::Add)),
            (1, Associate::LeftAssociative)
        );
        assert_eq!(
            Token::operator_priority(Token::Operator(Operator::Sub)),
            (1, Associate::LeftAssociative)
        );
        assert_eq!(
            Token::operator_priority(Token::Operator(Operator::Mul)),
            (2, Associate::LeftAssociative)
        );
        assert_eq!(
            Token::operator_priority(Token::Operator(Operator::Div)),
            (2, Associate::LeftAssociative)
        );
        assert_eq!(
            Token::operator_priority(Token::Operator(Operator::Pow)),
            (3, Associate::RightAssociative)
        );
        assert_eq!(
            Token::operator_priority(Token::Operator(Operator::Une)),
            (4, Associate::RightAssociative)
        );
        assert_eq!(
            Token::operator_priority(Token::Operator(Operator::Fac)),
            (5, Associate::LeftAssociative)
        );
    }
}
