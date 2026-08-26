# Comparison and Logical Operators Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add ten operators to yarer — `<` `>` `<=` `>=` `==` `<>`, `and` `or` `xor` `not`, and `mod` — without changing the meaning of any expression that evaluates today.

**Architecture:** No new types and no new error variants. `Operator` gains ten variants, the precedence ladder is renumbered from six levels to ten with the existing six keeping their relative order, and the tokeniser learns four two-character symbols and five words. Comparisons yield `1` or `0` as in GNU bc; the logical operators read any non-zero value as true; `mod` truncates toward zero and routes its zero check through the crate's single `checked_div`.

**Tech Stack:** Rust 2021. `num-bigint`/`num-rational` for exact arithmetic, `num-traits` for numeric predicates, `thiserror` for the error types, `rustyline`+`clap` for the binary. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-08-26-logical-operators-design.md`. Read it before starting; this plan argues from it and does not repeat its reasoning.

## Global Constraints

- Branch: `logical-operators`, cut from `master` after Stage 2 has merged. **This work depends on Stage 2**: `Operator` is `pub(crate)` only since Stage 2's Task 10, which is what makes adding ten variants a non-breaking change.
- **Never run `git stash` in this repository.** Two protected user stashes from June 2025 live here and must not be disturbed. If you need a clean tree, use `git worktree` and remove it when done.
- After any edit, confirm the build actually recompiled: if `cargo test` output has no `Compiling yarer` line after a source change, run `cargo clean -p yarer` and try again.
- Every task ends with `cargo test` green and `cargo fmt --check` clean.
- **The 143 value-asserting macro invocations in `tests/integration_tests.rs` must stay byte-identical.** They are the test of this plan's central claim — that renumbering the precedence ladder changes no existing meaning. If one needs editing, stop: the ladder is wrong, and that is a finding, not a test to adjust. Check with:
  ```bash
  grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' tests/integration_tests.rs | sort | md5sum
  ```
  Record the digest before you start and compare after every task.
- Clippy is compared **per lint**, on a cold cache, never by counting warnings:
  ```bash
  cargo clean -p yarer >/dev/null 2>&1
  cargo clippy --all-targets --message-format=json 2>/dev/null \
    | grep -oP '"code":"clippy::[a-z_]+"' | sed 's/.*clippy:://; s/"//' | sort | uniq -c | sort -rn
  ```
- Do not add dependencies. Do not reintroduce `anyhow`.
- Error message text is lower case and carries no category prefix — `Error::render` adds it, once.
- `Cargo.toml` stays at `version = "0.2.0"`. The bump belongs to the release.
- Commit messages: imperative subject line, no tool attribution of any kind, no `Co-Authored-By` trailer, no mention of any AI tool.

## The compiler is most of the checklist

Three matches in this crate are exhaustive with no catch-all arm, by deliberate choice made during Stage 2. Adding a variant to `Operator` therefore **fails to compile** until each is updated:

- `Token::operator_priority` (`src/token.rs`) — its catch-all was deleted in Stage 2 precisely so a new operator cannot silently inherit a precedence.
- `impl Display for Operator` (`src/token.rs`).
- the `match op` inside the evaluation loop (`src/expression.rs`).

One match is **not** exhaustive, and that is where the only real trap lives: `src/validate.rs`'s `match (&expect, op)` ends in `(Expect::Value, _)` and `(Expect::Operator, _)`. A new binary operator is handled correctly by those catch-alls — which is why the nine binary additions need no validator change at all. `not` is not binary, and the catch-all would silently accept `1 not 2` as a binary operation and let it fail later with a worse message. That is exactly the defect `max(1,*2)` had before Stage 2, and Task 4 exists to not reintroduce it.

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `src/token.rs` | the `Operator` vocabulary, `is_unary`, the precedence ladder, `Display`, the two-character tokens | 1, 2 |
| `src/parser.rs` | the expression regex | 2 |
| `src/validate.rs` | `not`'s two state-machine arms | 4 |
| `src/expression.rs` | the ten evaluation arms, the `is_unary` extraction | 2, 3, 4, 5 |
| `tests/integration_tests.rs` | precedence boundaries, truthiness, `mod`'s signs, reserved words | 2-6 |
| `README.md`, `src/lib.rs`, `docs/tech-debt.md` | documentation and the declared break | 7 |

---

### Task 1: The operator vocabulary

Component B of the spec, plus the renaming that makes it readable. Nothing can produce these operators yet — the tokeniser cannot spell them — so this task lands green with every new variant unreachable.

**Files:**
- Modify: `src/token.rs` — the `Operator` enum, `operator_priority`, `impl Display for Operator`, `from_operator`
- Modify: `src/expression.rs` — the `match op` arm list, to keep compiling

**Interfaces:**
- Produces:
  - `Operator::{Less, Greater, LessEq, GreaterEq, Equal, NotEqual, And, Or, Xor, Not, Mod}`
  - `Operator::Eql` renamed to `Operator::Assign`
  - `pub(crate) const fn Operator::is_unary(self) -> bool`

**The rename is not cosmetic.** `Eql` currently means *assignment*. Introducing `Equal` meaning *equality* beside a variant called `Eql` meaning *assignment* is a trap laid for every future reader of this file, and the moment to avoid it is before the second one exists. `Operator` is `pub(crate)`, so this is not a breaking change; the compiler will find every site.

- [ ] **Step 1: Record the baselines**

```bash
grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' tests/integration_tests.rs | sort | md5sum
cargo clean -p yarer >/dev/null 2>&1
cargo clippy --all-targets --message-format=json 2>/dev/null \
  | grep -oP '"code":"clippy::[a-z_]+"' | sed 's/.*clippy:://; s/"//' | sort | uniq -c | sort -rn
```

Put both in the commit message body.

- [ ] **Step 2: Write the failing tests**

In `src/token.rs`'s test module:

```rust
/// The whole safety argument for renumbering the ladder. Every operator that
/// existed before this work keeps its position relative to the others, so no
/// expression that evaluates today can change meaning. If this ever goes red,
/// some existing expression has quietly been re-parsed.
#[test]
fn test_the_existing_operators_keep_their_relative_order() {
    let ascending = [
        Operator::Assign,
        Operator::Add,
        Operator::Mul,
        Operator::Pow,
        Operator::Une,
        Operator::Fac,
    ];
    let levels: Vec<u8> = ascending
        .iter()
        .map(|o| Token::operator_priority(*o).0)
        .collect();
    assert!(
        levels.windows(2).all(|pair| pair[0] < pair[1]),
        "levels were {levels:?}, which is not strictly ascending"
    );
}

/// `mod` shares a level with `*` and `/` rather than getting one of its own,
/// so `7 mod 3 * 2` groups left to right.
#[test]
fn test_mod_sits_with_multiplication() {
    assert_eq!(
        Token::operator_priority(Operator::Mod),
        Token::operator_priority(Operator::Mul)
    );
}

/// The three prefix and postfix operators take no left operand. The evaluation
/// loop asks this question twice, and before this function existed it spelled
/// the answer out both times.
#[test]
fn test_only_the_three_unary_operators_report_as_unary() {
    for unary in [Operator::Une, Operator::Fac, Operator::Not] {
        assert!(unary.is_unary(), "{unary} should be unary");
    }
    for binary in [
        Operator::Add, Operator::Sub, Operator::Mul, Operator::Div, Operator::Pow,
        Operator::Assign, Operator::Less, Operator::Greater, Operator::LessEq,
        Operator::GreaterEq, Operator::Equal, Operator::NotEqual,
        Operator::And, Operator::Or, Operator::Xor, Operator::Mod,
    ] {
        assert!(!binary.is_unary(), "{binary} should be binary");
    }
}

/// Every operator renders as the text a user would type, which is what the
/// `found '{}'` half of a parse error shows.
#[test]
fn test_the_new_operators_render_as_they_are_written() {
    let pairs = [
        (Operator::Less, "<"), (Operator::Greater, ">"),
        (Operator::LessEq, "<="), (Operator::GreaterEq, ">="),
        (Operator::Equal, "=="), (Operator::NotEqual, "<>"),
        (Operator::And, "and"), (Operator::Or, "or"),
        (Operator::Xor, "xor"), (Operator::Not, "not"),
        (Operator::Mod, "mod"), (Operator::Assign, "="),
    ];
    for (op, text) in pairs {
        assert_eq!(op.to_string(), text);
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --lib token`
Expected: compilation failure — the variants do not exist.

- [ ] **Step 4: Extend the enum and rename `Eql`**

```rust
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Operator {
    /// Binary Add ('1+1')
    Add,
    /// Binary Sub ('2-1')
    Sub,
    /// Binary Mul ('2*2')
    Mul,
    /// Binary Div ('3/3')
    Div,
    /// Binary Pow ('base^exponent')
    Pow,
    /// Unary Neg ('-1')
    Une,
    /// Factorial ('0!')
    Fac,
    /// Binary Assignment ('a=1'). Named for what it does: `Equal` below is the
    /// comparison, and the two must not be confusable at a glance.
    Assign,
    /// Less than ('1<2')
    Less,
    /// Greater than ('2>1')
    Greater,
    /// Less than or equal ('1<=1')
    LessEq,
    /// Greater than or equal ('1>=1')
    GreaterEq,
    /// Equality ('1==1'). Not assignment; see [`Operator::Assign`].
    Equal,
    /// Inequality ('1<>2'). Spelled `<>` rather than `!=` because `!` is the
    /// postfix factorial, which would make `5!=3` ambiguous.
    NotEqual,
    /// Logical and ('1 and 0')
    And,
    /// Logical or ('1 or 0')
    Or,
    /// Logical exclusive or ('1 xor 0')
    Xor,
    /// Logical negation ('not 0'). Prefix, and spelled as a word because `!` is taken.
    Not,
    /// Remainder, truncating toward zero ('7 mod 3')
    Mod,
}
```

Rename every `Operator::Eql` to `Operator::Assign`. The compiler lists them; there is no judgement involved.

- [ ] **Step 5: Rewrite the precedence ladder**

```rust
/// The precedence and associativity of an operator.
///
/// Ten levels, weakest first. The six operators that predate the comparison and
/// logical set keep their order relative to one another — assignment below
/// addition below multiplication below power below unary minus below factorial —
/// so renumbering them cannot change how any existing expression groups. The new
/// levels all sit below addition, except `Mod`, which joins an existing level
/// rather than creating one.
///
/// This takes an [`Operator`] rather than a [`Token`] because that is its domain.
/// The wider parameter is what once forced a `_ => panic!` arm here.
fn operator_priority(o: Operator) -> (u8, Associate) {
    match o {
        Operator::Assign => (0, Associate::RightAssociative),
        Operator::Or | Operator::Xor => (1, Associate::LeftAssociative),
        Operator::And => (2, Associate::LeftAssociative),
        Operator::Not => (3, Associate::RightAssociative),
        Operator::Less
        | Operator::Greater
        | Operator::LessEq
        | Operator::GreaterEq
        | Operator::Equal
        | Operator::NotEqual => (4, Associate::LeftAssociative),
        Operator::Add | Operator::Sub => (5, Associate::LeftAssociative),
        Operator::Mul | Operator::Div | Operator::Mod => (6, Associate::LeftAssociative),
        Operator::Pow => (7, Associate::RightAssociative),
        Operator::Une => (8, Associate::RightAssociative),
        Operator::Fac => (9, Associate::LeftAssociative),
    }
}
```

- [ ] **Step 6: Add `is_unary` and extend `Display`**

```rust
impl Operator {
    /// Whether this operator takes no left operand.
    ///
    /// The evaluation loop asks twice — once for the value and once for the
    /// variable name beside it — and spelled the answer out both times before
    /// this existed.
    pub(crate) const fn is_unary(self) -> bool {
        matches!(self, Operator::Une | Operator::Fac | Operator::Not)
    }
}
```

`Display` gains the twelve renderings the Step 2 test pins. `Une` keeps rendering as `#`, as it does today.

- [ ] **Step 7: Make `src/expression.rs` compile again**

The `match op` in the evaluation loop is exhaustive. Add the eleven new variants to it as a single temporary arm that cannot be reached, because nothing tokenises them yet:

```rust
Operator::Less | Operator::Greater | Operator::LessEq | Operator::GreaterEq
| Operator::Equal | Operator::NotEqual | Operator::And | Operator::Or
| Operator::Xor | Operator::Not | Operator::Mod => {
    // Unreachable until the tokeniser learns these, two tasks from now.
    // Tasks 2 through 5 replace this arm with the real ones; it exists so
    // this task compiles on its own.
    return Err(EvalError::Malformed { span: Some(t.span) });
}
```

- [ ] **Step 8: Run everything**

Run: `cargo test`
Expected: green, with the four new tests passing and every existing test untouched. Re-run the value-assertion digest from Step 1 and confirm it is unchanged.

- [ ] **Step 9: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/token.rs src/expression.rs
git commit -m "Give the operators their names and their order"
```

---

### Task 2: The comparison operators

Component A's symbols and the first six evaluation arms. After this task `1 < 2` evaluates. The nine binary additions need **no** validator change: `src/validate.rs`'s `(Expect::Operator, _)` arm already treats any operator as binary, which is correct for all of them.

**Files:**
- Modify: `src/parser.rs` — `EXPRESSION_REGEX`
- Modify: `src/token.rs` — `tokenize`, `from_operator`, and a new `from_two_char_operator`
- Modify: `src/expression.rs` — replace six of the eleven variants in Task 1's temporary arm with real ones, and add a `boolean` helper

**Interfaces:**
- Consumes: `Operator::{Less, Greater, LessEq, GreaterEq, Equal, NotEqual}`, `Operator::is_unary` (Task 1).
- Produces: `fn boolean(truth: bool) -> Number` in `src/expression.rs`, used by Tasks 3 and 4.

**A deviation from the spec, and why.** The spec's component D routes comparisons through `partial_cmp` and gives the unreachable `None` case `EvalError::Malformed`. Use `Number`'s comparison operators directly instead — `left_value < right_value` and friends. They are defined in terms of `partial_cmp` returning `Some`, so an unreachable `None` becomes plain `false` rather than a branch nobody can test. This removes a defensive arm rather than adding one, which is the better outcome; record it in your report.

- [ ] **Step 1: Write the failing tests**

In `tests/integration_tests.rs`:

```rust
#[test]
fn test_comparisons_yield_one_or_zero() {
    let session = Session::init();
    for (source, expected) in [
        ("1 < 2", 1), ("2 < 1", 0), ("2 < 2", 0),
        ("2 > 1", 1), ("1 > 2", 0),
        ("2 <= 2", 1), ("3 <= 2", 0),
        ("2 >= 2", 1), ("1 >= 2", 0),
        ("2 == 2", 1), ("2 == 3", 0),
        ("2 <> 3", 1), ("2 <> 2", 0),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// Comparison asks the mathematical value, not the enum tag — the property the
/// previous stage established when it made PartialEq and PartialOrd agree.
#[test]
fn test_comparison_crosses_the_number_variants() {
    let session = Session::init();
    for source in ["2 == 6/3", "0.5 < 2/3", "1.0 >= 1"] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(1)),
            "for {source}"
        );
    }
}

/// A two-character operator occupies two bytes, and its span must say so.
/// Nothing else in the suite would notice if it did not: the message stays
/// right while the caret moves one column left.
#[test]
fn test_a_two_character_operator_spans_two_bytes() {
    let err = Expression::compile("1 <= ").unwrap_err();
    assert_eq!(err.span().map(|s| (s.start, s.end)), Some((5, 5)));
    let err = Expression::compile("1 <= <= 2").unwrap_err();
    assert!(
        matches!(err, ParseError::ExpectedValue { ref found, span } if found == "<=" && (span.start, span.end) == (5, 7)),
        "got {err:?}"
    );
}

/// `=` is still assignment and nothing about it moved.
#[test]
fn test_assignment_is_untouched_by_the_comparison_operators() {
    let session = Session::init();
    let expr = Expression::compile("x = 5").expect("compiles");
    assert_eq!(expr.eval(&session).unwrap(), Number::NaturalNumber(BigInt::from(5)));
    let expr = Expression::compile("x == 5").expect("compiles");
    assert_eq!(expr.eval(&session).unwrap(), Number::NaturalNumber(BigInt::from(1)));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test integration_tests comparison`
Expected: failure — `1 < 2` does not tokenise, so `compile` returns `UnexpectedCharacter`.

- [ ] **Step 3: Extend the regex**

In `src/parser.rs`. The two-character alternatives go **first**, because regex alternation is ordered and `<=` must not be read as `<` followed by `=`:

```rust
static EXPRESSION_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(<=|>=|==|<>|\d+\.?\d*|\.\d+|[-+*/^(),=<>\[\]×÷!;]|[a-zA-Z_][a-zA-Z0-9_]*)")
        .expect("Should compile regex")
});
```

- [ ] **Step 4: Teach `tokenize` the two-character forms, and drop two `unwrap`s while there**

`tokenize` currently lists the operator characters twice — once in a `match` guard and once in `from_operator` — and calls `.unwrap()` on the result. Adding `<` and `>` would mean adding them in both places, where forgetting one produces a panic. Restructure so the list exists once:

```rust
pub(crate) fn tokenize(t: &str) -> Token<'_> {
    // Two-character operators are matched against the whole token, before the
    // single-character route below: the first character of "<=" is "<", which
    // would otherwise become a comparison followed by an assignment.
    if let Some(op) = Token::from_two_char_operator(t) {
        return Token::Operator(op);
    }

    if let Some(c) = t.chars().next() {
        if let Some(token) = Token::from_operator(c) {
            return token;
        }
        if let Some(token) = Token::from_bracket(c) {
            return token;
        }
        match c {
            ',' => return Token::Comma,
            ';' => return Token::SemiColon,
            _ => (),
        }
    }

    // …the existing number, decimal, function and variable routes, unchanged.
}

fn from_two_char_operator(t: &str) -> Option<Operator> {
    match t {
        "<=" => Some(Operator::LessEq),
        ">=" => Some(Operator::GreaterEq),
        "==" => Some(Operator::Equal),
        "<>" => Some(Operator::NotEqual),
        _ => None,
    }
}
```

and `from_operator` gains two lines:

```rust
'<' => Some(Token::Operator(Operator::Less)),
'>' => Some(Token::Operator(Operator::Greater)),
```

Calling `from_operator` on the first character of every token is safe: it returns `None` for digits, letters and `.`, which is exactly what the old guard achieved by listing characters.

- [ ] **Step 5: Write the six evaluation arms**

In `src/expression.rs`, replacing six of the eleven variants in Task 1's temporary arm. Add the helper first:

```rust
/// Truth as this crate represents it: `1` and `0`, as in GNU bc.
fn boolean(truth: bool) -> Number {
    Number::NaturalNumber(BigInt::from(u8::from(truth)))
}
```

Then, inside `match op`, six arms of the shape the file already uses for `Add`:

```rust
Operator::Less => {
    let value = boolean(left_value < right_value);
    limits::check_size(&value, limits).map_err(at)?;
    result_stack.push_back(value);
    var_stack.push_back(None);
}
```

and the same for `Greater` (`>`), `LessEq` (`<=`), `GreaterEq` (`>=`), `Equal` (`==`) and `NotEqual` (`!=` on `Number`, which is `PartialEq`'s, not a yarer operator).

The `check_size` call cannot fail — `1` and `0` occupy one bit. It is there because the rule this crate keeps is that every arm which pushes a value checks it, and the register records what happened the last time that rule was given a reasonable-looking exception.

- [ ] **Step 6: Run everything**

Run: `cargo test`
Expected: green. Re-run the value-assertion digest and confirm it is unchanged — this is the first task that could have changed how an existing expression parses.

- [ ] **Step 7: Commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/parser.rs src/token.rs src/expression.rs tests/integration_tests.rs
git commit -m "Let expressions ask which of two values is larger"
```

---

### Task 3: `and`, `or` and `xor`

The first word operators, and the truthiness rule.

**Files:**
- Modify: `src/token.rs` — the word route in `tokenize`
- Modify: `src/expression.rs` — three evaluation arms and an `is_truthy` helper
- Modify: `tests/integration_tests.rs`

**Interfaces:**
- Consumes: `boolean` (Task 2), `Operator::{And, Or, Xor}` (Task 1).
- Produces: `fn is_truthy(value: &Number) -> bool` in `src/expression.rs`, used by Task 4.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn test_the_logical_operators_read_any_non_zero_value_as_true() {
    let session = Session::init();
    for (source, expected) in [
        ("1 and 1", 1), ("1 and 0", 0), ("0 and 0", 0),
        ("1 or 0", 1), ("0 or 0", 0),
        ("1 xor 0", 1), ("1 xor 1", 0), ("0 xor 0", 0),
        // Truth is not confined to 1: a fraction, a negative and a big value
        // are all true, and only zero is false.
        ("0.5 or 0", 1), ("(0-1) and 1", 1), ("1/3 and 2", 1),
        ("0.0 or 0", 0),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// Case-insensitive, like every other word in the language.
#[test]
fn test_the_word_operators_are_case_insensitive() {
    let session = Session::init();
    for source in ["1 and 1", "1 AND 1", "1 And 1", "1 aNd 1"] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(expr.eval(&session).unwrap(), Number::NaturalNumber(BigInt::from(1)));
    }
}

/// The one declared break: these words are no longer variable names.
#[test]
fn test_the_new_words_are_reserved() {
    for source in ["and = 5", "or + 1", "xor"] {
        assert!(
            matches!(Expression::compile(source), Err(ParseError::ExpectedValue { .. })),
            "{source} should be refused as a misplaced operator"
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test integration_tests logical`
Expected: failure — `and` tokenises as a variable, so `1 and 1` is two operands in a row.

- [ ] **Step 3: Add the word route to `tokenize`**

Immediately before the function-name route, so the words are decided before `Token::get_some` and long before the fall-through to `Token::Variable`:

```rust
if let Some(op) = Token::from_word_operator(t) {
    return Token::Operator(op);
}

fn from_word_operator(t: &str) -> Option<Operator> {
    match t.to_lowercase().as_str() {
        "and" => Some(Operator::And),
        "or" => Some(Operator::Or),
        "xor" => Some(Operator::Xor),
        "not" => Some(Operator::Not),
        "mod" => Some(Operator::Mod),
        _ => None,
    }
}
```

All five words are recognised here, in this task, even though `not` and `mod` have no evaluation arm until Tasks 4 and 5. That is deliberate and it is safe: Task 1's temporary arm still covers them, so they compile, and the intermediate state refuses them with an error rather than computing something wrong. Splitting the word list across three tasks would mean editing this function three times.

- [ ] **Step 4: Write the three evaluation arms**

```rust
/// Zero is false and everything else is true — including negative and
/// fractional values, which is why this asks the value rather than the variant.
/// `Number`'s `PartialEq` compares mathematically, so a `DecimalNumber` holding
/// zero is false too, exactly as `checked_div` relies on for its divisor test.
fn is_truthy(value: &Number) -> bool {
    value != &Number::NaturalNumber(BigInt::zero())
}
```

```rust
Operator::And => {
    let value = boolean(is_truthy(&left_value) && is_truthy(&right_value));
    limits::check_size(&value, limits).map_err(at)?;
    result_stack.push_back(value);
    var_stack.push_back(None);
}
```

and the same for `Or` (`||`) and `Xor` (`!=` between the two booleans).

Both operands are already on the stack when the operator runs, so `&&` here does not short-circuit anything: the right-hand expression was evaluated before this arm was reached. That is a property of the stack machine, not of this line, and the spec declares it.

- [ ] **Step 5: Run everything, then commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/token.rs src/expression.rs tests/integration_tests.rs
git commit -m "Add the logical operators and the rule for what counts as true"
```

Re-run the value-assertion digest before committing.

---

### Task 4: `not`

The only new operator that is not binary, and the only one the validator has to learn.

**Files:**
- Modify: `src/validate.rs` — two arms in the operator match
- Modify: `src/expression.rs` — one evaluation arm, and the `is_unary` extraction
- Modify: `tests/integration_tests.rs`

**Interfaces:**
- Consumes: `Operator::Not`, `Operator::is_unary` (Task 1), `boolean` and `is_truthy` (Tasks 2 and 3).

**Why the validator must be told.** `src/validate.rs`'s operator match ends in `(Expect::Value, _)` and `(Expect::Operator, _)`. The second accepts any operator as binary — correct for the nine binary additions, wrong for `not`. Without an explicit arm, `1 not 2` is accepted by the validator and fails later in the evaluator with a message about a stack, which is the exact defect `max(1,*2)` had before the previous stage.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn test_not_negates_truth() {
    let session = Session::init();
    for (source, expected) in [
        ("not 0", 1), ("not 1", 0), ("not 5", 0), ("not 0.5", 0),
        ("not not 1", 1),
        ("not (1 < 2)", 0),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// `not` is prefix. In operator position it is not a binary operator, and the
/// diagnosis must say so where it happens rather than surfacing later as a
/// stack complaint.
#[test]
fn test_not_in_operator_position_is_diagnosed_where_it_occurs() {
    let err = Expression::compile("1 not 2").unwrap_err();
    assert!(
        matches!(err, ParseError::ExpectedOperator { ref found, span } if found == "not" && (span.start, span.end) == (2, 5)),
        "got {err:?}"
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test integration_tests not`
Expected: `not 0` fails — Task 1's temporary arm returns `Malformed` — and `1 not 2` fails with the wrong error.

- [ ] **Step 3: Add the two validator arms**

In `src/validate.rs`'s `match (&expect, op)`, **above** the two catch-all arms:

```rust
// `not` is prefix: it wants a value on its right and takes nothing on its
// left, so it belongs where a value belongs and leaves the state where it
// found it.
(Expect::Value, Operator::Not) => {
    mark_content(&mut frames, &mut segment_has_content);
    out.push(t.clone());
}
// In operator position it is not a binary operator. Without this arm the
// catch-all below would accept it as one.
(Expect::Operator, Operator::Not) => {
    return Err(ParseError::ExpectedOperator {
        found: text_at(source, t.span),
        span: t.span,
    })
}
```

- [ ] **Step 4: Extract `is_unary` in the evaluation loop**

The loop spells the unary test out twice:

```rust
let left_value = if op != &Operator::Une && op != &Operator::Fac { … } else { zero.clone() };
let left_var   = if op != &Operator::Une && op != &Operator::Fac { … } else { None };
```

Both become `if !op.is_unary()`. Adding `not` to a duplicated three-term condition is how the third copy gets written.

- [ ] **Step 5: Write the evaluation arm**

For a unary operator the loop puts the operand in `right_value`:

```rust
Operator::Not => {
    let value = boolean(!is_truthy(&right_value));
    limits::check_size(&value, limits).map_err(at)?;
    result_stack.push_back(value);
    var_stack.push_back(None);
}
```

- [ ] **Step 6: Run everything, then commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/validate.rs src/expression.rs tests/integration_tests.rs
git commit -m "Teach the validator that not takes no left operand"
```

---

### Task 5: `mod`

The last arm, and the one with real arithmetic in it.

**Files:**
- Modify: `src/expression.rs` — one evaluation arm, and the removal of Task 1's temporary arm
- Modify: `tests/integration_tests.rs`

- [ ] **Step 1: Write the failing tests**

```rust
/// Truncating toward zero, as in C, Rust, bc and BASIC: the result takes the
/// sign of the dividend. All four sign combinations, because getting one right
/// by accident is easy and getting all four right by accident is not.
#[test]
fn test_mod_truncates_toward_zero() {
    let session = Session::init();
    for (source, expected) in [
        ("7 mod 3", 1), ("-7 mod 3", -1), ("7 mod -3", 1), ("-7 mod -3", -1),
        ("6 mod 3", 0), ("2 mod 5", 2),
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// Defined on rationals, not only integers, because the formula is the same one.
#[test]
fn test_mod_works_on_rationals() {
    resolve_decimal!("7.5 mod 2", 1.5);
    resolve_decimal!("1/2 mod 1/3", 1.0 / 6.0);
}

/// Its zero check is the crate's one zero check, so it reports the same error
/// division does.
#[test]
fn test_mod_by_zero_is_the_same_error_as_dividing_by_zero() {
    let session = Session::init();
    let expr = Expression::compile("7 mod 0").expect("compiles");
    assert!(matches!(expr.eval(&session), Err(EvalError::DivisionByZero { .. })));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test integration_tests mod`
Expected: failure — Task 1's temporary arm answers `Malformed`.

- [ ] **Step 3: Write the arm and delete the temporary one**

```rust
Operator::Mod => {
    // The zero check is checked_div's, which is the only place this crate
    // decides what a division by zero is.
    let quotient = left_value
        .checked_div(&right_value)
        .ok_or(EvalError::DivisionByZero { span: Some(t.span) })?;
    // `From<Number> for BigInt` truncates toward zero rather than flooring,
    // which is exactly what makes -7 mod 3 be -1 and not 2.
    let truncated = Number::NaturalNumber(BigInt::from(quotient));
    let value = left_value - right_value * truncated;
    limits::check_size(&value, limits).map_err(at)?;
    result_stack.push_back(value);
    var_stack.push_back(None);
}
```

Task 1's temporary `Malformed` arm now covers no variants and must be deleted. If the compiler does not complain that it is unreachable, some variant is still missing an arm — find it rather than leaving the temporary in place.

- [ ] **Step 4: Run everything, then commit**

```bash
cargo fmt && cargo test && cargo fmt --check
git add src/expression.rs tests/integration_tests.rs
git commit -m "Add mod, truncating toward zero like every language it borrows from"
```

---

### Task 6: The precedence boundaries

Every operator works; this task proves they compose in the right order. These tests could not be written earlier because each one needs operators from two different levels.

**Files:**
- Modify: `tests/integration_tests.rs`

**The rule for this task:** a precedence test must give a *different* answer under the wrong grouping. A test that passes under both readings is not testing precedence, and writing one is worse than writing none, because it reports coverage that is not there.

- [ ] **Step 1: Write the tests**

```rust
/// Each row fails with the value in the comment if its boundary is wrong, so
/// each one actually separates the two readings rather than merely passing.
#[test]
fn test_the_precedence_boundaries_hold() {
    let session = Session::init();
    for (source, expected) in [
        ("1 or 0 and 0", 1),   // 0 if `or` and `and` shared a level
        ("not 5 == 1", 1),     // 0 with C's precedence for `not`
        ("2 + 3 < 6", 1),      // 3 if comparison bound tighter than `+`
        ("7 mod 3 * 2", 2),    // 1 if `mod` were weaker than `*`
        ("1 < 2 == 1", 1),     // the six comparisons share one level, left to right
        ("2 * 3 mod 4", 2),    // left to right within the level: (2*3) mod 4
        ("not 0 and 0", 0),    // `not` binds tighter than `and`: (not 0) and 0
        ("0 == 0 or 0", 1),    // comparison binds tighter than `or`
    ] {
        let expr = Expression::compile(source).expect("compiles");
        assert_eq!(
            expr.eval(&session).unwrap(),
            Number::NaturalNumber(BigInt::from(expected)),
            "for {source}"
        );
    }
}

/// Unary minus still binds tighter than everything it used to, now that four
/// levels have been inserted below it.
#[test]
fn test_the_old_operators_still_group_as_they_did() {
    resolve_decimal!("-2^-2", 0.25);
    resolve_natural!("2*3+4", 10);
    resolve_natural!("2+3*4", 14);
    resolve_natural!("-7 mod 3", -1);   // (-7) mod 3, not -(7 mod 3)
}
```

Derive each expected value by hand from the ladder in the spec before running anything. If one disagrees with what the code produces, that is a finding about the ladder, not a number to adjust.

- [ ] **Step 2: Run them, then commit**

Run: `cargo test`
Expected: green, and the value-assertion digest unchanged.

```bash
cargo fmt && cargo test && cargo fmt --check
git add tests/integration_tests.rs
git commit -m "Pin every boundary in the precedence ladder"
```

---

### Task 7: Documentation and the declared break

**Files:**
- Modify: `README.md`, `src/lib.rs`, `docs/tech-debt.md`

- [ ] **Step 1: Document the operators**

The README's function list gains an operator table: the ten new operators, their precedence relative to one another, and the fact that comparisons yield `1` or `0`. The crate documentation gains one worked example — a comparison used for what comparisons are for, not a bare `1 < 2`.

Say plainly that `and`, `or`, `xor`, `not` and `mod` are reserved words and can no longer be variable names, and that `!=` does not exist because `!` is the factorial.

- [ ] **Step 2: Record the break for the CHANGELOG**

Add the row to the README's migration table:

| Before | After |
|---|---|
| `and`, `or`, `xor`, `not`, `mod` are valid variable names | reserved words, in every casing |

- [ ] **Step 3: Update `docs/tech-debt.md`**

Two entries, both honest about being small:

- The `check_size` call on a comparison's result cannot fail, because `1` and `0` occupy one bit. It is maintained by review rather than by test. Recording it is the alternative to pretending it is covered.
- No short-circuit evaluation: `0 and (2^1000000)` evaluates its right operand and is refused by the size budget rather than returning `0`. Short-circuiting needs jumps in the compiled form, which is a change to the evaluation model rather than an operator.

- [ ] **Step 4: Run everything, then commit**

Run: `cargo test` — the doctests compile, so this step catches a broken example.

```bash
cargo fmt && cargo test && cargo fmt --check
git add README.md src/lib.rs docs/tech-debt.md
git commit -m "Document the operators and the words they take away"
```

---

## Definition of done for the work

```bash
cargo clean -p yarer
cargo test
cargo fmt --check
grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' tests/integration_tests.rs | sort | md5sum
cargo clippy --all-targets --message-format=json 2>/dev/null \
  | grep -oP '"code":"clippy::[a-z_]+"' | sed 's/.*clippy:://; s/"//' | sort | uniq -c | sort -rn
```

- `cargo test` green, `cargo fmt --check` silent.
- **The value-assertion digest matches the one recorded in Task 1.** This is the plan's central claim, and it is the one number that must not have moved.
- Clippy compared per lint against the Task 1 baseline; any category that went up is reported even if the total went down.
- Every level of the precedence ladder exercised by a test that distinguishes it from its neighbours.
- No new `panic!`, and the two `unwrap`s removed in Task 2 stay removed.
- The reserved-word break recorded for the 0.3.0 CHANGELOG.
- `Cargo.toml` still `version = "0.2.0"`.
