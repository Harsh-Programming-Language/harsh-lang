# harsh-kernel

A Jupyter kernel for [Harsh](https://gitlab.com/bahiminin.benoit.dah.opensource/harsh-lang),
Rust without the braces. Each cell is transpiled to Rust with `hrs` and run by
[evcxr](https://github.com/evcxr/evcxr), Rust's own Jupyter kernel; an error
points at the line of the cell it is on.

```sh
cargo install harsh-lang                               # hrs
cargo install evcxr_jupyter && evcxr_jupyter --install  # the Rust kernel underneath
pip install harsh-kernel
python3 -m harsh_kernel.install                        # registers "Harsh" with Jupyter
```

Then pick the **Harsh** kernel in Jupyter. Variables, functions and types
carry from one cell to the next, as in evcxr.

MPL-2.0, like Harsh.
