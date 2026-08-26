
YARER - The math expression Evaluator
===========================

[<img alt="github" src="https://img.shields.io/badge/github-davassi/davassi?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/davassi/yarer)
[<img alt="build status" src="https://github.com/davassi/yarer/actions/workflows/rust.yml/badge.svg" height="20">](https://github.com/davassi/yarer/actions?query=branch%3Amaster)
[<img alt="crates.io" src="https://img.shields.io/crates/v/yarer.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/yarer)
[<img alt="docs.rs" src="https://img.shields.io/docsrs/yarer?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/yarer)
[![Downloads](https://img.shields.io/crates/d/yarer.svg)](https://crates.io/crates/yarer)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Project Status: Active – The project has reached a stable, usable state and is being actively developed.](https://www.repostatus.org/badges/latest/active.svg)](https://www.repostatus.org/#active)

Yarer (Yet Another Rust Expression Resolver) is a library for evaluating mathematical expressions. Internally it uses the shunting yard algorithm.

## Usage

Example of usage of the library:

```rust
      use yarer::{Expression, Session};

      let session = Session::init();
      let expr = Expression::compile("1+2").unwrap(); // or even "(cos(10+e)+3*sin(9/pi))^2"

      println!("The result is {}", expr.eval(&session).unwrap());
```

All that's needed is to compile the expression into an `Expression` and evaluate it against a `Session`.
The library returns a `Number`, and the value decides which variant, not the expression that produced it. An integral result always comes back as `Number::NaturalNumber`, whatever it came from — `2.5+2.5` is `5`, `1/cos(0)` is `1`, `6/3` is `2`. `Number::DecimalNumber` appears only when the value genuinely has a fractional part, as in `0.1+0.2` or `1/3`. Every mathematical value therefore has exactly one representation.

## Variables

Yarer handles variables and functions. Here is an example:

```rust
      use yarer::{Expression, Session};

      let session = Session::init();
      let expr = Expression::compile("1/cos(x^2)").unwrap();

      session.set("x", 1).unwrap();
      println!("The result is {}", expr.eval(&session).unwrap());
```

and of course, the expression can be re-evaluated if the variable changes.

```rust
      //...
      session.set("x", -1).unwrap();
      println!("The result is {}", expr.eval(&session).unwrap());

      session.setf("x", 0.001).unwrap();
      println!("The result is {}", expr.eval(&session).unwrap());
      //...
```

`Expression` is `Clone`, and cloning copies the whole compiled token sequence, so
it's cheap to compile once and evaluate against several sessions, but worth
knowing before cloning inside a hot loop.

## Casting

The result can be converted into an i32 or an f64 (if decimal) using the
fallible `TryFrom`/`TryInto` conversions, which return an error instead of
panicking when the value does not fit the target type:

```rust
      let result: Number = expr.eval(&session).unwrap();

      let int : i32 = result.clone().try_into().unwrap();
      // or
      let float : f64 = result.try_into().unwrap();
```

## Errors

`Expression::compile` fails with a `ParseError`, `expr.eval` with an `EvalError`.
Both carry a byte-range `Span` when the failure is about a specific token, and
both convert into the union type `Error` for a caller that wants one type
across both calls. `Error::render` turns that into the message, the source
line, and a caret under the offending token — the same rendering the bundled
REPL uses:

```rust
      use yarer::{Error, Expression, ParseError};

      let source = "max(1,*2)";
      match Expression::compile(source) {
          Err(err @ ParseError::ExpectedValue { .. }) => {
              println!("{}", Error::from(err).render(source));
          }
          other => panic!("expected a parse error, got {other:?}"),
      }
```

prints:

```text
Parse error: expected a value, found '*'
  max(1,*2)
        ^
```

## CLI

Yarer can also be used from the command line and behaves similarly to GNU bc

```text
      $ yarer
      Yarer v.0.2.0 - Yet Another Rust Expression Resolver.
      License MIT OR Apache-2.0
      > (1+9)*(8+2)+0!
      101
      > (1./2)+atan(10)
      1.1483608274590869
      > x=10
      > 3/sin(5*x^2)
      -6.41338354698791
      > ln(1)
      0
      > log(10)
      1
      > -2^-2
      0.25
      > 1/(log(10)+cos(0))^-2
      4
      > 4.5+7.9*2.2
      21.88
      > 9801/(2206*sqrt(2)) // approx of PI
      3.1415927300133055
      
```
## News and Updates

### Unreleased — migrating from 0.2.0

The next release (targeting 0.3.0, not yet published) replaces the public API
with one where every failure is a typed error carrying a position, instead of
a string. It is a breaking change; everything that moved is in this table.

| Before | After |
|---|---|
| `session.process(s) -> RpnResolver` | `Expression::compile(s) -> Result<Expression, ParseError>` |
| `resolver.resolve() -> anyhow::Result<Number>` | `expr.eval(&session) -> Result<Number, EvalError>` |
| `RpnResolver::parse_with_borrowed_heap(..)` public | removed from the public surface |
| `Parser`, `Token`, `Operator`, `Bracket`, `Associate` public | `pub(crate)` |
| `a / b` on `Number`, panics when `b` is zero | `a.checked_div(&b) -> Option<Number>` |
| `Token::compare_operator_priority` public, panics | internal, and total |
| `session.set(..)` / `setf(..)` return `()` | return `Result<(), EvalError>` |
| `Limits { max_value_bits: n }` | `Limits::default().with_max_value_bits(n)` |
| errors are `anyhow::Error` strings | `Error`, `ParseError`, `EvalError`, with spans |
| `!5` returns `120` | `ParseError::ExpectedValue` |
| `()`, `2 3`, `2(3+4)`, `1+`, `max(1,*2)` all "malformed" | five distinct errors, each with a caret position |
| `max(1,(2,3))` claims no call is open | `ParseError::CommaInPlainBracket` |

Unchanged on purpose: undefined variables still read as `0`; `sin[5]` still
evaluates; chained assignment (`x=y=5`) and chained expressions
(`x=2; y=3; x*y`) are unaffected; every numeric result already documented
above stays the same.

### Version 0.2.0

Yarer 0.2.0 is a correctness-focused release that includes a breaking API change.

**Breaking change:** conversions from `Number` to `i32`, `i64`, `i128` and `f64` are now
fallible. They are exposed via `TryFrom`/`TryInto` (returning a `ConversionError`) instead of
the previous panicking `From`/`Into`. Conversion to `BigInt` remains infallible via `From`.
Update `let n: i32 = result.into();` to `let n: i32 = result.try_into()?;` (or `.unwrap()`).

Other changes:

* Out-of-range numeric conversions now return an error instead of panicking.
* `Number` → `BigInt` conversion is exact (truncates the rational toward zero) rather than round-tripping through `f64`, so precision is no longer silently lost.
* A trailing `;` now returns the last segment's value instead of reporting a spurious "malformed expression" error after the assignment already took effect.
* Malformed segments inside a `;`-chained expression are now rejected instead of being silently discarded.
* Stricter parser validation: malformed expressions and unexpected tokens raise a clear error.
* Removed an unused, internally-panicking `BitXor` implementation for `Number`.

### Version 0.1.8

Yarer 0.1.8 comes with several enhancements:

* Decimal numbers are now represented using the `num-rational` crate for higher precision.
* Added new math functions: `floor`, `ceil`, `round`, `exp`, `pdf` and `cdf`.
* Expressions can be chained with semicolons, e.g. `x=2; y=3; x*y`.
* Variable assignments inside expressions are handled more reliably.
* This README includes a demonstration of the Black–Scholes formula.

Starting with Yarer version 0.1.7, natural numbers are implemented internally using [BigInt](https://crates.io/crates/num-bigint) from the [num crate](https://crates.io/crates/num). Now it is possible to do calculations with arbitrarily large natural numbers.

```text
    $ yarer
      Yarer v.0.2.0 - Yet Another Rust Expression Resolver.
      License MIT OR Apache-2.0
      > 78!
      1132428117820629783145752115873204622873174957948825.....
      > 2^78
      302231454903657293676544
```

From Yarer version 0.1.5 it's possible to share a single session, and therefore a single heap of variables, for multiple compiled expressions. The library is not intended to be thread-safe.

```rust
    use yarer::{Expression, Session};

    let session = Session::init();

    let expr1 = Expression::compile("x ^ 2").unwrap();
    let expr2 = Expression::compile("x! - (x-1)!").unwrap();

    session.set("x", 10).unwrap();

    if let (Ok(a), Ok(b)) = (expr1.eval(&session), expr2.eval(&session)) {
        println!("{} {}", a, b); // 100 3265920
    }
```

## Built-in Defined Functions

There are several math functions defined that you can use in your expression. More to come!
There are many examples of processed expressions in the [integration test file](https://github.com/davassi/yarer/blob/master/tests/integration_tests.rs).

```text
    Sin
    Cos
    Tan
    ASin
    ACos
    ATan
    Ln
    Log
    Abs
    Sqrt
    Max
    Min
    Floor
    Ceil
    Round
    Exp
    Pdf
    Cdf
```

Function arguments are always parenthesised: `sqrt(16)`, `max(1,2)`.

## Built-in Defined Constants

There are a few predefined math constants available:

```text
    PI    -> 3.14159265...
    e     -> 2.7182818...
    tau   -> 6.2831853...
    phi   -> 1.6180339...
    gamma -> 0.57721566...
```

## Example: Black-Scholes Option Pricing

Using Yarer, the Black–Scholes formula for a European call option can be evaluated straight from the CLI.

```text
      $ yarer
      Yarer v.0.2.0 - Yet Another Rust Expression Resolver.
      License MIT OR Apache-2.0
      > S=100;K=100;T=1;r=0.05;sigma=0.2;
      > d1=(ln(S/K)+(r+sigma^2/2)*T)/(sigma*sqrt(T))
      > d2=d1-sigma*sqrt(T)
      > S*cdf(d1)-K*exp(-r*T)*cdf(d2)
      10.450583572185565
```

## Execute

To run it from cargo, just type:

```console
cargo run -q -- 
```

For logging debug just run with:

```console
env RUST_LOG=yarer=debug cargo run -q -- 
```

or to build and install a release from the code:

```console
cargo build --release
cargo install --path .
./target/release/yarer
```

## Internal Implementation

Each expression goes through the following steps, the first two run by `Expression::compile` and the third by `Expression::eval`:

Step 1 - Parser: A string is "regexed" and converted into a token array.

Step 2 - Shunting yard: the token array is converted from infix to postfix (RPN) notation, becoming a compiled `Expression`.

Step 3 - Expression: The resulting RPN (Reverse Polish Notation) expression is evaluated against a `Session`.

It's worth mentioning that the Session is responsible for storing all variables (and constants) that are borrowed by every `Expression` evaluated against it.

## Contribution

Besides being stable, Yarer is a work in progress. If you have suggestions for features (i.e. more math functions to implement), or if you find any issues in the code, design, interface, etc, please feel free to share them on our [GitHub](https://github.com/davassi/yarer/issues).

I appreciate very much your feedback!
