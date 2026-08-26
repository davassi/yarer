# Design — Comparison and Logical Operators

Date: 2026-08-26
Status: implemented on branch `logical-operators`.
Target release: 0.3.0, together with Stages 1 and 2.

Two claims below did not survive implementation, and are left standing with
this note rather than quietly edited, because a design record that revises
itself to match the outcome stops being evidence of anything.

- **Component C is incomplete.** It says the validation pass is the only one
  `not` affects. The shunting yard is the other: a *prefix* operator arrives
  where a value arrives, so nothing on its operator stack has a right operand
  yet and nothing may be popped for it — a rule `Une` never needed, being
  stronger than anything it could displace. Without it `1 - not 0` pops the
  `-` and hands the evaluator a binary minus with one operand.
- **The "one rule stays untestable" claim under Testing is false.** The size
  check on a comparison's result *is* reachable: `Limits::with_max_value_bits`
  has no lower bound, and under a zero-bit budget `0 == 0` has two operands
  costing nothing and an answer costing one bit. It is reachable for the six
  comparisons and for `not`, and shadowed by the operand check for `and`,
  `or`, `xor` and `mod`. The register carries the corrected version.

## Context

Yarer's `Operator` enum has eight variants — `Add`, `Sub`, `Mul`, `Div`, `Pow`,
`Une`, `Fac`, `Eql` — and nothing that compares two values or combines two truths.
An expression language that can compute `sin(x)/2` but cannot ask whether it is
larger than one is missing a category, not a convenience.

This adds ten operators in one category:

- **comparison:** `<` `>` `<=` `>=` `==` `<>`
- **logical:** `and` `or` `xor` `not`
- **arithmetic:** `mod`

It ships in 0.3.0 rather than later for one practical reason: 0.3.0 is already a
breaking release because of Stage 2's API change, and the only break this work
introduces — five new reserved words — is far cheaper to deliver alongside that
one than in a second breaking release of its own. Someone upgrading reads one
migration note instead of two.

**This work depends on Stage 2 having landed**, and not only in sequence.
`Operator` became `pub(crate)` in Stage 2's Task 10, so adding ten variants to it
is not a breaking change. Done before Stage 2, it would have been one.

## What this is not

- **No boolean type.** A comparison yields `Number::NaturalNumber(1)` or
  `NaturalNumber(0)`, as in GNU bc, which the README already names as yarer's
  model. `Number` does not grow a variant and `eval`'s signature does not change.
  The price is that `(1<2) + 5` is a legal expression worth 6. That is the
  accepted cost of not introducing a second kind of value into a crate whose
  entire surface is built around one.
- **No bitwise operators.** `and`/`or`/`xor`/`not` are logical. Yarer's values are
  arbitrary-precision rationals, and `1/3 and 2` is well defined under a
  truthiness rule while `1/3 & 2` is not defined at all. On the 0 and 1 that
  comparisons produce, the two readings coincide anyway; only `not` genuinely
  diverges, and there the logical reading (`not 0` is 1) is the one that pairs
  with the rest.
- **No `!=`.** `!` is postfix factorial in yarer, so `5!=3` can be read as
  `(5!) = 3` or as `5 != 3`. `<>` is unambiguous and was chosen for that reason,
  not for nostalgia. For the same reason `not` is a word: the symbol is taken.
- **No short-circuit evaluation.** See component D.
- **No `elif`, no conditional expression, no statements.** These operators produce
  values. What a caller does with a 1 or a 0 is the caller's business.

## Components

### A. Tokenising

Two changes to `src/parser.rs` and one to `src/token.rs`.

The expression regex gains `<` and `>` in its single-character class, and — the
part that must not be got wrong — the four two-character forms as alternatives
**ahead of** that class, because regex alternation is ordered:

```rust
r"(<=|>=|==|<>|\d+\.?\d*|\.\d+|[-+*/^(),=<>\[\]×÷!;]|[a-zA-Z_][a-zA-Z0-9_]*)"
```

Written the other way round, `<=` tokenises as `<` followed by `=`: a comparison
followed by an assignment, which then fails somewhere else with a message about
the wrong thing.

The five words join `Token::tokenize` on the same route as the function names —
matched case-insensitively, before the fall-through to `Token::Variable`. Yarer's
functions and variables are already case-insensitive, so `and`, `And` and `AND`
all work, exactly as `sin`, `Sin` and `SIN` do. Lower case is the canonical
spelling for documentation and error messages.

**Consequence, declared:** `and`, `or`, `xor`, `not` and `mod` stop being usable
variable names, in every casing. This is the only break this work introduces.

### B. The precedence ladder

From six levels to ten, weakest to strongest:

| level | operators | associativity | note |
|---|---|---|---|
| 0 | `=` | right | assignment, unchanged |
| 1 | `or` `xor` | left | new |
| 2 | `and` | left | new |
| 3 | `not` | right | new, prefix |
| 4 | `<` `>` `<=` `>=` `==` `<>` | left | new |
| 5 | `+` `-` | left | was 1 |
| 6 | `*` `/` `mod` | left | was 2 |
| 7 | `^` | right | was 3 |
| 8 | unary `-` | right | was 4 |
| 9 | `!` | left | was 5 |

**The safety property is not that these numbers are right; it is that the six
existing operators keep their relative order.** `0 < 5 < 6 < 7 < 8 < 9` preserves
exactly `0 < 1 < 2 < 3 < 4 < 5`, and no new level is interleaved *between* two old
ones — the new levels are all below `+`/`-`, except `mod`, which joins `*` and `/`
at an existing level. So no expression that evaluates today changes meaning, and
the suite's 143 value assertions are the test of that claim.

Two decisions inside the table:

**All six comparisons share one level**, rather than splitting equality from
relational as C does. Under left associativity `a < b == c` is `(a<b) == c` either
way; splitting them would change only `a == b < c`, which nobody writes. One fewer
level to explain.

**`not` sits below the comparisons**, as in Python and unlike C. Because it is
spelled as a word, `not a == b` should read the way it reads in English —
`not (a == b)`. C's precedence would make it `(not a) == b`, which is the wrong
reading for that spelling.

### C. The validation pass

`src/validate.rs`'s table grows by one row — though that row is two match arms,
one per state. The comparisons, `and`, `or`,
`xor` and `mod` are binary and behave exactly as `*` and `/` do, so they fall into
arms that already exist. Only `not` is new:

| token | in *expect value* | in *expect operator* |
|---|---|---|
| `not` | ok, **stays** expect value | `ExpectedOperator` |

Both arms must be written explicitly, and the second is the one that matters. The
existing `(Expect::Operator, _)` arm accepts any operator as binary, so without an
explicit arm `1 not 2` would be accepted by the validator and fail later with a
worse message — which is precisely the defect `max(1,*2)` had before Stage 2.

### D. Evaluation

In `src/expression.rs`'s loop, ten new arms, and one existing duplication removed.

The loop distinguishes unary operators twice, with the same condition written out
both times:

```rust
let left_value = if op != &Operator::Une && op != &Operator::Fac { … } else { zero.clone() };
let left_var   = if op != &Operator::Une && op != &Operator::Fac { … } else { None };
```

With `not` that becomes a three-term condition written twice. It becomes
`Operator::is_unary(self) -> bool`, called in both places.

**Semantics, and what each reuses:**

- **Comparisons** go through the `PartialOrd` that `Number` already has — the one
  Stage 1 made agree with `PartialEq` by comparing mathematical value rather than
  enum variant. So `2 == 6/3` is true with no new code. `partial_cmp` returns an
  `Option`; `None` is unreachable, because both variants delegate to totally
  ordered types, and it takes `EvalError::Malformed` for the same reason the
  crate's other unreachable branches do.
- **`and` / `or` / `xor`** treat any non-zero value as true, including negative and
  fractional ones, and yield `1` or `0`.
- **`not`** yields `1` for zero and `0` for anything else.
- **`mod`** is `a - b*trunc(a/b)`, truncating toward zero, so `-7 mod 3` is `-1`
  and `7 mod -3` is `1` — the convention of C, Rust, bc and BASIC, every language
  whose spelling this borrows. It is defined on rationals at no extra cost:
  `7.5 mod 2` is `1.5`. **Its zero check goes through `Number::checked_div`**,
  reusing the single place this crate decides what a division by zero is — the one
  Stage 2's Task 7 consolidated from three copies.

**Every new arm calls `limits::check_size` on its result.** A comparison yields one
bit and the check cannot fail; it is there because the rule Stage 1 established is
that *every arm that pushes a value checks it*, and a reasonable-looking exception
is how such an invariant is lost. The register already records a case where
"bounded by construction" was not "checked" and a two-bit value passed a one-bit
budget.

**No short-circuit evaluation.** `0 and (2^1000000)` evaluates its right operand,
because a stack machine has both operands before it sees the operator. That
expression is therefore refused by the size budget rather than returning `0`.
Short-circuiting would need jumps in the compiled form — a real change, not a
detail, and not part of this work.

### E. Errors

**No new error variants.** `mod` by zero is `EvalError::DivisionByZero`. An
operator in the wrong position is `ParseError::ExpectedValue` or
`ParseError::ExpectedOperator`, both of which exist and both of which carry a span.
The defensive `partial_cmp` branch is `EvalError::Malformed`.

That the error enum does not grow is a small piece of evidence that these
operators fit the shape Stage 2 built rather than straining it.

## Declared changes for 0.3.0

| Before | After |
|---|---|
| `and`, `or`, `xor`, `not`, `mod` are valid variable names | reserved words, in every casing |

Everything else is additive: no public signature changes, no public type grows,
and `Operator` is `pub(crate)`.

## Work order

1. `token.rs` — the ten `Operator` variants, `is_unary`, and the precedence table.
2. `parser.rs` — the regex, and the five words in `tokenize`.
3. `validate.rs` — the two `not` arms.
4. `expression.rs` — the ten evaluation arms, the `is_unary` extraction, `mod`.
5. Tests, including the precedence boundaries and the two-character spans.
6. README, crate docs, and the CHANGELOG line for the reserved words.

**Step 1 lands green alone**: the new variants exist but nothing produces them,
because the tokeniser cannot yet spell them, so every new evaluation arm is
unreachable.

**Steps 2, 3 and 4 must land together.** The moment the tokeniser emits a `mod`
token, the validator has to have an opinion about it and the evaluator has to be
able to compute it. Splitting them across commits leaves an intermediate state
where a legal expression reaches an evaluator with no arm for it — which, in a
crate whose whole previous stage was about not having unreachable-looking
branches that are actually reachable, is the wrong thing to leave in history even
briefly.

## Testing

- **The 143 value assertions stay green, untouched.** This is the test of section
  B's claim. If one needs changing, the ladder is wrong.
- **Precedence tests are written at the boundary between two levels, with inputs
  that give a *different* answer if the boundary is wrong.** A precedence test that
  passes under both readings is not testing precedence:

  | expression | correct | if the level were wrong |
  |---|---|---|
  | `1 or 0 and 0` | `1` | `0` (if `or` and `and` shared a level) |
  | `not 5 == 1` | `1` | `0` (with C's precedence for `not`) |
  | `2 + 3 < 6` | `1` | `3` (if comparison bound tighter than `+`) |
  | `7 mod 3 * 2` | `2` | `1` (if `mod` were weaker than `*`) |

  The second was hard to find: with 0 and 1 the two readings of `not` agree in most
  cases — `not 0 == 0`, `not 1 == 0` and `not 2 == 2` all give the same answer
  either way. Only an input where `not x` is 0 and x differs from the comparand
  separates them.
- **Two-character tokens carry two-byte spans.** `<=` spans `2..4`, not `2..3`.
  This is the class of error that is invisible to every other test, because the
  message stays right while the caret moves, and this crate has been bitten by it
  before.
- **Truthiness across the value space:** `0.5 or 0`, `(-1) and 1`, `1/3 and 2`.
- **`mod`'s sign convention on all four combinations** of operand signs, plus a
  rational operand and `7 mod 0`.
- **Comparison across variants:** `2 == 6/3` and `0.5 < 2/3`, which pin that
  comparison asks the value and not the enum tag.
- **The reserved words:** `and = 5` now fails, with `ParseError::ExpectedValue`
  rather than the generic message.

**One rule stays untestable, and the register should say so:** the `check_size`
call on a comparison's result cannot fail, because 1 and 0 occupy one bit. It is
maintained by review, not by test.

## Definition of done

- `cargo test` green with the 143 value assertions unmodified.
- `cargo fmt --check` clean; clippy compared per lint against the branch point,
  measured on a cold cache.
- Every row of the precedence table exercised by a test that distinguishes it from
  its neighbours.
- The reserved-word break recorded for the 0.3.0 CHANGELOG.
- `docs/tech-debt.md` updated with the untestable `check_size` rule, and with
  anything this work leaves behind.
- `Cargo.toml` still at `0.2.0`; the version bump belongs to the release.
