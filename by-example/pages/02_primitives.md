# 2. Primitives

## 2.1 Literals and operators

```rust harsh
fn main$:
    // A suffix names the type; an underscore is a spacer.
    let big = 1_000_000u64
    let small: i8 = -7
    let ratio = 2.5f32

    // Integer, float, bool, char.
    let yes = true
    let letter = 'H'

    println! "{big} {small} {ratio} {yes} {letter}"

    // The operators are Rust's, unchanged.
    println! "{}" (1 + 2 * 3)
    println! "{}" (7 / 2)          // integer division truncates
    println! "{}" (7 % 2)
    println! "{}" (1u32 << 5)
    println! "{}" (true && false)
```

```text
1000000 -7 2.5 true H
7
3
1
32
false
```

Numbers, `bool` and `char` are Rust's, and so is every operator in that list.
Harsh changes none of it. The one thing to watch is the parentheses: `1 + 2 * 3`
is an operator expression, and an operator expression handed to a call is
isolated, so it is `println! "{}" (1 + 2 * 3)`.

## 2.2 Tuples

```rust harsh
// A tuple struct: its fields juxtapose, like any application.
// (Parens would hold *one* payload: `struct Wrapped (i32, i32)` has a
// single field that is a pair.)
#[derive Debug]
struct Matrix f32 f32 f32 f32

fn transpose (m: Matrix) -> Matrix:
    // Fields of a tuple are reached by number.
    Matrix (m <- 0) (m <- 2) (m <- 1) (m <- 3)

fn main$:
    // A tuple value: one construct, commas inside.
    let pair = (1, "one")
    println! "{:?}" pair

    // Taking it apart binds both halves.
    let (n, name) = pair
    println! "{n} is {name}"

    // Constructing juxtaposes, as every application does.
    let m = Matrix 1.1 1.2 2.1 2.2
    println! "{:?}" m
    println! "{:?}" (transpose m)
```

```text
(1, "one")
1 is one
Matrix(1.1, 1.2, 2.1, 2.2)
Matrix(1.1, 2.1, 1.2, 2.2)
```

Two different things share the parentheses here, and it is worth keeping them
apart.

A **tuple value** is `(1, "one")` — one construct, commas inside, exactly as in
Rust. A **tuple struct** declares its fields by juxtaposition:
`struct Matrix f32 f32 f32 f32`. Parentheses in a declaration would hold a
single payload, so `struct Wrapped (i32, i32)` is one field that happens to be
a pair, not two fields.

Constructing follows the same rule as any call — `Matrix 1.1 1.2 2.1 2.2` —
and reaching a field by number is `m <- 0`.

## 2.3 Arrays and slices

```rust harsh
fn main$:
    // An array: fixed length, all one type.
    let xs: [i32; 5] = [1, 2, 3, 4, 5]

    // `[value; count]` fills it.
    let zeros = [0; 3]

    // Indexing is tight against the name.
    println! "{} {}" xs[0] xs[4]
    println! "{:?} {:?}" xs zeros
    println! "{}" (xs <- len$)

    // A slice borrows part of it.
    let middle = &xs[1..4]
    println! "{:?}" middle

    // Out of range is a panic, not a wrong answer -- see the next example.
    for x in xs <- iter$:
        print! "{x} "
    println! ""
```

```text
1 5
[1, 2, 3, 4, 5] [0, 0, 0]
5
[2, 3, 4]
1 2 3 4 5
```

An index is written tight against the name, `xs[0]`. That tightness is what
makes it part of the atom: in an application, `f xs[0]` passes the element,
while a spaced `f xs [0]` is refused rather than guessed at.

A slice borrows a run of it: `&xs[1..4]`.

Reading past the end is a panic, which is to say it stops rather than
returning something wrong:

```rust harsh
fn main$:
    let xs = [1, 2, 3]
    let n = 5
    println! "reaching for element {n}"
    println! "{}" xs[n]
```

```text
reaching for element 5
thread 'main' panicked at out_of_range.hrs:5:20:
```
