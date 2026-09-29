# hrs_syn

Harsh's counterpart to Rust's `syn`, for procedural macros written in Harsh:

```rust harsh
use hrs_proc_macro.TokenStream
use hrs_quote.quote
use hrs_syn.{parse_macro_input, DeriveInput}

#[proc_macro_derive~ HelloMacro]
pub fn hello_macro_derive (input: TokenStream) -> TokenStream:
    let input = parse_macro_input! { input as DeriveInput }
    let name = input <- ident
    quote~ do:
        impl HelloMacro for #name
            fn hello_macro$:
                println! "Hello, Macro! My name is {}!" (stringify! #name)
```

`DeriveInput` is `syn`'s shape -- attributes, visibility, name, generics, and
the struct's fields or the enum's variants -- read from Harsh's item grammar.
