use yarer::rpn_resolver::*;
use yarer::token::*;

macro_rules! resolve {
    ($expr:expr, $expected:expr) => {{
        let mut resolver = RpnResolver::parse($expr);
        assert_eq!(resolver.resolve().unwrap(), $expected);
    }};
}

#[test]
fn test_expressions() {
    resolve!(
        "(3 + 4 * (2 - (3 + 1) * 5 + 3) - 6) * 2 + 4",
        Number::NaturalNumber(-122)
    );
    resolve!("3 * 2^3 + 6 / (2 + 1)", Number::DecimalNumber(26.0));
    //resolve!("PI * 4. + 2^PI", Number::DecimalNumber(std::f64::consts::PI * 4.0 + 2.0f64.powf(std::f64::consts::PI)));
    /*resolve!("sin(PI / 4) + cos(PI / 4)", Number::DecimalNumber(1.414213562373095)); // Approximately sqrt(2)
    resolve!("tan(PI / 4) * cos(PI / 6)", Number::DecimalNumber(0.86602540378)); // Approximately sqrt(3)/2
    resolve!("ln(e) + log10(100)", Number::NaturalNumber(3));
    resolve!("3 * 2^3! - 2 * 3 + 6 / (2 + 1)", Number::NaturalNumber(230));
    resolve!("cos(sin(0.5) * PI / 2)", Number::DecimalNumber(0.87758256189)); // Approximately cos(PI/4)
    resolve!("PI * 2^3 + PI / 2 - e", Number::DecimalNumber(2.0 * std::f64::consts::PI + 8.0 * std::f64::consts::PI / 2.0 - std::f64::consts::E));
    resolve!("2 ^ 3 ^ 2", Number::NaturalNumber(512));
    resolve!("ln(e^2) - log10(1000)", Number::NaturalNumber(0));
    resolve!("PI^2 - e^2", Number::DecimalNumber(std::f64::consts::PI * std::f64::consts::PI - std::f64::consts::E * std::f64::consts::E));
    resolve!("(2 + 3) * (3 + 4) - (4 + 5) * (5 + 6)", Number::NaturalNumber(-34));
    resolve!("tan(0) * sin(PI) + cos(PI / 2)", Number::NaturalNumber(0));
    resolve!("2^2^2 - 3^3", Number::NaturalNumber(-19));
    resolve!("(2 + 3 * 4 + 5) * 2", Number::NaturalNumber(34));
    resolve!("4! - 3!", Number::NaturalNumber(18));
    resolve!("(2^3 + 3^2) * 4", Number::NaturalNumber(104));
    resolve!("e * PI - PI * e", Number::NaturalNumber(0));
    resolve!("(2 + 3) * (4 - 5) + (6 - 7) * (8 + 9)", Number::NaturalNumber(-3));
    resolve!("ln(e^3) / log10(1000)", Number::NaturalNumber(3));
    resolve!("(2^2 + 3^2) * (4^2 + 5^2)", Number::NaturalNumber(725));
    resolve!("PI * e * (PI - e)", Number::DecimalNumber(std::f64::consts::PI * std::f64::consts::E * (std::f64::consts::PI - std::f64::consts::E)));
    */
}
