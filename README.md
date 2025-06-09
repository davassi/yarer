
YARER - The math expression Evaluator
===========================

[<img alt="github" src="https://img.shields.io/badge/github-davassi/davassi?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/davassi/yarer)
[<img alt="build status" src="https://github.com/davassi/yarer/actions/workflows/rust.yml/badge.svg" height="20">](https://github.com/davassi/yarer/actions?query=branch%3Amaster)
[<img alt="crates.io" src="https://img.shields.io/crates/v/yarer.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/yarer)
[<img alt="docs.rs" src="https://img.shields.io/docsrs/yarer?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/yarer)
[![Downloads](https://img.shields.io/crates/d/yarer.svg)](https://crates.io/crates/yarer)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Project Status: Active – The project has reached a stable, usable state and is being actively developed.](https://www.repostatus.org/badges/latest/active.svg)](https://www.repostatus.org/#active)

Yarer (Yet Another Rust Expression Resolver) is a library for evaluating mathematical expressions. Internally it uses the shunting yard algorithm.

## Usage

Example of usage of the library:

```rust
      let session = Session::init();
      let mut resolver = session.process("1+2"); // or even "(cos(10+e)+3*sin(9/pi))^2" 

      println!("The result is {}", resolver.resolve());
```

All that's needed is to get a new instance of the 'resolver' from a Session and hand over the expression to be analysed.
The library just returns a variant natural number, or a decimal number if one exists in the expression (i.e '2.1+1') or is present a trigonometric function (i.e. 1/cos(x+1)).

## Variables

Yarer handles variables and functions. Here an example:

```rust
      let session = Session::init();
      let mut resolver = session.process("1/cos(x^2)");

      session.set("x",1);
      println!("The result is {}", resolver.resolve());
```

and of course, the expression can be re-evaluated if the variable changes.

```rust
      //...
      session.set("x",-1);
      println!("The result is {}", resolver.resolve());

      session.set("x",0.001); 
      println!("The result is {}", resolver.resolve());
      //...
```

## Casting

The result can be simply casted into a i32 or a f64 (if decimal) simply with

```rust
      let result: Number = resolver.resolve().unwrap();

      let int : i32 = result.into();
      // or
      let float : f64 = result.into();
```

## CLI

Yarer can be used also from command line, and behaves in a very similar manner to GNU bc

```rust
      $ yarer
      Yarer v.0.1.7 - Yet Another Rust Expression Resolver.
      License MIT OR Apache-2.0
      > (1+9)*(8+2)+0!
      101
      > (1./2)+atan(10)
      1.1483608274590869
      > x=10
      > 3/sin(5*x^2)
      -6.41338354698791
      > ln(1)
      0
      > log(10)
      1
      > -2^-2
      0.25
      > 1/(log(10)+cos(0))^-2
      4
      > 4.5+7.9*2.2
      21.88
      > 9801/(2206*sqrt(2)) // approx of PI
      3.1415927300133055
      
```
## News and Updates

Starting with Yarer version 0.1.7, natural numbers are implemented internally using [BigInt](https://crates.io/crates/num-bigint) from the [num crate](https://crates.io/crates/num). Now it is possible to do calculations with arbitrarily large natural numbers.

```rust
    $ yarer
      Yarer v.0.1.7 - Yet Another Rust Expression Resolver.
      License MIT OR Apache-2.0
      > 78!
      1132428117820629783145752115873204622873174957948825.....
      > 2^78
      302231454903657293676544
```

From Yarer version 0.1.5 it's possible to share a single session, and therefore a single heap of variables, for multiple resolvers. The library is not intended to be thread-safe.

```rust
    let session = Session::init();
    
    let mut res = session.process("x ^ 2");
    let mut res2 = session.process("x! - (x-1)!");

    session.set("x", 10);
   
    if let (Ok(a), Ok(b)) = (res.resolve(),res2.resolve()) {
        println!("{} {}", a, b); // 100 3265920
    }
```

## Built-in Defined Functions

There are several math functions defined that you can use in your expression. More to come!
There are many examples of processed expressions in the [integration test file](https://github.com/davassi/yarer/blob/master/tests/integration_tests.rs).

```rust
    Sin
    Cos
    Tan
    ASin
    ACos
    ATan
    Ln
    Log
    Abs
    Sqrt
    Max
    Min
    Floor
    Ceil
    Round
    Exp
```

## Built-in Defined Constants

There are a few predefined math constants available:

```rust
    PI    -> 3.14159265...
    e     -> 2.7182818...
    tau   -> 6.2831853...
    phi   -> 1.6180339...
    gamma -> 0.57721566...
```

## Execute

To run it from cargo, just type:

```console
cargo run -q -- 
```

For logging debug just run with:

```console
env RUST_LOG=yarer=debug cargo run -q -- 
```

or to build and install a release from the code:

```console
cargo build --release
cargo install --path .
./target/release/yarer
```

## Internal Implementation

Each expression is the result of an evaluation by the following actors

Step1 - Parser: A string is "regexed" and converted into a token array.

Step 2 - RpnResolver: Using the Shunting Yard algorithm the token array is converted from infix to postfix notation.

Step 3 - RpnResolver: The resulting RPN (Reverse Polish Notation) expression is evaluated.

Worth to mention that the Session is responsible to store all variables (and constants) that are borrowed by all the RpnResolvers.

## Contribution

Besides being stable, Yarer is a work in progress. If you have suggestions for features (i.e. more math functions to implement), or if you find any issues in the code, design, interface, etc, please feel free to share them on our [GitHub](https://github.com/davassi/yarer/issues).

I appreciate very much your feedback!
