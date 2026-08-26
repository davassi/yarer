//! Every input the fuzzer ever crashed on, replayed on stable.
//!
//! `cargo fuzz` needs a nightly toolchain and runs on a schedule; this runs on
//! every push. A crash found once is copied into `tests/fuzz_regressions/` and
//! becomes a permanent test, so it cannot come back — and this corpus travels
//! inside the published package, unlike `fuzz/`, which is excluded from it. A
//! test reading `fuzz/corpus/` would fail for anyone running `cargo test`
//! against a packaged or vendored copy.
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
