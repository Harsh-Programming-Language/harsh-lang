# The Harsh Programming Language

## Who this is for

You have programmed before — in Python, JavaScript, Elixir, anything — and you have not programmed in Rust. This book teaches you Rust, using Harsh as the notation.

That sentence has a trap in it, so here it is on page one: **Harsh makes Rust quieter, not easier.** Harsh is a spelling of Rust — indentation for structure, `f a b` for applying a function, `<-` for reaching into a value — and it changes nothing about what Rust means. Ownership, borrowing, lifetimes, the type system, the compiler saying no: all of it arrives on schedule, unchanged, and this book is mostly about *that*. The notation just gets out of the way while you learn it.

If you already know Rust, *The Harsh Language Guide*, which teaches only the Harsh superset, will onboard you faster.

## How to read it

Every example is shown two ways: the **Harsh** you would write, and what it **prints** when run. Both are produced by the build from a single source file — the file is transpiled, compiled and run to make the page — so they cannot disagree. When an example is *meant* to fail, and learning Rust means learning what the compiler refuses and why, you see the Harsh and then the error the compiler reports, pointing at the line you wrote.

@harsh Harsh is one spelling of Rust, and not the usual one. Rust as you will meet it in crates and in answers on the internet is written differently — the same language, the same meanings, other punctuation. Nothing in this book depends on that other spelling and none of it appears here. When you want the two set side by side, *The Harsh Language Guide* is where that comparison lives, and `hrs export` turns any project of yours into the other spelling to read.

The chapters follow the order of *The Rust Programming Language* (the "Rust Book"), because that order has been refined over years to introduce each idea exactly when you need it; the prose and the examples here are original.


## Setting up

You need the Rust toolchain and the Harsh tools. *The Harsh Language Guide*'s "Setting up" section has the exact commands; in short:

```text
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh     # Rust, if you do not have it
cargo install --path harsh                                        # from the Harsh repository
hrs new hello && cd hello && hrs run                              # a first project
```

If the last line prints `Hello from Harsh`, you are ready.
