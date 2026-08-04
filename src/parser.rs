use crate::rpn_resolver::MALFORMED_ERR;
use crate::token::{self, Operator, Token};

use anyhow::{anyhow, Result};
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
    /// Parses and splits a &str into a vec of &str with
    /// the help of [`EXPRESSION_REGEX`] and then wraps in tokens the &str chunks
    ///
    pub fn parse(expr: &str) -> Result<Vec<Token<'_>>> {
        let mut vex: Vec<Token<'_>> = Vec::new();
        let mut cursor = 0;

        for m in EXPRESSION_REGEX.find_iter(expr) {
            Self::validate_gap(expr, cursor, m.start())?;
            vex.push(Token::tokenize(m.as_str()).ok_or_else(|| anyhow!(MALFORMED_ERR))?);
            cursor = m.end();
        }

        Self::validate_gap(expr, cursor, expr.len())?;

        if vex.is_empty() {
            return Err(anyhow!(MALFORMED_ERR));
        }

        Ok(Self::mod_unary_operators(&vex))
    }

    /// Finds out all the unary operators that are present in the expression
    ///
    fn mod_unary_operators<'a>(v: &[Token<'a>]) -> Vec<Token<'a>> {
        let mut mod_vec: Vec<Token> = Vec::new();
        let mut expect_operand_next = true;

        for token in v {
            debug!("{}", token);

            match &token {
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
                                mod_vec.push(token::Token::Operator(token::Operator::Une));
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

    fn validate_gap(expr: &str, start: usize, end: usize) -> Result<()> {
        let gap = &expr[start..end];
        if gap.chars().all(char::is_whitespace) {
            return Ok(());
        }

        Err(anyhow!("Parse Error: Unexpected token '{}'.", gap.trim()))
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
            Parser::parse("1+2*3/(4-5)").unwrap(),
            (vec![
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
            ])
        );
    }

    #[test]
    fn test_parse_invalid_character() {
        assert!(Parser::parse("1@2").is_err());
    }

    #[test]
    fn test_multiple_unary_ops2() {
        // -(+(-5*-5)) to #((#5*#5))

        let input = vec![
            Token::Operator(Operator::Sub),
            Token::Bracket(Bracket::Open),
            Token::Operator(Operator::Add),
            Token::Bracket(Bracket::Open),
            Token::Operator(Operator::Sub),
            Token::Operand(Number::NaturalNumber(BigInt::from(5u8))),
            Token::Operator(Operator::Mul),
            Token::Operator(Operator::Sub),
            Token::Operand(Number::NaturalNumber(BigInt::from(5u8))),
            Token::Bracket(Bracket::Close),
            Token::Bracket(Bracket::Close),
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

        let result = Parser::mod_unary_operators(&input);
        assert_eq!(result, expected);
    }
}
