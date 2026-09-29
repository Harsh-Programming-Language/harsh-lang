# hrs_std

Harsh's standard library: the types behind the matrix and vector literals of
[Harsh](https://gitlab.com/bahiminin.benoit.dah.opensource/harsh-lang) —
*Rust without braces, with pipes, partial application, comprehensions and
linear algebra*.

```rust harsh
let x = m~ [1.0 1.0; 1.0 2.0; 1.0 3.0]
let y = v~ [1.0, 2.0, 2.9]
let beta = x <- solve (&y)          // Julia's X \ y: the least-squares fit
```

`Matrix<T>` and `Vector<T>` follow Julia, over [nalgebra](https://nalgebra.org):
`*` is the matrix product, scaling or matrix-times-vector, chosen from the
operand types; `solve` is Julia's `\` (exact for a square matrix, least squares
for a tall one); `inv`, `det`, `transpose`, `dot`, `norm`, `UniformScaling`,
`I`, `hcat`, `vcat`. Sizes are values, and a mismatch is a `DimensionMismatch`
in Julia's words, reported at the caller's line. `try_solve` and `try_inv`
(0.1.4) do the same work and return a `Result<_, LinAlgError>` instead of
panicking, for a matrix from data the caller has not checked.

**Slicing** keeps Harsh's 0-based ranges, `..` alone being Julia's `:`.
`a <- slice (0..2) (..)` copies; `a <- view (0..2) (..)` borrows -- a *view*,
Julia's `view(a, 1:2, :)`, a value holding the matrix's reference and its
window -- and `a <- view_mut …` writes through. An axis taken by a number is
dropped, as in Julia. **Broadcasting** is Julia's: `a .* b`, `.+`, `.-`, `./`,
shapes stretched where a length is 1, and `f<> a` for Julia's `f.(a)`. From
Rust these are `&a * DOT * &b`, `each!(f, a)`, `a.slice(0..2, ..)` and
`a.view(0..2, ..)`.

The crate has no `unsafe` (since 0.1.3: views were once references forged
over a zero-sized slice, which Miri found undefined behaviour).

The literals `m~` and `v~` are part of the language; a Harsh project that uses
them adds one line:

```toml
[dependencies]
hrs_std = "0.1"
```

The Book's chapter 16 teaches it. From Rust, the types are usable directly.

MPL-2.0.
