
YARER - Rpn Resolver
===========================
[<img alt="github" src="https://img.shields.io/badge/github-davassi/davassi?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/davassi/Yarer)
[<img alt="build status" src="https://github.com/davassi/yarer/actions/workflows/rust.yml/badge.svg" height="20">](https://github.com/davassi/yarer/actions?query=branch%3Amaster)
[<img alt="crates.io" src="https://img.shields.io/crates/v/syn.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/yarer)
[<img alt="docs.rs" src="https://img.shields.io/docsrs/yarer?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/yarer)


Yarer (Yet another Rpn expression resolver) is a flexible library, written in Rust, for the processing, compilation and evaluation of Reverse Polish mathematical expressions.

Example of usage of the library: 
      
```rust
      let exp = "((10 + 5) – 3 * (9 / 3)) + 2";
      let resolver = RpnResolver::parse(exp);
      println!("The result of {} is {}", exp, resolver.resolve());
```

All that's needed is to create a new instance of the RpnResolver and hand over the expression to be analysed.
The library just returns a variant natural number, or a decimal number if one exists in the expression (i.e '2.+1').

Yarer can handle also variables and functions. Here an example:

```rust
      let resolver = RpnResolver::parse("1/cos(x^2)");
      resolver.set("x",1);
      println!("The result is {}", resolver.resolve());
```

and of course, the expression can be re-evaluated if the variable changes.

```rust
      //...
      resolver.set("x",-1);
      println!("The result is {}", resolver.resolve());
      resolver.set("x",0.001); 
      println!("The result is {}", resolver.resolve());
      //...
```

Yarer can be used also from command line, and behaves in a very similar manner to GNU bc

```rust
      Yarer
      > 4+2.2
      6.2
      > (1./2)+max(10,8)
      10.5
      > x=10
      > 3/sin(5*x^2)
      -6.41338354698791
      > ln(1)
      0
      > log(10)
      1
```

For logging debug just run with:

```bash
env RUST_LOG=yarer=debug cargo run -q -- 
```




