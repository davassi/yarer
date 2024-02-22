//#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
//! Yarer (Yet another (Rusty || Rpn) expression resolver) is a flexible library, written in Rust, for the processing, compilation and evaluation of mathematical expressions using Reverse Polish Notation.
//!
//! # Example of usage of the library:
//!
//!  ```
//!     use yarer::{rpn_resolver::RpnResolver, session::Session, token::Number};
//!
//!     let exp = "((10 + 5) - 3 * ( 9 / 3 )) + 2";
//!     let session = Session::init();
//!     let mut resolver: RpnResolver = session.process(&exp);
//!
//!     let result: Number = resolver.resolve().unwrap();
//!     println!("The result of {} is {}", exp, result);
//!  ```
//!
//! All that's needed is to create a new instance of the [`RpnResolver`] and hand over the expression to be analysed.
//! The library just returns a variant natural number, or a decimal number if one exists in the expression (i.e '2.1+1') or there's a trigonometric function (i.e. 1/cos(x+1)).
//!
//! Yarer can handle also variables and functions. Here an example:
//!
//! ```
//! # use yarer::{rpn_resolver::RpnResolver, session::Session};
//!
//! let session: Session = Session::init();
//! let mut resolver: RpnResolver = session.process("1/cos(x^2)");
//! session.set("x",1);
//!
//! println!("The result is {}", resolver.resolve().unwrap());
//! ```
//!
//! and of course, the expression can be re-evaluated if the variable changes.
//!
//! ```
//! # use yarer::{rpn_resolver::RpnResolver, session::Session};
//! # let session: Session = Session::init();
//! # let mut resolver: RpnResolver = session.process("1/cos(x^2)");
//!
//! session.set("x",-1);
//! println!("The result is {}", resolver.resolve().unwrap());
//!
//! session.setf("x",0.001);
//! println!("The result is {}", resolver.resolve().unwrap());
//! ```
//!
//! The result can be simply converted into a i32 or a f64 (if decimal) simply with
//!
//! ```
//! # use yarer::{rpn_resolver::RpnResolver, session::Session, token::Number};
//! # let session: Session = Session::init();
//! # let mut resolver: RpnResolver = session.process("1/cos(x^2)");
//!
//! let result: Number = resolver.resolve().unwrap();
//!
//! let int : i32 = result.clone().into();
//! // or
//! let float : f64 = result.into();
//! ```
//!
//! Yarer can be used also from command line, and behaves in a very similar manner to GNU bc
//!
//! ```ignore
//! $ yarer
//! Yarer v.0.1.1 - Yet Another (Rusty||Rpn) Expression Resolver.
//! License MIT OR Apache-2.0
//! > (1+9)*(8+2)
//! 100
//! > (1./2)+atan(10)
//! 1.1483608274590869
//! > x=10
//! > 3/sin(5*x^2)
//! -6.41338354698791
//! > ln(1)
//! 0
//! > log(10)
//! 1
//! > -2^-2
//! 0.25
//! > 1/(log(10)+cos(0))^-2
//! 4
//! > 4.5+7.9*2.2
//! 21.88
//! > 9801/(2206*sqrt(2)) // approx of PI
//! 3.1415927300133055
//!
//! ```
//!
//! ## Built-in Defined Functions
//!
//! There are several math functions defined that you can use in your expression. More to come!
//! There are many examples of processed expressions in the [integration test file](https://github.com/davassi/yarer/blob/master/tests/integration_tests.rs)
//!
//! ```ignore
//! Sin
//! Cos
//! Tan
//! ASin
//! ACos
//! ATan
//! Ln
//! Log
//! Abs
//! Sqrt
//! ```
/// Parser
pub mod parser;
/// `RpnResolver`
pub mod rpn_resolver;
/// Session
pub mod session;
/// Token
pub mod token;
