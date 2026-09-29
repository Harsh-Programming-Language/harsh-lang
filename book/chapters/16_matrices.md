# 16. Matrices and linear algebra

The third thing Harsh adds is linear algebra that reads like Julia's. A matrix literal with the same grammar, `*` meaning the matrix product, `A \ b` solving a system, and the same errors when sizes do not fit. If you have written Julia, nothing here will surprise you; if you have not, it is the notation of the textbooks, which is why Julia chose it.

The literals, `m~` and `v~`, are part of the language — they are in the prelude, like `g~`. The types they build, `Matrix` and `Vector`, live in `hrs_std`, Harsh's standard library, a crate you add to a project once:

```toml
[dependencies]
hrs_std = "0.1"
```

A project that uses `m~` without it is stopped by `hrs`, with that line to add.

## 16.1 Writing a matrix

`Cargo.toml`

```text
[dependencies]
hrs_std = "0.1"
```

`src/main.hrs`

```rust harsh
fn main$:
    // Spaces between entries, `;` between rows.
    let a = m~ [1 2 3; 4 5 6]
    println! "{a}"

    // A line break is a `;`: write a matrix as it reads.
    let b =
        m~ [1 2 3
            4 5 6]
    // A bracket inside is a block: here, three columns side by side.
    let c = m~ [[1, 4] [2, 5] [3, 6]]
    println! "{} {}" (a == b) (a == c)

    // Commas make a vector, which is a column.
    let v = v~ [0.5, 1.5, 2.5]
    println! "{v}"
```

```text
$ hrs run
2×3 Matrix<i32>:
 1 2 3
 4 5 6
true true
3-element Vector<f64>:
 0.5
 1.5
 2.5
```

Three rules build every matrix literal, and they compose:

- a **space** puts things side by side;
- a **`;`** or a **line break** puts them one above another;
- a **comma** makes the entries of a vector, which is a column.

A bracket inside the literal is a *block*, built by the same rules: `[1, 4]` is a column, so `[[1, 4] [2, 5] [3, 6]]` is three columns side by side — the same matrix as `[1 2 3; 4 5 6]`. The rules are Julia's, and so are the refusals: `m~ [[1 2], [3 4]]` is not a matrix, because a comma never joins — in Julia it is a vector holding two matrices — and `v~ [1 2 3]` is not a vector, because spaces make a row, which is a matrix.

A matrix prints its size and the type of its entries, as Julia's do. Sizes are values, not part of the type: a `Matrix<i32>` may be any size, and its size is known when the program runs.

## 16.2 Arithmetic

`Cargo.toml`

```text
[dependencies]
hrs_std = "0.1"
```

`src/main.hrs`

```rust harsh
fn main$:
    let a = m~ [1.0 2.0; 3.0 4.0]
    let b = m~ [0.0 1.0; 1.0 0.0]
    let x = v~ [1.0, 1.0]

    println! "{}" (&a * &b)
    println! "{}" (2.0 * (&a + &b))
    println! "{}" (&a * &x)
    println! "{:?}" (a <- size$)
    println! "{}" (a <- transpose$)
```

```text
$ hrs run
2×2 Matrix<f64>:
 2 1
 4 3
2×2 Matrix<f64>:
 2 6
 8 8
2-element Vector<f64>:
 3
 7
(2, 2)
2×2 Matrix<f64>:
 1 3
 2 4
```

`*` between two matrices is the matrix product. Between a number and a matrix it scales every entry. Between a matrix and a vector it gives a vector. Nothing marks which is meant: the compiler chooses from what is being multiplied, the way Julia chooses when the program runs.

`&a * &b` borrows the two matrices, so they can be used again; `a * b` would consume them. That is Rust's ownership rule, and the one visible difference from Julia. `size$` gives the rows and the columns, and `transpose$` is Julia's `transpose` — the `A'` of Julia is not available, because `'` begins a character or a lifetime in Rust.

## 16.3 The identity

`Cargo.toml`

```text
[dependencies]
hrs_std = "0.1"
```

`src/main.hrs`

```rust harsh
use hrs_std.(UniformScaling, I)

fn main$:
    let a = m~ [1 2; 3 4]
    let u = UniformScaling 2
    println! "{}" (&a + u)
    println! "{}" (&a * u)
    println! "{}" (&a + I)
```

```text
$ hrs run
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

`UniformScaling k` is Julia's: `k` times an identity matrix of whatever size the other side needs. `I` is the identity itself. `a + I` adds one along the diagonal; `a * u` scales by `k`.

## 16.4 Joining matrices

`Cargo.toml`

```text
[dependencies]
hrs_std = "0.1"
```

`src/main.hrs`

```rust harsh
fn main$:
    let a = m~ [1 2; 3 4]
    let z = m~ [0 0; 0 0]
    println! "{}" (m~ [(&a) (&z)])
    println! "{}" (m~ [(&a) (&z)
                       (&z) (&a)])
```

```text
$ hrs run
2×4 Matrix<i32>:
 1 2 0 0
 3 4 0 0
4×4 Matrix<i32>:
 1 2 0 0
 3 4 0 0
 0 0 1 2
 0 0 3 4
```

A block can be any matrix, not only a number, so the literal that builds a matrix from numbers also joins matrices: side by side with a space, one above another with a `;` or a new line. Here a 2×2 block matrix and a 4×4 one. Each block is isolated in parentheses, `(&a)`, as any argument with an operator in it is.

## 16.5 Solving

`Cargo.toml`

```text
[dependencies]
hrs_std = "0.1"
```

`src/main.hrs`

```rust harsh
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
$ hrs run
x = 2, y = 1
det 1
2×2 Matrix<f64>:
 1 -1
 -1 2
length 5, dot 25
```

`a <- solve (&b)` is Julia's `a \ b`: the `x` for which `a * x` equals `b`. Harsh cannot use `\` for it — `\` opens a specification block — so the method is named for what it does. `det$` and `inv$` are the determinant and the inverse, with Julia's names. Asking for the inverse of a matrix that has none stops the program with Julia's `SingularException`.

A vector has a length, `norm$`, and a dot product, `dot`. Entries are read by index: `v[0]` for a vector and `a[(0, 1)]` for a matrix. Indexes start at 0, as they do for every collection in Harsh — this is the one place Harsh departs from Julia, which counts from 1.

## 16.6 Fitting a line

`Cargo.toml`

```text
[dependencies]
hrs_std = "0.1"
```

`src/main.hrs`

```rust harsh
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
$ hrs run
score = 47.20 + 4.37 × hours
residual 1.76
7 hours: 77.8
```

This is linear regression: a straight line through data that does not lie on one. Given students' hours and scores, find the intercept and slope that fit best.

Each student is a row of the *design matrix*: a 1, which will be multiplied by the intercept, and their hours, which will be multiplied by the slope. A comprehension builds those rows from the data. The matrix has six rows and two columns — more equations than unknowns — so no line fits every point exactly, and `solve` returns the *least-squares* line: the one that makes the total squared error smallest. That is what Julia's `X \ y` does for a tall matrix, and it is the first step of most statistics and machine learning. The residual measures what the line leaves unexplained, and the line predicts a score for seven hours.

## 16.7 Taking a part

`Cargo.toml`

```text
[dependencies]
hrs_std = "0.1"
```

`src/main.hrs`

```rust harsh
fn main$:
    // Three days of readings from three sensors: a row a day.
    let mut readings = m~ [21 22 19; 22 24 21; 18 19 17]
    let first_two_days = readings <- slice (0..2) (..)
    println! "{first_two_days}"
    let day_one = readings <- slice 1 (..)
    println! "{day_one}"
    // A view borrows: a window, nothing copied.
    let window = readings <- view (1..) (..=1)
    println! "{window}"
    let kept = window <- copy$
    // An element, then a whole row, written through.
    readings[0, 0] = 20
    readings <- view_mut 2 (..) <- fill 0
    println! "{readings}"
    println! "{kept}"
```

```text
$ hrs run
2×3 Matrix<i32>:
 21 22 19
 22 24 21
3-element Vector<i32>:
 22
 24
 21
2×2 view of Matrix<i32>:
 22 24
 18 19
3×3 Matrix<i32>:
 20 22 19
 22 24 21
 0 0 0
2×2 Matrix<i32>:
 22 24
 18 19
```

A matrix is rarely wanted whole. `readings <- slice (0..2) (..)` takes rows 0 and 1 and every column, as a matrix of its own. The ranges are the ones you know from chapter 8 — counted from 0, the end left out — and `..` by itself means *all of it*. Name an axis with a single number and that axis disappears: `slice 1 (..)` is one day, a vector, not a 1×3 matrix.

A view does the same without copying. `readings <- view (1..) (..=1)` is a window onto `readings`, borrowed, exactly as `&names[1..3]` is a window onto a vector in chapter 4 — and the borrow checker guards it in the same way, so a view cannot outlive or be written under the matrix it looks into. It is a value: you can name it, pass it, print it, use it in `.*` like a matrix, and `<- copy$` turns it into a matrix you own. Julia writes it `view(readings, 2:3, 1:2)`.

What can be read can be written. `readings[0, 0] = 20` sets one element; for more than one, `view_mut` is the view that writes through: `readings <- view_mut 2 (..) <- fill 0` sets a whole row, and `*(readings <- view_mut 1 1) = 5` one element through it.

An index with several axes is written with a comma, `readings[0, 0]`. The comma makes the axes one value, a tuple, so `readings[(0, 0)]` means the same; the rule is the language's, not the matrix's, and any type indexed by a tuple may be written so. An index gives one element; a part of a matrix is a `slice` or a `view` — Rust's index must hand back something stored in the matrix, and a window onto it is not.

Julia writes a part `readings[2:3, 1:2]`, counting from 1 and including the end. Harsh keeps the ranges of every other collection, so there is one way to count in a program.

## 16.8 Element by element

`Cargo.toml`

```text
[dependencies]
hrs_std = "0.1"
```

`src/main.hrs`

```rust harsh
fn relu (x: f64) -> f64:
    x <- max 0.0

fn main$:
    let scores = m~ [1.0 -2.0; -3.0 4.0]
    let weights = m~ [10.0 20.0; 30.0 40.0]
    let bias = m~ [100.0 200.0]
    // The product, and then the same two matrices element by element.
    println! "{}" (&scores * &weights)
    println! "{}" (&scores .* &weights)
    // A number and a row stretch to fit.
    println! "{}" (&scores .* 2.0 .+ &bias)
    // A function applied to each element; two arguments; a closure.
    println! "{}" (relu<> scores)
    println! "{}" (f64.powf<> scores 2.0)
    println! "{}" ((|s, w| s > 0.0 && w > 15.0)<> scores weights)
    // In a pipeline, and `scores` is still ours afterwards.
    println! "{}" (scores |> relu<> |> f64.sqrt<>)
    println! "{} by {}" (scores <- nrows$) (scores <- ncols$)
```

```text
$ hrs run
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
 false true
2×2 Matrix<f64>:
 1 0
 0 2
2 by 2
```

`*` between two matrices is the matrix product. With a dot before it, `.*`, the operation is applied *element by element*: the first with the first, the second with the second. `.+`, `.-` and `./` are the same. The two sides need not be the same size, as long as they can be *stretched* to it: along each axis the two lengths must be equal, or one of them 1. So a number combines with a matrix, and so does a single row — `bias` is added to every row of the result. A size that cannot stretch stops the program, as in the next section.

`relu<> scores` applies the function `relu` to each element. `<>` is a mark written tight against a function, like the `$` of chapter 3: `f$` applies `f` to nothing, `f<>` applies it to each. It takes up to three arguments and stretches them as the dotted operators do, so `f64.powf<> scores 2.0` squares every element; it works on a closure in parentheses; the elements may change type, as the comparison does, giving a matrix of `bool`; and it takes its place in a pipeline. It only ever borrows: `scores` is still there at the end.

Beneath the mark is a method, `scores <- map relu`, as `slice` is beneath the index.

A dotted operator keeps the rank of the operator it is made from: `x + a .* b` multiplies first, and `a * b .* c` works from left to right. Julia writes these `a .* b` and `f.(a)`. The first is the same in Harsh; the second could not be, because `f.(a)` already begins a group of paths, as in a `use`.

## 16.9 When the sizes do not fit

`Cargo.toml`

```text
[dependencies]
hrs_std = "0.1"
```

`src/main.hrs`

```rust harsh
fn main$:
    let a = m~ [1 2 3; 4 5 6]
    // A 2×3 matrix times a 2×3 matrix: the inner sizes, 3 and 2, differ.
    println! "{}" (&a * &a)
```

```text
$ hrs run
thread 'main' panicked at src/main.hrs:4:20:
DimensionMismatch: matrix A has dimensions (2, 3), matrix B has dimensions (2, 3)
```

A product needs the columns of the left matrix to match the rows of the right. Sizes are known when the program runs, not when it compiles, so a mismatch is caught then and reported in Julia's words: `DimensionMismatch`, with the two sizes.

## 16.10 What you have

`m~ [1 2; 3 4]` — spaces for side by side, `;` or a new line for one above another, brackets for blocks — and `v~ [1, 2, 3]` for a vector. `*` is the product, scaling or matrix-times-vector by what it multiplies; `+` and `-` are entrywise. `UniformScaling k` and `I` for the identity. `solve` for Julia's `\`, exact for a square matrix and least squares for a tall one; `inv$`, `det$`, `transpose$`, `norm$`, `dot`. Sizes are values checked at run time, and indexes start at 0. The types come from `hrs_std`.

`slice` copies a part and an index with ranges borrows one; `.*` and its kin work element by element, and `f<>` applies a function to each.

Next: back to Rust's own ground — cargo, and the second half of the book.
