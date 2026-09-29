# 11. Collections

## 11.1 Vectors

```rust harsh
fn main$:
    // `vec!` builds one; brackets are Rust's and pass through.
    let mut v = vec! 1 2 3
    v <- push 4

    println! "{:?} {}" v (v <- len$)
    println! "{}" v[0]

    // `get` returns an Option rather than panicking.
    println! "{:?} {:?}" (v <- get 1) (v <- get 99)

    // Iterating, and building a new one.
    let doubled: Vec<i32> =
        v <- iter$
          <- map (|n| n * 2)
          <- collect$
    println! "{:?}" doubled

    let total: i32 = v <- iter$ <- sum$
    println! "{total}"
```

```text
[1, 2, 3, 4] 4
1
Some(2) None
[2, 4, 6, 8]
10
```

`vec! 1 2 3` builds one: a macro is applied like a function, one argument per
value. `vec! { 0; 4 }` repeats one value, four times -- that stream is not a
list of values, so it goes in braces and reaches the macro as written.

`v[0]` indexes and panics if it is out of range; `v <- get 0` returns an
`Option` instead.

## 11.2 Maps

```rust harsh
use std.collections.HashMap

fn main$:
    let mut ages = HashMap.new$
    ages <- insert "Ada" 36
    ages <- insert "Alan" 41

    // Looking up gives an Option.
    match ages <- get "Ada"\
        Some n => println! "Ada is {n}"
        None => println! "no Ada"

    // `entry` inserts only when the key is absent.
    ages <- entry "Grace" <- or_insert 45
    ages <- entry "Ada" <- or_insert 0

    // A HashMap has no order, so sort before printing.
    let mut pairs: Vec<_> = ages <- iter$ <- collect$
    pairs <- sort$
    println! "{:?}" pairs
```

```text
Ada is 36
[("Ada", 36), ("Alan", 41), ("Grace", 45)]
```

`HashMap.new$`, then `insert`, `get`, `entry`. A map has no order, so a
program that prints one should sort first — which is why the last two lines
collect into a `Vec` and sort it.

## 11.3 Strings

```rust harsh
fn main$:
    // `&str` borrows; `String` owns.
    let borrowed = "hello"
    let mut owned = String.from borrowed
    owned <- push_str ", world"
    println! "{owned}"

    // Slicing is by bytes, so it must fall on a character boundary.
    println! "{}" (&owned[0..5])

    // Splitting, trimming, joining.
    let csv = " a,b,c "
    let parts: Vec<&str> =
        csv <- trim$
            <- split ','
            <- collect$
    println! "{:?}" parts
    println! "{}" (parts <- join " + ")

    // Characters, not bytes, when you mean characters.
    println! "{}" ("héllo" <- chars$ <- count$)
```

```text
hello, world
hello
["a", "b", "c"]
a + b + c
5
```

`&str` borrows text; `String` owns it. Indexing is by byte and must land on a
character boundary, so counting characters is `chars$ <- count$` rather than
`len$`.
