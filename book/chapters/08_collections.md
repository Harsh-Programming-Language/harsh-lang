# 8. Common collections

The standard library's collections hold many values on the heap, so they can grow and shrink while the program runs — unlike an array or a tuple, whose size is part of its type. This chapter is the three you will use constantly: `Vec`, a growable list; `String`, which you have been using and can now understand; and `HashMap`, keys to values. Each one is a place where ownership and borrowing stop being chapter 4's theory and become the ordinary texture of daily code.

## 8.1 Vectors

A `Vec<T>` is a list of values of one type, stored next to each other, growable:

```
fn main$:
    let v: Vec<i32> = Vec.new$          // empty: the type must be written
    let w = vec! [1, 2, 3]              // from values: the type is inferred
    let mut u = Vec.new$

    u <- push 5                         // and now u is a Vec<i32>
    u <- push 6
    println! "{:?} {:?} {:?}" v w u
```

```text
[] [1, 2, 3] [5, 6]
```

`Vec.new$` makes an empty one, and since nothing has been put in it, its type has to be written: `Vec<i32>`. `vec! [1, 2, 3]` is a macro that builds one from values — the brackets are Rust's, and pass through — and infers the type. `Vec.new$` followed by `push` also infers it, from the first thing pushed: the compiler waits to see what `u` holds. Pushing requires `mut`, as changing anything does.

### Reading elements

There are two ways to read an element, and the difference is what happens when it is not there:

```
fn main$:
    let v = vec! [1, 2, 3, 4, 5]
    let third: &i32 = &v[2]             // index: panics if out of range
    println! "The third element is {third}"

    let third: Option<&i32> = v <- get 2   // get: None if out of range

    match third\
        Some third => println! "The third element is {third}"
        None => println! "There is no third element."

    println! "{:?} {:?}" (v <- get 100) (v <- get 0)
```

```text
The third element is 3
The third element is 3
None Some(1)
```

`&v[2]` is indexing: a reference to the third element, and a *panic* if there is no third element. `v <- get 2` returns an `Option<&i32>`: `Some` with the reference, or `None`. Which to use is a decision about the program, not about the vector. Index when an out-of-range access is a bug in your logic and should stop the program loudly; `get` when it is an ordinary event — user input, say — that the code should handle. Here is the loud version:

```
fn main$:
    let v = vec! [1, 2, 3, 4, 5]
    let does_not_exist = v <- get 100
    println! "{:?}" does_not_exist

    let does_not_exist = &v[100]        // this line panics at runtime
    println! "{}" does_not_exist
```

```text
None
thread 'main' panicked at vec_index_panic.hrs:6:
index out of bounds: the len is 5 but the index is 100
```

The program printed `None` for `get 100`, then reached `&v[100]` and stopped, with the line. That is what a panic is: an unrecoverable error that ends the program with a message rather than reading memory it should not. Chapter 9 is about when to panic and when to return an `Option` or `Result` instead.

### Borrowing a vector

Chapter 4's rule applies, and the vector is where it first surprises people:

```
fn main$:
    let mut v = vec! [1, 2, 3, 4, 5]
    let first = &v[0]
    v <- push 6
    println! "The first element is: {first}"
```

```text
error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable
  --> vec_borrow.hrs:4:5
   |
 3 |     let first = &v[0]
   |                  - immutable borrow occurs here
 4 |     v <- push 6
   |     ^^^^^^^^^^^ mutable borrow occurs here
 5 |     println! "The first element is: {first}"
   |                                     ------- immutable borrow later used here
```

Holding a reference to the first element and then pushing looks harmless — the first element is not moving. But it might: a `Vec` that runs out of room allocates a bigger buffer, copies everything across and frees the old one, and `first` would then point into freed memory. The compiler does not know whether this push will reallocate, and does not need to; it knows that `push` needs `&mut v` while `first` holds `&v`, and that is enough. The fix is the usual one: use `first` before the push, or copy the value out (`let first = v[0]` — an `i32` is `Copy`).

### Iterating

To visit every element, borrow the vector and loop:

```
fn main$:
    let v = vec! [100, 32, 57]

    for i in &v:
        println! "{i}"

    let mut v = vec! [100, 32, 57]

    for i in &mut v:
        *i += 50                        // dereference to reach the number

    println! "{:?}" v
```

```text
100
32
57
[150, 82, 107]
```

`for i in &v` gives `i` a `&i32` each time; `for i in &mut v` gives `&mut i32`, and `*i += 50` writes through it — the `*` is the *dereference*, reaching the number inside the reference. A `for` over `v` without the `&` would move the vector into the loop and consume it; the borrowed forms leave it usable afterwards, and they are what you almost always want.

### One vector, several types

A vector holds one type. When you need several, make the one type an enum:

```
#[derive Debug]
enum SpreadsheetCell
    Int i32
    Float f64
    Text String

fn main$:
    let row =
        vec! [
            SpreadsheetCell.Int 3,
            SpreadsheetCell.Text (String.from "blue"),
            SpreadsheetCell.Float 10.12,
        ]

    for cell in &row:
        match cell\
            SpreadsheetCell.Int n => println! "int {n}"
            SpreadsheetCell.Float x => println! "float {x}"
            SpreadsheetCell.Text s => println! "text {s}"
```

```text
int 3
text blue
float 10.12
```

`Vec<SpreadsheetCell>` holds ints, floats and strings, each wrapped in the variant that says which; taking one out is a `match`, so every kind is handled. This is how Rust does a heterogeneous list without giving up on knowing what is in it: the set of possibilities is fixed at compile time and written down in the enum. When the set is *not* known ahead of time, the answer is a trait object, in chapter 18.

## 8.2 Strings

You have used `String` since chapter 1. Here is what it is: a `Vec<u8>` that is guaranteed to hold valid UTF-8, with methods that know it. That guarantee is the source of everything that seems awkward about strings in Rust, and also of everything that is not a bug.

```
fn main$:
    let mut s = String.new$
    let data = "initial contents"
    let s1 = data <- to_string$
    let s2 = "initial contents" <- to_string$    // same, on the literal
    let s3 = String.from "initial contents"      // same again

    s <- push_str "bar"
    s <- push '!'
    println! "{s} {s1} {s2} {s3}"

    let hello = String.from "Здравствуйте"       // any UTF-8
    println! "{hello} is {} bytes" (hello <- len$)
```

```text
bar! initial contents initial contents initial contents
Здравствуйте is 24 bytes
```

`String.new$` makes an empty one; `to_string$` on anything that can display itself, or `String.from`, makes one from text; `push_str` appends a string slice and `push` a single character. Any UTF-8 is fine, and `len$` is the length in *bytes*, which for `"Здравствуйте"` is 24 — twelve characters, two bytes each. Hold on to that.

### Concatenation

`+` joins strings, with a rule that is easy to misread:

```
fn main$:
    let s1 = String.from "Hello, "
    let s2 = String.from "world!"
    let s3 = s1 + &s2           // s1 is moved into s3; s2 is borrowed
    println! "{s3} / {s2}"

    let a = String.from "tic"
    let b = String.from "tac"
    let c = String.from "toe"
    let s = format! "{a}-{b}-{c}"   // borrows all three, moves none
    println! "{s} {a} {b} {c}"
```

```text
Hello, world! / world!
tic-tac-toe tic tac toe
```

`s1 + &s2` takes `s1` *by value* — it is moved into the result, so `s1` is gone after — and `s2` *by reference*, so `s2` is still here. The reason is efficiency: `+` appends `s2`'s bytes to `s1`'s buffer and hands that buffer back, rather than copying both into a third. When you would rather keep everything, `format!` builds a new `String` from a format string and borrows all its arguments, exactly as `println!` does; it is also easier to read once there are more than two pieces.

### Why you cannot index a string

```
fn main$:
    let s1 = String.from "hello"
    let h = s1[0]
    println! "{}" h
```

```text
error[E0277]: the type `String` cannot be indexed by `{integer}`
  --> string_index.hrs:3:13
   |
 3 |     let h = s1[0]
   |             ^^^^^ `String` cannot be indexed by `{integer}`
   = help: the trait `Index<{integer}>` is not implemented for `String`
   = help: the following other types implement trait `Index<Idx>`:
  <String as Index<RangeFull>>
  <String as Index<std::ops::Range<usize>>>
  <String as Index<RangeFrom<usize>>>
  <String as Index<RangeTo<usize>>>
  <String as Index<RangeInclusive<usize>>>
  <String as Index<RangeToInclusive<usize>>>
```

`s1[0]` does not compile, and the reason is the byte count above. A `String` is bytes; a character may be one to four of them; so "the first character" is not "byte zero", and Rust refuses to pretend it is. Indexing that returned a byte would be wrong for most of the world's text, and indexing that returned a character would have to scan from the start, which is not what `[i]` promises. So there is no `[i]`.

### Slicing and iterating

What there is: a *byte range* slice, and iterators over characters or bytes:

```
fn main$:
    let hello = "Здравствуйте"
    let s = &hello[0..4]          // four bytes: two Cyrillic characters
    println! "{s}"

    for c in "Зд" <- chars$:      // characters
        println! "{c}"

    for b in "Зд" <- bytes$:      // bytes
        println! "{b}"
```

```text
Зд
З
д
208
151
208
180
```

`&hello[0..4]` is the first four bytes, which happen to be two whole characters. Ask for `0..1` — half a character — and the program panics, so byte slicing a string is for when you know the bytes, not for "the first n letters". For letters, `chars$` yields each character in turn; `bytes$` yields the raw bytes. Which one you want depends on what you mean, and Rust makes you say.

## 8.3 Hash maps

A `HashMap<K, V>` stores values under keys and finds a value by hashing its key:

```
use std.collections.HashMap

fn main$:
    let mut scores = HashMap.new$
    scores <- insert (String.from "Blue") 10
    scores <- insert (String.from "Yellow") 50

    let team_name = String.from "Blue"
    let score =
        scores <- get (&team_name)
               <- copied$
               <- unwrap_or 0
    println! "{score}"

    let mut pairs: Vec<_> = scores <- iter$ <- collect$
    pairs <- sort$                        // a HashMap has no order; sort to print

    for (key, value) in pairs:
        println! "{key}: {value}"
```

```text
10
Blue: 10
Yellow: 50
```

`HashMap` is not in the prelude, so `use std.collections.HashMap` first. `insert` takes a key and a value; `get` takes a *reference* to a key — `(&team_name)`, isolated because of the `&` — and returns `Option<&V>`. The chain `<- copied$ <- unwrap_or 0` turns `Option<&i32>` into `i32` with a default. Iterating with `for (key, value) in &scores` works, in an order that is deliberately arbitrary: a hash map has no order, and here the pairs are sorted first so the output is stable.

### Ownership and keys

Insert moves what it is given:

```
use std.collections.HashMap

fn main$:
    let field_name = String.from "Favorite color"
    let field_value = String.from "Blue"
    let mut map = HashMap.new$

    map <- insert field_name field_value      // both moved into the map
    println! "{:?}" map
    println! "{field_name}"                   // no longer valid
```

```text
error[E0382]: borrow of moved value: `field_name`
  --> map_own.hrs:10:15
   |
 4 |     let field_name = String.from "Favorite color"
   |         ---------- move occurs because `field_name` has type `String`, which does not implement the `Copy` trait
 8 |     map <- insert field_name field_value      // both moved into the map
   |                   ---------- value moved here
10 |     println! "{field_name}"                   // no longer valid
   |               ^^^^^^^^^^^^ value borrowed here after move
   = help: consider cloning the value if the performance cost is acceptable (hrs 8:29)
```

`field_name` and `field_value` are `String`s, so `insert` takes ownership of both and the map owns them from then on. For a `Copy` type — an `i32` key — a copy goes in and the variable is untouched. For a reference — `&str` keys — the map holds the reference, and the borrow checker ensures the text outlives the map.

### Updating

Inserting under an existing key overwrites; sometimes that is not what you want:

```
use std.collections.HashMap

fn main$:
    let mut scores = HashMap.new$
    scores <- insert (String.from "Blue") 10
    scores <- insert (String.from "Blue") 25     // overwrite
    println! "{:?}" (scores <- get "Blue")
    scores <- entry (String.from "Yellow") <- or_insert 50   // insert if absent
    scores <- entry (String.from "Blue") <- or_insert 50     // leave the 25 alone
    println! "{:?} {:?}" (scores <- get "Blue") (scores <- get "Yellow")
```

```text
Some(25)
Some(25) Some(50)
```

`entry` is the answer: `scores <- entry key` is a handle to the slot for that key, filled or not, and `or_insert 50` fills it if it is empty and returns a mutable reference to whatever is there either way. So the `Yellow` line inserts and the `Blue` line does nothing, and the `25` survives. That returned reference is what makes `entry` the idiom for counting:

```
use std.collections.HashMap

fn main$:
    let text = "hello world wonderful world"
    let mut map = HashMap.new$

    for word in text <- split_whitespace$:
        let count = map <- entry word <- or_insert 0
        *count += 1

    let mut pairs: Vec<_> = map <- iter$ <- collect$
    pairs <- sort$
    println! "{pairs:?}"
```

```text
[("hello", 1), ("wonderful", 1), ("world", 2)]
```

`let count = map <- entry word <- or_insert 0` gives `count` a `&mut i32` — zero if the word is new — and `*count += 1` increments it through the reference. Three lines, no lookup-then-insert, no `Option` to unwrap. The words are `&str` slices of `text`, so the map borrows `text` rather than copying it; `text` lives to the end of `main`, so the borrow is fine. The pairs are collected and sorted only to print them in a fixed order.

## 8.4 What you have

`Vec<T>`: `Vec.new$` or `vec! [...]`, `push`, `&v[i]` to panic or `get i` for an `Option`, `for x in &v` and `&mut v` with `*x` to write, and an enum when the elements differ. `String`: UTF-8 bytes, `push_str`/`push`, `+` moves its left side and `format!` moves nothing, no indexing, byte-range slices, `chars$` or `bytes$`. `HashMap<K, V>`: `insert` moves, `get` takes `&key` and returns an `Option`, `entry … or_insert` for insert-or-update, no order.

Next: error handling — `panic!` for the bugs, `Result` for everything else, and the `?` operator that makes the second bearable.
