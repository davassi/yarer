use crate::error::EvalError;
use crate::limits::Limits;
use crate::token::Number;
use num_bigint::BigInt;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// A [`Session`] is an object that holds a variable heap in the form of a [`HashMap`]
/// that every [`Expression`](crate::Expression) evaluated against it
/// reads and writes.
///
/// Example
///
pub struct Session {
    variable_heap: Rc<RefCell<HashMap<String, Number>>>,
    limits: Limits,
}

const BUILTIN_CONSTANTS: [&str; 5] = ["pi", "e", "tau", "phi", "gamma"];

impl Session {
    /// Default builder constructor without any arguments
    ///
    /// # Examples
    ///
    /// ```
    /// #    use yarer::{Expression, Session};
    ///
    ///      let exp = "4 + 4 * 2 / ( 1 - 5 )";
    ///      let session = Session::init();
    ///      let expr = Expression::compile(exp).unwrap();
    ///      let result = expr.eval(&session).unwrap();
    ///  ```
    ///
    #[must_use]
    pub fn init() -> Session {
        Session::with_limits(Limits::default())
    }

    /// Builds a session whose evaluations are bound by `limits`.
    ///
    /// The bound reaches the built-in constants too, and they are wider than
    /// they look: `pi`, `e`, `tau`, `phi` and `gamma` are `f64`s held exactly as
    /// rationals, costing numerator bits plus denominator bits — 99 for `pi`, and
    /// 107 for `gamma`, the widest. A `max_value_bits` under 107 therefore
    /// rejects a value the caller never supplied. A limit meant to bound
    /// untrusted input should sit well above that.
    #[must_use]
    pub fn with_limits(limits: Limits) -> Session {
        Session {
            variable_heap: Rc::new(RefCell::new(Session::init_local_heap())),
            limits,
        }
    }

    /// The limits every [`Expression::eval`](crate::Expression::eval)
    /// against this session uses.
    #[must_use]
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// The value of `name`, or [`None`] if it has never been set.
    ///
    /// Lowercases `name` itself, like [`Session::assign`] does, so a caller
    /// never has to pre-lowercase before looking a variable up.
    pub(crate) fn lookup(&self, name: &str) -> Option<Number> {
        self.variable_heap
            .borrow()
            .get(&name.to_lowercase())
            .cloned()
    }

    /// Writes `value` into the heap, refusing the built-in constants.
    ///
    /// This is the one place that refusal is decided. `set`, `setf` and the
    /// evaluator's assignment operator all come through here.
    ///
    /// # Errors
    /// [`EvalError::ReadOnlyConstant`] when `name` is a built-in constant.
    pub(crate) fn assign(&self, name: &str, value: Number) -> Result<(), EvalError> {
        let name = name.to_lowercase();
        if Session::is_constant_name(&name) {
            return Err(EvalError::ReadOnlyConstant { name, span: None });
        }
        self.variable_heap.borrow_mut().insert(name, value);
        Ok(())
    }

    /// Creates a Variables heap (name-value)
    ///
    fn init_local_heap() -> HashMap<String, Number> {
        let mut local_heap: HashMap<String, Number> = HashMap::new();
        local_heap.insert(
            "pi".to_string(),
            Number::decimal(num_rational::BigRational::from_float(std::f64::consts::PI).unwrap()),
        );
        local_heap.insert(
            "e".to_string(),
            Number::decimal(num_rational::BigRational::from_float(std::f64::consts::E).unwrap()),
        );
        local_heap.insert(
            "tau".to_string(),
            Number::decimal(num_rational::BigRational::from_float(std::f64::consts::TAU).unwrap()),
        );
        local_heap.insert(
            "phi".to_string(),
            Number::decimal(
                num_rational::BigRational::from_float(f64::midpoint(1.0, 5.0f64.sqrt())).unwrap(),
            ),
        );
        local_heap.insert(
            "gamma".to_string(),
            Number::decimal(
                num_rational::BigRational::from_float(0.577_215_664_901_532_9_f64).unwrap(),
            ),
        );
        local_heap
    }

    fn is_constant_name(key: &str) -> bool {
        BUILTIN_CONSTANTS.contains(&key)
    }

    /// Declares or overwrites an integer variable.
    ///
    /// Example
    /// ``
    ///     session.set("foo", 42).expect("not a constant");
    /// ``
    ///
    /// # Errors
    /// [`EvalError::ReadOnlyConstant`] when `key` names a built-in constant.
    pub fn set(&self, key: &str, value: i64) -> Result<(), EvalError> {
        self.assign(key, Number::NaturalNumber(BigInt::from(value)))
    }

    /// Declares or overwrites a variable from an [`f64`].
    ///
    /// The value decides the representation, not the setter. The rational is
    /// built through [`Number::decimal`], so an integral `f64` is stored as a
    /// [`Number::NaturalNumber`] — `setf("x", 4.0)` stores `4` — and only a
    /// genuinely fractional value is stored as a [`Number::DecimalNumber`].
    ///
    /// Example
    /// ``
    ///     session.setf("x", 1.5).expect("not a constant");
    /// ``
    ///
    /// # Errors
    /// [`EvalError::ReadOnlyConstant`] when `key` names a built-in constant, and
    /// [`EvalError::NotFinite`] for NaN or an infinity — which used to be accepted
    /// silently and stored nothing.
    pub fn setf(&self, key: &str, value: f64) -> Result<(), EvalError> {
        let number = Number::try_from(value).map_err(|_| EvalError::NotFinite { value })?;
        self.assign(key, number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::Expression;
    use crate::token::Number;

    /// Compiles and evaluates in one step, for the tests below that care about
    /// the value and not about which of the two steps produced it.
    fn eval(session: &Session, source: &str) -> Number {
        Expression::compile(source).unwrap().eval(session).unwrap()
    }

    /// Asserts both the value and the variant. Cross-variant equality means a
    /// value-only assertion cannot tell `NaturalNumber(-5)` from
    /// `DecimalNumber(-5/1)`, so only the `matches!` makes this sensitive to the
    /// canonicalisation invariant it is here to protect.
    #[test]
    fn test_session() {
        let session = Session::init();
        let result = eval(&session, "1+2*3/(4-5)");
        assert_eq!(result, Number::NaturalNumber(BigInt::from(-5)));
        assert!(
            matches!(result, Number::NaturalNumber(_)),
            "produced {result:?}, expected a NaturalNumber"
        );
    }

    /// Test for setting an integer variable
    #[test]
    fn test_session_set() {
        let session = Session::init();
        session.set("x", 4).expect("not a constant");
        let result = eval(&session, "x+2*3/(4-5)");
        assert_eq!(result, Number::NaturalNumber(BigInt::from(-2)));
        assert!(
            matches!(result, Number::NaturalNumber(_)),
            "produced {result:?}, expected a NaturalNumber"
        );
    }

    /// Test for setting a float variable
    #[test]
    fn test_session_setf() {
        let session = Session::init();
        session.setf("x", 4.5).expect("not a constant");
        assert_eq!(
            eval(&session, "x+2*3/(4-5)"),
            Number::DecimalNumber(num_rational::BigRational::from_float(-1.5).unwrap())
        );
    }

    /// Test for the default variables initialization
    #[test]
    fn test_session_default_vars() {
        let session = Session::init();
        assert_eq!(
            eval(&session, "pi + e"),
            Number::DecimalNumber(
                num_rational::BigRational::from_float(std::f64::consts::PI).unwrap()
                    + num_rational::BigRational::from_float(std::f64::consts::E).unwrap()
            )
        );
    }

    /// Test for the tau variable
    #[test]
    fn test_session_tau() {
        let session = Session::init();
        assert_eq!(
            eval(&session, "tau / 2"),
            Number::DecimalNumber(
                num_rational::BigRational::from_float(std::f64::consts::TAU / 2.0).unwrap(),
            )
        );
    }
}
