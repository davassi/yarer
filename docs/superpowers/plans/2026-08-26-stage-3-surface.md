# Stage 3 (Surface) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make yarer present itself properly to the outside world — invocable from a shell, honest about what it makes people download and which compiler it needs, and gated by CI that actually checks something — while closing the `f64` narrowing defect that ships three wrong answers in 0.3.0.

**Architecture:** Nothing about the evaluator's semantics changes except the narrowing refusal. The binary grows two non-interactive input modes over one shared evaluation path; the four CLI dependencies move behind a feature that defaults on; `eval_with` is split so that every function fits under clippy's threshold; and the three places that turn a `Number` into an `f64` become one.

**Tech Stack:** Rust 2021, MSRV to be measured (expected 1.86). `num-bigint`/`num-rational` for exact arithmetic, `thiserror` for errors, `clap`+`rustyline` for the CLI (now optional), `cargo-fuzz`/libFuzzer for the fuzz target. No new runtime dependencies.

**Spec:** `docs/superpowers/specs/2026-08-26-stage-3-surface-design.md`. Read it before starting; this plan argues from it and does not repeat its reasoning.

## Global Constraints

- Branch: `stage-3-surface`, cut from `master` after 0.3.0 was published.
- **Never run `git stash` in this repository.** Two protected user stashes from June 2025 live here and must not be disturbed. If you need a clean tree, use `git worktree` and remove it when done.
- After any edit, confirm the build actually recompiled: if `cargo test` output has no `Compiling yarer` line after a source change, run `cargo clean -p yarer` and try again.
- Every task ends with `cargo test` green and `cargo fmt --check` clean.
- **The 143 value-asserting macro invocations in `tests/integration_tests.rs` must survive verbatim.** Task 7 restructures the evaluation loop; they are the proof it changed no meaning. Check with a **superset test against the merge base**, never with a digest of the whole set — a digest changes when a test is legitimately added, and this plan adds many:
  ```bash
  git show master:tests/integration_tests.rs \
    | grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' | sort > /tmp/baseline.txt
  grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' tests/integration_tests.rs \
    | sort | comm -23 /tmp/baseline.txt -
  # must print nothing
  ```
  If a line appears, stop: an existing expression changed meaning, and that is a finding, not a test to adjust.
- Clippy is compared **per lint on a cold cache**, never by counting warnings, until Task 8 makes the count zero:
  ```bash
  cargo clean -p yarer >/dev/null 2>&1
  cargo clippy --all-targets --message-format=json 2>/dev/null \
    | grep -oP '"code":"clippy::[a-z_]+"' | sed 's/.*clippy:://; s/"//' | sort | uniq -c | sort -rn
  ```
- Do not add runtime dependencies. Do not reintroduce `anyhow`.
- Error message text is lower case and carries no category prefix — `Error::render` adds it, once.
- `Cargo.toml` stays at `version = "0.3.0"` until Task 11.
- Commit messages: imperative subject line, no tool attribution of any kind, no `Co-Authored-By` trailer, no mention of any AI tool.

## Baselines to record before Task 1

Put all three in Task 1's commit message body.

```bash
grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' tests/integration_tests.rs | sort | md5sum
# expected: a9904e579d888f7a0c6d22f96088ea6d  (143 invocations)

cargo clean -p yarer >/dev/null 2>&1
cargo clippy --all-targets --message-format=json 2>/dev/null \
  | grep -oP '"code":"clippy::[a-z_]+"' | sed 's/.*clippy:://; s/"//' | sort | uniq -c | sort -rn
# expected 20 distinct sites; see the spec's component D for the full list

cargo tree --edges normal --prefix none | awk '{print $1}' | grep -v '^$' | sort -u | wc -l
# expected: 76
```

## The compiler is most of the checklist

Three matches in this crate are exhaustive with no catch-all arm, by deliberate choice made during Stage 2. Adding a variant to `EvalError` (Task 1) therefore **fails to compile** until each is updated:

- `EvalError::span` (`src/error.rs`) — the `match self` listing every span-carrying variant.
- `EvalError::at` (`src/error.rs`) — the same list again, binding `span: slot`.
- `test_every_variant_can_be_asked_for_its_span` (`src/error.rs`'s test module) — an array naming every variant, which is the thing that makes the two above provably complete.

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `src/token.rs` | `Narrowing`, `narrow_to_f64`, and the two callers that live here (`Display`, `TryFrom<Number> for f64`) | 1, 8 |
| `src/error.rs` | `EvalError::OperandTooSmallForFloat` | 1 |
| `src/functions.rs` | `number_to_f64` loses its `on_error` parameter and picks the variant itself | 1, 8 |
| `src/parser.rs` | `once_cell::sync::Lazy` → `std::sync::LazyLock` | 2, 8 |
| `Cargo.toml` | dead dependencies out, `cli` feature, `rust-version`, `exclude`, version bump | 2, 3, 4, 9, 11 |
| `src/bin/main.rs` | the shared evaluation path and the three input modes | 5, 6, 8 |
| `src/expression.rs` | `Stacks`, `apply_operator`, and `eval_with` reduced to a walk | 7, 8 |
| `tests/cli.rs` | **new** — spawns the real binary: exit codes, streams, stop-at-first-error | 5, 6 |
| `tests/readme.rs` | **new** — executes the README's CLI transcripts | 6 |
| `tests/fuzz_regressions.rs` | **new** — replays the curated corpus on stable | 9 |
| `tests/fuzz_regressions/` | **new** — seeds and every input that ever crashed | 9 |
| `fuzz/` | **new** — cargo-fuzz crate, excluded from the package | 9 |
| `.github/workflows/rust.yml` | six jobs replacing three steps | 10 |
| `CHANGELOG.md` | **new** — 0.1.x through 0.4.0 | 11 |
| `README.md`, `src/lib.rs`, `docs/tech-debt.md` | documentation, the corrected transcripts, the register | 6, 11 |

**`src/token.rs` is 1127 lines and holds `Number`, `Operator`, `Token`, `MathFunction` and every conversion between them.** It is the obvious candidate for a split, and this plan deliberately does not do one: Task 1 adds ~35 lines to it and Task 8 edits nine doc comments in it, neither of which is a reason to move four types between files during a release. It gets a register entry in Task 11 instead.

---

### Task 1: One narrowing, and the symmetric refusal

Spec component G. The correctness fix, first, because the rest of the stage puts a CI gate around this code.

**Files:**
- Modify: `src/error.rs` — the `OperandTooSmallForFloat` variant and its three lists
- Modify: `src/token.rs` — `Narrowing`, `narrow_to_f64`, `Display for Number`, `TryFrom<Number> for f64`
- Modify: `src/functions.rs` — `number_to_f64` and its twelve call sites
- Test: `tests/integration_tests.rs`

**Interfaces:**
- Produces:
  - `pub(crate) enum Narrowing { TooLarge, TooSmall }` in `src/token.rs`
  - `pub(crate) fn narrow_to_f64(value: &Number) -> Result<f64, Narrowing>` in `src/token.rs`
  - `pub(crate) fn number_to_f64(value: &Number) -> Result<f64, EvalError>` in `src/functions.rs` — **note the dropped second parameter**
  - `EvalError::OperandTooSmallForFloat { span: Option<Span> }`

- [ ] **Step 1: Record the baselines**

Run the three commands under "Baselines to record before Task 1" above. Put all three in this task's commit message body.

- [ ] **Step 2: Write the failing tests**

Append to `tests/integration_tests.rs`. First add `ConversionError` to the import at the top of the file:

```rust
use yarer::{
    ConversionError, Error, EvalError, Expression, Limits, MathFunction, Number, ParseError,
    Session, Span,
};
```

Then the tests:

```rust
// ---------------------------------------------------------------------------
// Narrowing to f64
// ---------------------------------------------------------------------------

/// The mirror of `OperandTooLargeForFloat`, and the reason it was needed.
///
/// `to_f64` answers `Some(0.0)` for a value too small to represent, which
/// arrives looking like a success, so `number_to_f64` zeroed the operand before
/// the function ran. A function that shrinks toward its input does not care —
/// `sin x ≈ x` — but one that expands small values is wrecked by it:
/// `log(1/(10^400))` is exactly -400 and 0.3.0 refused it as not a real number,
/// and `sqrt(1/(10^400))` is 1e-200, comfortably inside f64's range, where
/// 0.3.0 answered 0.
///
/// The span is the function name's, because that is the token whose operand is
/// the problem and the thing the user would have to change.
#[test]
fn test_an_operand_too_small_for_a_float_is_refused_not_zeroed() {
    let session = Session::init();
    for (source, start, end) in [
        ("log(1/(10^400))", 0, 3),
        ("ln(1/(10^400))", 0, 2),
        ("sqrt(1/(10^400))", 0, 4),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        let err = expr.eval(&session).expect_err("must be refused, not zeroed");
        assert!(
            matches!(err, EvalError::OperandTooSmallForFloat { span: Some(s) }
                if (s.start, s.end) == (start, end)),
            "for {source}, got {err:?}"
        );
    }
}

/// The declared withdrawal. These answered correctly — `sin` of something that
/// rounds to zero really is 0 — and are now refused by the same rule, exactly
/// as Stage 2 knowingly withdrew `atan(10^400) = pi/2` when it made the
/// overflow side refuse. One rule is worth more than a handful of preserved
/// answers at the extreme edge of the value space, and pinning the withdrawal
/// makes it a decision rather than a surprise.
#[test]
fn test_the_functions_that_tolerated_a_zeroed_operand_now_refuse_it_too() {
    let session = Session::init();
    for source in [
        "sin(1/(10^400))",
        "cos(1/(10^400))",
        "exp(1/(10^400))",
        "atan(1/(10^400))",
        "cdf(1/(10^400))",
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert!(
            matches!(
                expr.eval(&session),
                Err(EvalError::OperandTooSmallForFloat { .. })
            ),
            "for {source}"
        );
    }
}

/// The overflow side is unchanged, which is what makes the pair symmetric
/// rather than one rule replacing another.
#[test]
fn test_an_operand_too_large_for_a_float_is_still_refused_as_before() {
    let session = Session::init();
    for source in ["sin(10^400)", "atan(10^400)", "exp(10^400)"] {
        let expr = Expression::compile(source).expect("compiles");
        assert!(
            matches!(
                expr.eval(&session),
                Err(EvalError::OperandTooLargeForFloat { .. })
            ),
            "for {source}"
        );
    }
}

/// Zero narrows to 0.0 successfully. This is the case that separates
/// "underflowed to zero" from "is zero", and without it the rule above would
/// refuse `sin(0)`.
#[test]
fn test_a_genuine_zero_still_narrows_to_a_float() {
    let session = Session::init();
    for (source, expected) in [("sin(0)", 0), ("cos(0)", 1), ("exp(0)", 1)] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
    assert_eq!(
        f64::try_from(Number::NaturalNumber(BigInt::zero())).unwrap(),
        0.0
    );
}

/// `TryFrom` had the same hole at the other end of the crate: a value that
/// prints correctly as a ratio converted silently to zero, while a value too
/// large correctly errored.
#[test]
fn test_converting_a_value_too_small_for_a_float_errors_rather_than_zeroing() {
    let session = Session::init();

    let tiny = Expression::compile("1/(10^400)")
        .unwrap()
        .eval(&session)
        .unwrap();
    assert!(
        tiny.to_string().starts_with("1/1"),
        "it prints as a ratio, so it must not convert to 0: {tiny}"
    );
    assert!(matches!(
        f64::try_from(tiny),
        Err(ConversionError::OutOfRange { .. })
    ));

    // An ordinary fraction is untouched.
    let third = Expression::compile("1/3").unwrap().eval(&session).unwrap();
    assert!((f64::try_from(third).unwrap() - 1.0 / 3.0).abs() < 1e-15);
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --test integration_tests narrow small large genuine converting`

Expected: `test_an_operand_too_small_for_a_float_is_refused_not_zeroed` fails to compile (`OperandTooSmallForFloat` does not exist). After Step 4 it will fail on behaviour instead.

- [ ] **Step 4: Add the error variant**

In `src/error.rs`, immediately after `OperandTooLargeForFloat`:

```rust
    #[error("operand is too small for floating-point evaluation")]
    OperandTooSmallForFloat { span: Option<Span> },
```

Then add `| EvalError::OperandTooSmallForFloat { span }` to the list in `EvalError::span`, `| EvalError::OperandTooSmallForFloat { span: slot }` to the list in `EvalError::at`, and `EvalError::OperandTooSmallForFloat { span: None },` to the array in `test_every_variant_can_be_asked_for_its_span`. The compiler names the first two; the third is what keeps them honest, so do not skip it.

- [ ] **Step 5: Write the one narrowing**

In `src/token.rs`, immediately above `impl Display for Number`:

```rust
/// Which end of `f64`'s range a value fell off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Narrowing {
    /// The magnitude exceeds what `f64` can hold.
    TooLarge,
    /// The value is not zero, but rounds to zero.
    TooSmall,
}

/// The one place this crate narrows a [`Number`] to `f64`.
///
/// `to_f64` signals neither of its failures: it answers `Some(inf)` when the
/// value is too large and `Some(0.0)` when it is too small, so both losses
/// arrive looking like successes and every caller that forgets to check
/// inherits a wrong answer. Three callers did. `Display` was fixed during
/// Stage 2 and the other two were not, which is how `log(1/(10^400))` came to
/// be refused as not a real number when it is exactly -400.
///
/// A genuine zero narrows to `0.0` successfully. Only a `0.0` that came from a
/// non-zero value is [`Narrowing::TooSmall`], which is what lets
/// `sqrt(1/(10^400))` be refused without also refusing `sqrt(0)`.
pub(crate) fn narrow_to_f64(value: &Number) -> Result<f64, Narrowing> {
    let (narrowed, is_zero) = match value {
        Number::NaturalNumber(v) => (v.to_f64(), v.is_zero()),
        Number::DecimalNumber(v) => (v.to_f64(), v.numer().is_zero()),
    };
    // Both types answer `Some` for every input, so this arm is unreachable. It
    // reports `TooLarge` rather than panicking because a `None` could only ever
    // mean the value did not fit.
    let Some(f) = narrowed else {
        return Err(Narrowing::TooLarge);
    };
    if !f.is_finite() {
        return Err(Narrowing::TooLarge);
    }
    if f == 0.0 && !is_zero {
        return Err(Narrowing::TooSmall);
    }
    Ok(f)
}
```

- [ ] **Step 6: Route the two callers in `token.rs` through it**

`Display for Number`'s decimal arm becomes:

```rust
            Number::DecimalNumber(v) => {
                if v.denom().is_one() {
                    write!(f, "{}", v.to_integer())
                } else if let Ok(fl) = narrow_to_f64(self) {
                    write!(f, "{fl}")
                } else {
                    // Too large or too small to be a float, and exact either
                    // way: print what it actually is.
                    write!(f, "{}/{}", v.numer(), v.denom())
                }
            }
```

and `TryFrom<Number> for f64` becomes:

```rust
impl TryFrom<Number> for f64 {
    type Error = ConversionError;

    fn try_from(n: Number) -> Result<Self, Self::Error> {
        narrow_to_f64(&n).map_err(|_| ConversionError::OutOfRange {
            value: n.to_string(),
            target: "f64",
        })
    }
}
```

Both ends are `OutOfRange`: a value that does not fit does not fit, and which end it fell off is not something a conversion's caller can act on differently.

- [ ] **Step 7: Route `functions.rs` through it, and drop the parameter**

```rust
/// Narrows an operand to `f64` for the built-ins that are defined in terms of
/// one, answering the error that says which end of the range it fell off.
///
/// The `on_error` parameter this used to take was passed
/// `EvalError::OperandTooLargeForFloat` at all twelve call sites and nothing
/// else was ever possible — which is precisely why the underflow case could not
/// be reported. Choosing the variant is this function's job now.
///
/// # Errors
/// [`EvalError::OperandTooLargeForFloat`] or
/// [`EvalError::OperandTooSmallForFloat`].
pub(crate) fn number_to_f64(value: &Number) -> Result<f64, EvalError> {
    crate::token::narrow_to_f64(value).map_err(|why| match why {
        Narrowing::TooLarge => EvalError::OperandTooLargeForFloat { span: None },
        Narrowing::TooSmall => EvalError::OperandTooSmallForFloat { span: None },
    })
}
```

Change line 8 of `src/functions.rs` from `use crate::token::{MathFunction, Number};` to
`use crate::token::{MathFunction, Narrowing, Number};`.

Then delete the second argument at all twelve call sites — `number_to_f64(&value, EvalError::OperandTooLargeForFloat { span: None })?` becomes `number_to_f64(&value)?`. The compiler lists every one; there is no judgement involved.

- [ ] **Step 8: Run everything**

Run: `cargo test`

Expected: green. Then confirm the narrowing really is the only one:

```bash
grep -rn 'to_f64' src/
# exactly one line: the two calls inside narrow_to_f64
```

and re-run the value-assertion superset check from the Global Constraints.

- [ ] **Step 9: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/error.rs src/token.rs src/functions.rs tests/integration_tests.rs
git commit -m "Refuse an operand too small for a float instead of zeroing it"
```

---

### Task 2: The dead dependencies, and one that std replaced

Spec component A, first half. Three dependencies leave and nothing else changes.

**Files:**
- Modify: `Cargo.toml` — remove `bigdecimal`, `lazy_static`, `once_cell`
- Modify: `src/parser.rs` — `once_cell::sync::Lazy` → `std::sync::LazyLock`

- [ ] **Step 1: Confirm two of them are genuinely unreferenced**

```bash
grep -rn 'bigdecimal\|BigDecimal\|lazy_static' src/ tests/
# must print nothing
```

If anything prints, stop and report it: the spec's claim was measured on 0.3.0 and something has changed.

- [ ] **Step 2: Replace `Lazy` with `LazyLock`**

In `src/parser.rs`, delete the line `use once_cell::sync::Lazy;` and change the import block and the static to:

```rust
use regex::Regex;
use std::sync::LazyLock;

// ...

static EXPRESSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // The two-character operators come first because regex alternation is
    // ordered: written after the character class, `<=` would match as `<`
    // followed by `=` — a comparison and then an assignment, which fails
    // somewhere else with a message about the wrong thing.
    Regex::new(r"(<=|>=|==|<>|\d+\.?\d*|\.\d+|[-+*/^(),=<>\[\]×÷!;]|[a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Should compile regex")
});
```

`LazyLock` has been stable since Rust 1.80, comfortably under the floor Task 4 declares. This also silences the `non_std_lazy_statics` warning that Task 8 would otherwise have to deal with.

- [ ] **Step 3: Remove the three dependency lines**

Delete `bigdecimal`, `lazy_static` and `once_cell` from `[dependencies]` in `Cargo.toml`.

- [ ] **Step 4: Run everything**

Run: `cargo test`
Expected: green, no warnings.

```bash
cargo tree --edges normal --depth 1 | grep -E 'bigdecimal|lazy_static|once_cell'
# must print nothing — they may still appear deeper in the tree as
# somebody else's dependency, which is fine and not yarer's business
```

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add Cargo.toml src/parser.rs
git commit -m "Drop two dependencies nothing used and one std replaced"
```

---

### Task 3: Put the CLI behind a feature

Spec component A, second half. The change that takes a library user from 76 crates to 41.

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Record what a library user compiles today**

```bash
cargo tree --edges normal --prefix none | awk '{print $1}' | grep -v '^$' | sort -u | wc -l
```

Record the number in the commit message. It should be 73 after Task 2 (76 minus the three that left).

- [ ] **Step 2: Make the four optional and declare the feature**

In `Cargo.toml`, change the four CLI dependency lines and add the two new sections:

```toml
clap = { version = "4.4.2", features = ["derive"], optional = true }
rustyline = { version = "16.0.0", optional = true }
env_logger = { version = "0.11.2", optional = true }
dirs = { version = "6.0.0", optional = true }

[features]
# On by default so that `cargo install yarer` keeps working exactly as it did
# and no existing library user has to change anything. A library that only
# wants the evaluator opts out with `default-features = false` and stops
# compiling an argument parser, a line editor, and — through env_logger —
# jiff, a complete datetime library.
default = ["cli"]
cli = ["dep:clap", "dep:rustyline", "dep:env_logger", "dep:dirs"]
```

and add `required-features` to the existing `[[bin]]` section:

```toml
[[bin]]
name = "yarer"
path = "src/bin/main.rs"
required-features = ["cli"]
```

`log` stays unconditional: the library's own `debug!` calls need the facade, and it pulls nothing.

- [ ] **Step 3: Verify both builds**

```bash
cargo build --no-default-features    # library only, must succeed
cargo test  --no-default-features    # must be green
cargo build                          # binary must still be produced
ls target/debug/yarer                # must exist
```

If `cargo build --no-default-features` fails with an unresolved `clap`/`rustyline`/`dirs`/`env_logger` import, the failure is in library code, not in the manifest — find the `use` and report it, because that is exactly the leak this feature exists to expose.

- [ ] **Step 4: Measure what it bought**

```bash
cargo tree --edges normal --prefix none --no-default-features \
  | awk '{print $1}' | grep -v '^$' | sort -u | wc -l
```

Expected: about 41. Record the before and after in the commit message.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add Cargo.toml
git commit -m "Put the CLI behind a feature that defaults on"
```

---

### Task 4: Declare the minimum supported Rust version

Spec component B. An unverified `rust-version` is a comment; this task measures it.

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Find the floor by bisecting**

`Vec::pop_if` at `src/shunting.rs:77` stabilised in 1.86 and is the newest thing yarer's own code uses, so 1.86 is the expected answer — but a dependency may sit above it. Measure rather than assume:

```bash
for v in 1.85.0 1.86.0 1.87.0 1.88.0; do
  rustup toolchain install "$v" --profile minimal -q 2>/dev/null
  printf '%s: ' "$v"
  if cargo +"$v" build --all-features -q 2>/dev/null; then echo OK; else echo FAIL; fi
done
```

The floor is the lowest version that prints OK with every higher one also OK. If a *dependency* is the blocker rather than yarer's own code, the build error names it — record which one in the commit message, because that is the fact that will make someone want to change it later.

- [ ] **Step 2: Check the floor for the slim build too**

```bash
cargo +<floor> build --no-default-features -q && echo "slim OK at the floor"
```

The library's own floor may be lower than the default-feature floor, because the CLI dependencies are the demanding ones. `rust-version` is a single per-package value and takes the **higher** of the two, so this step is confirming that the declared value is not *below* what some feature set needs — not looking for a lower one.

- [ ] **Step 3: Declare it**

In `Cargo.toml`'s `[package]` section, after `edition`:

```toml
rust-version = "1.86"
```

using whatever Step 1 actually measured, not this number if they differ.

- [ ] **Step 4: Verify the declaration is true**

```bash
cargo +<floor> build --all-features -q          && echo "all-features OK"
cargo +<floor> test  --all-features -q          && echo "tests OK"
cargo +<floor> build --no-default-features -q   && echo "slim OK"
```

All three must pass. Then confirm the version *below* the floor fails, which is what makes the number meaningful rather than merely safe:

```bash
cargo +<floor minus one release> build --all-features 2>&1 | tail -3
```

If it also succeeds, the floor is lower than declared — go back to Step 1 and widen the search downward.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add Cargo.toml
git commit -m "Declare the Rust version this crate actually needs"
```

---

### Task 5: Script mode — the shared path and `-e`

Spec component C, first half. After this task `yarer -e "1+2"` works and is composable in a shell; stdin is still the REPL's.

**Files:**
- Modify: `src/bin/main.rs`
- Test: `tests/cli.rs` (**new**)

**Interfaces:**
- Produces, all in `src/bin/main.rs`:
  - `fn evaluate(session: &Session, line: &str) -> Result<Number, Error>`
  - `fn report(session: &Session, line: &str) -> bool` — prints, answers whether it succeeded
  - `fn run_expressions(session: &Session, expressions: &[String]) -> ExitCode`
  - `fn run_repl(session: &Session, quiet: bool) -> ExitCode`

- [ ] **Step 1: Write the failing tests**

Create `tests/cli.rs`:

```rust
//! The command-line binary, exercised by running it.
//!
//! Exit status, which stream each kind of output goes to, and what happens
//! after a failure are all observable only from outside the process, and none
//! of them was covered before this file existed — which is how the README came
//! to document two things the binary does not do.

#![cfg(feature = "cli")]

use std::io::Write;
use std::process::{Command, Stdio};

/// The binary this test crate was built alongside. Cargo sets
/// `CARGO_BIN_EXE_<name>` for test targets, so there is no need for a helper
/// crate to find it.
fn yarer() -> Command {
    Command::new(env!("CARGO_BIN_EXE_yarer"))
}

#[test]
fn test_e_prints_the_value_to_stdout_and_exits_zero() {
    let out = yarer().args(["-e", "2^10"]).output().expect("runs");
    assert!(out.status.success(), "status was {:?}", out.status);
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "1024");
    assert!(
        out.stderr.is_empty(),
        "stderr should be silent on success: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The whole point of the contract: `x=$(yarer -e ...)` must be able to tell
/// success from failure, and must not capture an error message as if it were a
/// value.
#[test]
fn test_a_failure_goes_to_stderr_and_exits_one() {
    let out = yarer().args(["-e", "1/0"]).output().expect("runs");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        out.stdout.is_empty(),
        "stdout must stay clean: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("division by zero") && stderr.contains('^'),
        "the rendered error with its caret should reach stderr: {stderr}"
    );
}

/// A parse failure is reported the same way an evaluation failure is: one exit
/// code, because a shell script cannot act differently on the difference.
#[test]
fn test_a_parse_failure_uses_the_same_contract() {
    let out = yarer().args(["-e", "1+"]).output().expect("runs");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("expected a value"));
}

#[test]
fn test_several_expressions_share_one_session() {
    let out = yarer()
        .args(["-e", "x=2", "-e", "x*3"])
        .output()
        .expect("runs");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).lines().collect::<Vec<_>>(),
        ["2", "6"],
        "the assignment must be visible to the next expression"
    );
}

/// A run that half-succeeded is the hardest outcome to debug, so it is made
/// impossible: nothing after the failure runs.
#[test]
fn test_evaluation_stops_at_the_first_failure() {
    let out = yarer()
        .args(["-e", "1+1", "-e", "1/0", "-e", "2+2"])
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).lines().collect::<Vec<_>>(),
        ["2"],
        "2+2 must not have run"
    );
}

/// The banner is for a human at a prompt. On stdout it would end up inside
/// `x=$(yarer -e "2^10")`.
#[test]
fn test_the_banner_stays_out_of_script_mode() {
    let out = yarer().args(["-e", "1+1"]).output().expect("runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("Yarer v."), "banner leaked into stdout: {stdout}");
    assert!(!String::from_utf8_lossy(&out.stderr).contains("Yarer v."));
}

/// An assignment evaluates to the value assigned, and script mode prints what
/// the expression evaluates to. The README claimed otherwise for three
/// releases; Task 6 corrects it.
#[test]
fn test_an_assignment_prints_its_value() {
    let out = yarer().args(["-e", "x=10"]).output().expect("runs");
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "10");
}
```

`use std::io::Write;` and `Stdio` are unused until Task 6; add them then rather than now, or accept one unused-import warning until Task 6 lands. Prefer adding them in Task 6 — this task's file should compile clean.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test cli`

Expected: every test fails. `-e` is not a recognised argument, so clap exits with status 2 and a usage message.

- [ ] **Step 3: Restructure `main.rs` around one evaluation path**

Replace the imports and `main` in `src/bin/main.rs`. The three input modes differ only in where lines come from and where output goes, so they share `evaluate` and `report`; that is what stops them drifting in how they report the same failure.

```rust
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use yarer::{Error, Expression, Number, Session};

use log::debug;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static HISTORY_FILE: &str = ".yarer_history";
```

The `Cli` struct gains one field:

```rust
struct Cli {
    /// Evaluate an expression and exit. May be given more than once; all the
    /// expressions share a single session, so a variable set by one is visible
    /// to the next.
    #[arg(short = 'e', long = "expr", value_name = "EXPR")]
    expr: Vec<String>,

    #[arg(short, long)]
    quiet: bool,
}
```

and these three functions go above `main`:

```rust
/// Compiles and evaluates one line against `session`.
///
/// The one step every input mode shares. `compile` and `eval` fail with
/// different types; `Error` is the union the library provides for exactly this
/// caller.
fn evaluate(session: &Session, line: &str) -> Result<Number, Error> {
    Expression::compile(line)
        .map_err(Error::from)
        .and_then(|expr| expr.eval(session).map_err(Error::from))
}

/// Evaluates `line` and reports it: the value on stdout, or the rendered error
/// — message, source line and caret — on stderr. Answers whether it succeeded.
///
/// Which stream each goes to is the whole of the shell contract. A value on
/// stdout can be captured; an error there would be captured too.
fn report(session: &Session, line: &str) -> bool {
    match evaluate(session, line) {
        Ok(value) => {
            println!("{value}");
            true
        }
        Err(err) => {
            eprintln!("{}", err.render(line));
            false
        }
    }
}

/// `-e` mode: each expression in order against one session, stopping at the
/// first failure.
fn run_expressions(session: &Session, expressions: &[String]) -> ExitCode {
    for line in expressions {
        if !report(session, line) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
```

- [ ] **Step 4: Move the REPL into its own function and give `main` a mode choice**

The existing loop body is unchanged; it moves into `run_repl`, which now owns the banner and returns an `ExitCode` instead of propagating `rustyline::Result`.

```rust
/// The interactive REPL: unchanged behaviour, and now one of three modes
/// rather than the only one.
fn run_repl(session: &Session, quiet: bool) -> ExitCode {
    if !quiet {
        println!("Yarer v.{VERSION} - Yet Another Rust Expression Resolver.");
        println!("License MIT OR Apache-2.0");
    }

    let mut rl = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("Could not start the interactive editor: {err}");
            return ExitCode::FAILURE;
        }
    };

    let local_history = dirs::config_dir()
        .unwrap_or_default()
        .join(HISTORY_FILE);
    let local_history = local_history.as_os_str().to_str().unwrap_or(HISTORY_FILE);
    debug!("Local history file: '{local_history}'");
    let _ = rl.load_history(local_history);

    loop {
        match rl.readline("> ") {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                if line.trim().eq_ignore_ascii_case("quit") {
                    break;
                }
                let _ = rl.add_history_entry(line.as_str());
                report(session, &line);
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                println!("quit");
                break;
            }
            Err(err) => {
                eprintln!("Error: {err:?}");
                break;
            }
        }
    }

    let _ = rl.save_history(local_history);
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    env_logger::init();

    let session = Session::init();

    if !cli.expr.is_empty() {
        return run_expressions(&session, &cli.expr);
    }

    run_repl(&session, cli.quiet)
}
```

Note the two incidental repairs the move makes: `unwrap_or(PathBuf::default())` becomes `unwrap_or_default()` (this is the `unwrap_or_default` clippy warning at `main.rs:69`), and the REPL's errors now go to stderr like every other error the binary produces. `PathBuf` is still needed for the `dirs::config_dir()` fallback type; keep the import.

- [ ] **Step 5: Run the tests**

Run: `cargo test --test cli`
Expected: all eight pass.

Run: `cargo test`
Expected: green everywhere.

- [ ] **Step 6: Check the REPL by hand**

Script mode must not have changed the interactive path:

```bash
cargo run -q
> 1+2
> x=10
> 1/0
> quit
```

Expect `3`, then `10`, then the rendered division-by-zero error, then exit. The banner should still appear.

- [ ] **Step 7: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/bin/main.rs tests/cli.rs
git commit -m "Let an expression be evaluated from the command line"
```

---

### Task 6: Script mode — the stream, and the README's transcripts

Spec component C, second half, and the test that makes the README honest.

**Files:**
- Modify: `src/bin/main.rs` — `run_stream`, and the terminal check in `main`
- Modify: `tests/cli.rs` — the stream tests
- Modify: `README.md` — correct the false transcripts
- Test: `tests/readme.rs` (**new**)

**Interfaces:**
- Consumes: `report`, `run_expressions`, `run_repl` (Task 5).
- Produces: `fn run_stream(session: &Session) -> ExitCode` in `src/bin/main.rs`.

- [ ] **Step 1: Write the failing stream tests**

Append to `tests/cli.rs`, and add `use std::io::Write;` and `Stdio` to its imports now:

```rust
/// Runs the binary with `input` on stdin and no arguments.
fn piped(input: &str) -> std::process::Output {
    let mut child = yarer()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawns");
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(input.as_bytes())
        .expect("writes");
    child.wait_with_output().expect("runs")
}

#[test]
fn test_a_pipe_evaluates_every_line() {
    let out = piped("1+1\n2+2\n3+3\n");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).lines().collect::<Vec<_>>(),
        ["2", "4", "6"]
    );
}

/// One session for the whole stream, as in GNU bc, which is what makes a piped
/// file of expressions a script rather than a list of unrelated sums.
#[test]
fn test_a_pipe_shares_one_session_across_lines() {
    let out = piped("x=2\ny=3\nx*y\n");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).lines().collect::<Vec<_>>(),
        ["2", "3", "6"]
    );
}

#[test]
fn test_a_pipe_stops_at_the_first_failure() {
    let out = piped("1+1\n1/0\n2+2\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).lines().collect::<Vec<_>>(),
        ["2"],
        "2+2 must not have run"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("division by zero"));
}

#[test]
fn test_a_pipe_skips_blank_lines_and_honours_quit() {
    let out = piped("1+1\n\n\n2+2\nquit\n3+3\n");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).lines().collect::<Vec<_>>(),
        ["2", "4"],
        "blank lines contribute nothing and quit ends the stream"
    );
}

#[test]
fn test_an_empty_pipe_succeeds_silently() {
    let out = piped("");
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn test_the_banner_stays_out_of_a_pipe_too() {
    let out = piped("1+1\n");
    assert!(!String::from_utf8_lossy(&out.stdout).contains("Yarer v."));
}

/// `-e` wins. A caller who passed an expression asked for that expression, and
/// whatever happens to be on stdin is not an instruction.
#[test]
fn test_an_expression_argument_wins_over_a_pipe() {
    let mut child = yarer()
        .args(["-e", "7*6"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawns");
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(b"1+1\n")
        .expect("writes");
    let out = child.wait_with_output().expect("runs");
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "42");
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --test cli pipe`

Expected: failure. Without the stream branch the binary tries to start a line editor on a non-terminal stdin, so it either exits immediately or errors — either way the expected output is absent.

- [ ] **Step 3: Add the stream mode**

In `src/bin/main.rs`, add `BufRead` and `IsTerminal` to the imports:

```rust
use std::io::{self, BufRead, IsTerminal};
```

and the function:

```rust
/// Stream mode: one expression per line from standard input, against one
/// session, stopping at the first failure.
///
/// Lines are read and evaluated one at a time rather than slurped, so a long
/// pipe reports its first few results before its producer has finished.
fn run_stream(session: &Session) -> ExitCode {
    for line in io::stdin().lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                eprintln!("Error reading standard input: {err}");
                return ExitCode::FAILURE;
            }
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("quit") {
            break;
        }
        if !report(session, line) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
```

and the branch in `main`, between the `-e` check and the REPL:

```rust
    if !cli.expr.is_empty() {
        return run_expressions(&session, &cli.expr);
    }

    // Not a terminal means something is feeding us: a pipe, a redirect, a
    // here-doc. Starting a line editor on that would be wrong, and printing a
    // banner into it would be worse.
    if !io::stdin().is_terminal() {
        return run_stream(&session);
    }

    run_repl(&session, cli.quiet)
```

`std::io::IsTerminal` has been stable since Rust 1.70, well under the declared floor, and costs no dependency.

- [ ] **Step 4: Run the stream tests**

Run: `cargo test --test cli`
Expected: all fifteen pass.

- [ ] **Step 5: Write the README transcript test**

Create `tests/readme.rs`:

```rust
//! The README's CLI transcripts, executed.
//!
//! Two of them were false for three releases. `9801/(2206*sqrt(2)) // approx of
//! PI` was documented as printing an approximation of pi; it is a parse error,
//! because yarer has no comment syntax. `x=10` was documented as printing
//! nothing; it prints `10`. Neither is a code defect — they are documentation
//! that lies about the tool, and nothing caught them because the CLI could not
//! be run from a test until now.
//!
//! Each fenced block containing a `> ` prompt is a transcript. Within a block,
//! a `> expression` line is an input and the lines after it, up to the next
//! prompt, are its expected output. Each block gets its own session, because
//! that is what a reader starting at the top of it would have.

#![cfg(feature = "cli")]

use std::io::Write;
use std::process::{Command, Stdio};

/// An expected line ending in an ellipsis asserts a prefix instead of an exact
/// match, so that the README can elide the 100-digit tail of `78!` without
/// this test either failing or having to carry the whole number.
const ELISION: &str = "...";

fn transcripts(readme: &str) -> Vec<Vec<(String, Vec<String>)>> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<String>> = None;

    for line in readme.lines() {
        if line.trim_start().starts_with("```") {
            if let Some(block) = current.take() {
                if block.iter().any(|l| l.trim_start().starts_with("> ")) {
                    blocks.push(block);
                }
            } else {
                current = Some(Vec::new());
            }
            continue;
        }
        if let Some(block) = current.as_mut() {
            block.push(line.to_string());
        }
    }

    blocks
        .into_iter()
        .map(|block| {
            let mut pairs: Vec<(String, Vec<String>)> = Vec::new();
            for raw in block {
                let line = raw.trim();
                if line.is_empty()
                    || line.starts_with('$')
                    || line.starts_with("Yarer v.")
                    || line.starts_with("License")
                {
                    continue;
                }
                if let Some(expression) = line.strip_prefix("> ") {
                    pairs.push((expression.trim().to_string(), Vec::new()));
                } else if let Some((_, expected)) = pairs.last_mut() {
                    expected.push(line.to_string());
                }
            }
            pairs
        })
        .filter(|pairs| !pairs.is_empty())
        .collect()
}

#[test]
fn test_the_readme_cli_transcripts_are_true() {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))
        .expect("the README is next to Cargo.toml");
    let blocks = transcripts(&readme);
    assert!(
        blocks.len() >= 3,
        "expected several transcript blocks, found {}",
        blocks.len()
    );

    for (n, pairs) in blocks.iter().enumerate() {
        let input: String = pairs
            .iter()
            .map(|(expression, _)| format!("{expression}\n"))
            .collect();

        let mut child = Command::new(env!("CARGO_BIN_EXE_yarer"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawns");
        child
            .stdin
            .take()
            .expect("stdin was piped")
            .write_all(input.as_bytes())
            .expect("writes");
        let out = child.wait_with_output().expect("runs");

        assert!(
            out.status.success(),
            "transcript {n} failed:\n{}\ninput was:\n{input}",
            String::from_utf8_lossy(&out.stderr)
        );

        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut produced = stdout.lines();
        for (expression, expected) in pairs {
            for want in expected {
                let got = produced.next().unwrap_or_else(|| {
                    panic!("transcript {n}: '{expression}' produced no output, expected '{want}'")
                });
                if let Some(prefix) = want.strip_suffix(ELISION) {
                    let prefix = prefix.trim_end_matches('.');
                    assert!(
                        got.starts_with(prefix),
                        "transcript {n}: '{expression}' gave '{got}', \
                         which does not start with the documented '{prefix}'"
                    );
                } else {
                    assert_eq!(
                        got, want,
                        "transcript {n}: '{expression}' gave '{got}', \
                         the README says '{want}'"
                    );
                }
            }
        }
    }
}
```

- [ ] **Step 6: Run it and watch it fail on the real defects**

Run: `cargo test --test readme`

Expected: failure. It should report the `// approx of PI` line as a parse failure, and it should report that `x=10` produced `10` where the README expects the next documented output. Read the failures before fixing anything — they are the point of the task.

- [ ] **Step 7: Correct the README's transcripts**

Three kinds of correction, all to `README.md`:

1. **Delete the comment that is not a comment.** `9801/(2206*sqrt(2)) // approx of PI` becomes `9801/(2206*sqrt(2))`, and the words move outside the block:

   ```text
         > 9801/(2206*sqrt(2))
         3.1415927300133055
   ```

   with a sentence under the block saying that this is Ramanujan's approximation of pi. Do **not** add comment syntax to the language; that is a change to the tokeniser and this stage is surface.

2. **Show the output of every assignment.** The REPL prints what an expression evaluates to, and an assignment evaluates to the value assigned. Every `> x=10` needs its `10` on the next line, in the `## CLI` block and in the Black–Scholes block.

3. **The elided factorial keeps its ellipsis.** `78!` in the 0.1.8 section prints a 116-digit number; leave the `.....` and let the prefix rule handle it.

Run `cargo test --test readme` after each correction until green.

- [ ] **Step 8: Run everything**

```bash
cargo test
cargo test --no-default-features    # tests/cli.rs and tests/readme.rs compile away
```

Both green.

- [ ] **Step 9: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/bin/main.rs tests/cli.rs tests/readme.rs README.md
git commit -m "Evaluate a piped stream, and make the README's transcripts true"
```

---

### Task 7: Split `eval_with`

Spec component D, the structural half. The register opened this entry when the operators work took `eval_with` from 146 lines to 218.

**Files:**
- Modify: `src/expression.rs`
- Test: none new — the 143 value assertions and the existing 92 integration tests are the proof this changed no meaning

**Interfaces:**
- Produces, in `src/expression.rs`:
  - `struct Stacks { values: VecDeque<Number>, vars: VecDeque<Option<String>> }`
  - `Stacks::push_checked(&mut self, value: Number, limits: Limits) -> Result<(), EvalError>`
  - `Stacks::push_truth(&mut self, truth: bool, limits: Limits) -> Result<(), EvalError>`
  - `fn apply_operator(session: &Session, op: Operator, span: Span, limits: Limits, stacks: &mut Stacks) -> Result<(), EvalError>`

**A deviation from the spec, and the reason.** The spec calls for three extractions — `eval_with`, `apply_operator` and `apply_truth_operator`. Do the first two, then **measure**. `Stacks::push_truth` collapses each of the ten truth arms to a single line, which the spec's estimate did not account for, and `apply_operator` is expected to land near 70 lines rather than the 113 the spec predicted. If it measures under 100, **stop at two**: `apply_truth_operator` would need a nested match over ten operators plus an arm listing the other six unreachably, and this crate has spent two stages removing exactly that kind of branch. Two functions with no unreachable code is a better outcome than three with one. If it measures over 100, do the third extraction as the spec describes and say so in the commit message.

- [ ] **Step 1: Record what you are about to change**

```bash
echo 'too-many-lines-threshold = 40' > clippy.toml
cargo clean -p yarer >/dev/null 2>&1
cargo clippy --all-targets --message-format=json 2>/dev/null \
  | grep -o 'too many lines ([0-9]*/40)' | sort -u
rm clippy.toml
```

A threshold of 40 makes clippy report the length of functions that are under 100, which is the only way to measure `apply_operator` after the split. Expect `216/40` or `218/40` for `eval_with` before you start.

Also record the value-assertion baseline from the Global Constraints — this is the task that could change an existing expression's meaning.

- [ ] **Step 2: Add `Stacks`**

In `src/expression.rs`, above `impl<'a> Expression<'a>`:

```rust
/// The two stacks the evaluation loop walks.
///
/// They are one type because they must stay in lockstep: every value pushed
/// gets a variable slot beside it, and every pop takes both. That was
/// maintained by hand at fifteen sites, and an operator arm that pushed a value
/// without pushing `None` beside it would have desynchronised the assignment
/// target for everything after it.
struct Stacks {
    values: VecDeque<Number>,
    vars: VecDeque<Option<String>>,
}

impl Stacks {
    fn new() -> Stacks {
        Stacks {
            values: VecDeque::new(),
            vars: VecDeque::new(),
        }
    }

    /// Pushes an operator's result: measured against the budget first, and with
    /// no variable name beside it, because an operator's result is not an
    /// assignable place.
    ///
    /// # Errors
    /// [`EvalError::ValueTooLarge`] when the result exceeds the budget.
    fn push_checked(&mut self, value: Number, limits: Limits) -> Result<(), EvalError> {
        limits::check_size(&value, limits)?;
        self.values.push_back(value);
        self.vars.push_back(None);
        Ok(())
    }

    /// Pushes a truth as this crate represents it — `1` or `0`, as in GNU bc.
    ///
    /// The size check is not decoration and is not unreachable: a zero-bit
    /// budget refuses `0 == 0`, whose operands cost nothing and whose answer
    /// costs one bit.
    ///
    /// # Errors
    /// As [`Stacks::push_checked`].
    fn push_truth(&mut self, truth: bool, limits: Limits) -> Result<(), EvalError> {
        self.push_checked(boolean(truth), limits)
    }
}
```

- [ ] **Step 3: Move the numeric kernels out of the impl block**

`apply_operator` is a free function and needs to call `power` and the factorial
helpers. They are currently associated functions on `Expression<'a>`, and
`Expression::power(..)` from outside the impl leaves `'a` unconstrained — which
compiles today only by inference and is a poor thing to depend on.

Move `power`, `power_integer`, `pow_big_int`, `pow_big_rational` and
`factorial_helper` out of `impl<'a> Expression<'a>` and into module-level free
functions in the same file, unchanged apart from `Self::` becoming a bare call.
None of them takes `self` or mentions `'a`, so this is mechanical, and the
compiler finds every call site.

While there, extract the factorial arm's body — about twenty lines, which would
otherwise dominate `apply_operator` — into:

```rust
/// Factorial, defined on non-negative integers.
///
/// Predicts the size first, so that `999999999!` is refused in microseconds
/// rather than computed, and measures the result afterwards, because the
/// prediction is an asymptotic series rounded up and is a bit short of the
/// truth at `n = 2`. The prediction buys the speed; the measurement buys the
/// exactness. Both are load-bearing and the register records what happened when
/// one of them was missing.
///
/// # Errors
/// [`EvalError::FactorialNotNatural`], [`EvalError::FactorialOperandTooLarge`],
/// or [`EvalError::ValueTooLarge`].
fn factorial(operand: &Number, span: Span, limits: Limits) -> Result<Number, EvalError> {
```

with the existing body moved in verbatim, and add `Span` to the span import at
the top of the file:

```rust
    span::{Span, Spanned},
```

- [ ] **Step 4: Extract `apply_operator`**

Move the whole `Token::Operator(op) => { … }` arm body into a free function. It needs the session (for assignment), the operator, the token's span (for errors), the limits, and the stacks — and nothing else, which is what makes the boundary a real one.

```rust
/// Applies one operator to the top of the stacks.
///
/// Split out of [`Expression::eval_with`] because that function's job is to
/// walk the compiled sequence and dispatch by token kind, and this one's is to
/// know what each operator means. Sixteen arms of the former buried the latter.
///
/// # Errors
/// Any [`EvalError`] an operator can raise: a division by zero, a value over
/// the budget, an assignment with no target, and so on.
fn apply_operator(
    session: &Session,
    op: Operator,
    span: Span,
    limits: Limits,
    stacks: &mut Stacks,
) -> Result<(), EvalError> {
    let at = |e: EvalError| e.at(span);

    let right_value: Number = stacks
        .values
        .pop_back()
        .ok_or(EvalError::Malformed { span: Some(span) })?;
    stacks.vars.pop_back();

    let (left_value, left_var) = if op.is_unary() {
        (Number::NaturalNumber(BigInt::zero()), None)
    } else {
        let value = stacks
            .values
            .pop_back()
            .ok_or(EvalError::Malformed { span: Some(span) })?;
        (value, stacks.vars.pop_back().unwrap_or(None))
    };

    match op {
        Operator::Add => stacks.push_checked(left_value + right_value, limits).map_err(at)?,
        Operator::Sub => stacks.push_checked(left_value - right_value, limits).map_err(at)?,
        Operator::Mul => stacks.push_checked(left_value * right_value, limits).map_err(at)?,
        Operator::Div => {
            let value = left_value
                .checked_div(&right_value)
                .ok_or(EvalError::DivisionByZero { span: Some(span) })?;
            stacks.push_checked(value, limits).map_err(at)?;
        }
        // `power` applies the budget itself, through a prediction that refuses
        // `2^100000000` without computing it, so it does not go through
        // `push_checked`.
        Operator::Pow => {
            let value = power(left_value, right_value, limits).map_err(at)?;
            stacks.values.push_back(value);
            stacks.vars.push_back(None);
        }
        Operator::Assign => {
            let Some(var) = left_var else {
                return Err(EvalError::AssignmentTargetMissing { span: Some(span) });
            };
            // `assign` decides the refusal, here and for `set`/`setf` alike;
            // this only supplies the position it happened at.
            session.assign(&var, right_value.clone()).map_err(at)?;
            stacks.values.push_back(right_value);
            stacks.vars.push_back(None);
        }
        Operator::Fac => {
            let value = factorial(&right_value, span, limits)?;
            stacks.push_checked(value, limits).map_err(at)?;
        }
        Operator::Une => {
            stacks.push_checked(right_value * Number::NaturalNumber(BigInt::from(-1)), limits)
                .map_err(at)?;
        }
        // The six comparisons ask `Number`'s own `PartialOrd`, which Stage 1
        // made agree with `PartialEq` by comparing mathematical value rather
        // than enum variant — so `2 == 6/3` is true with no code of its own.
        Operator::Less => stacks.push_truth(left_value < right_value, limits).map_err(at)?,
        Operator::Greater => stacks.push_truth(left_value > right_value, limits).map_err(at)?,
        Operator::LessEq => stacks.push_truth(left_value <= right_value, limits).map_err(at)?,
        Operator::GreaterEq => stacks.push_truth(left_value >= right_value, limits).map_err(at)?,
        Operator::Equal => stacks.push_truth(left_value == right_value, limits).map_err(at)?,
        Operator::NotEqual => stacks.push_truth(left_value != right_value, limits).map_err(at)?,
        // Both operands are already on the stack, so the `&&` below
        // short-circuits nothing: the right-hand expression was evaluated
        // before this arm was reached.
        Operator::And => stacks
            .push_truth(is_truthy(&left_value) && is_truthy(&right_value), limits)
            .map_err(at)?,
        Operator::Or => stacks
            .push_truth(is_truthy(&left_value) || is_truthy(&right_value), limits)
            .map_err(at)?,
        Operator::Xor => stacks
            .push_truth(is_truthy(&left_value) != is_truthy(&right_value), limits)
            .map_err(at)?,
        // Prefix, so the operand is the one `is_unary` left in `right_value`.
        Operator::Not => stacks.push_truth(!is_truthy(&right_value), limits).map_err(at)?,
        Operator::Mod => {
            // The zero check is `checked_div`'s, the one place this crate
            // decides what a division by zero is.
            let quotient = left_value
                .checked_div(&right_value)
                .ok_or(EvalError::DivisionByZero { span: Some(span) })?;
            // `From<Number> for BigInt` truncates toward zero rather than
            // flooring, which is what makes `-7 mod 3` be -1 and not 2.
            let truncated = Number::NaturalNumber(BigInt::from(quotient));
            stacks
                .push_checked(left_value - right_value * truncated, limits)
                .map_err(at)?;
        }
    }
    Ok(())
}
```

The factorial arm is ~20 lines today and would dominate this function. Extract it too, as `Expression::factorial(&Number, Span, Limits) -> Result<Number, EvalError>`, moving the existing body unchanged — including the predict-then-measure comment, which explains why both checks are there and must travel with the code.

- [ ] **Step 5: Reduce `eval_with` to a walk**

`eval_with` keeps the loop, the operand/variable/function/semicolon arms and the tail, and replaces the operator arm with one call:

```rust
                Token::Operator(op) => {
                    apply_operator(session, *op, t.span, limits, &mut stacks)?;
                }
```

Everywhere it said `result_stack` it now says `stacks.values`, and `var_stack` becomes `stacks.vars`. The `functions::eval` call becomes:

```rust
                    let result = functions::eval(*fun, value, &mut stacks.values, &mut stacks.vars)
                        .map_err(at)?;
```

Two disjoint field borrows in one call are fine; the borrow checker accepts them.

- [ ] **Step 6: Measure, and decide about the third extraction**

```bash
echo 'too-many-lines-threshold = 40' > clippy.toml
cargo clean -p yarer >/dev/null 2>&1
cargo clippy --all-targets --message-format=json 2>/dev/null \
  | grep -o 'too many lines ([0-9]*/40)' | sort -u
rm clippy.toml
```

`eval_with` should be near 63 and `apply_operator` near 70. If `apply_operator` is under 100, stop here and record both numbers in the commit message. If it is over, extract `apply_truth_operator(op, &left, &right, limits, stacks)` holding the ten truth arms, with an explicit arm listing the other six operators returning `EvalError::Malformed` and a comment saying it is unreachable because the caller's own match gates it.

- [ ] **Step 7: Run everything, including the proof it changed nothing**

```bash
cargo test
git show master:tests/integration_tests.rs \
  | grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' | sort > /tmp/baseline.txt
grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' tests/integration_tests.rs \
  | sort | comm -23 /tmp/baseline.txt -
# must print nothing
```

If a value assertion appears, stop: this refactor changed what an existing expression means, which is a finding and not a test to adjust.

- [ ] **Step 8: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/expression.rs
git commit -m "Split the evaluation loop from what its operators mean"
```

---

### Task 8: Clippy to zero

Spec component D, the sweep. Everything that is left after Tasks 2, 5 and 7 have incidentally fixed three of them.

**Files:**
- Modify: `src/token.rs`, `src/expression.rs`, `src/session.rs`, `src/lib.rs`, `src/validate.rs`, `src/functions.rs`

- [ ] **Step 1: See what is actually left**

```bash
cargo clean -p yarer >/dev/null 2>&1
cargo clippy --all-targets --all-features --message-format=json 2>/dev/null \
  | python3 -c "
import sys,json
seen=set()
for l in sys.stdin:
    try: m=json.loads(l)
    except: continue
    msg=m.get('message') or {}
    code=((msg.get('code') or {}).get('code') or '')
    if not code.startswith('clippy::'): continue
    for s in msg.get('spans',[]):
        if s.get('is_primary'):
            seen.add((s['file_name'], s['line_start'], code.replace('clippy::','')))
            break
for f,l,c in sorted(seen): print(f'{f}:{l}  {c}')
"
```

Work from this list, not from the spec's — Tasks 2, 5 and 7 will have removed `non_std_lazy_statics`, `unwrap_or_default` and one `too_many_lines`, and Tasks 5 and 6 will have added new code that may have introduced others.

- [ ] **Step 2: Take the mechanical ones with `--fix` where it is safe**

```bash
cargo clippy --all-targets --all-features --fix --allow-dirty
cargo test
git diff
```

`--fix` handles `needless_borrow`, `useless_vec`, `manual_midpoint` and `unwrap_or_default` reliably. **Read the diff.** `manual_midpoint` rewrites `(1.0 + 5.0f64.sqrt()) / 2.0` in `session.rs` as `1.0_f64.midpoint(5.0f64.sqrt())`, which is the same value by a route that cannot overflow — confirm `phi` is unchanged by running `cargo run -q -- -e "phi"` and comparing against `1.618033988749895`.

- [ ] **Step 3: Fix the documentation lints by hand**

`doc_markdown` wants backticks around anything that looks like an identifier. The five sites in `token.rs` are prose mentioning `BigInt`, `BigRational` and similar; wrap each in backticks. `doc_lazy_continuation` at `token.rs:444,445` wants a list continuation indented to line up with the text above it rather than the marker. Neither changes meaning; read each one and make the smallest edit that satisfies it.

- [ ] **Step 4: Deal with `needless_pass_by_value` case by case**

There are three: `token::apply_functional_token_operation`,
`expression::factorial_helper` and `expression::power` (the last two now free
functions after Task 7). They are not all the same, so for each decide whether
it genuinely wants ownership:

- If it does not, take `&T` and fix the callers.
- If it does — because it stores the value, or because the signature must match its siblings — keep it and add `#[expect(clippy::needless_pass_by_value, reason = "…")]` naming the reason.

`Number::decimal` already carries an `#[allow]` for this with a written
justification; convert it to `#[expect]` while you are there.

`apply_functional_token_operation` is worth looking at closely rather than
suppressing. It does `match (ln, rn.clone())` and the match consumes both by
value, so the clone is never used — a register entry since Stage 1, costing up
to roughly 128 KiB copied on every `+`, `-`, `*` and `/` under the default
budget. Deleting the `.clone()` is likely to satisfy clippy and discharge that
entry at the same time. Confirm with the benchmark harness
(`cargo test --release -- --ignored measure_the_cost`) that nothing regressed.

- [ ] **Step 5: Allow the one that cannot be fixed**

At the top of `src/lib.rs`, under the existing lint attributes:

```rust
// Every one of these is a `windows-*` duplicate — windows-sys 0.59 against
// 0.60, and nine windows_* target crates at 0.52.6 against 0.53 — reached
// transitively through clap, rustyline, dirs and env_logger. Nothing in this
// repository can resolve them. Re-check when those dependencies bump; if the
// list ever empties, this attribute starts warning and should be deleted.
#![allow(clippy::multiple_crate_versions)]
```

`#![expect]` is not usable at crate level for a lint that fires from the manifest rather than from a span in the source, so this one stays an `allow` with the re-check instruction in its comment.

- [ ] **Step 6: Justify the two long functions that stay long**

`validate::validate` and `functions::eval` keep their length and say why:

```rust
#[expect(
    clippy::too_many_lines,
    reason = "one match over Token variants, and the length is what buys five \
              distinct positioned diagnoses where there used to be five \
              identical 'malformed expression' failures. The Bracket(Close) \
              arm is the separable part if this ever has to shrink."
)]
pub(crate) fn validate<'a>(
```

```rust
#[expect(
    clippy::too_many_lines,
    reason = "a flat dispatch over eighteen built-ins; splitting it buys line \
              count and costs the ability to read the whole function table at \
              once."
)]
pub(crate) fn eval(
```

Convert the two `#[allow]`s on `limits::predicted_factorial_bits` to `#[expect]` with their existing justifications, which discharges a register entry.

- [ ] **Step 7: Verify zero, on a cold cache**

```bash
cargo clean -p yarer >/dev/null 2>&1
cargo clippy --all-targets --all-features -- -D warnings && echo "clippy is silent"
cargo clippy --no-default-features -- -D warnings && echo "slim is silent too"
```

Both must pass. The second matters: `#[expect]` on a function that is compiled out under a feature would itself warn.

- [ ] **Step 8: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add -A
git commit -m "Take clippy to zero so CI can start denying warnings"
```

---

### Task 9: Fuzzing, and a corpus that runs on stable

Spec component E.

**Files:**
- Create: `fuzz/Cargo.toml`, `fuzz/fuzz_targets/compile_eval.rs`, `fuzz/.gitignore`
- Create: `tests/fuzz_regressions.rs`, `tests/fuzz_regressions/` (seed files)
- Modify: `Cargo.toml` — add `fuzz/**` to `exclude`
- Modify: `.gitignore`

- [ ] **Step 1: Write the stable replay test first**

It runs on every push and needs no nightly, so it is the part that must exist even if the fuzzer never runs again. Create `tests/fuzz_regressions.rs`:

```rust
//! Every input the fuzzer ever crashed on, replayed on stable.
//!
//! `cargo fuzz` needs a nightly toolchain and runs on a schedule; this runs on
//! every push. A crash found once is copied into `tests/fuzz_regressions/` and
//! becomes a permanent test, so it cannot come back — and the corpus travels
//! inside the published package, unlike `fuzz/`, which is excluded from it.
//!
//! The assertion is only that nothing panics. What a given input *evaluates*
//! to is the integration suite's business; this file exists because
//! `Expression::compile` and `eval` take arbitrary text and must never abort
//! the process, which is the register's standing claim about this crate.

use yarer::{Expression, Limits, Session};

#[test]
fn test_no_corpus_input_panics() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fuzz_regressions");
    let entries = std::fs::read_dir(dir).expect("the corpus directory ships with the crate");

    let session = Session::init();
    // The same budget the fuzz target uses, so that an input which is fast
    // under the fuzzer is fast here.
    let limits = Limits::default().with_max_value_bits(4096);

    let mut checked = 0;
    for entry in entries {
        let path = entry.expect("readable entry").path();
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(&path).expect("readable file");
        let Ok(source) = std::str::from_utf8(&bytes) else {
            continue;
        };
        if let Ok(expr) = Expression::compile(source) {
            let _ = expr.eval_with(&session, limits);
        }
        checked += 1;
    }

    assert!(
        checked > 0,
        "the corpus is empty, so this test asserts nothing"
    );
}
```

- [ ] **Step 2: Seed the corpus**

Create `tests/fuzz_regressions/` and put one file per input, named for what it exercises. These are the shapes most likely to reach a panic — deep nesting, the unary rewrite, the guards, the two-character operators, and the inputs Stages 1 and 2 found defects in:

```bash
mkdir -p tests/fuzz_regressions
cd tests/fuzz_regressions
printf '%s' '((((((((((1))))))))))'        > deep_brackets
printf '%s' '-----------------5'           > repeated_unary
printf '%s' '2^2^2^2^2^2'                  > power_tower
printf '%s' '999999999!'                   > factorial_guard
printf '%s' '1/(10^400)'                   > underflow
printf '%s' 'max(1,*2)'                    > operator_in_value_position
printf '%s' ';;;;'                         > only_separators
printf '%s' '1 <= <= 2'                    > repeated_two_char_operator
printf '%s' 'not not not not 0'            > repeated_prefix
printf '%s' '1 - not 0'                    > prefix_after_binary
printf '%s' 'x=y=z=w=1'                    > chained_assignment
printf '%s' '2×3÷4'                        > multibyte_operators
printf '%s' '.5+.5'                        > leading_point_literals
printf '%s' '0.00000000000000000000001!'   > decimal_factorial
cd ../..
```

- [ ] **Step 3: Run it**

Run: `cargo test --test fuzz_regressions`
Expected: PASS, having checked 14 inputs. If any input panics, that is a real defect — stop and report it rather than removing the input.

- [ ] **Step 4: Create the fuzz crate**

```bash
cargo install cargo-fuzz     # if not present
cargo fuzz init --target compile_eval
```

`cargo fuzz init` writes `fuzz/Cargo.toml` with its own empty `[workspace]`
table, which is what keeps it out of the parent build. Confirm it is there;
without it `cargo build` at the root tries to build the fuzz crate on stable and
fails.

Change the generated dependency line to skip the CLI feature — the fuzz target
calls neither a line editor nor an argument parser, and building them for every
fuzz run is pure waste:

```toml
[dependencies.yarer]
path = ".."
default-features = false
```

Replace `fuzz/fuzz_targets/compile_eval.rs` with:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use yarer::{Expression, Limits, Session};

// Compile and evaluate arbitrary text. The assertion is implicit and is the
// register's standing claim about this crate: no input reaches a panic.
//
// The budget is tight on purpose. Without it the fuzzer spends its time in
// bignum arithmetic on inputs like `2^999999` instead of exploring the parser;
// with it, the predictive guards refuse those in microseconds without computing
// anything.
fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    if source.len() > 4096 {
        return;
    }
    let session = Session::init();
    let limits = Limits::default().with_max_value_bits(4096);
    if let Ok(expr) = Expression::compile(source) {
        let _ = expr.eval_with(&session, limits);
    }
});
```

- [ ] **Step 5: Keep the fuzzer's own corpus out of git and out of the package**

`fuzz/.gitignore`:

```
target/
corpus/
artifacts/
coverage/
```

and add `"fuzz/**"` to the `exclude` array in the root `Cargo.toml`, beside `"docs/superpowers/**"`.

- [ ] **Step 6: Run the fuzzer briefly, seeded from the committed corpus**

```bash
cargo +nightly fuzz run compile_eval tests/fuzz_regressions -- -max_total_time=120
```

Expected: no crashes in two minutes. If it finds one, copy the offending input from `fuzz/artifacts/compile_eval/` into `tests/fuzz_regressions/` with a descriptive name, confirm `cargo test --test fuzz_regressions` now fails, fix the defect, and confirm it passes. That sequence — reproduce on stable, then fix — is the whole reason the two directories exist.

- [ ] **Step 7: Verify the package still builds without `fuzz/`**

```bash
cargo publish --dry-run 2>&1 | grep -E 'Packaged|error'
cargo package --list | grep -c fuzz     # must be 0 for fuzz/, but
cargo package --list | grep fuzz_regr   # tests/fuzz_regressions must be present
```

The second and third are the packaging trap this design exists to avoid: if `tests/fuzz_regressions/` is missing from the package, the replay test fails for anyone testing a vendored copy.

- [ ] **Step 8: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add fuzz tests/fuzz_regressions tests/fuzz_regressions.rs Cargo.toml .gitignore
git commit -m "Fuzz the front to back path, and replay what it finds on stable"
```

---

### Task 10: CI that checks something

Spec component F, first half.

**Files:**
- Modify: `.github/workflows/rust.yml`

- [ ] **Step 1: Replace the workflow**

The current one runs `cargo build`, `cargo test`, and uploads to Codecov — and nothing in it generates coverage, so that upload has never had a file to send. It is removed rather than wired up; adding coverage tooling is a stage of its own, and a number nobody generates is worse than no number. Task 11 records that in the register.

```yaml
name: Rust

on:
  push:
    branches: [ "master" ]
  pull_request:
    branches: [ "master" ]
  schedule:
    # Weekly, Sunday 03:17 UTC. The fuzz job only.
    - cron: '17 3 * * 0'

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    if: github.event_name != 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --verbose
      - run: cargo test --verbose

  fmt:
    if: github.event_name != 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo fmt --check

  clippy:
    if: github.event_name != 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo clippy --all-targets --all-features -- -D warnings

  slim:
    # The library must build and test without the CLI feature. Nothing else
    # would catch a `use clap::..` finding its way into library code, and the
    # whole point of the feature is that a library user does not compile it.
    if: github.event_name != 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --no-default-features --verbose
      - run: cargo test --no-default-features --verbose

  msrv:
    # An unverified rust-version is a comment. This is what makes it a claim.
    if: github.event_name != 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.86.0
      - run: cargo build --all-features
      - run: cargo build --no-default-features
      - run: cargo test --all-features

  fuzz:
    if: github.event_name == 'schedule'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo install cargo-fuzz
      - run: cargo fuzz run compile_eval tests/fuzz_regressions -- -max_total_time=600
```

Use whatever version Task 4 actually measured in the `msrv` job's toolchain line, not `1.86.0` if they differ. The version appears in two places — `Cargo.toml` and here — and they must agree; Task 11's register entry notes that as a small maintenance hazard.

- [ ] **Step 2: Run every job's command locally**

```bash
cargo build --verbose
cargo test --verbose
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build --no-default-features --verbose
cargo test --no-default-features --verbose
cargo +<floor> build --all-features
cargo +<floor> test --all-features
```

Every one must pass before the workflow is committed. A CI file that fails on its first run is worse than none, because the failure looks like the workflow's fault rather than the code's.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/rust.yml
git commit -m "Check formatting, lints, the slim build and the MSRV in CI"
```

---

### Task 11: The CHANGELOG, the register, and 0.4.0

Spec component F, second half, plus the release.

**Files:**
- Create: `CHANGELOG.md`
- Modify: `README.md`, `Cargo.toml`, `docs/tech-debt.md`, `src/lib.rs`

- [ ] **Step 1: Write `CHANGELOG.md`**

Keep a Changelog format, newest first, covering 0.1.1 through 0.4.0. The 0.2.0 and 0.3.0 entries are written from the README's existing News sections — move the content, do not paraphrase it, and keep the 0.3.0 migration table intact, because that table is what an upgrader needs and it took two stages to write.

The 0.4.0 entry, in full:

```markdown
## [0.4.0] - 2026-08-26

### Added
- `yarer -e "1+2"` evaluates an expression and exits. May be repeated; all the
  expressions share one session.
- Piped or redirected input is evaluated a line at a time against one session,
  as in GNU bc. The interactive REPL is unchanged and is still what you get
  from a terminal.
- A `cli` feature, on by default. `yarer = { version = "0.4",
  default-features = false }` builds the library without `clap`, `rustyline`,
  `dirs` or `env_logger` — 41 crates instead of 73.
- `rust-version = "1.86"`, verified in CI at exactly that toolchain.
- `EvalError::OperandTooSmallForFloat`, the mirror of
  `OperandTooLargeForFloat`.

### Changed
- Values go to stdout and errors to stderr, and the binary exits 1 on the first
  failure. Nothing after a failing expression runs. The startup banner is no
  longer printed in non-interactive modes, where it would corrupt a captured
  value.
- `bigdecimal` and `lazy_static` are gone — nothing referenced them.
  `once_cell` is gone too, replaced by `std::sync::LazyLock`.

### Fixed
- An operand too small to be represented as an `f64` is refused instead of
  being silently replaced by zero. `log(1/(10^400))` is `-400` and was reported
  as "not a real number"; `sqrt(1/(10^400))` is `1e-200` and answered `0`.
- `f64::try_from` on such a value answers `Err(OutOfRange)` rather than
  `Ok(0.0)`.
- Two of the README's CLI transcripts documented behaviour the binary does not
  have. They are corrected, and executed by a test now.

### Removed
- Functions whose operand underflows to zero no longer answer as though the
  operand were zero. `sin(1/(10^400))` returned `0`, which was correct, and is
  now refused with `OperandTooSmallForFloat`. This is the same trade Stage 2
  made on the overflow side when `atan(10^400)` stopped returning `pi/2`: one
  rule about what can be represented, applied in both directions, is worth more
  than a handful of correct answers at the extreme edge of the value space.
```

- [ ] **Step 2: Shrink the README's News section**

Replace the News sections for 0.1.x through 0.3.0 with a link to `CHANGELOG.md` and keep a short "what's new in 0.4.0" summary in the README — five or six bullets, not the full table. Add the slim-build instruction under a new heading:

```markdown
## Using it as a library only

The command-line binary's dependencies are behind a feature that is on by
default, so `cargo install yarer` works unchanged. A program that only wants
the evaluator can turn it off:

```toml
yarer = { version = "0.4", default-features = false }
```

which drops `clap`, `rustyline`, `dirs` and `env_logger` and takes the
dependency tree from 73 crates to 41.
```

- [ ] **Step 3: Update `docs/tech-debt.md`**

Six entries. Add:

- **`statrs` is 26 of the 41 crates a slim build compiles**, for one import — `Normal`, used by `pdf` and `cdf`. It pulls `nalgebra`, `matrixmultiply` and `rand`. Replacing it means hand-writing an erf approximation, which moves numbers the suite pins and the README's Black–Scholes example quotes, so it is a correctness change rather than a dependency one.
- **There is no coverage measurement.** The Codecov step was removed because nothing generated a report for it to upload. `cargo llvm-cov` is the obvious way back if coverage is wanted.
- **The MSRV is declared in two places** — `Cargo.toml` and the `msrv` CI job's toolchain line — and nothing checks that they agree.
- **`src/token.rs` is over 1100 lines** and holds `Number`, `Operator`, `Token`, `MathFunction` and every conversion between them. `Number` and its conversions are the separable part.
- **Script mode has no comment syntax**, which is the first thing a piped file of expressions wants. The README used to document a `//` comment that never existed.

Update the existing `too_many_lines` entry with the numbers Task 7 measured, and remove the `Expression::eval_with` entry that Task 7 discharged, replacing it with a sentence saying what it became. Convert the `#[allow]`-should-be-`#[expect]` entry to done.

- [ ] **Step 4: Bump the version**

```toml
version = "0.4.0"
```

and update the `Yarer v.0.3.0` banner text in `README.md` and `src/lib.rs`'s ignored doc block to `v.0.4.0`. The README transcript test will fail if the banner lines drift, which is what it is for.

- [ ] **Step 5: Run everything, one last time**

```bash
cargo clean -p yarer
cargo test
cargo test --no-default-features
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo publish --dry-run
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Release 0.4.0"
```

---

## Definition of done for the work

```bash
cargo clean -p yarer
cargo test                                                  # green
cargo test --no-default-features                            # green
cargo fmt --check                                           # silent
cargo clippy --all-targets --all-features -- -D warnings    # silent
cargo +<floor> build --all-features                         # green
grep -rn 'to_f64' src/                                      # one site
cargo tree --edges normal --prefix none --no-default-features \
  | awk '{print $1}' | sort -u | wc -l                      # about 41
```

- The 143 value assertions present and unmodified, checked with `comm -23` against `master`, not with a digest.
- `eval_with` and `apply_operator` both under 100 lines, with no `too_many_lines` suppression between them.
- `log(1/(10^400))`, `ln(1/(10^400))` and `sqrt(1/(10^400))` raise `OperandTooSmallForFloat` with a span; the withdrawal of `sin(1/(10^400))` is in the CHANGELOG.
- `yarer -e`, a pipe, and the REPL each covered by tests that spawn the binary.
- The README's CLI transcripts executed by `tests/readme.rs` and passing.
- `tests/fuzz_regressions/` present in `cargo package --list`; `fuzz/` absent from it.
- `rust-version` declared, and the same version in the `msrv` CI job.
- `CHANGELOG.md` covering 0.1.x through 0.4.0.
- `Cargo.toml` at `0.4.0`.
