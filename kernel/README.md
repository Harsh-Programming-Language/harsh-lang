# harsh-kernel

A Jupyter kernel for [Harsh](https://gitlab.com/bahiminin.benoit.dah.opensource/harsh-lang),
Rust without the braces. Each cell is transpiled to Rust with `hrs` and run by
[evcxr](https://github.com/evcxr/evcxr), Rust's own Jupyter kernel; an error
points at the line of the cell it is on.

```sh
cargo install harsh-lang                               # hrs
cargo install evcxr_jupyter && evcxr_jupyter --install  # the Rust kernel underneath
python -m pip install harsh-kernel
python -m harsh_kernel.install                         # registers "Harsh" with Jupyter
```

`python -m …` for both, so pip and the kernel are the same Python -- on a
Mac, `python3` can be Xcode's while `pip` is conda's or Homebrew's.

Then pick the **Harsh** kernel in Jupyter. Variables, functions and types
carry from one cell to the next, as in evcxr.

MPL-2.0, like Harsh.
