# Public API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace yarer's stringly-typed `anyhow` errors with typed errors that carry source positions, add the validation pass that gives five currently indistinguishable malformed expressions five distinct diagnoses, and close the two public functions that panic on ordinary input.

**Architecture:** The pipeline grows a stage and loses a coupling. `&str` → tokenise (now keeping byte spans) → **validate** (a new pass: a two-state machine that both marks unary operators and refuses illegal token sequences) → shunting yard (now only Dijkstra's algorithm) → `Expression`. Evaluation becomes `expr.eval(&session)`, so a parse failure is reported by `compile` instead of being flattened into a `String` and deferred to the first evaluation. `Rc<RefCell<..>>` stays inside `Session`: `Send`/`Sync` is out of scope.

**Tech Stack:** Rust 2021. `num-bigint`/`num-rational` for exact arithmetic, `num-traits` for numeric predicates, `statrs` for the normal distribution, `thiserror` for the error types, `rustyline`+`clap` for the binary. `anyhow` is **removed** during Task 7.

**Spec:** `docs/superpowers/specs/2026-08-25-public-api-design.md`. Read it before starting; this plan argues from it and does not repeat its reasoning.

## Global Constraints

- Branch: `production-ready-api`, already created, already holding the spec.
- **Never run `git stash` in this repository.** Two user stashes from June 2025 live here and must not be disturbed. If you need a clean tree, use `git worktree`.
- After any edit, confirm the build actually recompiled: cargo occasionally skips it. If `cargo test` output has no `Compiling yarer` line after a source change, run `cargo clean -p yarer` and try again.
- Every task ends with `cargo test` green and `cargo fmt --check` clean.
- Clippy is compared **per lint**, never by counting warnings: run `cargo clean -p yarer` first, then
  measure through clippy's JSON output. The human-readable output cannot be counted:
  it prints a `= note: #[warn(clippy::x)]` line once per lint *kind*, at that lint's
  first occurrence, so a second offence of the same lint is invisible. Only the JSON
  carries one record per diagnostic. The command is `cargo clippy --all-targets --message-format=json 2>/dev/null \
  | grep -oP '"code":"clippy::[a-z_]+"' | sed 's/.*clippy:://; s/"//' | sort | uniq -c | sort -rn`. A stable total can hide one regression cancelling one improvement. The baseline at the start of this stage is whatever that command prints on `master`; record it in Task 1 and compare against it, not against a number quoted here.
- Do not add dependencies. `thiserror` is already in `Cargo.toml`.
- `Cargo.toml` stays at version `0.2.0`. The bump to `0.3.0` goes with the release.
- Commit messages: imperative subject line, no tool attribution of any kind, no `Co-Authored-By` trailer.
- Error message text is lower case and carries no `Parse Error:` / `Runtime error:` prefix. The prefix is added once, by `Error::render`.
- Undefined variables keep evaluating to `0`. Chained assignment (`x=y=5`) and chained expressions (`x=2; y=3; x*y`) keep working. `sin[5]` keeps evaluating.
- Every numeric result Stage 1 pinned stays pinned. If a test that asserts a *value* changes, you have broken something; only tests that assert *errors* change in this stage.

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `src/span.rs` (new) | `Span` (public, byte offsets) and `Spanned<T>` (internal) | 1 |
| `src/error.rs` (new) | `Error`, `ParseError`, `EvalError`, `Display`, `span()`, `render()` | 1 |
| `src/parser.rs` | tokenising only; produces `Vec<Spanned<Token>>` | 2 |
| `src/validate.rs` (new) | the two-state machine: unary marking, sequence rules, bracket frames, arity | 3 |
| `src/shunting.rs` (new, from `rpn_resolver.rs`) | infix → RPN, nothing else | 4 |
| `src/expression.rs` (new, from `rpn_resolver.rs`) | `Expression`, `compile`, `eval`, `eval_with`, the evaluation loop | 5, 6 |
| `src/rpn_resolver.rs` | deleted at the end of Task 6 | 6 |
| `src/token.rs` | `Number`, `Token`, `MathFunction`; `checked_div`; total `operator_priority` | 7 |
| `src/limits.rs` | budget checks, now returning `EvalError` | 5 |
| `src/functions.rs` | built-in evaluation, now returning `EvalError` | 5 |
| `src/session.rs` | the variable heap, `lookup`/`assign`, fallible setters, `limits()` | 6, 8 |
| `src/lib.rs` | module wiring, root re-exports, crate docs | 10 |
| `src/bin/main.rs` | the REPL; first consumer of `render` | 11 |

---

### Task 1: The error module

Component A and the renderer of component B. Nothing consumes these types yet, so this task lands green on its own.

**Files:**
- Create: `src/span.rs`
- Create: `src/error.rs`
- Modify: `src/lib.rs` — add `mod span;`, `mod error;` and the two re-exports

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct Span { pub start: usize, pub end: usize }` with `Span::new(start: usize, end: usize) -> Span`, `Copy`, `PartialEq`, `Eq`, `Debug`
  - `pub(crate) struct Spanned<T> { pub node: T, pub span: Span }`, `Clone`, `PartialEq`, `Debug`
  - `pub enum ParseError` and `pub enum EvalError` exactly as listed below
  - `pub enum Error { Parse(ParseError), Eval(EvalError) }` with `From` for both
  - `Error::span(&self) -> Option<Span>`, `Error::render(&self, source: &str) -> String`
  - `ParseError::span(&self) -> Option<Span>`, `EvalError::span(&self) -> Option<Span>`

- [ ] **Step 1: Record the clippy baseline**

Run and paste the output into the task's commit message:

```bash
cargo clean -p yarer && cargo clippy --all-targets --message-format=json 2>/dev/null \
  | grep -oP '"code":"clippy::[a-z_]+"' | sed 's/.*clippy:://; s/"//' | sort | uniq -c | sort -rn
```

- [ ] **Step 2: Write the failing tests**

Create `src/error.rs` containing only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_puts_the_caret_under_the_offending_token() {
        let err = Error::Parse(ParseError::ExpectedValue {
            found: "*".to_string(),
            span: Span::new(6, 7),
        });
        assert_eq!(
            err.render("max(1,*2)"),
            "Parse error: expected a value, found '*'\n  max(1,*2)\n        ^"
        );
    }

    /// '×' is two bytes wide and one column wide. A caret positioned by byte
    /// offset lands one column past the token it is meant to point at, and no
    /// other test in the suite can notice, because the message stays correct.
    #[test]
    fn test_render_counts_columns_in_chars_not_bytes() {
        let err = Error::Eval(EvalError::DivisionByZero {
            span: Some(Span::new(3, 4)),
        });
        assert_eq!(
            err.render("2×3"),
            "Eval error: division by zero\n  2×3\n    ^"
        );
    }

    #[test]
    fn test_render_without_a_span_prints_only_the_message() {
        let err = Error::Eval(EvalError::NotFinite { value: f64::NAN });
        assert_eq!(err.render("whatever"), "Eval error: NaN is not a finite number");
    }

    /// `render` takes the source from the caller, who may hand it the wrong
    /// string. Slicing it must not panic.
    #[test]
    fn test_render_survives_a_span_that_does_not_fit_the_source() {
        let err = Error::Parse(ParseError::UnbalancedBracket {
            span: Span::new(50, 51),
        });
        assert_eq!(err.render("1+1"), "Parse error: unbalanced brackets");
    }

    #[test]
    fn test_the_two_size_errors_are_distinguishable_without_reading_the_text() {
        let measured = EvalError::ValueTooLarge { bits: 12, limit: 8, span: None };
        let predicted = EvalError::ComputationTooLarge { predicted_bits: 12, limit: 8, span: None };
        assert!(matches!(measured, EvalError::ValueTooLarge { .. }));
        assert!(matches!(predicted, EvalError::ComputationTooLarge { .. }));
        assert_ne!(measured.to_string(), predicted.to_string());
    }

    #[test]
    fn test_a_parse_error_converts_into_the_union_type() {
        let err: Error = ParseError::EmptyExpression.into();
        assert!(matches!(err, Error::Parse(ParseError::EmptyExpression)));
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --lib error`
Expected: compilation failure — `Error`, `ParseError`, `EvalError`, `Span` do not exist.

- [ ] **Step 4: Write `src/span.rs`**

```rust
//! Byte offsets into the expression an error came from.

/// A half-open range of bytes in the source expression.
///
/// Offsets are in bytes because that is what the tokeniser produces. Turning
/// them into terminal columns is [`crate::error::Error::render`]'s job, and it
/// is not a cast: `×` is two bytes and one column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Offset of the first byte.
    pub start: usize,
    /// Offset one past the last byte.
    pub end: usize,
}

impl Span {
    /// Builds a span from a half-open byte range.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }
}

/// A value carrying the span of the text it came from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub(crate) fn new(node: T, span: Span) -> Spanned<T> {
        Spanned { node, span }
    }
}
```

- [ ] **Step 5: Write `src/error.rs`**

Above the test module written in Step 2:

```rust
//! The errors yarer reports, and where in the expression they happened.
//!
//! Two kinds, kept apart in the type system rather than in a message prefix:
//! [`ParseError`] is produced while an expression is being compiled, and
//! [`EvalError`] while a compiled expression is being evaluated. A caller that
//! wants one type across both calls converts into [`Error`].
//!
//! Both enums are `#[non_exhaustive]`: yarer will grow error conditions, and
//! that must not be a breaking change. Match with a `_` arm.

use crate::span::Span;
use crate::token::MathFunction;

/// A failure while compiling an expression.
///
/// Every variant carries the position of the token it is about, with one
/// exception: [`ParseError::Malformed`] names no token — it is the structural
/// fallback for a condition the validation pass is supposed to have made
/// unreachable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// Text that is not part of any token, e.g. the `@` in `1@2`.
    #[error("unexpected character '{text}'")]
    UnexpectedCharacter { text: String, span: Span },
    /// The expression contains no tokens at all.
    #[error("the expression is empty")]
    EmptyExpression,
    /// A bracket pair enclosing nothing, e.g. `()`.
    #[error("empty brackets are not a value")]
    EmptyGroup { span: Span },
    /// A bracket that is never closed, or one that closes nothing.
    #[error("unbalanced brackets")]
    UnbalancedBracket { span: Span },
    /// A `,` with no bracket open at all.
    #[error("',' is only valid between the arguments of a function call")]
    CommaOutsideCall { span: Span },
    /// A `,` inside brackets that group a value rather than open a call.
    #[error("',' separates function arguments, but these brackets group a value")]
    CommaInPlainBracket { span: Span },
    /// An argument slot with nothing in it, e.g. `max(1,)`.
    #[error("a function argument cannot be empty")]
    EmptyArgument { span: Span },
    /// A call with the wrong number of arguments.
    #[error("function '{}' expects {expected} argument(s), {given} given", .function.to_string().to_lowercase())]
    WrongArity {
        function: MathFunction,
        expected: u8,
        given: usize,
        span: Span,
    },
    /// A function name not followed by an opening bracket.
    #[error("function '{}' must be followed by '(' or '['", .function.to_string().to_lowercase())]
    FunctionRequiresParentheses { function: MathFunction, span: Span },
    /// A `;` reached with a bracket still open.
    #[error("a bracket must be closed before ';'")]
    BracketUnclosedAtSemicolon { span: Span },
    /// A value was required here and something else appeared.
    #[error("expected a value, found '{found}'")]
    ExpectedValue { found: String, span: Span },
    /// An operator was required here and something else appeared.
    #[error("expected an operator, found '{found}'")]
    ExpectedOperator { found: String, span: Span },
    /// The structural fallback. Reaching it means an invariant this crate
    /// maintains has been broken, not that the user wrote something odd.
    #[error("the expression is malformed")]
    Malformed,
}

/// A failure while evaluating a compiled expression.
///
/// Spans are optional here because an evaluation error need not come from
/// source text at all: `Session::setf("x", f64::NAN)` fails with no expression
/// in existence.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum EvalError {
    #[error("division by zero")]
    DivisionByZero { span: Option<Span> },
    /// A value that was computed and measured over the budget.
    #[error("the result occupies about {bits} bits, over the size limit of {limit} bits")]
    ValueTooLarge { bits: u128, limit: u64, span: Option<Span> },
    /// A computation refused before running, on an estimate of its size.
    #[error("the result would need about {predicted_bits} bits, over the size limit of {limit} bits")]
    ComputationTooLarge { predicted_bits: u128, limit: u64, span: Option<Span> },
    #[error("factorial is only defined for non-negative integers")]
    FactorialNotNatural { span: Option<Span> },
    #[error("the factorial operand is too large")]
    FactorialOperandTooLarge { span: Option<Span> },
    #[error("the exponent is too large to evaluate under any size limit")]
    ExponentTooLarge { span: Option<Span> },
    #[error("power operands are too large for non-integer evaluation")]
    PowerOperandsTooLarge { span: Option<Span> },
    #[error("invalid power operation")]
    InvalidPower { span: Option<Span> },
    #[error("operand is too large for floating-point evaluation")]
    OperandTooLargeForFloat { span: Option<Span> },
    #[error("function result is not a real number")]
    NotARealNumber { span: Option<Span> },
    #[error("'{name}' is a built-in constant and is read-only")]
    ReadOnlyConstant { name: String, span: Option<Span> },
    #[error("no variable has been defined for assignment")]
    AssignmentTargetMissing { span: Option<Span> },
    #[error("{value} is not a finite number")]
    NotFinite { value: f64 },
    /// The evaluation stack did not end with exactly one value. Same status as
    /// [`ParseError::Malformed`]: a broken invariant, not a user mistake.
    #[error("the expression is malformed")]
    Malformed { span: Option<Span> },
}

/// Either kind of failure, for callers that want one type across `compile` and
/// `eval`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Eval(#[from] EvalError),
}
```

Then the accessors. `ParseError::span` and `EvalError::span` are written as
matches with no catch-all arm, so that adding a variant without giving it a
position is a compile error rather than a silent `None`:

```rust
impl ParseError {
    /// Where in the expression this error is, if it is about a token.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        match self {
            ParseError::UnexpectedCharacter { span, .. }
            | ParseError::EmptyGroup { span }
            | ParseError::UnbalancedBracket { span }
            | ParseError::CommaOutsideCall { span }
            | ParseError::CommaInPlainBracket { span }
            | ParseError::EmptyArgument { span }
            | ParseError::WrongArity { span, .. }
            | ParseError::FunctionRequiresParentheses { span, .. }
            | ParseError::BracketUnclosedAtSemicolon { span }
            | ParseError::ExpectedValue { span, .. }
            | ParseError::ExpectedOperator { span, .. } => Some(*span),
            ParseError::EmptyExpression | ParseError::Malformed => None,
        }
    }
}

impl EvalError {
    /// Where in the expression this error is, if it came from one.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        match self {
            EvalError::DivisionByZero { span }
            | EvalError::ValueTooLarge { span, .. }
            | EvalError::ComputationTooLarge { span, .. }
            | EvalError::FactorialNotNatural { span }
            | EvalError::FactorialOperandTooLarge { span }
            | EvalError::ExponentTooLarge { span }
            | EvalError::PowerOperandsTooLarge { span }
            | EvalError::InvalidPower { span }
            | EvalError::OperandTooLargeForFloat { span }
            | EvalError::NotARealNumber { span }
            | EvalError::ReadOnlyConstant { span, .. }
            | EvalError::AssignmentTargetMissing { span }
            | EvalError::Malformed { span } => *span,
            EvalError::NotFinite { .. } => None,
        }
    }
}

impl Error {
    /// Where in the expression this error is, if it is about a position.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        match self {
            Error::Parse(e) => e.span(),
            Error::Eval(e) => e.span(),
        }
    }

    /// The message, and — when the error has a position and `source` is the
    /// expression it came from — the expression with a caret under it.
    ///
    /// The category prefix is added here, once, rather than being baked into a
    /// dozen message constants where it has already drifted.
    #[must_use]
    pub fn render(&self, source: &str) -> String {
        let (prefix, message) = match self {
            Error::Parse(e) => ("Parse error", e.to_string()),
            Error::Eval(e) => ("Eval error", e.to_string()),
        };
        let head = format!("{prefix}: {message}");

        let Some(span) = self.span() else {
            return head;
        };
        // The caller supplies `source`, and may supply the wrong one. Both
        // slices are taken through `get`, so a span that does not fit — or that
        // lands inside a multi-byte character — degrades to the message alone
        // instead of panicking.
        let Some(before) = source.get(..span.start) else {
            return head;
        };
        let Some(token) = source.get(span.start..span.end) else {
            return head;
        };
        let column = before.chars().count();
        // A zero-width span still gets one caret: an error at end of input has
        // nothing to underline but still has somewhere to point.
        let width = token.chars().count().max(1);
        format!(
            "{head}\n  {source}\n  {}{}",
            " ".repeat(column),
            "^".repeat(width)
        )
    }
}
```

- [ ] **Step 6: Wire the modules into `src/lib.rs`**

Add next to the existing module declarations:

```rust
/// Typed errors
pub mod error;
mod span;

pub use error::{Error, EvalError, ParseError};
pub use span::Span;
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test --lib error`
Expected: 6 passed.

- [ ] **Step 8: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/error.rs src/span.rs src/lib.rs
git commit -m "Give every failure a name and a place"
```

---

### Task 2: Spans through the parser

Component B's producing half. `Parser` keeps only tokenising; the unary pass stays here for now and moves in Task 3.

**Files:**
- Modify: `src/parser.rs` — the whole file
- Modify: `src/token.rs:251` — `tokenize` becomes total
- Modify: `src/rpn_resolver.rs:96-120` — bridge the new error type into the still-`anyhow` pipeline

**Interfaces:**
- Consumes: `Span`, `Spanned`, `ParseError` from Task 1.
- Produces:
  - `pub(crate) fn Parser::parse(expr: &str) -> Result<Vec<Spanned<Token<'_>>>, ParseError>`
  - `pub(crate) fn Token::tokenize(t: &str) -> Token<'_>` — no longer `Option`

- [ ] **Step 1: Write the failing tests**

In `src/parser.rs`'s test module, replacing `test_parse_valid`'s error handling and adding:

```rust
#[test]
fn test_parse_records_the_span_of_every_token() {
    let tokens = Parser::parse("1 + 23").unwrap();
    let spans: Vec<(usize, usize)> = tokens.iter().map(|t| (t.span.start, t.span.end)).collect();
    assert_eq!(spans, vec![(0, 1), (2, 3), (4, 6)]);
}

#[test]
fn test_parse_reports_an_unexpected_character_with_its_position() {
    assert_eq!(
        Parser::parse("1@2"),
        Err(ParseError::UnexpectedCharacter {
            text: "@".to_string(),
            span: Span::new(1, 2),
        })
    );
}

#[test]
fn test_parse_rejects_an_expression_with_no_tokens() {
    assert_eq!(Parser::parse("   "), Err(ParseError::EmptyExpression));
}

/// The unary minus keeps the position of the '-' it replaces, so an error
/// reported against it later points at something the user actually typed.
/// '×' is two bytes and one column. Every other test in this module uses pure
/// ASCII, where the byte offset and the char offset are the same number, so
/// none of them would notice if these spans started being counted in chars —
/// while every caret in the crate would silently shift on any expression
/// containing '×' or '÷'.
#[test]
fn test_spans_are_byte_offsets_not_char_offsets() {
    let tokens = Parser::parse("2×3").unwrap();
    let spans: Vec<(usize, usize)> = tokens.iter().map(|t| (t.span.start, t.span.end)).collect();
    assert_eq!(spans, vec![(0, 1), (1, 3), (3, 4)]);
}

#[test]
fn test_the_unary_minus_keeps_its_position() {
    let tokens = Parser::parse("-5").unwrap();
    assert_eq!(tokens[0].node, Token::Operator(Operator::Une));
    assert_eq!(tokens[0].span, Span::new(0, 1));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib parser`
Expected: compilation failure — `parse` returns `Vec<Token>`, which has no `.span`.

- [ ] **Step 3: Make `Token::tokenize` total**

In `src/token.rs`, the `None => return None` arm exists for an empty string, and
the regex never produces an empty match. Change the signature to
`pub(crate) fn tokenize(t: &str) -> Token<'_>` and replace the outer `match` on
`t.chars().next()` with `if let Some(s) = t.chars().next()`, dropping the `None`
arm. Every `return Some(x)` becomes `return x`, and the final line returns
`Token::Variable(t)`.

- [ ] **Step 4: Rewrite `Parser::parse` and `mod_unary_operators`**

```rust
impl Parser {
    /// Splits `expr` into tokens, each carrying the byte range it came from.
    pub(crate) fn parse(expr: &str) -> Result<Vec<Spanned<Token<'_>>>, ParseError> {
        let mut vex: Vec<Spanned<Token<'_>>> = Vec::new();
        let mut cursor = 0;

        for m in EXPRESSION_REGEX.find_iter(expr) {
            Self::validate_gap(expr, cursor, m.start())?;
            vex.push(Spanned::new(
                Token::tokenize(m.as_str()),
                Span::new(m.start(), m.end()),
            ));
            cursor = m.end();
        }

        Self::validate_gap(expr, cursor, expr.len())?;

        if vex.is_empty() {
            return Err(ParseError::EmptyExpression);
        }

        Ok(Self::mod_unary_operators(&vex))
    }

    fn validate_gap(expr: &str, start: usize, end: usize) -> Result<(), ParseError> {
        let gap = &expr[start..end];
        if gap.chars().all(char::is_whitespace) {
            return Ok(());
        }
        // The span covers the trimmed text, not the surrounding whitespace, so
        // the caret sits under the offending characters themselves.
        let leading = gap.len() - gap.trim_start().len();
        let trimmed = gap.trim();
        Err(ParseError::UnexpectedCharacter {
            text: trimmed.to_string(),
            span: Span::new(start + leading, start + leading + trimmed.len()),
        })
    }
}
```

`mod_unary_operators` keeps its current logic and gains spans: it iterates
`&[Spanned<Token<'a>>]`, pushes `token.clone()` where it pushed a token, skips
the unary `+` as before, and pushes
`Spanned::new(Token::Operator(Operator::Une), token.span)` where it pushed
`Une`. Its `match` inspects `&token.node`.

- [ ] **Step 5: Bridge into the still-`anyhow` pipeline**

In `src/rpn_resolver.rs`, `parse_with_borrowed_heap` currently chains
`Parser::parse(exp).and_then(...)`. The two error types now differ, so convert:

```rust
match Parser::parse(exp)
    .map_err(anyhow::Error::from)
    .and_then(|tokenised_expr| {
        RpnResolver::reverse_polish_notation(&tokenised_expr, heap_for_parse)
    })
```

and `reverse_polish_notation` takes `&[Spanned<Token<'a>>]`, reading `t.node`
where it read `*t` and `t.clone()` where it cloned. Its output stays
`VecDeque<Token>` for now — spans reach the evaluation loop in Task 5. This is
scaffolding, deliberately thrown away two tasks later; keeping it means Tasks 2
and 3 each land green.

- [ ] **Step 6: Run the whole suite**

Run: `cargo test`
Expected: all green. The four new parser tests pass; no existing test changes.

- [ ] **Step 7: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/parser.rs src/token.rs src/rpn_resolver.rs
git commit -m "Stop throwing away the positions the tokeniser already knows"
```

---

### Task 3: The validation pass

Component C. The largest task in the plan, and the one that gives five expressions five diagnoses. `mod_unary_operators` moves out of `parser.rs`, keeps everything it did, and gains the ability to refuse.

The bracket bookkeeping moves here too, from the shunting yard, in the same task — the two share one traversal, and splitting them across tasks would mean writing the frame logic twice. Until Task 4 the yard keeps its own copies; they simply stop being reachable, because validation runs first and refuses the same inputs earlier with better messages.

**Files:**
- Create: `src/validate.rs`
- Modify: `src/parser.rs` — delete `mod_unary_operators` and its test, stop calling it
- Modify: `src/rpn_resolver.rs` — call `validate` between `Parser::parse` and `reverse_polish_notation`
- Modify: `src/lib.rs` — `mod validate;`

**Interfaces:**
- Consumes: `Parser::parse -> Result<Vec<Spanned<Token>>, ParseError>` (Task 2), `Span`, `Spanned`, `ParseError` (Task 1), `MathFunction::arity(self) -> u8` (existing, `token.rs:161`).
- Produces: `pub(crate) fn validate<'a>(tokens: &[Spanned<Token<'a>>], source: &str) -> Result<Vec<Spanned<Token<'a>>>, ParseError>`

**Why `validate` takes the source text.** The `found` field of `ExpectedValue` and `ExpectedOperator` quotes what the user typed. `Token`'s own `Display` wraps everything in parentheses — `Token::Operator(Mul)` prints as `(*)` — so building the message from the token would produce `expected a value, found '(*)'`. Slicing the source by the token's span gives the real text, including the difference between `*` and `×`.

- [ ] **Step 1: Write the failing tests**

In `src/validate.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn check(source: &str) -> Result<Vec<Spanned<Token<'_>>>, ParseError> {
        let tokens = Parser::parse(source).expect("tokenises");
        validate(&tokens, source)
    }

    /// One case per error-producing rule. Each input is one that today lands in
    /// a *different* guard — four of them in the generic malformed message and
    /// one, `max(1,*2)`, in an arity complaint about `max`, whose arity is
    /// correct. Asserting the exact variant is what makes these tests fail for
    /// the right reason.
    #[test]
    fn test_every_illegal_sequence_gets_its_own_diagnosis() {
        let cases: Vec<(&str, ParseError)> = vec![
            ("()", ParseError::EmptyGroup { span: Span::new(1, 2) }),
            ("2 3", ParseError::ExpectedOperator { found: "3".to_string(), span: Span::new(2, 3) }),
            ("2(3+4)", ParseError::ExpectedOperator { found: "(".to_string(), span: Span::new(1, 2) }),
            ("1+", ParseError::ExpectedValue { found: "end of expression".to_string(), span: Span::new(2, 2) }),
            ("max(1,*2)", ParseError::ExpectedValue { found: "*".to_string(), span: Span::new(6, 7) }),
            ("!5", ParseError::ExpectedValue { found: "!".to_string(), span: Span::new(0, 1) }),
            ("(1,2)", ParseError::CommaInPlainBracket { span: Span::new(2, 3) }),
            ("1,2", ParseError::CommaOutsideCall { span: Span::new(1, 2) }),
            ("1+;2", ParseError::ExpectedValue { found: ";".to_string(), span: Span::new(2, 3) }),
        ];
        for (source, expected) in cases {
            assert_eq!(check(source), Err(expected), "for input {source}");
        }
    }

    /// The bracket and arity rules that used to live in the shunting yard.
    /// Their messages are unchanged; only their home and their positions are new.
    #[test]
    fn test_the_bracket_rules_moved_without_changing_what_they_say() {
        let cases: Vec<(&str, ParseError)> = vec![
            ("max()", ParseError::WrongArity { function: MathFunction::Max, expected: 2, given: 0, span: Span::new(0, 3) }),
            ("max(1)", ParseError::WrongArity { function: MathFunction::Max, expected: 2, given: 1, span: Span::new(0, 3) }),
            ("max(1,2,3)", ParseError::WrongArity { function: MathFunction::Max, expected: 2, given: 3, span: Span::new(0, 3) }),
            ("sin(1,2)", ParseError::WrongArity { function: MathFunction::Sin, expected: 1, given: 2, span: Span::new(0, 3) }),
            ("max(1,)", ParseError::EmptyArgument { span: Span::new(6, 7) }),
            ("max(,1)", ParseError::EmptyArgument { span: Span::new(4, 5) }),
            ("sin 5", ParseError::FunctionRequiresParentheses { function: MathFunction::Sin, span: Span::new(0, 3) }),
            ("sin", ParseError::FunctionRequiresParentheses { function: MathFunction::Sin, span: Span::new(0, 3) }),
            ("(1+1", ParseError::UnbalancedBracket { span: Span::new(0, 1) }),
            ("1+2)", ParseError::UnbalancedBracket { span: Span::new(3, 4) }),
            ("(1;2)", ParseError::BracketUnclosedAtSemicolon { span: Span::new(2, 3) }),
        ];
        for (source, expected) in cases {
            assert_eq!(check(source), Err(expected), "for input {source}");
        }
    }

    /// The load-bearing behaviour of the crate. Every one of these evaluates
    /// today and must still reach the shunting yard.
    #[test]
    fn test_what_worked_before_still_validates() {
        for source in [
            "1+2*3/(4-5)", "-2^-2", "--5", "5--3", "x=y=5", "x=2; y=3; x*y",
            "max(1,2)", "sin[5]", "sin(-1)", "5!", "2.0!", ";1+1", "1+2;",
            "1/cos(x^2)", "pi + e", "9801/(2206*sqrt(2))",
        ] {
            assert!(check(source).is_ok(), "{source} was refused");
        }
    }

    /// The unary rewrite is unchanged apart from its address, and the token it
    /// produces inherits the position of the '-' it replaces.
    #[test]
    fn test_the_unary_minus_is_rewritten_in_place() {
        let tokens = check("-5").unwrap();
        assert_eq!(tokens[0].node, Token::Operator(Operator::Une));
        assert_eq!(tokens[0].span, Span::new(0, 1));
    }

    /// A leading '+' has never meant anything and is still dropped.
    #[test]
    fn test_a_unary_plus_disappears() {
        let tokens = check("+5").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].node, Token::Operand(Number::NaturalNumber(BigInt::from(5u8))));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib validate`
Expected: compilation failure — `validate` does not exist.

- [ ] **Step 3: Write `src/validate.rs`**

```rust
//! The pass between tokenising and the shunting yard.
//!
//! It walks the token stream once, holding one bit of state — whether a value
//! or an operator is expected next — and one stack of bracket frames. Both
//! already existed: the bit is `mod_unary_operators`' `expect_operand_next`,
//! which used it to tell a binary `-` from a unary one but had no way to
//! refuse, and the frames are the shunting yard's, which counted arguments
//! while it was busy reordering. Bringing them together costs no extra
//! traversal and gives every rejection a position.

use crate::error::ParseError;
use crate::span::{Span, Spanned};
use crate::token::{Bracket, MathFunction, Number, Operator, Token};

/// One entry per open bracket.
struct Frame {
    /// The call this bracket opens, with the position of the function name —
    /// which is where an arity error points, since that is what the user has
    /// to change.
    function: Option<(MathFunction, Span)>,
    /// Where the bracket itself is, for the unbalanced-bracket message.
    open: Span,
    /// Separators seen so far inside this bracket.
    commas: usize,
    /// Whether the current argument slot — since the bracket opened, or since
    /// the last `,` — has seen anything.
    has_content: bool,
}

#[derive(PartialEq, Eq)]
enum Expect {
    Value,
    Operator,
}

/// Checks that the token sequence is a well-formed expression, and rewrites the
/// unary operators while it is there.
///
/// # Errors
/// Every [`ParseError`] variant except `UnexpectedCharacter`, `EmptyExpression`
/// and `Malformed`, which belong to the tokeniser and the shunting yard.
pub(crate) fn validate<'a>(
    tokens: &[Spanned<Token<'a>>],
    source: &str,
) -> Result<Vec<Spanned<Token<'a>>>, ParseError> {
    let mut out: Vec<Spanned<Token<'a>>> = Vec::with_capacity(tokens.len());
    let mut expect = Expect::Value;
    let mut frames: Vec<Frame> = Vec::new();
    let mut pending_function: Option<(MathFunction, Span)> = None;
    // Whether the current ';'-separated segment has anything in it. A leading
    // or trailing ';' leaves an empty segment, which has always been legal; an
    // incomplete one — "1+;" — has not.
    let mut segment_has_content = false;

    for t in tokens {
        // A function name must be followed by an opening bracket. Checked here
        // against whatever came next, and once more after the loop for a name
        // with nothing after it at all.
        if let Some((fun, span)) = pending_function {
            if !matches!(t.node, Token::Bracket(Bracket::Open)) {
                return Err(ParseError::FunctionRequiresParentheses { function: fun, span });
            }
        }

        match &t.node {
            Token::Operand(_) | Token::Variable(_) => {
                require_value_position(&expect, t, source)?;
                expect = Expect::Operator;
                mark_content(&mut frames, &mut segment_has_content);
                out.push(t.clone());
            }

            Token::Function(fun) => {
                require_value_position(&expect, t, source)?;
                pending_function = Some((*fun, t.span));
                mark_content(&mut frames, &mut segment_has_content);
                out.push(t.clone());
            }

            Token::Bracket(Bracket::Open) => {
                require_value_position(&expect, t, source)?;
                frames.push(Frame {
                    function: pending_function.take(),
                    open: t.span,
                    commas: 0,
                    has_content: false,
                });
                out.push(t.clone());
            }

            Token::Bracket(Bracket::Close) => {
                let frame = frames
                    .pop()
                    .ok_or(ParseError::UnbalancedBracket { span: t.span })?;

                if frame.has_content {
                    // '(1+)': the group holds something but ends mid-expression.
                    if expect == Expect::Value {
                        return Err(ParseError::ExpectedValue {
                            found: text_at(source, t.span),
                            span: t.span,
                        });
                    }
                } else if frame.commas > 0 {
                    // 'max(1,)': the final slot is empty.
                    return Err(ParseError::EmptyArgument { span: t.span });
                } else if let Some((fun, fspan)) = frame.function {
                    // 'max()': keeps the arity wording it has today.
                    return Err(ParseError::WrongArity {
                        function: fun,
                        expected: fun.arity(),
                        given: 0,
                        span: fspan,
                    });
                } else {
                    // '()': brackets around nothing.
                    return Err(ParseError::EmptyGroup { span: t.span });
                }

                if let Some((fun, fspan)) = frame.function {
                    let given = frame.commas + 1;
                    let expected = fun.arity();
                    if given != usize::from(expected) {
                        return Err(ParseError::WrongArity {
                            function: fun,
                            expected,
                            given,
                            span: fspan,
                        });
                    }
                }

                // Any bracket reaching this point enclosed something, so the
                // enclosing slot has content. Stage 1 had to defer this
                // decision — an empty group contributed nothing and had to be
                // told apart from one that did — and that bookkeeping is gone,
                // because an empty group is now an error in its own right.
                expect = Expect::Operator;
                mark_content(&mut frames, &mut segment_has_content);
                out.push(t.clone());
            }

            Token::Operator(op) => match (&expect, op) {
                // A '+' in value position has never meant anything.
                (Expect::Value, Operator::Add) => {
                    mark_content(&mut frames, &mut segment_has_content);
                }
                // A '-' in value position is the unary operator, and inherits
                // the position of the '-' it replaces.
                (Expect::Value, Operator::Sub) => {
                    mark_content(&mut frames, &mut segment_has_content);
                    out.push(Spanned::new(Token::Operator(Operator::Une), t.span));
                }
                (Expect::Value, _) => {
                    return Err(ParseError::ExpectedValue {
                        found: text_at(source, t.span),
                        span: t.span,
                    })
                }
                // '!' is postfix: it consumes the value on its left and leaves
                // one in its place, so the state does not move.
                (Expect::Operator, Operator::Fac) => out.push(t.clone()),
                (Expect::Operator, _) => {
                    expect = Expect::Value;
                    out.push(t.clone());
                }
            },

            Token::Comma => {
                let Some(frame) = frames.last_mut() else {
                    return Err(ParseError::CommaOutsideCall { span: t.span });
                };
                if frame.function.is_none() {
                    return Err(ParseError::CommaInPlainBracket { span: t.span });
                }
                if !frame.has_content {
                    return Err(ParseError::EmptyArgument { span: t.span });
                }
                if expect == Expect::Value {
                    return Err(ParseError::ExpectedValue {
                        found: text_at(source, t.span),
                        span: t.span,
                    });
                }
                frame.commas += 1;
                frame.has_content = false;
                expect = Expect::Value;
                out.push(t.clone());
            }

            Token::SemiColon => {
                if !frames.is_empty() {
                    return Err(ParseError::BracketUnclosedAtSemicolon { span: t.span });
                }
                if expect == Expect::Value && segment_has_content {
                    return Err(ParseError::ExpectedValue {
                        found: text_at(source, t.span),
                        span: t.span,
                    });
                }
                expect = Expect::Value;
                segment_has_content = false;
                out.push(t.clone());
            }
        }
    }

    if let Some((fun, span)) = pending_function {
        return Err(ParseError::FunctionRequiresParentheses { function: fun, span });
    }
    if let Some(frame) = frames.last() {
        return Err(ParseError::UnbalancedBracket { span: frame.open });
    }
    if expect == Expect::Value && segment_has_content {
        // Nothing to underline, but somewhere to point: a zero-width span at
        // the end of the source, which `render` widens to a single caret.
        let end = source.len();
        return Err(ParseError::ExpectedValue {
            found: "end of expression".to_string(),
            span: Span::new(end, end),
        });
    }

    Ok(out)
}

/// Refuses a value where an operator was required.
fn require_value_position(
    expect: &Expect,
    t: &Spanned<Token<'_>>,
    source: &str,
) -> Result<(), ParseError> {
    if *expect == Expect::Operator {
        return Err(ParseError::ExpectedOperator {
            found: text_at(source, t.span),
            span: t.span,
        });
    }
    Ok(())
}

/// Records that the innermost argument slot, and the current segment, are no
/// longer empty.
fn mark_content(frames: &mut [Frame], segment_has_content: &mut bool) {
    if let Some(frame) = frames.last_mut() {
        frame.has_content = true;
    }
    *segment_has_content = true;
}

/// The text the user actually typed for this token. The fallback cannot be
/// reached — every span here came from this same source — and exists so that
/// building an error message can never panic.
fn text_at(source: &str, span: Span) -> String {
    source.get(span.start..span.end).unwrap_or("?").to_string()
}
```

The test module needs its own imports, which the implementation does not use:

```rust
use crate::parser::Parser;
use crate::token::{MathFunction, Number, Operator, Token};
use num_bigint::BigInt;
```

- [ ] **Step 4: Strip `parser.rs` back to tokenising**

Delete `mod_unary_operators` and `test_multiple_unary_ops2` from `src/parser.rs`
(the behaviour they covered is now covered by
`test_the_unary_minus_is_rewritten_in_place` and
`test_a_unary_plus_disappears`), and change `parse`'s last line from
`Ok(Self::mod_unary_operators(&vex))` to `Ok(vex)`.

- [ ] **Step 5: Call `validate` from the pipeline**

In `src/rpn_resolver.rs`, `parse_with_borrowed_heap`:

```rust
match Parser::parse(exp)
    .and_then(|tokens| crate::validate::validate(&tokens, exp))
    .map_err(anyhow::Error::from)
    .and_then(|tokenised_expr| {
        RpnResolver::reverse_polish_notation(&tokenised_expr, heap_for_parse)
    })
```

- [ ] **Step 6: Run the whole suite**

Run: `cargo test`
Expected: the new tests pass. Some existing tests in `tests/integration_tests.rs`
that assert `is_err()` still pass — the error simply arrives earlier and reads
better. **`!5` is the one behaviour change**: if a test asserts it returns 120,
update it to assert the parse error and note the change in the commit message.

- [ ] **Step 7: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/validate.rs src/parser.rs src/rpn_resolver.rs src/lib.rs
git commit -m "Tell the five malformed expressions apart"
```

---

### Task 4: The shunting yard sheds its guard duty

Component C's second half, and the structural debt entry on `reverse_polish_notation`. What leaves is not split off — it was already moved in Task 3, and this task deletes the copies that validation has made unreachable.

**Files:**
- Create: `src/shunting.rs` — moved out of `src/rpn_resolver.rs:330-570`
- Modify: `src/rpn_resolver.rs` — call `shunting::to_rpn`, keep the evaluation loop
- Modify: `src/lib.rs` — `mod shunting;`

**Interfaces:**
- Consumes: `validate` (Task 3), `Spanned`, `ParseError`.
- Produces: `pub(crate) fn to_rpn<'a>(tokens: &[Spanned<Token<'a>>]) -> Result<VecDeque<Spanned<Token<'a>>>, ParseError>`

**Deleted from it, all now enforced upstream:** `struct BracketFrame` and
`argument_count`, `pending_function`, the comma counting, the arity check,
`EMPTY_ARGUMENT_ERR`, `COMMA_OUTSIDE_CALL_ERR`, `UNBALANCED_BRACKET_ERR`,
`BRACKET_UNCLOSED_AT_SEMICOLON_ERR`, `function_requires_parentheses_err`.

**Also deleted: the variable heap parameter.** `reverse_polish_notation` takes
the `Rc` and pre-registers every variable it sees with a value of zero
(`rpn_resolver.rs:565-570`). The write is vestigial — `resolve` reads variables
with `unwrap_or_else(|| zero)` — so removing it changes no result, and it is
what stops compilation from being a pure function of the text.

**Kept: the two `if !found_open` branches**, retyped as `ParseError::Malformed`.
Read the comment above them before deciding they are dead code. They are
unreachable, and deleting them would not turn a broken invariant into an error —
it would turn it into `2*(3+4)` evaluated as `2 3 * 4 +`, which is 10 instead
of 14.

- [ ] **Step 1: Write the failing test**

In `src/shunting.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::validate::validate;

    fn rpn(source: &str) -> Vec<Token<'_>> {
        let tokens = Parser::parse(source).expect("tokenises");
        let validated = validate(&tokens, source).expect("validates");
        to_rpn(&validated)
            .expect("reorders")
            .into_iter()
            .map(|t| t.node)
            .collect()
    }

    #[test]
    fn test_infix_becomes_postfix() {
        assert_eq!(
            rpn("1+2"),
            vec![
                Token::Operand(Number::NaturalNumber(BigInt::from(1u8))),
                Token::Operand(Number::NaturalNumber(BigInt::from(2u8))),
                Token::Operator(Operator::Add),
            ]
        );
    }

    /// Grouping is the whole point of the algorithm, and the property the two
    /// unreachable branches exist to protect: mis-grouping produces a number,
    /// not an error.
    #[test]
    fn test_brackets_survive_the_reordering() {
        let out = rpn("2*(3+4)");
        assert_eq!(out.last(), Some(&Token::Operator(Operator::Mul)));
    }

    /// The spans travel through to the output, in the order the operators are
    /// applied, so an evaluation error can name the operator that produced it.
    #[test]
    fn test_the_output_keeps_the_positions() {
        let tokens = Parser::parse("1+2").expect("tokenises");
        let validated = validate(&tokens, "1+2").expect("validates");
        let out = to_rpn(&validated).expect("reorders");
        assert_eq!(out[2].span, Span::new(1, 2));
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib shunting`
Expected: compilation failure — `to_rpn` does not exist.

- [ ] **Step 3: Move the function and cut what validation now owns**

Create `src/shunting.rs` with the module docs, `to_rpn` (the body of
`reverse_polish_notation` minus everything in the deleted list above), and the
two `DisplayThatVec` / `DisplayThisDeque` helpers it uses for `debug!` logging.
The signature loses the heap and gains spans:

```rust
pub(crate) fn to_rpn<'a>(
    tokens: &[Spanned<Token<'a>>],
) -> Result<VecDeque<Spanned<Token<'a>>>, ParseError>
```

Inside, `match *t` becomes `match t.node`, every `postfix_stack.push_back(...)`
and `operators_stack.push(...)` moves the `Spanned` value rather than the bare
token, and the three `anyhow!(MALFORMED_ERR)` sites become
`ParseError::Malformed`.

Move `test_reverse_polish_notation` out of `rpn_resolver.rs`'s test module and
into this one, adapted to the helper above. `rpn_resolver.rs` is deleted in Task
6; a test left behind in it is a test deleted without anyone deciding to.

- [ ] **Step 4: Point `rpn_resolver.rs` at it**

`parse_with_borrowed_heap` becomes:

```rust
match Parser::parse(exp)
    .and_then(|tokens| validate::validate(&tokens, exp))
    .and_then(|validated| shunting::to_rpn(&validated))
{
    Ok(rpn_expr) => RpnResolver { rpn_expr, local_heap: borrowed_heap, build_error: None, limits },
    Err(err) => RpnResolver {
        rpn_expr: VecDeque::new(),
        local_heap: borrowed_heap,
        build_error: Some(err.to_string()),
        limits,
    },
}
```

`build_error` stays a `String` for one more task; it disappears in Task 6.
`resolve`'s loop iterates `&self.rpn_expr` and matches on `&t.node`; the spans
are unused until Task 5.

- [ ] **Step 5: Run the whole suite and record the line count**

Run: `cargo test`
Expected: all green.

Then record what the debt entry asked for, whatever it turns out to be:

```bash
awk '/pub\(crate\) fn to_rpn/,/^}/' src/shunting.rs | wc -l
```

Put the number in the commit message. If it is still over clippy's 100-line
threshold, say so plainly rather than quietly moving on — Stage 1's lesson was
that an unattainable target quoted in advance is worse than a measured one
reported after.

- [ ] **Step 6: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/shunting.rs src/rpn_resolver.rs src/lib.rs
git commit -m "Leave the shunting yard with one job"
```

---

### Task 5: Evaluation errors

Component A applied to the evaluation half. Internal only: `resolve` still returns `anyhow::Result` at its boundary, so no test in `tests/` changes yet. The public shape moves in Task 6.

**Files:**
- Modify: `src/limits.rs` — `check_bits`, `check_size`, `check_predicted_size`
- Modify: `src/functions.rs` — `eval`, `number_to_f64`, `decimal_from_f64`
- Modify: `src/rpn_resolver.rs` — the whole evaluation loop and the error statics
- Modify: `src/error.rs` — add `EvalError::at`

**Interfaces:**
- Consumes: `EvalError`, `Span` (Task 1); `Spanned` RPN output (Task 4).
- Produces:
  - `pub(crate) fn EvalError::at(self, span: Span) -> EvalError`
  - `pub(crate) fn limits::check_size(value: &Number, limits: Limits) -> Result<(), EvalError>`
  - `pub(crate) fn limits::check_predicted_size(predicted_bits: u128, limits: Limits) -> Result<(), EvalError>`
  - `pub(crate) fn functions::eval(fun: MathFunction, value: Number, result_stack: &mut VecDeque<Number>, var_stack: &mut VecDeque<Option<String>>) -> Result<Number, EvalError>`
  - `pub(crate) fn functions::number_to_f64(value: &Number, on_error: EvalError) -> Result<f64, EvalError>`
  - `pub(crate) fn functions::decimal_from_f64(value: f64, on_error: EvalError) -> Result<Number, EvalError>`

**How a span reaches an error raised deep in the call stack.** It does not.
`limits.rs` and `functions.rs` know nothing about positions and build their
errors with `span: None`; the evaluation loop, which is holding the `Spanned`
token, stamps the position on the way out with `at`. That keeps spans out of two
modules entirely instead of threading them through every helper.

- [ ] **Step 1: Write the failing tests**

In `src/error.rs`'s test module:

```rust
/// Every variant that can carry a position must actually receive one from
/// `at`, or an error raised inside `limits.rs` or `functions.rs` arrives
/// positionless and the caret silently disappears for that condition. The
/// compiler enforces the same thing — `at` matches without a catch-all arm —
/// but only this test says what the behaviour is supposed to be.
#[test]
fn test_at_stamps_every_variant_that_has_room_for_a_position() {
    let span = Span::new(4, 5);
    let carriers = vec![
        EvalError::DivisionByZero { span: None },
        EvalError::ValueTooLarge { bits: 1, limit: 1, span: None },
        EvalError::ComputationTooLarge { predicted_bits: 1, limit: 1, span: None },
        EvalError::FactorialNotNatural { span: None },
        EvalError::FactorialOperandTooLarge { span: None },
        EvalError::ExponentTooLarge { span: None },
        EvalError::PowerOperandsTooLarge { span: None },
        EvalError::InvalidPower { span: None },
        EvalError::OperandTooLargeForFloat { span: None },
        EvalError::NotARealNumber { span: None },
        EvalError::ReadOnlyConstant { name: "pi".to_string(), span: None },
        EvalError::AssignmentTargetMissing { span: None },
        EvalError::Malformed { span: None },
    ];
    for error in carriers {
        let stamped = error.clone().at(span);
        assert_eq!(stamped.span(), Some(span), "{error:?} was not stamped");
    }
}

/// A position already set is the more precise one: an outer frame must not
/// overwrite it with its own.
#[test]
fn test_at_does_not_overwrite_a_position_already_set() {
    let inner = Span::new(1, 2);
    let error = EvalError::DivisionByZero { span: Some(inner) };
    assert_eq!(error.at(Span::new(9, 10)).span(), Some(inner));
}
```

In `src/limits.rs`'s test module, replace `test_check_rejects_above_the_budget_and_accepts_at_it` with:

```rust
#[test]
fn test_the_two_checks_report_two_different_conditions() {
    let limits = Limits { max_value_bits: 64 };
    assert!(check_predicted_size(64, limits).is_ok());
    assert!(matches!(
        check_predicted_size(65, limits),
        Err(EvalError::ComputationTooLarge { predicted_bits: 65, limit: 64, .. })
    ));

    let big = Number::NaturalNumber(BigInt::from(1u8) << 65);
    assert!(matches!(
        check_size(&big, limits),
        Err(EvalError::ValueTooLarge { limit: 64, .. })
    ));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib error && cargo test --lib limits`
Expected: compilation failure — `at` does not exist, `check_size` returns `anyhow::Result`.

- [ ] **Step 3: Write `EvalError::at`**

In `src/error.rs`. The or-pattern binds one `&mut Option<Span>` across every
variant that has one, so adding a variant with a span and forgetting it here is
a compile error, not a missing caret:

```rust
impl EvalError {
    /// Fills in where this error happened, unless it already knows.
    ///
    /// Errors raised inside `limits.rs` and `functions.rs` carry no position —
    /// those modules never see a token. The evaluation loop stamps them as they
    /// pass, which is why neither module has to thread a span through every
    /// helper it owns.
    pub(crate) fn at(mut self, span: Span) -> EvalError {
        match &mut self {
            EvalError::DivisionByZero { span: slot }
            | EvalError::ValueTooLarge { span: slot, .. }
            | EvalError::ComputationTooLarge { span: slot, .. }
            | EvalError::FactorialNotNatural { span: slot }
            | EvalError::FactorialOperandTooLarge { span: slot }
            | EvalError::ExponentTooLarge { span: slot }
            | EvalError::PowerOperandsTooLarge { span: slot }
            | EvalError::InvalidPower { span: slot }
            | EvalError::OperandTooLargeForFloat { span: slot }
            | EvalError::NotARealNumber { span: slot }
            | EvalError::ReadOnlyConstant { span: slot, .. }
            | EvalError::AssignmentTargetMissing { span: slot }
            | EvalError::Malformed { span: slot } => {
                if slot.is_none() {
                    *slot = Some(span);
                }
            }
            EvalError::NotFinite { .. } => {}
        }
        self
    }
}
```

- [ ] **Step 4: Convert `limits.rs`**

`check_bits` and its `phrase` parameter disappear: the distinction it carried in
two words of prose is now the difference between two variants.

```rust
pub(crate) fn check_size(value: &Number, limits: Limits) -> Result<(), EvalError> {
    let bits = u128::from(size_in_bits(value));
    if bits > u128::from(limits.max_value_bits) {
        return Err(EvalError::ValueTooLarge { bits, limit: limits.max_value_bits, span: None });
    }
    Ok(())
}

pub(crate) fn check_predicted_size(predicted_bits: u128, limits: Limits) -> Result<(), EvalError> {
    if predicted_bits > u128::from(limits.max_value_bits) {
        return Err(EvalError::ComputationTooLarge {
            predicted_bits,
            limit: limits.max_value_bits,
            span: None,
        });
    }
    Ok(())
}
```

Keep both doc comments as they are — they explain predict-versus-measure, which
has not changed.

- [ ] **Step 5: Convert `functions.rs`**

`number_to_f64` and `decimal_from_f64` take the error to raise instead of a
message string; the call sites already chose the wording, and now choose the
variant:

```rust
pub(crate) fn number_to_f64(value: &Number, on_error: EvalError) -> Result<f64, EvalError> {
    match value {
        Number::NaturalNumber(v) => v.to_f64().ok_or(on_error),
        Number::DecimalNumber(v) => v.to_f64().ok_or(on_error),
    }
}
```

Inside `eval`, `FLOAT_EVAL_TOO_LARGE_ERR` becomes
`EvalError::OperandTooLargeForFloat { span: None }` and
`INVALID_FUNCTION_RESULT_ERR` becomes `EvalError::NotARealNumber { span: None }`.
The two `MALFORMED_ERR` sites in the `Max` and `Min` arms and the
`MathFunction::None` arm all become `EvalError::Malformed { span: None }`: after
Task 3, arity is guaranteed before evaluation begins, so a missing second operand
is a broken invariant rather than a user mistake.

- [ ] **Step 6: Convert the evaluation loop**

In `src/rpn_resolver.rs`, delete every error static except none — they all go —
and give the loop body a stamping closure. Inside `for t in &self.rpn_expr`,
before the `match`:

```rust
let at = |e: EvalError| e.at(t.span);
```

Then each site becomes, for example:

```rust
Token::Operand(n) => {
    limits::check_size(n, limits).map_err(at)?;
    ...
}
Operator::Div => {
    if right_value == zero {
        return Err(EvalError::DivisionByZero { span: Some(t.span) });
    }
    ...
}
```

`power`, `power_integer` and the factorial arm take the same treatment:
`INVALID_POWER_ERR` → `InvalidPower`, `POWER_TOO_LARGE_ERR` →
`PowerOperandsTooLarge`, `EXPONENT_TOO_LARGE_ERR` → `ExponentTooLarge`,
`FACTORIAL_NATURAL_ERR` → `FactorialNotNatural`, the factorial's
`to_u64` failure → `FactorialOperandTooLarge`, `BUILTIN_CONSTANT_ERR` →
`ReadOnlyConstant { name }`, `NO_VARIABLE_ERR` → `AssignmentTargetMissing`, and
every `MALFORMED_ERR` → `Malformed`.

`resolve` keeps its `anyhow::Result` signature for now:

```rust
pub fn resolve(&mut self) -> anyhow::Result<Number> {
    self.eval_inner().map_err(anyhow::Error::from)
}
```

where `eval_inner` is the converted loop returning `Result<Number, EvalError>`.
This wrapper exists for exactly one task.

- [ ] **Step 7: Run the whole suite**

Run: `cargo test`
Expected: all green, including every `err.contains(...)` assertion in
`tests/integration_tests.rs` — the wording of the size messages was chosen in
Task 1 to keep "occupies" and "would need". If one of those fails, the message
drifted; fix the message, not the test. They are converted to variants in Task 6.

- [ ] **Step 8: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/error.rs src/limits.rs src/functions.rs src/rpn_resolver.rs
git commit -m "Give the evaluator errors it can be asked about"
```

---

### Task 6: `Expression`, `compile` and `eval`

Component D. The public break lands here, and `anyhow` leaves the crate.

**Files:**
- Create: `src/expression.rs` — from the evaluation half of `src/rpn_resolver.rs`
- Delete: `src/rpn_resolver.rs`
- Modify: `src/session.rs` — `lookup`, `assign`, delete `process`
- Modify: `src/lib.rs` — module list, and the doc examples so they compile
- Modify: `tests/integration_tests.rs` — the whole file's call sites, and the error assertions
- Modify: `Cargo.toml` — remove `anyhow`

**Interfaces:**
- Consumes: `to_rpn` (Task 4), `EvalError` (Task 5), `Limits` (existing).
- Produces:
  - `pub struct Expression<'a>` — borrows the source text, exactly as `RpnResolver<'a>` did
  - `pub fn Expression::compile(source: &'a str) -> Result<Expression<'a>, ParseError>`
  - `pub fn Expression::eval(&self, session: &Session) -> Result<Number, EvalError>`
  - `pub fn Expression::eval_with(&self, session: &Session, limits: Limits) -> Result<Number, EvalError>`
  - `pub(crate) fn Session::lookup(&self, name: &str) -> Option<Number>`
  - `pub(crate) fn Session::assign(&self, name: &str, value: Number) -> Result<(), EvalError>`
  - `pub fn Session::limits(&self) -> Limits`

**`eval` takes `&Session`, not `&mut Session`.** Assignment is an expression:
`x=5` writes into the heap through the `RefCell` the session already holds. That
is unchanged from 0.2.0 and is why this stage does not need `&mut`.

- [ ] **Step 1: Write the failing tests**

In `tests/integration_tests.rs`, at the top, plus one representative conversion
of an existing assertion:

```rust
#[test]
fn test_a_parse_failure_is_reported_by_compile_not_by_eval() {
    assert!(matches!(
        Expression::compile("1+"),
        Err(ParseError::ExpectedValue { .. })
    ));
}

#[test]
fn test_a_compiled_expression_survives_a_change_of_variable() {
    let session = Session::init();
    session.set("x", 2);
    let expr = Expression::compile("x*3").expect("compiles");
    assert_eq!(expr.eval(&session).unwrap(), Number::NaturalNumber(BigInt::from(6)));
    session.set("x", 5);
    assert_eq!(expr.eval(&session).unwrap(), Number::NaturalNumber(BigInt::from(15)));
}

#[test]
fn test_the_size_limit_reports_which_check_refused_the_value() {
    let session = Session::with_limits(Limits { max_value_bits: 128 });
    let expr = Expression::compile("2^10000").expect("compiles");
    assert!(matches!(
        expr.eval(&session),
        Err(EvalError::ComputationTooLarge { .. })
    ));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test integration_tests`
Expected: compilation failure — `Expression` does not exist.

- [ ] **Step 3: Write `src/expression.rs`**

```rust
/// A compiled expression: the token sequence in postfix order, ready to be
/// evaluated as often as wanted, against any [`Session`].
///
/// The lifetime is the source text's. Compilation is a pure function of that
/// text — it consults no session and touches no variable heap — so one
/// `Expression` can be evaluated against several sessions, and under several
/// budgets.
pub struct Expression<'a> {
    rpn: VecDeque<Spanned<Token<'a>>>,
}

impl<'a> Expression<'a> {
    /// Compiles `source` into an expression.
    ///
    /// # Errors
    /// Any [`ParseError`]: the text does not tokenise, the token sequence is
    /// not a well-formed expression, or the brackets do not balance.
    pub fn compile(source: &'a str) -> Result<Expression<'a>, ParseError> {
        let tokens = Parser::parse(source)?;
        let validated = validate::validate(&tokens, source)?;
        Ok(Expression { rpn: shunting::to_rpn(&validated)? })
    }

    /// Evaluates against `session`, under the session's own limits.
    ///
    /// # Errors
    /// Any [`EvalError`]: a division by zero, a value over the size budget, an
    /// assignment to a built-in constant, and so on.
    pub fn eval(&self, session: &Session) -> Result<Number, EvalError> {
        self.eval_with(session, session.limits())
    }

    /// Evaluates against `session`, under `limits` instead of the session's.
    ///
    /// Use this to run untrusted input under a tighter budget than trusted
    /// input, against the same variables. Mind the floor the built-in constants
    /// impose: `pi`, `e`, `tau`, `phi` and `gamma` are `f64`s held exactly as
    /// rationals and cost up to 107 bits, so a budget below that refuses a
    /// value the caller never supplied.
    ///
    /// # Errors
    /// As [`Expression::eval`].
    pub fn eval_with(&self, session: &Session, limits: Limits) -> Result<Number, EvalError> {
        // the loop moved from RpnResolver::resolve, unchanged apart from:
        //   - variables read through session.lookup(..)
        //   - assignment written through session.assign(..)?
        //   - `for t in &self.rpn`, matching on &t.node
    }
}
```

The loop itself is `RpnResolver::resolve`'s, moved. Every change to it, in full:

1. `for t in &self.rpn_expr` becomes `for t in &self.rpn`, and every `match`
   inside it reads `&t.node`.
2. The `Token::Variable` arm reads `session.lookup(&var_name)` instead of
   borrowing `self.local_heap`, keeping the `unwrap_or_else(|| zero)` that makes
   an undefined variable read as `0`.
3. The `Operator::Eql` arm calls `session.assign(&var, right_value.clone())`,
   stamping the span with `at` on failure. Its own `is_constant_name` check goes:
   `assign` is now the one place that refusal is decided.
4. `self.limits` becomes the `limits` parameter.
5. The `build_error` guard at the top disappears entirely.

Move `power`, `power_integer`, `pow_big_int`, `pow_big_rational` and
`factorial_helper` across unchanged apart from their error types, and move the
rest of `rpn_resolver.rs`'s test module — `test_factorial`, `test_max_min` and
whatever else survives there — with them. `build_error` does not move: a
compilation failure is now returned by `compile`.

- [ ] **Step 4: Give `Session` the two accessors and take away `process`**

```rust
impl Session {
    /// The value of `name`, or [`None`] if it has never been set.
    pub(crate) fn lookup(&self, name: &str) -> Option<Number> {
        self.variable_heap.borrow().get(name).cloned()
    }

    /// Writes `value` into the heap, refusing the built-in constants.
    ///
    /// This is the one place that refusal is decided. `set`, `setf` and the
    /// evaluator's assignment operator all come through here.
    ///
    /// # Errors
    /// [`EvalError::ReadOnlyConstant`] when `name` is a built-in constant.
    pub(crate) fn assign(&self, name: &str, value: Number) -> Result<(), EvalError> {
        let name = name.to_lowercase();
        if Session::is_constant_name(&name) {
            return Err(EvalError::ReadOnlyConstant { name, span: None });
        }
        self.variable_heap.borrow_mut().insert(name, value);
        Ok(())
    }

    /// The limits every [`Expression::eval`] against this session uses.
    #[must_use]
    pub fn limits(&self) -> Limits {
        self.limits
    }
}
```

`set` and `setf` delegate to `assign` and, for this one task, discard its
result — `let _ = self.assign(..);` with a comment pointing at Task 8, which
flips their signatures. Their current duplicated constant check goes now; only
the swallowing survives, and only until Task 8.

Delete `Session::process`.

- [ ] **Step 5: Delete `rpn_resolver.rs` and remove `anyhow`**

Remove `pub mod rpn_resolver;` from `src/lib.rs`, delete the file, and remove the
`anyhow` line from `Cargo.toml`. Then:

```bash
grep -rn "anyhow" src/ Cargo.toml
```

Expected: no output.

- [ ] **Step 6: Convert `tests/integration_tests.rs`**

Every `session.process(&exp)` becomes `Expression::compile(&exp).unwrap()`, and
every `resolver.resolve()` becomes `expr.eval(&session)`. Then the 15 assertions
of the form:

```rust
let err = resolver.resolve().unwrap_err().to_string();
assert!(err.contains("occupies"), "message was: {err}");
```

become:

```rust
assert!(matches!(expr.eval(&session), Err(EvalError::ValueTooLarge { .. })));
```

with `"would need"` becoming `EvalError::ComputationTooLarge`, `"exponent is too
large"` becoming `EvalError::ExponentTooLarge`, and so on. Where an assertion
checked that a message did *not* mention something — `!err.contains("Invalid
power operation")` at `integration_tests.rs:582` — the variant assertion already
carries that, because a variant is not another variant.

Also update the doc examples in `src/lib.rs` and `src/session.rs` enough to
compile; they are rewritten properly in Task 11.

- [ ] **Step 7: Run the whole suite**

Run: `cargo test`
Expected: all green. Every test that asserts a numeric *value* must be untouched.
If you found yourself changing one, stop and find out what broke.

- [ ] **Step 8: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add -A
git commit -m "Report a parse failure when the expression is parsed"
```

---

### Task 7: The two public panics

Component E. Two panics, two different cures.

**Files:**
- Modify: `src/token.rs:282-308` — `operator_priority` and `compare_operator_priority`
- Modify: `src/token.rs:432-451` — `impl Div for Number` becomes `Number::checked_div`
- Modify: `src/shunting.rs` — the one caller of `compare_operator_priority`
- Modify: `src/expression.rs` — the `Div` arm and the two reciprocal sites in `power_integer`

**Interfaces:**
- Produces:
  - `pub fn Number::checked_div(&self, rhs: &Number) -> Option<Number>`
  - `pub(crate) fn Token::compare_operator_priority(op1: Operator, op2: Operator) -> bool`

- [ ] **Step 1: Write the failing tests**

In `src/token.rs`'s test module:

```rust
/// The reason this exists: `a / b` panicked here, inside a public std::ops
/// impl, on an input any caller can supply.
#[test]
fn test_dividing_by_zero_answers_none_instead_of_panicking() {
    let one = Number::NaturalNumber(BigInt::from(1));
    let zero = Number::NaturalNumber(BigInt::from(0));
    assert_eq!(one.checked_div(&zero), None);
    assert_eq!(Number::decimal(BigRational::new(BigInt::from(1), BigInt::from(2))).checked_div(&zero), None);
}

#[test]
fn test_checked_div_still_divides() {
    let six = Number::NaturalNumber(BigInt::from(6));
    let three = Number::NaturalNumber(BigInt::from(3));
    assert_eq!(six.checked_div(&three), Some(Number::NaturalNumber(BigInt::from(2))));
}
```

The existing `test_operator_priority` tests change shape: they pass
`Operator::Add` where they passed `Token::Operator(Operator::Add)`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib token`
Expected: compilation failure — `checked_div` does not exist.

- [ ] **Step 3: Narrow `operator_priority`'s parameter**

```rust
/// The precedence and associativity of an operator.
///
/// This takes an [`Operator`] rather than a [`Token`] because that is its
/// domain. The wider parameter is what forced the `_ => panic!` arm it used to
/// carry: a function that accepts brackets and commas has to say something when
/// it gets one. With the narrow type the match is exhaustive and there is
/// nothing left to refuse.
fn operator_priority(o: Operator) -> (u8, Associate) {
    match o {
        Operator::Add | Operator::Sub => (1, Associate::LeftAssociative),
        Operator::Mul | Operator::Div => (2, Associate::LeftAssociative),
        Operator::Pow => (3, Associate::RightAssociative),
        Operator::Une => (4, Associate::RightAssociative),
        Operator::Fac => (5, Associate::LeftAssociative),
        Operator::Eql => (0, Associate::RightAssociative),
    }
}
```

`compare_operator_priority` takes two `Operator`s and becomes `pub(crate)`. In
`src/shunting.rs`, the caller already matches `Token::Operator(_)` to get there;
bind the operator in that pattern and pass it.

- [ ] **Step 4: Replace `impl Div` with `checked_div`**

```rust
impl Number {
    /// Divides, or answers [`None`] when `rhs` is zero.
    ///
    /// There is no `impl Div for Number`: division is partial, and a
    /// `std::ops` impl has nowhere to say so. [`Add`], [`Sub`] and [`Mul`] are
    /// total and stay.
    #[must_use]
    pub fn checked_div(&self, rhs: &Number) -> Option<Number> {
        if rhs == &Number::NaturalNumber(BigInt::zero()) {
            return None;
        }
        Some(match (self.clone(), rhs.clone()) {
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
        })
    }
}
```

The zero test compares by value, so it catches a `DecimalNumber(0/1)` as well as
a `NaturalNumber(0)` — `Number`'s `PartialEq` has compared by value since
Stage 1.

- [ ] **Step 5: Route the three zero-divisor sites through it**

In `src/expression.rs`, the `Operator::Div` arm loses its separate `if
right_value == zero` test:

```rust
Operator::Div => {
    let value = left_value
        .checked_div(&right_value)
        .ok_or(EvalError::DivisionByZero { span: Some(t.span) })?;
    limits::check_size(&value, limits).map_err(at)?;
    ...
}
```

and both negative-exponent branches in `power_integer` — which build a
reciprocal and guard the zero themselves — become one `checked_div` of
`Number::NaturalNumber(BigInt::one())` by the computed power, with the same
`ok_or`. Three hand-written zero tests become one, inside the only function that
knows how to divide.

- [ ] **Step 6: Run the whole suite**

Run: `cargo test`
Expected: all green, including the existing `1/0` and `0^-1` tests, which now
travel a different route to the same error.

- [ ] **Step 7: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/token.rs src/shunting.rs src/expression.rs
git commit -m "Close both public panics, each the way it deserves"
```

---

### Task 8: Limits that can grow, and setters that report

Components F and G.

**Files:**
- Modify: `src/limits.rs` — `#[non_exhaustive]`, `with_max_value_bits`
- Modify: `src/session.rs` — `set`, `setf`
- Modify: `src/token.rs` — `impl TryFrom<f64> for Number`, `ConversionError::NotFinite`
- Modify: `tests/integration_tests.rs` — every `Limits { max_value_bits: n }` literal

**Interfaces:**
- Produces:
  - `pub fn Limits::with_max_value_bits(self, bits: u64) -> Limits`
  - `pub fn Session::set(&self, key: &str, value: i64) -> Result<(), EvalError>`
  - `pub fn Session::setf(&self, key: &str, value: f64) -> Result<(), EvalError>`
  - `impl TryFrom<f64> for Number`, `Error = ConversionError`

**`#[non_exhaustive]` closes struct-literal construction outside the crate**, so
every `Limits { max_value_bits: n }` in `tests/` stops compiling. That is the
point — it is the same door that would have to stay open forever for a second
knob to be a non-breaking addition. Find them with `grep -n "Limits {" tests/`.

**`ConversionError` loses its `Eq` derive.** The new variant holds an `f64`,
which is `PartialEq` and not `Eq`. Keep `PartialEq`; drop `Eq` from the derive
list on that enum only.

- [ ] **Step 1: Write the failing tests**

In `tests/integration_tests.rs`:

```rust
#[test]
fn test_setting_a_built_in_constant_is_refused_out_loud() {
    let session = Session::init();
    assert!(matches!(
        session.set("pi", 3),
        Err(EvalError::ReadOnlyConstant { .. })
    ));
    // And the refusal is real: pi is still pi.
    let expr = Expression::compile("pi").expect("compiles");
    assert!(matches!(expr.eval(&session).unwrap(), Number::DecimalNumber(_)));
}

#[test]
fn test_setting_a_variable_to_a_non_number_is_refused_out_loud() {
    let session = Session::init();
    assert!(matches!(session.setf("x", f64::NAN), Err(EvalError::NotFinite { .. })));
    assert!(matches!(session.setf("x", f64::INFINITY), Err(EvalError::NotFinite { .. })));
}

#[test]
fn test_one_expression_can_be_evaluated_under_two_budgets() {
    let session = Session::init();
    let expr = Expression::compile("2^64").expect("compiles");
    assert!(expr.eval(&session).is_ok());
    assert!(matches!(
        expr.eval_with(&session, Limits::default().with_max_value_bits(8)),
        Err(EvalError::ComputationTooLarge { .. })
    ));
    // The session's own budget is untouched by the tighter evaluation.
    assert!(expr.eval(&session).is_ok());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test integration_tests`
Expected: compilation failure — `set` returns `()`, `with_max_value_bits` does not exist.

- [ ] **Step 3: Implement**

```rust
// src/limits.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits { /* unchanged */ }

impl Limits {
    /// The same limits with a different size budget.
    ///
    /// ```
    /// # use yarer::Limits;
    /// let tight = Limits::default().with_max_value_bits(4096);
    /// ```
    #[must_use]
    pub fn with_max_value_bits(mut self, bits: u64) -> Limits {
        self.max_value_bits = bits;
        self
    }
}
```

```rust
// src/token.rs
/// Builds a [`Number`] from an [`f64`], refusing NaN and the infinities.
///
/// The value decides the variant, as everywhere else: an integral `f64`
/// becomes a [`Number::NaturalNumber`].
impl TryFrom<f64> for Number {
    type Error = ConversionError;

    fn try_from(value: f64) -> Result<Number, ConversionError> {
        BigRational::from_float(value)
            .map(Number::decimal)
            .ok_or(ConversionError::NotFinite { value })
    }
}
```

```rust
// src/session.rs
/// Declares or overwrites an integer variable.
///
/// # Errors
/// [`EvalError::ReadOnlyConstant`] when `key` names a built-in constant.
pub fn set(&self, key: &str, value: i64) -> Result<(), EvalError> {
    self.assign(key, Number::NaturalNumber(BigInt::from(value)))
}

/// Declares or overwrites a variable from an [`f64`].
///
/// # Errors
/// [`EvalError::ReadOnlyConstant`] when `key` names a built-in constant, and
/// [`EvalError::NotFinite`] for NaN or an infinity — which used to be accepted
/// silently and stored nothing.
pub fn setf(&self, key: &str, value: f64) -> Result<(), EvalError> {
    let number = Number::try_from(value).map_err(|_| EvalError::NotFinite { value })?;
    self.assign(key, number)
}
```

- [ ] **Step 4: Convert the `Limits` literals in the tests**

Every `Session::with_limits(Limits { max_value_bits: n })` becomes
`Session::with_limits(Limits::default().with_max_value_bits(n))`. Inside the
crate the literal still compiles; leave `src/` alone unless clippy complains.

- [ ] **Step 5: Run the whole suite**

Run: `cargo test`
Expected: all green. Existing `session.set(..)` calls in tests now produce an
unused-`Result` warning — add `.expect("not a constant")` to each, which also
documents that they are not expected to fail.

- [ ] **Step 6: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add -A
git commit -m "Stop swallowing what the caller asked for"
```

---

### Task 9: `Number::decimal` — measure, then decide

Component H. The only task whose outcome is not decided in advance.

**Files:**
- Modify: `src/token.rs` — `Number::decimal`
- Modify: `tests/integration_tests.rs` — the measurement harness

- [ ] **Step 1: Write the harness**

```rust
/// Not an assertion: a measurement, for the decision recorded in component H
/// of the spec. Run before and after adding `.reduced()` to `Number::decimal`.
///
/// Run with: cargo test --release -- --ignored --nocapture
#[test]
#[ignore = "timing"]
fn measure_the_cost_of_reducing_every_decimal() {
    let session = Session::init();
    for (name, source, rounds) in [
        ("small rationals", "1/3 + 1/7 + 1/11", 20_000),
        ("one large rational", "(2^60000)/3", 200),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        let start = std::time::Instant::now();
        for _ in 0..rounds {
            expr.eval(&session).expect("evaluates");
        }
        println!("{name}: {rounds} evaluations in {:?}", start.elapsed());
    }
}
```

- [ ] **Step 2: Record the baseline**

Run: `cargo test --release -- --ignored --nocapture`
Write both numbers down. They are the "before".

- [ ] **Step 3: Write the failing test for the defect itself**

```rust
#[test]
fn test_an_unreduced_rational_does_not_become_a_decimal() {
    // Ratio::new_raw skips reduction, so 4/2 arrives integral but unreduced.
    // Number::decimal is the constructor that upholds the invariant, and it
    // has to reduce to see that this value is a whole number.
    let unreduced = BigRational::new_raw(BigInt::from(4), BigInt::from(2));
    assert!(matches!(Number::decimal(unreduced), Number::NaturalNumber(_)));
}
```

- [ ] **Step 4: Add `.reduced()` and measure again**

```rust
pub fn decimal(value: BigRational) -> Number {
    let value = value.reduced();
    if value.denom().is_one() {
        Number::NaturalNumber(value.to_integer())
    } else {
        Number::DecimalNumber(value)
    }
}
```

Run the harness again.

- [ ] **Step 5: Decide by the recorded criterion**

- **Both cases within 5% of the baseline:** keep this. One constructor, one
  invariant, done. Record both measurements in the commit message.
- **Either case worse than 5%:** split. Public `decimal` keeps `.reduced()`;
  add `pub(crate) fn decimal_unchecked(value: BigRational) -> Number` without
  it, and use it from `checked_div`, `apply_functional_token_operation`,
  `power_integer` and `decimal_from_f64` — every path where the rational comes
  out of `BigRational`'s own arithmetic, which reduces its results. Record both
  measurements and the ratio in the commit message.

Do not decide by preference. The criterion was fixed before the numbers existed
precisely so that it could not be.

- [ ] **Step 6: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/token.rs tests/integration_tests.rs
git commit -m "Make the canonical constructor actually canonical"
```

---

### Task 10: The narrowed surface

Component D's last piece.

**Files:**
- Modify: `src/lib.rs` — module visibility and re-exports
- Modify: `src/parser.rs`, `src/token.rs` — item visibility

- [ ] **Step 1: Narrow, then re-export**

In `src/lib.rs`:

```rust
mod error;
mod expression;
mod functions;
mod parser;
mod session;
mod shunting;
mod span;
mod token;
mod validate;
pub mod limits;

pub use error::{Error, EvalError, ParseError};
pub use expression::Expression;
pub use limits::Limits;
pub use session::Session;
pub use span::Span;
pub use token::{ConversionError, MathFunction, Number};
```

`Parser`, `Token`, `Operator`, `Bracket` and `Associate` become `pub(crate)`.
`Number`, `MathFunction` and `ConversionError` stay public: the first is the
result type, the second appears inside `ParseError::WrongArity`, the third
inside `TryFrom`.

- [ ] **Step 2: Compile and fix what falls out**

Run: `cargo test`
Expected: errors in `tests/integration_tests.rs` where it imports
`yarer::token::Number` and friends. Rewrite those imports as
`use yarer::{Expression, EvalError, Limits, Number, ParseError, Session};`.

Rust will also point out any public function that leaks a now-private type in
its signature. There should be none; if there is, that signature was public by
accident and this is the moment it shows.

- [ ] **Step 3: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add -A
git commit -m "Publish the interface and keep the mechanism"
```

---

### Task 11: The REPL, the README and the crate docs

The renderer's first consumer, and the documentation that still describes 0.2.0.

**Files:**
- Modify: `src/bin/main.rs` — the read-eval-print loop
- Modify: `src/lib.rs` — the crate-level documentation
- Modify: `src/session.rs`, `src/limits.rs` — doc examples
- Modify: `README.md`

- [ ] **Step 1: Rewrite the REPL's evaluation arm**

```rust
let outcome = Expression::compile(&line)
    .map_err(Error::from)
    .and_then(|expr| expr.eval(&session).map_err(Error::from));

match outcome {
    Ok(value) => println!("{value}"),
    Err(err) => println!("{}", err.render(&line)),
}
```

This is the whole reason `Error` exists as a union: two calls, two error types,
one rendering path.

- [ ] **Step 2: Check it by eye**

Run:

```bash
printf 'max(1,*2)\n1/(2-2)\n!5\n2×3\nsin(0.5)\nquit\n' | cargo run --quiet -- -q
```

Expected: a caret under the `*`, a caret under the `/`, a caret under the `!`,
`6` for `2×3` (an evaluation, not an error — it is there to prove the multi-byte
operator does not disturb the loop), and a number for `sin(0.5)`.

- [ ] **Step 3: Rewrite the crate documentation**

Every doctest in `src/lib.rs` uses `session.process(..)`. Rewrite them around
`Expression::compile` and `expr.eval(&session)`, and add one that shows an error
being handled — the point of the whole stage:

```rust
//! ```
//! use yarer::{Expression, ParseError, Session};
//!
//! let session = Session::init();
//! match Expression::compile("max(1,*2)") {
//!     Err(ParseError::ExpectedValue { span, .. }) => {
//!         assert_eq!((span.start, span.end), (6, 7));
//!     }
//!     other => panic!("expected a parse error, got {other:?}"),
//! }
//! ```
```

- [ ] **Step 4: Rewrite the README**

Update the API examples the same way, add the declared-changes table from the
spec as a migration note, and fix the fenced blocks that hold REPL transcripts
and the built-in function list: they are tagged ```` ```rust ```` and are not
Rust. Tag them ```` ```text ````.

- [ ] **Step 5: Run everything, including the doctests**

Run: `cargo test`
Expected: all green, doctests included. `cargo test --doc` alone is the fast way
to iterate on step 3.

- [ ] **Step 6: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add -A
git commit -m "Show the caret where the user can see it"
```

---

### Task 12: Close the register's untested behaviours

The last section of `docs/tech-debt.md` that this stage can close, and the update to the register itself. Tests only, plus prose.

**Files:**
- Modify: `tests/integration_tests.rs`
- Modify: `src/expression.rs` — split `test_max_min`
- Modify: `docs/tech-debt.md`

- [ ] **Step 1: Write the tests**

```rust
/// Canonicalisation made `2.0!` return 2 where it used to error, and nothing
/// pinned it.
#[test]
fn test_factorial_accepts_an_integral_decimal_literal() {
    let session = Session::init();
    let expr = Expression::compile("2.0!").expect("compiles");
    assert_eq!(expr.eval(&session).unwrap(), Number::NaturalNumber(BigInt::from(2)));
}

/// `1/0` is tested; `1/0.0` — the form that actually used to panic, before
/// canonicalisation turned the literal into a NaturalNumber — was not.
#[test]
fn test_dividing_by_a_decimal_zero_is_an_error_not_a_panic() {
    let session = Session::init();
    let expr = Expression::compile("1/0.0").expect("compiles");
    assert!(matches!(expr.eval(&session), Err(EvalError::DivisionByZero { .. })));
}

/// Only the even-exponent degenerate case was pinned, and it cannot tell a
/// correct sign from a discarded one.
#[test]
fn test_a_negative_base_keeps_its_sign_for_an_odd_exponent() {
    let session = Session::init();
    for (source, expected) in [("(-1)^3", -1), ("(-1)^4", 1), ("(-2)^3", -8)] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// The early returns: exponent zero, and the two factorial base cases.
#[test]
fn test_the_degenerate_cases_return_one() {
    let session = Session::init();
    for source in ["5^0", "0!", "1!", "0^0"] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(1)),
            "for {source}"
        );
    }
}

/// The budget is documented as a limit, not a threshold: a value that occupies
/// exactly the budget is admitted. 15 needs 4 bits, 16 needs 5.
#[test]
fn test_a_value_landing_exactly_on_the_budget_is_admitted() {
    let session = Session::with_limits(Limits::default().with_max_value_bits(4));
    assert!(Expression::compile("15").unwrap().eval(&session).is_ok());
    assert!(matches!(
        Expression::compile("16").unwrap().eval(&session),
        Err(EvalError::ValueTooLarge { .. })
    ));
}
```

And add `"0.5+0.5"`, `"1.5/0.5"` and `"(0.5)^-1"` to the loop in
`test_integral_results_are_natural_numbers` (`tests/integration_tests.rs:487`).
Those three are the only inputs that reach `checked_div`'s Decimal/Decimal arm,
`apply_functional_token_operation`'s decimal arms, and `power_integer`'s decimal
arms; without them the canonicalisation invariant is unchecked on all three.

- [ ] **Step 2: Split `test_max_min`**

It moved into `src/expression.rs` with the rest of the evaluation tests in Task 6.
It is one loop over three expressions, so a failure on the first hides the other
two. Give each expression its own `assert_eq!` with its own message.

- [ ] **Step 3: Run the whole suite**

Run: `cargo test`
Expected: all green. If one of these fails, it has found a real defect — none of
them is a new requirement, they all pin behaviour the crate is supposed to have
already.

- [ ] **Step 4: Update `docs/tech-debt.md`**

Remove the entries this stage closed: both public panics, the `Session` limits
accessor, `Limits` not being `#[non_exhaustive]`, `setf` swallowing NaN,
`Number::decimal` not reducing, the whole "Untested behaviours" section,
`COMMA_OUTSIDE_CALL_ERR`'s wording, the `sin[5]` contradiction, `max(1,*2)`
passing the arity check, the bare `()` message, `EXPONENT_TOO_LARGE_ERR`'s
capitalisation, `resolve_decimal!`'s misnomer if it went with the loop, and
`test_max_min`.

Keep, and re-verify against the new code: `parse_decimal_literal`'s quadratic
denominator, `apply_functional_token_operation`'s needless clone,
`factorial_helper` being the naive product, and the function-length entries with
whatever numbers Tasks 4 and 6 actually produced.

Add anything this stage introduced. Be specific about what is new debt rather
than inherited: if `validate` grew past 100 lines, say so and say why it was
worth it.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add -A
git commit -m "Pin what canonicalisation quietly changed"
```

---

## Definition of done for the stage

Run all of these before opening the pull request:

```bash
cargo clean -p yarer
cargo test
cargo fmt --check
cargo clippy --all-targets --message-format=json 2>/dev/null \
  | grep -oP '"code":"clippy::[a-z_]+"' | sed 's/.*clippy:://; s/"//' | sort | uniq -c | sort -rn
grep -rn "anyhow" src/ Cargo.toml
grep -c "contains(" tests/integration_tests.rs
```

- `cargo test` green, `cargo fmt --check` silent.
- The clippy table compared per lint against the Task 1 baseline. A category that
  got worse is reported in the pull request even if the total went down.
- `anyhow` returns nothing.
- No `err.contains(...)` assertion remains for an error this stage typed.
- The `.reduced()` measurement and the resulting decision recorded.
- The line counts of `to_rpn` and the evaluation loop recorded as numbers.
- Every row of the spec's changes table reflected in the code and ready for the
  0.3.0 CHANGELOG.
- `docs/tech-debt.md` updated.
- `Cargo.toml` still says `version = "0.2.0"`.
