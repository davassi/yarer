use num::BigInt;
use yarer::limits::Limits;
use yarer::rpn_resolver::*;
use yarer::session::Session;
use yarer::token::*;

macro_rules! resolve {
    ($expr:expr, $expected:expr) => {{
        let session = Session::init();
        let mut resolver = session.process($expr);
        assert_eq!(resolver.resolve().unwrap(), $expected);
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
        let mut resolver = session.process($expr);
        let result = resolver.resolve().unwrap();
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

macro_rules! resolve_err {
    ($expr:expr) => {{
        let session = Session::init();
        let mut resolver = session.process($expr);
        assert!(resolver.resolve().is_err());
    }};
    () => {
        panic!("Expected an error, but got a valid result.")
    };
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
    let mut resolver: RpnResolver = session.process("x ^ 2");

    for i in 1..=64 {
        session.set("x", i);

        let result: Number = resolver.resolve().unwrap();

        println!("{}^2={}", i, result);
        assert!(result == Number::NaturalNumber(BigInt::from(i * i)));
    }
}

#[test]
fn test_sharing_session() {
    let session = Session::init();

    let mut res = session.process("x ^ 2");
    let mut res2 = session.process("x! - (x-1)!");

    session.set("x", 10);

    if let (Ok(a), Ok(b)) = (res.resolve(), res2.resolve()) {
        assert!(a == Number::NaturalNumber(BigInt::from(100)));

        let b: i64 = b.try_into().unwrap();
        assert!(b == 3265920i64);
    }
}

#[test]
fn test_session_set() {
    let session = Session::init();
    session.set("x", 4);
    let mut resolver: RpnResolver = session.process("x+2*3/(4-5)");
    let result = resolver.resolve().unwrap();
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
    let mut resolver = session.process("(-1)!");
    assert!(resolver.resolve().is_err());

    let mut resolver = session.process("2.5!");
    assert!(resolver.resolve().is_err());
}

#[test]
fn test_chained_expressions() {
    let session = Session::init();
    let mut resolver = session.process("x=2; y=3; x*y");
    assert_eq!(
        resolver.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(6))
    );
}

#[test]
fn test_chained_without_assignment() {
    let session = Session::init();
    let mut resolver = session.process("1+2; 3+4");
    assert_eq!(
        resolver.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(7))
    );
}

#[test]
fn test_trailing_semicolon_returns_last_value() {
    // A trailing ';' must return the last completed segment's value, not error.
    let session = Session::init();
    let mut resolver = session.process("x=2;");
    assert_eq!(
        resolver.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(2))
    );
}

#[test]
fn test_trailing_semicolon_does_not_error_after_assignment() {
    // The assignment side-effect and the reported result must agree: a successful
    // assignment must not be reported as a malformed-expression error.
    let session = Session::init();
    let mut resolver = session.process("a=5;");
    assert!(resolver.resolve().is_ok());

    let mut reader = session.process("a");
    assert_eq!(
        reader.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(5))
    );
}

#[test]
fn test_malformed_segment_in_chain_is_rejected() {
    // A malformed segment ('1 2' is two adjacent operands) must not be silently
    // discarded by the following ';'.
    let session = Session::init();
    let mut resolver = session.process("1 2; 3");
    assert!(resolver.resolve().is_err());
}

#[test]
fn test_invalid_input_is_rejected() {
    resolve_err!("1@2");
    resolve_err!("1 2");
    resolve_err!("(1+2");
    resolve_err!("1+2)");
}

#[test]
fn test_unicode_operators_work() {
    resolve_decimal!("2×3 + 8÷4", 8.0);
}

#[test]
fn test_decimal_literals_remain_exact() {
    let session = Session::init();
    let mut resolver = session.process("0.1+0.2");
    assert_eq!(
        resolver.resolve().unwrap(),
        Number::DecimalNumber(num_rational::BigRational::new(
            BigInt::from(3),
            BigInt::from(10)
        ))
    );
}

#[test]
fn test_large_integer_division_and_negative_power_do_not_panic() {
    let session = Session::init();
    let mut resolver = session.process("(10^100)/2");
    assert_eq!(
        format!("{}", resolver.resolve().unwrap()),
        format!("5{}", "0".repeat(99))
    );

    let mut resolver = session.process("(10^100)^-1 * 10^100");
    let result = resolver.resolve().unwrap();
    assert_eq!(result, Number::NaturalNumber(BigInt::from(1)));
    // A reciprocal multiplied back out lands exactly on 1, which is integral:
    // the variant is the whole point here, and cross-variant equality would let
    // DecimalNumber(1/1) through the assertion above.
    assert!(
        matches!(result, Number::NaturalNumber(_)),
        "produced {result:?}, expected a NaturalNumber"
    );
}

#[test]
fn test_builtin_constants_are_read_only() {
    let session = Session::init();
    session.set("pi", 0);

    let mut resolver = session.process("pi");
    let pi: f64 = resolver.resolve().unwrap().try_into().unwrap();
    assert!((pi - std::f64::consts::PI).abs() < 1e-10);

    let mut resolver = session.process("pi=0");
    assert!(resolver.resolve().is_err());
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

#[test]
fn test_domain_errors_are_rejected() {
    resolve_err!("sqrt(-1)");
    resolve_err!("ln(0)");
    resolve_err!("ln(-5)");
    resolve_err!("log(0)");
    resolve_err!("1/0");
    resolve_err!("5/(3-3)");
}

#[test]
fn test_variable_names_are_case_insensitive() {
    let session = Session::init();
    let mut resolver = session.process("X=7; x");
    assert_eq!(
        resolver.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(7))
    );
}

#[test]
fn test_chained_assignment_sets_all_variables() {
    let session = Session::init();
    let mut resolver = session.process("x=y=5");
    assert_eq!(
        resolver.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(5))
    );

    let mut reader = session.process("x+y");
    assert_eq!(
        reader.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(10))
    );
}

#[test]
fn test_large_result_to_i64_returns_error_not_panic() {
    // 2^200 vastly exceeds i64: the public TryFrom must report an error.
    let session = Session::init();
    let mut resolver = session.process("2^200");
    let n = resolver.resolve().unwrap();
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
    let mut resolver = session.process("2^100");
    assert_eq!(
        format!("{}", resolver.resolve().unwrap()),
        "1267650600228229401496703205376"
    );
}

#[test]
fn test_setf_declares_a_decimal_variable() {
    let session = Session::init();
    session.setf("r", 2.5);
    let mut resolver = session.process("r*2");
    let v: f64 = resolver.resolve().unwrap().try_into().unwrap();
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
    for expr in ["6/3", "floor(3.7)", "exp(0)", "max(1,2)", "sqrt(16)"] {
        let mut resolver = session.process(expr);
        let result = resolver.resolve().unwrap();
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
        let mut resolver = session.process(expr);
        let result = resolver.resolve().unwrap();
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
    let mut resolver = session.process("999999999!");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("size limit"), "message was: {err}");
}

#[test]
fn test_oversized_power_is_refused_not_computed() {
    let session = Session::init();
    let mut resolver = session.process("10^100000000");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("size limit"), "message was: {err}");
}

#[test]
fn test_legitimate_big_values_still_pass_the_default_limit() {
    resolve_natural!("2^64", 18_446_744_073_709_551_616_i128);
    let session = Session::init();
    let mut resolver = session.process("1000!");
    // 1000! needs about 8530 bits, comfortably inside the default budget.
    assert!(resolver.resolve().is_ok());
}

#[test]
fn test_the_limit_is_configurable() {
    let session = Session::with_limits(Limits { max_value_bits: 64 });
    let mut resolver = session.process("2^100");
    assert!(
        resolver.resolve().is_err(),
        "2^100 needs 101 bits, over a 64-bit budget"
    );

    let mut small = session.process("2^10");
    assert!(small.resolve().is_ok());
}

#[test]
fn test_growth_through_multiplication_is_caught() {
    // Budget 4000 is exactly the power's own prediction for this base and
    // exponent (size_in_bits(2) * 2000 = 2 * 2000 = 4000): the largest value the
    // predictive power check admits for "2^2000", so a failure here can only come
    // from the multiplication, not from the power check firing again. 2^2000 is
    // actually 2001 bits (passes both checks); squaring it needs 4001 bits, over
    // budget, and only the post-hoc Mul check can catch that.
    let session = Session::with_limits(Limits {
        max_value_bits: 4000,
    });
    let mut resolver = session.process("x=2^2000; x*x");
    let err = resolver.resolve().unwrap_err().to_string();
    // "occupies" is the post-hoc wording; a prediction says "would need". Asserting
    // the wording is what makes the paragraph above a claim the test checks rather
    // than a comment asking to be believed.
    assert!(err.contains("occupies"), "message was: {err}");
}

#[test]
fn test_oversized_exponent_reports_its_own_message() {
    // The exponent itself doesn't fit in a u64 (it's far larger than u64::MAX),
    // so this must be refused before any size prediction is even attempted - and
    // with its own message, not the unrelated "Invalid power operation" that
    // covers a different failure (a non-integer powf conversion).
    let session = Session::init();
    let mut resolver = session.process("2^99999999999999999999");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("exponent is too large"), "message was: {err}");
    assert!(
        !err.contains("Invalid power operation"),
        "message was: {err}"
    );
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
    let session = Session::with_limits(Limits { max_value_bits: 32 });
    let mut resolver = session.process("99999999999999999999");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("occupies"), "message was: {err}");
    assert!(err.contains("size limit"), "message was: {err}");

    // A literal inside the budget is untouched, so the guard is not simply
    // refusing everything.
    let mut inside = session.process("123+1");
    assert_eq!(
        inside.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(124))
    );
}

#[test]
fn test_an_oversized_variable_is_refused() {
    // Same hole as the literal, one match arm over. Bounding untrusted input is
    // the whole reason to call with_limits, and setf is a way in that does not
    // pass through any of the checked operators: 1e308 is stored as a ~1024-bit
    // integer, and reading it back needs no arithmetic at all.
    let session = Session::with_limits(Limits { max_value_bits: 64 });
    session.setf("x", 1e308);

    // Bare "x" is the case no other guard can catch: the value is pushed and
    // returned without a single operator running over it.
    let mut resolver = session.process("x");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("occupies"), "message was: {err}");
    assert!(err.contains("size limit"), "message was: {err}");

    // A variable inside the budget still reads back normally.
    session.set("y", 7);
    let mut inside = session.process("y*2");
    assert_eq!(
        inside.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(14))
    );

    // An undefined variable still resolves to zero. That is deliberate, and a
    // size check on the variable push must not turn it into an error.
    let mut undefined = session.process("z+1");
    assert_eq!(
        undefined.resolve().unwrap(),
        Number::NaturalNumber(BigInt::from(1))
    );
}

#[test]
fn test_a_tiny_budget_rejects_the_builtin_constants() {
    // The constants are f64s held exactly as rationals, so they are wide:
    // numerator bits plus denominator bits, tau is the narrowest at 98 and gamma
    // the widest at 107 (pi is 884279719003555/281474976710656, 50 + 49 = 99).
    // A variable is size-checked as it is pushed, so a small budget rejects a
    // value the caller never supplied. This test exists so the 107-bit floor
    // quoted by Session::with_limits stays a measured number, not folklore.
    let below_all = Session::with_limits(Limits { max_value_bits: 97 });
    for name in ["pi", "e", "tau", "phi", "gamma"] {
        let mut resolver = below_all.process(name);
        let err = resolver.resolve().unwrap_err().to_string();
        assert!(err.contains("size limit"), "{name} reported: {err}");
    }

    // 107 is the exact floor, pinned from both sides: one bit under it the
    // widest constant still does not fit, at it every constant does. Asserting
    // only the lower bound would pass for any number that happens to be too
    // small, which is how a figure like this drifts out of date unnoticed.
    let one_short = Session::with_limits(Limits {
        max_value_bits: 106,
    });
    let mut widest = one_short.process("gamma");
    assert!(widest.resolve().is_err(), "gamma fits in 106 bits?");

    let exact = Session::with_limits(Limits {
        max_value_bits: 107,
    });
    for name in ["pi", "e", "tau", "phi", "gamma"] {
        let mut resolver = exact.process(name);
        assert!(
            resolver.resolve().is_ok(),
            "{name} was rejected at 107 bits"
        );
    }
}

#[test]
fn test_wrong_arity_is_diagnosed_by_name() {
    let session = Session::init();
    for (expr, expected, given) in [("max(1)", 2, 1), ("max(1,2,3)", 2, 3), ("sin(1,2)", 1, 2)] {
        let mut resolver = session.process(expr);
        let err = resolver.resolve().unwrap_err().to_string();
        assert!(
            err.contains(&format!("expects {expected}")) && err.contains(&format!("{given} given")),
            "{expr} reported: {err}"
        );
    }
}

#[test]
fn test_empty_argument_list_is_diagnosed() {
    let session = Session::init();
    let mut resolver = session.process("max()");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("0 given"), "message was: {err}");
}

#[test]
fn test_comma_outside_a_function_call_is_diagnosed() {
    let session = Session::init();
    let mut resolver = session.process("(1,2)");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("function call"), "message was: {err}");
}

#[test]
fn test_a_function_name_requires_parentheses() {
    let session = Session::init();
    // "sin;" pins down that a pending function cannot survive a ';' statement
    // boundary either: the mandatory-parenthesis check fires on the very next
    // token, whatever it is, before the ';' arm ever runs.
    for expr in ["sin 5", "sqrt 16", "cos", "sin;"] {
        let mut resolver = session.process(expr);
        let err = resolver.resolve().unwrap_err().to_string();
        assert!(
            err.contains("must be followed by"),
            "{expr} reported: {err}"
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
    let session = Session::init();
    let mut resolver = session.process("max(1; 2)");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("before ';'"), "message was: {err}");
    assert!(
        !err.contains("expects"),
        "should not be reported as an arity mismatch: {err}"
    );
}

#[test]
fn test_empty_argument_slot_is_diagnosed() {
    // A comma with nothing before or after it must be its own diagnosis, not
    // silently absorbed into the argument count.
    let session = Session::init();
    for expr in ["max(,1)", "max(1,)"] {
        let mut resolver = session.process(expr);
        let err = resolver.resolve().unwrap_err().to_string();
        assert!(err.contains("empty"), "{expr} reported: {err}");
    }
}

#[test]
fn test_nested_empty_group_does_not_fake_an_argument() {
    // The inner "()" is empty and must not be counted as content for the
    // outer call's only argument slot.
    let session = Session::init();
    let mut resolver = session.process("sin(())");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(
        err.contains("expects 1") && err.contains("0 given"),
        "message was: {err}"
    );
}

#[test]
fn test_unbalanced_closing_bracket_is_diagnosed() {
    let session = Session::init();
    let mut resolver = session.process("1+2)");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("Unbalanced brackets"), "message was: {err}");
}

#[test]
fn test_unbalanced_opening_bracket_is_diagnosed() {
    // The same condition class as the test above, and it used to get the generic
    // "malformed expression" message instead of the named one. "max(1,2" is the
    // sharper case: the bracket never closes, so the arity check on the closing
    // bracket never ran at all.
    let session = Session::init();
    for expr in ["(1+2", "max(1,2"] {
        let mut resolver = session.process(expr);
        let err = resolver.resolve().unwrap_err().to_string();
        assert!(
            err.contains("Unbalanced brackets"),
            "{expr} reported: {err}"
        );
        assert!(!err.contains("malformed"), "{expr} reported: {err}");
    }
}

#[test]
fn test_single_argument_function_called_empty_is_diagnosed() {
    let session = Session::init();
    let mut resolver = session.process("sin()");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(
        err.contains("expects 1") && err.contains("0 given"),
        "message was: {err}"
    );
}

#[test]
fn test_comma_inside_nested_plain_group_within_a_call_is_diagnosed() {
    let session = Session::init();
    let mut resolver = session.process("max((1,2),3)");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("function call"), "message was: {err}");
}
