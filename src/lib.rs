//#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
// Three duplicates, none of them resolvable from this repository: bitflags
// 1.3.2 against 2.13.1, syn 2.0.119 against 3.0.4, and windows-sys 0.59.0
// against 0.61.2, all reached transitively. The lint reads the whole
// dependency graph from the manifest rather than the features actually
// resolved, so it fires for `--no-default-features` too and gating the CLI
// behind a feature does not silence it. `expect` is not usable here: the lint
// has no span in this source to attach to. Re-check when those dependencies
// bump, and delete this if the list ever empties.
#![allow(clippy::multiple_crate_versions)]
//! Yarer (Yet another (Rusty || Rpn) expression resolver) is a flexible library, written in Rust, for the processing, compilation and evaluation of mathematical expressions using Reverse Polish Notation.
//!
//! The landing page, [davassi.github.io/yarer](https://davassi.github.io/yarer/),
//! shows what the crate does on one page, with every printed value taken from
//! the release binary.
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
//! ## Operators
//!
//! Weakest to strongest: `=`, then `or` `xor`, `and`, `not`, the six
//! comparisons `<` `>` `<=` `>=` `==` `<>`, `+` `-`, `*` `/` `mod`, `^`,
//! unary `-`, and postfix `!`. `=`, `^` and `not` associate to the right and
//! the rest to the left.
//!
//! A comparison yields `1` or `0`, as in GNU bc: there is no boolean type, and
//! [`Number`] does not grow a variant. Because the answer is a number, a
//! comparison doubles as a mask — which is how to write a branch in a language
//! that has none.
//!
//! ```
//! use yarer::{Expression, Number, Session};
//!
//! let session = Session::init();
//! // The payoff of a European call at expiry, max(S-K, 0), with no branch:
//! // the comparison is 1 when the option is in the money and 0 when it is not.
//! let payoff = Expression::compile("(S > K) * (S - K)").unwrap();
//!
//! session.set("K", 100).unwrap();
//!
//! session.set("S", 120).unwrap();
//! assert_eq!(payoff.eval(&session).unwrap().to_string(), "20");
//!
//! session.set("S", 80).unwrap();
//! assert_eq!(payoff.eval(&session).unwrap().to_string(), "0");
//! ```
//!
//! The logical operators read **any** non-zero value as true, fractions and
//! negatives included, so `1/3 and 2` is 1 and only zero is false. `mod`
//! truncates toward zero, so the result takes the sign of the dividend:
//! `-7 mod 3` is `-1`. There is no `!=`, because `!` is the factorial and
//! `5!=3` would be ambiguous; `<>` is the spelling. And `and`, `or`, `xor`,
//! `not` and `mod` are reserved words in every casing, so they cannot be
//! variable names.
//!
//! ## Errors
//!
//! Two kinds, kept apart in the type system rather than in a message prefix:
//! [`ParseError`] is produced while an expression is being compiled, and
//! [`EvalError`] while a compiled expression is being evaluated. A caller that
//! wants one type across both calls converts into [`Error`].
//!
//! Every error that is about a specific token carries a [`Span`]: a byte range
//! into the source text. [`Error::render`] turns that into the message plus the
//! source line plus a caret under the offending token — which is what the
//! bundled REPL uses to report a bad expression.
//!
//! ```
//! use yarer::{Error, Expression, ParseError};
//!
//! let source = "max(1,*2)";
//! let err = Expression::compile(source).unwrap_err();
//!
//! // React to *what* went wrong...
//! assert!(matches!(err, ParseError::ExpectedValue { .. }));
//!
//! // ...and know *where*.
//! let span = err.span().unwrap();
//! assert_eq!((span.start, span.end), (6, 7));
//!
//! assert_eq!(
//!     Error::from(err).render(source),
//!     "Parse error: expected a value, found '*'\n  max(1,*2)\n        ^"
//! );
//! ```
//!
//! ## Limits
//!
//! [`Expression::eval`] runs under the session's own [`Limits`].
//! [`Expression::eval_with`] runs the *same* compiled expression under a
//! different budget instead, against the same variables — which is how to
//! give text you don't control a tight budget and text you do a loose one,
//! without maintaining two sessions.
//!
//! ```
//! use yarer::{Expression, Session, Limits};
//!
//! let session = Session::init();
//! let expr = Expression::compile("2^1000").unwrap();
//!
//! // A tight budget for text you don't control...
//! let tight = Limits::default().with_max_value_bits(64);
//! assert!(expr.eval_with(&session, tight).is_err());
//!
//! // ...and the session's own, looser budget for text you do.
//! assert!(expr.eval(&session).is_ok());
//! ```
//!
//! Mind the floor the built-in constants impose: `pi`, `e`, `tau`, `phi` and
//! `gamma` are held as exact rationals that cost up to 107 bits, so a budget
//! under that refuses a value the caller never actually supplied.
//!
//! Yarer can be used also from command line, and behaves in a very similar manner to GNU bc
//!
//! ```ignore
//! $ yarer
//! Yarer v.0.4.0 - Yet Another Rust Expression Resolver.
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
