# Reliable Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the four reliability defects in yarer's evaluation core — factorial rejecting integral decimals, `Number` violating the `PartialEq`/`PartialOrd` contract, factorial and power never terminating on large operands, and function arity never being validated.

**Architecture:** Four changes in a fixed order. First a pure extraction that moves function evaluation out of the 243-line `resolve()` into its own module, guarded by the existing suite. Then a canonical representation for `Number` that makes "an integral value tagged as decimal" unrepresentable. Then size limits that predict the size of a result and refuse before computing it. Finally argument counting inside the shunting yard.

**Tech Stack:** Rust 2021. `num-bigint`/`num-rational` for exact arithmetic, `num-traits` for the numeric predicates, `statrs` for the normal distribution, `anyhow` for errors, `rustyline`+`clap` for the binary.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-04-reliable-core-design.md`. Read it before starting.
- Errors stay `anyhow` strings in this stage. Every new condition gets its own distinct message — never another reuse of `MALFORMED_ERR`. Typed errors are Stage 2.
- Undefined variables keep evaluating to `0`. Prefix factorial (`!5` → `120`) stays accepted. Both are deliberate exclusions, not oversights.
- Every task ends with `cargo test` green and `cargo fmt --check` clean.
- `cargo clippy --all-targets` must not gain warnings. The baseline on `master` is 40, one of which is `this function has too many lines (243/100)` for `resolve()` and must be gone by the end of Task 1.
- Commit messages: imperative subject line, no tool attribution of any kind, no `Co-Authored-By` trailer.
- Branch: `production-ready-core`, already created, already holding the spec.
- Do not add dependencies. Everything needed is already in `Cargo.toml`.

---

### Task 1: Extract function evaluation into its own module

Component D of the spec. This is a **pure move with no behaviour change**: the existing 64 tests are the proof, and they must be green before and after with no edits to any test.

**Files:**
- Create: `src/functions.rs`
- Modify: `src/lib.rs` (add the module declaration)
- Modify: `src/rpn_resolver.rs` — the `Token::Function` arm at `:186-299`, the helpers at `:488-517`, and the three error statics at `:21-32`

**Interfaces:**
- Consumes: nothing.
- Produces, all `pub(crate)` in `crate::functions`:
  - `eval(fun: MathFunction, value: Number, result_stack: &mut VecDeque<Number>, var_stack: &mut VecDeque<Option<String>>) -> anyhow::Result<Number>`
  - `number_to_f64(value: &Number, error_message: &'static str) -> anyhow::Result<f64>`
  - `decimal_from_f64(value: f64, error_message: &'static str) -> anyhow::Result<Number>`
  - `number_to_rational(value: Number) -> BigRational`
  - `to_decimal_number(value: Number) -> Number`

- [ ] **Step 1: Record the baseline**

Run: `cargo test 2>&1 | grep "^test result"`
Expected: four lines, `31 passed`, `0 passed`, `28 passed`, `5 passed; 0 failed; 2 ignored`. Write the numbers down — they must be identical at Step 6.

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning:"`
Expected: `40`.

- [ ] **Step 2: Widen the three error statics that the new module needs**

In `src/rpn_resolver.rs`, change the visibility of exactly these three, leaving the others untouched:

```rust
pub(crate) static MALFORMED_ERR: &str = "Runtime Error: The mathematical expression is malformed.";
pub(crate) static INVALID_FUNCTION_RESULT_ERR: &str = "Runtime error: Function result is not a real number.";
pub(crate) static FLOAT_EVAL_TOO_LARGE_ERR: &str = "Runtime error: Operand is too large for floating-point evaluation.";
```

They are imported by the new module rather than copied. Duplicating an error string is how two messages silently drift apart.

- [ ] **Step 3: Create `src/functions.rs`**

Write this header, then **move** — cut, do not retype — the body of the `match fun { … }` expression currently at `src/rpn_resolver.rs:194-296` into the `match` below, and the four helper functions currently at `src/rpn_resolver.rs:488-517` beneath it, changing each helper's declaration from `fn name(…)` to `pub(crate) fn name(…)` and dropping the `Self::` qualifier from every internal call.

```rust
//! Evaluation of the built-in mathematical functions, together with the
//! numeric conversions they rely on.
//!
//! Split out of [`crate::rpn_resolver`], which owns the shunting-yard
//! translation and the evaluation loop; this module owns what happens when
//! that loop meets a [`MathFunction`].

use crate::rpn_resolver::{FLOAT_EVAL_TOO_LARGE_ERR, INVALID_FUNCTION_RESULT_ERR, MALFORMED_ERR};
use crate::token::{MathFunction, Number};
use anyhow::anyhow;
use num::{Integer, Signed};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};
use statrs::distribution::{Continuous, ContinuousCDF, Normal};
use std::collections::VecDeque;

/// Evaluates `fun` against `value`, the operand already popped by the caller.
///
/// A two-argument function pops its second operand from `result_stack` itself,
/// keeping `var_stack` in step. Both stacks belong to the evaluation loop in
/// [`crate::rpn_resolver::RpnResolver::resolve`].
pub(crate) fn eval(
    fun: MathFunction,
    value: Number,
    result_stack: &mut VecDeque<Number>,
    var_stack: &mut VecDeque<Option<String>>,
) -> anyhow::Result<Number> {
    let result = match fun {
        // ... moved verbatim from rpn_resolver.rs:194-296 ...
    };
    Ok(result)
}
```

Three mechanical adjustments while moving, and nothing else:
1. `MathFunction::Sin => …` etc. match on `fun` by value now, not on `&fun`, so the arms lose no patterns but the leading `*` disappears if present.
2. Calls of the form `Self::decimal_from_f64(…)`, `Self::number_to_f64(…)`, `Self::number_to_rational(…)`, `Self::to_decimal_number(…)` become bare `decimal_from_f64(…)` and so on.
3. `result_stack` and `var_stack` are already the parameter names used in the moved code, so the `Max` and `Min` arms need no edit.

- [ ] **Step 4: Declare the module and rewrite the call site**

In `src/lib.rs`, after the existing `/// Parser` / `pub mod parser;` pair, add:

```rust
/// Built-in function evaluation
mod functions;
```

It is private: nothing outside the crate calls it.

In `src/rpn_resolver.rs`, add `use crate::functions;` to the existing `use crate::{…}` group, delete the four helper functions at `:488-517`, and replace the whole `Token::Function(fun) => { … }` arm with:

```rust
                Token::Function(fun) => {
                    let value: Number = result_stack.pop_back().ok_or(anyhow!(
                        "{} {}",
                        MALFORMED_ERR,
                        "Wrong use of function"
                    ))?;
                    var_stack.pop_back();

                    let result = functions::eval(*fun, value, &mut result_stack, &mut var_stack)?;
                    result_stack.push_back(result);
                    var_stack.push_back(None);
                }
```

`power` and `power_integer` still call `number_to_f64` and `decimal_from_f64`, so add `use crate::functions::{decimal_from_f64, number_to_f64};` and drop the `Self::` prefix at those three call sites (`:532`, `:533`, `:534`).

- [ ] **Step 5: Verify nothing moved but the code**

Run: `cargo test 2>&1 | grep "^test result"`
Expected: exactly the four lines recorded in Step 1. **No test file was edited** — `git status` must show no changes under `tests/`.

Run: `cargo clippy --all-targets 2>&1 | grep "too many lines"`
Expected: no output. `resolve()` is now roughly 140 lines.

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning:"`
Expected: 40 or fewer. If it grew, fix what was introduced before committing.

Run: `cargo fmt --check`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/functions.rs src/lib.rs src/rpn_resolver.rs
git commit -m "Move function evaluation into its own module

resolve() was 243 lines and clippy flagged it. The MathFunction dispatch
and the numeric conversions it needs now live in src/functions.rs, leaving
the shunting-yard translation and the evaluation loop in rpn_resolver.

Pure move: no behaviour change, no test edited, same 64 tests green."
```

---

### Task 2: Canonical `Number`

Component A of the spec. Closes defect 1 (`abs(-3)!` fails) and defect 2 (`PartialEq`/`PartialOrd` disagree).

**Invariant introduced: `Number::DecimalNumber` never holds a rational whose denominator is 1.**

**Files:**
- Modify: `src/token.rs` — the derive at `:12`, `Div` at `:335-354`, `apply_functional_token_operation` at `:294-309`, `tokenize` at `:222-224`; add an `impl Number` block and a `PartialEq` impl
- Modify: `src/functions.rs` — `decimal_from_f64`, and delete `to_decimal_number` along with its five call sites
- Modify: `src/rpn_resolver.rs` — the `Operator::Fac` arm, `integer_exponent` and `power_integer`
- Modify: `src/session.rs` — `init_local_heap`, `setf`
- Test: `tests/integration_tests.rs` — the `resolve_decimal!` macro at `:17-28`, plus two new tests
- Test: `src/token.rs` — new unit tests in the existing `mod tests`

**Interfaces:**
- Consumes: `crate::functions::{decimal_from_f64, to_decimal_number}` from Task 1.
- Produces:
  - `Number::decimal(value: BigRational) -> Number` — public, the only sanctioned way to build a decimal
  - `Number::as_integer(&self) -> Option<BigInt>` — public, `Some` when the value is a whole number
  - `to_decimal_number` no longer exists

- [ ] **Step 1: Write the failing tests**

Append to `mod tests` in `src/token.rs`:

```rust
    #[test]
    fn test_decimal_constructor_degrades_integral_rationals() {
        // 4/2 reduces to 2, which is an integer: it must not stay tagged as decimal.
        let n = Number::decimal(BigRational::new(BigInt::from(4), BigInt::from(2)));
        assert_eq!(n, Number::NaturalNumber(BigInt::from(2)));
        assert!(matches!(n, Number::NaturalNumber(_)));

        // 1/2 is not integral and must stay decimal.
        let half = Number::decimal(BigRational::new(BigInt::from(1), BigInt::from(2)));
        assert!(matches!(half, Number::DecimalNumber(_)));
    }

    #[test]
    fn test_as_integer_reads_the_value_not_the_tag() {
        assert_eq!(
            Number::NaturalNumber(BigInt::from(7)).as_integer(),
            Some(BigInt::from(7))
        );
        // Built by hand, bypassing the constructor: the value is still integral.
        assert_eq!(
            Number::DecimalNumber(BigRational::from_integer(BigInt::from(7))).as_integer(),
            Some(BigInt::from(7))
        );
        assert_eq!(
            Number::DecimalNumber(BigRational::new(BigInt::from(3), BigInt::from(2))).as_integer(),
            None
        );
    }

    #[test]
    fn test_eq_agrees_with_partial_cmp_across_variants() {
        use std::cmp::Ordering;
        let pairs = [
            (
                Number::NaturalNumber(BigInt::from(2)),
                Number::DecimalNumber(BigRational::from_integer(BigInt::from(2))),
            ),
            (
                Number::NaturalNumber(BigInt::from(-3)),
                Number::DecimalNumber(BigRational::new(BigInt::from(-6), BigInt::from(2))),
            ),
            (
                Number::NaturalNumber(BigInt::from(2)),
                Number::DecimalNumber(BigRational::new(BigInt::from(5), BigInt::from(2))),
            ),
        ];
        for (a, b) in pairs {
            let equal_by_eq = a == b;
            let equal_by_ord = a.partial_cmp(&b) == Some(Ordering::Equal);
            assert_eq!(
                equal_by_eq, equal_by_ord,
                "PartialEq and PartialOrd disagree on {a} vs {b}"
            );
        }
    }
```

Append to `tests/integration_tests.rs`:

```rust
#[test]
fn test_factorial_accepts_integral_results_of_functions() {
    // These all produced "Factorial is only defined for non-negative integers"
    // before the Number invariant, because the functions tagged an integral
    // result as decimal and the factorial branched on the tag.
    resolve_natural!("abs(-3)!", 6);
    resolve_natural!("floor(2.5)!", 2);
    resolve_natural!("max(3,2)!", 6);
    resolve_natural!("round(2.4)!", 2);
    resolve_natural!("(6/3)!", 2);
}

#[test]
fn test_integral_results_are_natural_numbers() {
    let session = Session::init();
    for expr in ["6/3", "floor(3.7)", "exp(0)", "max(1,2)", "sqrt(16)"] {
        let mut resolver = session.process(expr);
        let result = resolver.resolve().unwrap();
        assert!(
            matches!(result, Number::NaturalNumber(_)),
            "{expr} produced {result:?}, expected a NaturalNumber"
        );
    }
}

#[test]
fn test_non_integral_results_stay_decimal() {
    let session = Session::init();
    for expr in ["1/3", "abs(-2.5)", "2^-3", "sqrt(2)"] {
        let mut resolver = session.process(expr);
        let result = resolver.resolve().unwrap();
        assert!(
            matches!(result, Number::DecimalNumber(_)),
            "{expr} produced {result:?}, expected a DecimalNumber"
        );
    }
}
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo test test_decimal_constructor_degrades_integral_rationals`
Expected: FAIL — `no function or associated item named 'decimal' found`.

Run: `cargo test --test integration_tests test_factorial_accepts_integral_results_of_functions`
Expected: FAIL — the assertion reports the factorial error message.

Run: `cargo test test_eq_agrees_with_partial_cmp_across_variants`
Expected: FAIL — `PartialEq and PartialOrd disagree on 2 vs 2`. (This one is a unit test inside the library, so it takes no `--test` flag.)

- [ ] **Step 3: Add the constructor, the accessor and value-based equality**

In `src/token.rs`, remove `PartialEq` from the derive on `Number` at `:12`:

```rust
#[derive(Debug, Clone)]
pub enum Number {
```

Add, next to the existing `impl Display for Number`:

```rust
impl Number {
    /// Builds a decimal number, degrading to [`Number::NaturalNumber`] when the
    /// rational turns out to be a whole number.
    ///
    /// This is the only sanctioned way to build a [`Number::DecimalNumber`]:
    /// it upholds the invariant that a decimal never carries a denominator of 1,
    /// so a given mathematical value has exactly one representation.
    #[must_use]
    pub fn decimal(value: BigRational) -> Number {
        if value.denom().is_one() {
            Number::NaturalNumber(value.to_integer())
        } else {
            Number::DecimalNumber(value)
        }
    }

    /// Returns the integral value of this number, or [`None`] when it has a
    /// fractional part.
    ///
    /// The decimal arm matters only for values built by hand from outside the
    /// crate, which can bypass [`Number::decimal`]; internally the invariant
    /// makes it unreachable.
    #[must_use]
    pub fn as_integer(&self) -> Option<BigInt> {
        match self {
            Number::NaturalNumber(v) => Some(v.clone()),
            Number::DecimalNumber(v) if v.denom().is_one() => Some(v.to_integer()),
            Number::DecimalNumber(_) => None,
        }
    }
}

/// Equality by mathematical value, so that it agrees with [`PartialOrd`].
///
/// The derived implementation compared enum variants, which made
/// `NaturalNumber(2) == DecimalNumber(2/1)` false while `>=` reported true —
/// a violation of the `PartialOrd` contract that generic code relies on.
impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::NaturalNumber(a), Number::NaturalNumber(b)) => a == b,
            (Number::DecimalNumber(a), Number::DecimalNumber(b)) => a == b,
            (Number::NaturalNumber(a), Number::DecimalNumber(b))
            | (Number::DecimalNumber(b), Number::NaturalNumber(a)) => {
                BigRational::from(a.clone()) == *b
            }
        }
    }
}
```

- [ ] **Step 4: Route every decimal construction through the constructor**

`src/token.rs`, `Div` at `:335-354` — all four arms:

```rust
impl Div for Number {
    type Output = Number;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => {
                Number::decimal(BigRational::new(v1, v2))
            }
            (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => {
                Number::decimal(BigRational::from(v1) / v2)
            }
            (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => {
                Number::decimal(v1 / BigRational::from(v2))
            }
            (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => Number::decimal(v1 / v2),
        }
    }
}
```

`src/token.rs`, `apply_functional_token_operation` at `:294-309` — the three arms that produce a decimal:

```rust
        (Number::NaturalNumber(v1), Number::NaturalNumber(v2)) => Number::NaturalNumber(nf(v1, v2)),
        (Number::NaturalNumber(v1), Number::DecimalNumber(v2)) => {
            Number::decimal(df(BigRational::from(v1), v2))
        }
        (Number::DecimalNumber(v1), Number::NaturalNumber(v2)) => {
            Number::decimal(df(v1, BigRational::from(v2)))
        }
        (Number::DecimalNumber(v1), Number::DecimalNumber(v2)) => Number::decimal(df(v1, v2)),
```

`src/token.rs`, `tokenize` at `:222-224`:

```rust
        if let Some(v) = parse_decimal_literal(t) {
            return Some(Token::Operand(Number::decimal(v)));
        }
```

`src/functions.rs`, `decimal_from_f64`:

```rust
pub(crate) fn decimal_from_f64(value: f64, error_message: &'static str) -> anyhow::Result<Number> {
    if !value.is_finite() {
        return Err(anyhow!(error_message));
    }

    BigRational::from_float(value)
        .map(Number::decimal)
        .ok_or_else(|| anyhow!(error_message))
}
```

`src/session.rs` — `init_local_heap` builds the five constants and `setf` stores a float. Replace every `Number::DecimalNumber(x)` with `Number::decimal(x)` in both. None of the five constants is integral, so the constants are unaffected in practice; the change keeps a single construction path.

`src/rpn_resolver.rs`, `power_integer` at `:537-577` — the three `Number::DecimalNumber(…)` constructions become `Number::decimal(…)`:

```rust
                    let value = Self::pow_big_int(base, exponent);
                    Ok(Number::decimal(BigRational::new(BigInt::one(), value)))
```

```rust
                let value = Self::pow_big_rational(base, exponent);
                if is_negative {
                    Ok(Number::decimal(value.recip()))
                } else {
                    Ok(Number::decimal(value))
                }
```

- [ ] **Step 5: Delete `to_decimal_number` and fix the factorial**

In `src/functions.rs`, delete the `to_decimal_number` function and unwrap its five call sites, which become the value itself:

- `Abs`: `match value { Number::NaturalNumber(v) => Number::NaturalNumber(v.abs()), Number::DecimalNumber(v) => Number::decimal(v.abs()) }`
- `Max`: `if value >= value2 { value } else { value2 }`
- `Min`: `if value <= value2 { value } else { value2 }`
- `Floor`, `Ceil`, `Round`: `Number::NaturalNumber(…)` directly, dropping the wrapper

In `src/rpn_resolver.rs`, replace the whole `Operator::Fac` arm:

```rust
                        Operator::Fac => {
                            // Factorial is defined on non-negative integers. It asks the
                            // value, not the enum tag: floor(2.5) and 6/3 are integers.
                            let n = right_value
                                .as_integer()
                                .ok_or_else(|| anyhow!(FACTORIAL_NATURAL_ERR))?;
                            if n < BigInt::zero() {
                                return Err(anyhow!(FACTORIAL_NATURAL_ERR));
                            }
                            let n = n.to_u64().ok_or_else(|| {
                                anyhow!("Runtime Error: Factorial operand is too large")
                            })?;
                            let res = Self::factorial_helper(n.into());
                            result_stack.push_back(Number::NaturalNumber(res.into()));
                            var_stack.push_back(None);
                        }
```

Delete `integer_exponent` at `:519-525` and change `power` to use the accessor:

```rust
    fn power(left_value: Number, right_value: Number) -> anyhow::Result<Number> {
        if let Some(exponent) = right_value.as_integer() {
            return Self::power_integer(left_value, exponent);
        }

        let base = number_to_f64(&left_value, POWER_TOO_LARGE_ERR)?;
        let exponent = number_to_f64(&right_value, POWER_TOO_LARGE_ERR)?;
        decimal_from_f64(base.powf(exponent), INVALID_POWER_ERR)
    }
```

- [ ] **Step 6: Stop the test macro asserting the enum variant**

In `tests/integration_tests.rs:17-28`, remove the `matches!` line from `resolve_decimal!` and say why:

```rust
/// Asserts the numeric value of an expression, within 1e-10.
///
/// It deliberately does not assert which `Number` variant came back: under the
/// canonicalisation invariant an integral result is a `NaturalNumber`, and
/// which side of that line an expression falls on is asserted once, on purpose,
/// by `test_integral_results_are_natural_numbers`.
macro_rules! resolve_decimal {
    ($expr:expr, $expected:expr) => {{
        let session = Session::init();
        let mut resolver = session.process($expr);
        let result = resolver.resolve().unwrap();
        let res_f: f64 = result.try_into().unwrap();
        assert!((res_f - $expected).abs() < 1e-10);
    }};
    () => {
        panic!("Expected a decimal number, but got an invalid result.");
    };
}
```

Note `result.try_into()` no longer needs the `clone()`, since `result` is not used afterwards.

- [ ] **Step 7: Run the whole suite**

Run: `cargo test`
Expected: all green, with 3 new unit tests in `token.rs` and 3 new integration tests, so `34 passed` and `31 passed`.

If `test_max_min` in `src/rpn_resolver.rs` fails, read the failure before touching it: it uses `assert_eq!` against `DecimalNumber(2.0)`, which value-based equality should now satisfy against `NaturalNumber(2)`. A failure there means `PartialEq` is wrong, not the test.

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning:"`
Expected: no higher than after Task 1.

Run: `cargo fmt --check`
Expected: clean.

- [ ] **Step 8: Commit**

```bash
git add -A src tests
git commit -m "Give every numeric value one representation

NaturalNumber(2) and DecimalNumber(2/1) meant the same number, and code
branched on the tag rather than the value. Two defects followed: factorial
rejected abs(-3), floor(2.5) and max(3,2), because those functions forced
the decimal tag onto integral results; and Number violated the PartialOrd
contract, with 2 == 2/1 false while 2 >= 2/1 was true.

Number::decimal is now the only way to build a decimal and degrades an
integral rational to NaturalNumber, Number::as_integer replaces the two
copies of the is-this-a-whole-number test, and PartialEq compares values."
```

---

### Task 3: Size limits

Component B of the spec. Closes defect 3: `999999999!` and `10^100000000` never return.

**Files:**
- Create: `src/limits.rs`
- Modify: `src/lib.rs` (declare the module, publicly)
- Modify: `src/session.rs` (a `limits` field, `with_limits`, pass it to the resolver)
- Modify: `src/rpn_resolver.rs` (a `limits` field, the arithmetic arms, `power_integer`, the factorial arm, the `test_resolve` unit test's struct literal)
- Test: `tests/integration_tests.rs`

**Interfaces:**
- Consumes: `Number::as_integer` from Task 2.
- Produces:
  - `limits::Limits { pub max_value_bits: u64 }`, `Copy`, with `Default` giving `1 << 20`
  - `limits::size_in_bits(value: &Number) -> u64`
  - `limits::check_size(value: &Number, limits: Limits) -> anyhow::Result<()>`
  - `limits::check_predicted_size(predicted_bits: u128, limits: Limits) -> anyhow::Result<()>`
  - `Session::with_limits(limits: Limits) -> Session`
  - `RpnResolver::parse_with_borrowed_heap(exp, borrowed_heap, limits)` — one extra parameter, a breaking change to a public method

- [ ] **Step 1: Write the failing tests**

Append to `tests/integration_tests.rs`:

```rust
#[test]
fn test_oversized_factorial_is_refused_not_computed() {
    // Before the limit this did not return at all. The test is its own alarm:
    // if the guard stops working, the suite hangs here instead of failing.
    let session = Session::init();
    let mut resolver = session.process("999999999!");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("size limit"), "message was: {err}");
}

#[test]
fn test_oversized_power_is_refused_not_computed() {
    let session = Session::init();
    let mut resolver = session.process("10^100000000");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("size limit"), "message was: {err}");
}

#[test]
fn test_legitimate_big_values_still_pass_the_default_limit() {
    resolve_natural!("2^64", 18_446_744_073_709_551_616_i128);
    let session = Session::init();
    let mut resolver = session.process("1000!");
    // 1000! needs about 8530 bits, comfortably inside the default budget.
    assert!(resolver.resolve().is_ok());
}

#[test]
fn test_the_limit_is_configurable() {
    let session = Session::with_limits(Limits { max_value_bits: 64 });
    let mut resolver = session.process("2^100");
    assert!(resolver.resolve().is_err(), "2^100 needs 101 bits, over a 64-bit budget");

    let mut small = session.process("2^10");
    assert!(small.resolve().is_ok());
}

#[test]
fn test_growth_through_multiplication_is_caught() {
    // 2^3000 occupies 3001 bits and is admitted; squaring it needs 6001, which
    // is not. Each step is individually under budget only until it is not.
    let session = Session::with_limits(Limits { max_value_bits: 4096 });
    let mut resolver = session.process("x=2^3000; x*x");
    assert!(resolver.resolve().is_err(), "the product needs 6001 bits");
}
```

Add `use yarer::limits::Limits;` to the imports at the top of the file.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --test integration_tests test_the_limit_is_configurable`
Expected: FAIL to compile — `unresolved import 'yarer::limits'`.

Do **not** run `test_oversized_factorial_is_refused_not_computed` yet: without the guard it does not terminate.

- [ ] **Step 3: Create `src/limits.rs`**

```rust
//! Bounds on how large a value an evaluation may produce.
//!
//! The strategy is to predict the size of a result and refuse before computing
//! it, rather than computing under a timeout: no threads, no interruption, and
//! a decision that is deterministic and instantaneous.

use crate::token::Number;
use anyhow::anyhow;

/// Resource bounds applied while evaluating an expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The largest value any intermediate or final result may occupy, in bits.
    pub max_value_bits: u64,
}

impl Default for Limits {
    /// 1 Mibit, roughly 315_000 decimal digits.
    fn default() -> Self {
        Limits {
            max_value_bits: 1 << 20,
        }
    }
}

/// The size of a value in bits: for a rational, numerator plus denominator.
#[must_use]
pub fn size_in_bits(value: &Number) -> u64 {
    match value {
        Number::NaturalNumber(v) => v.bits(),
        Number::DecimalNumber(v) => v.numer().bits() + v.denom().bits(),
    }
}

/// Rejects a value that has already been computed and turned out too large.
///
/// # Errors
/// When the value exceeds `limits.max_value_bits`.
pub fn check_size(value: &Number, limits: Limits) -> anyhow::Result<()> {
    check_predicted_size(u128::from(size_in_bits(value)), limits)
}

/// Rejects a computation whose result was predicted to be too large, before it runs.
///
/// # Errors
/// When `predicted_bits` exceeds `limits.max_value_bits`.
pub fn check_predicted_size(predicted_bits: u128, limits: Limits) -> anyhow::Result<()> {
    if predicted_bits > u128::from(limits.max_value_bits) {
        return Err(anyhow!(
            "Runtime error: the result would need about {predicted_bits} bits, over the size limit of {} bits.",
            limits.max_value_bits
        ));
    }
    Ok(())
}

/// Predicts the bit length of `n!` without computing it, via Stirling:
/// `log2(n!) ≈ n·log2(n) − 1.44·n`.
#[must_use]
pub fn predicted_factorial_bits(n: u64) -> u128 {
    if n < 2 {
        return 1;
    }
    let n_f = n as f64;
    let bits = n_f.mul_add(n_f.log2(), -1.442_695_040_888_963_4 * n_f);
    // Round up and never report less than one bit.
    bits.max(1.0).ceil() as u128
}

/// An upper bound on the bit length of `base^exponent` for an integral exponent.
///
/// It uses `bits(base)` where `log2(base)` would be exact, so it overestimates by
/// up to a factor of two for small bases — `2^100` is predicted at 200 bits and
/// occupies 101. A guard that errs toward refusing is the right direction to err in,
/// and the discrepancy only matters within a factor of two of the budget.
#[must_use]
pub fn predicted_power_bits(base: &Number, exponent_magnitude: u64) -> u128 {
    let base_bits = size_in_bits(base).max(1);
    u128::from(base_bits) * u128::from(exponent_magnitude)
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    #[test]
    fn test_size_of_a_rational_counts_both_halves() {
        let third = Number::DecimalNumber(BigRational::new(BigInt::from(1), BigInt::from(3)));
        assert_eq!(size_in_bits(&third), 1 + 2);
    }

    #[test]
    fn test_factorial_prediction_is_in_the_right_ballpark() {
        // 1000! is 8529 bits; Stirling must land close and never far under.
        let predicted = predicted_factorial_bits(1000);
        assert!(
            (8000..=9200).contains(&predicted),
            "predicted {predicted} bits for 1000!"
        );
    }

    #[test]
    fn test_power_prediction_multiplies_base_size_by_exponent() {
        let ten = Number::NaturalNumber(BigInt::from(10));
        assert_eq!(predicted_power_bits(&ten, 100), 400);
    }

    #[test]
    fn test_check_rejects_above_the_budget_and_accepts_at_it() {
        let limits = Limits { max_value_bits: 64 };
        assert!(check_predicted_size(64, limits).is_ok());
        assert!(check_predicted_size(65, limits).is_err());
    }
}
```

- [ ] **Step 4: Thread the limits through `Session` and `RpnResolver`**

`src/lib.rs`:

```rust
/// Evaluation limits
pub mod limits;
```

`src/session.rs` — add the field and the constructor, and pass the value on:

```rust
pub struct Session {
    variable_heap: Rc<RefCell<HashMap<String, Number>>>,
    limits: Limits,
}
```

```rust
    #[must_use]
    pub fn init() -> Session {
        Session::with_limits(Limits::default())
    }

    /// Builds a session whose evaluations are bound by `limits`.
    #[must_use]
    pub fn with_limits(limits: Limits) -> Session {
        Session {
            variable_heap: Rc::new(RefCell::new(Session::init_local_heap())),
            limits,
        }
    }
```

```rust
    #[must_use]
    pub fn process<'a>(&'a self, line: &'a str) -> RpnResolver<'a> {
        let clone = Rc::clone(&self.variable_heap);
        RpnResolver::parse_with_borrowed_heap(line, clone, self.limits)
    }
```

`src/rpn_resolver.rs` — the struct, both construction arms, and the signature:

```rust
pub struct RpnResolver<'a> {
    rpn_expr: VecDeque<Token<'a>>,
    local_heap: Rc<RefCell<HashMap<String, Number>>>,
    build_error: Option<String>,
    limits: Limits,
}
```

```rust
    pub fn parse_with_borrowed_heap<'a>(
        exp: &'a str,
        borrowed_heap: Rc<RefCell<HashMap<String, Number>>>,
        limits: Limits,
    ) -> RpnResolver<'a> {
```

Both the `Ok` and the `Err` arm of that function gain `limits,` in the struct literal. The `test_resolve` unit test at the bottom of the file builds an `RpnResolver` literally and needs `limits: Limits::default(),` too.

`parse_with_borrowed_heap` is public, but `src/session.rs:42` is its only caller in the repository — verified by grep across sources, tests and docs. Nothing else needs updating.

- [ ] **Step 5: Apply the checks**

Add the imports first: `use crate::limits::{self, Limits};` in `src/rpn_resolver.rs`, and `use crate::limits::Limits;` in `src/session.rs`.

In `resolve()`, the four arithmetic arms check the result they just produced. The loop runs over `&self.rpn_expr`, which borrows `self`, so a `self.check(…)` method would not borrow-check inside it. Copy the limits into a local before the loop and call the free function:

```rust
        let limits = self.limits;
```

then each arm becomes, for example:

```rust
                        Operator::Add => {
                            let value = left_value + right_value;
                            limits::check_size(&value, limits)?;
                            result_stack.push_back(value);
                            var_stack.push_back(None);
                        }
```

Apply the same shape to `Sub`, `Mul` and `Div` — for `Div`, after the existing divide-by-zero guard.

The factorial arm checks before computing, right after `to_u64`:

```rust
                            let n = n.to_u64().ok_or_else(|| {
                                anyhow!("Runtime Error: Factorial operand is too large")
                            })?;
                            limits::check_predicted_size(
                                limits::predicted_factorial_bits(n),
                                limits,
                            )?;
                            let res = Self::factorial_helper(n.into());
```

`power_integer` needs the limits passed in, since it is an associated function. Change its signature to `fn power_integer(base: Number, exponent: BigInt, limits: Limits)`, have `power` take and forward a `limits: Limits` parameter, and update the single call site in `resolve()` to `Self::power(left_value, right_value, limits)?`. Inside `power_integer`, after computing `exponent` as a `BigUint`, predict before computing:

```rust
        let exponent_magnitude = exponent
            .to_u64()
            .ok_or_else(|| anyhow!(INVALID_POWER_ERR))?;
        limits::check_predicted_size(
            limits::predicted_power_bits(&base, exponent_magnitude),
            limits,
        )?;
```

Place it after the `is_negative`/`magnitude` computation and before the `match base`. An exponent that does not fit in a `u64` is refused outright: any base of at least one bit raised to it exceeds every budget.

- [ ] **Step 6: Run the tests**

Run: `cargo test`
Expected: all green. The two "oversized" tests now complete in milliseconds; if either hangs, the guard is not on the path the expression takes — investigate rather than raising the limit.

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning:"`, then `cargo fmt --check`
Expected: no growth, clean.

- [ ] **Step 7: Measure the default, do not assert it**

The spec requires the default to come from a timing rather than a guess. Find the slowest expression the default still accepts and time it:

```bash
cargo build --release
time (printf '200000!\nquit\n' | ./target/release/yarer -q)
time (printf '2^1000000\nquit\n' | ./target/release/yarer -q)
```

`200000!` predicts roughly 3.2 Mibit and is therefore already refused by the 1 Mibit default; walk `n` down until it is accepted, and time that. If the slowest accepted case takes more than a second, lower `max_value_bits` until it does not, then record the final figure, the expression and the timing in the spec under component B, replacing the sentence that says the default is provisional.

- [ ] **Step 8: Commit**

```bash
git add -A src tests docs
git commit -m "Refuse results that are too large instead of computing forever

999999999! and 10^100000000 never returned: the only guard on the factorial
was that its operand fit in a u64, and none at all on exponentiation.

Both are now predicted before being computed - Stirling for the factorial,
base size times exponent for the power - and the four arithmetic operators
check the result they produced, so growth through repeated multiplication
is caught too. The budget is one knob, Limits::max_value_bits, on by
default and configurable through Session::with_limits."
```

---

### Task 4: Function arity

Component C of the spec. Closes defect 4, and makes parentheses mandatory after a function name.

**Files:**
- Modify: `src/token.rs` (`MathFunction::arity`)
- Modify: `src/rpn_resolver.rs` (`reverse_polish_notation`, plus three new error statics)
- Test: `tests/integration_tests.rs`, `src/token.rs`

**Interfaces:**
- Consumes: nothing from Tasks 2 and 3.
- Produces: `MathFunction::arity(self) -> u8`.

- [ ] **Step 1: Write the failing tests**

Append to `mod tests` in `src/token.rs`:

```rust
    #[test]
    fn test_arity_of_the_two_argument_functions() {
        assert_eq!(MathFunction::Max.arity(), 2);
        assert_eq!(MathFunction::Min.arity(), 2);
        assert_eq!(MathFunction::Sin.arity(), 1);
        assert_eq!(MathFunction::Cdf.arity(), 1);
    }
```

Append to `tests/integration_tests.rs`:

```rust
#[test]
fn test_wrong_arity_is_diagnosed_by_name() {
    let session = Session::init();
    for (expr, expected, given) in [("max(1)", 2, 1), ("max(1,2,3)", 2, 3), ("sin(1,2)", 1, 2)] {
        let mut resolver = session.process(expr);
        let err = resolver.resolve().unwrap_err().to_string();
        assert!(
            err.contains(&format!("expects {expected}")) && err.contains(&format!("{given} given")),
            "{expr} reported: {err}"
        );
    }
}

#[test]
fn test_empty_argument_list_is_diagnosed() {
    let session = Session::init();
    let mut resolver = session.process("max()");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("0 given"), "message was: {err}");
}

#[test]
fn test_comma_outside_a_function_call_is_diagnosed() {
    let session = Session::init();
    let mut resolver = session.process("(1,2)");
    let err = resolver.resolve().unwrap_err().to_string();
    assert!(err.contains("function call"), "message was: {err}");
}

#[test]
fn test_a_function_name_requires_parentheses() {
    let session = Session::init();
    for expr in ["sin 5", "sqrt 16", "cos"] {
        let mut resolver = session.process(expr);
        let err = resolver.resolve().unwrap_err().to_string();
        assert!(err.contains("must be followed by"), "{expr} reported: {err}");
    }
    // The parenthesised form is untouched.
    resolve_decimal!("sin(5)", -0.9589242746631385);
}

#[test]
fn test_nested_and_multi_argument_calls_still_work() {
    resolve_natural!("min(max(2,3),max(5,1))", 3);
    resolve_natural!("max(1+2,3*4)-min(10,5)", 7);
    resolve_natural!("max(1,(2+3))", 5);
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --test integration_tests test_wrong_arity_is_diagnosed_by_name`
Expected: FAIL — the message is the generic "The mathematical expression is malformed."

Run: `cargo test test_arity_of_the_two_argument_functions`
Expected: FAIL — `no method named 'arity'`.

- [ ] **Step 3: Declare the arity**

In `src/token.rs`, next to the `MathFunction` enum:

```rust
impl MathFunction {
    /// How many arguments this function takes.
    ///
    /// [`MathFunction::None`] is unreachable — [`Token::get_some`] never yields
    /// it — and reports 1 rather than panicking, so that no input can reach a
    /// panic through this path.
    #[must_use]
    pub const fn arity(self) -> u8 {
        match self {
            MathFunction::Max | MathFunction::Min => 2,
            _ => 1,
        }
    }
}
```

- [ ] **Step 4: Count arguments in the shunting yard**

In `src/rpn_resolver.rs`, add the three error statics beside the existing ones:

```rust
static COMMA_OUTSIDE_CALL_ERR: &str =
    "Parse Error: ',' is only valid between the arguments of a function call.";
static UNBALANCED_BRACKET_ERR: &str = "Parse Error: Unbalanced brackets.";
```

Add the frame type just above `impl RpnResolver`:

```rust
/// One entry per open bracket, recording whether that bracket opens a function
/// call and how many arguments it has seen so far.
struct BracketFrame {
    function: Option<MathFunction>,
    commas: usize,
    has_content: bool,
}

impl BracketFrame {
    /// An empty pair of brackets carries zero arguments; otherwise there is one
    /// more argument than there are separators.
    fn argument_count(&self) -> usize {
        if self.has_content {
            self.commas + 1
        } else {
            0
        }
    }
}
```

In `reverse_polish_notation`, declare two locals next to `operators_stack`:

```rust
        let mut bracket_stack: Vec<BracketFrame> = Vec::new();
        let mut pending_function: Option<MathFunction> = None;
```

At the very top of the `for t in infix_stack` loop, before the `match`, enforce the parenthesis rule and record that the innermost bracket has content:

```rust
            if let Some(fun) = pending_function {
                if !matches!(t, Token::Bracket(token::Bracket::Open)) {
                    return Err(anyhow!(
                        "Parse Error: Function '{}' must be followed by '('.",
                        fun.to_string().to_lowercase()
                    ));
                }
            }

            if !matches!(t, Token::Comma | Token::Bracket(token::Bracket::Close)) {
                if let Some(frame) = bracket_stack.last_mut() {
                    frame.has_content = true;
                }
            }
```

Then extend four arms of the existing `match`:

`Token::Function(f)` — record it as pending in addition to pushing it:

```rust
                Token::Function(f) => {
                    pending_function = Some(f);
                    operators_stack.push(t.clone());
                }
```

`Token::Bracket(Open)` — open a frame, consuming any pending function:

```rust
                Token::Bracket(token::Bracket::Open) => {
                    bracket_stack.push(BracketFrame {
                        function: pending_function.take(),
                        commas: 0,
                        has_content: false,
                    });
                    operators_stack.push(t.clone());
                }
```

`Token::Bracket(Close)` — close the frame and check, before the existing operator-popping loop:

```rust
                Token::Bracket(token::Bracket::Close) => {
                    let frame = bracket_stack.pop().ok_or_else(|| anyhow!(UNBALANCED_BRACKET_ERR))?;
                    if let Some(fun) = frame.function {
                        let given = frame.argument_count();
                        let expected = usize::from(fun.arity());
                        if given != expected {
                            return Err(anyhow!(
                                "Parse Error: Function '{}' expects {} argument(s), {} given.",
                                fun.to_string().to_lowercase(),
                                expected,
                                given
                            ));
                        }
                    }
                    // ... the existing body, unchanged, follows here ...
                }
```

`Token::Comma` — attribute the separator to the enclosing call, before the existing operator-flushing loop:

```rust
                Token::Comma => {
                    let frame = bracket_stack
                        .last_mut()
                        .ok_or_else(|| anyhow!(COMMA_OUTSIDE_CALL_ERR))?;
                    if frame.function.is_none() {
                        return Err(anyhow!(COMMA_OUTSIDE_CALL_ERR));
                    }
                    frame.commas += 1;
                    // ... the existing body, unchanged, follows here ...
                }
```

After the loop, an expression that ends on a function name is rejected:

```rust
        if let Some(fun) = pending_function {
            return Err(anyhow!(
                "Parse Error: Function '{}' must be followed by '('.",
                fun.to_string().to_lowercase()
            ));
        }
```

`MathFunction` needs to be in scope in `rpn_resolver.rs`; it already is, through the existing `use crate::token::{self, MathFunction, Number, Operator, Token};`.

- [ ] **Step 5: Run the tests**

Run: `cargo test`
Expected: all green.

Two existing tests need attention if they fail, and both are legitimate consequences to accept rather than bugs to work around:
- anything asserting a bare `sin 5` form — there is none at the time of writing, confirmed by grep
- `test_invalid_input_is_rejected` may already cover `(1,2)`; if it asserts the old generic message, update it to the new specific one

Run: `cargo clippy --all-targets 2>&1 | grep -c "^warning:"`, then `cargo fmt --check`

- [ ] **Step 6: Update the documentation for the parenthesis rule**

`README.md` and the module docs in `src/lib.rs` both list the built-in functions. Add one sentence after that list in each:

```
Function arguments are always parenthesised: `sqrt(16)`, `max(1,2)`.
```

- [ ] **Step 7: Commit**

```bash
git add -A src tests README.md
git commit -m "Validate function arity while building the RPN form

max(1), max(1,2,3), sin(1,2) and max() all reported the same generic
'malformed expression'. The shunting yard now keeps one frame per open
bracket, recording whether it opens a call and how many arguments it has
seen, and reports the function by name with the counts it expected and
received. A comma outside a call is diagnosed on its own terms.

Parentheses after a function name become mandatory: 'sin 5' used to work
by accident, and argument counting has no meaning without a bracket to
count within."
```

---

## Definition of done for the stage

- [ ] `cargo test` green, with the new tests from all four tasks.
- [ ] `test_chained_assignment_sets_all_variables` (`x=y=5` sets both and returns 5) and `test_chained_expressions` (`x=2; y=3; x*y` returns 6) still pass untouched. The spec names them because they are load-bearing behaviour that none of these four changes may disturb.
- [ ] `cargo clippy --all-targets` at or below 40 warnings, with `too many lines` gone.
- [ ] `cargo fmt --check` clean.
- [ ] The measured `max_value_bits` figure and its timing recorded in the spec.
- [ ] The three declared behaviour changes gathered for the 0.3.0 CHANGELOG: integral results now come back as `NaturalNumber`, oversized results are refused, parentheses after a function name are mandatory.
