use std::{fmt::Display, ops::{Add, Sub, Div, BitXor, Mul}};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Number {
    NaturalNumber(i32),
    DecimalNumber(f64),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Eql
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Associate {
    LeftAssociative,
    RightAssociative
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Bracket {
    Open,
    Close,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Token {
    Operand(Number),
    Operator(Operator),
    Bracket(Bracket),
    Function,
    Variable,
}

impl Token {

    fn operator_priority(o : Token) -> (u8, Associate) {
        match o {
            
            Token::Operator(Operator::Add) => (1 , Associate::LeftAssociative),
            Token::Operator(Operator::Sub) => (1 , Associate::LeftAssociative),
            Token::Operator(Operator::Mul) => (2 , Associate::LeftAssociative),
            Token::Operator(Operator::Div) => (2 , Associate::LeftAssociative),
            Token::Operator(Operator::Pow) => (3 , Associate::RightAssociative),
            Token::Operator(Operator::Eql) => (0 , Associate::LeftAssociative),
            _ => panic!("Operator '{}' not recognised. This must not happen!", o),
        }
    }

    pub fn compare_operator_priority(op1 : Token, op2 : Token) -> bool {
        
        let v_op1: (u8, Associate) = self::Token::operator_priority(op1);
        let v_op2: (u8, Associate) = self::Token::operator_priority(op2);

        v_op1.1 == Associate::LeftAssociative && v_op1.0 <= v_op2.0 || 
            v_op1.1 == Associate::RightAssociative && v_op1.0 < v_op2.0
    }
   
    fn from_operator(c : char) -> Result<Token, &'static str> {
        match c {
            '+' => Ok(Token::Operator(Operator::Add)),
            '-' => Ok(Token::Operator(Operator::Sub)),
            '*' => Ok(Token::Operator(Operator::Mul)),
            '/' => Ok(Token::Operator(Operator::Div)),
            '^' => Ok(Token::Operator(Operator::Pow)),
            '=' => Ok(Token::Operator(Operator::Eql)),
            _ => Err("Operator not supported."),
        }
    }

    fn from_bracket(c : char) -> Result<Token, &'static str> {
        match c {
            '(' | '[' => Ok(Token::Bracket(Bracket::Open)),
            ')' | ']' => Ok(Token::Bracket(Bracket::Close)),
            _ => Err("operator not supported."),
        }
    }

    fn from_natural_number(n : &str) -> Result<Token, &'static str> {
        
        match n.parse::<i32>() {
            Ok(v) => Ok(Token::Operand(Number::NaturalNumber(v))),
            Err(_) => Err("Failed to parse natural number"),
        }
    }

    fn from_decimal_number(f : &str) -> Result<Token, &'static str> {

        match f.parse::<f64>() {
            Ok(v) => Ok(Token::Operand(Number::DecimalNumber(v))),
            Err(_) => Err("Failed to parse decimal number"),
        }
    }

    fn tokenize(t: &str) -> Result<Token, &str> {
        
        match t {
            c@ ("+" | "-" | "*" | "/" | "^" | "=") =>  Token::from_operator(c.chars().next().unwrap()),
            b@ ("(" | ")" | "[" | "]") =>  Token::from_bracket(b.chars().next().unwrap()),
            n if n.parse::<u32>().is_ok() => Token::from_natural_number(n),
            f if f.parse::<f64>().is_ok() => Token::from_decimal_number(f),
            _ => Err("The Token is not supported."),
        }
    }

    /* Mapping a vec of str in a vec of Tokens */
    pub fn tokenize_vec<'a>(v : &[&'a str]) -> Result<Vec<Token>, &'a str> {
        v.iter()
        .map(|t| Token::tokenize(t))
        .collect::<Result<Vec<Token>, _>>()
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Number::NaturalNumber(v) =>  write!(f, "{}", v),
            Number::DecimalNumber(v) =>  write!(f, "{}", v),
        }
    }
}

fn generic_functional_token_operation<NF, DF>(ln: Number, rn: Number, nf : NF, df: DF) -> Number
    where NF: Fn(i32,i32) -> i32,
          DF: Fn(f64,f64) -> f64, {

        match ln {
            Number::NaturalNumber(v1) => {
                match rn {
                    Number::NaturalNumber(v2) => {
                        return crate::token::Number::NaturalNumber(nf(v1,v2));
                    },
                    Number::DecimalNumber(v2) => {
                        return crate::token::Number::DecimalNumber(df(v1 as f64,v2));
                    },
                }
            },
            Number::DecimalNumber(v1) => {
                match rn {
                    Number::NaturalNumber(v2) => {
                        return crate::token::Number::DecimalNumber(df(v1, v2 as f64));
                    },
                    Number::DecimalNumber(v2) => {
                        return crate::token::Number::DecimalNumber(df(v1, v2));
                    },
                }
            },
        }
}

impl Add for Number {
    type Output = Number;

    fn add(self, rhs: Self) -> Self::Output {
        let ln: Number = self as crate::token::Number;
        let rn: Number = rhs as crate::token::Number;

        generic_functional_token_operation(ln, rn, |a : i32,b: i32| a+b, |a : f64,b: f64| a+b)
    }
}

impl Sub for Number {
    type Output = Number;

    fn sub(self, rhs: Self) -> Self::Output {
        let ln: Number = self as crate::token::Number;
        let rn: Number = rhs as crate::token::Number;

        generic_functional_token_operation(ln, rn, |a : i32,b: i32| a-b, |a : f64,b: f64| a-b)
    }
}

impl Mul for Number {
    type Output = Number;

    fn mul(self, rhs: Self) -> Self::Output {
        let ln: Number = self as crate::token::Number;
        let rn: Number = rhs as crate::token::Number;

        generic_functional_token_operation(ln, rn, |a : i32,b: i32| a*b, |a : f64,b: f64| a*b)
    }
}

impl Div for Number {
    type Output = Number;

    fn div(self, rhs: Self) -> Self::Output {
        let ln: Number = self as crate::token::Number;
        let rn: Number = rhs as crate::token::Number;

        generic_functional_token_operation(ln, rn, |a : i32,b: i32| a/b, |a : f64,b: f64| a/b)
    }
}

impl BitXor for Number {
    type Output = Number;

    fn bitxor(self, rhs: Self) -> Self::Output {
        crate::token::Number::NaturalNumber(11)
    }
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Operator::Add => write!(f, "+"),
            Operator::Sub => write!(f, "-"),
            Operator::Mul => write!(f, "*"),
            Operator::Div => write!(f, "/"),
            Operator::Pow => write!(f, "^"),
            Operator::Eql => write!(f, "="),
        }
    }
}

impl Display for Bracket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Bracket::Open => write!(f, "("),
            Bracket::Close => write!(f, ")"),
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Token::Operand(v) => write!(f, "({})", v),
            Token::Operator(v) => write!(f, "({})", v),
            Token::Bracket(v) => write!(f, "({})", v),
            Token::Function => write!(f, "TBD"),
            Token::Variable => write!(f, "TBD"),
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

  /*  #[test]
    fn test_operator_priority() {
        assert_eq!(Token::compare_operator_priority(Operator::Add, Operator::Sub), false);
        assert_eq!(Token::compare_operator_priority(Operator::Add, Operator::Mul), false);
        assert_eq!(Token::compare_operator_priority(Operator::Add, Operator::Pow), false);
        assert_eq!(Token::compare_operator_priority(Operator::Mul, Operator::Pow), false);
        assert_eq!(Token::compare_operator_priority(Operator::Pow, Operator::Pow), false);

        assert_eq!(Token::compare_operator_priority(Operator::Pow, Operator::Mul), true);
        assert_eq!(Token::compare_operator_priority(Operator::Pow, Operator::Add), true);
        assert_eq!(Token::compare_operator_priority(Operator::Mul, Operator::Add), true);
        assert_eq!(Token::compare_operator_priority(Operator::Div, Operator::Sub), true);
    } */

    #[test]
    fn test_tokenise_operators() {
        let v = vec!["1", "+", "2.1"];
        assert_eq!(Token::tokenize(v[1]).unwrap(), Token::Operator(Operator::Add));
        assert_eq!(Token::tokenize(v[0]).unwrap(), Token::Operand(Number::NaturalNumber(1)));
        assert_eq!(Token::tokenize(v[2]).unwrap(), Token::Operand(Number::DecimalNumber(2.1)));
    }

    #[test]
    fn test_from_operator_valid() {
        assert_eq!(Token::from_operator('+'), Ok(Token::Operator(Operator::Add)));
        assert_eq!(Token::from_operator('-'), Ok(Token::Operator(Operator::Sub)));
        assert_eq!(Token::from_operator('*'), Ok(Token::Operator(Operator::Mul)));
        assert_eq!(Token::from_operator('/'), Ok(Token::Operator(Operator::Div)));
    }

    #[test]
    fn test_from_operator_invalid() {
        assert_eq!(Token::from_operator('a'), Err("Operator not supported."));
        assert_eq!(Token::from_operator('1'), Err("Operator not supported."));
        assert_eq!(Token::from_operator('!'), Err("Operator not supported."));
    }

    #[test]
    fn test_from_natural_number() {
        assert_eq!(Token::from_natural_number("42"), Ok(Token::Operand(Number::NaturalNumber(42))));
        assert_eq!(Token::from_natural_number("10"), Ok(Token::Operand(Number::NaturalNumber(10))));
        assert_eq!(Token::from_natural_number("0"), Ok(Token::Operand(Number::NaturalNumber(0))));
        assert_eq!(Token::from_natural_number("123456"), Ok(Token::Operand(Number::NaturalNumber(123456))));
    }

    #[test]
    fn test_from_natural_number_invalid() {
        assert_eq!(Token::from_natural_number("10.5"), Err("Failed to parse natural number"));
        assert_eq!(Token::from_natural_number("-1"), Err("Failed to parse natural number"));
        assert_eq!(Token::from_natural_number("abc"), Err("Failed to parse natural number"));
    }

    #[test]
    fn test_tokenize_valid() {
        assert_eq!(Token::tokenize("+"), Ok(Token::Operator(Operator::Add)));
        assert_eq!(Token::tokenize("100"), Ok(Token::Operand(Number::NaturalNumber(100))));
        assert_eq!(Token::tokenize("3.14"), Ok(Token::Operand(Number::DecimalNumber(3.14))));
        assert_eq!(Token::tokenize("("), Ok(Token::Bracket(Bracket::Open)));
    }

    #[test]
    fn test_tokenize_invalid() {
        assert_eq!(Token::tokenize("abc"), Err("The Token is not supported."));
        assert_eq!(Token::tokenize("1.2.3"), Err("The Token is not supported."));
        assert_eq!(Token::tokenize("++"), Err("The Token is not supported."));
    }

    #[test]
    fn test_tokenize_vec_valid() {
        let input = vec!["+", "100", "3.14", "("];
        let expected = Ok(vec![
            Token::Operator(Operator::Add),
            Token::Operand(Number::NaturalNumber(100)),
            Token::Operand(Number::DecimalNumber(3.14)),
            Token::Bracket(Bracket::Open)
        ]);
        assert_eq!(Token::tokenize_vec(&input), expected);
    }
}
