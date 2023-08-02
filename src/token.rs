

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Number {
    NaturalNumber(u32),
    DecimalNumber(f64),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div
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
   
    fn from_operator(c : char) -> Result<Token, &'static str> {
        match c {
            '+' => Ok(Token::Operator(Operator::Add)),
            '-' => Ok(Token::Operator(Operator::Sub)),
            '*' => Ok(Token::Operator(Operator::Mul)),
            '/' => Ok(Token::Operator(Operator::Div)),
            _ => Err("operator not supported."),
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
        
        match n.parse::<u32>() {
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
            c@ ("+" | "-" | "*" | "/") =>  Token::from_operator(c.chars().next().unwrap()),
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



#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(Token::from_operator('a'), Err("operator not supported."));
        assert_eq!(Token::from_operator('1'), Err("operator not supported."));
        assert_eq!(Token::from_operator('!'), Err("operator not supported."));
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

