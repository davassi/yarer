
pub mod parser;
pub mod rpn_resolver;
pub mod token;

use crate::rpn_resolver::*;

/*
  Reverse Polish Notification expression resolver

  1 parse and convert a string into a vec of &str TODO
  2 map a vec of &str into a vec of tokens OK
  3 reverse polish notification the vec
  4 resolve the expression!

  Example
      
      let exp = "1.2+2-1.0";
      let resolver = RpnResolver::parse(exp);
      println!("The result of {} is {}", exp, resolver.resolve());
 */
fn main() {

    /* 
     Input: A + B * C + D
     Output: ABC*+D+
     let exp = "4+5*5+6"; // 4 5 5 * + 6 +

     Input: ((A + B) – C * (D / E)) + F
     Output: AB+CDE/ *-F+   

    */
    let exp = "((10 + 5) – 3 * (9 / 3)) + 2"; // 10 5 + 3 9 3 / * - 2 +
    let resolver : RpnResolver = RpnResolver::parse(exp);
    println!("The result of {} is {:?}", exp, resolver.resolve());
    
}



