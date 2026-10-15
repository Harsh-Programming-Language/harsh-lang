# 15. The tools

Harsh is a transpiler, so the tools are a thin layer over cargo's.

    hrs new NAME          a project: Cargo.toml, src/main.hrs, .gitignore
    hrs run               transpile what changed, then cargo run
    hrs build             transpile, then cargo build
    hrs test              transpile, then cargo test
    hrs check             transpile, then cargo check
    hrs lint              the same, through clippy, with its notes remapped
    hrs fmt               lay out the Harsh in the canonical form
    hrs watch             rebuild as files change
    hrs expand FILE       the file with its Harsh macros unfolded
    hrs -c CMD            cargo CMD in export/: hrs -c publish for crates.io
    hrs -- PROGRAM        any program, run in export/: hrs -- dx serve

A project keeps its Harsh in `src/**.hrs`, and its `Cargo.toml` is cargo's,
unchanged. Each build writes the project as Rust to `export/` -- its
`Cargo.toml`, `src/` transpiled, the other files beside them -- and cargo
builds it there:

    myapp/
        Cargo.toml  Hrs.toml  src/main.hrs
        export/     Cargo.toml  src/main.rs  target/
        .hrs/       the source maps

`hrs new` puts `/export` and `/.hrs` in `.gitignore`: both are written again
from the `.hrs` files.

```rust harsh
// This is what `hrs new` writes for you, and what `hrs run` builds.
fn main$:
    let args: Vec<String> = std.env.args$ <- collect$
    let name =
        if (args <- len$) > 1:
            args[1] <- clone$
        else:
            String.from "world"
    println! "hello, {name}"
```

```text
hello, world
```

Every diagnostic comes back on the `.hrs` line and column that produced it,
not on the generated Rust, because the transpiler writes a source map and the
driver runs cargo's output back through it. That is true of rustc's errors,
its suggestions, and clippy's lints alike.

`hrs-from` goes the other way, turning Rust into Harsh — which is how an
existing crate is brought across.
