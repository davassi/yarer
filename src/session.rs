use crate::{rpn_resolver::RpnResolver, token::Number};
use num_bigint::BigInt;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// A [`Session`] is an object that holds a variable heap in the form of a [`HashMap`]
/// that is borrowed to all the [`RpnResolver`] instances built using [`process()`]
///
/// Example
///
pub struct Session {
    variable_heap: Rc<RefCell<HashMap<String, Number>>>,
}

impl Session {
    /// Default builder constructor without any arguments
    ///
    /// # Examples
    ///   
    /// ```
    /// #    use yarer::{rpn_resolver::RpnResolver, session::Session};
    ///
    ///      let exp = "4 + 4 * 2 / ( 1 - 5 )";
    ///      let mut session = Session::init();
    ///      let mut resolver: RpnResolver = session.process(&exp);
    ///  ```
    ///
    #[must_use]
    pub fn init() -> Session {
        // let variable_heap: HashMap<String, Number> = ;
        Session {
            variable_heap: Rc::new(RefCell::new(Session::init_local_heap())),
        }
    }

    /// The [`RpnResolver`] single line builder. It needs the math expression to process
    ///
    #[must_use]
    pub fn process<'a>(&'a self, line: &'a str) -> RpnResolver<'a> {
        let clone = Rc::clone(&self.variable_heap); // clones the Rc pointer, not the whole heap!
        RpnResolver::parse_with_borrowed_heap(line, clone)
    }

    /// Creates a Variables heap (name-value)
    ///
    fn init_local_heap() -> HashMap<String, Number> {
        let mut local_heap: HashMap<String, Number> = HashMap::new();
        local_heap.insert(
            "pi".to_string(),
            Number::DecimalNumber(
                num_rational::BigRational::from_float(std::f64::consts::PI).unwrap(),
            ),
        );
        local_heap.insert(
            "e".to_string(),
            Number::DecimalNumber(num_rational::BigRational::from_float(std::f64::consts::E).unwrap()),
        );
        local_heap.insert(
            "tau".to_string(),
            Number::DecimalNumber(
                num_rational::BigRational::from_float(std::f64::consts::TAU).unwrap(),
            ),
        );
        local_heap.insert(
            "phi".to_string(),
            Number::DecimalNumber(
                num_rational::BigRational::from_float((1.0 + 5.0f64.sqrt()) / 2.0).unwrap(),
            ),
        );
        local_heap.insert(
            "gamma".to_string(),
            Number::DecimalNumber(
                num_rational::BigRational::from_float(0.577_215_664_901_532_9_f64).unwrap(),
            ),
        );
        local_heap
    }

    /// Declares and saves a new integer variable ([`Number::NaturalNumber`])
    ///
    /// Example
    /// ``
    ///     session.set("foo", 42);
    /// ``
    ///
    pub fn set(&self, key: &str, value: i64) {
        self.variable_heap.borrow_mut().insert(
            key.to_lowercase(),
            Number::NaturalNumber(BigInt::from(value)),
        );
    }

    /// Declares and saves a new float variable ([`Number::DecimalNumber`])
    ///
    /// Example
    /// ``
    ///     session.setf("x", 1.5);
    /// ``
    ///
    pub fn setf(&self, key: &str, value: f64) {
        self.variable_heap
            .borrow_mut()
            .insert(
                key.to_lowercase(),
                Number::DecimalNumber(num_rational::BigRational::from_float(value).unwrap()),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Number;

    /// Test for the session initialization and basic expression processing
    #[test]
    fn test_session() {
        let session = Session::init();
        let mut resolver: RpnResolver = session.process("1+2*3/(4-5)");
        assert_eq!(
            resolver.resolve().unwrap(),
            Number::DecimalNumber(num_rational::BigRational::from_float(-5.0).unwrap())
        );
    }

    /// Test for setting an integer variable
    #[test]
    fn test_session_set() {
        let session = Session::init();
        session.set("x", 4);
        let mut resolver: RpnResolver = session.process("x+2*3/(4-5)");
        assert_eq!(
            resolver.resolve().unwrap(),
            Number::DecimalNumber(num_rational::BigRational::from_float(-2.0).unwrap())
        );
    }

    /// Test for setting a float variable
    #[test]
    fn test_session_setf() {
        let session = Session::init();
        session.setf("x", 4.5);
        let mut resolver: RpnResolver = session.process("x+2*3/(4-5)");
        assert_eq!(
            resolver.resolve().unwrap(),
            Number::DecimalNumber(num_rational::BigRational::from_float(-1.5).unwrap())
        );
    }

    /// Test for the default variables initialization
    #[test]
    fn test_session_default_vars() {
        let session = Session::init();
        let mut resolver: RpnResolver = session.process("pi + e");
        assert_eq!(
            resolver.resolve().unwrap(),
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
        let mut resolver: RpnResolver = session.process("tau / 2");
        assert_eq!(
            resolver.resolve().unwrap(),
            Number::DecimalNumber(
                num_rational::BigRational::from_float(std::f64::consts::TAU / 2.0).unwrap(),
            )
        );
    }
}
