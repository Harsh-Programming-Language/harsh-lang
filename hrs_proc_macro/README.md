# hrs_proc_macro

The runtime of Harsh's own procedural macros. A Harsh proc macro is an
ordinary Harsh function marked `#[proc_macro~]`:

```rust harsh
use hrs_proc_macro.TokenStream

#[proc_macro~]
pub fn hello_macro (input: TokenStream) -> TokenStream:
    let input_str = input <- to_string$
    let output = format! "Hello, {}!" input_str
    output <- parse$ <- unwrap$
```

and is called `hello_macro~ world`. It runs when `hrs` transpiles the
calling crate, not when rustc compiles it: what it returns is Harsh, which
`hrs` splices where the call stood, so the generated Rust holds no macro.

In this version a `TokenStream` is Harsh text, convertible both ways with
`to_string` and `parse`. `serve` is the entry point of the small runner
binary `hrs` generates; a macro author never calls it.
