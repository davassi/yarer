
use std::io;
use clap::Parser;

pub mod parser;
pub mod rpn_resolver;
pub mod token;

use crate::rpn_resolver::*;

static VERSION : &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
struct Cli {
    #[arg(short,long)]
    quiet: bool,
}

///
///  Yarer - Reverse Polish Notification expression resolver
///
///  The internal flow is conceptually pretty simple: 
///
///  1 Yarer parses and converts a str into a vec of borrowed &str
///  2 map a vec of &str into a vec of tokens
///  3 reverse polish notification the vec
///  4 resolve the expression!
/// 
///  Point 1 and 2 are executed by the Parser, 3 and 4 by the RpnResolver
///
///  Example 
///  ```   
///      let exp = "4 + 4 * 2 / ( 1 - 5 )";
///      let mut resolver : RpnResolver = RpnResolver::parse(exp);
///      let result: token::Number = resolver.resolve().unwrap();
///      println!("The result of {} is {}", exp, result);
///  ```
///
fn main() {

    let cli = Cli::parse();

    if !cli.quiet {
        println!("Yarer v.{} - Yet Another Rust Rpn Expression Resolver.", VERSION);
        println!("License MIT OR Apache-2.0");
    }

    loop {
        let mut input : String = String::new();
        io::stdin().read_line(&mut input).expect("Input error.");

        if input.trim().is_empty() { continue; }
        if input.trim().to_lowercase().eq("quit") { break; }
        
        let mut resolver : RpnResolver = RpnResolver::parse(&input);
        let _ = resolver.resolve()
            .and_then(|res: token::Number| {println!("{}", res); Ok(res)})
            .or_else(|err| {println!("Error: {}", err); Err("Error")});
    }
}



