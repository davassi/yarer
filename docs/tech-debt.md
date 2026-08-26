# Tech debt

Known structural and maintainability debt, and follow-up work deliberately left
undone. Entries were verified against the source at `a7f1220`, the commit the
production-ready-api stage (Stage 2 of the 0.3.0 plan)'s own code and
documentation changes landed on, re-verified against the comparison and logical
operators work that followed it into 0.3.0, and re-verified again against
Stage 3 (surface) for 0.4.0, which closed three of these entries and corrected
a fourth that turned out not to be reproducible. The fix
wave that followed Stage 2's whole-branch review revisited two of them, and both
revisions say so where they stand.

Entries name files and symbols rather than line numbers: line numbers drift,
and half of those recorded during Stage 1 were stale within a day. Several
symbols named in the Stage 1 register were renamed or moved outright during
Stage 2 — `RpnResolver` is gone; its two former responsibilities are now
`Expression::eval_with` and `shunting::to_rpn` — so this revision updates
those names too, not just the numbers attached to them.

Nothing here is a known wrong answer — but that sentence needs its history
attached, because the one that used to follow it was false. It read "neither
stage's review found an incorrect result", and the whole-branch review that
read Stage 2 end to end found one: `Display for Number` printed `inf` for
`(10^400)/3` and `0` for `1/(10^400)`. Both values were held exactly and
computed correctly — `(10^400)/3 * 3` returned the right 401-digit integer —
and only the printed form was wrong, because the `numer/denom` fallback was
guarded on `BigRational::to_f64` answering `None`, which it does not do: it
answers `Some(inf)` on overflow and `Some(0.0)` on underflow. Nothing
signalled the loss, and `f64::try_from` inherited it, reporting `value 'inf'
is out of range` for a value that is neither infinite nor NaN. It is fixed and
pinned, in `token::tests` and in
`test_a_rational_no_f64_can_hold_prints_as_a_ratio_not_as_infinity`.

The claim is kept rather than deleted because it is worth being able to make,
and the correction is kept next to it because a register that quietly drops a
falsified claim is worth less than one that records what falsified it. What
still holds unqualified is the rest: no reachable panic on any evaluation
path, and no way to route an expression around a size guard. These are the
things worth fixing next, not things that are broken now.

## Structural

**Two functions exceed the 100-line clippy `too_many_lines` threshold**,
re-measured cold for 0.4.0: `validate::validate` at 163 and `functions::eval`
at 104. Both now carry `#[expect(clippy::too_many_lines, reason = "…")]` with a
written justification, so CI can deny every warning and a suppression that
stops being needed reports itself.

The other two are gone. `Expression::eval_with` went from 216 to 60 when
Stage 3 split it (see below), and the `test_expressions` integration test is no
longer the outlier it was. `shunting::to_rpn` has never crossed the threshold
and is at 94.

`validate::validate` is the longest function in the crate again, now that
`eval_with` is not. It replaced logic that used to be scattered — partly inside
the shunting yard, partly in a `mod_unary_operators` pass that had no way to
refuse anything — with a single walk over the token stream that gives every
rejection a position. That is the most visible thing Stage 2 did: five
previously identical "malformed expression" failures are now five distinct,
positioned diagnoses. The length is worth that, and the `expect`'s reason says
so. Structurally it is one `match` over `Token` variants, with
`Token::Bracket(Bracket::Close)` (~50 lines) the largest and most separable
arm. If it ever has to shrink, that arm is where to start.

**Stage 3 closed this entry, and the shape of the fix was not quite the one
proposed.** `Expression::eval_with` was 216 lines; it is 60, and
`apply_operator` — which holds what each operator means — is 95. `Stacks`
exists as proposed, and its `push_checked`/`push_truth` pair is what reduced
every operator arm to a single line. The proposal expected a third function
holding the ten truth operators; it was not needed, and would have cost a
nested match over ten operators plus an arm listing the other six unreachably.
`apply_operator` has five lines of headroom, so a seventeenth operator tips it
back over the threshold, and that third function is the answer already
designed for when it does.

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

**`apply_functional_token_operation`'s needless clone is gone, and the cost
this entry claimed for it was never measured.** It did `match (ln, rn.clone())`
where the match consumes both by value, so the clone was dead. This entry said
that meant "up to roughly 128 KiB copied on every `+`, `-`, `*` and `/`" under
the default budget. That is an upper bound on the *size* of a copy, not a
measurement of its cost, and measuring does not support it: best of seven runs
of 2000 evaluations of an expression with 60000-bit operands gives 14.7 ms
without the clone and 15.4 ms with, while two runs without differ by 0.4 ms
between themselves. The bignum arithmetic dominates. Removing it was right
because it was dead code; it was not a performance fix, and the entry is kept
in corrected form because a register that quietly restates a claim it could not
reproduce is worth less than one that says so.

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

**Done in Stage 3.** The two `#[allow]` attributes on
`predicted_factorial_bits` are `#[expect]` now, with a written reason, as is
the one on `Number::decimal`. Every suppression in the crate self-reports if it
stops being needed — except `multiple_crate_versions` at the crate root, which
must stay an `allow` because the lint fires from the manifest and has no span
in the source to attach an `expect` to.

**`MathFunction::None` is public and cannot be produced by parsing.**
`Token::get_some` never yields it, so no expression compiles to one, and
`functions::eval` answers `EvalError::Malformed` for it — an arm that exists
only to keep the `match` exhaustive over a variant no input reaches. It costs
more than a dead arm, because `MathFunction` is public payload inside
`ParseError::WrongArity`: an embedder matching on it sees a variant the
documentation has to explain cannot occur, and `arity()` has to answer
something for it (it answers 1, deliberately, rather than panicking). The
tokeniser's `Token::get_some` already returns `Option<MathFunction>`, so the
"no function here" answer `None` stands in for is carried elsewhere already:
removing the variant means deleting its declaration, its arm in `arity()`, its
arm in `functions::eval`, and the two doc paragraphs that exist to explain
it. It is left alone here because removing a variant from a
public enum is a design change, not a fix, and this fix wave was scoped to
fixes; `#[non_exhaustive]` on `MathFunction`, added in the same wave, is what
makes the removal additive-cost rather than a second break when it happens.

**The size check on `and`, `or`, `xor` and `mod`'s results is shadowed by the
operand check, and no test can reach it.** Every arm of the evaluation loop
that pushes a value calls `limits::check_size` on it, the new ones included.

The design assumed this was unreachable for all ten operators that answer a
truth — "1 and 0 occupy one bit, so the call cannot fail" — and recorded it as
a rule to be maintained by review. That was wrong, and the branch review found
it by running the library rather than reading it. `Limits::with_max_value_bits`
has no lower bound, so a zero-bit budget is constructible; under it both
operands of `0 == 0` cost nothing while the answer costs one bit, and the check
fires with the span on the `==`. The six comparisons and `not` are therefore
testable and are tested, in
`test_a_truth_answered_from_nothing_is_still_checked_against_the_budget`.

What remains genuinely unreachable is the other four. `and`, `or` and `xor` can
only answer 1 if an operand was already worth at least a bit, and `mod`'s
result is bounded by a divisor that must be non-zero and so costs a bit too —
in every case the operand check upstream fires first. Those four are the entry.
They are recorded rather than exempted because the alternative is a
reasonable-looking exception to a rule, and this file already records what
happened the last time one was made: `floor(exp(1))!` slipped a two-bit result
past a one-bit budget because a function result was "bounded by construction"
and therefore not checked.

**No short-circuit evaluation.** `0 and (2^1000000)` evaluates its right
operand and is refused by the size budget rather than answering `0`. This is a
property of the evaluation model, not of the `and` arm: a stack machine has
both operands on the stack before it sees the operator that combines them.
Short-circuiting would need jumps in the compiled form — a conditional in the
RPN, and an evaluator that can skip over a span of it — which is a change to
what a compiled `Expression` *is*, not an operator. Documented in the README
so that nobody discovers it by timing out.

**`Session` has no public way to read a variable back.** `Session::set` and
`setf` write; `Session::lookup` is `pub(crate)`, so a caller holding a session
that has just evaluated `x = 0 or 1` can only learn what `x` is by compiling
and evaluating the expression `"x"`. That is what
`test_assignment_still_binds_more_weakly_than_everything` does, and writing it
is how this was noticed. The gap is additive to close — a `Session::get(&self,
name: &str) -> Option<Number>` alongside `set` — and it is left open here only
because adding a public method is a design decision for the release rather
than part of adding operators.

**`statrs` is 26 of the 41 crates a slim library build compiles**, for one
import: `Normal`, used by `pdf` and `cdf`. It brings `nalgebra`,
`matrixmultiply` and `rand`. It is a larger dependency than the entire CLI
stack, which Stage 3 was able to put behind a feature precisely because it is
optional to the library — `statrs` is not. Replacing it means hand-writing an
erf approximation, which would move numbers the suite pins and the README's
Black–Scholes example quotes, so it is a correctness change rather than a
dependency one and wants its own design.

**There is no coverage measurement.** The workflow uploaded to Codecov for
years with no step generating a report, so whatever that badge showed was not
measured. Stage 3 removed the upload rather than wiring it up. `cargo llvm-cov`
is the obvious way back if coverage is wanted, and wiring it is a small piece
of work that nobody should mistake for having been done.

**The MSRV is declared in two places** — `rust-version` in `Cargo.toml` and the
`msrv` job's toolchain line in `.github/workflows/rust.yml` — and nothing
checks that they agree. Raising one without the other leaves CI verifying a
claim the manifest is not making, or the reverse.

**The floor is 1.88 because of a transitive dependency, not because of yarer.**
The library alone builds on 1.86, where `Vec::pop_if` in the shunting yard is
the newest thing it uses. `rustyline 16` requires `home ^0.5.12`, and `home`
0.5.12 requires 1.88. `rust-version` is a single per-package value and takes
the higher, so a library user on 1.86 or 1.87 is refused by cargo even though
their build would work. Nothing to do about it while rustyline is a default
dependency; worth knowing before anyone tries to lower the floor.

**`src/token.rs` is over 1100 lines** and holds `Number`, `Operator`, `Token`,
`MathFunction` and every conversion between them. `Number` and its conversions
are the separable part. Stage 3 added about 35 lines to it and edited nine doc
comments, neither of which is a reason to move four types between files during
a release.

**Script mode has no comment syntax**, which is the first thing a piped file of
expressions wants. The README documented a `//` comment for three releases that
never existed — `9801/(2206*sqrt(2)) // approx of PI` is a parse error — and
Stage 3 deleted the claim rather than implementing it, because adding tokeniser
syntax is a language change. If script mode gets used, this is the first thing
it will ask for.

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
