use crate::error::ParseError;
use crate::span::{Span, Spanned};
use crate::token::{self, Operator, Token};

use log::debug;
use once_cell::sync::Lazy;
use regex::Regex;

/// The Parser has 2 primary functions:
/// to parse the math expression with a Regex and to tokenise the math &[str] expression
///
#[derive(Debug)]
pub struct Parser;

static EXPRESSION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d+\.?\d*|\.\d+|[-+*/^(),=\[\]×÷!;]|[a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Should compile regex")
});

impl Parser {
    /// Splits `expr` into tokens, each carrying the byte range it came from.
    pub(crate) fn parse(expr: &str) -> Result<Vec<Spanned<Token<'_>>>, ParseError> {
        let mut vex: Vec<Spanned<Token<'_>>> = Vec::new();
        let mut cursor = 0;

        for m in EXPRESSION_REGEX.find_iter(expr) {
            Self::validate_gap(expr, cursor, m.start())?;
            vex.push(Spanned::new(
                Token::tokenize(m.as_str()),
                Span::new(m.start(), m.end()),
            ));
            cursor = m.end();
        }

        Self::validate_gap(expr, cursor, expr.len())?;

        if vex.is_empty() {
            return Err(ParseError::EmptyExpression);
        }

        Ok(Self::mod_unary_operators(&vex))
    }

    /// Finds out all the unary operators that are present in the expression
    ///
    fn mod_unary_operators<'a>(v: &[Spanned<Token<'a>>]) -> Vec<Spanned<Token<'a>>> {
        let mut mod_vec: Vec<Spanned<Token<'a>>> = Vec::new();
        let mut expect_operand_next = true;

        for token in v {
            debug!("{}", token.node);

            match &token.node {
                Token::Operand(_) | Token::Variable(_) | Token::Operator(Operator::Fac) => {
                    expect_operand_next = false;
                }
                Token::Operator(o) => {
                    if expect_operand_next {
                        debug!("-> Unary operator detected");
                        match o {
                            token::Operator::Add => {
                                // an unary + can be simply ignored.
                                continue;
                            }
                            token::Operator::Sub => {
                                // an unary - is a special right-associative op with the highest precedence
                                mod_vec.push(Spanned::new(
                                    token::Token::Operator(token::Operator::Une),
                                    token.span,
                                ));
                                continue;
                            }
                            _ => (),
                        }
                    }
                    expect_operand_next = true;
                }
                Token::Comma | Token::SemiColon => {
                    expect_operand_next = true;
                }
                _ => (),
            }
            mod_vec.push(token.clone());
        }
        mod_vec
    }

    fn validate_gap(expr: &str, start: usize, end: usize) -> Result<(), ParseError> {
        let gap = &expr[start..end];
        if gap.chars().all(char::is_whitespace) {
            return Ok(());
        }
        // The span covers the trimmed text, not the surrounding whitespace, so
        // the caret sits under the offending characters themselves.
        let leading = gap.len() - gap.trim_start().len();
        let trimmed = gap.trim();
        Err(ParseError::UnexpectedCharacter {
            text: trimmed.to_string(),
            span: Span::new(start + leading, start + leading + trimmed.len()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{Bracket, Number, Operator};
    use num_bigint::BigInt;

    #[test]
    fn test_parse_valid() {
        assert_eq!(
            Parser::parse("1+2*3/(4-5)")
                .unwrap()
                .into_iter()
                .map(|t| t.node)
                .collect::<Vec<_>>(),
            vec![
                Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
                Token::Operator(Operator::Add),
                Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
                Token::Operator(Operator::Mul),
                Token::Operand(Number::NaturalNumber(BigInt::from(3u8))),
                Token::Operator(Operator::Div),
                Token::Bracket(Bracket::Open),
                Token::Operand(Number::NaturalNumber(BigInt::from(4u8))),
                Token::Operator(Operator::Sub),
                Token::Operand(Number::NaturalNumber(BigInt::from(5u8))),
                Token::Bracket(Bracket::Close),
            ]
        );
    }

    #[test]
    fn test_parse_invalid_character() {
        assert!(Parser::parse("1@2").is_err());
    }

    #[test]
    fn test_parse_records_the_span_of_every_token() {
        let tokens = Parser::parse("1 + 23").unwrap();
        let spans: Vec<(usize, usize)> =
            tokens.iter().map(|t| (t.span.start, t.span.end)).collect();
        assert_eq!(spans, vec![(0, 1), (2, 3), (4, 6)]);
    }

    #[test]
    fn test_parse_reports_an_unexpected_character_with_its_position() {
        assert_eq!(
            Parser::parse("1@2"),
            Err(ParseError::UnexpectedCharacter {
                text: "@".to_string(),
                span: Span::new(1, 2),
            })
        );
    }

    #[test]
    fn test_parse_rejects_an_expression_with_no_tokens() {
        assert_eq!(Parser::parse("   "), Err(ParseError::EmptyExpression));
    }

    /// '×' is two bytes and one column. Every other test in this module uses pure
    /// ASCII, where the byte offset and the char offset are the same number, so
    /// none of them would notice if these spans started being counted in chars —
    /// while every caret in the crate would silently shift on any expression
    /// containing '×' or '÷'.
    #[test]
    fn test_spans_are_byte_offsets_not_char_offsets() {
        let tokens = Parser::parse("2×3").unwrap();
        let spans: Vec<(usize, usize)> =
            tokens.iter().map(|t| (t.span.start, t.span.end)).collect();
        assert_eq!(spans, vec![(0, 1), (1, 3), (3, 4)]);
    }

    /// The unary minus keeps the position of the '-' it replaces, so an error
    /// reported against it later points at something the user actually typed.
    #[test]
    fn test_the_unary_minus_keeps_its_position() {
        let tokens = Parser::parse("-5").unwrap();
        assert_eq!(tokens[0].node, Token::Operator(Operator::Une));
        assert_eq!(tokens[0].span, Span::new(0, 1));
    }

    #[test]
    fn test_multiple_unary_ops2() {
        // -(+(-5*-5)) to #((#5*#5))

        // The spans carried on the input tokens are irrelevant to this test —
        // it exercises the unary-marking logic, not span propagation — so an
        // arbitrary placeholder span is used throughout.
        let no_span = Span::new(0, 0);
        let input = vec![
            Spanned::new(Token::Operator(Operator::Sub), no_span),
            Spanned::new(Token::Bracket(Bracket::Open), no_span),
            Spanned::new(Token::Operator(Operator::Add), no_span),
            Spanned::new(Token::Bracket(Bracket::Open), no_span),
            Spanned::new(Token::Operator(Operator::Sub), no_span),
            Spanned::new(
                Token::Operand(Number::NaturalNumber(BigInt::from(5u8))),
                no_span,
            ),
            Spanned::new(Token::Operator(Operator::Mul), no_span),
            Spanned::new(Token::Operator(Operator::Sub), no_span),
            Spanned::new(
                Token::Operand(Number::NaturalNumber(BigInt::from(5u8))),
                no_span,
            ),
            Spanned::new(Token::Bracket(Bracket::Close), no_span),
            Spanned::new(Token::Bracket(Bracket::Close), no_span),
        ];

        let expected = vec![
            Token::Operator(Operator::Une),
            Token::Bracket(Bracket::Open),
            Token::Bracket(Bracket::Open),
            Token::Operator(Operator::Une),
            Token::Operand(Number::NaturalNumber(BigInt::from(5u8))),
            Token::Operator(Operator::Mul),
            Token::Operator(Operator::Une),
            Token::Operand(Number::NaturalNumber(BigInt::from(5u8))),
            Token::Bracket(Bracket::Close),
            Token::Bracket(Bracket::Close),
        ];

        let result: Vec<Token> = Parser::mod_unary_operators(&input)
            .into_iter()
            .map(|t| t.node)
            .collect();
        assert_eq!(result, expected);
    }
}
