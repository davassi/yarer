
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

## Limits

`expr.eval(&session)` runs under the session's own `Limits`. `expr.eval_with(&session, limits)` runs the *same* compiled expression under a different budget instead, against the same variables — which is how to give text you don't control a tight budget and text you do a loose one, without maintaining two sessions:

```rust
      use yarer::{Expression, Session, Limits};

      let session = Session::init();
      let expr = Expression::compile("2^1000").unwrap();

      // A tight budget for text you don't control...
      let tight = Limits::default().with_max_value_bits(64);
      assert!(expr.eval_with(&session, tight).is_err());

      // ...and the session's own, looser budget for text you do.
      assert!(expr.eval(&session).is_ok());
```

Mind the floor the built-in constants impose: `pi`, `e`, `tau`, `phi` and `gamma` are held as exact rationals that cost up to 107 bits, so a budget under that refuses a value the caller never actually supplied.

## CLI

Yarer can also be used from the command line and behaves similarly to GNU bc

```text
      $ yarer
      Yarer v.0.4.0 - Yet Another Rust Expression Resolver.
      License MIT OR Apache-2.0
      > (1+9)*(8+2)+0!
      101
      > (1./2)+atan(10)
      1.9711276743037347
      > x=10
      10
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
      > 9801/(2206*sqrt(2))
      3.1415927300133055
```

That last one is Ramanujan's approximation of pi. Yarer has no comment
syntax, so the note lives here rather than after the expression.

An assignment prints the value assigned, because that is what the expression
evaluates to.
## News and Updates

### What's new in 0.4.0

* `yarer -e "1+2"` and piped input, alongside the interactive REPL.
* The CLI's dependencies are behind a feature that is on by default, so a
  library-only build compiles 41 crates instead of 74.
* A declared and CI-verified minimum Rust version.
* An operand too small to be an `f64` is refused rather than silently zeroed —
  `log(1/(10^400))` is `-400`, not an error.

**The full history, and the migration table for 0.2.0 → 0.3.0, is in
[CHANGELOG.md](CHANGELOG.md).**

## Using it as a library only

The command-line binary's dependencies are behind a feature that is on by
default, so `cargo install yarer` works unchanged. A program that only wants
the evaluator can turn it off:

```toml
yarer = { version = "0.4", default-features = false }
```

which drops `clap`, `rustyline`, `dirs` and `env_logger` and takes the
dependency tree from 74 crates to 41 — no argument parser, no line editor, and
no `jiff`, which `env_logger` pulls in to format timestamps.

## Operators

| | operators | associativity |
|---|---|---|
| weakest | `=` assignment | right |
| | `or` `xor` | left |
| | `and` | left |
| | `not` (prefix) | right |
| | `<` `>` `<=` `>=` `==` `<>` | left |
| | `+` `-` | left |
| | `*` `/` `mod` | left |
| | `^` | right |
| | unary `-` | right |
| strongest | `!` factorial (postfix) | left |

A comparison yields `1` or `0`, as in GNU bc — there is no boolean type, and
`(1<2) + 5` is a legal expression worth 6. The logical operators read **any**
non-zero value as true, fractions and negatives included, so `1/3 and 2` is 1
and only zero is false.

Because the answer is a number, a comparison doubles as a mask, which is how to
write a branch in a language that has none:

```text
      > S=120; K=100
      100
      > (S > K) * (S - K)
      20
      > S=80
      80
      > (S > K) * (S - K)
      0
```

`mod` truncates toward zero, so the result takes the sign of the dividend:
`-7 mod 3` is `-1` and `7 mod -3` is `1`, the convention of C, Rust, bc and
BASIC. It is defined on rationals too — `7.5 mod 2` is `1.5`.

`not` binds more weakly than the comparisons, as in Python and unlike C, so
`not a == b` reads the way its spelling suggests: `not (a == b)`.

**There is no `!=`.** `!` is the postfix factorial, so `5!=3` could be read as
`(5!) = 3` or as `5 != 3`. `<>` is unambiguous, and that is why it is the
spelling. For the same reason `not` is a word: the symbol is taken.

**`and`, `or`, `xor`, `not` and `mod` are reserved words**, in every casing, and
can no longer be used as variable names. Like the function names, they are
matched case-insensitively: `and`, `And` and `AND` are the same operator.

Because yarer holds decimals as exact rationals, comparison answers what
floating point cannot: `0.1+0.2 == 0.3` is `1`, and so is
`1/3 + 1/3 + 1/3 == 1`. There is no epsilon to choose because there is no
rounding error to absorb.

There is no short-circuit evaluation. `0 and (2^1000000)` evaluates its right
operand — a stack machine has both operands before it sees the operator — so
that expression is refused by the size budget rather than answering `0`, and
`0 and 1/0` is a division-by-zero error rather than `0`. `and` and `or` combine
two values; they do not guard one with the other.

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
      Yarer v.0.4.0 - Yet Another Rust Expression Resolver.
      License MIT OR Apache-2.0
      > S=100;K=100;T=1;r=0.05;sigma=0.2;
      0.2
      > d1=(ln(S/K)+(r+sigma^2/2)*T)/(sigma*sqrt(T))
      0.35
      > d2=d1-sigma*sqrt(T)
      0.15
      > S*cdf(d1)-K*exp(-r*T)*cdf(d2)
      10.45058357218556
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

Each expression goes through the following steps, the first three run by `Expression::compile` and the fourth by `Expression::eval`:

Step 1 - Parser: A string is "regexed" and converted into a token array.

Step 2 - Validation: the token array is walked once, checking that it describes a well-formed expression — brackets balance, calls get the right number of arguments, values and operators alternate correctly — and rewriting unary operators along the way. This is the pass that turns five previously-identical "malformed expression" failures into five distinct, positioned diagnoses.

Step 3 - Shunting yard: the validated token array is converted from infix to postfix (RPN) notation, becoming a compiled `Expression`.

Step 4 - Expression: The resulting RPN (Reverse Polish Notation) expression is evaluated against a `Session`.

It's worth mentioning that the Session is responsible for storing all variables (and constants) that are borrowed by every `Expression` evaluated against it.

## Contribution

Besides being stable, Yarer is a work in progress. If you have suggestions for features (i.e. more math functions to implement), or if you find any issues in the code, design, interface, etc, please feel free to share them on our [GitHub](https://github.com/davassi/yarer/issues).

I appreciate very much your feedback!
