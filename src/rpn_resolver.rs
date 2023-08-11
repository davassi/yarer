

use std::collections::{HashMap, VecDeque};

use crate::{parser::*, token::{Token, Operator, Number, MathFunction}};
pub struct RpnResolver<'a> {
    rpn_expr: VecDeque<Token<'a>>,
    local_heap: HashMap<String, Number>,
}

fn dump_debug(v: &VecDeque<Token>) -> () {
    v.iter().for_each(|f| print!("{}",f));      
}

fn dump_debug2(v: &Vec<Token>) -> () {
    v.iter().for_each(|f| print!("{}",f));      
}

/// Here relies the core logic of Yarer. 
impl RpnResolver<'_> {

    pub fn parse<'a>(exp : &'a str) -> RpnResolver {

        let tokenised_expr: Vec<Token<'a>> = Parser::parse(exp).unwrap(); //dump_debug(&tokenised_expr);
        let (rpn_expr , local_heap)
             = RpnResolver::reverse_polish_notation(&tokenised_expr);

        RpnResolver { rpn_expr, local_heap }
    }

    pub fn resolve(&mut self) -> Result<Number, &str> {
        
        let mut result_stack: VecDeque<Number> = VecDeque::new();

        while !self.rpn_expr.is_empty() {
            let t: Token = self.rpn_expr.pop_front().unwrap();
           
            match t {
                Token::Operand(n) => {
                    result_stack.push_back(n);
                },
                Token::Operator(op) => {
                    let right_value: Number = result_stack.pop_back().unwrap();
                    let left_value: Number = result_stack.pop_back().unwrap();

                    match op {
                        Operator::Add => result_stack.push_back(left_value+right_value),
                        Operator::Sub => result_stack.push_back(left_value-right_value),
                        Operator::Mul => result_stack.push_back(left_value*right_value),
                        Operator::Div => result_stack.push_back(left_value/right_value),
                        Operator::Pow => result_stack.push_back(left_value^right_value),
                        Operator::Eql => {
                            println!("LEFT VALUE {} RIGHT VALUE {}", left_value.to_string(), right_value);
                            self.local_heap.insert(left_value.to_string(), right_value);
                            result_stack.push_back(right_value)
                        }
                        _ => panic!("rpn_resolver.rs:55 - Operator {} Not implemented!", op)
                    }
                },
                Token::Function(fun) => {
                    let value: Number = result_stack.pop_back().unwrap();
                    
                    let res = match fun {
                        MathFunction::Sin => f64::sin(value.into()),
                        MathFunction::Cos => f64::cos(value.into()),
                        MathFunction::Tan => f64::tan(value.into()),
                        MathFunction::Abs => f64::abs(value.into()),
                        MathFunction::Max => {
                            let value2: Number = result_stack.pop_back().unwrap();
                            f64::max(value.into(), value2.into())
                        },
                        MathFunction::Min => {
                            let value2: Number = result_stack.pop_back().unwrap();
                            f64::min(value.into(), value2.into())
                        },
                        MathFunction::None => panic!("This should not happen!"),
                    };
                    result_stack.push_back(Number::DecimalNumber(res));
                },
                Token::Variable(v) => {

                    let n = self.local_heap.get(v)
                        .unwrap_or_else(|| {&Number::NaturalNumber(0)});
                    result_stack.push_back(*n);
                }
                _ => panic!("This '{}' cannot be yet recognised!", t),
            }
        }
        result_stack.pop_front().ok_or("Something went terribly wrong here.")
       
    }

    /* Transforming an infix notation to Reverse Polish Notation (RPN) */
    fn reverse_polish_notation<'a>(infix_stack: &Vec<Token<'a>>) -> (VecDeque<Token<'a>>, HashMap<String, Number>) {
        
        /*  Create an empty stack for keeping operators. Create an empty list for output. */
        let mut operators_stack: Vec<Token> = Vec::new();
        let mut postfix_stack: VecDeque<Token> = VecDeque::new();
        let mut local_heap: HashMap<String, Number> = RpnResolver::init_local_heap();

        /* Scan the infix expression from left to right. */
        infix_stack.into_iter().for_each(|t: &Token| {

            match *t {
                /* If the token is an operand, add it to the output list. */
                Token::Operand(_) => postfix_stack.push_back(*t),

                /* If the token is a left parenthesis, push it on the stack. */
                Token::Bracket(crate::token::Bracket::Open) => operators_stack.push(*t),
                
                /* If the token is a right parenthesis:
                    Pop the stack and add operators to the output list until you encounter a left parenthesis.
                    Pop the left parenthesis from the stack but do not add it to the output list.*/
                Token::Bracket(crate::token::Bracket::Close) => {

                    while let Some(token) = operators_stack.pop() {
                        match token {
                            Token::Bracket(crate::token::Bracket::Open) => break, // discards left parenthesis
                            _ => postfix_stack.push_back(token),
                        }
                    }
                },

                /* If the token is an operator, op1, then:
                   while there is an operator, op2, at the top of the stack, and op1 is left-associative 
                   and its precedence is less than or equal to that of op2, 
                   or op1 is right-associative and its precedence is less than that of op2:
                      pop op2 off the stack, onto the output list;
                    push op1 on the stack.*/
                Token::Operator(_) => {
                    let op1 = *t;
                    if !operators_stack.is_empty() {
                        let op2: &Token = operators_stack.last().unwrap();
                        match op2 {
                            Token::Operator(_) => {
                                if Token::compare_operator_priority(op1, *op2) {
                                    postfix_stack.push_back(operators_stack.pop().unwrap());
                                }
                            },
                            _ => (),
                        }
                    }
                    operators_stack.push(op1);   
                },

                Token::Function(_) => { 
                    operators_stack.push(*t);
                },

                /* If the token is a variable, add it to the output list and to the local_heap with a default value*/
                Token::Variable(s) => { 
                    postfix_stack.push_back(*t);
                    local_heap.insert(s.to_string(), Number::NaturalNumber(0));
                },
                
            }            
            print!("Inspecting... {}", *t);
            print!(" - OUT ");dump_debug(&postfix_stack);
            print!(" - OP ");dump_debug2(&operators_stack);
            println!();
        });

        /* After all tokens are read, pop remaining operators from the stack and add them to the list.  */
        while !operators_stack.is_empty() {
            postfix_stack.push_back(operators_stack.pop().unwrap());
        }
      
        /*print!(" - OUT ");dump_debug(&postfix_stack);
        print!(" - OP ");dump_debug2(&operators_stack);
        println!();*/
        (postfix_stack, local_heap)
    }

    fn init_local_heap() -> HashMap<String, Number> {
        static PI: Number = Number::DecimalNumber(3.1415);
        let mut local_heap: HashMap<String, Number> = HashMap::new();
        local_heap.insert("PI".to_string(), PI);
        local_heap
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{Number,Operator};

    #[test]
    fn test_reverse_polish_notation() {
        let a: Vec<Token> = vec![Token::Operand(Number::NaturalNumber(1)), Token::Operator(Operator::Add), Token::Operand(Number::NaturalNumber(2))];
        let b: Vec<Token> = vec![Token::Operand(Number::NaturalNumber(1)), Token::Operand(Number::NaturalNumber(2)),Token::Operator(Operator::Add)];
        assert_eq!(RpnResolver::reverse_polish_notation(&a).0, b);
    }

}
