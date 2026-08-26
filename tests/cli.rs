//! The command-line binary, exercised by running it.
//!
//! Exit status, which stream each kind of output goes to, and what happens
//! after a failure are all observable only from outside the process, and none
//! of them was covered before this file existed — which is how the README came
//! to document two things the binary does not do.

#![cfg(feature = "cli")]

use std::io::Write;
use std::process::{Command, Stdio};

/// The binary this test crate was built alongside. Cargo sets
/// `CARGO_BIN_EXE_<name>` for test targets, so there is no need for a helper
/// crate to find it.
fn yarer() -> Command {
    Command::new(env!("CARGO_BIN_EXE_yarer"))
}

#[test]
fn test_e_prints_the_value_to_stdout_and_exits_zero() {
    let out = yarer().args(["-e", "2^10"]).output().expect("runs");
    assert!(out.status.success(), "status was {:?}", out.status);
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "1024");
    assert!(
        out.stderr.is_empty(),
        "stderr should be silent on success: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The whole point of the contract: `x=$(yarer -e ...)` must be able to tell
/// success from failure, and must not capture an error message as if it were a
/// value.
#[test]
fn test_a_failure_goes_to_stderr_and_exits_one() {
    let out = yarer().args(["-e", "1/0"]).output().expect("runs");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        out.stdout.is_empty(),
        "stdout must stay clean: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("division by zero") && stderr.contains('^'),
        "the rendered error with its caret should reach stderr: {stderr}"
    );
}

/// A parse failure is reported the same way an evaluation failure is: one exit
/// code, because a shell script cannot act differently on the difference.
#[test]
fn test_a_parse_failure_uses_the_same_contract() {
    let out = yarer().args(["-e", "1+"]).output().expect("runs");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("expected a value"));
}

#[test]
fn test_several_expressions_share_one_session() {
    let out = yarer()
        .args(["-e", "x=2", "-e", "x*3"])
        .output()
        .expect("runs");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["2", "6"],
        "the assignment must be visible to the next expression"
    );
}

/// A run that half-succeeded is the hardest outcome to debug, so it is made
/// impossible: nothing after the failure runs.
#[test]
fn test_evaluation_stops_at_the_first_failure() {
    let out = yarer()
        .args(["-e", "1+1", "-e", "1/0", "-e", "2+2"])
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["2"],
        "2+2 must not have run"
    );
}

/// The banner is for a human at a prompt. On stdout it would end up inside
/// `x=$(yarer -e "2^10")`.
#[test]
fn test_the_banner_stays_out_of_script_mode() {
    let out = yarer().args(["-e", "1+1"]).output().expect("runs");
    // Asserted before the banner check, and not merely for tidiness: without
    // it this test passes while `-e` is still an unrecognised argument,
    // because clap's usage error leaves stdout empty and an empty stdout
    // contains no banner either.
    assert!(out.status.success(), "status was {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Yarer v."),
        "banner leaked into stdout: {stdout}"
    );
    assert!(!String::from_utf8_lossy(&out.stderr).contains("Yarer v."));
}

/// An assignment evaluates to the value assigned, and script mode prints what
/// the expression evaluates to. The README claimed otherwise for three
/// releases; Task 6 corrects it.
#[test]
fn test_an_assignment_prints_its_value() {
    let out = yarer().args(["-e", "x=10"]).output().expect("runs");
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "10");
}

/// Runs the binary with `input` on stdin and no arguments.
fn piped(input: &str) -> std::process::Output {
    let mut child = yarer()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawns");
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(input.as_bytes())
        .expect("writes");
    child.wait_with_output().expect("runs")
}

#[test]
fn test_a_pipe_evaluates_every_line() {
    let out = piped("1+1\n2+2\n3+3\n");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["2", "4", "6"]
    );
}

/// One session for the whole stream, as in GNU bc, which is what makes a piped
/// file of expressions a script rather than a list of unrelated sums.
#[test]
fn test_a_pipe_shares_one_session_across_lines() {
    let out = piped("x=2\ny=3\nx*y\n");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["2", "3", "6"]
    );
}

#[test]
fn test_a_pipe_stops_at_the_first_failure() {
    let out = piped("1+1\n1/0\n2+2\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["2"],
        "2+2 must not have run"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("division by zero"));
}

#[test]
fn test_a_pipe_skips_blank_lines_and_honours_quit() {
    let out = piped("1+1\n\n\n2+2\nquit\n3+3\n");
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["2", "4"],
        "blank lines contribute nothing and quit ends the stream"
    );
}

#[test]
fn test_an_empty_pipe_succeeds_silently() {
    let out = piped("");
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn test_the_banner_stays_out_of_a_pipe_too() {
    let out = piped("1+1\n");
    assert!(out.status.success());
    assert!(!String::from_utf8_lossy(&out.stdout).contains("Yarer v."));
}

/// `-e` wins. A caller who passed an expression asked for that expression, and
/// whatever happens to be on stdin is not an instruction.
#[test]
fn test_an_expression_argument_wins_over_a_pipe() {
    let mut child = yarer()
        .args(["-e", "7*6"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawns");
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(b"1+1\n")
        .expect("writes");
    let out = child.wait_with_output().expect("runs");
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "42");
}
