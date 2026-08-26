//! The pass between tokenising and the shunting yard.
//!
//! It walks the token stream once, holding one bit of state — whether a value
//! or an operator is expected next — and one stack of bracket frames. Both
//! already existed: the bit is `mod_unary_operators`' `expect_operand_next`,
//! which used it to tell a binary `-` from a unary one but had no way to
//! refuse, and the frames are the shunting yard's, which counted arguments
//! while it was busy reordering. Bringing them together costs no extra
//! traversal and gives every rejection a position.

use crate::error::ParseError;
use crate::span::{Span, Spanned};
use crate::token::{Bracket, MathFunction, Operator, Token};

/// One entry per open bracket.
struct Frame {
    /// The call this bracket opens, with the position of the function name —
    /// which is where an arity error points, since that is what the user has
    /// to change.
    function: Option<(MathFunction, Span)>,
    /// Where the bracket itself is, for the unbalanced-bracket message.
    open: Span,
    /// Separators seen so far inside this bracket.
    commas: usize,
    /// Whether the current argument slot — since the bracket opened, or since
    /// the last `,` — has seen anything.
    has_content: bool,
}

#[derive(PartialEq, Eq)]
enum Expect {
    Value,
    Operator,
}

/// Whether anything content-bearing has been seen — within the current
/// `;`-separated segment, and anywhere in the input at all.
///
/// `segment` resets at every `;`: a leading or trailing separator leaves an
/// empty segment, which has always been legal, while an incomplete one —
/// `"1+;"` — has not. `any` never resets, and is what an expression made of
/// nothing but separators trips over. Asking only the per-segment question let
/// `";"` validate, reach the evaluator, produce no value and land in the
/// positionless `EvalError::Malformed` — the generic message the rest of this
/// pass exists to eliminate.
#[derive(Default)]
struct Content {
    segment: bool,
    any: bool,
}

impl Content {
    fn mark(&mut self) {
        self.segment = true;
        self.any = true;
    }
}

/// Checks that the token sequence is a well-formed expression, and rewrites the
/// unary operators while it is there.
///
/// # Errors
/// Every [`ParseError`] variant except `UnexpectedCharacter` and `Malformed`,
/// which belong to the tokeniser and the shunting yard. `EmptyExpression` is
/// shared with the tokeniser: it raises it for input holding no tokens, this
/// pass for input holding nothing but separators.
#[expect(
    clippy::too_many_lines,
    reason = "one match over Token variants, and the length is what buys five \
              distinct positioned diagnoses where there used to be five \
              identical 'malformed expression' failures. The Bracket(Close) \
              arm is the separable part if this ever has to shrink."
)]
pub(crate) fn validate<'a>(
    tokens: &[Spanned<Token<'a>>],
    source: &str,
) -> Result<Vec<Spanned<Token<'a>>>, ParseError> {
    let mut out: Vec<Spanned<Token<'a>>> = Vec::with_capacity(tokens.len());
    let mut expect = Expect::Value;
    let mut frames: Vec<Frame> = Vec::new();
    let mut pending_function: Option<(MathFunction, Span)> = None;
    let mut content = Content::default();

    for t in tokens {
        // A function name must be followed by an opening bracket. Checked here
        // against whatever came next, and once more after the loop for a name
        // with nothing after it at all.
        if let Some((fun, span)) = pending_function {
            if !matches!(t.node, Token::Bracket(Bracket::Open)) {
                return Err(ParseError::FunctionRequiresParentheses {
                    function: fun,
                    span,
                });
            }
        }

        match &t.node {
            Token::Operand(_) | Token::Variable(_) => {
                require_value_position(&expect, t, source)?;
                expect = Expect::Operator;
                mark_content(&mut frames, &mut content);
                out.push(t.clone());
            }

            Token::Function(fun) => {
                require_value_position(&expect, t, source)?;
                pending_function = Some((*fun, t.span));
                mark_content(&mut frames, &mut content);
                out.push(t.clone());
            }

            Token::Bracket(Bracket::Open) => {
                require_value_position(&expect, t, source)?;
                frames.push(Frame {
                    function: pending_function.take(),
                    open: t.span,
                    commas: 0,
                    has_content: false,
                });
                out.push(t.clone());
            }

            Token::Bracket(Bracket::Close) => {
                let frame = frames
                    .pop()
                    .ok_or(ParseError::UnbalancedBracket { span: t.span })?;

                if frame.has_content {
                    // '(1+)': the group holds something but ends mid-expression.
                    if expect == Expect::Value {
                        return Err(ParseError::ExpectedValue {
                            found: text_at(source, t.span),
                            span: t.span,
                        });
                    }
                } else if frame.commas > 0 {
                    // 'max(1,)': the final slot is empty.
                    return Err(ParseError::EmptyArgument { span: t.span });
                } else if let Some((fun, fspan)) = frame.function {
                    // 'max()': keeps the arity wording it has today.
                    return Err(ParseError::WrongArity {
                        function: fun,
                        expected: fun.arity(),
                        given: 0,
                        span: fspan,
                    });
                } else {
                    // '()': brackets around nothing.
                    return Err(ParseError::EmptyGroup { span: t.span });
                }

                if let Some((fun, fspan)) = frame.function {
                    let given = frame.commas + 1;
                    let expected = fun.arity();
                    if given != usize::from(expected) {
                        return Err(ParseError::WrongArity {
                            function: fun,
                            expected,
                            given,
                            span: fspan,
                        });
                    }
                }

                // Any bracket reaching this point enclosed something, so the
                // enclosing slot has content. Stage 1 had to defer this
                // decision — an empty group contributed nothing and had to be
                // told apart from one that did — and that bookkeeping is gone,
                // because an empty group is now an error in its own right.
                expect = Expect::Operator;
                mark_content(&mut frames, &mut content);
                out.push(t.clone());
            }

            Token::Operator(op) => match (&expect, op) {
                // A '+' in value position has never meant anything.
                (Expect::Value, Operator::Add) => {
                    mark_content(&mut frames, &mut content);
                }
                // A '-' in value position is the unary operator, and inherits
                // the position of the '-' it replaces.
                (Expect::Value, Operator::Sub) => {
                    mark_content(&mut frames, &mut content);
                    out.push(Spanned::new(Token::Operator(Operator::Une), t.span));
                }
                // `not` is prefix: it wants a value on its right and takes
                // nothing on its left, so it belongs exactly where a value
                // belongs and leaves the state where it found it. That is what
                // makes `not not 1` and `not (1 < 2)` legal.
                (Expect::Value, Operator::Not) => {
                    mark_content(&mut frames, &mut content);
                    out.push(t.clone());
                }
                (Expect::Value, _) => {
                    return Err(ParseError::ExpectedValue {
                        found: text_at(source, t.span),
                        span: t.span,
                    })
                }
                // '!' is postfix: it consumes the value on its left and leaves
                // one in its place, so the state does not move.
                (Expect::Operator, Operator::Fac) => out.push(t.clone()),
                // `not` is not a binary operator, and the catch-all below
                // would accept it as one. `1 not 2` would then validate,
                // reach the evaluator, and fail there with a complaint about
                // a stack — which is the defect `max(1,*2)` had before this
                // pass existed, reintroduced by a new operator.
                (Expect::Operator, Operator::Not) => {
                    return Err(ParseError::ExpectedOperator {
                        found: text_at(source, t.span),
                        span: t.span,
                    })
                }
                (Expect::Operator, _) => {
                    expect = Expect::Value;
                    out.push(t.clone());
                }
            },

            Token::Comma => {
                let Some(frame) = frames.last_mut() else {
                    return Err(ParseError::CommaOutsideCall { span: t.span });
                };
                if frame.function.is_none() {
                    return Err(ParseError::CommaInPlainBracket { span: t.span });
                }
                if !frame.has_content {
                    return Err(ParseError::EmptyArgument { span: t.span });
                }
                if expect == Expect::Value {
                    return Err(ParseError::ExpectedValue {
                        found: text_at(source, t.span),
                        span: t.span,
                    });
                }
                frame.commas += 1;
                frame.has_content = false;
                expect = Expect::Value;
                out.push(t.clone());
            }

            Token::SemiColon => {
                if !frames.is_empty() {
                    return Err(ParseError::BracketUnclosedAtSemicolon { span: t.span });
                }
                if expect == Expect::Value && content.segment {
                    return Err(ParseError::ExpectedValue {
                        found: text_at(source, t.span),
                        span: t.span,
                    });
                }
                expect = Expect::Value;
                content.segment = false;
                out.push(t.clone());
            }
        }
    }

    if let Some((fun, span)) = pending_function {
        return Err(ParseError::FunctionRequiresParentheses {
            function: fun,
            span,
        });
    }
    if let Some(frame) = frames.last() {
        return Err(ParseError::UnbalancedBracket { span: frame.open });
    }
    // Separators and nothing else: every segment was empty, so no segment was
    // ever incomplete and the check below cannot speak for this. `";"` is as
    // empty an expression as `""`, and gets the same answer.
    if !content.any {
        return Err(ParseError::EmptyExpression);
    }
    if expect == Expect::Value && content.segment {
        // Nothing to underline, but somewhere to point: a zero-width span at
        // the end of the source, which `render` widens to a single caret.
        let end = source.len();
        return Err(ParseError::ExpectedValue {
            found: "end of expression".to_string(),
            span: Span::new(end, end),
        });
    }

    Ok(out)
}

/// Refuses a value where an operator was required.
fn require_value_position(
    expect: &Expect,
    t: &Spanned<Token<'_>>,
    source: &str,
) -> Result<(), ParseError> {
    if *expect == Expect::Operator {
        return Err(ParseError::ExpectedOperator {
            found: text_at(source, t.span),
            span: t.span,
        });
    }
    Ok(())
}

/// Records that the innermost argument slot, and the input, are no longer
/// empty.
fn mark_content(frames: &mut [Frame], content: &mut Content) {
    if let Some(frame) = frames.last_mut() {
        frame.has_content = true;
    }
    content.mark();
}

/// The text the user actually typed for this token. The fallback cannot be
/// reached — every span here came from this same source — and exists so that
/// building an error message can never panic.
fn text_at(source: &str, span: Span) -> String {
    source.get(span.start..span.end).unwrap_or("?").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::token::{MathFunction, Number, Operator, Token};
    use num_bigint::BigInt;

    fn check(source: &str) -> Result<Vec<Spanned<Token<'_>>>, ParseError> {
        let tokens = Parser::parse(source).expect("tokenises");
        validate(&tokens, source)
    }

    /// One case per error-producing rule. Each input is one that today lands in
    /// a *different* guard — four of them in the generic malformed message and
    /// one, `max(1,*2)`, in an arity complaint about `max`, whose arity is
    /// correct. Asserting the exact variant is what makes these tests fail for
    /// the right reason.
    #[test]
    fn test_every_illegal_sequence_gets_its_own_diagnosis() {
        let cases: Vec<(&str, ParseError)> = vec![
            (
                "()",
                ParseError::EmptyGroup {
                    span: Span::new(1, 2),
                },
            ),
            (
                "2 3",
                ParseError::ExpectedOperator {
                    found: "3".to_string(),
                    span: Span::new(2, 3),
                },
            ),
            (
                "2(3+4)",
                ParseError::ExpectedOperator {
                    found: "(".to_string(),
                    span: Span::new(1, 2),
                },
            ),
            (
                "1+",
                ParseError::ExpectedValue {
                    found: "end of expression".to_string(),
                    span: Span::new(2, 2),
                },
            ),
            (
                "max(1,*2)",
                ParseError::ExpectedValue {
                    found: "*".to_string(),
                    span: Span::new(6, 7),
                },
            ),
            (
                "!5",
                ParseError::ExpectedValue {
                    found: "!".to_string(),
                    span: Span::new(0, 1),
                },
            ),
            (
                "(1,2)",
                ParseError::CommaInPlainBracket {
                    span: Span::new(2, 3),
                },
            ),
            (
                "1,2",
                ParseError::CommaOutsideCall {
                    span: Span::new(1, 2),
                },
            ),
            (
                "1+;2",
                ParseError::ExpectedValue {
                    found: ";".to_string(),
                    span: Span::new(2, 3),
                },
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(check(source), Err(expected), "for input {source}");
        }
    }

    /// The bracket and arity rules that used to live in the shunting yard.
    /// Their messages are unchanged; only their home and their positions are new.
    #[test]
    fn test_the_bracket_rules_moved_without_changing_what_they_say() {
        let cases: Vec<(&str, ParseError)> = vec![
            (
                "max()",
                ParseError::WrongArity {
                    function: MathFunction::Max,
                    expected: 2,
                    given: 0,
                    span: Span::new(0, 3),
                },
            ),
            (
                "max(1)",
                ParseError::WrongArity {
                    function: MathFunction::Max,
                    expected: 2,
                    given: 1,
                    span: Span::new(0, 3),
                },
            ),
            (
                "max(1,2,3)",
                ParseError::WrongArity {
                    function: MathFunction::Max,
                    expected: 2,
                    given: 3,
                    span: Span::new(0, 3),
                },
            ),
            (
                "sin(1,2)",
                ParseError::WrongArity {
                    function: MathFunction::Sin,
                    expected: 1,
                    given: 2,
                    span: Span::new(0, 3),
                },
            ),
            (
                "max(1,)",
                ParseError::EmptyArgument {
                    span: Span::new(6, 7),
                },
            ),
            (
                "max(,1)",
                ParseError::EmptyArgument {
                    span: Span::new(4, 5),
                },
            ),
            (
                "sin 5",
                ParseError::FunctionRequiresParentheses {
                    function: MathFunction::Sin,
                    span: Span::new(0, 3),
                },
            ),
            (
                "sin",
                ParseError::FunctionRequiresParentheses {
                    function: MathFunction::Sin,
                    span: Span::new(0, 3),
                },
            ),
            (
                "(1+1",
                ParseError::UnbalancedBracket {
                    span: Span::new(0, 1),
                },
            ),
            (
                "1+2)",
                ParseError::UnbalancedBracket {
                    span: Span::new(3, 4),
                },
            ),
            (
                "(1;2)",
                ParseError::BracketUnclosedAtSemicolon {
                    span: Span::new(2, 3),
                },
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(check(source), Err(expected), "for input {source}");
        }
    }

    /// An expression of nothing but separators used to validate.
    /// `Content::segment` is per-segment and never became true, so the
    /// end-of-input check could not speak for it; the token stream reached the
    /// evaluator, produced no value, and landed in the positionless
    /// `EvalError::Malformed` — a sixth malformed input in the bucket the
    /// other five were taken out of. It is as empty as `""`, and now says so.
    #[test]
    fn test_an_expression_of_only_separators_is_empty_not_malformed() {
        for source in [";", ";;", " ; ", " ;; "] {
            assert_eq!(
                check(source),
                Err(ParseError::EmptyExpression),
                "for input {source}"
            );
        }
    }

    /// The load-bearing behaviour of the crate. Every one of these evaluates
    /// today and must still reach the shunting yard.
    #[test]
    fn test_what_worked_before_still_validates() {
        for source in [
            "1+2*3/(4-5)",
            "-2^-2",
            "--5",
            "5--3",
            "x=y=5",
            "x=2; y=3; x*y",
            "max(1,2)",
            "sin[5]",
            "sin(-1)",
            "5!",
            "2.0!",
            ";1+1",
            "1+2;",
            "1/cos(x^2)",
            "pi + e",
            "9801/(2206*sqrt(2))",
        ] {
            assert!(check(source).is_ok(), "{source} was refused");
        }
    }

    /// Master's `test_multiple_unary_ops2`, restored against this pass. It
    /// pinned the multi-token unary rewrite *by value* — `-(+(-5*-5))` becomes
    /// `#((#5*#5))` — and was deleted along with `mod_unary_operators` without
    /// a replacement. Since then only the single-token `-5` was pinned by
    /// value, while `--5` and `5--3` appeared solely in a test asserting
    /// `is_ok()`, which cannot tell a correct rewrite from a wrong one.
    ///
    /// It runs through the whole front end rather than over hand-built tokens,
    /// so it pins what a user typing those characters actually gets.
    #[test]
    fn test_the_unary_rewrite_holds_across_nesting_and_repetition() {
        let une = || Token::Operator(Operator::Une);
        let open = || Token::Bracket(Bracket::Open);
        let close = || Token::Bracket(Bracket::Close);
        let num = |n: u8| Token::Operand(Number::NaturalNumber(BigInt::from(n)));

        let cases: Vec<(&str, Vec<Token>)> = vec![
            // The leading '+' is dropped, and each of the three '-' in value
            // position becomes the unary operator; the '*' keeps its own.
            (
                "-(+(-5*-5))",
                vec![
                    une(),
                    open(),
                    open(),
                    une(),
                    num(5),
                    Token::Operator(Operator::Mul),
                    une(),
                    num(5),
                    close(),
                    close(),
                ],
            ),
            ("--5", vec![une(), une(), num(5)]),
            // The first '-' follows a value, so it stays binary; only the
            // second is rewritten.
            (
                "5--3",
                vec![num(5), Token::Operator(Operator::Sub), une(), num(3)],
            ),
        ];

        for (source, expected) in cases {
            let actual: Vec<Token> = check(source)
                .unwrap_or_else(|e| panic!("{source} was refused: {e}"))
                .into_iter()
                .map(|t| t.node)
                .collect();
            assert_eq!(actual, expected, "for input {source}");
        }
    }

    /// The unary rewrite is unchanged apart from its address, and the token it
    /// produces inherits the position of the '-' it replaces.
    #[test]
    fn test_the_unary_minus_is_rewritten_in_place() {
        let tokens = check("-5").unwrap();
        assert_eq!(tokens[0].node, Token::Operator(Operator::Une));
        assert_eq!(tokens[0].span, Span::new(0, 1));
    }

    /// A leading '+' has never meant anything and is still dropped.
    #[test]
    fn test_a_unary_plus_disappears() {
        let tokens = check("+5").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0].node,
            Token::Operand(Number::NaturalNumber(BigInt::from(5u8)))
        );
    }
}
