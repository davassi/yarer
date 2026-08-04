# Design — Stage 1: A Reliable Core

Date: 2026-08-04
Status: approved, not yet implemented
Target release: part of 0.3.0 (published after Stage 2; Stage 1 does not ship on its own)

## Context

Yarer 0.2.0 is published on crates.io and is therefore used as a library, not only
as a REPL. The goal for the next development cycle is to make it *production ready*:
something an embedder can rely on. Breaking changes are acceptable — the crate is
pre-1.0 and semver permits them — provided they are declared.

"Production ready" spans three subsystems with a one-way dependency between them,
so the work is split into three stages, each with its own spec, plan and merge:

1. **Reliable core** (this document) — correctness, guaranteed termination, argument validation.
2. **Public API** — typed errors replacing `anyhow::Error`, error positions, `Send`/`Sync` session.
3. **Surface** — script mode for the binary, clippy in CI, MSRV, CHANGELOG, fuzzing.

The order matters: typing the error enum before deciding which error conditions exist
means writing that enum twice. The size limits introduced in Stage 1 create an error
condition that does not exist today.

## Problems in scope

Every item below was reproduced against the 0.2.0 binary before being written down.

| # | Defect | Evidence |
|---|--------|----------|
| 1 | `abs(-3)!`, `floor(2.5)!` and `max(3,2)!` all fail | "Factorial is only defined for non-negative integers" |
| 2 | `Number` violates the `PartialEq`/`PartialOrd` contract | `NaturalNumber(2) == DecimalNumber(2/1)` is `false`, `>=` is `true` |
| 3 | `999999999!` and `10^100000000` never terminate | both hit a 5-second timeout (exit 124); the second also exhausts memory |
| 4 | Function arity is never validated | `max(1,2,3)` reports the generic "malformed expression" |

Defects 1 and 2 share a single root cause: the same mathematical value has two
representations (`NaturalNumber(2)` and `DecimalNumber(2/1)`), and the code branches
on the enum tag instead of the value.

## Explicitly out of scope

- **Undefined variables keep evaluating to 0.** `y+1` returns `1` with no warning.
  This is a deliberate decision by the maintainer: it matches `bc`, and changing it
  would require distinguishing an assignment target from a value read, since `x=5`
  reads `x` before assigning it.
- **Prefix factorial stays accepted.** `!5` returns `120` because `mod_unary_operators`
  (`parser.rs:56`) treats `Fac` as an operand-seen token. It produces no wrong result
  and no hang — it only accepts a spelling nobody writes. Tightening the grammar needs
  a sequence-validation pass, which is designed far better alongside error positions in
  Stage 2. Doing it now means writing that pass twice.
- Typed errors, error positions, `Send`/`Sync`, script mode, clippy in CI, MSRV,
  CHANGELOG, fuzzing — Stages 2 and 3.
- New mathematical capability (`%`, scientific notation, hyperbolic functions,
  user-defined functions). Those are features, not reliability, and deserve their own
  decision.

## Components

### A. Canonical `Number` — closes defects 1 and 2

**Invariant: `DecimalNumber` implies a denominator different from 1.** An integral
value has exactly one representation, `NaturalNumber`.

- A single internal constructor `Number::decimal(BigRational) -> Number` degrades to
  `NaturalNumber` when `denom == 1`, and becomes the only internal way to build a decimal.
- `to_decimal_number()` (`rpn_resolver.rs:512`) is deleted. It is the function that
  *creates* defect 1, forcing the `Decimal` tag onto integral results of `abs`, `floor`,
  `ceil`, `round`, `max` and `min`, which factorial then rejects.
- Factorial stops inspecting the tag and inspects the value, through a new
  `Number::as_integer(&self) -> Option<BigInt>` returning `Some` for a `NaturalNumber`
  and for a rational whose denominator is 1. That logic already exists as
  `integer_exponent()` (`rpn_resolver.rs:519`), written for exponentiation: it becomes
  one method used by both call sites rather than a second copy.
- `PartialEq` changes from derived to hand-written and value-based, consistent with the
  existing hand-written `PartialOrd`.

The last two points are redundant by construction — with the invariant in force the two
representations never coexist. Both are done anyway because `Number` is a **public enum**:
any consumer can construct `Number::DecimalNumber(BigRational::from_integer(2.into()))`
by hand. The invariant protects our code, value-based `PartialEq` protects us from theirs.

The tests in `session.rs` that assert `DecimalNumber(-5.0)` keep passing once the result
becomes `NaturalNumber(-5)`, because equality is by value.

**The integration tests do not all survive, and that was checked rather than assumed.**
The `resolve_decimal!` macro (`tests/integration_tests.rs:17`) asserts the *variant* as
well, through `matches!(result, Number::DecimalNumber(_))`, across roughly 80 call sites.
Under the invariant every integral result — `3*2^3+6/(2+1)`, `sqrt(16)`, `exp(0)`,
`floor(3.7)` — becomes a `NaturalNumber` and those assertions fail.

The repair is one line, not eighty. Those assertions never expressed intended behaviour;
they photographed the enum tag as a side effect of the macro's name. The property worth
asserting is the value, which the macro already checks separately. So the macro drops the
variant assertion, and the invariant gains two dedicated tests that state it outright:
integral results are `NaturalNumber`, non-integral results stay `DecimalNumber`. Eighty
incidental assertions become two intentional ones.

### B. Size limits — closes defect 3

The principle is to **predict the size of the result and refuse before computing it**,
rather than computing under a timeout. No threads, no interruption: a deterministic,
testable, instantaneous decision.

- **`n!`** — the bit length of `n!` is estimable without computing it
  (`≈ n·log₂n − 1.44·n`). `999999999!` exceeds any sensible threshold and is refused
  immediately; `1000!` occupies 8530 bits and passes.
- **`base^exp` with integral `exp`** — the result needs `size(base) × |exp|` bits, known
  exactly and immediately. `10^100000000` asks for ~4·10⁸ bits and is refused; `2^1000`
  passes. A negative exponent is measured on its magnitude: the reciprocal of a huge
  integer is an equally huge rational.
- **`+ − × ÷`** — the result already exists, so it is checked after the fact
  (`bits()` is cheap). This is required because a chain such as
  `x=10^1000; x=x*x; x=x*x` grows through multiplications that are individually
  under the threshold.

Throughout, `size(n)` means `bits()` for a `NaturalNumber` and
`numer().bits() + denom().bits()` for a `DecimalNumber`. It is one function, used by
every check.

One knob: `Limits { max_value_bits: u64 }`, default 1 Mibit (1_048_576 bits,
roughly 315_000 decimal digits) — generous for any legitimate use. It lives on the
`Session`, reached through `Session::with_limits(limits: Limits) -> Session`;
`Session::init()` delegates to it with `Limits::default()`. This is a pure addition:
no existing signature changes.

Limits are **on by default**. An opt-in limit leaves the default configuration
non-terminating, which is precisely the state this stage exists to remove.

**The 1 Mibit default was measured, not guessed.** A bit budget bounds memory directly
and running time only indirectly: `n!` is not one multiplication but a loop of `n`
bignum multiplications, so cost grows faster than the size of the result. A limit
calibrated on memory alone can still admit a computation taking several seconds, which
inside a service is the same availability problem in a quieter form. `power_integer`
uses repeated squaring — `O(log exponent)` multiplications — so for a fixed bit budget
the factorial is the more expensive of the two predictive checks; it dominates the
worst case.

The factorial prediction uses the full Stirling series, `n·log2(n) − n·log2(e) +
0.5·log2(2πn)`, not just its two leading terms: the omitted correction term is close to
ten bits at the scale this measurement operates at, enough to let a two-term prediction
admit a value whose actual size is over budget (an earlier pass of this measurement
caught exactly that — see the note below). With the correction included, the prediction
matches `lgamma(n+1)/ln(2)` to under a bit, so — like the power prediction, which
overestimates for the opposite reason — the factorial prediction no longer
underestimates the size it guards.

Walking `n` down from `200000!` (refused, ~3.2 Mibit) against a release build (`cargo
build --release`) found the boundary the 1 Mibit default admits: `71421!` succeeds
(predicted 1_048_568 bits, matching its actual bit length exactly), while `71422!` is
refused (predicted 1_048_584 bits, likewise exact). Timing the admitted boundary case,
```
time (printf '71421!\nquit\n' | ./target/release/yarer -q)
```
took ~0.43s across three runs, comfortably below one second on ordinary hardware. For
comparison, the boundary power case, `2^524288` (the largest exponent whose predicted
1_048_576 bits still fits), took ~0.05s — confirming the factorial, not the power, sets
the worst case. Since the worst case the default admits already stays well under the
one-second budget, `max_value_bits` keeps its `1 << 20` value; no lowering was needed.

**Note on the correction term's origin.** The first pass of this measurement used only
the two leading Stirling terms and found the boundary at `71422!` — but that build's
prediction for `71422!` (1_048_574 bits) undershot the value's actual bit length
(1_048_584, verified independently), meaning the guard admitted a computation 8 bits
over its own nominal budget. The gap matched the omitted `0.5·log2(2πn)` term almost
exactly, which is why that term is now included above rather than left as a documented
caveat.

One consequence to accept knowingly: for `+ − × ÷` the check happens after the fact,
so an operation whose operands are each just under the limit allocates the oversized
result before it is rejected. The overshoot is bounded by roughly a factor of two and
is transient. Avoiding it would mean predicting the size of every arithmetic result,
which buys little for the complexity it costs.

### C. Function arity — closes defect 4

`MathFunction` gains `arity(self) -> u8`: 2 for `Max` and `Min`, 1 for the rest.

The `MathFunction::None` variant is unreachable — `Token::get_some` never yields it, and
`resolve()` already answers it with "This should never happen!". `arity()` reports 1 for
it rather than panicking, so that no input can reach a panic through this path. Removing
the variant outright is a public-API change and belongs to the Stage 2 API pass.

The shunting yard keeps an **argument-count stack** alongside the operator stack, with
one entry per open bracket recording whether that bracket opens a function call and how
many arguments it has seen. Each `,` increments the top entry; on the closing bracket the
count is compared against the declared arity. A `,` whose enclosing bracket is not a
function call becomes its own diagnosable error instead of the generic "malformed".

The check happens while building the RPN form, not while evaluating it: it does not
depend on runtime values, so `max(1,2,3)` is wrong always, not "wrong once you reach it".

**Declared behaviour change beyond the four defects.** Today `sin 5` works and equals
`sin(5)`: parentheses are optional. That is not a design decision but an accident of the
algorithm — the function sits on the operator stack and is emitted at the end regardless.
Argument counting has no defined meaning without a bracket to count within, so
**parentheses after a function name become mandatory**, as documented in every calculator
and as `bc` requires. Neither the tests nor the README use the bare form, so migration
cost is nil, but it is observable and belongs in the CHANGELOG.

### D. Extracting function evaluation

`resolve()` is 243 lines (clippy: `this function has too many lines (243/100)`), and
component B adds checks at three separate points inside it. The `match fun { … }` block
spanning `rpn_resolver.rs:194`–`:296` moves into a dedicated unit with a clear boundary:
it takes the function and the operand stack and returns `Result<Number>`.

This is not gratuitous refactoring — it is the code the other components modify, and
leaving it in place would mean shipping a 280-line function.

## Errors

Error values remain `anyhow` strings in this stage; typing them is Stage 2. The four new
conditions are nevertheless introduced as **distinct, specific messages**, never as another
reuse of `MALFORMED_ERR`:

- size limit exceeded — names the limit and the requested size
- wrong arity — names the function, the expected count and the given count
- comma outside a function call
- function name not followed by `(`

The reason is practical: Stage 2 turns conditions into enum variants, and every condition
that collapses into the generic string today is a variant to excavate tomorrow.

## Work order

`D → A → B → C`, deliberately:

1. **D** is a pure move with unchanged behaviour, so it goes first, with the existing
   64 tests green on both sides of it as the evidence.
2. **A** changes the type the other two components consume, so it precedes them.
3. **B**, then **C** — independent of each other.

Behaviour changes then land in an already-clean structure and each commit has a
readable diff.

## Testing

Test-driven: for each defect the failing test is written first.

- `abs(-3)!`, `floor(2.5)!`, `max(3,2)!` return the correct factorial.
- A table of pairs asserting `a == b` if and only if `a.partial_cmp(&b) == Some(Equal)`,
  across both variants.
- `999999999!` and `10^100000000` return `Err`. These tests are self-diagnosing: they must
  complete instantly, so a broken limit hangs the suite visibly rather than passing quietly.
- A `Session` built with a deliberately small `max_value_bits` refuses an expression that
  the default limit accepts, proving the knob is wired through.
- `max(1)`, `max(1,2,3)`, `sin(1,2)`, `max()` each produce their specific message.
- `sin 5` is rejected; `sin(5)` is unchanged.
- `x=y=5` still assigns both variables and returns 5, and `x=2; y=3; x*y` still returns 6
  (chained assignment and chained expressions are load-bearing existing behaviour).
- The 64 existing tests stay green except where this document declares otherwise.

## Definition of done

- `cargo test` green.
- `cargo clippy --all-targets` at or below the current 40 warnings.

  Not "with `too many lines` gone", which was the original wording and was
  unattainable: splitting a 243-line function into 141 + 102 leaves both over
  clippy's 100-line threshold, and the later components add lines to each. Component D
  buys the module boundary and a 40% reduction, not silence from that lint. Check the
  count per category, too — a stable total can hide one regression offsetting one
  improvement, which is exactly what happened when component D landed.
- `cargo fmt --check` clean.
- The default `max_value_bits` justified by a recorded timing, not asserted.
- Every declared behaviour change recorded for the 0.3.0 CHANGELOG entry.
