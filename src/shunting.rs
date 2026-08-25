//! Dijkstra's shunting-yard algorithm: reorders a validated infix token
//! stream into Reverse Polish Notation.
//!
//! Everything that used to guard against a malformed expression here —
//! bracket frames, argument counting, the arity check — now lives in
//! [`crate::validate`], which runs first and refuses those inputs before this
//! module ever sees them. What is left is the reordering itself, plus two
//! branches that are unreachable once validation has run and are kept purely
//! as a backstop; see the comments above them.

use crate::error::ParseError;
use crate::span::Spanned;
use crate::token::{self, Token};
use log::debug;
use std::collections::VecDeque;
use std::fmt::Display;

/// Transforms infix notation into Reverse Polish Notation (RPN), using
/// Dijkstra's shunting-yard algorithm.
///
/// Example
/// ``
///     "3 * 4 + 5 * 6" becomes "3 4 * 5 6 * +"
/// ``
///
/// `tokens` is expected to have already passed [`crate::validate::validate`]:
/// this function no longer checks that brackets balance, that a `,` sits
/// inside a call, or that a call has the right number of arguments — it
/// trusts the caller for all of that and only reorders.
///
/// # Errors
/// [`ParseError::Malformed`], from three sites that are each unreachable once
/// `tokens` has passed validation. They stay rather than being deleted: see
/// the comments on the two `found_open` checks below for why.
pub(crate) fn to_rpn<'a>(
    tokens: &[Spanned<Token<'a>>],
) -> Result<VecDeque<Spanned<Token<'a>>>, ParseError> {
    /*  Create an empty stack for keeping operators. Create an empty list for output. */
    let mut operators_stack: Vec<Spanned<Token>> = Vec::new();
    let mut postfix_stack: VecDeque<Spanned<Token>> = VecDeque::new();

    /* Scan the infix expression from left to right. */
    for t in tokens {
        match t.node {
            /* If the token is an operand, add it to the output list. */
            Token::Operand(_) => postfix_stack.push_back(t.clone()),

            /* If the token is a left parenthesis, push it on the stack. */
            Token::Bracket(token::Bracket::Open) => operators_stack.push(t.clone()),

            /* If the token is a right parenthesis:
            Pop the stack and add operators to the output list until you encounter a left parenthesis.
            Pop the left parenthesis from the stack but do not add it to the output list.*/
            Token::Bracket(token::Bracket::Close) => {
                let mut found_open = false;
                while let Some(token) = operators_stack.pop() {
                    match token.node {
                        Token::Bracket(token::Bracket::Open) => {
                            found_open = true;
                            // If the token is a left parenthesis, pop it from the stack
                            if let Some(op) = operators_stack.last() {
                                if matches!(op.node, Token::Function(_)) {
                                    postfix_stack.push_back(
                                        operators_stack.pop().expect("It should not happen."),
                                    );
                                }
                            }
                            break;
                        } // discards left parenthesis
                        _ => postfix_stack.push_back(token),
                    }
                }
                // `validate` has already walked this same token stream and refused
                // it unless every bracket is balanced, so a `Bracket(Close)` here
                // always has a matching `Bracket(Open)` somewhere below it on
                // `operators_stack`: the `Bracket(Open)` arm above is the only
                // thing that pushes an open bracket, this arm is the only thing
                // that pops one, and it pops exactly one per close. This branch is
                // therefore unreachable, and no test can cover it. It stays
                // anyway, because the loop above has by then drained the operator
                // stack into the output in exactly the order the normal
                // end-of-expression drain uses: the postfix sequence is still
                // evaluable, so falling through would not raise an error, it
                // would return a number computed with the bracket grouping
                // dissolved — `2*(3+4)` as `2 3 * 4 +`, which is 10 rather
                // than 14.
                if !found_open {
                    return Err(ParseError::Malformed);
                }
            }

            Token::Comma => {
                let mut found_open = false;
                while let Some(token) = operators_stack.last() {
                    if matches!(token.node, Token::Bracket(token::Bracket::Open)) {
                        found_open = true;
                        break;
                    }
                    postfix_stack.push_back(operators_stack.pop().expect("It should not happen."));
                }
                // Same invariant as the closing-bracket arm above: `validate`
                // only lets a `,` reach here inside a call it has already
                // confirmed is bracketed, so that call's open bracket is still on
                // `operators_stack`. This arm only peeks at it rather than
                // popping it, so it leaves the stack as the invariant expects.
                // Also unreachable, and kept for the same reason: the loop above
                // has already moved the intervening operators into the output,
                // so falling through would silently mis-group the arguments
                // rather than fail.
                if !found_open {
                    return Err(ParseError::Malformed);
                }
            }

            Token::SemiColon => {
                while let Some(token) = operators_stack.pop() {
                    postfix_stack.push_back(token);
                }
                postfix_stack.push_back(t.clone());
            }

            Token::Operator(_op) => {
                let op1: Spanned<Token<'_>> = t.clone();

                while !operators_stack.is_empty() {
                    let op2: &Spanned<Token> = operators_stack.last().unwrap();
                    match op2.node {
                        Token::Operator(_) => {
                            if Token::compare_operator_priority(op1.node.clone(), op2.node.clone())
                            {
                                postfix_stack.push_back(
                                    operators_stack.pop().expect("It should not happen."),
                                );
                            } else {
                                break;
                            }
                        }
                        Token::Function(_) => {
                            postfix_stack
                                .push_back(operators_stack.pop().expect("It should not happen."));
                        }
                        _ => break,
                    }
                }
                operators_stack.push(op1);
            }

            Token::Function(_) => operators_stack.push(t.clone()),

            /* If the token is a variable, add it to the output list. */
            Token::Variable(_) => postfix_stack.push_back(t.clone()),
        }
        debug!(
            "Inspecting... {} - OUT {} - OP - {}",
            t.node,
            DisplayThisDeque(&postfix_stack),
            DisplayThatVec(&operators_stack)
        );
    }

    /* After all tokens are read, pop remaining operators from the stack and add them to the list. */
    operators_stack.reverse();
    for t in &operators_stack {
        // Same reasoning as the two `found_open` checks above: a bracket left on
        // `operators_stack` once every token has been scanned means one was
        // opened and never closed, which `validate` has already refused. Kept
        // rather than deleted for the same reason — it would otherwise fall
        // through to a number computed with the grouping dissolved.
        if matches!(t.node, Token::Bracket(_)) {
            return Err(ParseError::Malformed);
        }
        postfix_stack.push_back(t.clone());
    }

    debug!(
        "DEBUG: EOF - OUT {} - OP - {}",
        DisplayThisDeque(&postfix_stack),
        DisplayThatVec(&operators_stack)
    );

    Ok(postfix_stack)
}

struct DisplayThatVec<'a>(&'a Vec<Spanned<Token<'a>>>);
struct DisplayThisDeque<'a>(&'a VecDeque<Spanned<Token<'a>>>);

impl Display for DisplayThatVec<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|t| t.node.to_string())
                .collect::<String>()
        )
    }
}

impl Display for DisplayThisDeque<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|t| t.node.to_string())
                .collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::span::Span;
    use crate::token::{Number, Operator};
    use crate::validate::validate;
    use num_bigint::BigInt;

    fn rpn(source: &str) -> Vec<Token<'_>> {
        let tokens = Parser::parse(source).expect("tokenises");
        let validated = validate(&tokens, source).expect("validates");
        to_rpn(&validated)
            .expect("reorders")
            .into_iter()
            .map(|t| t.node)
            .collect()
    }

    #[test]
    fn test_infix_becomes_postfix() {
        assert_eq!(
            rpn("1+2"),
            vec![
                Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
                Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
                Token::Operator(Operator::Add),
            ]
        );
    }

    /// Grouping is the whole point of the algorithm, and the property the two
    /// unreachable branches exist to protect: mis-grouping produces a number,
    /// not an error.
    #[test]
    fn test_brackets_survive_the_reordering() {
        let out = rpn("2*(3+4)");
        assert_eq!(out.last(), Some(&Token::Operator(Operator::Mul)));
    }

    /// The spans travel through to the output, in the order the operators are
    /// applied, so an evaluation error can name the operator that produced it.
    #[test]
    fn test_the_output_keeps_the_positions() {
        let tokens = Parser::parse("1+2").expect("tokenises");
        let validated = validate(&tokens, "1+2").expect("validates");
        let out = to_rpn(&validated).expect("reorders");
        assert_eq!(out[2].span, Span::new(1, 2));
    }
}
