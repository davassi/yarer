use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use std::{
    fmt::Display,
    ops::{Add, Mul, Sub},
};

/// Enum Type [Number]. Either an `BigInt` integer [`Number::NaturalNumber`]
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
/// integral rational to [`Number::NaturalNumber`], and it is the one to reach
/// for from outside. Four paths inside the crate — `checked_div`, the shared
/// `Add`/`Sub`/`Mul` closure, `power_integer` and `decimal_from_f64` — use a
/// `decimal_unchecked` that skips the reduction, because `BigRational` has
/// already reduced the value they hand it; the invariant holds on those paths
/// too, just without paying for a second gcd. Both variants are nevertheless
/// publicly constructible, so code that builds a [`Number::DecimalNumber`]
/// directly is responsible for upholding the rule itself — prefer
/// [`Number::decimal`].
#[derive(Debug, Clone)]
pub enum Number {
    /// an Integer [`BigInt`]
    NaturalNumber(BigInt),
    /// a Rational number [`BigRational`]
    DecimalNumber(BigRational),
}

/// A binary or unary Math [`Operator`]
///
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Operator {
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
    /// Binary Assignment ('a=1'). Named for what it does: [`Operator::Equal`]
    /// is the comparison, and the two must not be confusable at a glance.
    Assign,
    /// Less than ('1<2')
    Less,
    /// Greater than ('2>1')
    Greater,
    /// Less than or equal ('1<=1')
    LessEq,
    /// Greater than or equal ('1>=1')
    GreaterEq,
    /// Equality ('1==1'). Not assignment; see [`Operator::Assign`].
    Equal,
    /// Inequality ('1<>2'). Spelled `<>` rather than `!=` because `!` is the
    /// postfix factorial, which would make `5!=3` ambiguous.
    NotEqual,
    /// Logical and ('1 and 0')
    And,
    /// Logical or ('1 or 0')
    Or,
    /// Logical exclusive or ('1 xor 0')
    Xor,
    /// Logical negation ('not 0'). Prefix, and spelled as a word because `!` is taken.
    Not,
    /// Remainder, truncating toward zero ('7 mod 3')
    Mod,
}

impl Operator {
    /// Whether this operator takes no left operand.
    ///
    /// The evaluation loop asks twice — once for the value and once for the
    /// variable name beside it — and spelled the answer out both times before
    /// this existed.
    pub(crate) const fn is_unary(self) -> bool {
        matches!(self, Operator::Une | Operator::Fac | Operator::Not)
    }

    /// Whether this operator is written before its operand.
    ///
    /// A prefix operator appears where a *value* appears, so every operator
    /// already waiting on the shunting yard's stack is still short of its own
    /// right operand and none of them may be displaced — whatever the
    /// precedence arithmetic says. [`Operator::Une`] never had to state this:
    /// at the second-strongest level it is stronger than anything it could
    /// displace. [`Operator::Not`] is the weakest of the three unary
    /// operators, and without the rule `1 - not 0` pops the `-` before its
    /// right operand exists.
    ///
    /// [`Operator::Fac`] is unary but *postfix* — it consumes the value on its
    /// left — so it is not one of these, and popping on its behalf is correct.
    pub(crate) const fn is_prefix(self) -> bool {
        matches!(self, Operator::Une | Operator::Not)
    }
}

/// The "associativity" of an operator dictates the direction
/// in which operations of equal precedence are evaluated when they appear
///
#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum Associate {
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
pub(crate) enum Bracket {
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
pub(crate) enum Token<'a> {
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
/// `#[non_exhaustive]`, and for a sharper reason than the other public enums
/// here: this is the one the README explicitly commits to growing ("More to
/// come!"), and it is public only because it appears inside
/// [`crate::ParseError::WrongArity`]. Payload inside a `#[non_exhaustive]`
/// error enum still breaks a caller who matched it exhaustively, so adding a
/// function in a later stage would have been a breaking change by the back
/// door. Match with a `_` arm.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
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
    /// No function expected.
    ///
    /// It cannot occur through parsing: `Token::get_some` never yields it, so
    /// no expression compiles to a `MathFunction::None`, and the evaluator
    /// answers [`crate::EvalError::Malformed`] if one somehow arrives. It is
    /// public and unconstructible-by-parsing, which the register proposes
    /// removing; that is a design change rather than a fix, so it stays for
    /// now.
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
            '=' => Some(Token::Operator(Operator::Assign)),
            '<' => Some(Token::Operator(Operator::Less)),
            '>' => Some(Token::Operator(Operator::Greater)),
            _ => None,
        }
    }

    /// Converts a whole token to the [`Operator`] it spells, for the operators
    /// written as words, or [`None`] if it spells none of them.
    ///
    /// Case-insensitive, because every other word in this language is: `and`,
    /// `And` and `AND` all work, exactly as `sin`, `Sin` and `SIN` do. Lower
    /// case is the canonical spelling for documentation and error messages.
    ///
    /// Asked before [`Token::get_some`] and long before the fall-through to
    /// [`Token::Variable`], which is what makes these five words reserved.
    fn from_word_operator(t: &str) -> Option<Operator> {
        match t.to_lowercase().as_str() {
            "and" => Some(Operator::And),
            "or" => Some(Operator::Or),
            "xor" => Some(Operator::Xor),
            "not" => Some(Operator::Not),
            "mod" => Some(Operator::Mod),
            _ => None,
        }
    }

    /// Converts a whole token to the [`Operator`] it spells, for the operators
    /// written with two characters, or [`None`] if it spells none of them.
    ///
    /// This asks about the whole token rather than its first character, and is
    /// asked before [`Token::from_operator`] is: the first character of `<=` is
    /// `<`, which on its own is a perfectly good comparison.
    fn from_two_char_operator(t: &str) -> Option<Operator> {
        match t {
            "<=" => Some(Operator::LessEq),
            ">=" => Some(Operator::GreaterEq),
            "==" => Some(Operator::Equal),
            "<>" => Some(Operator::NotEqual),
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
        // Ahead of the single-character route below, for the reason given on
        // `from_two_char_operator`.
        if let Some(op) = Token::from_two_char_operator(t) {
            return Token::Operator(op);
        }

        // Asking the two converters directly, rather than listing the operator
        // and bracket characters in a guard here and then unwrapping what they
        // return, is what keeps each list in one place: the guard and the
        // converter used to spell out the same set, and a character added to
        // one and not the other turned that `unwrap` into a panic.
        if let Some(c) = t.chars().next() {
            if let Some(token) = Token::from_operator(c) {
                return token;
            }
            if let Some(token) = Token::from_bracket(c) {
                return token;
            }
            match c {
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

        if let Some(op) = Token::from_word_operator(t) {
            return Token::Operator(op);
        }

        if let Some(fun) = Token::get_some(t) {
            return Token::Function(fun);
        }

        Token::Variable(t)
    }

    /// The precedence and associativity of an operator.
    ///
    /// Ten levels, weakest first. The six operators that predate the comparison
    /// and logical set keep their order relative to one another — assignment
    /// below addition below multiplication below power below unary minus below
    /// factorial — so renumbering them cannot change how any existing
    /// expression groups. The new levels all sit below addition, except
    /// [`Operator::Mod`], which joins an existing level rather than creating one.
    ///
    /// This takes an [`Operator`] rather than a [`Token`] because that is its
    /// domain. The wider parameter is what forced the `_ => panic!` arm it used to
    /// carry: a function that accepts brackets and commas has to say something when
    /// it gets one. With the narrow type the match is exhaustive and there is
    /// nothing left to refuse.
    fn operator_priority(o: Operator) -> (u8, Associate) {
        match o {
            Operator::Assign => (0, Associate::RightAssociative),
            Operator::Or | Operator::Xor => (1, Associate::LeftAssociative),
            Operator::And => (2, Associate::LeftAssociative),
            Operator::Not => (3, Associate::RightAssociative),
            Operator::Less
            | Operator::Greater
            | Operator::LessEq
            | Operator::GreaterEq
            | Operator::Equal
            | Operator::NotEqual => (4, Associate::LeftAssociative),
            Operator::Add | Operator::Sub => (5, Associate::LeftAssociative),
            Operator::Mul | Operator::Div | Operator::Mod => (6, Associate::LeftAssociative),
            Operator::Pow => (7, Associate::RightAssociative),
            Operator::Une => (8, Associate::RightAssociative),
            Operator::Fac => (9, Associate::LeftAssociative),
        }
    }

    /// Checks if an operator has priority over another one.
    ///
    /// For example `*` has priority over `+`, `^` over `*`, and unary `-` over
    /// `^`. The operators need their backticks: without them rustdoc read the
    /// `*` as a list marker and rendered the three examples as one bullet.
    ///
    #[must_use]
    pub(crate) fn compare_operator_priority(op1: Operator, op2: Operator) -> bool {
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
    ///
    /// `value` is taken by value, matching every other public numeric
    /// constructor on [`Number`], even though the body only ever borrows it
    /// through [`BigRational::reduced`] (which clones internally regardless);
    /// taking `&BigRational` here would save nothing and would put this
    /// constructor's signature out of step with its callers and its sibling
    /// [`Number::decimal_unchecked`].
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "taken by value to match every other public numeric \
                  constructor on Number, and its sibling decimal_unchecked; \
                  the body borrows through BigRational::reduced, which clones \
                  internally regardless."
    )]
    pub fn decimal(value: BigRational) -> Number {
        let value = value.reduced();
        if value.denom().is_one() {
            Number::NaturalNumber(value.to_integer())
        } else {
            Number::DecimalNumber(value)
        }
    }

    /// Builds a decimal number without reducing first, degrading to
    /// [`Number::NaturalNumber`] when the rational is already a whole number.
    ///
    /// Measurement showed `.reduced()` costing well over 5% on both the
    /// harness's small- and large-rational cases (see the commit that
    /// introduced this function for the numbers), so [`Number::decimal`]'s
    /// gcd is skipped here. Safe only where `value` is already known to be in
    /// lowest terms — every caller must be a path where `value` came straight
    /// out of `BigRational`'s own arithmetic, which reduces its own results.
    #[must_use]
    pub(crate) fn decimal_unchecked(value: BigRational) -> Number {
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
/// A [`Number::DecimalNumber`] is shown as an `f64` when one can carry it, and
/// as `numer/denom` when one cannot. The test for "cannot" is not
/// `to_f64() == None`: [`BigRational::to_f64`] answers `Some(±inf)` on overflow
/// and `Some(0.0)` on underflow rather than `None`, so a fallback guarded on
/// `None` alone never fires and an exactly held value such as `(10^400)/3`
/// prints as `inf`. The predicate below asks instead whether the `f64` is a
/// faithful rendering of the rational. Its second half is what keeps a genuine
/// zero printing as `0`: only a *non-zero* rational that has underflowed to
/// `0.0` needs the ratio.
/// Which end of `f64`'s range a value fell off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Narrowing {
    /// The magnitude exceeds what `f64` can hold.
    TooLarge,
    /// The value is not zero, but rounds to zero.
    TooSmall,
}

/// The one place this crate narrows a [`Number`] to `f64`.
///
/// `to_f64` signals neither of its failures: it answers `Some(inf)` when the
/// value is too large and `Some(0.0)` when it is too small, so both losses
/// arrive looking like successes and every caller that forgets to check
/// inherits a wrong answer. Three callers did. [`Display`] was fixed during
/// Stage 2 and the other two were not, which is how `log(1/(10^400))` came to
/// be refused as not a real number when it is exactly -400.
///
/// A genuine zero narrows to `0.0` successfully. Only a `0.0` that came from a
/// non-zero value is [`Narrowing::TooSmall`], which is what lets
/// `sqrt(1/(10^400))` be refused without also refusing `sqrt(0)`.
///
/// # Errors
/// [`Narrowing::TooLarge`] or [`Narrowing::TooSmall`], naming which end of the
/// range the value fell off.
pub(crate) fn narrow_to_f64(value: &Number) -> Result<f64, Narrowing> {
    let (narrowed, is_zero) = match value {
        Number::NaturalNumber(v) => (v.to_f64(), v.is_zero()),
        Number::DecimalNumber(v) => (v.to_f64(), v.numer().is_zero()),
    };
    // Both types answer `Some` for every input, so this arm is unreachable. It
    // reports `TooLarge` rather than panicking because a `None` could only ever
    // mean the value did not fit.
    let Some(f) = narrowed else {
        return Err(Narrowing::TooLarge);
    };
    if !f.is_finite() {
        return Err(Narrowing::TooLarge);
    }
    if f == 0.0 && !is_zero {
        return Err(Narrowing::TooSmall);
    }
    Ok(f)
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::NaturalNumber(v) => write!(f, "{v}"),
            Number::DecimalNumber(v) => {
                if v.denom().is_one() {
                    write!(f, "{}", v.to_integer())
                } else if let Ok(fl) = narrow_to_f64(self) {
                    write!(f, "{fl}")
                } else {
                    // Too large or too small to be a float, and exact either
                    // way: print what it actually is.
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
/// (op) can be [Add], [Mul], [Sub], [`BitXor`], ...
///
/// We define 2 closures: 1 specialised for Natural Numbers and the other one specialised for Decimals.
///
fn apply_functional_token_operation<NF, DF>(ln: Number, rn: Number, nf: NF, df: DF) -> Number
where
    NF: Fn(BigInt, BigInt) -> BigInt,
    DF: Fn(BigRational, BigRational) -> BigRational,
{
    // `df` is always `+`, `-` or `*` on `BigRational`, and `num-rational`
    // reduces the result of every one of its own arithmetic ops — so the
    // value handed to `Number::decimal_unchecked` here is already reduced.
    // `rn` was cloned here and the clone was never used: the match consumes
    // both operands by value, so the copy — up to about 128 KiB on every `+`,
    // `-` and `*` under the default budget — was pure waste.
    match (ln, rn) {
        (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => Number::NaturalNumber(nf(v1, v2)),
        (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => {
            Number::decimal_unchecked(df(BigRational::from(v1), v2))
        }
        (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => {
            Number::decimal_unchecked(df(v1, BigRational::from(v2)))
        }
        (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => {
            Number::decimal_unchecked(df(v1, v2))
        }
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

impl Number {
    /// Divides, or answers [`None`] when `rhs` is zero.
    ///
    /// There is no `impl Div for Number`: division is partial, and a
    /// `std::ops` impl has nowhere to say so. [`Add`], [`Sub`] and [`Mul`] are
    /// total and stay.
    ///
    /// The zero test compares by value, so it catches a `DecimalNumber(0/1)`
    /// as well as a `NaturalNumber(0)` — [`Number`]'s [`PartialEq`] has
    /// compared by value since Stage 1.
    #[must_use]
    pub fn checked_div(&self, rhs: &Number) -> Option<Number> {
        if rhs == &Number::NaturalNumber(BigInt::zero()) {
            return None;
        }
        // `BigRational::new` reduces on construction, and `/` on `BigRational`
        // reduces its result the same way `+`, `-` and `*` do — every branch
        // here hands `Number::decimal_unchecked` an already-reduced value.
        Some(match (self.clone(), rhs.clone()) {
            (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => {
                Number::decimal_unchecked(BigRational::new(v1, v2))
            }
            (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => {
                Number::decimal_unchecked(BigRational::from(v1) / v2)
            }
            (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => {
                Number::decimal_unchecked(v1 / BigRational::from(v2))
            }
            (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => {
                Number::decimal_unchecked(v1 / v2)
            }
        })
    }
}

/// `PartialOrd` between [Number]s with the required conversions.
///
impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => v1.partial_cmp(v2),
            (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => {
                BigRational::from(v1.clone()).partial_cmp(v2)
            }
            (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => {
                v1.partial_cmp(&BigRational::from(v2.clone()))
            }
            (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => v1.partial_cmp(v2),
        }
    }
}

/// Error returned when a [`Number`] cannot be converted into a fixed-size
/// numeric type because the value falls outside that type's representable range.
///
/// This was [`Eq`] in 0.2.0 and is not any more: [`ConversionError::NotFinite`]
/// carries an `f64`, which is not [`Eq`], and no manual implementation could
/// honestly supply one. [`PartialEq`] is unchanged, so `==` still works — only
/// an `Eq` bound stops compiling. The loss is declared in the README's
/// migration table, which is where the 0.3.0 CHANGELOG is written from.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ConversionError {
    /// The value does not fit in the requested target type.
    #[error("value '{value}' is out of range for target type {target}")]
    OutOfRange {
        /// The offending value, rendered as a decimal string.
        value: String,
        /// The name of the target type that could not hold the value.
        target: &'static str,
    },
    /// The value is NaN or infinite, which has no rational representation.
    #[error("{value} is not a finite number")]
    NotFinite {
        /// The offending value.
        value: f64,
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
        // Both ends are `OutOfRange`: a value that does not fit does not fit,
        // and which end it fell off is not something a conversion's caller can
        // act on differently.
        narrow_to_f64(&n).map_err(|_| ConversionError::OutOfRange {
            value: n.to_string(),
            target: "f64",
        })
    }
}

/// Builds a [`Number`] from an [`f64`], refusing NaN and the infinities.
///
/// The value decides the variant, as everywhere else: an integral `f64`
/// becomes a [`Number::NaturalNumber`].
impl TryFrom<f64> for Number {
    type Error = ConversionError;

    fn try_from(value: f64) -> Result<Number, ConversionError> {
        BigRational::from_float(value)
            .map(Number::decimal)
            .ok_or(ConversionError::NotFinite { value })
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
            Operator::Assign => write!(f, "="),
            Operator::Less => write!(f, "<"),
            Operator::Greater => write!(f, ">"),
            Operator::LessEq => write!(f, "<="),
            Operator::GreaterEq => write!(f, ">="),
            Operator::Equal => write!(f, "=="),
            Operator::NotEqual => write!(f, "<>"),
            Operator::And => write!(f, "and"),
            Operator::Or => write!(f, "or"),
            Operator::Xor => write!(f, "xor"),
            Operator::Not => write!(f, "not"),
            Operator::Mod => write!(f, "mod"),
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
        let v = ["1", "+", "2.1"];
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

    /// The whole safety argument for renumbering the ladder. Every operator that
    /// existed before the comparison and logical set keeps its position relative
    /// to the others, so no expression that evaluates today can change meaning.
    /// If this ever goes red, some existing expression has quietly been
    /// re-parsed.
    ///
    /// It asserts the order rather than the numbers on purpose: the numbers are
    /// an implementation detail that a later operator may shift again, and a
    /// test restating them would have to be edited every time — which is how a
    /// test stops being able to fail.
    #[test]
    fn test_the_existing_operators_keep_their_relative_order() {
        let ascending = [
            Operator::Assign,
            Operator::Add,
            Operator::Mul,
            Operator::Pow,
            Operator::Une,
            Operator::Fac,
        ];
        let levels: Vec<u8> = ascending
            .iter()
            .map(|o| Token::operator_priority(*o).0)
            .collect();
        assert!(
            levels.windows(2).all(|pair| pair[0] < pair[1]),
            "levels were {levels:?}, which is not strictly ascending"
        );
    }

    /// Associativity is the other half of how an expression groups, and the
    /// order test above says nothing about it. These four are the ones an
    /// existing expression depends on: `-2^-2` needs `^` right-associative,
    /// `x=y=5` needs `=` right-associative, and `2-3-4` needs `-` left.
    #[test]
    fn test_the_existing_operators_keep_their_associativity() {
        for (op, associativity) in [
            (Operator::Assign, Associate::RightAssociative),
            (Operator::Add, Associate::LeftAssociative),
            (Operator::Sub, Associate::LeftAssociative),
            (Operator::Mul, Associate::LeftAssociative),
            (Operator::Div, Associate::LeftAssociative),
            (Operator::Pow, Associate::RightAssociative),
            (Operator::Une, Associate::RightAssociative),
            (Operator::Fac, Associate::LeftAssociative),
        ] {
            assert_eq!(
                Token::operator_priority(op).1,
                associativity,
                "for operator {op}"
            );
        }
    }

    /// `mod` shares a level with `*` and `/` rather than getting one of its own,
    /// so `7 mod 3 * 2` groups left to right.
    #[test]
    fn test_mod_sits_with_multiplication() {
        assert_eq!(
            Token::operator_priority(Operator::Mod),
            Token::operator_priority(Operator::Mul)
        );
    }

    /// The three prefix and postfix operators take no left operand. The
    /// evaluation loop asks this question twice, and before this function
    /// existed it spelled the answer out both times.
    #[test]
    fn test_only_the_three_unary_operators_report_as_unary() {
        for unary in [Operator::Une, Operator::Fac, Operator::Not] {
            assert!(unary.is_unary(), "{unary} should be unary");
        }
        for binary in [
            Operator::Add,
            Operator::Sub,
            Operator::Mul,
            Operator::Div,
            Operator::Pow,
            Operator::Assign,
            Operator::Less,
            Operator::Greater,
            Operator::LessEq,
            Operator::GreaterEq,
            Operator::Equal,
            Operator::NotEqual,
            Operator::And,
            Operator::Or,
            Operator::Xor,
            Operator::Mod,
        ] {
            assert!(!binary.is_unary(), "{binary} should be binary");
        }
    }

    /// Every operator renders as the text a user would type, which is what the
    /// `found '{}'` half of a parse error shows. `Une` is the exception and
    /// stays one: it is a rewrite of `-` that no user ever types, and `#` is
    /// how the debug output has always spelled it.
    #[test]
    fn test_the_new_operators_render_as_they_are_written() {
        let pairs = [
            (Operator::Less, "<"),
            (Operator::Greater, ">"),
            (Operator::LessEq, "<="),
            (Operator::GreaterEq, ">="),
            (Operator::Equal, "=="),
            (Operator::NotEqual, "<>"),
            (Operator::And, "and"),
            (Operator::Or, "or"),
            (Operator::Xor, "xor"),
            (Operator::Not, "not"),
            (Operator::Mod, "mod"),
            (Operator::Assign, "="),
        ];
        for (op, text) in pairs {
            assert_eq!(op.to_string(), text);
        }
    }

    /// The reason this exists: `a / b` panicked here, inside a public `std::ops`
    /// impl, on an input any caller can supply.
    #[test]
    fn test_dividing_by_zero_answers_none_instead_of_panicking() {
        let one = Number::NaturalNumber(BigInt::from(1));
        let zero = Number::NaturalNumber(BigInt::from(0));
        assert_eq!(one.checked_div(&zero), None);
        assert_eq!(
            Number::decimal(BigRational::new(BigInt::from(1), BigInt::from(2))).checked_div(&zero),
            None
        );
    }

    #[test]
    fn test_checked_div_still_divides() {
        let six = Number::NaturalNumber(BigInt::from(6));
        let three = Number::NaturalNumber(BigInt::from(3));
        assert_eq!(
            six.checked_div(&three),
            Some(Number::NaturalNumber(BigInt::from(2)))
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
        // Also unreachable from the pipeline — the regex has no '#' — and it
        // moved when the two duplicated character lists became one. It used to
        // fall through to a variable named "#"; it now spells the unary minus,
        // which is how `Display` has always written it. Pinned so that the
        // change is a decision rather than a side effect.
        assert_eq!(Token::tokenize("#"), Token::Operator(Operator::Une));
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

    /// `(10^400)/3` and `1/(10^400)` are held exactly and compute correctly —
    /// only their printed form was wrong. `BigRational::to_f64` answers
    /// `Some(inf)` and `Some(0.0)` rather than `None` for them, so the
    /// `numer/denom` fallback, guarded on `None`, never ran: the first printed
    /// `inf` and the second `0`, with nothing signalling the loss.
    #[test]
    fn test_display_falls_back_to_a_ratio_when_no_f64_can_carry_the_value() {
        let huge = BigInt::from(10).pow(400_u32);

        let overflows = Number::DecimalNumber(BigRational::new(huge.clone(), BigInt::from(3)));
        assert_eq!(overflows.to_string(), format!("{huge}/3"));

        let underflows = Number::DecimalNumber(BigRational::new(BigInt::from(1), huge.clone()));
        assert_eq!(underflows.to_string(), format!("1/{huge}"));

        // Negative overflow takes the same route: `to_f64` answers Some(-inf).
        let negative = Number::DecimalNumber(BigRational::new(-huge.clone(), BigInt::from(3)));
        assert_eq!(negative.to_string(), format!("-{huge}/3"));
    }

    /// The other direction of the same predicate. A rational that underflows to
    /// `0.0` must take the fallback, but one that *is* zero must still print
    /// `0` — the two are indistinguishable by their `f64` alone, which is why
    /// the numerator is consulted as well.
    #[test]
    fn test_display_still_prints_a_genuine_zero_as_zero() {
        assert_eq!(Number::NaturalNumber(BigInt::zero()).to_string(), "0");
        // new_raw skips reduction, so this reaches the f64 branch with a
        // denominator that is not 1 — the only way a zero gets that far.
        let unreduced_zero =
            Number::DecimalNumber(BigRational::new_raw(BigInt::zero(), BigInt::from(3)));
        assert_eq!(unreduced_zero.to_string(), "0");
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
