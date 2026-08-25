use crate::functions::{decimal_from_f64, number_to_f64};
use crate::limits::{self, Limits};
use crate::{
    functions,
    parser::Parser,
    session::Session,
    span::Spanned,
    token::{self, MathFunction, Number, Operator, Token},
};
use anyhow::anyhow;
use log::debug;
use num::Integer;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    fmt::Display,
    rc::Rc,
};

use num::{BigInt, BigUint, One, Zero};
use num_rational::BigRational;
use num_traits::ToPrimitive;

pub(crate) static MALFORMED_ERR: &str = "Runtime Error: The mathematical expression is malformed.";
static DIVISION_ZERO_ERR: &str = "Runtime error: Divide by zero.";
static NO_VARIABLE_ERR: &str = "Runtime error: No variable has been defined for assignment.";
static FACTORIAL_NATURAL_ERR: &str =
    "Runtime error: Factorial is only defined for non-negative integers.";
static BUILTIN_CONSTANT_ERR: &str = "Runtime error: Built-in constants are read-only.";
pub(crate) static INVALID_FUNCTION_RESULT_ERR: &str =
    "Runtime error: Function result is not a real number.";
static INVALID_POWER_ERR: &str = "Runtime error: Invalid power operation.";
pub(crate) static FLOAT_EVAL_TOO_LARGE_ERR: &str =
    "Runtime error: Operand is too large for floating-point evaluation.";
static POWER_TOO_LARGE_ERR: &str =
    "Runtime error: Power operands are too large for non-integer evaluation.";
static EXPONENT_TOO_LARGE_ERR: &str =
    "Runtime error: the exponent is too large to evaluate under any size limit.";
static COMMA_OUTSIDE_CALL_ERR: &str =
    "Parse Error: ',' is only valid between the arguments of a function call.";
static UNBALANCED_BRACKET_ERR: &str = "Parse Error: Unbalanced brackets.";
static EMPTY_ARGUMENT_ERR: &str = "Parse Error: A function argument cannot be empty.";
static BRACKET_UNCLOSED_AT_SEMICOLON_ERR: &str =
    "Parse Error: A bracket must be closed before ';'.";

/// The main [`RpnResolver`] contains the core logic of Yarer
/// for parsing and evaluating a math expression.
///
/// It holds the tokenised expression (by the [`Parser`]) and
/// a heap of local variables borrowed from a [`Session`]
///
pub struct RpnResolver<'a> {
    rpn_expr: VecDeque<Token<'a>>,
    local_heap: Rc<RefCell<HashMap<String, Number>>>,
    build_error: Option<String>,
    limits: Limits,
}

/// One entry per open bracket, recording whether that bracket opens a function
/// call and how many arguments it has seen so far.
struct BracketFrame {
    function: Option<MathFunction>,
    /// Number of `,` separators seen so far in this bracket.
    commas: usize,
    /// Whether the *current* argument slot — since the bracket opened, or
    /// since the last `,` — has seen any content. Reset after each `,` so an
    /// empty slot (`max(,1)`, `max(1,)`) is caught exactly where it occurs,
    /// rather than being silently absorbed into the overall argument count.
    has_content: bool,
}

impl BracketFrame {
    /// An empty pair of brackets carries zero arguments; otherwise there is one
    /// more argument than there are separators. This is only accurate once
    /// every slot has been confirmed non-empty, which the caller enforces at
    /// each `,` and at the closing bracket before trusting this count.
    fn argument_count(&self) -> usize {
        if self.has_content {
            self.commas + 1
        } else {
            0
        }
    }
}

/// The error reported when a function name is not immediately followed by an
/// opening bracket. Built in one place because the check itself runs at two
/// points: mid-scan, against whatever token follows the function name, and
/// once more at end of input, for a function name with nothing after it at all.
fn function_requires_parentheses_err(fun: MathFunction) -> anyhow::Error {
    anyhow!(
        "Parse Error: Function '{}' must be followed by '('.",
        fun.to_string().to_lowercase()
    )
}

impl RpnResolver<'_> {
    /// Generates a new [`RpnResolver`] instance with borrowed heap
    ///
    pub fn parse_with_borrowed_heap<'a>(
        exp: &'a str,
        borrowed_heap: Rc<RefCell<HashMap<String, Number>>>,
        limits: Limits,
    ) -> RpnResolver<'a> {
        let heap_for_parse = Rc::clone(&borrowed_heap);
        match Parser::parse(exp)
            .map_err(anyhow::Error::from)
            .and_then(|tokenised_expr| {
                RpnResolver::reverse_polish_notation(&tokenised_expr, heap_for_parse)
            }) {
            Ok((rpn_expr, local_heap)) => RpnResolver {
                rpn_expr,
                local_heap,
                build_error: None,
                limits,
            },
            Err(err) => RpnResolver {
                rpn_expr: VecDeque::new(),
                local_heap: borrowed_heap,
                build_error: Some(err.to_string()),
                limits,
            },
        }
    }

    /// This method evaluates the rpn expression stack
    ///
    pub fn resolve(&mut self) -> anyhow::Result<Number> {
        if let Some(build_error) = &self.build_error {
            return Err(anyhow!(build_error.clone()));
        }

        let zero: Number = Number::NaturalNumber(Zero::zero());
        let minus_one: Number = Number::NaturalNumber(BigInt::from(-1));
        let limits = self.limits;

        let mut result_stack: VecDeque<Number> = VecDeque::new();
        let mut var_stack: VecDeque<Option<String>> = VecDeque::new();
        let mut last_result: Option<Number> = None;

        for t in &self.rpn_expr {
            match t {
                Token::Operand(n) => {
                    // The budget is documented as bounding every intermediate and
                    // final result, so it has to hold for a value that arrives as a
                    // literal too — otherwise a long enough literal is returned
                    // above the limit the caller asked for.
                    limits::check_size(n, limits)?;
                    result_stack.push_back(n.clone());
                    var_stack.push_back(None);
                }
                Token::Operator(op) => {
                    let right_value: Number = result_stack
                        .pop_back()
                        .ok_or_else(|| anyhow!("{} {}", MALFORMED_ERR, "Invalid Right Operand."))?;

                    var_stack.pop_back();

                    let left_value = if op != &Operator::Une && op != &Operator::Fac {
                        result_stack.pop_back().ok_or_else(|| {
                            anyhow!("{} {}", MALFORMED_ERR, "Invalid Left Operand.")
                        })?
                    } else {
                        zero.clone()
                    };
                    let left_var = if op != &Operator::Une && op != &Operator::Fac {
                        var_stack.pop_back().unwrap_or(None)
                    } else {
                        None
                    };

                    match op {
                        Operator::Add => {
                            let value = left_value + right_value;
                            limits::check_size(&value, limits)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Sub => {
                            let value = left_value - right_value;
                            limits::check_size(&value, limits)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Mul => {
                            let value = left_value * right_value;
                            limits::check_size(&value, limits)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Div => {
                            if right_value == zero {
                                return Err(anyhow!(DIVISION_ZERO_ERR));
                            }
                            let value = left_value / right_value;
                            limits::check_size(&value, limits)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Pow => {
                            result_stack.push_back(Self::power(left_value, right_value, limits)?);
                            var_stack.push_back(None);
                        }
                        Operator::Eql => {
                            if let Some(var) = left_var {
                                if Session::is_constant_name(&var) {
                                    return Err(anyhow!(BUILTIN_CONSTANT_ERR));
                                }
                                self.local_heap
                                    .borrow_mut()
                                    .insert(var.clone(), right_value.clone());

                                result_stack.push_back(right_value);
                                var_stack.push_back(None);
                            } else {
                                return Err(anyhow!(NO_VARIABLE_ERR));
                            }
                        }
                        Operator::Fac => {
                            // Factorial is defined on non-negative integers. It asks the
                            // value, not the enum tag: floor(2.5) and 6/3 are integers.
                            let n = right_value
                                .as_integer()
                                .ok_or_else(|| anyhow!(FACTORIAL_NATURAL_ERR))?;
                            if n < BigInt::zero() {
                                return Err(anyhow!(FACTORIAL_NATURAL_ERR));
                            }
                            let n = n.to_u64().ok_or_else(|| {
                                anyhow!("Runtime Error: Factorial operand is too large")
                            })?;
                            // Predict first, to refuse `999999999!` in
                            // milliseconds rather than computing it...
                            limits::check_predicted_size(
                                limits::predicted_factorial_bits(n),
                                limits,
                            )?;
                            let res = Self::factorial_helper(n.into());
                            // ...then measure what was actually built, because
                            // the prediction is an asymptotic series rounded up
                            // and is a bit short of the truth at `n = 2`. The
                            // prediction buys the speed; this buys the exactness.
                            let value = Number::NaturalNumber(res.into());
                            limits::check_size(&value, limits)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
                        Operator::Une => {
                            //# unary neg
                            result_stack.push_back(right_value * minus_one.clone());
                            var_stack.push_back(None);
                        }
                    }
                }
                Token::Variable(v) => {
                    let var_name = v.to_lowercase();
                    debug!("Heap {:?}", self.local_heap);
                    let heap = self.local_heap.borrow();
                    // An undefined variable reads as zero, deliberately.
                    let n = heap
                        .get(&var_name)
                        .cloned()
                        .unwrap_or_else(|| Number::NaturalNumber(BigInt::zero()));
                    // Same reasoning as the operand arm above: a variable is a
                    // value on the stack like any other. `set`/`setf` put values
                    // into the heap without passing through any checked operator,
                    // so without this an expression that only reads a variable
                    // returns it however large it is.
                    limits::check_size(&n, limits)?;
                    result_stack.push_back(n);
                    var_stack.push_back(Some(var_name));
                }
                Token::Function(fun) => {
                    let value: Number = result_stack.pop_back().ok_or(anyhow!(
                        "{} {}",
                        MALFORMED_ERR,
                        "Wrong use of function"
                    ))?;
                    var_stack.pop_back();

                    let result = functions::eval(*fun, value, &mut result_stack, &mut var_stack)?;
                    // Every arm that pushes a value checks it. A function result
                    // is bounded by construction — the built-ins all route
                    // through `f64` — but "bounded" is not "checked", and the
                    // difference is not academic: an unchecked value goes on to
                    // feed guards that assume their input was checked, which is
                    // how `floor(exp(1))!` slipped a 2-bit result past a 1-bit
                    // budget through the factorial's predictive guard.
                    limits::check_size(&result, limits)?;
                    result_stack.push_back(result);
                    var_stack.push_back(None);
                }
                Token::SemiColon => {
                    // A chained segment just ended. A well-formed segment leaves exactly
                    // one value on the stack; capture it as the running result, then reset
                    // for the next segment. An empty segment (e.g. a leading ';') is a no-op.
                    if !result_stack.is_empty() {
                        if result_stack.len() != 1 {
                            return Err(anyhow!(MALFORMED_ERR));
                        }
                        last_result = result_stack.pop_back();
                    }
                    result_stack.clear();
                    var_stack.clear();
                }
                _ => {
                    return Err(anyhow!(
                        "{} Internal Error at line: {}.",
                        MALFORMED_ERR,
                        line!()
                    ))
                }
            }
        }

        // A trailing ';' leaves the working stack empty: fall back to the last
        // completed segment's value rather than reporting a spurious error.
        if result_stack.is_empty() {
            return last_result.ok_or_else(|| anyhow!(MALFORMED_ERR));
        }

        if result_stack.len() != 1 || var_stack.len() != 1 {
            return Err(anyhow!(MALFORMED_ERR));
        }

        result_stack.pop_back().ok_or(anyhow!("{}", MALFORMED_ERR))
    }

    /// Transforming an infix notation to Reverse Polish Notation (RPN)
    ///
    /// Example
    /// ``
    ///     "3 * 4 + 5 * 6" becomes "3 4 * 5 6 * +"
    /// ``
    fn reverse_polish_notation<'a>(
        infix_stack: &[Spanned<Token<'a>>],
        local_heap: Rc<RefCell<HashMap<String, Number>>>,
    ) -> anyhow::Result<(VecDeque<Token<'a>>, Rc<RefCell<HashMap<String, Number>>>)> {
        /*  Create an empty stack for keeping operators. Create an empty list for output. */
        let mut operators_stack: Vec<Token> = Vec::new();
        let mut postfix_stack: VecDeque<Token> = VecDeque::new();
        let mut seen_variables: Vec<String> = Vec::new();
        let mut bracket_stack: Vec<BracketFrame> = Vec::new();
        let mut pending_function: Option<MathFunction> = None;

        /* Scan the infix expression from left to right. */
        for t in infix_stack {
            if let Some(fun) = pending_function {
                if !matches!(t.node, Token::Bracket(token::Bracket::Open)) {
                    return Err(function_requires_parentheses_err(fun));
                }
            }

            // Both brackets are excluded here: an opening bracket has not yet
            // proven it contributes a value (the group it opens might turn out
            // empty, e.g. the inner `()` in `sin(())`), so marking is deferred
            // to the moment its matching close confirms the group was non-empty.
            // See the `Token::Bracket(Close)` arm below.
            if !matches!(t.node, Token::Comma | Token::Bracket(_)) {
                if let Some(frame) = bracket_stack.last_mut() {
                    frame.has_content = true;
                }
            }

            match t.node {
                /* If the token is an operand, add it to the output list. */
                Token::Operand(_) => postfix_stack.push_back(t.node.clone()),

                /* If the token is a left parenthesis, push it on the stack. */
                Token::Bracket(token::Bracket::Open) => {
                    bracket_stack.push(BracketFrame {
                        function: pending_function.take(),
                        commas: 0,
                        has_content: false,
                    });
                    operators_stack.push(t.node.clone());
                }

                /* If the token is a right parenthesis:
                Pop the stack and add operators to the output list until you encounter a left parenthesis.
                Pop the left parenthesis from the stack but do not add it to the output list.*/
                Token::Bracket(token::Bracket::Close) => {
                    let frame = bracket_stack
                        .pop()
                        .ok_or_else(|| anyhow!(UNBALANCED_BRACKET_ERR))?;
                    // A trailing separator (`max(1,)`) leaves the final slot
                    // empty; catch it here rather than letting it silently
                    // shrink the argument count by one.
                    if frame.commas > 0 && !frame.has_content {
                        return Err(anyhow!(EMPTY_ARGUMENT_ERR));
                    }
                    if let Some(fun) = frame.function {
                        let given = frame.argument_count();
                        let expected = usize::from(fun.arity());
                        if given != expected {
                            return Err(anyhow!(
                                "Parse Error: Function '{}' expects {} argument(s), {} given.",
                                fun.to_string().to_lowercase(),
                                expected,
                                given
                            ));
                        }
                    }
                    // This bracket produced a value: let the enclosing slot (if
                    // any) know, now that the group is confirmed non-empty
                    // rather than merely opened. An empty group like the inner
                    // `()` in `sin(())` must not count as content for the slot
                    // that contains it.
                    if frame.has_content {
                        if let Some(outer) = bracket_stack.last_mut() {
                            outer.has_content = true;
                        }
                    }

                    let mut found_open = false;
                    while let Some(token) = operators_stack.pop() {
                        match token {
                            Token::Bracket(token::Bracket::Open) => {
                                found_open = true;
                                // If the token is a left parenthesis, pop it from the stack
                                if let Some(Token::Function(_)) = operators_stack.last() {
                                    postfix_stack.push_back(
                                        operators_stack.pop().expect("It should not happen."),
                                    );
                                }
                                break;
                            } // discards left parenthesis
                            _ => postfix_stack.push_back(token),
                        }
                    }
                    // `bracket_stack` and `operators_stack` gain and lose open
                    // brackets together: the `Bracket(Open)` arm pushes to both
                    // with nothing between them that can fail, this arm is the
                    // only one that removes an open bracket from
                    // `operators_stack` and it removes exactly one per frame
                    // popped, the `Comma` arm stops at an open bracket without
                    // popping it, the `Operator` arm breaks on anything that is
                    // not an operator or a function, and the `SemiColon` arm
                    // refuses a non-empty `bracket_stack` before it drains.
                    // Popping a frame above therefore guarantees a matching open
                    // bracket is still below us here, so this branch is
                    // unreachable and no test can cover it. It stays anyway,
                    // because the loop above has by then drained the operator
                    // stack into the output in exactly the order the normal
                    // end-of-expression drain uses: the postfix sequence is still
                    // evaluable, so falling through would not raise an error, it
                    // would return a number computed with the bracket grouping
                    // dissolved — `2*(3+4)` as `2 3 * 4 +`, which is 10 rather
                    // than 14. Note also that the last clause above is doing real
                    // work: the `SemiColon` guard is what keeps the two stacks in
                    // step, and relaxing it makes this reachable again.
                    if !found_open {
                        return Err(anyhow!(MALFORMED_ERR));
                    }
                }

                Token::Comma => {
                    let frame = bracket_stack
                        .last_mut()
                        .ok_or_else(|| anyhow!(COMMA_OUTSIDE_CALL_ERR))?;
                    if frame.function.is_none() {
                        return Err(anyhow!(COMMA_OUTSIDE_CALL_ERR));
                    }
                    // The slot that ends here (since the open bracket or the
                    // previous ',') must not be empty, e.g. the first slot in
                    // `max(,1)`.
                    if !frame.has_content {
                        return Err(anyhow!(EMPTY_ARGUMENT_ERR));
                    }
                    frame.commas += 1;
                    frame.has_content = false; // a fresh slot starts now

                    let mut found_open = false;
                    while let Some(token) = operators_stack.last() {
                        if matches!(token, Token::Bracket(token::Bracket::Open)) {
                            found_open = true;
                            break;
                        }
                        postfix_stack
                            .push_back(operators_stack.pop().expect("It should not happen."));
                    }
                    // Same lockstep invariant as the closing-bracket arm above,
                    // reached here through `bracket_stack.last_mut()` having
                    // yielded a frame: an enclosing frame exists, so its open
                    // bracket is still on `operators_stack`. This arm only peeks
                    // at that bracket, so it also leaves the two in step. Also
                    // unreachable, and kept for the same reason: the loop above
                    // has already moved the operators into the output, so falling
                    // through would silently mis-group the arguments rather than
                    // fail, and the invariant it relies on rests on the
                    // `SemiColon` guard staying where it is.
                    if !found_open {
                        return Err(anyhow!(MALFORMED_ERR));
                    }
                }

                Token::SemiColon => {
                    // A bracket left open across a ';' is a parse error in its
                    // own right: without this check the frame just lingers
                    // (out of sync with `operators_stack`, which the loop below
                    // does drain), and the eventual mismatch on some later
                    // close surfaces as a confusing arity error instead of
                    // naming what's actually wrong.
                    if !bracket_stack.is_empty() {
                        return Err(anyhow!(BRACKET_UNCLOSED_AT_SEMICOLON_ERR));
                    }
                    while let Some(token) = operators_stack.pop() {
                        postfix_stack.push_back(token);
                    }
                    postfix_stack.push_back(Token::SemiColon);
                }

                Token::Operator(_op) => {
                    let op1: Token<'_> = t.node.clone();

                    while !operators_stack.is_empty() {
                        let op2: &Token = operators_stack.last().unwrap();
                        match op2 {
                            Token::Operator(_) => {
                                if Token::compare_operator_priority(op1.clone(), op2.clone()) {
                                    postfix_stack.push_back(
                                        operators_stack.pop().expect("It should not happen."),
                                    );
                                } else {
                                    break;
                                }
                            }
                            Token::Function(_) => {
                                postfix_stack.push_back(
                                    operators_stack.pop().expect("It should not happen."),
                                );
                            }
                            _ => break,
                        }
                    }
                    operators_stack.push(op1.clone());
                }

                Token::Function(f) => {
                    pending_function = Some(f);
                    operators_stack.push(t.node.clone());
                }

                /* If the token is a variable, add it to the output list and to the local_heap with a default value*/
                Token::Variable(s) => {
                    postfix_stack.push_back(t.node.clone());
                    seen_variables.push(s.to_lowercase());
                }
            }
            debug!(
                "Inspecting... {} - OUT {} - OP - {}",
                t.node,
                DisplayThisDeque(&postfix_stack),
                DisplayThatVec(&operators_stack)
            );
        }

        if let Some(fun) = pending_function {
            return Err(function_requires_parentheses_err(fun));
        }

        // A frame still on the stack is a bracket that was opened and never
        // closed. That is the same condition class as a stray closing bracket,
        // so it gets the same named diagnosis rather than falling through to the
        // generic malformed-expression message below.
        if !bracket_stack.is_empty() {
            return Err(anyhow!(UNBALANCED_BRACKET_ERR));
        }

        /* After all tokens are read, pop remaining operators from the stack and add them to the list. */
        operators_stack.reverse();
        for t in &operators_stack {
            if matches!(t, Token::Bracket(_)) {
                return Err(anyhow!(MALFORMED_ERR));
            }
            postfix_stack.push_back(t.clone());
        }

        let mut heap = local_heap.borrow_mut();
        for variable in seen_variables {
            heap.entry(variable)
                .or_insert(Number::NaturalNumber(Zero::zero()));
        }
        drop(heap);

        debug!(
            "DEBUG: EOF - OUT {} - OP - {}",
            DisplayThisDeque(&postfix_stack),
            DisplayThatVec(&operators_stack)
        );

        Ok((postfix_stack, local_heap))
    }

    fn factorial_helper(n: BigUint) -> BigUint {
        let mut acc = BigUint::one();
        let mut current = BigUint::one();

        while current <= n {
            acc *= &current;
            current += BigUint::one();
        }

        acc
    }

    fn power(left_value: Number, right_value: Number, limits: Limits) -> anyhow::Result<Number> {
        let value = if let Some(exponent) = right_value.as_integer() {
            Self::power_integer(left_value, exponent, limits)?
        } else {
            let base = number_to_f64(&left_value, POWER_TOO_LARGE_ERR)?;
            let exponent = number_to_f64(&right_value, POWER_TOO_LARGE_ERR)?;
            decimal_from_f64(base.powf(exponent), INVALID_POWER_ERR)?
        };

        // The prediction inside `power_integer` is an optimisation: it buys the
        // right to refuse `10^100000000` without computing it. It is not the
        // guarantee, and on two counts it cannot be. It never runs on the `powf`
        // path at all, and on the integer path it predicts the magnitude of
        // `base^|exponent|` while a negative exponent returns the reciprocal,
        // whose denominator `size_in_bits` also counts — `2^-1` predicts 2 bits
        // and yields `1/2`, which measures 3. Measuring the value we actually
        // built is what makes the budget exact, on every path.
        limits::check_size(&value, limits)?;
        Ok(value)
    }

    fn power_integer(base: Number, exponent: BigInt, limits: Limits) -> anyhow::Result<Number> {
        if exponent.is_zero() {
            return Ok(Number::NaturalNumber(BigInt::one()));
        }

        let is_negative = exponent < BigInt::zero();
        let magnitude = if is_negative { -exponent } else { exponent };
        let exponent = magnitude
            .to_biguint()
            .ok_or_else(|| anyhow!(INVALID_POWER_ERR))?;

        // A degenerate base short-circuits inside the prediction, before the
        // exponent's own magnitude is ever consulted, so `1^n` stays evaluable for
        // an `n` no `u64` could hold. Only a base that actually grows can make the
        // exponent unrepresentable, and that is the one case this message fits.
        let predicted_bits = limits::predicted_power_bits(&base, &exponent)
            .ok_or_else(|| anyhow!(EXPONENT_TOO_LARGE_ERR))?;
        limits::check_predicted_size(predicted_bits, limits)?;

        match base {
            Number::NaturalNumber(base) => {
                if is_negative {
                    if base.is_zero() {
                        return Err(anyhow!(DIVISION_ZERO_ERR));
                    }

                    let value = Self::pow_big_int(base, exponent);
                    Ok(Number::decimal(BigRational::new(BigInt::one(), value)))
                } else {
                    Ok(Number::NaturalNumber(Self::pow_big_int(base, exponent)))
                }
            }
            Number::DecimalNumber(base) => {
                if is_negative && base.is_zero() {
                    return Err(anyhow!(DIVISION_ZERO_ERR));
                }

                let value = Self::pow_big_rational(base, exponent);
                if is_negative {
                    Ok(Number::decimal(value.recip()))
                } else {
                    Ok(Number::decimal(value))
                }
            }
        }
    }

    fn pow_big_int(mut base: BigInt, mut exponent: BigUint) -> BigInt {
        let mut result = BigInt::one();

        while !exponent.is_zero() {
            if exponent.is_odd() {
                result *= &base;
            }
            exponent >>= 1_usize;
            if !exponent.is_zero() {
                base = &base * &base;
            }
        }

        result
    }

    fn pow_big_rational(mut base: BigRational, mut exponent: BigUint) -> BigRational {
        let mut result = BigRational::from_integer(BigInt::one());

        while !exponent.is_zero() {
            if exponent.is_odd() {
                result *= &base;
            }
            exponent >>= 1_usize;
            if !exponent.is_zero() {
                base = &base * &base;
            }
        }

        result
    }
}

struct DisplayThatVec<'a>(&'a Vec<Token<'a>>);
struct DisplayThisDeque<'a>(&'a VecDeque<Token<'a>>);

impl Display for DisplayThatVec<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.iter().map(ToString::to_string).collect::<String>()
        )
    }
}

impl Display for DisplayThisDeque<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.iter().map(ToString::to_string).collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        session::Session,
        span::Span,
        token::{Number, Operator},
    };
    use num_bigint::{BigInt, BigUint};

    #[test]
    fn test_reverse_polish_notation() {
        // The spans on the input tokens are irrelevant to this test — it
        // exercises the shunting-yard logic, not span propagation — so an
        // arbitrary placeholder span is used throughout.
        let no_span = Span::new(0, 0);
        let a: Vec<Spanned<Token>> = vec![
            Spanned::new(
                Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
                no_span,
            ),
            Spanned::new(Token::Operator(Operator::Add), no_span),
            Spanned::new(
                Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
                no_span,
            ),
        ];
        let b: Vec<Token> = vec![
            Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
            Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
            Token::Operator(Operator::Add),
        ];
        assert_eq!(
            RpnResolver::reverse_polish_notation(&a, Rc::new(RefCell::new(HashMap::new())))
                .unwrap()
                .0,
            b
        );
    }

    #[test]
    fn test_factorial() {
        assert_eq!(
            RpnResolver::factorial_helper(BigUint::from(5u8)),
            BigUint::from(120u16)
        );
    }

    #[test]
    fn test_resolve() {
        let mut resolver = RpnResolver {
            rpn_expr: VecDeque::from(vec![
                Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
                Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
                Token::Operator(Operator::Add),
            ]),
            local_heap: Rc::new(RefCell::new(HashMap::new())),
            build_error: None,
            limits: Limits::default(),
        };
        assert_eq!(
            resolver.resolve().unwrap(),
            Number::NaturalNumber(BigInt::from(3u8))
        );
    }

    #[test]
    fn test_invalid_factorial() {
        let session = Session::init();
        let mut resolver = session.process("(-1)!");
        assert!(resolver.resolve().is_err());
        let mut resolver2 = session.process("1.5!");
        assert!(resolver2.resolve().is_err());
    }

    /// `max` and `min` of integers return integers, and the enum tag has to say
    /// so. Asserting the value alone is not enough: cross-variant equality makes
    /// `NaturalNumber(2) == DecimalNumber(2/1)`, so a value-only assertion passes
    /// whichever variant comes back. That is exactly how this test used to read
    /// as "max returns a decimal" and stay green under either behaviour.
    #[test]
    fn test_max_min() {
        let session = Session::init();
        for (expr, expected) in [("max(1,2)", 2), ("min(1,2)", 1), ("min(max(1,2),3)", 2)] {
            let mut resolver = session.process(expr);
            let result = resolver.resolve().unwrap();
            assert_eq!(result, Number::NaturalNumber(BigInt::from(expected)));
            assert!(
                matches!(result, Number::NaturalNumber(_)),
                "{expr} produced {result:?}, expected a NaturalNumber"
            );
        }
    }
}
