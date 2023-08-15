use std::collections::HashMap;

use crate::{rpn_resolver::RpnResolver, token::Number};

/// A [Session] is an object that holds a variable heap in the form of a [HashMap]
/// that is borrowed to all the [RpnResolver] that are built from the builder [build_resolver_for()]
///
/// Example
///
pub struct Session {
    variable_heap: HashMap<String, Number>,
}

impl Session {
    /// Default builder constructor without any arguments
    ///
    /// # Examples
    ///   
    /// ```
    /// #     use yarer::session::Session;
    /// #     use yarer::rpn_resolver::RpnResolver;
    ///
    ///      let exp = "4 + 4 * 2 / ( 1 - 5 )";
    ///      let mut session = Session::init();
    ///      let mut resolver: RpnResolver = session.build_resolver_for(&exp);
    ///  ```
    ///
    pub fn init() -> Session {
        let variable_heap: HashMap<String, Number> = Session::init_local_heap();
        Session { variable_heap }
    }

    /// The [RpnResolver] single line builder. Needs the math expression to process
    ///
    pub fn build_resolver_for<'a>(&'a mut self, line: &'a str) -> RpnResolver<'_> {
        RpnResolver::parse_with_borrowed_heap(line, &mut self.variable_heap)
    }

    /// Creates a Variables heap (name-value)
    fn init_local_heap() -> HashMap<String, Number> {
        static PI: Number = Number::DecimalNumber(std::f64::consts::PI);
        static E: Number = Number::DecimalNumber(std::f64::consts::E);

        let mut local_heap: HashMap<String, Number> = HashMap::new();
        local_heap.insert("pi".to_string(), PI);
        local_heap.insert("e".to_string(), E);
        local_heap
    }
}
