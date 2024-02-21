use crate::token::{self, Operator, Token};

use log::debug;
use once_cell::sync::Lazy;
use regex::Regex;

/// The Parser has 2 primary functions:
/// to parse the math expression with a Regex and to tokenise the math &[str] expression
///
#[derive(Debug)]
pub struct Parser;

static EXPRESSION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d+\.?\d*|\.\d+|[-+*/^()=×÷!]|[a-zA-Z_][a-zA-Z0-9_]*|)")
    .expect("Should compile regex"));

impl Parser {
    /// Parses and splits a &str into a vec of &str with
    /// the help of [`EXPRESSION_REGEX`] and then wraps in tokens the &str chunks
    ///
    pub fn parse(expr: &str) -> Vec<Token> {
        let vex: Vec<Token<'_>> = EXPRESSION_REGEX
            .find_iter(expr)
            .map(|m| m.as_str())
            .filter_map(|s| Token::tokenize(s))
            .collect();

        Self::mod_unary_operators(&vex)
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
                _ => (),
            }
            mod_vec.push(token.clone());
        }
        mod_vec
    }
}

#[cfg(test)]
mod tests {
    use num_bigint::BigInt;
    use super::*;
    use crate::token::{Bracket, Number, Operator};

    #[test]
    fn test_parse_valid() {
        assert_eq!(
            Parser::parse("1+2*3/(4-5)"),
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
