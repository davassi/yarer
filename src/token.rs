use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Sub},
};

/// Enum Type [Number]. Either an BigInt integer [`Number::NaturalNumber`]
/// or a [`BigRational`] rational number [`Number::DecimalNumber`]
///
/// # Invariant
///
/// A [`Number::DecimalNumber`] never holds a denominator of 1. A value that is
/// mathematically a whole number is always a [`Number::NaturalNumber`], so every
/// mathematical value has exactly one representation and the two variants never
/// describe the same number.
///
/// [`Number::decimal`] is the constructor that maintains this, degrading an
/// integral rational to [`Number::NaturalNumber`]; everything inside the crate
/// builds decimals through it. Both variants are nevertheless publicly
/// constructible, so code that builds a [`Number::DecimalNumber`] directly is
/// responsible for upholding the rule itself — prefer [`Number::decimal`].
#[derive(Debug, Clone)]
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
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

impl MathFunction {
    /// How many arguments this function takes.
    ///
    /// [`MathFunction::None`] is unreachable — nothing in the tokenizer ever
    /// produces it — and reports 1 rather than panicking, so that no input can
    /// reach a panic through this path.
    ///
    /// The match is written out variant by variant, rather than falling back
    /// on a wildcard arm, so that adding a new function forces its author to
    /// state its arity here instead of silently inheriting 1.
    #[must_use]
    pub const fn arity(self) -> u8 {
        match self {
            MathFunction::Max | MathFunction::Min => 2,
            MathFunction::Sin
            | MathFunction::Cos
            | MathFunction::Tan
            | MathFunction::ASin
            | MathFunction::ACos
            | MathFunction::ATan
            | MathFunction::Ln
            | MathFunction::Log
            | MathFunction::Abs
            | MathFunction::Sqrt
            | MathFunction::Floor
            | MathFunction::Ceil
            | MathFunction::Round
            | MathFunction::Exp
            | MathFunction::Pdf
            | MathFunction::Cdf
            | MathFunction::None => 1,
        }
    }
}

impl Token<'_> {
    /// Converts a char to a [`Token::Operator`]
    /// or just returns [`None`] if nothing matches.
    ///
    const fn from_operator(c: char) -> Option<Token<'static>> {
        match c {
            '+' => Some(Token::Operator(Operator::Add)),
            '-' => Some(Token::Operator(Operator::Sub)),
            '*' | '×' => Some(Token::Operator(Operator::Mul)),
            '/' | '÷' => Some(Token::Operator(Operator::Div)),
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
            "log" | "log10" => Some(MathFunction::Log),
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
    pub(crate) fn tokenize(t: &str) -> Token<'_> {
        if let Some(s) = t.chars().next() {
            match s {
                c @ ('+' | '-' | '*' | '/' | '^' | '!' | '=' | '×' | '÷') => {
                    return Token::from_operator(c).unwrap()
                }
                b @ ('(' | ')' | '[' | ']') => return Token::from_bracket(b).unwrap(),
                ',' => return Token::Comma,
                ';' => return Token::SemiColon,
                _ => (), // continue the flow
            }
        }

        if let Ok(v) = t.parse::<BigInt>() {
            return Token::Operand(Number::NaturalNumber(v));
        }

        if let Some(v) = parse_decimal_literal(t) {
            return Token::Operand(Number::decimal(v));
        }

        if let Some(fun) = Token::get_some(t) {
            return Token::Function(fun);
        }

        Token::Variable(t)
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

impl Number {
    /// Builds a decimal number, degrading to [`Number::NaturalNumber`] when the
    /// rational turns out to be a whole number.
    ///
    /// This is the only sanctioned way to build a [`Number::DecimalNumber`]:
    /// it upholds the invariant that a decimal never carries a denominator of 1,
    /// so a given mathematical value has exactly one representation.
    #[must_use]
    pub fn decimal(value: BigRational) -> Number {
        if value.denom().is_one() {
            Number::NaturalNumber(value.to_integer())
        } else {
            Number::DecimalNumber(value)
        }
    }

    /// Returns the integral value of this number, or [`None`] when it has a
    /// fractional part.
    ///
    /// The decimal arm matters only for values built by hand from outside the
    /// crate, which can bypass [`Number::decimal`]; internally the invariant
    /// makes it unreachable.
    #[must_use]
    pub fn as_integer(&self) -> Option<BigInt> {
        match self {
            Number::NaturalNumber(v) => Some(v.clone()),
            Number::DecimalNumber(v) if v.denom().is_one() => Some(v.to_integer()),
            Number::DecimalNumber(_) => None,
        }
    }
}

/// Equality by mathematical value, so that it agrees with [`PartialOrd`].
///
/// The derived implementation compared enum variants, which made
/// `NaturalNumber(2) == DecimalNumber(2/1)` false while `>=` reported true —
/// a violation of the `PartialOrd` contract that generic code relies on.
impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::NaturalNumber(a), Number::NaturalNumber(b)) => a == b,
            (Number::DecimalNumber(a), Number::DecimalNumber(b)) => a == b,
            (Number::NaturalNumber(a), Number::DecimalNumber(b))
            | (Number::DecimalNumber(b), Number::NaturalNumber(a)) => {
                BigRational::from(a.clone()) == *b
            }
        }
    }
}

/// Let's display a [`Number::NaturalNumber`] or a [`Number::DecimalNumber`] properly
///
impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::NaturalNumber(v) => write!(f, "{v}"),
            Number::DecimalNumber(v) => {
                if v.denom().is_one() {
                    write!(f, "{}", v.to_integer())
                } else if let Some(fl) = v.to_f64() {
                    write!(f, "{fl}")
                } else {
                    write!(f, "{}/{}", v.numer(), v.denom())
                }
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
            Number::decimal(df(BigRational::from(v1), v2))
        }
        (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => {
            Number::decimal(df(v1, BigRational::from(v2)))
        }
        (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => Number::decimal(df(v1, v2)),
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
        match (self, rhs) {
            (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => {
                Number::decimal(BigRational::new(v1, v2))
            }
            (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => {
                Number::decimal(BigRational::from(v1) / v2)
            }
            (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => {
                Number::decimal(v1 / BigRational::from(v2))
            }
            (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => Number::decimal(v1 / v2),
        }
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

/// Error returned when a [`Number`] cannot be converted into a fixed-size
/// numeric type because the value falls outside that type's representable range.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConversionError {
    /// The value does not fit in the requested target type.
    #[error("value '{value}' is out of range for target type {target}")]
    OutOfRange {
        /// The offending value, rendered as a decimal string.
        value: String,
        /// The name of the target type that could not hold the value.
        target: &'static str,
    },
}

/// Converts a [`Number`] into a [`BigInt`], truncating any fractional part
/// toward zero. This is exact and infallible: a [`BigInt`] holds any integer.
impl From<Number> for BigInt {
    fn from(n: Number) -> BigInt {
        match n {
            Number::NaturalNumber(v) => v,
            // `to_integer` truncates the exact rational toward zero — no lossy f64 round-trip.
            Number::DecimalNumber(v) => v.to_integer(),
        }
    }
}

/// Fallible conversion to [`f64`]. Fails when the value cannot be represented
/// as a finite double (e.g. it exceeds [`f64::MAX`]).
impl TryFrom<Number> for f64 {
    type Error = ConversionError;

    fn try_from(n: Number) -> Result<Self, Self::Error> {
        let value = match &n {
            Number::NaturalNumber(v) => v.to_f64(),
            Number::DecimalNumber(v) => v.to_f64(),
        };
        value
            .filter(|f| f.is_finite())
            .ok_or_else(|| ConversionError::OutOfRange {
                value: n.to_string(),
                target: "f64",
            })
    }
}

/// Fallible conversion to [`i32`]: the fractional part is truncated toward zero,
/// then the integer must fit in the target type.
impl TryFrom<Number> for i32 {
    type Error = ConversionError;

    fn try_from(n: Number) -> Result<Self, Self::Error> {
        let value: BigInt = n.into();
        value.to_i32().ok_or_else(|| ConversionError::OutOfRange {
            value: value.to_string(),
            target: "i32",
        })
    }
}

/// Fallible conversion to [`i64`]: the fractional part is truncated toward zero,
/// then the integer must fit in the target type.
impl TryFrom<Number> for i64 {
    type Error = ConversionError;

    fn try_from(n: Number) -> Result<Self, Self::Error> {
        let value: BigInt = n.into();
        value.to_i64().ok_or_else(|| ConversionError::OutOfRange {
            value: value.to_string(),
            target: "i64",
        })
    }
}

/// Fallible conversion to [`i128`]: the fractional part is truncated toward zero,
/// then the integer must fit in the target type.
impl TryFrom<Number> for i128 {
    type Error = ConversionError;

    fn try_from(n: Number) -> Result<Self, Self::Error> {
        let value: BigInt = n.into();
        value.to_i128().ok_or_else(|| ConversionError::OutOfRange {
            value: value.to_string(),
            target: "i128",
        })
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
            Token::Comma => write!(f, "(,)"),
            Token::SemiColon => write!(f, "(;)"),
        }
    }
}

fn parse_decimal_literal(literal: &str) -> Option<BigRational> {
    let (whole, fractional) = literal.split_once('.')?;

    let whole = if whole.is_empty() {
        BigInt::zero()
    } else {
        whole.parse::<BigInt>().ok()?
    };
    let fractional = if fractional.is_empty() {
        BigInt::zero()
    } else {
        fractional.parse::<BigInt>().ok()?
    };
    let fractional_digits = literal
        .split_once('.')
        .map_or(0, |(_, digits)| digits.len());
    let mut exact_scale = BigInt::one();
    for _ in 0..fractional_digits {
        exact_scale *= 10_u8;
    }

    Some(BigRational::new(
        whole * exact_scale.clone() + fractional,
        exact_scale,
    ))
}

#[cfg(test)]
mod tests {
    use num::One;

    use super::*;

    #[test]
    fn test_tokenise_operators() {
        let v = vec!["1", "+", "2.1"];
        assert_eq!(Token::tokenize(v[1]), Token::Operator(Operator::Add));
        assert_eq!(
            Token::tokenize(v[0]),
            Token::Operand(Number::NaturalNumber(One::one()))
        );
        assert_eq!(
            Token::tokenize(v[2]),
            Token::Operand(Number::DecimalNumber(BigRational::new(
                BigInt::from(21),
                BigInt::from(10)
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
            Token::from_operator('×'),
            Some(Token::Operator(Operator::Mul))
        );
        assert_eq!(
            Token::from_operator('/'),
            Some(Token::Operator(Operator::Div))
        );
        assert_eq!(
            Token::from_operator('÷'),
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
        assert_eq!(Token::tokenize("+"), Token::Operator(Operator::Add));
        assert_eq!(
            Token::tokenize("100"),
            Token::Operand(Number::NaturalNumber(BigInt::from(100)))
        );
        assert_eq!(
            Token::tokenize("3.14"),
            Token::Operand(Number::DecimalNumber(BigRational::new(
                BigInt::from(157),
                BigInt::from(50)
            )))
        );
        assert_eq!(Token::tokenize("("), Token::Bracket(Bracket::Open));
    }

    #[test]
    fn test_tokenize_vec_valid() {
        assert_eq!(Token::tokenize("+"), Token::Operator(Operator::Add));
        assert_eq!(
            Token::tokenize("100"),
            Token::Operand(Number::NaturalNumber(BigInt::from(100)))
        );
        assert_eq!(
            Token::tokenize("3.14"),
            Token::Operand(Number::DecimalNumber(BigRational::new(
                BigInt::from(157),
                BigInt::from(50)
            )))
        );
        assert_eq!(Token::tokenize("("), Token::Bracket(Bracket::Open));
    }

    #[test]
    fn test_tryfrom_i32_out_of_range_is_err_not_panic() {
        // 2^100 is a valid NaturalNumber that does not fit in i32:
        // the conversion must return Err, never panic.
        let big = Number::NaturalNumber(BigInt::from(2).pow(100));
        assert!(i32::try_from(big).is_err());
    }

    #[test]
    fn test_tryfrom_i64_in_range_ok() {
        let n = Number::NaturalNumber(BigInt::from(3_265_920));
        assert_eq!(i64::try_from(n).unwrap(), 3_265_920_i64);
    }

    #[test]
    fn test_decimal_to_bigint_is_exact_for_large_values() {
        // f64 cannot represent 10^30 + 1 exactly, so a round-trip through f64
        // would lose the +1. Exact conversion via to_integer() must preserve it.
        let big = BigInt::from(10).pow(30) + BigInt::from(1);
        let n = Number::DecimalNumber(BigRational::from_integer(big.clone()));
        assert_eq!(BigInt::from(n), big);
    }

    #[test]
    fn test_decimal_to_bigint_truncates_toward_zero() {
        let pos = Number::DecimalNumber(BigRational::new(BigInt::from(7), BigInt::from(2)));
        assert_eq!(BigInt::from(pos), BigInt::from(3));
        let neg = Number::DecimalNumber(BigRational::new(BigInt::from(-7), BigInt::from(2)));
        assert_eq!(BigInt::from(neg), BigInt::from(-3));
    }

    #[test]
    fn test_tryfrom_f64_ok_and_overflow_is_err() {
        let half = Number::DecimalNumber(BigRational::new(BigInt::from(1), BigInt::from(2)));
        assert!((f64::try_from(half).unwrap() - 0.5_f64).abs() < f64::EPSILON);
        // 10^400 exceeds f64::MAX: must error, not silently become infinity.
        let huge = Number::NaturalNumber(BigInt::from(10).pow(400));
        assert!(f64::try_from(huge).is_err());
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

    #[test]
    fn test_operator_priority_for_assignment() {
        assert_eq!(
            Token::operator_priority(Token::Operator(Operator::Eql)),
            (0, Associate::RightAssociative)
        );
    }

    #[test]
    fn test_tokenize_edge_cases() {
        // The regex feeding `tokenize` never produces an empty match, so this
        // is unreachable in the real pipeline; the total function still needs
        // a defined answer, and an empty chunk falls through to a variable
        // with an empty name.
        assert_eq!(Token::tokenize(""), Token::Variable(""));
        assert_eq!(Token::tokenize("["), Token::Bracket(Bracket::Open));
        assert_eq!(Token::tokenize("]"), Token::Bracket(Bracket::Close));
        assert_eq!(Token::tokenize(";"), Token::SemiColon);
        assert_eq!(Token::tokenize(","), Token::Comma);
        assert_eq!(Token::tokenize("×"), Token::Operator(Operator::Mul));
        assert_eq!(Token::tokenize("÷"), Token::Operator(Operator::Div));
        assert_eq!(Token::tokenize("foo"), Token::Variable("foo"));
    }

    #[test]
    fn test_tokenize_functions_are_case_insensitive() {
        assert_eq!(Token::tokenize("SIN"), Token::Function(MathFunction::Sin));
        assert_eq!(Token::tokenize("Cos"), Token::Function(MathFunction::Cos));
        assert_eq!(Token::tokenize("log10"), Token::Function(MathFunction::Log));
    }

    #[test]
    fn test_parse_decimal_literal_variants() {
        assert_eq!(
            parse_decimal_literal(".5"),
            Some(BigRational::new(BigInt::from(1), BigInt::from(2)))
        );
        assert_eq!(
            parse_decimal_literal("1."),
            Some(BigRational::from_integer(BigInt::from(1)))
        );
        assert_eq!(
            parse_decimal_literal("3.14"),
            Some(BigRational::new(BigInt::from(157), BigInt::from(50)))
        );
        assert_eq!(
            parse_decimal_literal("0.001"),
            Some(BigRational::new(BigInt::from(1), BigInt::from(1000)))
        );
        // a token without a '.' is not a decimal literal
        assert_eq!(parse_decimal_literal("42"), None);
    }

    #[test]
    fn test_number_display() {
        assert_eq!(Number::NaturalNumber(BigInt::from(5)).to_string(), "5");
        // a rational that reduces to a whole number prints as an integer
        assert_eq!(
            Number::DecimalNumber(BigRational::new(BigInt::from(4), BigInt::from(2))).to_string(),
            "2"
        );
        assert_eq!(
            Number::DecimalNumber(BigRational::new(BigInt::from(1), BigInt::from(2))).to_string(),
            "0.5"
        );
        // 1/3 is not a finite decimal, so it is rendered via its f64 approximation
        let third = Number::DecimalNumber(BigRational::new(BigInt::from(1), BigInt::from(3)));
        assert_eq!(third.to_string(), format!("{}", 1.0_f64 / 3.0));
    }

    #[test]
    fn test_conversion_error_reports_target_type() {
        let big = Number::NaturalNumber(BigInt::from(2).pow(100));
        let msg = i32::try_from(big).unwrap_err().to_string();
        assert!(msg.contains("i32"), "message was: {msg}");
        assert!(msg.contains("out of range"), "message was: {msg}");
    }

    #[test]
    fn test_tryfrom_ok_paths() {
        assert_eq!(
            i32::try_from(Number::NaturalNumber(BigInt::from(42))).unwrap(),
            42_i32
        );
        // a decimal is truncated toward zero before the range check
        assert_eq!(
            i32::try_from(Number::DecimalNumber(BigRational::new(
                BigInt::from(7),
                BigInt::from(2)
            )))
            .unwrap(),
            3_i32
        );
        assert_eq!(
            i128::try_from(Number::NaturalNumber(BigInt::from(2).pow(70))).unwrap(),
            1_180_591_620_717_411_303_424_i128
        );
        assert!(
            (f64::try_from(Number::NaturalNumber(BigInt::from(10))).unwrap() - 10.0).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn test_decimal_constructor_degrades_integral_rationals() {
        // 4/2 reduces to 2, which is an integer: it must not stay tagged as decimal.
        let n = Number::decimal(BigRational::new(BigInt::from(4), BigInt::from(2)));
        assert_eq!(n, Number::NaturalNumber(BigInt::from(2)));
        assert!(matches!(n, Number::NaturalNumber(_)));

        // 1/2 is not integral and must stay decimal.
        let half = Number::decimal(BigRational::new(BigInt::from(1), BigInt::from(2)));
        assert!(matches!(half, Number::DecimalNumber(_)));
    }

    #[test]
    fn test_as_integer_reads_the_value_not_the_tag() {
        assert_eq!(
            Number::NaturalNumber(BigInt::from(7)).as_integer(),
            Some(BigInt::from(7))
        );
        // Built by hand, bypassing the constructor: the value is still integral.
        assert_eq!(
            Number::DecimalNumber(BigRational::from_integer(BigInt::from(7))).as_integer(),
            Some(BigInt::from(7))
        );
        assert_eq!(
            Number::DecimalNumber(BigRational::new(BigInt::from(3), BigInt::from(2))).as_integer(),
            None
        );
    }

    #[test]
    fn test_eq_agrees_with_partial_cmp_across_variants() {
        use std::cmp::Ordering;
        let pairs = [
            (
                Number::NaturalNumber(BigInt::from(2)),
                Number::DecimalNumber(BigRational::from_integer(BigInt::from(2))),
            ),
            (
                Number::NaturalNumber(BigInt::from(-3)),
                Number::DecimalNumber(BigRational::new(BigInt::from(-6), BigInt::from(2))),
            ),
            (
                Number::NaturalNumber(BigInt::from(2)),
                Number::DecimalNumber(BigRational::new(BigInt::from(5), BigInt::from(2))),
            ),
        ];
        for (a, b) in pairs {
            let equal_by_eq = a == b;
            let equal_by_ord = a.partial_cmp(&b) == Some(Ordering::Equal);
            assert_eq!(
                equal_by_eq, equal_by_ord,
                "PartialEq and PartialOrd disagree on {a} vs {b}"
            );
        }
    }

    #[test]
    fn test_arity_of_the_two_argument_functions() {
        assert_eq!(MathFunction::Max.arity(), 2);
        assert_eq!(MathFunction::Min.arity(), 2);
        assert_eq!(MathFunction::Sin.arity(), 1);
        assert_eq!(MathFunction::Cdf.arity(), 1);
    }
}
