

use regex::Regex;

use crate::token::Token;

/// The expression parser has only 2 functions: to parse and to tokenise a math expression
#[derive(Debug)]
pub struct Parser;

impl Parser {

    /// The Parser parses and splits a str into a vec of &str using 
    fn process(exp: &str) -> Result<Vec<&str>, &str> {

        let regex = Regex::new(r"(\d+\.?\d*|\.\d+|[-+*/^()=,×÷]|[a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

        Ok(regex.find_iter(exp)
           .map(|m| m.as_str())
           .collect())
    }
    
    /// tokenise a processed str expression
    pub fn parse(exp: &str) -> Result<Vec<Token>, &str> {

        Self::process(exp)
            .and_then(|v: Vec<&str>| Token::tokenize_vec(&v))
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
