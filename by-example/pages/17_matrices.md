# 17. Matrices

Harsh's matrices are Julia's, for anyone who knows Julia: the same literal,
the same meaning for `*`, the same errors. They live in `hrs_std`, Harsh's
standard library, which a project adds to its dependencies once. The literals
themselves, `m~` and `v~`, need no `use`: they are part of the language.

## 17.1 Writing them

```toml
[dependencies]
hrs_std = "0.1"
```

```
fn main$:
    // A matrix: spaces between entries, `;` between rows.
    let a = m~ [1 2 3; 4 5 6]
    println! "{a}"
    // Rows may be blocks of their own, stacked with `;`.
    let b = m~ [[1 2 3]; [4 5 6]]
    println! "{}" (a == b)

    // A vector is a column: commas, or one entry per line.
    let v = v~ [1.5, 2.5, 3.5]
    println! "{v}"

    // A bracket inside is a block. Side by side, columns make a matrix.
    let c = m~ [[1, 4] [2, 5] [3, 6]]
    println! "{}" (a == c)
```

```text
2×3 Matrix<i32>:
 1 2 3
 4 5 6
true
3-element Vector<f64>:
 1.5
 2.5
 3.5
true
```

Three rules make every matrix literal, and they compose. A **space** puts
things side by side. A **`;`** puts them one above another — and so does a
line break, so a matrix may be written one row per line, as it reads. A
**comma** makes the entries of a vector, which is a column.

A bracket inside the literal is a block built by the same rules, so
`[[1, 4] [2, 5] [3, 6]]` is three columns side by side — the same matrix as
`[1 2 3; 4 5 6]`. What the literal refuses, it refuses with Julia's reason:
`m~ [[1 2], [3 4]]` is two matrices in a vector, not one matrix, because a
comma never joins; and `v~ [1 2 3]` is a row, which is a matrix, not a vector.

A matrix prints its shape and its element type, as Julia does.

## 17.2 Arithmetic

```toml
[dependencies]
hrs_std = "0.1"
```

```
fn main$:
    let a = m~ [1.0 2.0; 3.0 4.0]
    let b = m~ [0.0 1.0; 1.0 0.0]
    let x = v~ [1.0, 1.0]

    // One `*`, three meanings, chosen from what it multiplies.
    println! "{}" (&a * &b)        // the matrix product
    println! "{}" (2.0 * (&a + &b))  // scaling a sum
    println! "{}" (&a * &x)        // a matrix times a vector is a vector

    println! "{:?} {}" (a <- size$) (a <- transpose$)
```

```text
2×2 Matrix<f64>:
 2 1
 4 3
2×2 Matrix<f64>:
 2 6
 8 8
2-element Vector<f64>:
 3
 7
(2, 2) 2×2 Matrix<f64>:
 1 3
 2 4
```

`*` between two matrices is the matrix product; between a number and a matrix
it is scaling; between a matrix and a vector it gives a vector. The compiler
chooses, from what is being multiplied — nothing has to be spelled out.

The `&` is ownership, Rust's rule: `&a * &b` borrows both, so `a` and `b` are
still there on the next line. Without it, the product would consume them.

## 17.3 The identity

```toml
[dependencies]
hrs_std = "0.1"
```

```
use hrs_std.(UniformScaling, I)

fn main$:
    let a = m~ [1 2; 3 4]
    let u = UniformScaling 2
    // As in Julia: `u` is 2 times an identity of whatever size is needed.
    println! "{}" (&a + u)
    println! "{}" (&a * u)
    println! "{}" (&a + I)
```

```text
2×2 Matrix<i32>:
 3 2
 3 6
2×2 Matrix<i32>:
 2 4
 6 8
2×2 Matrix<i32>:
 2 2
 3 5
```

`UniformScaling k` is `k` times an identity of whatever size the other side
needs, and `I` is the identity itself — Julia's own names.

## 17.4 Joining matrices

```toml
[dependencies]
hrs_std = "0.1"
```

```
fn main$:
    let a = m~ [1 2; 3 4]
    // Any matrix is a block, so the literal joins matrices as it joins numbers.
    println! "{}" (m~ [(&a) (&a)])
    println! "{}" (m~ [(&a); (&a)])
```

```text
2×4 Matrix<i32>:
 1 2 1 2
 3 4 3 4
4×2 Matrix<i32>:
 1 2
 3 4
 1 2
 3 4
```

A block may be any matrix, so the literal that builds a matrix from numbers
also joins matrices: side by side with a space, stacked with a `;`. Each block
is isolated in parentheses, `(&a)`, as any argument with an operator is.

## 17.5 Solving

```toml
[dependencies]
hrs_std = "0.1"
```

```
fn main$:
    // 2x + y = 5 and x + y = 3, as a matrix and a vector.
    let a = m~ [2.0 1.0; 1.0 1.0]
    let b = v~ [5.0, 3.0]
    // Julia's `a \ b`.
    let x = a <- solve (&b)
    println! "x = {}, y = {}" x[0] x[1]

    println! "det {}" (a <- det$)
    println! "{}" (a <- inv$)

    let v = v~ [3.0, 4.0]
    println! "length {}, dot {}" (v <- norm$) (v <- dot (&v))
```

```text
x = 2, y = 1
det 1
2×2 Matrix<f64>:
 1 -1
 -1 2
length 5, dot 25
```

`a <- solve (&b)` is Julia's `a \ b`: the `x` with `a * x == b`. `det$`,
`inv$`, `norm$` and `dot` carry Julia's names. Indexes start at 0, as for
every collection in Harsh.

## 17.6 Fitting a line

```toml
[dependencies]
hrs_std = "0.1"
```

```
use hrs_std.Matrix

fn main$:
    // Hours studied, and the score each student got.
    let hours = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    let score = v~ [52.0, 55.0, 61.0, 64.0, 70.0, 73.0]

    // The design matrix, one row per student: a 1 for the intercept,
    // then the hours. A comprehension builds the rows.
    let x = Matrix.from_rows (list~ (vec! 1.0 h) for h in hours)

    // Julia's `β = X \ y`: for a tall matrix, the least-squares fit.
    let beta = x <- solve (&score)
    println! "score = {:.2} + {:.2} × hours" beta[0] beta[1]

    let fitted = &x * &beta
    println! "residual {:.2}" ((&score - &fitted) <- norm$)
    println! "7 hours: {:.1}" (beta[0] + beta[1] * 7.0)
```

```text
score = 47.20 + 4.37 × hours
residual 1.76
7 hours: 77.8
```

With more rows than columns, `solve` returns the least-squares fit, as Julia's
`X \ y` does: linear regression in one line, with the design matrix built by
a comprehension. Chapter 16 of the Book explains each step.

## 17.7 Slicing

```toml
[dependencies]
hrs_std = "0.1"
```

```
fn main$:
    let mut a = m~ [1 2 3; 4 5 6; 7 8 9]
    // The method copies, as Julia's `a[1:2, :]` does. `..` alone is Julia's `:`.
    println! "{}" (a <- slice (0..2) (..))
    // An axis taken by a number is dropped: a row is a vector.
    println! "{}" (a <- slice 1 (..))
    // A view borrows, as Julia's `view(a, 2:3, 1:2)`: nothing copied.
    let window = a <- view (1..) (..=1)
    println! "{}" window
    let corner = window <- copy$
    println! "{}" corner
    // An element by its index; a whole row through a view that writes.
    a[1, 1] = 50
    a <- view_mut 2 (..) <- fill 0
    println! "{}" a
```

```text
2×3 Matrix<i32>:
 1 2 3
 4 5 6
3-element Vector<i32>:
 4
 5
 6
2×2 view of Matrix<i32>:
 4 5
 7 8
2×2 Matrix<i32>:
 4 5
 7 8
3×3 Matrix<i32>:
 1 2 3
 4 50 6
 0 0 0
```

Julia writes `a[1:2, :]`. Harsh keeps its own ranges -- 0-based, the end left
out, as for every collection -- and `..` alone is Julia's `:`. There are two
ways to take a part. `a <- slice (0..2) (..)` *copies*, as Julia's `a[1:2, :]`
does. `a <- view (0..2) (..)` *borrows*, as Julia's `view(a, 1:2, :)`: a
window onto `a`, guarded by the borrow checker like any borrow, and a value you
can name, print or use in `.*`; `<- copy$` makes it a matrix of its own. An
axis taken by a number is dropped, so `slice 1 (..)` and `view 1 (..)` are a
row, read as a vector. To write, `a[1, 1] = 50` sets an element, and
`view_mut` is the view that writes through: `a <- view_mut 2 (..) <- fill 0`.

An index takes its axes with a comma, `a[i, j]`; the parenthesised `a[(i, j)]`
means the same and is what the comma stands for. An index is one element: a
part of a matrix is a `slice` or a `view`.

## 17.8 Broadcasting

```toml
[dependencies]
hrs_std = "0.1"
```

```
fn relu (x: f64) -> f64:
    x <- max 0.0

fn main$:
    let a = m~ [1.0 -2.0; -3.0 4.0]
    let b = m~ [10.0 20.0; 30.0 40.0]
    let row = m~ [100.0 200.0]
    // `*` is the matrix product; `.*` multiplies element by element.
    println! "{}" (&a * &b)
    println! "{}" (&a .* &b)
    // Shapes stretch as in Julia: a number, a row, a column.
    println! "{}" (&a .* 2.0 .+ &row)
    // `f<>` applies `f` to each element: Julia's `f.(a)`.
    println! "{}" (relu<> a)
    println! "{}" (f64.powf<> a 2.0)
    println! "{}" ((|x, y| x > y)<> a b)
    // It pipes, and nothing above consumed `a`.
    println! "{}" (a |> relu<> |> f64.sqrt<>)
```

```text
2×2 Matrix<f64>:
 -50 -60
 90 100
2×2 Matrix<f64>:
 10 -40
 -90 160
2×2 Matrix<f64>:
 102 196
 94 208
2×2 Matrix<f64>:
 1 0
 0 4
2×2 Matrix<f64>:
 1 4
 9 16
2×2 Matrix<bool>:
 false false
 false false
2×2 Matrix<f64>:
 1 0
 0 2
```

A dot before an operator applies it element by element: `.*`, `.+`, `.-`, `./`,
Julia's own spellings. Shapes stretch as Julia stretches them -- along an axis
two lengths must be equal, or one of them 1 -- so a number, a row or a column
combines with a matrix. Precedence is the operator's own: `x + a .* b`
multiplies first.

`f<>` is *apply to each*, Julia's `f.(a)`: a mark written tight against a
function, as `$` and `!` are, saying how it is applied. It takes up to three
arguments, borrows them, works on a closure in parentheses, and pipes. With it
the operators that have no dotted form are ordinary functions: `f64.powf<> a 2.0`
is Julia's `a .^ 2`, and a comparison gives a matrix of `bool`. Beneath it is a
method, `a <- map f`, as `slice` is beneath the index.

## 17.9 When sizes do not fit

```toml
[dependencies]
hrs_std = "0.1"
```

```
use hrs_std.UniformScaling

fn main$:
    let b = m~ [1 2 3; 4 5 6]
    // Adding a scaled identity needs a square matrix, and this is 2×3.
    println! "{}" (&b - (UniformScaling 2))
```

```text
DimensionMismatch: matrix is not square: dimensions are (2, 3)
```

A matrix's size is a value, known when the program runs, not a type. So a
mismatch is caught then, and reported in Julia's words: `DimensionMismatch`,
with the sizes that did not fit.
