#![no_main]

use libfuzzer_sys::fuzz_target;
use yarer::{Expression, Limits, Session};

// Compile and evaluate arbitrary text. The assertion is implicit and is the
// register's standing claim about this crate: no input reaches a panic.
//
// The budget is tight on purpose. Without it the fuzzer spends its time in
// bignum arithmetic on inputs like `2^999999` instead of exploring the parser;
// with it, the predictive guards refuse those in microseconds without computing
// anything — `2^100000000` in about 15µs and `999999999!` in about 100µs.
fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    if source.len() > 4096 {
        return;
    }
    let session = Session::init();
    let limits = Limits::default().with_max_value_bits(4096);
    if let Ok(expr) = Expression::compile(source) {
        let _ = expr.eval_with(&session, limits);
    }
});
