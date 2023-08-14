//#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
//!
//!  Yarer - Reverse Polish Notification expression resolver
//!
//!  The internal flow is conceptually pretty simple:
//!
//!  1 Yarer parses and converts a str into a vec of borrowed &str
//!  2 Then it maps a vec of &str into a vec of tokens
//!  3 Then converts the infix expression to the postfix the vec
//!  4 resolve the expression!
//!
//!  Point 1 and 2 are executed by the Parser, 3 and 4 by the RpnResolver
//!
//!  # Usage
//!
//!  Example
//!  ```   
//      let exp = "4 + 4 * 2 / ( 1 - 5 )";
//      let mut session = Session::init();
//      let mut resolver: RpnResolver = session.build_resolver_for(&exp);
//
//      let result: token::Number = resolver.resolve().unwrap();
//      println!("The result of {} is {}", exp, result);
//!  ```
//!
//!
pub mod parser;
pub mod rpn_resolver;
pub mod session;
pub mod token;
