# Design — Stage 2: The Public API

Date: 2026-08-25
Status: approved, not yet implemented
Target release: 0.3.0, together with Stage 1. Neither stage ships on its own.

## Context

Stage 1 merged to `master` as `f4f3d3f` on 2026-08-04. It made the evaluation core
reliable: canonical `Number`, size limits that guarantee termination, validated
function arity. What it deliberately did not touch is the surface an embedder
programs against.

That surface is still the 0.2.0 one, and it has three properties no library that
calls itself production ready should keep:

1. **Every error is a string.** `anyhow::Error` reaches the caller with no
   structure. A consumer that wants to react to "division by zero" differently
   from "the expression is malformed" has to match on message text.
2. **No error knows where it is.** The parser has byte offsets in hand —
   `EXPRESSION_REGEX.find_iter` yields `m.start()` and `m.end()` in
   `parser.rs:26-31` — and discards them. An embedder putting yarer behind a text
   field cannot underline the offending token.
3. **Two public functions panic on ordinary input**, and one pass of validation is
   missing, so three distinct malformed expressions collapse into one generic
   message.

Stage 2 fixes all three. It is a breaking release by nature: typed errors change
every signature that can fail.

**`Send`/`Sync` is out.** It was listed for this stage, and the maintainer's
decision on 2026-08-25 is that yarer is a single-thread library. That removes the
only item that required restructuring ownership, so `Rc<RefCell<HashMap<..>>>`
stays inside `Session`. The one consequence worth recording, so the decision can be
revisited knowingly: a `!Send` `Session` cannot be held across an `.await` inside a
`tokio::spawn`, even when the surrounding logic is sequential.

## Problems in scope

Every row was reproduced against the binary built from `f4f3d3f` on 2026-08-25,
by piping the expression into the REPL. The message column is verbatim output.

| # | Expression | Today's output | Wrong because |
|---|---|---|---|
| 1 | `1@2` | `Parse Error: Unexpected token '@'.` | Correct, but the caller gets a string and no position |
| 2 | `()` | `Runtime Error: The mathematical expression is malformed.` | A parse defect diagnosed at evaluation, generically |
| 3 | `2 3` | `Runtime Error: The mathematical expression is malformed.` | Same |
| 4 | `2(3+4)` | `Runtime Error: The mathematical expression is malformed.` | Same |
| 5 | `1+` | `Runtime Error: ... malformed. Invalid Left Operand.` | Names a stack condition, not the missing operand |
| 6 | `max(1,*2)` | `Runtime Error: ... malformed. Wrong number of parameters for function Max` | **Blames `max`'s arity, which is correct.** The defect is the `*` in argument position |
| 7 | `max(1,(2,3))` | `Parse Error: ',' is only valid between the arguments of a function call.` | A call *is* open. The message is false for the nested case |
| 8 | `max(1,2,3)` | `Parse Error: Function 'max' expects 2 argument(s), 3 given.` | Correct and specific, but positionless and still a string |
| 9 | `!5` | `120` | Prefix factorial is an accident of `mod_unary_operators`, undocumented |
| 10 | `Number / Number` | panic on a zero divisor | A public `std::ops` impl that panics on ordinary input |
| 11 | `Token::compare_operator_priority` | panic on a non-operator token | Reachable from outside the crate |
| 12 | `session.set("pi", 3)` | silent no-op | The caller is not told the write was refused |
| 13 | `session.setf("x", f64::NAN)` | silent no-op | Same |

Rows 2 through 6 share one root cause: nothing validates the *sequence* of tokens.
The shunting yard checks brackets and arity, the evaluator checks the stack, and an
expression that is neither unbalanced nor of wrong arity falls between them and is
reported by whichever guard happens to trip first — which, as row 6 shows, can
accuse a construct that is not at fault.

## Explicitly out of scope

- **`Send`/`Sync` and the ownership restructure.** See Context.
- **Multiple diagnostics per parse.** Error recovery in a shunting yard buys little
  for single-line expressions, where the second error is usually an echo of the
  first. The first error, with a position, is the deliverable.
- **Implicit multiplication.** `2(3+4)` gets a precise error, not a meaning.
  Accepting it would be new mathematical capability, which is a feature decision.
- **Undefined variables keep evaluating to 0.** Unchanged from Stage 1, and for the
  same recorded reason.
- **Clippy in CI, MSRV, CHANGELOG, fuzzing, script mode.** Stage 3.
- **The `Cargo.toml` version.** Stays at 0.2.0. The bump to 0.3.0 goes with the
  release, not with a stage.

## Components

### A. The error module — `src/error.rs`

Three public types, all `#[non_exhaustive]`, all built with `thiserror` (already a
dependency, already used for `ConversionError` at `token.rs:471`).

```rust
pub struct Span { pub start: usize, pub end: usize }   // byte offsets, Copy

pub enum Error { Parse(ParseError), Eval(EvalError) }  // + From for both

pub enum ParseError {
    UnexpectedCharacter         { text: String, span: Span },
    EmptyExpression,
    EmptyGroup                  { span: Span },
    UnbalancedBracket           { span: Span },
    CommaOutsideCall            { span: Span },
    CommaInPlainBracket         { span: Span },
    EmptyArgument               { span: Span },
    WrongArity                  { function: MathFunction, expected: u8, given: usize, span: Span },
    FunctionRequiresParentheses { function: MathFunction, span: Span },
    BracketUnclosedAtSemicolon  { span: Span },
    ExpectedValue               { found: String, span: Span },
    ExpectedOperator            { found: String, span: Span },
    Malformed,                                     // structural fallback, names no token
}

pub enum EvalError {                    // every span here is Option<Span>
    DivisionByZero, ValueTooLarge { bits, limit }, ComputationTooLarge { predicted_bits, limit },
    FactorialNotNatural, FactorialOperandTooLarge, ExponentTooLarge,
    PowerOperandsTooLarge, InvalidPower, OperandTooLargeForFloat, NotARealNumber,
    ReadOnlyConstant { name: String }, AssignmentTargetMissing,
    NotFinite { value: f64 }, Malformed,
}
```

Four decisions inside that shape:

**`Span` is mandatory on parse errors and optional on evaluation errors.** A parse
error always comes from a token the user wrote — with the single exception of
`Malformed`, the structural fallback of component C, which names no token and
therefore carries no position. An evaluation error need not:
`session.setf("x", f64::NAN)` fails with no source text in existence. Making the
span mandatory would mean inventing a position for an error that has none.

**`ValueTooLarge` and `ComputationTooLarge` stay two variants.** They correspond to
`check_size` (measured, worded "occupies") and `check_predicted_size` (predicted,
worded "would need") at `limits.rs:70-78`. The tech-debt register records that this
distinction was, four times during Stage 1, the only thing separating a test that
passed for its stated reason from one shadowed by an upstream guard. Collapsing the
two variants would discard exactly the signal that worked.

**`CommaInPlainBracket` is new**, and exists because row 7 of the problem table is a
false statement, not merely a vague one.

**The `"Runtime error:"` / `"Parse Error:"` prefixes leave `Display`.** The category
now lives in the type. The prefixes have already drifted — the register notes
`Runtime Error:` in one place, `Runtime error:` in another, and one message in lower
case after the colon — and the drift is only possible because the prefix is
duplicated across a dozen string constants. Whoever prints composes it: the REPL,
once.

Two error paths disappear rather than being typed:

- `Token::tokenize` returns `Option<Token>` and `parser.rs:29` maps `None` to the
  malformed message. `None` is returned only for an empty string, and the regex
  never yields an empty match: it is a dead branch. With `Token` becoming internal
  (component D), `tokenize` returns `Token` and the branch goes.
- `anyhow` leaves `Cargo.toml`. 57 occurrences in `src/`, zero in `tests/`, zero in
  the public API. It is not being replaced; it is being removed.

The error types hold only `String`, integers, `f64`, `MathFunction` and `Span`, so
they are `Send + Sync + Clone + PartialEq` even though `Session` is not. An embedder
can box them as `dyn Error + Send + Sync`.

### B. Spans, end to end

`Parser` produces `Vec<Spanned<Token>>` where `Spanned<T> { node: T, span: Span }`,
keeping `Token` itself comparable so existing token-level tests are unaffected. The
spans survive the validation pass and the shunting yard, so the compiled expression
is a `VecDeque<Spanned<Token>>` and an evaluation error can name the operator that
produced it.

Presentation is one function, on `Error`:

```rust
pub fn span(&self) -> Option<Span>;
pub fn render(&self, source: &str) -> String;   // message, source line, caret
```

Both live on `Error`, and neither `compile` nor `eval` returns an `Error` — they
return the half they can produce. `Error` is the union for a caller that wants one
type across both calls, reached through `From`. That is how the REPL renders a parse
failure and an evaluation failure through a single code path.

`render` converts a byte offset to a column by counting `char`s in `source[..start]`.
This matters concretely rather than theoretically: the expression regex accepts `×`
and `÷` (`parser.rs:22`), which are two bytes each, so a caret positioned by byte
offset lands in the wrong column the first time anyone types `2×3`. Grapheme
clusters and double-width characters are out of scope — no accepted token contains
one.

### C. The validation pass

A pass between tokenising and the shunting yard, over `&[Spanned<Token>]`, with two
states: *expect value* and *expect operator*.

| token | in *expect value* | in *expect operator* |
|---|---|---|
| number, variable | ok → expect operator | `ExpectedOperator` |
| function name | ok, next token must be a bracket | `ExpectedOperator` |
| `(` `[` | ok, stays expect value | `ExpectedOperator` |
| `)` `]` | `EmptyGroup` if it closes an empty group, else `ExpectedValue` | ok |
| `+` `-` | become unary, stays expect value | ok → expect value |
| `*` `/` `^` `=` | `ExpectedValue` | ok → expect value |
| `!` | `ExpectedValue` (see below) | ok, stays expect operator |
| `,` | `EmptyArgument` | ok → expect value |
| `;` | ok (an empty segment is a no-op, unchanged) | ok → expect value |
| end of input | `ExpectedValue` | ok |

This is not new machinery. `mod_unary_operators` (`parser.rs:53`) already walks the
token stream holding exactly this state, in a flag named `expect_operand_next`, and
already uses it to tell a binary `-` from a unary one. What it cannot do is refuse:
it returns `Vec<Token>`. The pass grows the ability to say no and becomes
`validate`. The pipeline keeps three passes, not four.

**Two grammar decisions, taken 2026-08-25.**

- **`!5` becomes an error.** It returns `120` today because `mod_unary_operators`
  treats `Fac` as an operand-seen token. It is undocumented, unused, and accepting
  it means the grammar has both a prefix and a postfix factorial.
- **`[` and `]` stay aliases everywhere, including after a function name.**
  `sin[5]` evaluates today; the error text and the README claim a function must be
  followed by `(`. The text gives way, not the aliases: yarer has no indexing, so
  `sin[5]` cannot mean anything else, and an exception in the grammar would cost
  more to explain than it buys.

**What this lets the shunting yard drop.** With sequence validation upstream,
`reverse_polish_notation` no longer needs `struct BracketFrame`, `argument_count`,
the comma counting, the arity check, `pending_function`, `EMPTY_ARGUMENT_ERR`,
`COMMA_OUTSIDE_CALL_ERR` or `UNBALANCED_BRACKET_ERR`. All of it moves into the pass
that has the right traversal and now also the spans. What remains is Dijkstra's
algorithm.

The register's structural entry — `reverse_polish_notation` at 168 lines, grown
during Stage 1 in two tasks out of four — is therefore addressed by removing a job
that was never its own, not by splitting it in half. The measured result goes in the
definition of done rather than being promised here.

**What stays.** The two `if !found_open { … }` branches, with their comments, stay
as `ParseError::Malformed` — they live in the shunting yard, so they are compile-time
conditions; `EvalError::Malformed` is the separate evaluation-time fallback for a
stack that does not end with exactly one value. Validation makes them unreachable by a second route, but
the comment explains why they cannot simply be deleted: falling through would not
raise an error, it would return `2*(3+4)` evaluated as `2 3 * 4 +` — 10 instead of
14. A silently wrong number is worse than an error.

### D. `compile` / `eval` — the public shape

```rust
let mut session = Session::init();
session.set("x", 1)?;

let expr = Expression::compile("1/cos(x^2)")?;   // Result<Expression, ParseError>
let value = expr.eval(&session)?;                // Result<Number, EvalError>

session.set("x", -1)?;
let again = expr.eval(&session)?;                // no recompilation
```

The parse/eval split the error enum draws is now in the signatures: a function that
cannot fail at parse time does not carry parse variants in its error type.

`RpnResolver` disappears; `Expression` replaces it. `Expression<'a>` borrows the
expression text, exactly as `RpnResolver<'a>` does today — making it owned is a
separate change and not part of this stage. `eval` takes `&Session` rather than
`&mut Session` because assignment is an expression: `x=5` writes through the
`RefCell` the session already holds, which is unchanged from 0.2.0.

`build_error: Option<String>` (`rpn_resolver.rs:55`) disappears. It is the point at
which a structured failure is flattened to a string today, before it ever leaves the
crate, and it defers a compile failure to the first evaluation — which, in a loop,
means re-reporting it on every iteration.

Compilation also stops touching the variable heap. `reverse_polish_notation`
currently receives the `Rc` and pre-registers every variable it sees with a value of
zero (`rpn_resolver.rs:565-570`). That write is already vestigial: `resolve` reads
variables with `unwrap_or_else(|| zero)`, so removing it changes no result. Its only
observable consequence would be through a `Session` accessor that lists variables,
which does not exist and is not being added.

**File layout.** `rpn_resolver.rs` is 809 lines doing two unrelated jobs, along the
same seam `compile`/`eval` draws in the API. It splits into `src/shunting.rs` (infix
to RPN) and `src/expression.rs` (the `Expression` type and the evaluation loop),
plus the new `src/error.rs` and `src/validate.rs`.

**Root re-exports.** `lib.rs` re-exports `Expression`, `Session`, `Number`, `Error`,
`ParseError`, `EvalError`, `Span`, `Limits`, `MathFunction`, so the common case is
one `use`.

**Narrowed surface.** `Parser`, `Token`, `Operator`, `Bracket` and `Associate`
become `pub(crate)`. They are the mechanism, not the interface; nobody evaluating an
expression needs them. `Number` and `MathFunction` stay public — the first is the
result type, the second appears in `ParseError::WrongArity`. Reopening this later is
additive; closing it later would be a second break.

### E. Closing the two public panics

The register files these together — "same shape and same home: it wants a `Result`".
They are not the same shape, and only one wants a `Result`.

**`Token::operator_priority` (`token.rs:282-292`) does not become fallible. It
becomes impossible to call wrongly.** Its `_ => panic!` exists because the parameter
is a `Token` while the function's domain is an `Operator`: it accepts brackets,
commas and variables, then explains at run time that it did not want them. Changing
the parameter to `Operator` makes the match exhaustive and deletes the branch — with
no `Result` for callers to propagate, because there is no longer anything to get
wrong. Component D makes `compare_operator_priority` internal as well, so the
reachable path from outside the crate closes twice over.

**`impl Div for Number` (`token.rs:432`) does want a `Result`**, because zero is a
divisor a caller can genuinely supply. The impl is removed in favour of:

```rust
pub fn checked_div(&self, rhs: &Number) -> Option<Number>;
```

`Add`, `Sub` and `Mul` stay: they are total. The asymmetry is information, not
inconsistency.

This also removes duplicated logic. The zero divisor is checked in three places
today — the `Div` arm of `resolve`, and twice inside `power_integer` for a negative
exponent (`0^-1`, in the integer and the rational arm). All three become
`… .ok_or(EvalError::DivisionByZero { span })` over a single check inside
`checked_div`.

### F. Limits, and a budget per evaluation

`Limits` becomes `#[non_exhaustive]`, which closes struct-literal construction and
therefore needs a constructor:

```rust
let tight = Limits::default().with_max_value_bits(4096);
expr.eval_with(&session, tight)?;    // this evaluation only
expr.eval(&session)?;                // the session's budget
```

plus `Session::limits()`. This closes the register's entry about an embedder who
wants "tight budget for untrusted input, loose for trusted, same variables" and
today has to rebuild the heap to get it.

The warning already on `Session::with_limits` — that the built-in constants cost up
to 107 bits, so a budget below that refuses a value the caller never supplied —
applies verbatim to `eval_with` and is repeated there.

### G. Setters that report

`Session::set` and `Session::setf` return `Result<(), EvalError>`.

The register records that `setf` swallows NaN and infinity: `BigRational::from_float`
returns `None` and the `if let Some(value)` has no `else`. The same defect shape sits
one function above, unrecorded: `set` returns silently when the key names a built-in
constant (`session.rs:104-107`), so `session.set("pi", 3)` does nothing and says
nothing. Both are fixed together, reusing `EvalError::ReadOnlyConstant` and
`EvalError::NotFinite` — the same conditions the evaluator already reports for
`pi=3` and for a non-real function result.

The finiteness check lands in one place, filling in the missing direction of the
conversion table (`Number → f64` exists; `f64 → Number` does not):

```rust
impl TryFrom<f64> for Number { type Error = ConversionError; … }
```

`ConversionError` gains `NotFinite` and becomes `#[non_exhaustive]`.

### H. `Number::decimal` and the cost of `reduced()`

The register calls this a one-line fix: `Number::decimal` tests `denom().is_one()`
instead of reducing, so an externally built `Ratio::new_raw(4, 2)` is integral but
unreduced and slips through as a `DecimalNumber`, which also makes `PartialEq` and
`PartialOrd` disagree.

One line as a diff, but not free. `Number::decimal` is on the hot path — every
decimal result passes through it — and `reduced()` computes a gcd, which on values
near the 1 Mibit budget is real work. Values arriving from internal arithmetic are
in all likelihood already reduced, because `BigRational` reduces the results of its
own operations, which would make the gcd pure overhead.

So the shape is decided by measurement, with the criterion fixed in advance rather
than after the number is known: if `.reduced()` costs under 5% on a repeated decimal
expression, one constructor keeps reducing. If it costs more, the constructor splits
into a public `decimal` that reduces (upholding the invariant at the boundary) and a
`pub(crate) decimal_unchecked` for the paths where reduction is guaranteed by
construction.

## Declared API and behaviour changes for 0.3.0

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

Unchanged on purpose: undefined variables read as `0`; `sin[5]` evaluates; chained
assignment (`x=y=5`) and chained expressions (`x=2; y=3; x*y`); every numeric result
Stage 1 pinned.

## Work order

1. `src/error.rs` — the three enums, `Span`, `render`. Nothing consumes them yet.
2. Spans through the parser: `Spanned<Token>`, `tokenize` made total.
3. `src/validate.rs` — `mod_unary_operators` grows the state machine and the frames.
4. `shunting.rs` — drop what component C lists, convert the rest to `ParseError`.
5. `expression.rs` — `Expression`, `compile`, `eval`, `eval_with`; `build_error` goes.
6. Components E, F, G, H — the register's items, each with its own test.
7. `lib.rs` re-exports and the narrowed surface.
8. The REPL becomes the first consumer of `render`; README and doctests follow.

Steps 1 and 2 are additive and land green. The public surface breaks at step 5, and
the tests move with it.

## Testing

Test-driven: the failing test comes first, and for every new error the input is one
that today lands in a *different* guard, so the test cannot pass until the new
diagnosis actually exists. Row 6 of the problem table is the model — `max(1,*2)`
must demand `ParseError::ExpectedValue` while the current code answers with an arity
complaint about `max`.

- The 15 assertions of the form `err.contains("occupies")` in
  `tests/integration_tests.rs` become `matches!(e, EvalError::ValueTooLarge { .. })`.
  A variant cannot be satisfied by the wrong guard, and cannot silently lapse when a
  message is reworded.
- Every variant carrying a `Span` gets a test asserting `start` and `end` exactly.
  An unasserted span drifts on the next parser change without any message looking
  wrong.
- One renderer test on `2×3`: two bytes, one column. It is the only place a caret
  can land wrong without another test objecting.
- One case per error-producing cell of the validation table in component C.
- `!5` is pinned as an error, `5!` as `120`.
- The register's "Untested behaviours" section closes here, being tests only:
  `2.0!`, `1/0.0`, `(-1)^odd`, a zero exponent, the `n = 0` and `n = 1` factorial
  early returns, an expression landing exactly on the budget, and the three decimal
  arms the canonicalisation loop never reaches (`0.5+0.5`, `1.5/0.5`, `(0.5)^-1`).
- `test_max_min` splits: it is one loop over three expressions today, so a failure
  on the first hides the other two.
- Every test Stage 1 left green stays green except where the table above declares
  otherwise.

## Definition of done

- `cargo test` green, `cargo fmt --check` clean.
- `cargo clippy --all-targets` compared per lint against a freshly cleaned baseline,
  not by counting warnings on a warm cache. The line count of
  `reverse_polish_notation` after component C is recorded as a number, whatever it
  turns out to be.
- `anyhow` absent from `Cargo.toml`, and `grep -r "anyhow" src/` silent.
- No `panic!` added, and every surviving `unwrap`/`expect` carries a comment stating
  why it cannot fire. Proving unreachability by exhaustion is what fuzzing is for,
  and that is Stage 3.
- The `.reduced()` measurement recorded, and the chosen shape justified by it.
- Every row of the changes table recorded for the 0.3.0 CHANGELOG entry.
- `docs/tech-debt.md` updated: entries closed here removed, anything new recorded.
