use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{rpn_resolver::RpnResolver, token::Number};

/// A [`Session`] is an object that holds a variable heap in the form of a [`HashMap`]
/// that is borrowed to all the [`RpnResolver`] that are built from the builder [`build_resolver_for`()]
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
    pub fn process<'a>(&'a self, line: &'a str) -> RpnResolver<'_> {
        let clone = Rc::clone(&self.variable_heap); // clones the Rc pointer, not the whole heap!
        RpnResolver::parse_with_borrowed_heap(line, clone)
    }

    /// Creates a Variables heap (name-value)
    ///
    fn init_local_heap() -> HashMap<String, Number> {
        static PI: Number = Number::DecimalNumber(std::f64::consts::PI);
        static E: Number = Number::DecimalNumber(std::f64::consts::E);

        let mut local_heap: HashMap<String, Number> = HashMap::new();
        local_heap.insert("pi".to_string(), PI);
        local_heap.insert("e".to_string(), E);
        local_heap
    }

    /// Declares and saves a new integer variable ([`Number::NaturalNumber`])
    ///
    /// Example
    /// ``
    ///     session.set("foo", 42);
    /// ``
    ///
    pub fn set(&self, key: &str, value: i32) {
        self.variable_heap
            .borrow_mut()
            .insert(key.to_string(), Number::NaturalNumber(value));
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
            .insert(key.to_string(), Number::DecimalNumber(value));
    }
}
