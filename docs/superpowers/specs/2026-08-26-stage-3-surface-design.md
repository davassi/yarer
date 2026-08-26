# Design — Stage 3: Surface

Date: 2026-08-26
Status: approved, not yet implemented
Target release: 0.4.0.

## Context

Stages 1 and 2 shipped as 0.3.0 on 2026-08-26, together with the comparison and
logical operators. Both were about what yarer *computes* and what it *says when
it cannot*. Stage 3 is about everything else a crate presents to the world: how
it is invoked, what it makes people download, which compiler it promises to
work with, and what is checked before a change lands.

The stage was scoped as five items — script mode, clippy in CI, MSRV, a
CHANGELOG, fuzzing. Measuring the repository before designing turned up four
facts that reshaped it, and they are recorded here because they are the
justification for the parts of this design that were not on that list.

**Thirty-five crates exist in a library user's build for reasons that have
nothing to do with the library.** `clap`, `rustyline`, `dirs` and `env_logger`
serve the REPL binary and are unconditional dependencies of the crate, so
someone who writes `yarer = "0.3"` to add `2+2` to their program compiles an
argument parser, a line editor, and — through `env_logger` — `jiff`, a complete
datetime library. `bigdecimal` and `lazy_static` are declared and referenced
nowhere at all.

```
today                                        76 crates
--no-default-features, after this stage      41 crates   (35 fewer)
   of which statrs alone                     26
   yarer's own core (num family, regex,
   thiserror, log) and yarer itself         ~15
```

Counted by distinct crate name in `cargo tree --edges normal`, yarer included.

**All twenty `multiple_crate_versions` warnings are `windows-*` duplicates**
— `windows-sys` 0.59 against 0.60, and nine `windows_*` target crates at 0.52.6
against 0.53 — reached transitively through those same four dependencies.
Yarer cannot resolve them, so a `clippy::cargo` gate would have been
permanently red on something outside the repository.

**The README's CLI transcript contains two claims that are false.**
`9801/(2206*sqrt(2)) // approx of PI` is documented as printing
`3.1415927300133055`; it is a parse error, because yarer has no comment syntax.
`x=10` is documented as printing nothing; it prints `10`. Neither is a code
defect. They are documentation that lies about the tool, and they survived
because **the CLI has never been executable from a test**.

**Narrowing to `f64` loses small values silently, and it happens in three
places.** `BigRational::to_f64` signals neither of its two failures: it answers
`Some(inf)` when the value is too large and `Some(0.0)` when it is too small, so
both losses arrive looking like successes. The Stage 2 fix wave closed this in
`Display for Number`. The other two sites still have it.

`f64::try_from(1/(10^400))` answers `Ok(0.0)` while `f64::try_from(10^400)`
correctly answers `Err(OutOfRange)`.

Worse, `functions::number_to_f64` narrows every function's *operand* before
applying it, so the operand becomes `0.0` first:

| expression | 0.3.0 answers | true value |
|---|---|---|
| `log(1/(10^400))` | error, "function result is not a real number" | `-400` |
| `ln(1/(10^400))` | error, same | `-921.034…` |
| `sqrt(1/(10^400))` | `0` | `1e-200`, comfortably representable |
| `(1/(10^400))^0.5` | `0` | `1e-200` |
| `sin`, `cos`, `exp`, `atan`, `cdf` | correct | correct |

The split is not arbitrary. A function that shrinks toward its input — `sin x ≈
x` — is unharmed by a zeroed operand; one that expands small values is wrecked
by it. `log(1/(10^400))` is exactly `-400`, and yarer refuses it as not a real
number.

These are known wrong answers shipping in 0.3.0.

## What this is not

- **Not a language change.** No comment syntax, however much piping a file of
  expressions argues for one. The README's fake `//` comment is deleted rather
  than implemented. Adding tokeniser syntax is a stage of its own.
- **Not the `statrs` question.** `statrs` is 26 of the 41 crates a slim library
  build still compiles — `nalgebra`, `matrixmultiply`, `rand` — and yarer
  imports exactly one thing from it, `Normal`, for `pdf` and `cdf`. Replacing it
  means hand-writing an erf approximation, which moves numbers the suite pins
  and the README's Black–Scholes example quotes. That is a correctness change.
  It gets a register entry with these measurements, not a task.
- **Not `Session::get`.** The missing public way to read a variable back is an
  addition to the public API. It stays in the register.
- **Not making `limits` private.** That is a break, and this release does not
  need one.
- **Not coverage tooling.** The existing Codecov step is removed rather than
  wired up; see component F.
- **No REPL removal.** The interactive REPL stays the default and is not being
  touched except where the banner moves. Script mode is strictly an addition.

## Decisions

Five questions were put and answered before this design was written. They are
recorded with their reasoning because the plan must not relitigate them.

| question | decision |
|---|---|
| script mode's shell contract | values to stdout, rendered errors to stderr, exit 0/1 |
| several expressions, one fails | stop at the first failure; nothing after it runs |
| dependency slimming | feature-gate the CLI, `default = ["cli"]` — non-breaking |
| MSRV | declare the real floor and verify it in CI; do not contort code to lower it |
| fuzzing | `cargo-fuzz` on a schedule, with the curated corpus replayed on stable |
| clippy | clear every fixable warning, then `-D warnings` |

## Components

Order is forced by dependency, not preference. **A** (the manifest) licenses
part of **D**: the MSRV that **B** declares is what allows `std::sync::LazyLock`,
which is simultaneously a clippy fix. **D** must be silent before **F** can gate
on it. **C** adds code that **D** will lint, so it lands before the gate rather
than after. **E** and **G** are independent; **F**'s changelog goes last because
it describes everything above it.

### A. The manifest

`bigdecimal` and `lazy_static` are removed. `once_cell` is replaced by
`std::sync::LazyLock` at `parser.rs`'s `EXPRESSION_REGEX`, which removes a third
dependency and silences the `non_std_lazy_statics` warning in the same edit.

```toml
[dependencies]
log = "0.4"
num = "0.4.1"
num-bigint = "0.4.4"
num-rational = "0.4"
num-traits = "0.2.18"
regex = "1.9.3"
statrs = "0.18.0"
thiserror = "2.0.12"

clap       = { version = "4.4.2", features = ["derive"], optional = true }
rustyline  = { version = "16.0.0", optional = true }
dirs       = { version = "6.0.0",  optional = true }
env_logger = { version = "0.11.2", optional = true }

[features]
default = ["cli"]
cli = ["dep:clap", "dep:rustyline", "dep:dirs", "dep:env_logger"]

[[bin]]
name = "yarer"
path = "src/bin/main.rs"
required-features = ["cli"]
```

`log` stays unconditional: the library's `debug!` calls need the facade, and it
pulls nothing.

`default = ["cli"]` is the choice that makes this non-breaking. `cargo install
yarer` behaves exactly as it does today, and every existing library user
continues to build unchanged; the slim build is opted into with
`default-features = false` and documented in the README. Defaulting the feature
*off* would have given every library user the benefit automatically at the cost
of breaking the documented install path, which is not a trade worth making for
a crate whose binary is half its reason to exist.

**The split must be verified, not asserted.** Nothing stops a later change from
adding `use clap::…` to library code; only a build without the feature catches
it. Component F carries that job.

**`fuzz/` joins `docs/superpowers/**` in the package `exclude` list.**

### B. MSRV

`rust-version` is declared in `Cargo.toml` and a CI job pinned to exactly that
toolchain proves the claim. An unverified `rust-version` is a comment.

**The floor is found, not assumed.** `Vec::pop_if` at `shunting.rs:77`
stabilised in Rust 1.86 and is the newest thing yarer's own code uses, so 1.86
is the expected answer — but the dependency floors (`clap`, `rustyline`, and
`jiff` through `env_logger`) may sit above it. The plan bisects with
`rustup toolchain install` and records what it finds.

**The floor differs by feature set**, and `rust-version` is a single
per-package value. The declared value is the floor for the *default* feature
set, which is the higher of the two because the CLI dependencies are the
demanding ones. The MSRV job builds both `--all-features` and
`--no-default-features` at that toolchain, so a slim build that would work on
something older is not accidentally claimed to need more.

Declaring 1.86 takes nothing away from anyone: 0.3.0's code already requires it
through `pop_if`. Declaring it converts a confusing compile error into cargo's
"requires rustc 1.86" message and lets MSRV-aware resolution pick an older
yarer instead.

### C. Script mode

Three ways in, chosen in this order:

```
yarer -e "1+2"                → evaluate, print, exit    (-e wins over stdin)
printf '1+1\n2+2\n' | yarer   → evaluate each line, exit (stdin is not a tty)
yarer                         → REPL, exactly as today
```

The terminal test is `std::io::stdin().is_terminal()` from `std::io::IsTerminal`,
stable since 1.70 and well under the floor. No new dependency.

**The contract:**

- Values to **stdout**, one line per expression, assignments included:
  `yarer -e "x=10"` prints `10`, because that is what the expression evaluates
  to. (The REPL already behaves this way; only the README claimed otherwise.)
- Errors to **stderr**, rendered by `Error::render` — the same message, source
  line and caret the REPL prints.
- **Exit 0** on success, **1** on the first failure. Nothing after the failing
  expression runs. A script that half-succeeded is the hardest outcome to
  debug, and stopping makes it impossible.
- **One `Session`** shared by every `-e` flag and every stream line, so `x=2` on
  one line is visible on the next, as in GNU bc.
- **Blank lines skipped**; `quit` honoured in the stream as in the REPL.
- **The banner becomes REPL-only.** Printing `Yarer v.0.4.0…` on stdout would
  corrupt `x=$(yarer -e "2^10")`. `--quiet` keeps its meaning for the REPL.
- `main` returns `std::process::ExitCode` rather than `rustyline::Result<()>`.

**One evaluation path, three callers.** The three modes differ only in where
lines come from and where output goes. A single `evaluate` plus a single
renderer is what stops them drifting in how they report the same failure.
`src/bin/main.rs` grows from ~112 to roughly 200 lines as `run_expressions`,
`run_stream` and `run_repl` over that shared core.

### D. `eval_with`, and clippy to zero

Twenty warning sites, which is the entire inventory:

```
src/token.rs:9,31,33,572,657     doc_markdown
src/token.rs:444,445             doc_lazy_continuation
src/token.rs:662,669             needless_borrow
src/token.rs:576                 needless_pass_by_value
src/token.rs:878                 useless_vec
src/expression.rs:395,407        needless_pass_by_value
src/session.rs:108               manual_midpoint
src/bin/main.rs:69               unwrap_or_default
src/parser.rs:14                 non_std_lazy_statics     → component A
src/lib.rs:1                     multiple_crate_versions  → the one allow
src/expression.rs:92             too_many_lines  eval_with        218
src/validate.rs:67               too_many_lines  validate         163
src/functions.rs:23              too_many_lines  functions::eval  110
```

`multiple_crate_versions` is allowed at crate level with the `windows-*` cause
named in a comment and an instruction to re-check when those bump.

**Suppressions use `#[expect(lint, reason = "…")]`, never `#[allow]`.**
`#[expect]` warns when the suppression stops being needed, so a stale one
reports itself instead of hiding. This also discharges a standing register
entry: the two `#[allow]`s on `predicted_factorial_bits` become `#[expect]`.

**`eval_with` is split three ways.** The register's proposed `Stacks`
extraction alone does not reach the threshold, and the measurement is why the
split is what it is:

```
eval_with total                    218 code lines
  the Token::Operator arm          158
    of which `match op`            140
  everything else (walk, dispatch)  60

Stacks::push_checked alone                       ~173   still over
+ extract apply_operator      eval_with ~63,     ~113   still over
+ split the ten truth arms    apply_operator ~65, apply_truth ~40
```

The result is three functions with one responsibility each:

- `eval_with` — walk the RPN, dispatch by token kind.
- `apply_operator` — apply one operator.
- `apply_truth_operator` — the ten operators that answer `1` or `0` and share
  a tail.

with `Stacks { values, vars }` underneath all three. That struct is worth more
than the lines it saves: the value stack and the variable-name stack must stay
in lockstep, which today is maintained by hand at every push and pop, and
giving them one owner makes it a property of the type.

`validate` (163) and `functions::eval` (110) take `#[expect]` with written
reasons. The register already argues that `validate`'s length is earned by the
five distinct positioned diagnoses it replaced five identical ones with;
restructuring two more working functions to chase a threshold is not what this
stage is for.

### E. Fuzzing

One target, over the whole front-to-back path, because that is where the
register's standing "no reachable panic on any evaluation path" claim lives:

```rust
fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return };
    if s.len() > 4096 { return }
    let session = Session::init();
    let limits = Limits::default().with_max_value_bits(4096);
    if let Ok(expr) = Expression::compile(s) {
        let _ = expr.eval_with(&session, limits);
    }
});
```

The tight budget keeps the fuzzer exploring the parser rather than doing bignum
arithmetic, and the predictive guards are what make that work: `2^100000000` is
refused in about 15µs and `999999999!` in about 100µs, without either being
computed.

**Two corpus directories, and the reason is a packaging trap.** `fuzz/` is
excluded from the published tarball, so a test reading `fuzz/corpus/` would
fail for anyone running `cargo test` against a packaged or vendored copy.

```
fuzz/corpus/compile_eval/   the fuzzer's working corpus — gitignored, grows unboundedly
tests/fuzz_regressions/     curated: seeds, and every input that ever crashed.
                            committed, small, shipped inside the package
```

`tests/fuzz_regressions.rs` walks the second directory on stable and asserts
nothing panics, so it runs on every push with no nightly involved. An input the
fuzzer crashes on is copied there, which makes it a permanent regression test —
a crash found once cannot come back. The fuzzer seeds from it too.

### F. CI, and the CHANGELOG

The current workflow is `checkout@v3`, `cargo build`, `cargo test`, and a
Codecov upload. **Nothing in it generates coverage** — there is no `tarpaulin`,
`llvm-cov` or `grcov` step — so `codecov-action` has no file to find. Whatever
that badge reports is not measured. The step is removed rather than wired up:
adding coverage tooling is a stage of its own, and a number nobody generates is
worse than no number. It becomes a register entry. `checkout` goes to v4.

| job | runs |
|---|---|
| `test` | stable — build, test, doc tests |
| `fmt` | `cargo fmt --check` — not checked at all today |
| `clippy` | `--all-targets --all-features -- -D warnings` |
| `slim` | `cargo build` and `cargo test --no-default-features` — proves component A's split |
| `msrv` | pinned to the declared floor, both feature sets |
| `fuzz` | weekly schedule, nightly, `cargo fuzz run -max_total_time=600` |

`CHANGELOG.md` in Keep a Changelog format, holding the history back to 0.1.x,
written from the README's News sections. The 0.3.0 migration table moves there,
which is where someone upgrading looks; the README's News section shrinks to
what is new in 0.4.0 plus a link.

### G. One narrowing, and a symmetric refusal

The three narrowing sites become one function. It is the only place in the crate
that turns a [`Number`] into an `f64`, and it reports which way the value
escaped:

```rust
/// Which end of `f64`'s range a value fell off.
pub(crate) enum Narrowing {
    TooLarge,
    TooSmall,
}

/// The one place this crate narrows a `Number` to `f64`.
///
/// `to_f64` signals neither failure — `Some(inf)` on overflow, `Some(0.0)` on
/// underflow — so both arrive looking like successes and every caller that
/// forgets to check inherits a wrong answer. Three of them did.
pub(crate) fn narrow_to_f64(value: &Number) -> Result<f64, Narrowing>;
```

Zero narrows to `0.0` successfully; only a `0.0` that came from a *non-zero*
value is `TooSmall`.

Its three callers keep their own answers to a failure, because they want
different ones:

| caller | `TooLarge` | `TooSmall` |
|---|---|---|
| `Display for Number` | print the ratio | print the ratio |
| `TryFrom<Number> for f64` | `ConversionError::OutOfRange` | `ConversionError::OutOfRange` |
| `functions::number_to_f64` | `EvalError::OperandTooLargeForFloat` | `EvalError::OperandTooSmallForFloat` |

**`EvalError::OperandTooSmallForFloat` is new**, and it is the mirror of the
`OperandTooLargeForFloat` Stage 2 added for the overflow side. The rule becomes
symmetric and there is one sentence to explain it: *an operand that cannot be
represented as a float is refused, and the error says which end it fell off.*

**This withdraws correct answers, and that is declared.** `sin(1/(10^400))`
answered `0`, which is right, and will now be refused — exactly as Stage 2
knowingly withdrew `atan(10^400) = π/2` when it made the overflow side refuse.
Applying the rule per-function instead would preserve those answers at the cost
of a judgement, for each of eighteen built-ins, about whether a zeroed operand
is acceptable — a table that is silent when it is wrong. One rule is worth more
than six preserved answers at the extreme edge of the value space.

`number_to_f64` also loses its `on_error` parameter. Twelve call sites pass
`EvalError::OperandTooLargeForFloat { span: None }` and nothing else ever has;
choosing the variant is now the narrowing's job, which is what makes the
underflow case reachable at all.

## Testing

- **The 143 value assertions stay byte-identical.** Component D restructures the
  evaluation loop; they are the proof it changed no meaning. Verify with a
  superset check against the merge base, never with a digest of the whole set —
  a digest changes when a test is legitimately added.

  ```bash
  git show master:tests/integration_tests.rs \
    | grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' | sort > baseline.txt
  grep -oP 'resolve(_natural|_decimal)?!\([^;]*\);' tests/integration_tests.rs \
    | sort | comm -23 baseline.txt -      # must print nothing
  ```

- **Script mode is tested by spawning the real binary** through
  `env!("CARGO_BIN_EXE_yarer")`, which cargo sets for test targets — no
  `assert_cmd` dependency. Exit codes, stream ordering, stdout/stderr
  separation and stop-at-first-error are all observable only this way. The file
  carries `#![cfg(feature = "cli")]` so the slim job stays green.

- **The README's CLI transcript becomes a test.** Its inputs are fed to the
  binary and its documented outputs asserted. This is the check that would have
  caught both false claims, and it is the reason they went unnoticed for three
  releases.

- **The narrowing is pinned at every caller and in both directions**:
  `f64::try_from(1/(10^400))` errors and `f64::try_from(1/3)` still converts;
  `log(1/(10^400))` and `sqrt(1/(10^400))` raise `OperandTooSmallForFloat` with
  a span rather than answering `0`; `10^400` still raises
  `OperandTooLargeForFloat`; and a genuine zero still narrows to `0.0`, which is
  the case that separates "underflowed" from "is zero".

- **`cargo test --no-default-features`** passes, proving no library test reaches
  a CLI dependency.

## Definition of done

- `cargo test` green; `cargo test --no-default-features` green.
- `cargo clippy --all-targets --all-features -- -D warnings` silent.
- `cargo fmt --check` silent.
- `eval_with`, `apply_operator` and `apply_truth_operator` each under 100 lines,
  with no `too_many_lines` suppression among them.
- The 143 value assertions present and unmodified.
- `cargo build --no-default-features` compiles with `clap`, `rustyline`, `dirs`
  and `env_logger` absent from the tree.
- `rust-version` declared, and the pinned CI job green at exactly that version.
- `yarer -e`, piped stdin and the REPL all covered by tests that spawn the
  binary.
- `narrow_to_f64` is the only place in the crate that calls `to_f64`, verified
  by `grep -rn 'to_f64' src/` returning one site.
- `log(1/(10^400))`, `ln(1/(10^400))` and `sqrt(1/(10^400))` raise
  `OperandTooSmallForFloat` with a span; the withdrawal of `sin(1/(10^400))`
  is in the CHANGELOG.
- `CHANGELOG.md` present and covering 0.1.x through 0.4.0.
- `docs/tech-debt.md` updated: `statrs`, coverage tooling, comment syntax, and
  whatever this work leaves behind.
- `Cargo.toml` at `0.4.0`.
