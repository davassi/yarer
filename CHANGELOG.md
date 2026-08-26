# Changelog

All notable changes to this project are documented here, in the format of
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). This project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html); while the major
version is 0, a bump of the minor version is where breaking changes go.

## [0.4.0] - 2026-08-26

The surface release: yarer becomes usable from a shell, honest about what it
makes people download and which compiler it needs, and gated by CI that checks
something.

### Added
- `yarer -e "1+2"` evaluates an expression and exits. It may be given more than
  once, and all the expressions share one session, so a variable set by one is
  visible to the next.
- Piped or redirected input is evaluated a line at a time against one session,
  as in GNU bc. The interactive REPL is unchanged and is still what you get
  from a terminal.
- A `cli` feature, on by default. `yarer = { version = "0.4",
  default-features = false }` builds the library without `clap`, `rustyline`,
  `dirs` or `env_logger` — **41 crates instead of 74**, dropping an argument
  parser, a line editor and, through `env_logger`, `jiff`, a complete datetime
  library.
- `rust-version = "1.88"`, verified in CI at exactly that toolchain. The
  library alone builds on 1.86; the extra two versions come from
  `rustyline 16` requiring `home ^0.5.12`.
- `EvalError::OperandTooSmallForFloat` and `EvalError::PowerOperandsTooSmall`,
  the mirrors of the two "too large" variants.
- A fuzz target over compile-then-evaluate, and a committed corpus replayed on
  stable by an ordinary test on every push.

### Changed
- Values go to stdout and errors to stderr, and the binary exits 1 on the first
  failure. Nothing after a failing expression runs. The startup banner is no
  longer printed in the non-interactive modes, where it would corrupt a
  captured value. This also means the REPL's own errors now go to stderr.
- `bigdecimal` and `lazy_static` are gone — nothing referenced them.
  `once_cell` is gone too, replaced by `std::sync::LazyLock`.
- CI checks formatting, denies every clippy warning, builds and tests without
  the `cli` feature, and pins the MSRV. The Codecov upload is removed: nothing
  in the workflow generated a coverage report for it to send.

### Fixed
- **An operand too small to be represented as an `f64` is refused instead of
  being silently replaced by zero.** `log(1/(10^400))` is exactly `-400` and
  was reported as "function result is not a real number"; `ln(1/(10^400))` is
  `-921.03…` and was reported the same way; `sqrt(1/(10^400))` is `1e-200`,
  comfortably representable, and answered `0`. So did `(1/(10^400))^0.5`.
- `f64::try_from` on such a value answers `Err(OutOfRange)` rather than
  `Ok(0.0)`.
- Four of the README's CLI transcripts documented behaviour the binary does not
  have, including `(1./2)+atan(10)` given as `1.1483608274590869` when it is
  `1.9711276743037347`. They are corrected, and executed by a test now.
- `apply_functional_token_operation` cloned its right operand and never used
  the clone.

### Removed
- Functions whose operand underflows to zero no longer answer as though the
  operand were zero. `sin(1/(10^400))` returned `0`, which was correct, and is
  now refused with `OperandTooSmallForFloat`. This is the same trade 0.3.0 made
  on the overflow side when `atan(10^400)` stopped returning `pi/2`: one rule
  about what can be represented, applied in both directions, is worth more than
  a handful of correct answers at the extreme edge of the value space.

## [0.3.0] - 2026-08-26

Migrating from 0.2.0.

Yarer 0.3.0 replaces the public API with one where every failure is a typed
error carrying a position instead of a string, and adds ten operators. It is a
breaking change; everything that moved is in this table.

| Before | After |
|---|---|
| `session.process(s) -> RpnResolver` | `Expression::compile(s) -> Result<Expression, ParseError>` |
| `resolver.resolve() -> anyhow::Result<Number>` | `expr.eval(&session) -> Result<Number, EvalError>` |
| `RpnResolver::parse_with_borrowed_heap(..)` public | removed from the public surface |
| `Parser`, `Token`, `Operator`, `Bracket`, `Associate` public | `pub(crate)` |
| `yarer::token::{Number, ConversionError, MathFunction}` | `yarer::{Number, ConversionError, MathFunction}` |
| `yarer::session::Session` | `yarer::Session` |
| `a / b` on `Number`, panics when `b` is zero | `a.checked_div(&b) -> Option<Number>` |
| `Token::compare_operator_priority` public, panics | internal, and total |
| `session.set(..)` / `setf(..)` return `()` | return `Result<(), EvalError>` |
| `Limits { max_value_bits: n }` | `Limits::default().with_max_value_bits(n)` |
| errors are `anyhow::Error` strings | `Error`, `ParseError`, `EvalError`, with spans |
| `ConversionError` is `Eq` and exhaustive | gains `NotFinite { value: f64 }`, loses `Eq`, becomes `#[non_exhaustive]` |
| `MathFunction` is exhaustive | `#[non_exhaustive]`: an exhaustive `match` on it needs a `_` arm |
| `Number::decimal` keeps an unreduced rational a decimal | it reduces first: `Number::decimal(BigRational::new_raw(4, 2))` is `NaturalNumber(2)` |
| `!5` returns `120` | `ParseError::ExpectedValue` |
| a function on an operand too large for `f64` returned the limit value there, for the functions that have one (`atan(10^400)` was `pi/2`; also `exp`, `cdf`, `pdf`) | `EvalError::OperandTooLargeForFloat` |
| `()`, `2 3`, `2(3+4)`, `1+`, `max(1,*2)` all "malformed" | five distinct errors, each with a caret position |
| `max(1,(2,3))` claims no call is open | `ParseError::CommaInPlainBracket` |
| `and`, `or`, `xor`, `not`, `mod` are valid variable names | reserved words, in every casing |

Four of those rows want a sentence more than a cell.

**Module paths.** `Number`, `Session`, `ConversionError` and `MathFunction`
are unchanged as types, but `token`, `session` and `rpn_resolver` are no
longer public modules — everything public is re-exported from the crate root,
so one `use yarer::{..}` covers it. 0.2.0's own crate documentation told
adopters to write `use yarer::{rpn_resolver::RpnResolver, session::Session,
token::Number};`, so every 0.2.0 user has imports that need rewriting even
where the type they name did not move.

**`ConversionError` is no longer `Eq`.** The new `NotFinite { value: f64 }`
carries an `f64`, which is not `Eq`, and no manual implementation can honestly
supply one. `PartialEq` is unchanged, so `==` still works; a bound of `T: Eq`
does not. It is also `#[non_exhaustive]` now, like every other public enum
here, so an exhaustive `match` on it needs a `_` arm.

**`Number::decimal` reduces.** It used to test `denom().is_one()` without
reducing, so an externally built `Ratio::new_raw(4, 2)` was integral but
unreduced and came back as a `DecimalNumber` — which also made `PartialEq` and
`PartialOrd` disagree about it. Values built from yarer's own arithmetic are
unaffected: `BigRational` reduces its own results.

**Ten operators are new**, and one of them takes something away. `<` `>` `<=`
`>=` `==` `<>`, `and` `or` `xor` `not`, and `mod` are described under
[Operators](#operators) above. Everything about them is additive except the
five words, which stop being usable as variable names — the one break in this
half of the release. An expression that used `mod` or `and` as a variable now
fails to compile with `ParseError::ExpectedValue` rather than silently reading
the undefined variable as `0`, so the failure is loud and positioned.

Nothing that evaluated before changes value. The precedence ladder grew from
six levels to ten, but the six operators that predate the new ones keep their
order relative to one another and no new level was interleaved between two old
ones, so no existing expression re-groups. The suite's 143 value assertions are
the test of that claim and are unmodified.

Unchanged on purpose: undefined variables still read as `0`; `sin[5]` still
evaluates; chained assignment (`x=y=5`) and chained expressions
(`x=2; y=3; x*y`) are unaffected; every numeric result already documented
above stays the same.

## [0.2.0] - 2025-06-14

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

## [0.1.8]

Yarer 0.1.8 comes with several enhancements:

* Decimal numbers are now represented using the `num-rational` crate for higher precision.
* Added new math functions: `floor`, `ceil`, `round`, `exp`, `pdf` and `cdf`.
* Expressions can be chained with semicolons, e.g. `x=2; y=3; x*y`.
* Variable assignments inside expressions are handled more reliably.
* This README includes a demonstration of the Black–Scholes formula.

Starting with Yarer version 0.1.7, natural numbers are implemented internally using [BigInt](https://crates.io/crates/num-bigint) from the [num crate](https://crates.io/crates/num). Now it is possible to do calculations with arbitrarily large natural numbers.

```text
    $ yarer
      Yarer v.0.3.0 - Yet Another Rust Expression Resolver.
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
