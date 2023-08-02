

use std::collections::HashMap;

use crate::{parser::*, token::Token};

pub struct RpnResolver {
    rpn_expr: Vec<Token>,
    local_heap: HashMap<String, Token>,
}

impl RpnResolver {

    pub fn parse(exp : &str) -> Self {

        println!("{:?}", exp);

        let tokenised_expr: Vec<Token> = Parser::parse(exp).unwrap();
        println!("{:?}", tokenised_expr);
        
        let rpn_expr: Vec<Token> = self::RpnResolver::reverse_polish_notation(&tokenised_expr);
        println!("{:?}", rpn_expr);

        RpnResolver { rpn_expr, local_heap : HashMap::new() }
    }

    pub fn resolve(&self) -> Result<Token, &str> {
        
        Ok(Token::Operand(crate::token::Number::NaturalNumber(42)))
    }

    /* Transforming an infix notation to Reverse Polish Notation (RPN) */
    fn reverse_polish_notation(infix_stack: &Vec<Token>) -> Vec<Token> {
        
        /*  Create an empty stack for keeping operators. Create an empty list for output. */
        let mut operators_stack: Vec<Token> = Vec::new();
        let mut postfix_stack: Vec<Token> = Vec::new();

        /* Scan the infix expression from left to right. */
        infix_stack.into_iter().for_each(|t: &Token| {

            match *t {
                /* If the token is an operand, add it to the output list. */
                Token::Operand(_) => postfix_stack.push(*t),

                /* If the token is a left parenthesis, push it on the stack. */
                Token::Bracket(crate::token::Bracket::Open) => operators_stack.push(*t),
                
                /* If the token is a right parenthesis:
                    Pop the stack and add operators to the output list until you encounter a left parenthesis.
                    Pop the left parenthesis from the stack but do not add it to the output list.*/
                Token::Bracket(crate::token::Bracket::Close) => {

                    while let Some(token) = operators_stack.pop() {
                        match token {
                            Token::Bracket(crate::token::Bracket::Open) => break,
                            _ => postfix_stack.push(token),
                        }
                    }
                    operators_stack.pop();
                },

                /* If the token is an operator, op1, then:
                   while there is an operator, op2, at the top of the stack, and op1 is left-associative 
                   and its precedence is less than or equal to that of op2, 
                   or op1 is right-associative and its precedence is less than that of op2:
                      pop op2 off the stack, onto the output list;
                    push op1 on the stack.*/
                Token::Operator(_) => {

                },

                Token::Function => { todo!();},

                /* If the token is a variable, add it to the output list and to the local_heap */
                Token::Variable => { todo!();},
            }            
        });

        /* After all tokens are read, pop remaining operators from the stack and add them to the list.  */
        postfix_stack
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{Number,Operator,Bracket};

    #[test]
    fn test_reverse_polish_notation() {
        let a: Vec<Token> = vec![Token::Operand(Number::NaturalNumber(1)), Token::Operator(Operator::Add), Token::Operand(Number::NaturalNumber(2))];
        let b: Vec<Token> = vec![Token::Operand(Number::NaturalNumber(1)), Token::Operand(Number::NaturalNumber(2)),Token::Operator(Operator::Add)];
        assert_eq!(RpnResolver::reverse_polish_notation(&a), b);
    }




}
