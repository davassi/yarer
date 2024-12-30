
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use num_bigint::BigInt;
use crate::{rpn_resolver::RpnResolver, token::Number};

/// A [`Session`] represents a shared, mutable context that stores variables (in a [`HashMap`])
/// used by [`RpnResolver`] instances. Each [`RpnResolver`] created by [`Session::process`]
/// will reference the same variable heap, allowing variables to be set once and reused 
/// across multiple expressions.
///
/// # Example
///
/// ```
/// # use yarer::{rpn_resolver::RpnResolver, session::Session};
/// let expression = "1 + 2 * 3 / (4 - 5)";
/// let session = Session::init();
/// let mut resolver = session.process(expression);
///
/// let result = resolver.resolve().unwrap();
/// println!("The result of '{}' is {:?}", expression, result);
/// ```
pub struct Session {
    /// Shared variable heap for storing user defined expression variables (e.g., `x`, `k`, `pi`, etc.).
    variable_heap: Rc<RefCell<HashMap<String, Number>>>,
}

impl Session {

    /// Creates a new [`Session`] with some default variables (e.g., `pi`, `e`).
    ///
    /// # Example
    ///
    /// ```
    /// # use yarer::{rpn_resolver::RpnResolver, session::Session};
    /// let session = Session::init();
    /// let mut resolver = session.process("4 + 4 * 2 / (1 - 5)");
    ///
    /// let result = resolver.resolve().unwrap();
    /// assert_eq!(format!("{:?}", result), "DecimalNumber(2.0)"); // or similar comparison
    /// ```
    #[must_use]
    pub fn init() -> Session {
        Session {
            variable_heap: Rc::new(RefCell::new(Session::init_local_heap())),
        }
    }

    // -------------------------------------------------------------------------
    // Public API
    // -------------------------------------------------------------------------

    /// Parses the given `expression` into an [`RpnResolver`] that references this [`Session`]'s 
    /// shared variable heap. This allows you to update the session variables in-between
    /// evaluations if necessary.
    ///
    /// # Example
    ///
    /// ```
    /// # use yarer::{rpn_resolver::RpnResolver, session::Session};
    /// let session = Session::init();
    /// let mut resolver = session.process("1 + 2 * 3 / (4 - 5)");
    /// assert_eq!(resolver.resolve().unwrap().to_string(), "-5");
    /// ```
    #[must_use]
    pub fn process<'a>(&'a self, line: &'a str) -> RpnResolver<'_> {
        let clone = Rc::clone(&self.variable_heap); // clones the Rc pointer, not the whole heap!
        RpnResolver::parse_with_borrowed_heap(line, clone)
    }

    /// Sets a new **integer** variable in the session, stored as a [`Number::NaturalNumber`].
    ///
    /// # Example
    ///
    /// ```
    /// # use yarer::{rpn_resolver::RpnResolver, session::Session};
    /// let session = Session::init();
    /// session.set("answer", 42);
    ///
    /// let mut resolver = session.process("answer + 1");
    /// assert_eq!(resolver.resolve().unwrap().to_string(), "43");
    /// ```
    pub fn set(&self, key: &str, value: i64) {
        self.variable_heap
            .borrow_mut()
            .insert(key.to_string(), Number::NaturalNumber(BigInt::from(value)));
    }

    /// Sets a new **floating-point** variable in the session, stored as a [`Number::DecimalNumber`].
    ///
    /// # Example
    ///
    /// ```
    /// # use yarer::{rpn_resolver::RpnResolver, session::Session};
    /// let session = Session::init();
    /// session.setf("ratio", 1.5);
    ///
    /// let mut resolver = session.process("ratio * 2");
    /// assert_eq!(resolver.resolve().unwrap().to_string(), "3");
    /// ```
    pub fn setf(&self, key: &str, value: f64) {
        self.variable_heap
            .borrow_mut()
            .insert(key.to_string(), Number::DecimalNumber(value));
    }

    // -------------------------------------------------------------------------
    // Private Helpers
    // -------------------------------------------------------------------------

    /// Creates a default variable heap containing common mathematical constants.
    fn init_local_heap() -> HashMap<String, Number> {

        let mut local_heap: HashMap<String, Number> = HashMap::new();
        local_heap.insert("pi".to_string(), Number::DecimalNumber(std::f64::consts::PI));
        local_heap.insert("e".to_string(), Number::DecimalNumber(std::f64::consts::E));
        local_heap
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Number;

    #[test]
    fn test_session_init_and_process() {
        let session = Session::init();
        let mut resolver = session.process("1+2*3/(4-5)");
        assert_eq!(resolver.resolve().unwrap(), Number::DecimalNumber(-5.0));
    }

    #[test]
    fn test_session_set_integer() {
        let session = Session::init();
        session.set("x", 4);
        let mut resolver = session.process("x + 2 * 3 / (4 - 5)");
        assert_eq!(resolver.resolve().unwrap(), Number::DecimalNumber(-2.0));
    }

    #[test]
    fn test_session_set_float() {
        let session = Session::init();
        session.setf("x", 1.5);
        let mut resolver = session.process("x + 0.5");
        assert_eq!(resolver.resolve().unwrap(), Number::DecimalNumber(2.0));
    }
}