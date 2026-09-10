# Harsh

Development, issues and merge requests live at [gitlab.com/bahiminin.benoit.dah.opensource/harsh-lang](https://gitlab.com/bahiminin.benoit.dah.opensource/harsh-lang); [github.com/Harsh-Programming-Language/harsh-lang](https://github.com/Harsh-Programming-Language/harsh-lang) is a read-only mirror.

```
#[Ha<rs>.h]
     │  │
     │  └── .h
     └───── rs
```

*Rust without the braces.*

```
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

```
cargo build --release
```

Produces a library plus four binaries: `hrs` (the transpiler), `hrs-from` (the Rust-to-Harsh converter), `hrs-remap` (the diagnostic remapper), and `hrs-lsp` (the language server, on-type formatting only; design in `docs/LSP.md`). Both directions share the lexer, the block-kind table, and the carve-out list, which is why they live in one crate: separate crates would let the two directions drift apart silently.

## Use

Transpile a file, writing both the Rust output and a source map:

```
hrs src/main.hrs -o generated/main.rs --map generated/main.map.json
```

Build the generated crate and remap rustc's diagnostics back to your `.hrs` source:

```
cargo build --message-format=json | hrs-remap --map generated/main.map.json
```

The same works for clippy, which is where most lint value comes from without writing any lints:

```
cargo clippy --message-format=json | hrs-remap --map generated/main.map.json
```

Diagnostics report the correct line *and column* in the original file, because the source map records one entry per token rather than per line.

## Working on a project

```
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

```
hrs-from existing.rs -o existing.hrs
```

Converts Rust source to Harsh. Brace nesting comes from the lexer, so it is exact; the only judgement is whether each `{` becomes indentation, and when that is not clear-cut the braces are left alone, which is always valid Harsh.

This also serves as the project's test oracle. Converting the transpiler's own source to Harsh, transpiling it back, and building the result is an automated round-trip test over real code. The current state is a fixed point: the round-tripped transpiler compiles and its binaries produce byte-identical output to the original.

## Examples

Each `.hrs` file has its generated Rust alongside it.

- `examples/hello.hrs` — an axum web server. Attributes, `async`/`.await`, macros, pattern-destructured extractor parameters, multi-line use trees.
- `examples/edge.hrs` — match with inline and block arm bodies, `impl`, `enum`, tuple indexing, turbofish, `do:` as an expression, doc comments.
- `examples/params.hrs` — curried parameter groups, bare parameters, `$` for no parameters, function-typed parameters.
- `examples/general.hrs` — std only. Traits with required and default methods, generic functions with lifetime parameters and multi-line `where` clauses, nested modules, closures, `Result` with early return, `while let`, `if let`.

To run one:

```
cargo new --bin demo
hrs examples/general.hrs -o demo/src/main.rs --map demo/target/hrs.map.json
cd demo && cargo run
```

## Status

Stage one is complete: layout, token substitution, source map, diagnostic remapping, curried parameter declarations, and the Rust-to-Harsh converter. All four examples transpile, compile, and run, and the transpiler round-trips through its own converter to a fixed point.

Deferred, because each requires an expression parser rather than a layout pass: juxtaposed application at call sites (`f x y`), indentation-form struct literals (`Point:`), and `<-` for type paths (`Router <- new()`).

## Licence

Copyright (c) 2026 Bahiminin Benoit Dah. Source code is under the **Mozilla Public License 2.0** (`LICENSE`). File-level copyleft: modifications to Harsh's own files must be published, but there is **no obligation on code you write in Harsh, nor on the Rust it generates**. Build proprietary software with it freely.

The name and logo are reserved separately — see `docs/TRADEMARK.md`. Anyone may fork; a fork must be called something else.

- `docs/GOVERNANCE.md` — who decides what, and what the project will not do
- `CONTRIBUTING.md` — how to propose changes, and the contributor agreement

## Acknowledgements

Harsh is Rust. Every semantic guarantee a Harsh program has — type checking, ownership and borrowing, memory safety, the standard library, the crate ecosystem — is Rust's. The transpiler changes how Rust is written and nothing about what it means; `rustc` does the checking and the remapper only delivers its messages to the right line. The Rust Project and the Rust Foundation deserve the credit for all of that. Rust is a trademark of the Rust Foundation; Harsh is not affiliated with or endorsed by the Rust Project.

The language was designed by its author, Bahiminin Benoit Dah; the implementation was written with [Claude](https://claude.ai) (Anthropic), which did most of the coding under that direction, and whose habit of probing the compiler before believing a claim found several bugs the author would have shipped.
