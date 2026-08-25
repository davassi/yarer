//! The errors yarer reports, and where in the expression they happened.
//!
//! Two kinds, kept apart in the type system rather than in a message prefix:
//! [`ParseError`] is produced while an expression is being compiled, and
//! [`EvalError`] while a compiled expression is being evaluated. A caller that
//! wants one type across both calls converts into [`Error`].
//!
//! Both enums are `#[non_exhaustive]`: yarer will grow error conditions, and
//! that must not be a breaking change. Match with a `_` arm.

use crate::span::Span;
use crate::token::MathFunction;

/// A failure while compiling an expression.
///
/// Every variant carries the position of the token it is about, with one
/// exception: [`ParseError::Malformed`] names no token — it is the structural
/// fallback for a condition the validation pass is supposed to have made
/// unreachable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// Text that is not part of any token, e.g. the `@` in `1@2`.
    #[error("unexpected character '{text}'")]
    UnexpectedCharacter { text: String, span: Span },
    /// The expression contains no tokens at all.
    #[error("the expression is empty")]
    EmptyExpression,
    /// A bracket pair enclosing nothing, e.g. `()`.
    #[error("empty brackets are not a value")]
    EmptyGroup { span: Span },
    /// A bracket that is never closed, or one that closes nothing.
    #[error("unbalanced brackets")]
    UnbalancedBracket { span: Span },
    /// A `,` with no bracket open at all.
    #[error("',' is only valid between the arguments of a function call")]
    CommaOutsideCall { span: Span },
    /// A `,` inside brackets that group a value rather than open a call.
    #[error("',' separates function arguments, but these brackets group a value")]
    CommaInPlainBracket { span: Span },
    /// An argument slot with nothing in it, e.g. `max(1,)`.
    #[error("a function argument cannot be empty")]
    EmptyArgument { span: Span },
    /// A call with the wrong number of arguments.
    #[error("function '{}' expects {expected} argument(s), {given} given", .function.to_string().to_lowercase())]
    WrongArity {
        function: MathFunction,
        expected: u8,
        given: usize,
        span: Span,
    },
    /// A function name not followed by an opening bracket.
    #[error("function '{}' must be followed by '(' or '['", .function.to_string().to_lowercase())]
    FunctionRequiresParentheses { function: MathFunction, span: Span },
    /// A `;` reached with a bracket still open.
    #[error("a bracket must be closed before ';'")]
    BracketUnclosedAtSemicolon { span: Span },
    /// A value was required here and something else appeared.
    #[error("expected a value, found '{found}'")]
    ExpectedValue { found: String, span: Span },
    /// An operator was required here and something else appeared.
    #[error("expected an operator, found '{found}'")]
    ExpectedOperator { found: String, span: Span },
    /// The structural fallback. Reaching it means an invariant this crate
    /// maintains has been broken, not that the user wrote something odd.
    #[error("the expression is malformed")]
    Malformed,
}

/// A failure while evaluating a compiled expression.
///
/// Spans are optional here because an evaluation error need not come from
/// source text at all: `Session::setf("x", f64::NAN)` fails with no expression
/// in existence.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum EvalError {
    #[error("division by zero")]
    DivisionByZero { span: Option<Span> },
    /// A value that was computed and measured over the budget.
    #[error("the result occupies about {bits} bits, over the size limit of {limit} bits")]
    ValueTooLarge {
        bits: u128,
        limit: u64,
        span: Option<Span>,
    },
    /// A computation refused before running, on an estimate of its size.
    #[error(
        "the result would need about {predicted_bits} bits, over the size limit of {limit} bits"
    )]
    ComputationTooLarge {
        predicted_bits: u128,
        limit: u64,
        span: Option<Span>,
    },
    #[error("factorial is only defined for non-negative integers")]
    FactorialNotNatural { span: Option<Span> },
    #[error("the factorial operand is too large")]
    FactorialOperandTooLarge { span: Option<Span> },
    #[error("the exponent is too large to evaluate under any size limit")]
    ExponentTooLarge { span: Option<Span> },
    #[error("power operands are too large for non-integer evaluation")]
    PowerOperandsTooLarge { span: Option<Span> },
    #[error("invalid power operation")]
    InvalidPower { span: Option<Span> },
    #[error("operand is too large for floating-point evaluation")]
    OperandTooLargeForFloat { span: Option<Span> },
    #[error("function result is not a real number")]
    NotARealNumber { span: Option<Span> },
    #[error("'{name}' is a built-in constant and is read-only")]
    ReadOnlyConstant { name: String, span: Option<Span> },
    #[error("no variable has been defined for assignment")]
    AssignmentTargetMissing { span: Option<Span> },
    #[error("{value} is not a finite number")]
    NotFinite { value: f64 },
    /// The evaluation stack did not end with exactly one value. Same status as
    /// [`ParseError::Malformed`]: a broken invariant, not a user mistake.
    #[error("the expression is malformed")]
    Malformed { span: Option<Span> },
}

/// Either kind of failure, for callers that want one type across `compile` and
/// `eval`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Eval(#[from] EvalError),
}

impl ParseError {
    /// Where in the expression this error is, if it is about a token.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        match self {
            ParseError::UnexpectedCharacter { span, .. }
            | ParseError::EmptyGroup { span }
            | ParseError::UnbalancedBracket { span }
            | ParseError::CommaOutsideCall { span }
            | ParseError::CommaInPlainBracket { span }
            | ParseError::EmptyArgument { span }
            | ParseError::WrongArity { span, .. }
            | ParseError::FunctionRequiresParentheses { span, .. }
            | ParseError::BracketUnclosedAtSemicolon { span }
            | ParseError::ExpectedValue { span, .. }
            | ParseError::ExpectedOperator { span, .. } => Some(*span),
            ParseError::EmptyExpression | ParseError::Malformed => None,
        }
    }
}

impl EvalError {
    /// Where in the expression this error is, if it came from one.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        match self {
            EvalError::DivisionByZero { span }
            | EvalError::ValueTooLarge { span, .. }
            | EvalError::ComputationTooLarge { span, .. }
            | EvalError::FactorialNotNatural { span }
            | EvalError::FactorialOperandTooLarge { span }
            | EvalError::ExponentTooLarge { span }
            | EvalError::PowerOperandsTooLarge { span }
            | EvalError::InvalidPower { span }
            | EvalError::OperandTooLargeForFloat { span }
            | EvalError::NotARealNumber { span }
            | EvalError::ReadOnlyConstant { span, .. }
            | EvalError::AssignmentTargetMissing { span }
            | EvalError::Malformed { span } => *span,
            EvalError::NotFinite { .. } => None,
        }
    }
}

impl Error {
    /// Where in the expression this error is, if it is about a position.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        match self {
            Error::Parse(e) => e.span(),
            Error::Eval(e) => e.span(),
        }
    }

    /// The message, and — when the error has a position and `source` is the
    /// expression it came from — the expression with a caret under it.
    ///
    /// The category prefix is added here, once, rather than being baked into a
    /// dozen message constants where it has already drifted.
    #[must_use]
    pub fn render(&self, source: &str) -> String {
        let (prefix, message) = match self {
            Error::Parse(e) => ("Parse error", e.to_string()),
            Error::Eval(e) => ("Eval error", e.to_string()),
        };
        let head = format!("{prefix}: {message}");

        let Some(span) = self.span() else {
            return head;
        };
        // The caller supplies `source`, and may supply the wrong one. Both
        // slices are taken through `get`, so a span that does not fit — or that
        // lands inside a multi-byte character — degrades to the message alone
        // instead of panicking.
        let Some(before) = source.get(..span.start) else {
            return head;
        };
        let Some(token) = source.get(span.start..span.end) else {
            return head;
        };
        let column = before.chars().count();
        // A zero-width span still gets one caret: an error at end of input has
        // nothing to underline but still has somewhere to point.
        let width = token.chars().count().max(1);
        format!(
            "{head}\n  {source}\n  {}{}",
            " ".repeat(column),
            "^".repeat(width)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_puts_the_caret_under_the_offending_token() {
        let err = Error::Parse(ParseError::ExpectedValue {
            found: "*".to_string(),
            span: Span::new(6, 7),
        });
        assert_eq!(
            err.render("max(1,*2)"),
            "Parse error: expected a value, found '*'\n  max(1,*2)\n        ^"
        );
    }

    /// '×' is two bytes wide and one column wide. A caret positioned by byte
    /// offset lands one column past the token it is meant to point at, and no
    /// other test in the suite can notice, because the message stays correct.
    #[test]
    fn test_render_counts_columns_in_chars_not_bytes() {
        let err = Error::Eval(EvalError::DivisionByZero {
            span: Some(Span::new(3, 4)),
        });
        assert_eq!(
            err.render("2×3"),
            "Eval error: division by zero\n  2×3\n    ^"
        );
    }

    #[test]
    fn test_render_without_a_span_prints_only_the_message() {
        let err = Error::Eval(EvalError::NotFinite { value: f64::NAN });
        assert_eq!(
            err.render("whatever"),
            "Eval error: NaN is not a finite number"
        );
    }

    /// `render` takes the source from the caller, who may hand it the wrong
    /// string. Slicing it must not panic.
    #[test]
    fn test_render_survives_a_span_that_does_not_fit_the_source() {
        let err = Error::Parse(ParseError::UnbalancedBracket {
            span: Span::new(50, 51),
        });
        assert_eq!(err.render("1+1"), "Parse error: unbalanced brackets");
    }

    #[test]
    fn test_the_two_size_errors_are_distinguishable_without_reading_the_text() {
        let measured = EvalError::ValueTooLarge {
            bits: 12,
            limit: 8,
            span: None,
        };
        let predicted = EvalError::ComputationTooLarge {
            predicted_bits: 12,
            limit: 8,
            span: None,
        };
        assert!(matches!(measured, EvalError::ValueTooLarge { .. }));
        assert!(matches!(predicted, EvalError::ComputationTooLarge { .. }));
        assert_ne!(measured.to_string(), predicted.to_string());
    }

    #[test]
    fn test_a_parse_error_converts_into_the_union_type() {
        let err: Error = ParseError::EmptyExpression.into();
        assert!(matches!(err, Error::Parse(ParseError::EmptyExpression)));
    }
}
