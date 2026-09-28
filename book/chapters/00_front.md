# The Harsh Programming Language

## Why no braces

Here is what a brace language looks and reads like, once you notice it:

```text
I(would, like, to, go) {
    to the market so I(could buy) {
        some groceries
    }
    to the laundromat in order to(get) {
        clothes washed
    }
}
```

and here is the message it was trying to convey:

```text
I would like to go
    to the market so I could buy
        some groceries
    to the laundromat in order to get
        clothes washed
```

The second is already in the first — the indentation was there all along; the braces and parentheses only repeat it, in a form the eye has to skip past. That is sometimes hard to see, and because of it some very simple concepts become harder to learn than they are. Harsh weeds out the braces to let Rust shine.

Early in the work, Claude asked the author to justify removing braces from Rust at all. After a long enough exchange it put the answer in one line, and the line stayed:

> *Braces are for the compiler; indentation is for humans.* — Claude

**Rust without braces.**
**Rust with pipes, partial application, comprehensions and linear algebra.**
**Rust for functional programming, data science and machine learning.**

## What Harsh adds

Braces are where Harsh starts, not where it stops. It adds four things Rust has no syntax for — and they are the reason to use it. Each below is Harsh, then the Rust it replaces.

**Pipes and partial application.** Give a function fewer arguments than it takes, and you get a function waiting for the rest:

```
let double = 2.0 |> scale
let doubled: Vec<f64> = readings <- iter$ <- map (|&x| double x) <- collect$
```

**Generator comprehensions.** Say what a collection holds, not how to fill it:

```
let triples =
    list~ (a, b, c)
        for a in 1..20
        for b in a..20
        for c in b..20 if a * a + b * b == c * c
```

**Linear algebra, as in Julia.** Julia's matrix literal, Julia's `*`, and Julia's `X \ y`, here fitting a line by least squares:

```
let x = m~ [1.0 1.0; 1.0 2.0; 1.0 3.0]
let y = v~ [1.0, 2.0, 2.9]
let beta = x <- solve (&y)
```

Chapters 14, 15 and 16 teach them properly, once closures and iterators — chapter 13 — have given you what they are built from.

## Who this is for

You have programmed before — in Python, JavaScript, Elixir, anything — and you have not programmed in Rust. This book teaches you Rust, using Harsh as the notation.

That sentence has a trap in it, so here it is on page one: **Harsh makes Rust quieter, not easier.** Harsh is a spelling of Rust — indentation for structure, `f a b` for applying a function, `<-` for reaching into a value — and it changes nothing about what Rust means. Ownership, borrowing, lifetimes, the type system, the compiler saying no: all of it arrives on schedule, unchanged, and this book is mostly about *that*. The notation just gets out of the way while you learn it.

When you want practice rather than prose, [Harshlings](https://gitlab.com/bahiminin.benoit.dah.opensource/harshlings) is fifty small exercises of the kind this book explains: a short program with one thing wrong, the compiler pointing at your line, and the next one when you fix it.

If you already know Rust, *The Harsh Language Guide*, which teaches only the Harsh superset, will onboard you faster.

## How to read it

Every example is shown two ways: the **Harsh** you would write, and what it **prints** when run. Both are produced by the build from a single source file — the file is transpiled, compiled and run to make the page — so they cannot disagree. When an example is *meant* to fail, and learning Rust means learning what the compiler refuses and why, you see the Harsh and then the error the compiler reports, pointing at the line you wrote.

> **Harsh —** Harsh is one spelling of Rust, and not the usual one. Rust as you will meet it in crates and in answers on the internet is written differently — the same language, the same meanings, other punctuation. Nothing in this book depends on that other spelling and none of it appears here. When you want the two set side by side, *The Harsh Language Guide* is where that comparison lives, and `hrs export` turns any project of yours into the other spelling to read.

The chapters follow the order of *The Rust Programming Language* (the "Rust Book"), because that order has been refined over years to introduce each idea exactly when you need it; the prose and the examples here are original.


## Setting up

You need the Rust toolchain and the Harsh tools:

```text
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh     # Rust, if you do not have it
cargo install harsh-lang                                               # hrs, hrs-from, hrs-remap, hrs-lsp
hrs new hello && cd hello && hrs run                              # a first project
```

If the last line prints `Hello from Harsh`, you are ready. The tools are the crate [`harsh-lang`](https://crates.io/crates/harsh-lang); for the editor, install [Harsh](https://marketplace.visualstudio.com/items?itemName=harsh-lang.harsh-lang) from the VSCode Marketplace, and `hrs-lsp` — already on your `PATH` — gives it the layout-aware Enter, Tab and format-on-save the rest of this book assumes you have.

Add rust-analyzer, Rust's own language server, once — `rustup component add rust-analyzer`, or have VSCode's rust-analyzer extension, whose copy is used — and `hrs-lsp` gives the editor three more things: hover over a name to see its type and documentation, jump to where it is defined (F12 in VSCode), and completion as you type. `hrs-lsp` asks rust-analyzer about the Rust your file becomes and points the answer back at your Harsh, and converts the answer into Harsh — a signature reads `fn adding (x: i32) (y: i32) -> i32`, a path `std.slice.Iter` — and a definition in your project opens in its `.hrs` file, one in the standard library in Rust's. Hover and definition follow your last save: after you change a line, save, and they work on it again; completion works on what you are typing. Without rust-analyzer everything else works, and the editor says what to install.
