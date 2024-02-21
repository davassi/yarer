use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

use yarer::rpn_resolver::*;
use yarer::session::*;

static VERSION: &str = env!("CARGO_PKG_VERSION");
static HISTORY_FILE: &str = ".yarer_history";

#[derive(Parser)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
struct Cli {
    #[arg(short, long)]
    quiet: bool,
}

/**
Yarer - A resolver for mathematical expressions that uses Reverse Polish Notation internally.

The internal flow is conceptually straightforward:

 1 Yarer parses and converts a [str] into a vec of borrowed &[str]
 2 Then it maps a vec of &[str] into a vec of tokens
 3 Then it converts the infix expression to postfix 
 4 Finally it resolves the expression.

 Point 1 and 2 are executed by the Parser, 3 and 4 by the RpnResolver

 # Usage

 Example
 ```
     let exp = "4 + 4 * 2 / ( 1 - 5 )";
     let mut session = Session::init();
     let mut resolver: RpnResolver = session.process(&exp);

     let result: token::Number = resolver.resolve().unwrap();
     println!("The result of {} is {}", exp, result);
 ```
*/
fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    if !cli.quiet {
        println!(
            "Yarer v.{} - Yet Another Rust Expression Resolver.",
            VERSION
        );
        println!("License MIT OR Apache-2.0");
    }

    let mut rl = DefaultEditor::new()?;
    let _ = rl.load_history(HISTORY_FILE);

    let session = Session::init();
    loop {
        let readline = rl.readline("> ");

        match readline {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                if line.trim().to_lowercase().eq("quit") {
                    break;
                }

                let _ = rl.add_history_entry(line.as_str());

                let mut resolver: RpnResolver = session.process(&line);

                match resolver.resolve() {
                    Ok(value) => println!("{}", value),
                    Err(e) => println!("Error: {}", e),
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("quit");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    let _ = rl.save_history(HISTORY_FILE);
    Ok(())
}
