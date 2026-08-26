use crate::error::ParseError;
use crate::span::{Span, Spanned};
use crate::token::Token;

use once_cell::sync::Lazy;
use regex::Regex;

/// The Parser has 2 primary functions:
/// to parse the math expression with a Regex and to tokenise the math &[str] expression
///
#[derive(Debug)]
pub(crate) struct Parser;

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

        Ok(vex)
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
}
