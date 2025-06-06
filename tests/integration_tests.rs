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
}

#[test]
fn test_expressions() {
    resolve!(
        "(3 + 4 * (2 - (3 + 1) * 5 + 3) - 6) * 2 + 4",
        Number::NaturalNumber(BigInt::from(-122))
    );
    resolve!("tau", Number::DecimalNumber(std::f64::consts::TAU));
    resolve!(
        "phi",
        Number::DecimalNumber((1.0 + 5.0f64.sqrt()) / 2.0)
    );
    resolve!(
        "gamma",
        Number::DecimalNumber(0.577_215_664_901_532_9_f64)
    );
    resolve!("3 * 2^3 + 6 / (2 + 1)", Number::DecimalNumber(26.0));
    resolve!(
        "pi * 4. + 2^pi",
        Number::DecimalNumber(std::f64::consts::PI * 4.0 + 2.0f64.powf(std::f64::consts::PI))
    );
    resolve!(
        "2^3 * 4 + 5^2",
        Number::NaturalNumber(BigInt::from(8 * 4 + 25))
    );
    resolve!(
        "sin(pi / 4) + cos(pi / 4)",
        Number::DecimalNumber(1.414213562373095)
    ); // Approximately sqrt(2)
    resolve!(
        "tan(pi / 4) * cos(pi / 6)",
        Number::DecimalNumber(0.8660254037844386)
    ); // Approximately sqrt(3)/2
    resolve!("ln(e) + log10(100)", Number::DecimalNumber(1.0));
    //resolve!("3 * 2^3! - 2 * 3 + 6 / (2 + 1)", Number::NaturalNumber(230));
    resolve!(
        "cos(sin(0.5) * pi / 2)",
        Number::DecimalNumber(0.7295860397469262)
    ); // Approximately cos(PI/4)
    resolve!(
        "pi * 2^3 + pi / 2 - e",
        Number::DecimalNumber(
            8.0 * std::f64::consts::PI + std::f64::consts::PI / 2.0 - std::f64::consts::E
        )
    );
    resolve!("2 ^ 3 ^ 2", Number::NaturalNumber(BigInt::from(512)));
    resolve!("ln(e^2) - log10(1000)", Number::DecimalNumber(2.));
    resolve!(
        "pi^2 - e^2",
        Number::DecimalNumber(
            std::f64::consts::PI * std::f64::consts::PI - std::f64::consts::E * std::f64::consts::E
        )
    );
    resolve!(
        "(2 + 3) * (3 + 4) - (4 + 5) * (5 + 6)",
        Number::NaturalNumber(BigInt::from(-64))
    );
    resolve!(
        "tan(0) * sin(pi) + cos(pi / 2)",
        Number::DecimalNumber(6.123233995736766e-17)
    );
    resolve!("2^2^2 - 3^3", Number::NaturalNumber(BigInt::from(-11)));
    resolve!("(2 + 3 * 4 + 5) * 2", Number::NaturalNumber(BigInt::from(38)));
    resolve!("4! - 3!", Number::NaturalNumber(BigInt::from(18)));
    resolve!("(2^3 + 3^2) * 4", Number::NaturalNumber(BigInt::from(68)));
    resolve!("e * pi - pi * e", Number::DecimalNumber(0.0));
    resolve!(
        "(2 + 3) * (4 - 5) + (6 - 7) * (8 + 9)",
        Number::NaturalNumber(BigInt::from(-22))
    );
    resolve!("ln(e^3) / log10(1000)", Number::DecimalNumber(3.));
    resolve!("(2^2 + 3^2) * (4^2 + 5^2)", Number::NaturalNumber(BigInt::from(533)));
    resolve!(
        "pi*e*(pi-e)",
        Number::DecimalNumber(
            std::f64::consts::PI
                * std::f64::consts::E
                * (std::f64::consts::PI - std::f64::consts::E)
        )
    );
    resolve!("((10 + 5) - 3 * ( 9 / 3 )) + 2", Number::DecimalNumber(8.0));
    resolve!(
        "2^3^2 - 3^3",
        Number::NaturalNumber(BigInt::from(512 - 27))
    );
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

        let b : i64 = b.into(); 
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
