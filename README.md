# Harsh

Development, issues and merge requests live at [gitlab.com/bahiminin.benoit.dah.opensource/harsh-lang](https://gitlab.com/bahiminin.benoit.dah.opensource/harsh-lang); [github.com/Harsh-Programming-Language/harsh-lang](https://github.com/Harsh-Programming-Language/harsh-lang) is a read-only mirror; the website, written in Harsh, is `site/` and is published from the mirror to [harsh-lang.com](https://harsh-lang.com/).

```text
#[Ha<rs>.h]
     │  │
     │  └── .h
     └───── rs
```

**Rust without braces.**
**Rust with pipes, partial application, comprehensions and linear algebra.**
**Rust for functional programming, data science and machine learning.**

Braces are for the compiler; indentation is for humans.

## What Harsh adds

Braces are where Harsh starts, not where it stops. It adds four things Rust has no syntax for — and they are the reason to use it. Each below is Harsh, then the Rust it replaces.

**Pipes and partial application.** Give a function fewer arguments than it takes, and you get a function waiting for the rest:

```rust harsh
let double = 2.0 |> scale
let doubled: Vec<f64> =
    readings <- iter$
             <- map (|&x| double x)
             <- collect$
```

```rust
let double = |x| scale(2.0, x);
let doubled: Vec<f64> = readings.iter().map(|&x| double(x)).collect();
```

**Generator comprehensions.** Say what a collection holds, not how to fill it:

```rust harsh
let triples =
    list~ (a, b, c)
        for a in 1..20
        for b in a..20
        for c in b..20 if a * a + b * b == c * c
```

```rust
let triples: Vec<_> = (1..20)
    .flat_map(|a| (a..20).flat_map(move |b| (b..20).map(move |c| (a, b, c))))
    .filter(|&(a, b, c)| a * a + b * b == c * c)
    .collect();
```

**Linear algebra, as in Julia.** Julia's matrix literal, Julia's `*`, and Julia's `X \ y`, here fitting a line by least squares:

```rust harsh
let x = m~ [1.0 1.0; 1.0 2.0; 1.0 3.0]
let y = v~ [1.0, 2.0, 2.9]
let beta = x <- solve (&y)
```

```rust
let x = DMatrix::from_row_slice(3, 2, &[1.0, 1.0, 1.0, 2.0, 1.0, 3.0]);
let y = DVector::from_vec(vec![1.0, 2.0, 2.9]);
let beta = x.svd(true, true).solve(&y, 1e-12).unwrap();
```

The Book teaches all four in chapters 14 to 16; the Language Guide has a section on each; *Harsh by Example* has a page on each; Harshlings has exercises.

```sh
cargo install harsh-lang            # hrs, hrs-from, hrs-remap, hrs-lsp
hrs new hello && cd hello && hrs run
```

The toolchain is the crate [`harsh-lang`](https://crates.io/crates/harsh-lang); the VSCode extension is [`harsh-lang.harsh-lang`](https://marketplace.visualstudio.com/items?itemName=harsh-lang.harsh-lang) on the Marketplace (`code --install-extension harsh-lang.harsh-lang`).

A layout transformation over Rust. Blocks are delimited by indentation instead of braces, paths use `.` instead of `::`, and field access and method calls use `<-` instead of `.`. Source files carry the `.hrs` extension.

The transpiler understands block structure and nothing else. There is no semantic AST, no type knowledge, and no name resolution — every token it does not explicitly rewrite is copied through to Rust. That is why generics, lifetimes, closures, `impl Trait`, `async`/`.await`, macros, and every future Rust syntax addition work without the transpiler knowing they exist.

- `docs/START-HERE.md` — the map of everything else.
- `docs/LANGUAGE.md` — the language reference.
- `docs/SPEC.md` — implementation notes, block-kind rules, source map format, verification status.

## Build

```sh
cargo build --release
```

Produces a library plus four binaries: `hrs` (the transpiler), `hrs-from` (the Rust-to-Harsh converter), `hrs-remap` (the diagnostic remapper), and `hrs-lsp` (the language server, on-type formatting only; design in `docs/LSP.md`). Both directions share the lexer, the block-kind table, and the carve-out list, which is why they live in one crate: separate crates would let the two directions drift apart silently.

## Use

Transpile a file, writing both the Rust output and a source map:

```sh
hrs src/main.hrs -o generated/main.rs --map generated/main.map.json
```

Build the generated crate and remap rustc's diagnostics back to your `.hrs` source:

```sh
cargo build --message-format=json | hrs-remap --map generated/main.map.json
```

The same works for clippy, which is where most lint value comes from without writing any lints:

```sh
cargo clippy --message-format=json | hrs-remap --map generated/main.map.json
```

Diagnostics report the correct line *and column* in the original file, because the source map records one entry per token rather than per line.

## Working on a project

```sh
hrs new myapp          scaffold a project laid out for Harsh
cd myapp
hrs run                transpile, then cargo run
hrs watch              rebuild on every save
```

Harsh sources live in `src/**.hrs`. Generated Rust and its source maps go to `target/hrs/`, and `Cargo.toml` points its targets at that tree:

```toml
[[bin]]
name = "myapp"
path = "target/hrs/main.rs"
```

One manifest, dependencies untouched, and nothing extra to gitignore since Cargo already ignores `/target`. Plain `cargo build` works once the tree has been transpiled.

| Command | Does |
|---|---|
| `hrs build` | transpile changed files, then `cargo build` |
| `hrs run` | …then `cargo run` |
| `hrs test` | …then `cargo test` |
| `hrs check` | …then `cargo check` |
| `hrs lint` | …then `cargo clippy` |
| `hrs watch [sub]` | rebuild on save; defaults to `check` |
| `hrs new <name>` | scaffold a project |
| `hrs export [dir]` | write the project as a plain Rust crate, formatted with `cargo fmt` — the hand-off artifact; `target/hrs/` is for the compiler, not for reading |

Transpiling is incremental by modification time. Every diagnostic — from rustc or clippy — is remapped onto your `.hrs` source with the correct line and column; Harsh's own syntax errors are rendered the same way.

## Bringing existing Rust in

```sh
hrs-from existing.rs -o existing.hrs
```

Converts Rust source to Harsh. Brace nesting comes from the lexer, so it is exact; the only judgement is whether each `{` becomes indentation, and when that is not clear-cut the braces are left alone, which is always valid Harsh.

This also serves as the project's test oracle. Converting the transpiler's own source to Harsh, transpiling it back, and building the result is an automated round-trip test over real code. The current state is a fixed point: the round-tripped transpiler compiles and its binaries produce byte-identical output to the original.

## Learning it

- **[The Harsh Programming Language](book/HARSH-BOOK.md)** — Rust taught in Harsh, from the first program to async, for readers who do not know Rust. One file per chapter in [`book/chapters/`](book/chapters).
- **[The Harshonomicon](nomicon/THE-HARSHONOMICON.md)** — a companion to *The Rustonomicon*: unsafe Harsh, from memory layout to a `Vec` and an `Arc` built from scratch. One file per chapter in [`nomicon/pages/`](nomicon/pages).
- **[Harsh Design Patterns](patterns/HARSH-DESIGN-PATTERNS.md)** — a companion to *Rust Design Patterns*: the idioms, patterns and anti-patterns, every program in Harsh. One file per chapter in [`patterns/pages/`](patterns/pages).
- **[Harsh by Example](by-example/HARSH-BY-EXAMPLE.md)** — short programs, one idea each, for looking up how something is written. One file per page in [`by-example/pages/`](by-example/pages).
- **[The Harsh Language Guide](docs/LANGUAGE.md)** — every construct beside its Rust spelling, for Rust programmers: the fastest way in.
- **[Harshlings](https://gitlab.com/bahiminin.benoit.dah.opensource/harshlings)** — fifty small exercises, each a short program with one thing wrong; the compiler points at the line of the `.hrs` file you are editing, you fix it, and the runner moves on. `cargo install harsh-lang`, clone, `hrs run`. ([mirror](https://github.com/Harsh-Programming-Language/harshlings))

Every code sample in all four is transpiled, compiled and run by the build.

## Examples

Each `.hrs` file has its generated Rust alongside it.

- `examples/hello.hrs` — an axum web server. Attributes, `async`/`.await`, macros, pattern-destructured extractor parameters, multi-line use trees.
- `examples/edge.hrs` — match with inline and block arm bodies, `impl`, `enum`, tuple indexing, turbofish, `do:` as an expression, doc comments.
- `examples/params.hrs` — curried parameter groups, bare parameters, `$` for no parameters, function-typed parameters.
- `examples/general.hrs` — std only. Traits with required and default methods, generic functions with lifetime parameters and multi-line `where` clauses, nested modules, closures, `Result` with early return, `while let`, `if let`.

To run one:

```sh
cargo new --bin demo
hrs examples/general.hrs -o demo/src/main.rs --map demo/target/hrs.map.json
cd demo && cargo run
```

## Status

Stage one is complete: layout, token substitution, source map, diagnostic remapping, curried parameter declarations, and the Rust-to-Harsh converter. All four examples transpile, compile, and run, and the transpiler round-trips through its own converter to a fixed point.

Deferred, because each requires an expression parser rather than a layout pass: juxtaposed application at call sites (`f x y`), indentation-form struct literals (`Point:`), and `<-` for type paths (`Router <- new()`).

Procedural macros and support for foreign DSLs (`view!`, `rsx!`, `sql!`) are still in development. Declarative macros are finished: Harsh's own `macro_rules~`, which unfolds into Harsh, and Rust's `macro_rules!`, copied verbatim.

## Licence

Copyright (c) 2026 Bahiminin Benoit Dah. Source code is under the **Mozilla Public License 2.0** (`LICENSE`). File-level copyleft: modifications to Harsh's own files must be published, but there is **no obligation on code you write in Harsh, nor on the Rust it generates**. Build proprietary software with it freely.

The name and logo are reserved separately — see `docs/TRADEMARK.md`. Anyone may fork; a fork must be called something else.

- `docs/GOVERNANCE.md` — who decides what, and what the project will not do
- `CONTRIBUTING.md` — how to propose changes, and the contributor agreement

## Acknowledgements

Harsh is Rust. Every semantic guarantee a Harsh program has — type checking, ownership and borrowing, memory safety, the standard library, the crate ecosystem — is Rust's. The transpiler changes how Rust is written and nothing about what it means; `rustc` does the checking and the remapper only delivers its messages to the right line. The Rust Project and the Rust Foundation deserve the credit for all of that. Rust is a trademark of the Rust Foundation; Harsh is not affiliated with or endorsed by the Rust Project.

Harsh was designed by Bahiminin Benoit Dah and built with [Claude](https://claude.ai) (Anthropic): the design decisions are the author's, most of the code is Claude's, and every claim about the language was checked against a real compiler before it was believed — a habit that found several bugs on the way.

The two disagreed often, which was the point. The `\` mark, the three struct forms as equals, the pipes as the one deferral mechanism, the shape of a block literal, the name — each was the author's call, several against Claude's first opinion, and each held. The rule that made that work: every proposal had to be argued before it was built, and every claim checked against a compiler before it was believed — so a disagreement ended with an example either could point at, not with whoever spoke last.
