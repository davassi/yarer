# Tech debt

Known structural and maintainability debt, and follow-up work deliberately left
undone. Every entry below was verified against the source at `a7f1220`, the
commit the production-ready-api stage (Stage 2 of the 0.3.0 plan)'s own code
and documentation changes land on. This file's own commit adds and reorganises
tests, and adds two short documentation sections; nothing it touches changes
any claim below.

Entries name files and symbols rather than line numbers: line numbers drift,
and half of those recorded during Stage 1 were stale within a day. Several
symbols named in the Stage 1 register were renamed or moved outright during
Stage 2 — `RpnResolver` is gone; its two former responsibilities are now
`Expression::eval_with` and `shunting::to_rpn` — so this revision updates
those names too, not just the numbers attached to them.

Nothing here is a known wrong answer. Neither stage's review found an
incorrect result, a reachable panic on any evaluation path, or a way to route
an expression around a size guard. These are the things worth fixing next, not
things that are broken now.

## Structural

**Four functions exceed the 100-line clippy `too_many_lines` threshold**,
re-measured cold against `a7f1220`: `Expression::eval_with` at 146,
`validate::validate` at 150, `functions::eval` at 110, and the
`test_expressions` integration test at 115. `too_many_lines` is part of
`clippy::pedantic`, which is only turned on in `src/lib.rs`; the integration
test crate doesn't inherit it, so `test_expressions` has never been
clippy-flagged and this number is a manual line count, same as the original
entry's 107.

Two of the four are Stage 1 holdovers under new names.
`RpnResolver::resolve` (155 lines at Stage 1's close) became
`Expression::eval_with` (146) when `rpn_resolver.rs` was renamed to
`expression.rs`. `RpnResolver::reverse_polish_notation` (168 lines) became
`shunting::to_rpn` — and dropped **under** the threshold, to 91, when Stage 2
split the bracket/arity/comma bookkeeping out of it and into a dedicated
validation pass. That pass is the third function on this list.

`validate::validate` is new, and it is the longest function in the crate. It
replaced logic that used to be scattered — partly inside the shunting yard,
partly in a `mod_unary_operators` pass that had no way to refuse anything —
with a single walk over the token stream that gives every rejection a
position. That is the most visible thing Stage 2 did: five previously
identical "malformed expression" failures are now five distinct, positioned
diagnoses. The length is worth that. Structurally it is the same shape
`reverse_polish_notation` had before its own split: one `match` over `Token`
variants, with `Token::Bracket(Bracket::Close)` (~50 lines) the largest and
most separable arm, `Token::Comma` and `Token::Operator` next. If this needs
to shrink, that arm is where to start.

**The numeric kernels behind `^` and `!` live in `expression.rs`, not
`functions.rs`.** `Expression::power`, `power_integer`, `pow_big_int`,
`pow_big_rational` and `factorial_helper` are bignum arithmetic sitting next
to the evaluation loop that calls them, while `functions.rs`'s own module doc
describes its job as "what happens when that loop meets a `MathFunction`" —
the *named* built-ins (`sin`, `ln`, `abs`, ...), not the `^` and `!`
operators, which never reach `functions::eval` at all. The placement is
defensible on that boundary, and moving the kernels wouldn't change any
behaviour. It is still a real seam: a reader searching for "where does yarer
compute a power" has no reason to expect `expression.rs` over `functions.rs`,
which already holds the crate's other bignum-to-bignum conversions
(`decimal_from_f64`, `number_to_rational`, ...). Worth a look if `functions.rs`
and `expression.rs` are touched together again; not worth a dedicated pass on
its own.

## Performance

**`parse_decimal_literal` is quadratic in the number of fractional digits.**
It builds the denominator with a `for _ in 0..fractional_digits` loop of
bignum multiplications rather than one `pow`. Re-measured on this machine, in
release mode, `Expression::compile` only (no `eval`): a 100,000-digit
fractional literal takes 134 ms, a 200,000-digit one 534 ms — roughly a 4×
increase for a 2× input, confirming the quadratic shape still holds. (The
absolute numbers are machine-dependent and not comparable to the Stage 1
entry's 1.54 s, which was not recorded against a specific build profile or
measurement method; the scaling is the load-bearing fact, not the constant.)
Bounded by input length and outside the size budget's reach, since the budget
checks the value after it has been built.

**`apply_functional_token_operation` clones its right operand needlessly.** It
does `match (ln, rn.clone())` and the match consumes both by value, so the
clone is never used. Under the 1 Mibit default budget that is up to roughly
128 KiB copied on every `+`, `-`, `*` and `/`. Unchanged since Stage 1.

**`Expression::factorial_helper` is the naive sequential product.** Binary
splitting would decouple running time from the bit budget. Re-verified the
boundary this entry depends on: at the default 1 Mibit budget, `71421!` is
still admitted and `71422!` is still refused by the prediction check (in
single-digit microseconds — the prediction, not the computation, is what
refuses it). `71421!` itself now takes about 290 ms on this machine in release
mode; again not comparable to Stage 1's 0.43 s figure without knowing its
build profile, but the shape — the reason `max_value_bits`'s doc warns that
time scales superlinearly with the budget — is unchanged and only worth
fixing if someone actually raises the budget.

**`measure_the_cost_of_reducing_every_decimal`'s two cases exercise only two
of the four `Number::decimal_unchecked` call sites it was drafted to justify.**
Task 9's decision to split `Number::decimal` (reduces) from
`Number::decimal_unchecked` (doesn't) rests on this harness, and the split
covers four call sites: `checked_div`, `apply_functional_token_operation`,
`power_integer`, and `decimal_from_f64`. `"1/3 + 1/7 + 1/11"` exercises
`checked_div` and `apply_functional_token_operation`; `"(2^60000)/3"`
exercises `checked_div` again, on a large numerator. Neither expression
touches `power_integer`'s decimal arm (both bases are integers) or
`decimal_from_f64` (neither expression calls a trig/`sqrt`/`ln`-family
function or a non-integer power). Separately, `"(2^60000)/3"` divides a huge
numerator by a *small* literal denominator, so the Euclidean algorithm
resolves `gcd(huge, 3)` in essentially one step — that case measures the cheap
end of reduction, not the expensive end, which is two operands of comparable
size sharing no small common factor. Neither gap matters for the one-off,
`#[ignore]`d measurement this harness was built for. Both would matter if it
were ever turned into an automated regression gate: it would miss a
regression confined to `power_integer` or `decimal_from_f64`, and it would not
catch reduction getting slower on the case that actually costs the most.

## Polish

**Two `#[allow]` attributes on `predicted_factorial_bits`**
(`clippy::cast_precision_loss`, then `clippy::cast_possible_truncation` and
`clippy::cast_sign_loss`), where `#[expect(...)]` would self-report once the
casts stop needing suppression. Unchanged since Stage 1.

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

Stage 2 ran into the same shape of problem, one level up: a test that asserts
`is_err()` or a value alone, rather than *which* step failed or *which* enum
variant came back, stays green under a regression that changes *why* it
passes. `resolve_err!`'s split into compile-time and eval-time assertions
(`test_invalid_input_is_rejected`, `test_domain_errors_are_rejected`), the
`checked_div` cross-variant test, and `test_wrong_arity_is_diagnosed_by_name`
binding `function` instead of discarding it with `..` are all this same
lesson applied to the typed-error API.
