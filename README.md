
RpnResolver is a Reverse Polish Notation resolver written in .

Example of usage of the library: 
      
      let exp = "((10 + 5) – 3 * (9 / 3)) + 2";
      let resolver = RpnResolver::parse(exp);
      println!("The result of {} is {}", exp, resolver.resolve());