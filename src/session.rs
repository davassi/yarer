use std::collections::HashMap;

use crate::{rpn_resolver::RpnResolver, token::Number};

pub struct Session {
    variable_heap: HashMap<String, Number>,
}

impl Session {
    pub fn init() -> Session {
        let variable_heap: HashMap<String, Number> = Session::init_local_heap();
        Session { variable_heap }
    }

    pub fn build_resolver_for<'a>(&'a mut self, line: &'a str) -> RpnResolver<'_> {
        RpnResolver::parse_with_borrowed_heap(line, &mut self.variable_heap)
    }

    fn init_local_heap() -> HashMap<String, Number> {
        static PI: Number = Number::DecimalNumber(std::f64::consts::PI);
        static E: Number = Number::DecimalNumber(std::f64::consts::E);

        let mut local_heap: HashMap<String, Number> = HashMap::new();
        local_heap.insert("pi".to_string(), PI);
        local_heap.insert("e".to_string(), E);
        local_heap
    }
}
