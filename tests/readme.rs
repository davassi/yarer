//! The README's CLI transcripts, executed.
//!
//! Two of them were false for three releases. `9801/(2206*sqrt(2)) // approx of
//! PI` was documented as printing an approximation of pi; it is a parse error,
//! because yarer has no comment syntax. `x=10` was documented as printing
//! nothing; it prints `10`. Neither is a code defect — they are documentation
//! that lies about the tool, and nothing caught them because the CLI could not
//! be run from a test until now.
//!
//! Each fenced block containing a `> ` prompt is a transcript. Within a block,
//! a `> expression` line is an input and the lines after it, up to the next
//! prompt, are its expected output. Each block gets its own session, because
//! that is what a reader starting at the top of one would have.

#![cfg(feature = "cli")]

use std::io::Write;
use std::process::{Command, Stdio};

/// An expected line ending in an ellipsis asserts a prefix instead of an exact
/// match, so that the README can elide the hundred-digit tail of `78!` without
/// this test either failing or having to carry the whole number.
const ELISION: &str = "...";

/// Splits the README into transcripts: one entry per fenced block that has a
/// prompt in it, each a list of (expression, expected output lines).
fn transcripts(readme: &str) -> Vec<Vec<(String, Vec<String>)>> {
    let mut blocks: Vec<Vec<String>> = Vec::new();
    let mut current: Option<Vec<String>> = None;

    for line in readme.lines() {
        if line.trim_start().starts_with("```") {
            if let Some(block) = current.take() {
                if block.iter().any(|l| l.trim_start().starts_with("> ")) {
                    blocks.push(block);
                }
            } else {
                current = Some(Vec::new());
            }
            continue;
        }
        if let Some(block) = current.as_mut() {
            block.push(line.to_string());
        }
    }

    blocks
        .into_iter()
        .map(|block| {
            let mut pairs: Vec<(String, Vec<String>)> = Vec::new();
            for raw in block {
                let line = raw.trim();
                if line.is_empty()
                    || line.starts_with('$')
                    || line.starts_with("Yarer v.")
                    || line.starts_with("License")
                {
                    continue;
                }
                if let Some(expression) = line.strip_prefix("> ") {
                    pairs.push((expression.trim().to_string(), Vec::new()));
                } else if let Some((_, expected)) = pairs.last_mut() {
                    expected.push(line.to_string());
                }
            }
            pairs
        })
        .filter(|pairs| !pairs.is_empty())
        .collect()
}

#[test]
fn test_the_readme_cli_transcripts_are_true() {
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))
        .expect("the README is next to Cargo.toml");
    let blocks = transcripts(&readme);
    assert!(
        blocks.len() >= 3,
        "expected several transcript blocks, found {}",
        blocks.len()
    );

    for (n, pairs) in blocks.iter().enumerate() {
        let input: String = pairs
            .iter()
            .map(|(expression, _)| format!("{expression}\n"))
            .collect();

        let mut child = Command::new(env!("CARGO_BIN_EXE_yarer"))
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
        let out = child.wait_with_output().expect("runs");

        assert!(
            out.status.success(),
            "transcript {n} failed:\n{}\ninput was:\n{input}",
            String::from_utf8_lossy(&out.stderr)
        );

        let stdout = String::from_utf8_lossy(&out.stdout);
        let mut produced = stdout.lines();
        for (expression, expected) in pairs {
            for want in expected {
                let got = produced.next().unwrap_or_else(|| {
                    panic!("transcript {n}: '{expression}' produced no output, expected '{want}'")
                });
                if let Some(prefix) = want.strip_suffix(ELISION) {
                    let prefix = prefix.trim_end_matches('.');
                    assert!(
                        got.starts_with(prefix),
                        "transcript {n}: '{expression}' gave '{got}', \
                         which does not start with the documented '{prefix}'"
                    );
                } else {
                    assert_eq!(
                        got, want,
                        "transcript {n}: '{expression}' gave '{got}', \
                         the README says '{want}'"
                    );
                }
            }
        }
    }
}
