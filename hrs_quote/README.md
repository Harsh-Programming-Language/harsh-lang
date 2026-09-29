# hrs_quote

Harsh's counterpart to Rust's `quote`. In a Harsh proc-macro crate:

```rust harsh
use hrs_quote.quote

let expanded = quote~ do:
    impl HelloMacro for #name
        fn hello_macro$:
            println! "Hello, Macro! My name is {}!" (stringify! #name)
```

The template is the block beneath `quote~ do:`, laid out as the output
reads; `#name` interpolates anything that implements `ToTokens`, indented to
the column where it stands. `hrs` expands `quote~` into a call of this crate's
`quote!`, which assembles the stream at run time.
