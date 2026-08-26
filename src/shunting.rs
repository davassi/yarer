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
            // A variable is an operand in the algorithm's own terms: neither
            // needs anything done beyond landing in the output in the order
            // it was read.
            Token::Operand(_) | Token::Variable(_) => postfix_stack.push_back(t.clone()),

            // A left bracket and a function name both wait on the operator
            // stack instead of going straight to the output, and it is the
            // same closing bracket that releases both: the `Bracket(Close)`
            // arm below discards the open bracket, and — because a
            // function's argument list *is* the bracketed group that follows
            // it — immediately also emits a function found waiting directly
            // underneath that bracket. Pushing a function here the same way
            // as a bracket is what makes that single arm able to end the
            // call.
            Token::Bracket(token::Bracket::Open) | Token::Function(_) => {
                operators_stack.push(t.clone());
            }

            /* If the token is a right parenthesis:
            Pop the stack and add operators to the output list until you encounter a left parenthesis.
            Pop the left parenthesis from the stack but do not add it to the output list.*/
            Token::Bracket(token::Bracket::Close) => {
                let mut found_open = false;
                while let Some(token) = operators_stack.pop() {
                    match token.node {
                        Token::Bracket(token::Bracket::Open) => {
                            found_open = true;
                            // The open bracket is discarded here; a function
                            // waiting directly underneath it — pushed the
                            // same way as this bracket, for exactly this
                            // reason — is released to the output right after.
                            if let Some(function) =
                                operators_stack.pop_if(|op| matches!(op.node, Token::Function(_)))
                            {
                                postfix_stack.push_back(function);
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
                    // The loop condition just read `last()` and got `Some`, and
                    // nothing between there and here touches `operators_stack`,
                    // so this `pop` cannot be `None`.
                    postfix_stack
                        .push_back(operators_stack.pop().expect("the stack was not empty"));
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

            Token::Operator(op) => {
                let op1: Spanned<Token<'_>> = t.clone();

                // A prefix operator arrives where a value arrives, so nothing
                // on the stack has its right operand yet and nothing may be
                // popped for it — the precedence comparison below does not
                // apply. `Une` is stronger than anything it could displace and
                // never noticed; `not` is weaker than `+`, `-`, `*`, `/` and
                // `^`, and without this `1 - not 0` would pop the `-` and hand
                // the evaluator `1 - 0 not`, a binary minus with one operand.
                //
                // A postfix operator is a different matter: `!` consumes the
                // value on its left, which is already in the output, so popping
                // on its behalf is correct and it is not covered here.
                if !op.is_prefix() {
                    // Peeking with `while let` rather than testing `is_empty()` and
                    // unwrapping: the emptiness check and the value that depends on
                    // it are then the same expression, and cannot drift apart.
                    // Both `expect`s below rest on the same fact — the loop
                    // condition has just read `last()` and got `Some`, and nothing
                    // between there and the `pop` touches `operators_stack`, so
                    // neither can be `None`.
                    while let Some(op2) = operators_stack.last() {
                        match op2.node {
                            Token::Operator(op2_op) => {
                                if Token::compare_operator_priority(op, op2_op) {
                                    postfix_stack.push_back(
                                        operators_stack.pop().expect("the stack was not empty"),
                                    );
                                } else {
                                    break;
                                }
                            }
                            Token::Function(_) => {
                                postfix_stack.push_back(
                                    operators_stack.pop().expect("the stack was not empty"),
                                );
                            }
                            _ => break,
                        }
                    }
                }
                operators_stack.push(op1);
            }
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

    /// Same helper as `rpn`, but keeping the spans instead of discarding
    /// them, for the tests below that pin exactly which token a span landed
    /// on.
    fn rpn_spanned(source: &str) -> Vec<Spanned<Token<'_>>> {
        let tokens = Parser::parse(source).expect("tokenises");
        let validated = validate(&tokens, source).expect("validates");
        to_rpn(&validated).expect("reorders").into_iter().collect()
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
    ///
    /// "1+2" tokenises as '1'@(0,1) '+'@(1,2) '2'@(2,3). Neither operand is
    /// ever popped mid-loop — the `+` sits on the operator stack until the
    /// end-of-expression drain — so this pins the two easy cases: an
    /// operand's span survives untouched, and the operator that never left
    /// the stack keeps its own span too.
    #[test]
    fn test_the_output_keeps_the_positions() {
        let out = rpn_spanned("1+2");
        assert_eq!(out[0].span, Span::new(0, 1)); // '1'
        assert_eq!(out[1].span, Span::new(2, 3)); // '2'
        assert_eq!(out[2].span, Span::new(1, 2)); // '+'
    }

    /// "1+2+3" tokenises as '1'@(0,1) '+'@(1,2) '2'@(2,3) '+'@(3,4) '3'@(4,5).
    /// Both '+' have equal precedence and left-associate, so the second '+'
    /// pops the first out of the `Operator` arm's while loop the moment it
    /// arrives — the one mid-loop reordering path `test_the_output_keeps_the_positions`
    /// does not exercise. Expected RPN order, worked by hand from the
    /// algorithm: 1, 2, +(first, popped mid-loop), 3, +(second, popped by the
    /// end-of-expression drain).
    #[test]
    fn test_a_mid_loop_precedence_pop_keeps_its_span() {
        let out = rpn_spanned("1+2+3");
        assert_eq!(out[0].span, Span::new(0, 1)); // '1'
        assert_eq!(out[1].span, Span::new(2, 3)); // '2'
        assert_eq!(out[2].span, Span::new(1, 2)); // '+' popped mid-loop
        assert_eq!(out[3].span, Span::new(4, 5)); // '3'
        assert_eq!(out[4].span, Span::new(3, 4)); // '+' popped by the final drain
    }

    /// "(1+2)*3" tokenises as '('@(0,1) '1'@(1,2) '+'@(2,3) '2'@(3,4)
    /// ')'@(4,5) '*'@(5,6) '3'@(6,7). The '+' never reaches the operator
    /// arm's own priority loop — it is pushed straight onto an empty stack
    /// below the open bracket — so its span can only travel through the
    /// `Bracket(Close)` arm's drain, worked by hand: 1, 2, + (drained by the
    /// close bracket), 3, * (drained at end of expression).
    #[test]
    fn test_a_bracket_close_drain_keeps_its_span() {
        let out = rpn_spanned("(1+2)*3");
        assert_eq!(out[0].span, Span::new(1, 2)); // '1'
        assert_eq!(out[1].span, Span::new(3, 4)); // '2'
        assert_eq!(out[2].span, Span::new(2, 3)); // '+' drained by ')'
        assert_eq!(out[3].span, Span::new(6, 7)); // '3'
        assert_eq!(out[4].span, Span::new(5, 6)); // '*' drained at end
    }
}
