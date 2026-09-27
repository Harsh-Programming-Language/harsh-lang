"""A Jupyter kernel for Harsh.

A cell of Harsh is transpiled to Rust with `hrs`, handed to the evcxr Rust
kernel to evaluate, and evcxr's answer is relayed back. Errors that name a
line in the Rust are mapped back to the cell's own lines through the source
map `hrs` writes.

The kernel is a *wrapper*: it does not evaluate anything itself. It owns one
evcxr kernel per notebook, speaks the Jupyter protocol on both sides, and
translates in the middle. See `docs/JUPYTER.md`.
"""

__version__ = "0.1.3"
