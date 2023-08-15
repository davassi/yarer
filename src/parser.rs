use crate::token::{self, Token};
use clap::__derive_refs::once_cell;

use log::debug;
use regex::Regex;

use once_cell::sync::Lazy;

/// The Parser has 2 primary functions:
/// to parse the math expression with a Regex and to tokenise the math &str expression
#[derive(Debug)]
pub struct Parser;

static EXPRESSION_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\d+\.?\d*|\.\d+|[-+*/^()=×÷!]|[a-zA-Z_][a-zA-Z0-9_]*|)").unwrap());

impl Parser {
    /// Parses and splits a str into a vec of &str with the help of [`EXPRESSION_REGEX`]
    ///
    fn process(exp: &str) -> Vec<&str> {
        EXPRESSION_REGEX
            .find_iter(exp)
            .map(|m| m.as_str())
            .collect()
    }

    /// tokenise a processed str expression
    pub fn parse(expr: &str) -> Result<Vec<Token>, &str> {
        Ok(expr)
            .map(|a| Self::process(a))
            .map(|v: Vec<&str>| Token::tokenize_vec(&v))
            .map(|v: Vec<Token<'_>>| Self::mod_unary_operators(&v))
    }

    fn mod_unary_operators<'a>(v: &[Token<'a>]) -> Vec<Token<'a>> {
        let mut mod_vec: Vec<Token> = Vec::new();
        let mut expect_operand_next = true;

        for &token in v.iter() {
            debug!("{}", token);

            match token {
                Token::Operand(_) | Token::Variable(_) => {
                    expect_operand_next = false;
                }
                Token::Operator(o) => {
                    if expect_operand_next {
                        debug!("-> Unary operator detected");
                        match o {
                            token::Operator::Add => {
                                // an unary + is simply ignored.
                                continue;
                            }
                            token::Operator::Sub => {
                                // an unary - is a special op
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
            mod_vec.push(token);
        }
        mod_vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{Bracket, Number, Operator};

    #[test]
    fn test_process_valid() {
        assert_eq!(
            Parser::process("1+2*3/(4-5)"),
            vec!["1", "+", "2", "*", "3", "/", "(", "4", "-", "5", ")"]
        );
        assert_eq!(Parser::process("100*3.14"), (vec!["100", "*", "3.14"]));
        assert_eq!(Parser::process("x-y"), (vec!["x", "-", "y"]));
        assert_eq!(Parser::process("cos(10)"), (vec!["cos", "(", "10", ")"]));
        assert_eq!(
            Parser::process("cos(-10)"),
            vec!["cos", "(", "-", "10", ")"]
        );
        assert_eq!(Parser::process("3+5*x"), (vec!["3", "+", "5", "*", "x"]));
        assert_eq!(
            Parser::process("-3.14*variableName123/alpha_beta"),
            (vec!["-", "3.14", "*", "variableName123", "/", "alpha_beta"])
        );
    }

    #[test]
    fn test_parse_valid() {
        assert_eq!(
            Parser::parse("1+2*3/(4-5)"),
            Ok(vec![
                Token::Operand(Number::NaturalNumber(1)),
                Token::Operator(Operator::Add),
                Token::Operand(Number::NaturalNumber(2)),
                Token::Operator(Operator::Mul),
                Token::Operand(Number::NaturalNumber(3)),
                Token::Operator(Operator::Div),
                Token::Bracket(Bracket::Open),
                Token::Operand(Number::NaturalNumber(4)),
                Token::Operator(Operator::Sub),
                Token::Operand(Number::NaturalNumber(5)),
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
            Token::Operand(Number::NaturalNumber(5)),
            Token::Operator(Operator::Mul),
            Token::Operator(Operator::Sub),
            Token::Operand(Number::NaturalNumber(5)),
            Token::Bracket(Bracket::Close),
            Token::Bracket(Bracket::Close),
        ];

        let expected = vec![
            Token::Operator(Operator::Une),
            Token::Bracket(Bracket::Open),
            Token::Bracket(Bracket::Open),
            Token::Operator(Operator::Une),
            Token::Operand(Number::NaturalNumber(5)),
            Token::Operator(Operator::Mul),
            Token::Operator(Operator::Une),
            Token::Operand(Number::NaturalNumber(5)),
            Token::Bracket(Bracket::Close),
            Token::Bracket(Bracket::Close),
        ];

        let result = Parser::mod_unary_operators(&input);
        assert_eq!(result, expected);
    }
}
