use num::BigInt;
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

macro_rules! resolve_decimal {
    ($expr:expr, $expected:expr) => {{
        resolve!($expr, Number::DecimalNumber($expected));
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
    resolve_decimal!("sin(pi/4) + cos(pi/4)", 1.414213562373095); // Approximately sqrt(2)
    resolve_decimal!("tan(pi/4) * cos(pi/6)", 0.8660254037844386); // Approximately sqrt(3)/2
    resolve_decimal!("ln(e) + log10(100)", 1.0);
    //resolve_natural!("3 * 2^3! - 2 * 3 + 6 / (2 + 1)", 188);
    resolve_decimal!("cos(sin(0.5) * pi / 2)", 0.7295860397469262); // Approximately cos(PI/4)
    resolve_decimal!(
        "pi * 2^3 + pi / 2 - e",
        8.0 * std::f64::consts::PI + std::f64::consts::PI / 2.0 - std::f64::consts::E
    );
    resolve_natural!("2 ^ 3 ^ 2", 512);
    resolve_decimal!("ln(e^2) - log10(1000)", 2.);
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
    resolve_decimal!("ln(e^3) / log10(1000)", 3.);
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
    resolve_decimal!("max(1+2,3*4)-min(10,5)", 9.0);
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
    resolve_decimal!("max(1+2,2+2)", 6.0);
    //resolve_decimal!("min(3!,10)", 9.0);
    //resolve_decimal!("max(2^3,3^2)", 19683.0);
    //resolve_decimal!("min(max(2^3,3^3),max(4^2,2^5))", 4294967296.0);
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

        let b: i64 = b.into();
        assert!(b == 3265920i64);
    }
}

#[test]
fn test_session_set() {
    let session = Session::init();
    session.set("x", 4);
    let mut resolver: RpnResolver = session.process("x+2*3/(4-5)");
    assert_eq!(resolver.resolve().unwrap(), Number::DecimalNumber(-2.0));
}

#[test]
fn test_factorial_invalid_operand() {
    let session = Session::init();
    let mut resolver = session.process("(-1)!");
    assert!(resolver.resolve().is_err());

    let mut resolver = session.process("2.5!");
    assert!(resolver.resolve().is_err());
}
