use std::process::ExitCode;

use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use yarer::Error;
use yarer::Expression;
use yarer::Number;
use yarer::Session;

use log::debug;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static HISTORY_FILE: &str = ".yarer_history";

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Yarer (Yet Another Rust Expression Resolver)\n",
    long_about = "Yarer (Yet Another Rust Expression Resolver)\n\
                  Copyright (c) 2024 Davassi <gianluigi.davassi@gmail.com>\n\
                  License MIT OR Apache-2.0",
    help_template = "{before-help}{name} {version}\n{author-with-newline}{about-with-newline}{usage-heading} {usage}\n\n{all-args}{after-help}"
)]
struct Cli {
    /// Evaluate an expression and exit. May be given more than once; all the
    /// expressions share a single session, so a variable set by one is visible
    /// to the next.
    #[arg(short = 'e', long = "expr", value_name = "EXPR")]
    expr: Vec<String>,

    #[arg(short, long)]
    quiet: bool,
}

/// Compiles and evaluates one line against `session`.
///
/// The one step every input mode shares. `compile` and `eval` fail with
/// different types; [`Error`] is the union the library provides for exactly
/// this caller.
fn evaluate(session: &Session, line: &str) -> Result<Number, Error> {
    Expression::compile(line)
        .map_err(Error::from)
        .and_then(|expr| expr.eval(session).map_err(Error::from))
}

/// Evaluates `line` and reports it: the value on stdout, or the rendered error
/// — message, source line and caret — on stderr. Answers whether it succeeded.
///
/// Which stream each goes to is the whole of the shell contract. A value on
/// stdout can be captured; an error there would be captured too, and
/// `x=$(yarer -e "1/0")` would quietly hold a sentence about division.
fn report(session: &Session, line: &str) -> bool {
    match evaluate(session, line) {
        Ok(value) => {
            println!("{value}");
            true
        }
        Err(err) => {
            eprintln!("{}", err.render(line));
            false
        }
    }
}

/// `-e` mode: each expression in order against one session, stopping at the
/// first failure.
fn run_expressions(session: &Session, expressions: &[String]) -> ExitCode {
    for line in expressions {
        if !report(session, line) {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// The interactive REPL: unchanged behaviour, and now one of several modes
/// rather than the only one.
///
/// It owns the banner, because the banner is for a human at a prompt. In the
/// non-interactive modes it would land on stdout and corrupt a captured value.
fn run_repl(session: &Session, quiet: bool) -> ExitCode {
    if !quiet {
        println!("Yarer v.{VERSION} - Yet Another Rust Expression Resolver.");
        println!("License MIT OR Apache-2.0");
    }

    let mut rl = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(err) => {
            eprintln!("Could not start the interactive editor: {err}");
            return ExitCode::FAILURE;
        }
    };

    let local_history = dirs::config_dir().unwrap_or_default().join(HISTORY_FILE);
    let local_history = local_history.as_os_str().to_str().unwrap_or(HISTORY_FILE);
    debug!("Local history file: '{local_history}'");
    let _ = rl.load_history(local_history);

    loop {
        match rl.readline("> ") {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                if line.trim().eq_ignore_ascii_case("quit") {
                    break;
                }
                let _ = rl.add_history_entry(line.as_str());
                report(session, &line);
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                println!("quit");
                break;
            }
            Err(err) => {
                eprintln!("Error: {err:?}");
                break;
            }
        }
    }

    let _ = rl.save_history(local_history);
    ExitCode::SUCCESS
}

/**
Yarer - A resolver for mathematical expressions that uses Reverse Polish Notation internally.

The internal flow is conceptually straightforward:

 1 Yarer parses and converts a [str] into a vec of borrowed &[str]
 2 Then it maps a vec of &[str] into a vec of tokens
 3 Then it converts the infix expression to postfix
 4 Finally it resolves the expression.

 Point 1, 2 and 3 are executed by Expression::compile, 4 by Expression::eval

 # Usage

 Example
 ```ignore
     let exp = "4 + 4 * 2 / ( 1 - 5 )";
     let session = Session::init();
     let expr = Expression::compile(exp).unwrap();

     let result: Number = expr.eval(&session).unwrap();
     println!("The result of {} is {}", exp, result);
 ```
*/
fn main() -> ExitCode {
    let cli = Cli::parse();
    env_logger::init();

    let session = Session::init();

    // A caller who passed an expression asked for that expression.
    if !cli.expr.is_empty() {
        return run_expressions(&session, &cli.expr);
    }

    run_repl(&session, cli.quiet)
}
