
use std::io;

pub mod parser;
pub mod rpn_resolver;
pub mod token;

use crate::rpn_resolver::*;

static VERSION : f32 = 0.11;

/*
  Yarer 
  Reverse Polish Notification expression resolver
 
The flow is pretty simple: 

  1 parse and convert a string into a vec of &str
  2 map a vec of &str into a vec of tokens
  3 reverse polish notification the vec
  4 resolve the expression!

  Example
      
      let exp = "((10 + 5) – 3 * (9 / 3)) + 2";
      let resolver = RpnResolver::parse(exp);
      println!("The result of {} is {}", exp, resolver.resolve());
 */
fn main() {

    println!("Yarer v.{} - Yet Another Rust Rpn Expression Resolver.", VERSION);
    println!("License MIT-Apache");

    /* 
     Input: A + B * C + D
     Output: ABC*+D+
     let exp = "4+5*5+6"; // 4 5 5 * + 6 +

     Input: ((A + B) – C * (D / E)) + F
     Output: AB+CDE/ *-F+   

    */
    //let exp = "((10 + 5) – 3 * (9 / 3)) + 2"; // 10 5 + 3 9 3 / * - 2 +
    let _exp = "4*5-6/2";
    let exp = "3 + 4 * 2 / ( 1 − 5 ) ^ 2 ^ 3";
    let mut resolver : RpnResolver = RpnResolver::parse(exp);
    let result: token::Number = resolver.resolve().unwrap();
    println!("The result of {} is {}", exp, result);

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



