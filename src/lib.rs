//#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
//! Yarer (Yet another (Rusty || Rpn) expression resolver) is a flexible library, written in Rust, for the processing, compilation and evaluation of mathematical expressions using Reverse Polish Notation.
//!
//! # Example of usage of the library:
//!
//!  ```
//!     use yarer::{Expression, Session, Number};
//!
//!     let exp = "((10 + 5) - 3 * ( 9 / 3 )) + 2";
//!     let session = Session::init();
//!     let expr = Expression::compile(exp).unwrap();
//!
//!     let result: Number = expr.eval(&session).unwrap();
//!     println!("The result of {} is {}", exp, result);
//!  ```
//!
//! All that's needed is to compile the expression into an [`Expression`] and evaluate it against a [`Session`].
//! The library returns a [`Number`], and the value decides which variant, not the
//! expression that produced it. An integral result always comes back as
//! [`Number::NaturalNumber`], whatever it came from — `2.5+2.5` is `5`,
//! `1/cos(0)` is `1`, `6/3` is `2`. [`Number::DecimalNumber`] appears only
//! when the value genuinely has a fractional part, as in `0.1+0.2` or `1/3`. Every mathematical value therefore
//! has exactly one representation.
//!
//! Yarer can handle also variables and functions. Here an example:
//!
//! ```
//! # use yarer::{Expression, Session};
//!
//! let session: Session = Session::init();
//! let expr = Expression::compile("1/cos(x^2)").unwrap();
//! session.set("x",1).unwrap();
//!
//! println!("The result is {}", expr.eval(&session).unwrap());
//! ```
//!
//! and of course, the expression can be re-evaluated if the variable changes.
//!
//! ```
//! # use yarer::{Expression, Session};
//! # let session: Session = Session::init();
//! # let expr = Expression::compile("1/cos(x^2)").unwrap();
//!
//! session.set("x",-1).unwrap();
//! println!("The result is {}", expr.eval(&session).unwrap());
//!
//! session.setf("x",0.001).unwrap();
//! println!("The result is {}", expr.eval(&session).unwrap());
//! ```
//!
//! The result can be simply converted into a i32 or a f64 (if decimal) simply with
//!
//! ```
//! # use yarer::{Expression, Session, Number};
//! # let session: Session = Session::init();
//! # let expr = Expression::compile("1/cos(x^2)").unwrap();
//!
//! let result: Number = expr.eval(&session).unwrap();
//!
//! let int : i32 = result.clone().try_into().unwrap();
//! // or
//! let float : f64 = result.try_into().unwrap();
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
//! Max
//! Min
//! Floor
//! Ceil
//! Round
//! Exp
//! Pdf
//! Cdf
//! ```
//!
//! Function arguments are always parenthesised: `sqrt(16)`, `max(1,2)`.
mod error;
mod expression;
mod functions;
pub mod limits;
mod parser;
mod session;
mod shunting;
mod span;
mod token;
mod validate;

pub use error::{Error, EvalError, ParseError};
pub use expression::Expression;
pub use limits::Limits;
pub use session::Session;
pub use span::Span;
pub use token::{ConversionError, MathFunction, Number};
