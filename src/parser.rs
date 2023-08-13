

use log::debug;
use regex::Regex;

use crate::token::{Token, self};

/// The Parser has only 2 functions: to parse with a Regex and to tokenise a math &str expression
#[derive(Debug)]
pub struct Parser;

impl Parser { //r"(\d+\.?\d*|\.\d+|[-+*/^()=×÷]|[a-zA-Z_][a-zA-Z0-9_]*)"

    /// The Parser parses and splits a str into a vec of &str using 
    fn process(exp: &str) -> Result<Vec<&str>, &str> {

        Regex::new(r"(\d+\.?\d*|\.\d+|[-+*/^()=×÷]|[a-zA-Z_][a-zA-Z0-9_]*|)")
            .map_err(|_| {"Regex failed"})
            .map(|regex| regex.find_iter(exp)
            .map(|m| m.as_str())
            .collect())
    }
    
    /// tokenise a processed str expression
    pub fn parse(exp: &str) -> Result<Vec<Token>, &str> {

        Self::process(exp)
            .and_then(|v: Vec<&str>| Token::tokenize_vec(&v))
            .and_then(|v: Vec<Token<'_>>| Self::mod_unary_operators(&v))
    }

    fn mod_unary_operators<'a>(v: &[Token<'a>]) -> Result<Vec<Token<'a>>, &'a str> {

        let mut mod_vec: Vec<Token> = Vec::new();
        let mut last_was_operator = true;
      
        for &token in v.iter() {
            debug!("{}", token);

            match token {
                Token::Operand(_) => { last_was_operator = false; },
                Token::Operator(o) => {
                    if last_was_operator {
                        debug!("-> Unary operator detected");
                        last_was_operator = false;
                        match o {
                            token::Operator::Add => continue, // an unary + is simply ignored.
                            token::Operator::Sub => { // an unary - is a special op
                                mod_vec.push(token::Token::Operator(token::Operator::Une));
                                last_was_operator = true;
                                continue;
                            },
                            _ => (),
                        }
                    }
                    last_was_operator = true;
                },
                _ => (),
            }
            mod_vec.push(token);
        };
        Ok(mod_vec)
    }
        
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{Number,Operator,Bracket};

    #[test]
    fn test_process_valid() {
        assert_eq!(Parser::process("1+2*3/(4-5)"), Ok(vec!["1", "+", "2", "*", "3", "/", "(", "4", "-", "5", ")"]));
        assert_eq!(Parser::process("100*3.14"), Ok(vec!["100", "*", "3.14"]));
        assert_eq!(Parser::process("x-y"), Ok(vec!["x", "-", "y"]));  
        assert_eq!(Parser::process("cos(10)"), Ok(vec!["cos", "(", "10",")"]));  
        assert_eq!(Parser::process("cos(-10)"), Ok(vec!["cos", "(", "-", "10",")"]));  
        assert_eq!(Parser::process("3+5*x"), Ok(vec!["3", "+", "5", "*", "x"]));
        assert_eq!(Parser::process("-3.14*variableName123/alpha_beta"), Ok(vec!["-", "3.14", "*", "variableName123", "/", "alpha_beta"]));
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

   
}