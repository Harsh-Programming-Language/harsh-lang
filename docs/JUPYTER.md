# Harsh in a Jupyter notebook

A cell of Harsh runs. The kernel is thin: it transpiles the cell with `hrs`,
hands the Rust to the **evcxr** Rust kernel to evaluate, and relays evcxr's
answer — output, values, errors — back to the notebook. Nothing is evaluated
by the kernel itself.

## Install

```sh
cargo install harsh-lang                 # hrs, on your PATH
cargo install evcxr_jupyter              # the Rust kernel
evcxr_jupyter --install                  # registers it with Jupyter as "rust"
python -m pip install ./kernel           # from the harsh-lang checkout (or: python -m pip install harsh-kernel)
cd ~                                     # away from the checkout, so the installed kernel is the one found
python -m harsh_kernel.install           # registers "Harsh"
```

Use `python -m …` for every Python command, as above: pip, the kernel and
Jupyter must be the same Python, and on a Mac `python3` can be Xcode's while
`pip` is another's (conda's, Homebrew's). To reinstall the kernel alone,
without upgrading Jupyter's own packages under other tools (Spyder, for
one): `python -m pip install --force-reinstall --no-deps <the wheel>`.

Then start Jupyter and pick **Harsh** as the kernel. Cells are `.hrs`
fragments: statements, items, or a trailing expression, exactly as a doc
example is written.

## What a cell can hold

```rust harsh
let v = vec! 1 2 3
v <- iter$ <- sum.<i32>$          // a trailing expression is the cell's value
```

```rust harsh
fn twice n: i32 -> i32:            // an item persists for the rest of the session
    n * 2
```

```rust harsh
macro_rules~ pair                  // a Harsh macro unfolds in the cell
    (($a:expr) ($b:expr)) => do:
        ($a, $b)

pair~ 1 2
```

evcxr's own commands pass through untouched — `:dep rand = "0.8"`, `:vars`,
`:help` — since they are the kernel's, not the language's.

## Matrices

Harsh's matrices work in a notebook as in a project: `v~`, `m~`, `.*`,
slicing, `f<>`. They live in `hrs_std`, which ships inside `hrs`, not on
crates.io; so the first time a cell uses it, the kernel writes it with
`hrs dist` into its own folder and gives it to evcxr (`:dep`), once per
session. That first cell says so, and takes a minute or two while evcxr
compiles nalgebra; later cells are quick.

```rust harsh
let v = v~ [1, 3, 4]
v
```

Needs kernel 0.1.3 and `hrs` 0.1.34 or later.

## Errors

A Harsh error points at the cell's line and column (`cell:2:5`), the same
message `hrs` gives for a file. A Rust error from evcxr is relayed as evcxr
reports it, with its file reference rewritten to `cell:`.

## An evcxr quirk to know

An older evcxr (the 0.17 line, which is what a Rust 1.75 toolchain can build)
cannot display the value of a trailing expression whose type is fixed by a
turbofish on a method chain — `v <- iter$ <- sum.<i32>$` reports *no method
named `evcxr_display`*. Bind it first, `let s: i32 = …` then `s`, or use a
current evcxr. The same Rust fails straight into the Rust kernel, so it is not
the transpiler's doing.

## What it does not do yet

- Completion and hover: those come from `hrs-lsp`, once it has types.
