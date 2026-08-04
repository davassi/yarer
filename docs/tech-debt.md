# Tech debt

Known structural and maintainability debt, and follow-up work deliberately left
undone. Every entry below was verified against the source at `3104971`, the head
of the Stage 1 reliable-core work.

Entries name files and symbols rather than line numbers: line numbers drift, and
half of those recorded during Stage 1 were stale within a day.

Nothing here is a known wrong answer. The Stage 1 review found no incorrect
result, no reachable panic on any evaluation path, and no way to route an
expression around a size guard. These are the things worth fixing next, not
things that are broken now.

## Structural

**`impl Div for Number` panics on a zero divisor.** It builds
`BigRational::new(v1, v2)`, which panics when `v2` is zero. Unreachable through
evaluation — `RpnResolver::resolve` guards against a zero right operand before
dividing — but this is a public `std::ops` impl that panics on ordinary input, so
a consumer using `Number` directly can hit it. Fixing it requires `Div` to become
fallible, which is a breaking API change; it belongs with Stage 2's typed-error
pass rather than on its own.

**`Token::operator_priority` panics on an unrecognised operator**
(`_ => panic!("Operator '{o}' not recognised. This must not happen!")`). Reachable
from the public `Token::compare_operator_priority`. Same shape and same home as
the entry above: it wants a `Result`, which is a breaking change.

**Four functions exceed the 100-line clippy threshold**, measured at `3104971`:
`RpnResolver::reverse_polish_notation` at 168, `RpnResolver::resolve` at 155,
`functions::eval` at 108, and the `test_expressions` integration test at 107. All
four were already over before Stage 1, but `reverse_polish_notation` grew during
it, in Task 3 and again in Task 4. This is the clearest structural debt Stage 1
leaves behind. `reverse_polish_notation` is a single `match` over token kinds
with the bracket-frame bookkeeping interleaved; the arms are separable.

**`functions::eval`'s match arms are repetitive.** The trigonometric arms differ
only by the `f64` method they call. Folding them into a helper was deliberately
not done during Stage 1 because Tasks 2 through 4 all extended the same match and
the conflict was not worth it. That constraint is gone now.

**`Limits` is not `#[non_exhaustive]`**, and it has one field. Adding a second
knob later is a breaking change unless this is decided before 0.3.0 ships.

**`Session` exposes no accessor for its limits**, and its `variable_heap` is
private, so there is no way to evaluate against an existing variable heap under
different limits. An embedder wanting "tight budget for untrusted input, loose
for trusted, same variables" has to rebuild the heap themselves through
`RpnResolver::parse_with_borrowed_heap`. Fine for Stage 1's scope; it belongs in
Stage 2's API pass.

## Performance

**`parse_decimal_literal` is quadratic in the number of fractional digits.** It
builds the denominator with a `for _ in 0..fractional_digits` loop of bignum
multiplications rather than one `pow`. Measured at 1.54 s for a 200,000-digit
fractional literal. Bounded by input length and outside the size budget's reach,
since the budget checks the value after it has been built.

**`apply_functional_token_operation` clones its right operand needlessly.** It
does `match (ln, rn.clone())` and the match consumes both by value, so the clone
is never used. Under the 1 Mibit budget that is up to roughly 128 KB copied on
every `+`, `-`, `*` and `/`.

**`RpnResolver::factorial_helper` is the naive sequential product.** Binary
splitting would decouple running time from the bit budget. Only worth doing if
someone actually raises `Limits::max_value_bits` — at the 1 Mibit default the
worst admitted case, `71421!`, takes about 0.43 s in release. This is the reason
`max_value_bits`' doc warns that time scales superlinearly with the budget.

## Deferred from the Stage 1 review

Small items raised by reviewers, judged not worth their own fix round at the
time. Each was verified as still open at `3104971`.

**`Number::decimal` checks `denom().is_one()` rather than reducing.** An
externally built `Ratio::new_raw(4, 2)` is integral but unreduced, so it slips
past the canonicalisation invariant and becomes a `DecimalNumber(4/2)`. That same
value also makes `PartialEq` and `PartialOrd` disagree. One-line fix:
`value.reduced()`. Not reachable through parsing — the parser never produces an
unreduced rational — so this is about the public constructor's contract.

**`setf` silently does nothing when given NaN or infinity.**
`BigRational::from_float` returns `None` and the `if let Some(value)` has no
`else`, so the variable is simply never set and the caller is not told. The
rewritten doc comment does not mention it.

**Untested behaviours:**

- `2.0!` returns `2` since canonicalisation landed, where it used to error. User
  visible, and pinned by nothing.
- `1/0.0` — the decimal-literal form of division by zero, which used to panic and
  was incidentally fixed by canonicalisation. `1/0` is tested; the form that
  actually panicked is not.
- The canonicalisation invariant test never reaches `Div` Decimal/Decimal, the
  `apply_functional_token_operation` decimal arms, or `power_integer`'s decimal
  arms. Adding `0.5+0.5`, `1.5/0.5` and `(0.5)^-1` to the existing loop covers
  all three.
- `(-1)^odd`. Only the even-exponent degenerate case is pinned.
- Exponent zero, the `n = 0` and `n = 1` factorial early returns, and an
  expression landing exactly on the size budget.

**Diagnostics that are still generic or slightly wrong:**

- A bare `()` outside a function call falls through to the generic
  malformed-expression message. Pre-existing.
- `COMMA_OUTSIDE_CALL_ERR`'s wording misdescribes the nested case: a comma inside
  a plain bracket nested within a call reads as if no call were open at all.
- `sin[5]` evaluates, because `[` and `]` are bracket aliases, while the error
  text and the README both say a function must be followed by `(`. Either the
  aliases or the wording should give way.
- `max(1,*2)` passes the arity check with `given == 2` and fails later at
  evaluation. Operator-sequence validation is deferred to Stage 2 by design; this
  is the visible edge of that gap.

**Polish:**

- `resolve_decimal!` no longer asserts the decimal variant, so the name is a
  misnomer. `resolve_approx!` would read truer.
- `EXPONENT_TOO_LARGE_ERR` is lowercase after `Runtime error:`, unlike its
  siblings in `rpn_resolver.rs`, though consistent with `limits.rs`.
- Two `#[allow]` attributes on `predicted_factorial_bits` where `#[expect(...)]`
  would self-report once the casts stop needing suppression.
- `test_max_min` is a single loop over three expressions, so a failure on the
  first skips the other two.
- Several README fenced blocks holding CLI transcripts and the built-in function
  list are tagged ```rust. Pre-existing mislabelling.

## A pattern worth remembering

Four times during Stage 1, a test passed for a reason other than the one it
named, because a **different** guard caught its input first:

- `test_growth_through_multiplication_is_caught` was meant to exercise the
  post-hoc `Mul` check, but with the original budget the power prediction refused
  the input before any multiplication ran.
- The variable-budget test was first drafted as `"x+1"`, where the `Add` arm's
  existing `check_size` would have fired instead of the variable push under test.
  The committed test uses bare `"x"`.
- `2!` cannot demonstrate the factorial prediction's one-bit shortfall, because
  the operand check rejects the literal `2` first. The only route to an unchecked
  `2` is a function result.
- Closing the function arm then made the factorial's own post-hoc check
  unreachable, so the test that had just been written to cover it started passing
  for a third reason again.

A guard that is correct only because something upstream shields it looks tested
when it is merely shadowed. Two things catch it: assert *which* check fired, by
its distinctive wording, rather than merely that an error occurred; and where two
wordings cannot be told apart, break the guard on purpose and confirm the test
goes red.
