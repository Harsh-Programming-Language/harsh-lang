# Ownership

The rules the borrow checker enforces — and why unsafe code must uphold
them too, since the compiler optimises on the assumption that they hold. The
original chapter: [Ownership](https://doc.rust-lang.org/nomicon/ownership.html).

## 3.1 References

*Original: [References](https://doc.rust-lang.org/nomicon/references.html)*

Two kinds: a shared reference `&` and a mutable reference `&mut`. Two rules
govern them: a reference cannot outlive what it refers to, and a mutable
reference cannot be aliased — while it is alive, nothing else reaches the
same value. The compiler refuses a second `&mut`:

```rust harsh
fn main$:
    let mut total = 0
    let a = &mut total
    // Refused: `total` is already borrowed mutably by `a`.
    let b = &mut total
    *a += 1
    *b += 1
```

```text
error[E0499]: cannot borrow `total` as mutable more than once at a time
 --> references.hrs:5:13
  |
3 |     let a = &mut total;
  |             ---------- first mutable borrow occurs here
4 |     // Refused: `total` is already borrowed mutably by `a`.
5 |     let b = &mut total;
  |             ^^^^^^^^^^ second mutable borrow occurs here
6 |     *a += 1;
  |     ------- first borrow later used here

error: aborting due to previous error

For more information about this error, try `rustc --explain E0499`.
```

**In Harsh:** the refusal points at the `.hrs` lines of both borrows.

## 3.2 Aliasing

*Original: [Aliasing](https://doc.rust-lang.org/nomicon/aliasing.html)*

Why the rule matters: because a `&mut` is never aliased, the compiler may
assume a write through it changes nothing else, and a read through a `&`
sees a value that cannot change underneath it — and optimise accordingly.
Unsafe code that creates two live `&mut` to one place breaks that assumption:
undefined behaviour, whether or not it ever writes through both.

```rust harsh
fn main$:
    let mut xs = [1, 2, 3, 4, 5, 6]
    // Two `&mut` into one array -- to disjoint halves, so sound.
    let (left, right) = xs <- split_at_mut 3
    left[0] = 10
    right[0] = 40
    println! "{:?}" xs
```

```text
[10, 2, 3, 40, 5, 6]
```

**In Harsh:** `split_at_mut` is the standard way to get two `&mut` into one
slice — to *disjoint* halves, which is what makes it sound.

## 3.3 Lifetimes

*Original: [Lifetimes](https://doc.rust-lang.org/nomicon/lifetimes.html)*

A lifetime is the region of code for which a reference must stay valid.
Within a function the compiler infers them; across a function's signature you
name them, saying which inputs an output borrows from.

```rust harsh
// The output borrows from both inputs: it lives as long as the shorter.
fn longest<'a> (x: &'a str) (y: &'a str) -> &'a str:
    if x <- len$ >= y <- len$: x else: y

fn main$:
    let a = String.from "a long string"
    let result = do:
        let b = String.from "short"
        longest (&a) (&b) <- to_string$
    println! "{result}"
```

```text
a long string
```

**In Harsh:** lifetimes are written as in Rust, `fn longest<'a> (x: &'a str)
(y: &'a str) -> &'a str`.

## 3.4 Limits of Lifetimes

*Original: [Limits of Lifetimes](https://doc.rust-lang.org/nomicon/lifetime-mismatch.html)*

The borrow checker is conservative: some correct programs are refused,
because it reasons about a borrow's whole scope rather than every path. The
classic case: a borrow returned on one path of a function stays alive on the
others.

```rust harsh
use std.collections.HashMap

// Correct, yet refused: the borrow returned on the first path is taken to
// last for the whole function, so the insert conflicts with it.
fn get_default<'m> (map: &'m mut HashMap<i32, String>) (key: i32) -> &'m mut String:
    match map <- get_mut (&key)\
        Some value => value
        None =>
            let _ = map <- insert key (String.new$)
            map <- get_mut (&key) <- unwrap$

fn main$:
    let mut m = HashMap.new$
    println! "{}" (get_default (&mut m) 1)
```

```text
error[E0499]: cannot borrow `*map` as mutable more than once at a time
  --> limits.hrs:9:21
   |
5  |   fn get_default<'m>(map: &'m mut HashMap<i32, String>, key: i32) -> &'m mut String {
   |                  -- lifetime `'m` defined here
6  |       match map.get_mut(&key) {
   |       -     --- first mutable borrow occurs here
   |  _____|
   | |
7  | |         Some(value) => value,
8  | |         None => {
9  | |             let _ = map.insert(key, String::new());
   | |                     ^^^ second mutable borrow occurs here
10 | |             map.get_mut(&key).unwrap()
11 | |         },
12 | |     }
   | |_____- returning this value requires that `*map` is borrowed for `'m`

error[E0499]: cannot borrow `*map` as mutable more than once at a time
  --> limits.hrs:10:13
   |
5  |   fn get_default<'m>(map: &'m mut HashMap<i32, String>, key: i32) -> &'m mut String {
   |                  -- lifetime `'m` defined here
6  |       match map.get_mut(&key) {
   |       -     --- first mutable borrow occurs here
   |  _____|
   | |
7  | |         Some(value) => value,
8  | |         None => {
9  | |             let _ = map.insert(key, String::new());
10 | |             map.get_mut(&key).unwrap()
   | |             ^^^ second mutable borrow occurs here
11 | |         },
12 | |     }
   | |_____- returning this value requires that `*map` is borrowed for `'m`

error: aborting due to 2 previous errors

For more information about this error, try `rustc --explain E0499`.
```

**In Harsh:** rustc's refusal points at the `.hrs` lines; the usual way
around it is to look up twice, as the next program does.

```rust harsh
use std.collections.HashMap

// Looking up twice: no borrow outlives a path.
fn get_default<'m> (map: &'m mut HashMap<i32, String>) (key: i32) -> &'m mut String:
    if !map <- contains_key (&key):
        let _ = map <- insert key (String.from "default")
    map <- get_mut (&key) <- unwrap$

fn main$:
    let mut m = HashMap.new$
    println! "{}" (get_default (&mut m) 1)
```

```text
default
```

## 3.5 Lifetime Elision

*Original: [Lifetime Elision](https://doc.rust-lang.org/nomicon/lifetime-elision.html)*

Most signatures need no lifetime names: with one reference input, the output
borrows from it; with `&self`, the output borrows from `self`. The compiler
fills them in by these rules; you write them only when the rules do not
decide.

```rust harsh
struct Doc
    text: String

impl Doc
    // Elided: the output borrows from `self`.
    fn first_word (&self) -> &str:
        self <- text <- split_whitespace$ <- next$ <- unwrap_or ""
    // The same signature with its lifetimes written.
    fn first_word_explicit<'a> (&'a self) -> &'a str:
        self <- text <- split_whitespace$ <- next$ <- unwrap_or ""

fn main$:
    let d = Doc\ text = String.from "hello harsh world"
    println! "{} {}" (d <- first_word$) (d <- first_word_explicit$)
```

```text
hello hello
```

**In Harsh:** the elided and the explicit forms side by side, identical to
the compiler.

## 3.6 Unbounded Lifetimes

*Original: [Unbounded Lifetimes](https://doc.rust-lang.org/nomicon/unbounded-lifetimes.html)*

Dereferencing a raw pointer produces a reference whose lifetime nothing
constrains — it becomes whatever the context asks for, even `'static`. Bound it
at once, by returning it from a function whose signature ties it to an input.

```rust harsh
struct Owner
    value: i32

// The raw pointer's reference is bounded by the signature: it lives no
// longer than `owner`.
fn get<'a> (owner: &'a Owner) -> &'a i32:
    let p: *const i32 = &owner <- value
    unsafe: &*p

fn main$:
    let o = Owner\ value = 42
    println! "{}" (get (&o))
```

```text
42
```

**In Harsh:** the signature does the bounding, `fn get<'a> (owner: &'a Owner)
-> &'a i32`; the unsafe line inside stays one line.

## 3.7 Higher-Rank Trait Bounds

*Original: [Higher-Rank Trait Bounds](https://doc.rust-lang.org/nomicon/hrtb.html)*

A closure taking a reference must work for *every* lifetime its caller might
pass, not one chosen in advance: `for<'a> Fn(&'a T) -> &'a U`. Usually this is
implied by the `Fn(&T) -> &U` sugar, which is why it is rarely written.

```rust harsh
// The closure must work for every lifetime of the references it is given:
// a `for<'a>` bound, implied by the sugar.
fn apply_all (f: impl Fn (&str) -> usize) (words: &[String]) -> Vec<usize>:
    words <- iter$ <- map (|w| f w) <- collect$

fn main$:
    let words = vec! (String.from "harsh") (String.from "rust")
    println! "{:?}" (apply_all (|w| w <- len$) (&words))
```

```text
[5, 4]
```

**In Harsh:** the sugar, `impl Fn (&str) -> usize`, carries the `for<'a>` as
in Rust.

## 3.8 Subtyping and Variance

*Original: [Subtyping and Variance](https://doc.rust-lang.org/nomicon/subtyping.html)*

A longer lifetime can stand in for a shorter one: `&'static str` is usable
wherever a `&'a str` is expected. How that extends through a type is its
*variance*: `&'a T` is covariant in `'a` and `T`; `&'a mut T` is invariant in
`T`; `fn(T)` is contravariant in `T`. Getting variance wrong in unsafe code —
through a raw-pointer field, say — lets a short lifetime be stretched.

```rust harsh
// Expects references that live for some `'a`...
fn pick<'a> (a: &'a str) (b: &'a str) (first: bool) -> &'a str:
    if first: a else: b

fn main$:
    let owned = String.from "local"
    // ...and a `&'static str` stands in: 'static outlives 'a (covariance).
    let s: &'static str = "static"
    println! "{} {}" (pick s (&owned) true) (pick s (&owned) false)
```

```text
static local
```

**In Harsh:** nothing to change; variance is the type system's.

## 3.9 Drop Check

*Original: [Drop Check](https://doc.rust-lang.org/nomicon/dropck.html)*

A value with a destructor may use its borrows when it is dropped, so those
borrows must strictly outlive it. The compiler checks this — *drop check* —
and refuses a program where a borrowed value would be dropped first. The
standard library escapes this check for its collections with an unstable
attribute, `#[may_dangle]`, promising their destructors only drop what they
hold.

```rust harsh
struct Inspector<'a>
    seen: &'a String

impl<'a> Drop for Inspector<'a>
    fn drop (&mut self):
        // The destructor uses the borrow: it must still be valid.
        println! "inspecting {}" (self <- seen)

fn main$:
    let inspector
    let value = String.from "treasure"
    // Refused: `value` is dropped before `inspector`, whose drop reads it.
    inspector = Inspector\ seen = &value
    println! "{}" (inspector <- seen)
```

```text
error[E0597]: `value` does not live long enough
  --> dropck.hrs:17:15
   |
14 |     let value = String::from("treasure");
   |         ----- binding `value` declared here
...
17 |         seen: &value,
   |               ^^^^^^ borrowed value does not live long enough
...
20 | }
   | -
   | |
   | `value` dropped here while still borrowed
   | borrow might be used here, when `inspector` is dropped and runs the `Drop` code for type `Inspector`
   |
   = note: values in a scope are dropped in the opposite order they are defined

error: aborting due to previous error

For more information about this error, try `rustc --explain E0597`.
```

**In Harsh:** declaring the borrowed value first, so it is dropped last, is
the fix, as in Rust.

## 3.10 PhantomData

*Original: [PhantomData](https://doc.rust-lang.org/nomicon/phantom-data.html)*

A struct holding only raw pointers tells the compiler nothing about what it
owns or borrows. `PhantomData<T>` — a zero-sized field — says "act as if this
struct held a `T`", restoring the right variance, drop checking and auto
traits.

```rust harsh
use std.marker.PhantomData

// Only a raw pointer inside: PhantomData says "this borrows a T for 'a".
struct Slice<'a, T>
    start: *const T
    len: usize
    marker: PhantomData<&'a T>

impl<'a, T: Copy> Slice<'a, T>
    fn new (xs: &'a [T]) -> Self:
        Self\ start = xs <- as_ptr$, len = xs <- len$, marker = PhantomData
    fn get (&self) (i: usize) -> Option<T>:
        if i < self <- len:
            // Sound: in bounds, and PhantomData keeps `xs` borrowed.
            Some (unsafe: *self <- start <- add i)
        else:
            None

fn main$:
    let data = vec! 1 2 3
    let s = Slice.new (&data)
    println! "{:?} {:?}" (s <- get 2) (s <- get 3)
```

```text
Some(3) None
```

**In Harsh:** `PhantomData` is a value like any unit struct: `marker =
PhantomData` in the literal.

## 3.11 Splitting Borrows

*Original: [Splitting Borrows](https://doc.rust-lang.org/nomicon/borrow-splitting.html)*

The borrow checker understands a struct's fields as separate places — two
`&mut` to two fields are fine — but not a slice's halves, which is why
`split_at_mut` exists, with a little unsafe code inside.

```rust harsh
struct Point
    x: i32
    y: i32

fn bump (a: &mut i32) (b: &mut i32):
    *a += 1
    *b += 10

fn main$:
    let mut p = Point\ x = 0, y = 0
    // Two fields: two disjoint places, two `&mut` at once.
    bump (&mut p <- x) (&mut p <- y)
    println! "{} {}" (p <- x) (p <- y)
```

```text
1 10
```

**In Harsh:** field access is `<-`, so `&mut p <- x` and `&mut p <- y` are the
two disjoint borrows.
