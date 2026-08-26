use num::{BigInt, Zero};
use num_rational::BigRational;
use yarer::{
    Error, EvalError, Expression, Limits, MathFunction, Number, ParseError, Session, Span,
};

#[test]
fn test_a_parse_failure_is_reported_by_compile_not_by_eval() {
    assert!(matches!(
        Expression::compile("1+"),
        Err(ParseError::ExpectedValue { .. })
    ));
}

/// Both halves of the span plumbing are tested — the parser records offsets,
/// the shunting yard carries them through the reordering, `EvalError::at`
/// stamps them — and until now the join was not. Change `expression.rs`'s
/// `let at = |e: EvalError| e.at(t.span)` to stamp any other token in scope
/// and the rest of the suite stays green: every number is still right, and
/// only the caret moves. That is the exact failure mode the span tests one
/// layer down were written to prevent, and only an end-to-end assertion sees
/// it.
///
/// Each expectation is read off the source text, not off the output. In
/// `"1/0"` the `/` is byte 1 and ends at 2. In `"ln(0)"` the function name
/// occupies bytes 0..2, and it is the function token that raises the error.
/// In `"(-1)^0.5"` the `^` is byte 4, after `(`, `-`, `1`, `)`.
#[test]
fn test_an_evaluation_error_carries_the_position_of_the_token_that_raised_it() {
    let session = Session::init();
    for (source, start, end) in [("1/0", 1, 2), ("ln(0)", 0, 2), ("(-1)^0.5", 4, 5)] {
        let expr = Expression::compile(source).expect("compiles");
        let err = expr.eval(&session).expect_err("must fail");
        assert_eq!(
            err.span(),
            Some(Span::new(start, end)),
            "for input {source}, error {err:?}"
        );
    }
}

/// The commonest parse error there is, rendered end to end. `1+` produces a
/// zero-width span at the end of the source — there is nothing to underline —
/// and `render`'s `.max(1)` is what still gives it a caret. No other test
/// renders a zero-width span, so nothing else would notice that `.max(1)`
/// going away.
#[test]
fn test_an_error_at_end_of_input_still_gets_a_caret() {
    let err = Expression::compile("1+").expect_err("must fail");
    assert_eq!(err.span(), Some(Span::new(2, 2)));
    assert_eq!(
        Error::Parse(err).render("1+"),
        "Parse error: expected a value, found 'end of expression'\n  1+\n    ^"
    );
}

/// An expression of only separators compiled, then failed at evaluation with
/// the generic positionless message this stage exists to eliminate. It is
/// empty, it is a parse failure, and `compile` is where it is reported.
#[test]
fn test_an_expression_of_only_separators_is_refused_by_compile() {
    for source in [";", ";;", " ; "] {
        assert!(
            matches!(
                Expression::compile(source),
                Err(ParseError::EmptyExpression)
            ),
            "{source} was not refused as empty"
        );
    }
}

#[test]
fn test_a_compiled_expression_survives_a_change_of_variable() {
    let session = Session::init();
    session.set("x", 2).expect("not a constant");
    let expr = Expression::compile("x*3").expect("compiles");
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(6))
    );
    session.set("x", 5).expect("not a constant");
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(15))
    );
}

#[test]
fn test_the_size_limit_reports_which_check_refused_the_value() {
    let session = Session::with_limits(Limits::default().with_max_value_bits(128));
    let expr = Expression::compile("2^10000").expect("compiles");
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::ComputationTooLarge { .. })
    ));
}

macro_rules! resolve {
    ($expr:expr, $expected:expr) => {{
        let session = Session::init();
        let expr = Expression::compile($expr).unwrap();
        assert_eq!(expr.eval(&session).unwrap(), $expected);
    }};
    () => {
        panic!("Expected a valid result, but got an invalid expression.");
    };
}

/// Asserts the numeric value of an expression, within 1e-10.
///
/// It deliberately does not assert which `Number` variant came back: under the
/// canonicalisation invariant an integral result is a `NaturalNumber`, and
/// which side of that line an expression falls on is asserted once, on purpose,
/// by `test_integral_results_are_natural_numbers`.
macro_rules! resolve_decimal {
    ($expr:expr, $expected:expr) => {{
        let session = Session::init();
        let expr = Expression::compile($expr).unwrap();
        let result = expr.eval(&session).unwrap();
        let res_f: f64 = result.try_into().unwrap();
        assert!((res_f - $expected).abs() < 1e-10);
    }};
    () => {
        panic!("Expected a decimal number, but got an invalid result.");
    };
}

macro_rules! resolve_natural {
    ($expr:expr, $expected:expr) => {{
        resolve!($expr, Number::NaturalNumber(BigInt::from($expected)));
    }};
    () => {
        panic!("Expected a natural number, but got an invalid result.");
    };
}

/// Asserts that an expression fails, at whichever of the two steps owns the
/// failure: a malformed expression is refused by `compile`, a bad value by
/// `eval`. Succeeding at both is the only outcome this rejects.
macro_rules! resolve_err {
    ($expr:expr) => {{
        let session = Session::init();
        match Expression::compile($expr) {
            Err(_) => {}
            Ok(expr) => assert!(
                expr.eval(&session).is_err(),
                "{} was expected to fail and did not",
                $expr
            ),
        }
    }};
    () => {
        panic!("Expected an error, but got a valid result.")
    };
}

/// Canonicalisation made `2.0!` return 2 where it used to error, and nothing
/// pinned it.
#[test]
fn test_factorial_accepts_an_integral_decimal_literal() {
    let session = Session::init();
    let expr = Expression::compile("2.0!").expect("compiles");
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(2))
    );
}

/// `1/0` is tested; `1/0.0` — the form that actually used to panic, before
/// canonicalisation turned the literal into a NaturalNumber — was not.
#[test]
fn test_dividing_by_a_decimal_zero_is_an_error_not_a_panic() {
    let session = Session::init();
    let expr = Expression::compile("1/0.0").expect("compiles");
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::DivisionByZero { .. })
    ));
}

/// The doc comment on `checked_div` says the zero test reaches across both
/// variants. This is what makes that a claim the suite can falsify: a
/// `DecimalNumber` holding zero, which no internal path produces — `decimal`
/// normalises it — but which any caller can build, because the variants are
/// public.
#[test]
fn test_checked_div_catches_a_zero_in_either_variant() {
    let one = Number::NaturalNumber(BigInt::from(1));
    let decimal_zero = Number::DecimalNumber(BigRational::new_raw(BigInt::zero(), BigInt::from(3)));
    assert_eq!(one.checked_div(&decimal_zero), None);
}

/// Only the even-exponent degenerate case was pinned, and it cannot tell a
/// correct sign from a discarded one.
#[test]
fn test_a_negative_base_keeps_its_sign_for_an_odd_exponent() {
    let session = Session::init();
    for (source, expected) in [("(-1)^3", -1), ("(-1)^4", 1), ("(-2)^3", -8)] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// The early returns: exponent zero, and the two factorial base cases.
#[test]
fn test_the_degenerate_cases_return_one() {
    let session = Session::init();
    for source in ["5^0", "0!", "1!", "0^0"] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(1)),
            "for {source}"
        );
    }
}

/// The budget is documented as a limit, not a threshold: a value that occupies
/// exactly the budget is admitted. 15 needs 4 bits, 16 needs 5.
#[test]
fn test_a_value_landing_exactly_on_the_budget_is_admitted() {
    let session = Session::with_limits(Limits::default().with_max_value_bits(4));
    assert!(Expression::compile("15").unwrap().eval(&session).is_ok());
    assert!(matches!(
        Expression::compile("16").unwrap().eval(&session),
        Err(EvalError::ValueTooLarge { .. })
    ));
}

#[test]
fn test_expressions() {
    resolve_natural!("(3+4*(2-(3+1)*5+3)-6)*2+4", -122);
    resolve_decimal!("tau", std::f64::consts::TAU);
    resolve_decimal!("phi", (1.0 + 5.0f64.sqrt()) / 2.0);
    resolve_decimal!("gamma", 0.577_215_664_901_532_9_f64);
    resolve_decimal!("3*2^3+6/(2+1)", 26.0);
    resolve_decimal!(
        "pi*4.+2^pi",
        std::f64::consts::PI * 4.0 + 2.0f64.powf(std::f64::consts::PI)
    );
    resolve_natural!("2^3 * 4 + 5^2", 8 * 4 + 25);
    resolve_decimal!("sin(pi/4) + cos(pi/4)", std::f64::consts::SQRT_2);
    resolve_decimal!("tan(pi/4) * cos(pi/6)", 0.8660254037844386); // Approximately sqrt(3)/2
    resolve_decimal!("ln(e) + log10(100)", 3.0);
    //resolve_natural!("3 * 2^3! - 2 * 3 + 6 / (2 + 1)", 188);
    resolve_decimal!("cos(sin(0.5) * pi / 2)", 0.7295860397469262); // Approximately cos(PI/4)
    resolve_decimal!(
        "pi * 2^3 + pi / 2 - e",
        8.0 * std::f64::consts::PI + std::f64::consts::PI / 2.0 - std::f64::consts::E
    );
    resolve_natural!("2 ^ 3 ^ 2", 512);
    resolve_decimal!("ln(e^2) - log10(1000)", -1.0);
    resolve_decimal!(
        "pi^2 - e^2",
        std::f64::consts::PI * std::f64::consts::PI - std::f64::consts::E * std::f64::consts::E
    );
    resolve_natural!("(2 + 3) * (3 + 4) - (4 + 5) * (5 + 6)", -64);
    resolve_decimal!("tan(0) * sin(pi) + cos(pi / 2)", 6.123233995736766e-17);
    resolve_natural!("2^2^2 - 3^3", -11);
    resolve_natural!("(2 + 3 * 4 + 5) * 2", 38);
    resolve!("4! - 3!", Number::NaturalNumber(BigInt::from(18)));
    resolve!("(2^3 + 3^2) * 4", Number::NaturalNumber(BigInt::from(68)));
    resolve_decimal!("e * pi - pi * e", 0.0);
    resolve_natural!("(2 + 3) * (4 - 5) + (6 - 7) * (8 + 9)", -22);
    resolve_decimal!("ln(e^3) / log10(1000)", 1.0);
    resolve_natural!("(2^2 + 3^2) * (4^2 + 5^2)", 533);
    resolve_decimal!(
        "pi*e*(pi-e)",
        std::f64::consts::PI * std::f64::consts::E * (std::f64::consts::PI - std::f64::consts::E)
    );
    resolve_decimal!("((10 + 5) - 3 * ( 9 / 3 )) + 2", 8.0);
    resolve_natural!("2^3^2 - 3^3", 512 - 27);

    resolve_decimal!("min(1,2)", 1.0);
    resolve_decimal!("max(1,2)", 2.0);
    resolve_decimal!("min(max(2,3),max(5,1))", 3.0);

    resolve_decimal!("((2+3)!/5!)*(10-7)", 3.0);
    resolve_decimal!("log(1000)+ln(e^3)", 6.0);
    resolve_decimal!("sqrt(9)+abs(-2)-min(5,3)", 2.0);
    resolve_decimal!("max(1+2,3*4)-min(10,5)", 7.0);
    resolve_decimal!("sin(pi/2)+cos(0)", 2.0);
    resolve_decimal!("tan(pi/4)^2+1", 1.9999999999999998);
    resolve_natural!("(2^3+3^2)^(1+1)", 289);
    resolve_natural!("((3+5)*2)^2", 256);
    resolve_natural!("4^(3-1)+2!", 18);
    resolve_natural!("5!*2^2", 480);
    resolve_decimal!("sin(pi/6)*cos(pi/3)", 0.25);
    resolve_decimal!("abs(-10)+sqrt(16)", 14.0);
    resolve_decimal!("ln(e^(2*2))", 4.0);
    resolve_decimal!("log(100)+log(1000)", 5.0);
    resolve_decimal!("sin(pi)*cos(0)", 1.2246467991473532e-16);
    resolve_decimal!("sqrt(81)+sin(0)-tan(0)", 9.0);
    resolve_decimal!("max(4,2)+min(1,2)*abs(-3)", 7.0);
    resolve_decimal!("abs(-5^2)", 25.0);
    resolve_decimal!("ln(e)+log(10)", 2.0);
    resolve_decimal!("sqrt(2^3*4)", 5.656854249492381);
    resolve_natural!("2^(3! - 5)", 2);
    resolve_natural!("((3+1)!)+(2^3)", 32);
    resolve_decimal!("((4+2)!)/((2+1)!)", 120.0);
    resolve_decimal!("cos(pi/3)^2+sin(pi/3)^2", 1.0);
    resolve_decimal!("atan(1)*4", std::f64::consts::PI);
    resolve_decimal!("acos(0)", std::f64::consts::FRAC_PI_2);
    resolve_decimal!("asin(1)", std::f64::consts::FRAC_PI_2);
    resolve_decimal!("e^(ln(5))", 4.999999999999999);
    resolve_natural!("(2+3)^2*(3!)", 150);
    resolve_decimal!("sqrt(abs(-16))", 4.0);
    resolve_decimal!("max(1+2,2+2)", 4.0);
    resolve_decimal!("min(3!,10)", 6.0);
    resolve_decimal!("max(2^3,3^2)", 9.0);
    resolve_decimal!("min(max(2^3,3^3),max(4^2,2^5))", 27.0);
    resolve_natural!("3!+4!+5!", 150);
    resolve_decimal!("sqrt(3^2+4^2)", 5.0);
    resolve_decimal!("sin(pi/6)+cos(pi/3)", 1.0);
    resolve_decimal!("ln(e^2)+log(100)", 4.0);
    resolve_decimal!("sin(asin(1))", 1.0);
    resolve_decimal!("cos(acos(0))", 6.123233995736766e-17);
    resolve_decimal!("tan(atan(1))", 0.9999999999999999);
    resolve_decimal!("2^-2", 0.25);
    resolve_decimal!("3^-3", 0.037037037037037035);
    resolve_natural!("2^(3^2)", 512);
    resolve_natural!("4!+3!+2!", 32);
    resolve_decimal!("((2^3 + 4^2) / (5 - 3))", 12.0);
    resolve_decimal!("abs(-3)^2+abs(-4)^2", 25.0);
    resolve_decimal!("sqrt(2)^2", 2.0000000000000004);
    resolve_decimal!("sqrt(2)*sqrt(8)", 4.000000000000001);
    resolve_decimal!("ln(e^(ln(e)))", 1.0);

    resolve_decimal!("floor(3.7)", 3.0);
    resolve_decimal!("ceil(3.2)", 4.0);
    resolve_decimal!("round(3.6)", 4.0);
    resolve_decimal!("round(3.4)", 3.0);
    resolve_decimal!("exp(1)", std::f64::consts::E);
    resolve_decimal!("cdf(0)", 0.5);
    resolve_decimal!("pdf(0)", 0.39894228040143265);

    resolve_err!("min()");
    resolve_err!("max()");

    resolve_decimal!("sqrt(16)", 4.0);
    resolve_decimal!("abs(-3)", 3.0);
    resolve_decimal!("asin(1)", std::f64::consts::FRAC_PI_2);
    resolve_decimal!("acos(1)", 0.0);
    resolve_decimal!("atan(1)", std::f64::consts::FRAC_PI_4);
}

#[test]
fn test_programmatic() {
    let session: Session = Session::init();
    let expr = Expression::compile("x ^ 2").unwrap();

    for i in 1..=64 {
        session.set("x", i).expect("not a constant");

        let result: Number = expr.eval(&session).unwrap();

        println!("{}^2={}", i, result);
        assert!(result == Number::NaturalNumber(BigInt::from(i * i)));
    }
}

#[test]
fn test_sharing_session() {
    let session = Session::init();

    let res = Expression::compile("x ^ 2").unwrap();
    let res2 = Expression::compile("x! - (x-1)!").unwrap();

    session.set("x", 10).expect("not a constant");

    if let (Ok(a), Ok(b)) = (res.eval(&session), res2.eval(&session)) {
        assert!(a == Number::NaturalNumber(BigInt::from(100)));

        let b: i64 = b.try_into().unwrap();
        assert!(b == 3265920i64);
    }
}

#[test]
fn test_session_set() {
    let session = Session::init();
    session.set("x", 4).expect("not a constant");
    let expr = Expression::compile("x+2*3/(4-5)").unwrap();
    let result = expr.eval(&session).unwrap();
    assert_eq!(result, Number::NaturalNumber(BigInt::from(-2)));
    // Cross-variant equality would accept DecimalNumber(-2/1) above, so the
    // variant has to be asserted separately for this to notice a regression.
    assert!(
        matches!(result, Number::NaturalNumber(_)),
        "produced {result:?}, expected a NaturalNumber"
    );
}

#[test]
fn test_factorial_invalid_operand() {
    let session = Session::init();
    let expr = Expression::compile("(-1)!").unwrap();
    assert!(expr.eval(&session).is_err());

    let expr = Expression::compile("2.5!").unwrap();
    assert!(expr.eval(&session).is_err());
}

#[test]
fn test_chained_expressions() {
    let session = Session::init();
    let expr = Expression::compile("x=2; y=3; x*y").unwrap();
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(6))
    );
}

#[test]
fn test_chained_without_assignment() {
    let session = Session::init();
    let expr = Expression::compile("1+2; 3+4").unwrap();
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(7))
    );
}

#[test]
fn test_trailing_semicolon_returns_last_value() {
    // A trailing ';' must return the last completed segment's value, not error.
    let session = Session::init();
    let expr = Expression::compile("x=2;").unwrap();
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(2))
    );
}

#[test]
fn test_trailing_semicolon_does_not_error_after_assignment() {
    // The assignment side-effect and the reported result must agree: a successful
    // assignment must not be reported as a malformed-expression error.
    let session = Session::init();
    let expr = Expression::compile("a=5;").unwrap();
    assert!(expr.eval(&session).is_ok());

    let reader = Expression::compile("a").unwrap();
    assert_eq!(
        reader.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(5))
    );
}

#[test]
fn test_malformed_segment_in_chain_is_rejected() {
    // A malformed segment ('1 2' is two adjacent operands) must not be silently
    // discarded by the following ';'.
    assert!(matches!(
        Expression::compile("1 2; 3"),
        Err(ParseError::ExpectedOperator { .. })
    ));
}

/// These four are all parse failures: assert that `Expression::compile`
/// refuses each, not merely that the pipeline fails somewhere. `resolve_err!`
/// accepts failure at either step, which would stay green if one of these
/// started being accepted by `compile` and refused by `eval` instead — a real
/// change in behaviour nothing would report.
#[test]
fn test_invalid_input_is_rejected() {
    for source in ["1@2", "1 2", "(1+2", "1+2)"] {
        assert!(
            Expression::compile(source).is_err(),
            "{source} was expected to fail to compile"
        );
    }
}

#[test]
fn test_unicode_operators_work() {
    resolve_decimal!("2×3 + 8÷4", 8.0);
}

#[test]
fn test_decimal_literals_remain_exact() {
    let session = Session::init();
    let expr = Expression::compile("0.1+0.2").unwrap();
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::DecimalNumber(num_rational::BigRational::new(
            BigInt::from(3),
            BigInt::from(10)
        ))
    );
}

#[test]
fn test_large_integer_division_and_negative_power_do_not_panic() {
    let session = Session::init();
    let expr = Expression::compile("(10^100)/2").unwrap();
    assert_eq!(
        format!("{}", expr.eval(&session).unwrap()),
        format!("5{}", "0".repeat(99))
    );

    let expr = Expression::compile("(10^100)^-1 * 10^100").unwrap();
    let result = expr.eval(&session).unwrap();
    assert_eq!(result, Number::NaturalNumber(BigInt::from(1)));
    // A reciprocal multiplied back out lands exactly on 1, which is integral:
    // the variant is the whole point here, and cross-variant equality would let
    // DecimalNumber(1/1) through the assertion above.
    assert!(
        matches!(result, Number::NaturalNumber(_)),
        "produced {result:?}, expected a NaturalNumber"
    );
}

/// The one wrong *answer* the whole-branch review found. `(10^400)/3` is held
/// exactly and multiplies back out to the right 401-digit integer, but it
/// printed as `inf`, and `1/(10^400)` printed as `0`: `Display`'s
/// `numer/denom` fallback was guarded on `to_f64()` returning `None`, and
/// `BigRational::to_f64` answers `Some(inf)` on overflow and `Some(0.0)` on
/// underflow. Nothing signalled the loss.
#[test]
fn test_a_rational_no_f64_can_hold_prints_as_a_ratio_not_as_infinity() {
    let session = Session::init();

    let expr = Expression::compile("(10^400)/3").expect("compiles");
    let value = expr.eval(&session).expect("evaluates");
    assert_eq!(value.to_string(), format!("1{}/3", "0".repeat(400)));

    // The value was always right; only its rendering was not.
    let exact = Expression::compile("(10^400)/3 * 3").expect("compiles");
    assert_eq!(
        exact.eval(&session).unwrap().to_string(),
        format!("1{}", "0".repeat(400))
    );

    // Underflow is the same defect mirrored.
    let tiny = Expression::compile("1/(10^400)").expect("compiles");
    assert_eq!(
        tiny.eval(&session).unwrap().to_string(),
        format!("1/1{}", "0".repeat(400))
    );

    // And the leak: `TryFrom<Number> for f64` builds its message through
    // `Display`, so it used to report "value 'inf' is out of range" about a
    // value that is neither infinite nor NaN.
    let message = f64::try_from(value).unwrap_err().to_string();
    assert!(!message.contains("inf"), "message was: {message}");
}

#[test]
fn test_builtin_constants_are_read_only() {
    let session = Session::init();
    assert!(matches!(
        session.set("pi", 0),
        Err(EvalError::ReadOnlyConstant { .. })
    ));

    let expr = Expression::compile("pi").unwrap();
    let pi: f64 = expr.eval(&session).unwrap().try_into().unwrap();
    assert!((pi - std::f64::consts::PI).abs() < 1e-10);

    // Assignment now refuses through Session::assign, the one place the refusal
    // is decided; the variant is what says the refusal was that one.
    let expr = Expression::compile("pi=0").unwrap();
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::ReadOnlyConstant { .. })
    ));
}

#[test]
fn test_rounding_functions_on_negative_values() {
    // floor goes toward -inf, ceil toward +inf, round to nearest (half away from zero).
    resolve_decimal!("floor(-3.2)", -4.0);
    resolve_decimal!("ceil(-3.2)", -3.0);
    resolve_decimal!("round(-3.6)", -4.0);
    resolve_decimal!("round(-0.5)", -1.0);
    resolve_decimal!("round(2.5)", 3.0);
    resolve_decimal!("round(1.5)", 2.0);
    resolve_decimal!("floor(5.0)", 5.0);
    resolve_decimal!("ceil(5.0)", 5.0);
}

#[test]
fn test_power_edge_cases() {
    resolve_natural!("0^0", 1);
    resolve_natural!("5^0", 1);
    resolve_natural!("2^10", 1024);
    resolve_natural!("(-2)^3", -8);
    resolve_natural!("(-2)^2", 4);
    resolve_natural!("2^64", 18_446_744_073_709_551_616_i128);
    resolve_decimal!("2^-3", 0.125);
    // unary minus binds tighter than '^', so -2^2 = (-2)^2 = 4 (documented behaviour).
    resolve_natural!("-2^2", 4);
}

#[test]
fn test_factorial_and_abs_edge_cases() {
    resolve_natural!("0!", 1);
    resolve_natural!("1!", 1);
    resolve_natural!("6!", 720);
    resolve_decimal!("abs(-2.5)", 2.5);
    resolve_decimal!("abs(2.5)", 2.5);
    resolve_decimal!("max(-5,-3)", -3.0);
    resolve_decimal!("min(-5,-3)", -5.0);
    resolve_decimal!("exp(0)", 1.0);
}

/// `resolve_err!` accepts failure at either step, which is what `is_err()`
/// meant before `compile` and `eval` were separable. It cannot mean that any
/// more: these five are evaluation failures, and the test would stay green if
/// one of them started being refused at compile time instead — a real change
/// in behaviour that nothing would report.
#[test]
fn test_domain_errors_are_rejected() {
    let session = Session::init();
    for source in ["ln(0)", "1/0", "0^-1", "sqrt(-1)", "asin(2)"] {
        let expr = Expression::compile(source).expect("compiles");
        assert!(expr.eval(&session).is_err(), "{source} was accepted");
    }
}

#[test]
fn test_variable_names_are_case_insensitive() {
    let session = Session::init();
    let expr = Expression::compile("X=7; x").unwrap();
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(7))
    );
}

#[test]
fn test_chained_assignment_sets_all_variables() {
    let session = Session::init();
    let expr = Expression::compile("x=y=5").unwrap();
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(5))
    );

    let reader = Expression::compile("x+y").unwrap();
    assert_eq!(
        reader.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(10))
    );
}

#[test]
fn test_large_result_to_i64_returns_error_not_panic() {
    // 2^200 vastly exceeds i64: the public TryFrom must report an error.
    let session = Session::init();
    let expr = Expression::compile("2^200").unwrap();
    let n = expr.eval(&session).unwrap();
    assert!(i64::try_from(n).is_err());
}

#[test]
fn test_square_and_mixed_brackets() {
    resolve_natural!("[1+2]*3", 9);
    resolve_natural!("[(1+2)*3]", 9);
    resolve_natural!("2*[3+[4-1]]", 12);
}

#[test]
fn test_whitespace_is_ignored() {
    resolve_natural!("  1   +   2  ", 3);
    resolve_natural!("\t3*\t4", 12);
}

#[test]
fn test_functions_are_case_insensitive_end_to_end() {
    resolve_decimal!("COS(0)", 1.0);
    resolve_decimal!("SqRt(16)", 4.0);
    resolve_decimal!("LOG10(1000)", 3.0);
}

#[test]
fn test_large_factorial_is_exact() {
    // 20! = 2_432_902_008_176_640_000 (still fits in i64, but is computed exactly via BigInt)
    resolve_natural!("20!", 2_432_902_008_176_640_000_i64);
}

#[test]
fn test_large_power_is_exact() {
    let session = Session::init();
    let expr = Expression::compile("2^100").unwrap();
    assert_eq!(
        format!("{}", expr.eval(&session).unwrap()),
        "1267650600228229401496703205376"
    );
}

#[test]
fn test_setf_declares_a_decimal_variable() {
    let session = Session::init();
    session.setf("r", 2.5).expect("not a constant");
    let expr = Expression::compile("r*2").unwrap();
    let v: f64 = expr.eval(&session).unwrap().try_into().unwrap();
    assert!((v - 5.0).abs() < 1e-10);
}

#[test]
fn test_factorial_accepts_integral_results_of_functions() {
    // These all produced "Factorial is only defined for non-negative integers"
    // before the Number invariant, because the functions tagged an integral
    // result as decimal and the factorial branched on the tag.
    resolve_natural!("abs(-3)!", 6);
    resolve_natural!("floor(2.5)!", 2);
    resolve_natural!("max(3,2)!", 6);
    resolve_natural!("round(2.4)!", 2);
    resolve_natural!("(6/3)!", 2);
}

#[test]
fn test_integral_results_are_natural_numbers() {
    let session = Session::init();
    // "0.5+0.5", "1.5/0.5" and "(0.5)^-1" are the only inputs that reach
    // `checked_div`'s Decimal/Decimal arm, `apply_functional_token_operation`'s
    // decimal arms, and `power_integer`'s decimal arms respectively; without
    // them the canonicalisation invariant is unchecked on all three.
    for expr in [
        "6/3",
        "floor(3.7)",
        "exp(0)",
        "max(1,2)",
        "sqrt(16)",
        "0.5+0.5",
        "1.5/0.5",
        "(0.5)^-1",
    ] {
        let result = Expression::compile(expr).unwrap().eval(&session).unwrap();
        assert!(
            matches!(result, Number::NaturalNumber(_)),
            "{expr} produced {result:?}, expected a NaturalNumber"
        );
    }
}

#[test]
fn test_non_integral_results_stay_decimal() {
    let session = Session::init();
    for expr in ["1/3", "abs(-2.5)", "2^-3", "sqrt(2)"] {
        let result = Expression::compile(expr).unwrap().eval(&session).unwrap();
        assert!(
            matches!(result, Number::DecimalNumber(_)),
            "{expr} produced {result:?}, expected a DecimalNumber"
        );
    }
}

#[test]
fn test_oversized_factorial_is_refused_not_computed() {
    // Before the limit this did not return at all. The test is its own alarm:
    // if the guard stops working, the suite hangs here instead of failing.
    let session = Session::init();
    let expr = Expression::compile("999999999!").unwrap();
    // "size limit" was the wording the two size errors share, so the assertion
    // stays as wide as it was: refused by the budget, measured or predicted.
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::ValueTooLarge { .. } | EvalError::ComputationTooLarge { .. })
    ));
}

#[test]
fn test_oversized_power_is_refused_not_computed() {
    let session = Session::init();
    let expr = Expression::compile("10^100000000").unwrap();
    // As above: either size error satisfies what "size limit" used to say.
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::ValueTooLarge { .. } | EvalError::ComputationTooLarge { .. })
    ));
}

#[test]
fn test_legitimate_big_values_still_pass_the_default_limit() {
    resolve_natural!("2^64", 18_446_744_073_709_551_616_i128);
    let session = Session::init();
    let expr = Expression::compile("1000!").unwrap();
    // 1000! needs about 8530 bits, comfortably inside the default budget.
    assert!(expr.eval(&session).is_ok());
}

#[test]
fn test_the_limit_is_configurable() {
    let session = Session::with_limits(Limits::default().with_max_value_bits(64));
    let expr = Expression::compile("2^100").unwrap();
    assert!(
        expr.eval(&session).is_err(),
        "2^100 needs 101 bits, over a 64-bit budget"
    );

    let small = Expression::compile("2^10").unwrap();
    assert!(small.eval(&session).is_ok());
}

#[test]
fn test_growth_through_multiplication_is_caught() {
    // Budget 4000 is exactly the power's own prediction for this base and
    // exponent (size_in_bits(2) * 2000 = 2 * 2000 = 4000): the largest value the
    // predictive power check admits for "2^2000", so a failure here can only come
    // from the multiplication, not from the power check firing again. 2^2000 is
    // actually 2001 bits (passes both checks); squaring it needs 4001 bits, over
    // budget, and only the post-hoc Mul check can catch that.
    let session = Session::with_limits(Limits::default().with_max_value_bits(4000));
    let expr = Expression::compile("x=2^2000; x*x").unwrap();
    // ValueTooLarge is the post-hoc measurement; a prediction reports
    // ComputationTooLarge. Asserting which one is what makes the paragraph above
    // a claim the test checks rather than a comment asking to be believed.
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::ValueTooLarge { .. })
    ));
}

#[test]
fn test_oversized_exponent_reports_its_own_message() {
    // The exponent itself doesn't fit in a u64 (it's far larger than u64::MAX),
    // so this must be refused before any size prediction is even attempted - and
    // as its own condition, not the unrelated InvalidPower that covers a
    // different failure (a non-integer powf conversion). One variant is not the
    // other, so the assertion below carries that too.
    let session = Session::init();
    let expr = Expression::compile("2^99999999999999999999").unwrap();
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::ExponentTooLarge { .. })
    ));
}

/// `OperandTooLargeForFloat` and `PowerOperandsTooLarge` used to be
/// unreachable. Both were produced only through `number_to_f64`'s `ok_or`,
/// and `to_f64` answers `Some(±inf)` on overflow rather than `None`, so the
/// `ok_or` never fired. The infinity flowed on and was caught downstream by
/// `decimal_from_f64`'s finiteness test, under a different name:
/// `sqrt(2^5000)` said "function result is not a real number" about a number
/// that is perfectly real, and `(2^2000)^0.5` said "invalid power operation".
/// What failed in both is that the operand does not fit in an f64 — which is
/// what these two variants say, and neither could say it.
#[test]
fn test_an_operand_too_large_for_an_f64_reports_that_and_not_something_else() {
    let session = Session::init();

    let expr = Expression::compile("sqrt(2^5000)").expect("compiles");
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::OperandTooLargeForFloat { .. })
    ));

    let expr = Expression::compile("(2^2000)^0.5").expect("compiles");
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::PowerOperandsTooLarge { .. })
    ));

    // The two conditions those were misreported as are still reachable in
    // their own right, from operands an f64 holds perfectly well — so the fix
    // narrowed the diagnosis rather than moving it wholesale.
    let expr = Expression::compile("ln(0)").expect("compiles");
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::NotARealNumber { .. })
    ));

    let expr = Expression::compile("(-1)^0.5").expect("compiles");
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::InvalidPower { .. })
    ));
}

/// The finiteness filter above does not only intercept the unreal answers it
/// was aimed at. `atan(10^400)` narrows to `f64::INFINITY`, and unlike
/// `sqrt(inf)` (not finite, so it fell to `NotARealNumber` even before the
/// filter existed), `atan(inf)` is a perfectly real, finite limit: `pi/2`.
/// 0.2.0 computed exactly that. The filter cannot tell "the operand
/// overflowed and the answer is unreal" from "the operand overflowed and the
/// function has a finite limit anyway" — both narrow to an infinite `f64`, so
/// both are now refused. This crate accepts giving up the second case's
/// correct answer, because silently narrowing the operand — the failure mode
/// the filter exists to close — was judged the larger problem. See the
/// migration table in README.md.
#[test]
fn test_an_operand_too_large_for_an_f64_is_refused_even_with_a_finite_limit_there() {
    let session = Session::init();

    let expr = Expression::compile("atan(10^400)").expect("compiles");
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::OperandTooLargeForFloat { .. })
    ));
}

#[test]
fn test_degenerate_power_bases_are_not_refused() {
    // 1^n, 0^n and (-1)^n all stay tiny no matter how large n is, and are cheap to
    // compute (repeated squaring on a magnitude-1 base never grows). A size
    // prediction that multiplies base bits by the exponent must not refuse these.
    resolve_natural!("1^10000000", 1);
    resolve_natural!("0^10000000", 0);
    resolve_natural!("(-1)^10000000", 1); // even exponent

    // The same argument holds for an exponent too large to fit in a u64. Testing
    // the base's magnitude only *after* that conversion refused this one with
    // "the exponent is too large to evaluate under any size limit", which is
    // factually wrong for a base of magnitude 1: 1^n is 1 under every limit.
    resolve_natural!("1^99999999999999999999", 1);
    resolve_natural!("0^99999999999999999999", 0);
}

#[test]
fn test_an_oversized_literal_is_refused() {
    // max_value_bits is documented as bounding the size of any intermediate or
    // final result. A literal pushed straight onto the stack is one of those, so
    // the budget has to apply to it as well, whatever produced it.
    let session = Session::with_limits(Limits::default().with_max_value_bits(32));
    let expr = Expression::compile("99999999999999999999").unwrap();
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::ValueTooLarge { .. })
    ));

    // A literal inside the budget is untouched, so the guard is not simply
    // refusing everything.
    let inside = Expression::compile("123+1").unwrap();
    assert_eq!(
        inside.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(124))
    );
}

#[test]
fn test_an_oversized_variable_is_refused() {
    // Same hole as the literal, one match arm over. Bounding untrusted input is
    // the whole reason to call with_limits, and setf is a way in that does not
    // pass through any of the checked operators: 1e308 is stored as a ~1024-bit
    // integer, and reading it back needs no arithmetic at all.
    let session = Session::with_limits(Limits::default().with_max_value_bits(64));
    session.setf("x", 1e308).expect("not a constant");

    // Bare "x" is the case no other guard can catch: the value is pushed and
    // returned without a single operator running over it.
    let expr = Expression::compile("x").unwrap();
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::ValueTooLarge { .. })
    ));

    // A variable inside the budget still reads back normally.
    session.set("y", 7).expect("not a constant");
    let inside = Expression::compile("y*2").unwrap();
    assert_eq!(
        inside.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(14))
    );

    // An undefined variable still resolves to zero. That is deliberate, and a
    // size check on the variable push must not turn it into an error.
    let undefined = Expression::compile("z+1").unwrap();
    assert_eq!(
        undefined.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(1))
    );
}

#[test]
fn test_a_materialised_power_is_measured_not_just_predicted() {
    // The size prediction is a pre-filter, not the guarantee, and there are two
    // ways past it. Neither of these is caught by any predictive check.

    // 1. The powf path never consults the prediction at all: "2^0.5" is a
    //    non-integer exponent, so it converts to f64 and comes back as a
    //    rational of roughly 53 + 53 bits.
    let tight = Session::with_limits(Limits::default().with_max_value_bits(16));
    let irrational = Expression::compile("2^0.5").unwrap();
    assert!(matches!(
        irrational.eval(&tight),
        Err(EvalError::ValueTooLarge { .. })
    ));

    // 2. A negative exponent is predicted on the magnitude of base^|exponent|,
    //    but the value returned is the reciprocal, whose denominator counts too.
    //    "2^-1" predicts 2 bits and yields 1/2, which measures 1 + 2 = 3.
    let two_bits = Session::with_limits(Limits::default().with_max_value_bits(2));
    let reciprocal = Expression::compile("2^-1").unwrap();
    assert!(matches!(
        reciprocal.eval(&two_bits),
        Err(EvalError::ValueTooLarge { .. })
    ));

    // Three bits is exactly enough, which pins the boundary rather than just
    // asserting that something was refused.
    let three_bits = Session::with_limits(Limits::default().with_max_value_bits(3));
    let fits = Expression::compile("2^-1").unwrap();
    assert_eq!(fits.eval(&three_bits).unwrap().to_string(), "0.5");
}

#[test]
fn test_a_function_result_is_measured_like_any_other_value() {
    // A function result is bounded by construction, since every built-in routes
    // its argument through f64 -- but bounded is not measured, and the gap is not
    // academic. While this arm was unchecked it was the one way to get a value
    // onto the stack that nothing had measured, and the guards downstream assume
    // their inputs were measured. floor(exp(1))! returned 2 under a 1-bit budget
    // for exactly that reason: the factorial's predictive guard is a bit short at
    // n = 2, and 2 is a 2-bit operand that no checked arm would have admitted.
    let one_bit = Session::with_limits(Limits::default().with_max_value_bits(1));
    for expr in ["exp(1)", "floor(exp(1))", "floor(exp(1))!"] {
        let compiled = Expression::compile(expr).unwrap();
        let err = compiled.eval(&one_bit).unwrap_err();
        assert!(
            matches!(err, EvalError::ValueTooLarge { .. }),
            "{expr} reported: {err:?}"
        );
    }

    // Nothing legitimate is refused by this: a function result is f64-bounded, so
    // it is far below any budget anyone would set on purpose.
    resolve_natural!("1/cos(0)", 1);
    resolve_decimal!("sin(1)", 0.841_470_984_807_896_5);
    resolve_decimal!("9801/(2206*sqrt(2))", 3.141_592_730_013_305_5);

    // NOTE for whoever reads this next: closing this arm makes the factorial's
    // own post-hoc check unreachable, since every route to an operand now
    // measures it first. That check stays anyway -- n = 2 is the only value up to
    // 60000 where the prediction falls short, which is an empirical bound and not
    // a proof -- but it is now shadowed, and a test claiming to exercise it would
    // really be exercising this arm. That is why this test asserts the function
    // arm and says so, rather than keeping the old factorial framing green.

    // The predictive refusal must survive: 999999999! has to stay a fast "no",
    // not become a computation that is measured afterwards.
    let default = Session::init();
    let huge = Expression::compile("999999999!").unwrap();
    assert!(matches!(
        huge.eval(&default),
        Err(EvalError::ComputationTooLarge { .. })
    ));
}

#[test]
fn test_a_tiny_budget_rejects_the_builtin_constants() {
    // The constants are f64s held exactly as rationals, so they are wide:
    // numerator bits plus denominator bits, tau is the narrowest at 98 and gamma
    // the widest at 107 (pi is 884279719003555/281474976710656, 50 + 49 = 99).
    // A variable is size-checked as it is pushed, so a small budget rejects a
    // value the caller never supplied. This test exists so the 107-bit floor
    // quoted by Session::with_limits stays a measured number, not folklore.
    let below_all = Session::with_limits(Limits::default().with_max_value_bits(97));
    for name in ["pi", "e", "tau", "phi", "gamma"] {
        let err = Expression::compile(name)
            .unwrap()
            .eval(&below_all)
            .unwrap_err();
        assert!(
            matches!(
                err,
                EvalError::ValueTooLarge { .. } | EvalError::ComputationTooLarge { .. }
            ),
            "{name} reported: {err:?}"
        );
    }

    // 107 is the exact floor, pinned from both sides: one bit under it the
    // widest constant still does not fit, at it every constant does. Asserting
    // only the lower bound would pass for any number that happens to be too
    // small, which is how a figure like this drifts out of date unnoticed.
    let one_short = Session::with_limits(Limits::default().with_max_value_bits(106));
    let widest = Expression::compile("gamma").unwrap();
    assert!(widest.eval(&one_short).is_err(), "gamma fits in 106 bits?");

    let exact = Session::with_limits(Limits::default().with_max_value_bits(107));
    for name in ["pi", "e", "tau", "phi", "gamma"] {
        let compiled = Expression::compile(name).unwrap();
        assert!(
            compiled.eval(&exact).is_ok(),
            "{name} was rejected at 107 bits"
        );
    }
}

/// Binds `function` and checks it, unlike the substring assertion this test
/// used to make: `max(1)` and `sin(1,2)` differ only in which function is
/// named, and `expected`/`given` alone can't tell a report that named the
/// wrong function from one that got it right.
#[test]
fn test_wrong_arity_is_diagnosed_by_name() {
    for (expr, fun, expected, given) in [
        ("max(1)", MathFunction::Max, 2, 1),
        ("max(1,2,3)", MathFunction::Max, 2, 3),
        ("sin(1,2)", MathFunction::Sin, 1, 2),
    ] {
        let err = Expression::compile(expr).unwrap_err();
        assert!(
            matches!(err, ParseError::WrongArity { function, expected: e, given: g, .. }
                     if function == fun && e == expected && g == given),
            "{expr} reported: {err:?}"
        );
    }
}

#[test]
fn test_empty_argument_list_is_diagnosed() {
    let err = Expression::compile("max()").unwrap_err();
    assert!(
        matches!(err, ParseError::WrongArity { given: 0, .. }),
        "reported: {err:?}"
    );
}

#[test]
fn test_comma_outside_a_function_call_is_diagnosed() {
    let err = Expression::compile("1,2").unwrap_err();
    assert!(
        matches!(err, ParseError::CommaOutsideCall { .. }),
        "reported: {err:?}"
    );
}

#[test]
fn test_a_function_name_requires_parentheses() {
    // "sin;" pins down that a pending function cannot survive a ';' statement
    // boundary either: the mandatory-parenthesis check fires on the very next
    // token, whatever it is, before the ';' arm ever runs.
    for expr in ["sin 5", "sqrt 16", "cos", "sin;"] {
        let err = Expression::compile(expr).unwrap_err();
        assert!(
            matches!(err, ParseError::FunctionRequiresParentheses { .. }),
            "{expr} reported: {err:?}"
        );
    }
    // The parenthesised form is untouched.
    resolve_decimal!("sin(5)", -0.9589242746631385);
}

#[test]
fn test_nested_and_multi_argument_calls_still_work() {
    resolve_natural!("min(max(2,3),max(5,1))", 3);
    resolve_natural!("max(1+2,3*4)-min(10,5)", 7);
    resolve_natural!("max(1,(2+3))", 5);
}

#[test]
fn test_unclosed_bracket_before_semicolon_is_diagnosed() {
    // Before the fix, the open bracket's frame survived the ';' unclosed, and
    // the mismatch surfaced later as a misleading arity error instead of
    // naming the real problem.
    // The variant also carries what the old message assertion said it was not:
    // a WrongArity is not a BracketUnclosedAtSemicolon.
    let err = Expression::compile("max(1; 2)").unwrap_err();
    assert!(
        matches!(err, ParseError::BracketUnclosedAtSemicolon { .. }),
        "reported: {err:?}"
    );
}

#[test]
fn test_empty_argument_slot_is_diagnosed() {
    // A comma with nothing before or after it must be its own diagnosis, not
    // silently absorbed into the argument count.
    for expr in ["max(,1)", "max(1,)"] {
        let err = Expression::compile(expr).unwrap_err();
        assert!(
            matches!(err, ParseError::EmptyArgument { .. }),
            "{expr} reported: {err:?}"
        );
    }
}

#[test]
fn test_nested_empty_group_does_not_fake_an_argument() {
    // The inner "()" is empty and must not be counted as content for the
    // outer call's only argument slot.
    let err = Expression::compile("sin(())").unwrap_err();
    assert!(
        matches!(err, ParseError::EmptyGroup { .. }),
        "reported: {err:?}"
    );
}

#[test]
fn test_unbalanced_closing_bracket_is_diagnosed() {
    let err = Expression::compile("1+2)").unwrap_err();
    assert!(
        matches!(err, ParseError::UnbalancedBracket { .. }),
        "reported: {err:?}"
    );
}

#[test]
fn test_unbalanced_opening_bracket_is_diagnosed() {
    // The same condition class as the test above, and it used to get the generic
    // "malformed expression" message instead of the named one. "max(1,2" is the
    // sharper case: the bracket never closes, so the arity check on the closing
    // bracket never ran at all.
    // The variant carries the negative claim too: UnbalancedBracket is not
    // ParseError::Malformed.
    for expr in ["(1+2", "max(1,2"] {
        let err = Expression::compile(expr).unwrap_err();
        assert!(
            matches!(err, ParseError::UnbalancedBracket { .. }),
            "{expr} reported: {err:?}"
        );
    }
}

#[test]
fn test_single_argument_function_called_empty_is_diagnosed() {
    let err = Expression::compile("sin()").unwrap_err();
    assert!(
        matches!(
            err,
            ParseError::WrongArity {
                expected: 1,
                given: 0,
                ..
            }
        ),
        "reported: {err:?}"
    );
}

#[test]
fn test_comma_inside_nested_plain_group_within_a_call_is_diagnosed() {
    let err = Expression::compile("max((1,2),3)").unwrap_err();
    assert!(
        matches!(err, ParseError::CommaInPlainBracket { .. }),
        "reported: {err:?}"
    );
}

#[test]
fn test_setting_a_built_in_constant_is_refused_out_loud() {
    let session = Session::init();
    assert!(matches!(
        session.set("pi", 3),
        Err(EvalError::ReadOnlyConstant { .. })
    ));
    // And the refusal is real: pi is still pi.
    let expr = Expression::compile("pi").expect("compiles");
    assert!(matches!(
        expr.eval(&session).unwrap(),
        Number::DecimalNumber(_)
    ));
}

#[test]
fn test_setting_a_variable_to_a_non_number_is_refused_out_loud() {
    let session = Session::init();
    // All three of the values `f64::is_finite` refuses, in one array, so that
    // adding the next one is one word rather than another copied block —
    // NEG_INFINITY was missing precisely because it needed a third block.
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            matches!(session.setf("x", value), Err(EvalError::NotFinite { .. })),
            "setf accepted {value}"
        );
    }
}

#[test]
fn test_one_expression_can_be_evaluated_under_two_budgets() {
    let session = Session::init();
    let expr = Expression::compile("2^64").expect("compiles");
    assert!(expr.eval(&session).is_ok());
    assert!(matches!(
        expr.eval_with(&session, Limits::default().with_max_value_bits(8)),
        Err(EvalError::ComputationTooLarge { .. })
    ));
    // The session's own budget is untouched by the tighter evaluation.
    assert!(expr.eval(&session).is_ok());
}

/// Not an assertion: a measurement, for the decision recorded in component H
/// of the spec. Run before and after adding `.reduced()` to `Number::decimal`.
///
/// Run with: cargo test --release -- --ignored --nocapture
#[test]
#[ignore = "timing"]
fn measure_the_cost_of_reducing_every_decimal() {
    let session = Session::init();
    for (name, source, rounds) in [
        ("small rationals", "1/3 + 1/7 + 1/11", 20_000),
        ("one large rational", "(2^60000)/3", 200),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        let start = std::time::Instant::now();
        for _ in 0..rounds {
            expr.eval(&session).expect("evaluates");
        }
        println!("{name}: {rounds} evaluations in {:?}", start.elapsed());
    }
}

#[test]
fn test_an_unreduced_rational_does_not_become_a_decimal() {
    // Ratio::new_raw skips reduction, so 4/2 arrives integral but unreduced.
    // Number::decimal is the constructor that upholds the invariant, and it
    // has to reduce to see that this value is a whole number.
    let unreduced = num_rational::BigRational::new_raw(BigInt::from(4), BigInt::from(2));
    assert!(matches!(
        Number::decimal(unreduced),
        Number::NaturalNumber(_)
    ));
}

// ---------------------------------------------------------------------------
// Comparison and logical operators
// ---------------------------------------------------------------------------

/// A comparison yields 1 or 0, as in GNU bc. There is no boolean type: the
/// price of that is that `(1<2) + 5` is a legal expression worth 6, and the
/// benefit is that `Number` stays the crate's only kind of value.
#[test]
fn test_comparisons_yield_one_or_zero() {
    let session = Session::init();
    for (source, expected) in [
        ("1 < 2", 1),
        ("2 < 1", 0),
        ("2 < 2", 0),
        ("2 > 1", 1),
        ("1 > 2", 0),
        ("2 <= 2", 1),
        ("3 <= 2", 0),
        ("2 >= 2", 1),
        ("1 >= 2", 0),
        ("2 == 2", 1),
        ("2 == 3", 0),
        ("2 <> 3", 1),
        ("2 <> 2", 0),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// Comparison asks the mathematical value, not the enum tag — the property
/// Stage 1 established when it made `PartialEq` and `PartialOrd` agree. `6/3`
/// is a `NaturalNumber` and `0.5` a `DecimalNumber`, and neither fact is
/// visible to `<`.
#[test]
fn test_comparison_crosses_the_number_variants() {
    let session = Session::init();
    for source in ["2 == 6/3", "0.5 < 2/3", "1.0 >= 1", "1/2 == 0.5"] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(1)),
            "for {source}"
        );
    }
}

/// A two-character operator occupies two bytes, and its span must say so.
/// Nothing else in the suite would notice if it did not: the message stays
/// right while the caret moves one column left.
#[test]
fn test_a_two_character_operator_spans_two_bytes() {
    let err = Expression::compile("1 <= ").unwrap_err();
    assert_eq!(err.span().map(|s| (s.start, s.end)), Some((5, 5)));

    let err = Expression::compile("1 <= <= 2").unwrap_err();
    assert!(
        matches!(err, ParseError::ExpectedValue { ref found, span }
            if found == "<=" && (span.start, span.end) == (5, 7)),
        "got {err:?}"
    );
}

/// The ordered alternation in the regex, pinned from the outside. Written the
/// other way round, `<=` tokenises as `<` followed by `=` — a comparison
/// followed by an assignment, which then fails somewhere else with a message
/// about the wrong thing. `1 <= 2` would become `1 < (= 2)` and be refused for
/// a reason that has nothing to do with what the user typed.
#[test]
fn test_a_two_character_operator_is_not_read_as_two_one_character_ones() {
    let session = Session::init();
    for (source, expected) in [("1 <= 2", 1), ("1 >= 2", 0), ("1 == 1", 1), ("1 <> 1", 0)] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// `=` is still assignment and nothing about it moved.
#[test]
fn test_assignment_is_untouched_by_the_comparison_operators() {
    let session = Session::init();
    let expr = Expression::compile("x = 5").expect("compiles");
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(5))
    );
    let expr = Expression::compile("x == 5").expect("compiles");
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::from(1))
    );
    let expr = Expression::compile("x == 6").expect("compiles");
    assert_eq!(
        expr.eval(&session).unwrap(),
        Number::NaturalNumber(BigInt::zero())
    );
}

/// Zero is false and everything else is true — negatives and fractions
/// included, which is why the rule asks the value and not the variant.
#[test]
fn test_the_logical_operators_read_any_non_zero_value_as_true() {
    let session = Session::init();
    for (source, expected) in [
        ("1 and 1", 1),
        ("1 and 0", 0),
        ("0 and 1", 0),
        ("0 and 0", 0),
        ("1 or 0", 1),
        ("0 or 1", 1),
        ("0 or 0", 0),
        ("1 xor 0", 1),
        ("0 xor 1", 1),
        ("1 xor 1", 0),
        ("0 xor 0", 0),
        // Truth is not confined to 1: a fraction, a negative and a value
        // arriving as a decimal are all true, and only zero is false.
        ("0.5 or 0", 1),
        ("(0-1) and 1", 1),
        ("1/3 and 2", 1),
        ("0.0 or 0", 0),
        ("(1-1) or 0", 0),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// Case-insensitive, like every other word in the language.
#[test]
fn test_the_word_operators_are_case_insensitive() {
    let session = Session::init();
    for source in ["1 and 1", "1 AND 1", "1 And 1", "1 aNd 1"] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(1)),
            "for {source}"
        );
    }
}

/// The one break this work introduces: five words stop being variable names,
/// in every casing. Refused as a misplaced operator, which is what they now
/// are, rather than read as a variable that silently evaluates to zero.
#[test]
fn test_the_new_words_are_reserved() {
    for source in ["and = 5", "or + 1", "xor", "MOD = 1", "1 + and", "not = 2"] {
        assert!(
            matches!(
                Expression::compile(source),
                Err(ParseError::ExpectedValue { .. })
            ),
            "{source} should be refused as a misplaced operator, got {:?}",
            Expression::compile(source)
        );
    }
}

#[test]
fn test_not_negates_truth() {
    let session = Session::init();
    for (source, expected) in [
        ("not 0", 1),
        ("not 1", 0),
        ("not 5", 0),
        ("not 0.5", 0),
        ("not not 1", 1),
        ("not (1 < 2)", 0),
        ("not (0-1)", 0),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// `not` is prefix. In operator position it is not a binary operator, and the
/// diagnosis must say so where it happens rather than surfacing later as a
/// complaint about a stack — which is the exact defect `max(1,*2)` had before
/// the previous stage.
///
/// The second half is what makes this a test of the validator's explicit `not`
/// arm rather than of anything else. On its own the first half proves nothing:
/// before `not` was a word operator it was a *variable*, and a variable in
/// operator position is refused by the same rule with the same message and the
/// same span. Only the contrast separates them — `and` reaches the catch-all
/// that accepts any operator as binary and is fine here, `not` must not.
#[test]
fn test_not_in_operator_position_is_diagnosed_where_it_occurs() {
    let err = Expression::compile("1 not 2").unwrap_err();
    assert!(
        matches!(err, ParseError::ExpectedOperator { ref found, span }
            if found == "not" && (span.start, span.end) == (2, 5)),
        "got {err:?}"
    );

    for binary in ["1 and 2", "1 or 2", "1 xor 2", "1 mod 2"] {
        assert!(
            Expression::compile(binary).is_ok(),
            "{binary} is a binary operator and belongs in operator position"
        );
    }
}

/// A prefix operator appears where a value appears, so every operator already
/// waiting on the shunting yard's stack is still short of its own right
/// operand and none of them may be displaced — whatever the precedence
/// arithmetic says.
///
/// `Une` never had to state this: at the second-strongest level it is stronger
/// than anything it could displace. `not` is the weakest of the three unary
/// operators, and without the rule `1 - not 0` pops the `-` before its right
/// operand exists, producing the RPN `1 - 0 not` — which fails in the
/// evaluator with a positionless "malformed expression" for an input that is
/// nothing of the kind.
#[test]
fn test_a_prefix_operator_never_displaces_one_that_is_still_waiting() {
    let session = Session::init();
    for (source, expected) in [
        ("1 - not 0", 0),
        ("1 + not 0", 2),
        ("2 * not 0", 2),
        ("2 ^ not 3", 1),
        ("0 - not 1", 0),
        // `not` is weaker than `+`, so it takes the whole sum to its right.
        ("not 0 + 1", 0),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// Truncating toward zero, as in C, Rust, bc and BASIC — every language whose
/// spelling this borrows. The result takes the sign of the dividend. All four
/// sign combinations, because getting one right by accident is easy and
/// getting all four right by accident is not.
#[test]
fn test_mod_truncates_toward_zero() {
    let session = Session::init();
    for (source, expected) in [
        ("7 mod 3", 1),
        ("-7 mod 3", -1),
        ("7 mod -3", 1),
        ("-7 mod -3", -1),
        ("6 mod 3", 0),
        ("2 mod 5", 2),
        ("-2 mod 5", -2),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// Defined on rationals, not only integers, because the formula is the same
/// one. The brackets are not decoration: `1/2 mod 1/3` shares a precedence
/// level with `/` and would group as `((1/2) mod 1) / 3` — which happens to
/// give the same 1/6 and would make this test pass under a reading it does not
/// mean to assert.
#[test]
fn test_mod_works_on_rationals() {
    resolve_decimal!("7.5 mod 2", 1.5);
    resolve_decimal!("(1/2) mod (1/3)", 1.0 / 6.0);
    resolve_decimal!("(-7.5) mod 2", -1.5);
}

/// Its zero check is the crate's one zero check, so it reports the same error
/// division does.
#[test]
fn test_mod_by_zero_is_the_same_error_as_dividing_by_zero() {
    let session = Session::init();
    for source in ["7 mod 0", "7 mod 0.0", "7.5 mod (1-1)"] {
        let expr = Expression::compile(source).expect("compiles");
        assert!(
            matches!(expr.eval(&session), Err(EvalError::DivisionByZero { .. })),
            "for {source}"
        );
    }
}

/// Every boundary in the precedence ladder, each pinned by an input that gives
/// a *different* answer under the wrong grouping.
///
/// The third and fourth columns are what make this a test of precedence rather
/// than of arithmetic: they spell the wrong grouping out in brackets and assert
/// what it would produce. A row whose two readings agree proves nothing about
/// where the boundary is, and the `assert_ne!` refuses to let one be added.
///
/// Two rows took finding. With 0 and 1 the two readings of `not` usually agree
/// — `not 0 == 0`, `not 1 == 0` and `not 2 == 2` are the same either way — so
/// it needs an input where `not x` is 0 and x differs from the comparand. And
/// equality against relational needs `0 == 0 < 0`: the obvious `1 < 2 == 1`
/// groups identically whether the six comparisons share a level or equality
/// sits on a weaker one of its own, as it does in C.
#[test]
fn test_the_precedence_boundaries_hold() {
    let session = Session::init();
    let value = |source: &str| {
        Expression::compile(source)
            .unwrap_or_else(|e| panic!("{source} should compile: {e}"))
            .eval(&session)
            .unwrap_or_else(|e| panic!("{source} should evaluate: {e}"))
    };

    for (source, expected, misgrouped, wrong) in [
        ("1 or 0 and 0", 1, "(1 or 0) and 0", 0),
        ("1 or 1 xor 1", 0, "1 or (1 xor 1)", 1),
        ("0 and 0 < 1", 0, "(0 and 0) < 1", 1),
        ("2 == 0 or 1", 1, "2 == (0 or 1)", 0),
        ("not 5 == 1", 1, "(not 5) == 1", 0),
        ("not 0 and 0", 0, "not (0 and 0)", 1),
        ("0 == 0 < 0", 0, "0 == (0 < 0)", 1),
        ("2 + 3 < 6", 1, "2 + (3 < 6)", 3),
        ("2 + 7 mod 3", 3, "(2 + 7) mod 3", 0),
        ("7 mod 3 * 2", 2, "7 mod (3 * 2)", 1),
        ("2 * 3 mod 4", 2, "2 * (3 mod 4)", 6),
        ("not 3!", 0, "(not 3)!", 1),
    ] {
        assert_ne!(
            expected, wrong,
            "{source} does not separate the two readings"
        );
        assert_eq!(
            value(source),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
        assert_eq!(
            value(misgrouped),
            Number::NaturalNumber(BigInt::from(wrong)),
            "for {misgrouped}, the grouping {source} must not have"
        );
    }
}

/// Assignment stays the weakest level of all, below the logical operators that
/// were inserted above it.
///
/// The value of the expression cannot show this: `x = 0 or 1` is worth 1 under
/// either grouping. Only what lands in `x` separates them — grouped as
/// `(x = 0) or 1` the assignment stores 0 and the `or` merely reports 1. There
/// is no public way to read a variable back, so the second expression reads it
/// the way a user would.
#[test]
fn test_assignment_still_binds_more_weakly_than_everything() {
    let session = Session::init();
    Expression::compile("x = 0 or 1")
        .expect("compiles")
        .eval(&session)
        .expect("evaluates");
    assert_eq!(
        Expression::compile("x")
            .expect("compiles")
            .eval(&session)
            .unwrap(),
        Number::NaturalNumber(BigInt::from(1)),
        "the or must be the assignment's right operand, not the other way round"
    );
}

/// The four levels inserted below `+` did not disturb anything above it. These
/// are the groupings the whole renumbering had to preserve, asserted end to end
/// rather than by reading the table back.
#[test]
fn test_the_old_operators_still_group_as_they_did() {
    let session = Session::init();
    for (source, expected) in [
        ("2+3*4", 14),
        ("2*3+4", 10),
        ("2-3-4", -5),
        ("2^3^2", 512),
        ("3!*2", 12),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}
