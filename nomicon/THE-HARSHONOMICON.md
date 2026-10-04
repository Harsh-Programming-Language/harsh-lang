# The Harshonomicon

*A Harsh companion to [The Rustonomicon](https://doc.rust-lang.org/nomicon/) — the dark arts of unsafe Rust,
in the same order, with every program written in Harsh and built and run to
produce the output you see.*

*The Rustonomicon* is the book for the moment safe Rust is not enough: when
you write a data structure the borrow checker cannot follow, call into C, or
need to know exactly how a value sits in memory. It explains what `unsafe`
allows, what it does not, and the contracts you take on when you use it.

Harsh is Rust with another layout, and `unsafe` in Harsh is `unsafe` in Rust:
the same rules, the same undefined behaviour, the same compiler checking what
it can. Nothing in this book is a Harsh rule. What the companion adds is how
unsafe code reads in Harsh — `unsafe:` opening a block like any keyword, raw
pointers used with `<-` like any value — in programs you can run, each short,
each showing one rule at work.

Its chapters and sections are numbered as the original's, so a section here
and its original share a number. **This companion is shorter than its
original, on purpose.** Much of *The
Rustonomicon*'s value is its careful explanation of why each rule exists;
each page here states the rule in a few sentences of its own and links to the
original section. Before you write unsafe code for real, read the original.

Every program in this book is **well-defined**: none relies on undefined
behaviour, even to illustrate it. Where a page is about something that must
not be done, it shows the compiler refusing it, or the safe form.

*The Rustonomicon* is by the Rust Project developers, licensed under the
Apache License 2.0; this companion is under the Mozilla Public License 2.0, as
Harsh is. See ATTRIBUTION.md.


# Meet Safe and Unsafe

Safe Rust guarantees there is no undefined behaviour: no dangling reference,
no data race, no read of uninitialised memory. Unsafe Rust is the same
language with a few more abilities, whose correctness the compiler can no
longer check — you promise it instead. The original chapter:
[Meet Safe and Unsafe](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html).

## 1.1 How Safe and Unsafe Interact

*Original: [How Safe and Unsafe Interact](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html)*

`unsafe` marks a contract. On a function, it says the caller must uphold
conditions the compiler cannot check; on a block, it says *you* have checked
them. On a trait, it says implementers must uphold an invariant; on an
`impl`, that you have. Safe code must never be able to cause undefined
behaviour, however it calls a safe function — that is the safe function's
author's job.

```rust harsh
// A safe function around an unsafe operation: the check makes it sound.
fn get (xs: &[i32]) (i: usize) -> Option<i32>:
    if i < xs <- len$:
        // Sound: `i` was checked against the length just above.
        Some (unsafe: *xs <- get_unchecked i)
    else:
        None

// An unsafe function: the caller promises what the function cannot check.
unsafe fn first_unchecked (xs: &[i32]) -> i32:
    *xs <- get_unchecked 0

fn main$:
    let xs = [10, 20, 30]
    println! "{:?} {:?}" (get (&xs) 1) (get (&xs) 9)
    // We promise: `xs` is not empty.
    println! "{}" (unsafe: first_unchecked (&xs))
```

```text
Some(20) None
10
```

**In Harsh:** `unsafe fn` is declared like any function; `unsafe:` opens a
block, and a short one fits on its line: `unsafe: *xs <- get_unchecked i`.

## 1.2 What Unsafe Can Do

*Original: [What Unsafe Can Do](https://doc.rust-lang.org/nomicon/what-unsafe-does.html)*

Only a few things: dereference a raw pointer; call an `unsafe` function
(including C functions); implement an `unsafe` trait; access or modify a
`static mut`; access a `union`'s fields. Everything else — the borrow
checker, the type system — still applies inside `unsafe`. What unsafe code
must never do is the original's other list, of undefined behaviours: a
dangling or unaligned dereference, breaking the aliasing rules, a data race,
producing an invalid value (a `bool` that is not 0 or 1, an enum with no
such variant), among others.

```rust harsh
static mut COUNTER: u32 = 0

union IntOrFloat
    i: u32
    f: f32

fn main$:
    // 1. Dereference a raw pointer.
    let x = 5
    let p = &x as *const i32
    println! "{}" (unsafe: *p)
    // 2. Call an unsafe function.
    let v = vec! 1 2 3
    println! "{}" (unsafe: *v <- get_unchecked 2)
    // 3. Access a `static mut` (one thread only here).
    unsafe:
        COUNTER += 1
        println! "{}" COUNTER
    // 4. Read a union's field.
    let u = IntOrFloat\ f = 1.0
    println! "{:#x}" (unsafe: u <- i)
```

```text
5
3
1
0x3f800000
```

**In Harsh:** each power reads as in Rust; the raw-pointer dereference is
`*p`, isolated in parentheses where it is an argument, `(*p)`. Mind its
precedence: a prefix operator takes the whole chain after it, so `*p <- field`
dereferences the *field*; to reach through the pointer, write `(*p) <-
field`. The Book's appendix *Precedence* has the full table.

## 1.3 Working with Unsafe

*Original: [Working with Unsafe](https://doc.rust-lang.org/nomicon/working-with-unsafe.html)*

Unsafe code depends on invariants that *safe* code around it can break: a
`Vec`'s safe method that changes `len` wrongly is as dangerous as the unsafe
read that trusts it. So the boundary of safety is the **module**: keep the
fields that unsafe code relies on private, and let only the module's own
code touch them.

```rust harsh
mod stack
    // `len` is private: only this module's code can change it, so the
    // unsafe read below can trust it.
    pub struct Stack
        items: [i32; 8]
        len: usize

    impl Stack
        pub fn new$ -> Self:
            Self\ items = [0; 8], len = 0

        pub fn push (&mut self) (x: i32) -> bool:
            if self <- len == 8:
                return false
            self <- items[self <- len] = x
            self <- len += 1
            true

        pub fn top (&self) -> Option<i32>:
            if self <- len == 0:
                None
            else:
                // Sound because `len` is always <= 8, which only this
                // module can break.
                Some (unsafe: *self <- items <- get_unchecked (self <- len - 1))

fn main$:
    let mut s = stack.Stack.new$
    s <- push 1
    s <- push 2
    println! "{:?}" (s <- top$)
```

```text
Some(2)
```

**In Harsh:** the module and its private fields are the same `mod` and the
same absence of `pub`.


# Data Layout

How values sit in memory: their size, their alignment, the order of their
fields. Unsafe code that reads or writes memory directly depends on it. The
original chapter: [Data Representation](https://doc.rust-lang.org/nomicon/data.html).

## 2.1 repr(Rust)

*Original: [repr(Rust)](https://doc.rust-lang.org/nomicon/repr-rust.html)*

Every type has a size and an alignment. By default, Rust may reorder a
struct's fields to reduce padding, and promises nothing else about their
order: two structs with the same fields may be laid out differently.

```rust harsh
use std.mem

// Written loosely: a byte, a u32, a byte.
struct Loose
    a: u8
    b: u32
    c: u8

// The same fields, in C's order.
#[repr C]
struct InOrder
    a: u8
    b: u32
    c: u8

fn main$:
    // Rust may reorder the fields to pack them; C's order pads.
    println!
        "Loose:   size {} align {}"
        (mem.size_of<Loose>$)
        (mem.align_of<Loose>$)
    println!
        "InOrder: size {} align {}"
        (mem.size_of<InOrder>$)
        (mem.align_of<InOrder>$)
```

```text
Loose:   size 8 align 4
InOrder: size 12 align 4
```

**In Harsh:** `mem.size_of<Loose>$` — the generic list follows the name, and
the transpiler writes Rust's turbofish.

## 2.2 Exotically Sized Types

*Original: [Exotically Sized Types](https://doc.rust-lang.org/nomicon/exotic-sizes.html)*

Not every type has a known, non-zero size. *Dynamically sized* types — `[T]`,
`str`, `dyn Trait` — live behind pointers that carry their length or vtable,
so those pointers are twice as wide. *Zero-sized* types take no space at all;
*empty* types, like an enum with no variants, cannot even be built.

```rust harsh
use std.mem

struct Nothing
enum Void

trait Shape
    fn area (&self) -> f64

fn main$:
    // Zero-sized: no space at all.
    println! "()      {}" (mem.size_of<()>$)
    println! "Nothing {}" (mem.size_of<Nothing>$)
    println! "Void    {}" (mem.size_of<Void>$)
    // A pointer to a dynamically sized type carries its length or vtable.
    println! "&u8        {}" (mem.size_of<&u8>$)
    println! "&[u8]      {}" (mem.size_of<&[u8]>$)
    println! "&str       {}" (mem.size_of<&str>$)
    println! "&dyn Shape {}" (mem.size_of<&dyn Shape>$)
```

```text
()      0
Nothing 0
Void    0
&u8        8
&[u8]      16
&str       16
&dyn Shape 16
```

**In Harsh:** an empty enum is its header alone, `enum Void`, like any
declaration with nothing beneath.

## 2.3 Other reprs

*Original: [Other reprs](https://doc.rust-lang.org/nomicon/other-reprs.html)*

`#[repr(C)]` lays out fields in order, as C does — required for FFI.
`#[repr(u8)]` (and the other integers) fixes an enum's discriminant.
`#[repr(transparent)]` makes a one-field wrapper laid out exactly as its
field. `#[repr(packed)]` (or `packed(n)`) removes padding, at the cost of
unaligned fields; `#[repr(align(n))]` raises a type's alignment.

```rust harsh
use std.mem

// A fixed discriminant, one byte.
#[repr u8]
#[derive Clone Copy Debug]
enum Color
    Red = 1
    Green = 2

// A wrapper laid out exactly as its field.
#[repr transparent]
struct Meters f64

// No padding: the u32 may be unaligned.
#[repr C packed]
struct Packed
    a: u8
    b: u32

fn main$:
    println! "{} {}" (mem.size_of<Color>$) (Color.Green as u8)
    println! "{} {}" (mem.size_of<Meters>$) (mem.size_of<f64>$)
    println! "{}" (mem.size_of<Packed>$)
```

```text
1 2
8 8
5
```

**In Harsh:** attribute arguments juxtapose, `#[repr C]`, `#[repr u8]`,
`#[repr transparent]`.


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
 --> references.hrs:6:13
  |
4 |     let a = &mut total;
  |             ---------- first mutable borrow occurs here
5 |     // Refused: `total` is already borrowed mutably by `a`.
6 |     let b = &mut total;
  |             ^^^^^^^^^^ second mutable borrow occurs here
7 |     *a += 1;
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
  --> limits.hrs:10:21
   |
6  |   fn get_default<'m>(map: &'m mut HashMap<i32, String>, key: i32) -> &'m mut String {
   |                  -- lifetime `'m` defined here
7  |       match map.get_mut(&key) {
   |       -     --- first mutable borrow occurs here
   |  _____|
   | |
8  | |         Some(value) => value,
9  | |         None => {
10 | |             let _ = map.insert(key, String::new());
   | |                     ^^^ second mutable borrow occurs here
11 | |             map.get_mut(&key).unwrap()
12 | |         },
13 | |     }
   | |_____- returning this value requires that `*map` is borrowed for `'m`

error[E0499]: cannot borrow `*map` as mutable more than once at a time
  --> limits.hrs:11:13
   |
6  |   fn get_default<'m>(map: &'m mut HashMap<i32, String>, key: i32) -> &'m mut String {
   |                  -- lifetime `'m` defined here
7  |       match map.get_mut(&key) {
   |       -     --- first mutable borrow occurs here
   |  _____|
   | |
8  | |         Some(value) => value,
9  | |         None => {
10 | |             let _ = map.insert(key, String::new());
11 | |             map.get_mut(&key).unwrap()
   | |             ^^^ second mutable borrow occurs here
12 | |         },
13 | |     }
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
        self <- text
             <- split_whitespace$
             <- next$
             <- unwrap_or ""
    // The same signature with its lifetimes written.
    fn first_word_explicit<'a> (&'a self) -> &'a str:
        self <- text
             <- split_whitespace$
             <- next$
             <- unwrap_or ""

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
    words <- iter$
          <- map (|w| f w)
          <- collect$

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
  --> dropck.hrs:18:15
   |
15 |     let value = String::from("treasure");
   |         ----- binding `value` declared here
...
18 |         seen: &value,
   |               ^^^^^^ borrowed value does not live long enough
...
21 | }
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
        Self\
            start = xs <- as_ptr$
            len = xs <- len$
            marker = PhantomData
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


# Type Conversions

The ways a value changes type: implicitly, with `as`, or by reinterpreting
its bits. The original chapter: [Type Conversions](https://doc.rust-lang.org/nomicon/conversions.html).

## 4.1 Coercions

*Original: [Coercions](https://doc.rust-lang.org/nomicon/coercions.html)*

Some conversions happen on their own, at specific places: `&mut T` to `&T`,
`&String` to `&str` through `Deref`, an array to a slice, a concrete type to
`dyn Trait`. They are never applied to satisfy a trait bound.

```rust harsh
use std.fmt.Display

fn count (s: &str) -> usize:
    s <- len$

fn total (xs: &[i32]) -> i32:
    xs <- iter$ <- sum$

fn main$:
    let owned = String.from "harsh"
    // &String to &str, through Deref.
    println! "{}" (count (&owned))
    // An array to a slice.
    println! "{}" (total (&[1, 2, 3]))
    // A concrete type to a trait object.
    let shown: &dyn Display = &42
    println! "{shown}"
```

```text
5
6
42
```

**In Harsh:** the same places, the same conversions.

## 4.2 The Dot Operator

*Original: [The Dot Operator](https://doc.rust-lang.org/nomicon/dot-operator.html)*

A method call finds its method by trying the receiver, then `&receiver`,
then `&mut receiver`, then dereferencing and trying again. This is
convenient and occasionally surprising: `x.clone()` on a `&T` where `T` is not
`Clone` clones the *reference*.

```rust harsh
#[derive Clone Debug]
struct Named
    name: String

struct NotClone

fn main$:
    let n = Named\ name = String.from "a"
    let r = &n
    // `Named` is Clone: r <- clone$ is a Named.
    let copy: Named = r <- clone$
    println! "{:?}" copy
    // `NotClone` is not: r2 <- clone$ clones the reference itself.
    let x = NotClone
    let r2 = &x
    let also_ref: &NotClone = r2 <- clone$
    println! "{}" (std.ptr.eq r2 also_ref)
```

```text
Named { name: "a" }
true
```

**In Harsh:** `<-` is the dot and follows the same search. Writing the trait's
function, `T.clone x`, says which one you mean — found while writing *Harsh
Design Patterns*, where it mattered.

## 4.3 Casts

*Original: [Casts](https://doc.rust-lang.org/nomicon/casts.html)*

`as` converts between primitive types explicitly. Numeric casts truncate or
sign-extend; a float to an integer saturates (and NaN becomes 0); pointer
casts change the type the pointer claims. None of them is undefined
behaviour — but a pointer cast is only as good as what it points to.

```rust harsh
fn main$:
    // Integer to a smaller integer: truncates. (A literal that does not fit
    // is refused outright, by the `overflowing_literals` lint.)
    let big: i32 = 300
    println! "{}" (big as u8)
    // Signed to unsigned: the same bits.
    println! "{}" (-1i32 as u32)
    // Float to integer: saturates; NaN becomes 0.
    println! "{} {} {}" (3.9f64 as i32) (1e20f64 as i32) (f64.NAN as i32)
    // A reference to a raw pointer, and to an integer address.
    let x = 7
    let p = &x as *const i32
    println! "{}" ((p as usize) % (std.mem.align_of<i32>$))
```

```text
44
4294967295
3 2147483647 0
0
```

**In Harsh:** `as` binds as in Rust, `(big as u8)`.

## 4.4 Transmutes

*Original: [Transmutes](https://doc.rust-lang.org/nomicon/transmutes.html)*

`mem::transmute` reinterprets the bits of one type as another of the same
size. It is the most dangerous tool there is: almost every use has a safer
alternative (`f32::to_bits`, `from_ne_bytes`, pointer casts), which the next
program compares it with.

```rust harsh
fn main$:
    let x = 1.5f32
    // The dangerous tool...
    let bits: u32 = unsafe: std.mem.transmute x
    // ...and the safe one that does the same.
    println! "{:#x} {:#x}" bits (x <- to_bits$)
    let bytes = 0x12345678u32 <- to_be_bytes$
    println! "{:x?}" bytes
    println! "{:#x}" (u32.from_be_bytes bytes)
```

```text
0x3fc00000 0x3fc00000
[12, 34, 56, 78]
0x12345678
```

**In Harsh:** `mem.transmute<f32, u32> x` would need the turbofish before an
argument, which the transpiler does not yet write — one more reason to reach
for `to_bits`. The program names the types in a binding instead.


# Uninitialized Memory

Memory that has been allocated but not yet given a value. Reading it is
undefined behaviour; safe Rust makes that impossible, and unsafe code must
keep it so. The original chapter:
[Working With Uninitialized Memory](https://doc.rust-lang.org/nomicon/uninitialized.html).

## 5.1 Checked

*Original: [Checked](https://doc.rust-lang.org/nomicon/checked-uninit.html)*

A variable may be declared before it is given a value, but the compiler
tracks every path and refuses a read that might come first.

```rust harsh
fn main$:
    let x: i32
    let ready = std.env.args$ <- count$ > 5
    if ready:
        x = 1
    // Refused: on the other path, `x` was never given a value.
    println! "{x}"
```

```text
error[E0381]: used binding `x` is possibly-uninitialized
 --> checked.hrs:9:15
  |
3 |     let x: i32;
  |         - binding declared here but left uninitialized
...
6 |         x = 1
  |         ----- binding initialized here in some conditions
...
9 |     println!("{x}")
  |               ^^^ `x` used here but it is possibly-uninitialized
  |
  = note: this error originates in the macro `$crate::format_args_nl` which comes from the expansion of the macro `println` (in Nightly builds, run with -Z macro-backtrace for more info)

error: aborting due to previous error

For more information about this error, try `rustc --explain E0381`.
```

**In Harsh:** the error points at the `.hrs` line of the read.

## 5.2 Drop Flags

*Original: [Drop Flags](https://doc.rust-lang.org/nomicon/drop-flags.html)*

When a value is initialised on some paths only, or moved out on some, the
compiler keeps a hidden *drop flag* to know at run time whether to drop it.

```rust harsh
struct Loud
    name: &'static str

impl Drop for Loud
    fn drop (&mut self):
        println! "drop {}" (self <- name)

fn main$:
    let decide = std.env.args$ <- count$ < 5
    let a
    if decide:
        a = Loud\ name = "a"
    let b = Loud\ name = "b"
    let c = Loud\ name = "c"
    // `c` moved out on this path only: its drop flag decides at the end.
    if decide:
        drop c
    println! "end of main"
    let _keep = &b
```

```text
drop c
end of main
drop b
drop a
```

**In Harsh:** the same; the output shows which values were dropped, and when.

## 5.3 Unchecked

*Original: [Unchecked](https://doc.rust-lang.org/nomicon/unchecked-uninit.html)*

To initialise memory piece by piece — an array element by element, a buffer
from C — use `MaybeUninit<T>`: it holds possibly-uninitialised memory, and
`assume_init` is your promise that every part is now valid.

```rust harsh
use std.mem.MaybeUninit

fn main$:
    // Memory for four Strings, not yet any String in it.
    let mut slots: [MaybeUninit<String>; 4] = std.array.from_fn (|_| MaybeUninit.uninit$)
    for (i, slot) in slots <- iter_mut$ <- enumerate$:
        let _ = slot <- write (format! "item {i}")
    // Our promise: every slot was written.
    let items: [String; 4] = slots <- map (|s| unsafe: s <- assume_init$)
    println! "{:?}" items
```

```text
["item 0", "item 1", "item 2", "item 3"]
```

**In Harsh:** each write is `unsafe:`-free here, since `MaybeUninit.write`
is safe; only the final `assume_init` needs the promise.


# Ownership Based Resource Management

A value owns its resources — memory, files, locks — and releases them when it
is dropped. It is the pattern everything else in Rust rests on. The original
chapter: [OBRM](https://doc.rust-lang.org/nomicon/obrm.html).

## 6.1 Constructors

*Original: [Constructors](https://doc.rust-lang.org/nomicon/constructors.html)*

There is exactly one way to create a value: name its type and give every
field. `new` functions, `Default`, `Clone` are ordinary code built on that one
way; nothing runs implicitly — no copy constructor, no move constructor. A
move is always a plain copy of the bytes, after which the source is no longer
used.

There is no program on this page: there is nothing hidden to show.

## 6.2 Destructors

*Original: [Destructors](https://doc.rust-lang.org/nomicon/destructors.html)*

`Drop::drop` runs when a value goes out of scope, then its fields are dropped
in declaration order; local variables are dropped in reverse order of
declaration. `mem::forget` and `ManuallyDrop` stop a destructor from running.

```rust harsh
use std.mem.ManuallyDrop

struct Loud
    name: &'static str

impl Drop for Loud
    fn drop (&mut self):
        println! "drop {}" (self <- name)

struct Pair
    first: Loud
    second: Loud

fn main$:
    let _a = Loud\ name = "local a"
    let _b = Loud\ name = "local b"
    let _p =
        Pair\
            first = Loud\ name = "field first"
            second = Loud\ name = "field second"
    // Never dropped: forgotten, and kept from dropping.
    std.mem.forget (Loud\ name = "forgotten")
    let _m = ManuallyDrop.new (Loud\ name = "manual")
    println! "end of main"
```

```text
end of main
drop field first
drop field second
drop local b
drop local a
```

**In Harsh:** the order of the output is the order the rules predict.

## 6.3 Leaking

*Original: [Leaking](https://doc.rust-lang.org/nomicon/leaking.html)*

Leaking is *safe*: `mem::forget` does it on purpose, and two `Rc`s pointing at
each other do it by accident. So unsafe code must never rely on a destructor
running for soundness — only for tidiness. The original's examples show what
that costs: `Vec::drain` sets the length to zero before it starts, so a
forgotten `Drain` leaks rather than exposes moved-out elements; and the early
`thread::scoped` API was removed because a forgotten guard let a thread
outlive what it borrowed — which `thread::scope`, used in this book, avoids.

```rust harsh
use std.cell.RefCell
use std.rc.( Rc, Weak)

struct Node
    next: RefCell<Option<Rc<Node>>>
    back: RefCell<Weak<Node>>

fn main$:
    // A cycle of strong pointers: neither count ever reaches zero.
    let a = Rc.new (Node\ next = RefCell.new None, back = RefCell.new (Weak.new$))
    let b = Rc.new (Node\ next = RefCell.new (Some (a <- clone$)), back = RefCell.new (Weak.new$))
    *a <- next <- borrow_mut$ = Some (b <- clone$)
    println!
        "strong: a {} b {} -- a leak when both go"
        (Rc.strong_count (&a))
        (Rc.strong_count (&b))
    // Break it: a Weak pointer back does not keep its target alive.
    *a <- next <- borrow_mut$ = None
    *a <- back <- borrow_mut$ = Rc.downgrade (&b)
    println!
        "strong: a {} b {}"
        (Rc.strong_count (&a))
        (Rc.strong_count (&b))
```

```text
strong: a 2 b 2 -- a leak when both go
strong: a 2 b 1
```

**In Harsh:** `Weak` breaks the cycle; the counts show it.


# Unwinding

A panic unwinds the stack, running destructors as it goes. Unsafe code in
the middle of changing an invariant must leave things sound if a panic
interrupts it. The original chapter: [Unwinding](https://doc.rust-lang.org/nomicon/unwinding.html).

## 7.1 Exception Safety

*Original: [Exception Safety](https://doc.rust-lang.org/nomicon/exception-safety.html)*

Code that calls something which may panic must be *exception safe*: when the
panic unwinds through it, every invariant unsafe code depends on must still
hold. The usual tool is a guard whose destructor restores the invariant — as
the standard `BinaryHeap::sift_up` does with a *hole* that is always filled
back in, even if a comparison panics.

```rust harsh
use std.panic

struct Restore<'a>
    flag: &'a mut bool

impl<'a> Drop for Restore<'a>
    fn drop (&mut self):
        // Runs during the unwind too: the invariant comes back.
        *self <- flag = true
        println! "invariant restored"

fn risky (flag: &mut bool):
    *flag = false
    let _guard = Restore\ flag = flag
    panic! "interrupted in the middle"

fn main$:
    // No message on stderr for the expected panic.
    panic.set_hook (Box.new (|_| ()))
    let mut consistent = true
    let result = panic.catch_unwind (panic.AssertUnwindSafe (|| risky (&mut consistent)))
    println!
        "panicked: {}, consistent: {}"
        (result <- is_err$)
        consistent
```

```text
invariant restored
panicked: true, consistent: true
```

**In Harsh:** the guard is a struct with a `Drop`; the output shows it
running during the unwind.

## 7.2 Poisoning

*Original: [Poisoning](https://doc.rust-lang.org/nomicon/poisoning.html)*

A `Mutex` whose holder panicked is *poisoned*: its data may be half-updated,
so the next `lock` returns an error. The data is still reachable, for code
that knows how to check it.

```rust harsh
use std.sync.( Arc, Mutex)
use std.thread

fn main$:
    std.panic.set_hook (Box.new (|_| ()))
    let data = Arc.new (Mutex.new (vec! 1 2 3))
    let d = data <- clone$
    let _ =
        thread.spawn (
            move ||:
                let mut v = d <- lock$ <- unwrap$
                v <- push 4
                panic! "the holder panicked"
        ) <- join$
    println! "poisoned: {}" (data <- is_poisoned$)
    let result = data <- lock$
    let shown =
        match result\
            Ok v => format! "ok {:?}" (*v)
            Err poisoned =>
                format! "recovered {:?}" (*poisoned <- into_inner$)
    println! "{shown}"
```

```text
poisoned: true
recovered [1, 2, 3, 4]
```

**In Harsh:** the panicking thread's closure is a `move ||:` block; `lock`'s
result is matched like any other.


# Concurrency

Safe Rust makes data races impossible; it does not make every race
impossible. Unsafe code that shares memory between threads must uphold the
same guarantee. The original chapter:
[Concurrency and Parallelism](https://doc.rust-lang.org/nomicon/concurrency.html).

## 8.1 Races

*Original: [Races](https://doc.rust-lang.org/nomicon/races.html)*

A *data race* — two threads touching the same memory unsynchronised, at least
one writing — is undefined behaviour, and safe Rust prevents it. A *race
condition* — a result depending on timing — is only a bug, and safe Rust
allows it. Atomics and locks are how threads share memory without a data race.

```rust harsh
use std.sync.atomic.( AtomicUsize, Ordering)
use std.thread

fn main$:
    let counter = AtomicUsize.new 0
    thread.scope (|s|:
        for _ in 0..4:
            let _ = s <- spawn (||:
                for _ in 0..1000:
                    let _ = counter <- fetch_add 1 Ordering.Relaxed
            )
    )
    // Four threads, no data race: always 4000.
    println! "{}" (counter <- load Ordering.Relaxed)
```

```text
4000
```

**In Harsh:** the threads are spawned in a scope, so they may borrow the
counter; each adds with an atomic `fetch_add`.

## 8.2 Send and Sync

*Original: [Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)*

`Send`: a value may move to another thread. `Sync`: a reference to it may be
shared between threads. The compiler derives both from a type's fields; raw
pointers are neither, so a type built on them states its own, with an
`unsafe impl` — a promise.

```rust harsh
use std.thread

// A raw pointer is neither Send nor Sync: the wrapper promises it is safe to
// send, because the data outlives the thread and nothing else touches it.
struct Pointer (*mut i32)
unsafe impl Send for Pointer

fn main$:
    let mut value = 41
    let p = Pointer (&mut value as *mut i32)
    thread.scope (|s|:
        let _ = s <- spawn (move ||:
            let p = p
            unsafe:
                *p <- 0 += 1
        )
    )
    println! "{value}"
```

```text
42
```

**In Harsh:** `unsafe impl Send for Pointer` is one line with no body.

## 8.3 Atomics

*Original: [Atomics](https://doc.rust-lang.org/nomicon/atomics.html)*

Atomics carry an *ordering*: `Relaxed` orders nothing but the atomic itself;
`Release` on a store and `Acquire` on the load that sees it make everything
written before the store visible after the load; `SeqCst` adds a single total
order. Most synchronisation is a Release/Acquire pair.

```rust harsh
use std.sync.atomic.( AtomicBool, AtomicU64, Ordering)
use std.thread

static DATA: AtomicU64 = AtomicU64.new 0
static READY: AtomicBool = AtomicBool.new false

fn main$:
    thread.scope (|s|:
        let _ = s <- spawn (||:
            DATA <- store 42 Ordering.Relaxed
            // Release: everything before this store is visible to the
            // thread whose Acquire load sees it.
            READY <- store true Ordering.Release
        )
        let _ = s <- spawn (||:
            while !READY <- load Ordering.Acquire:
                std.hint.spin_loop$
            println! "data = {}" (DATA <- load Ordering.Relaxed)
        )
    )
```

```text
data = 42
```

**In Harsh:** `Ordering.Release`, `Ordering.Acquire` — paths with dots.


# Implementing Vec

*Original: [Implementing Vec](https://doc.rust-lang.org/nomicon/vec/vec.html)*

The original builds `Vec` in eleven steps. This companion follows them as
sections, each saying what the step does and where it is in one program —
given in full under *Final Code* — which covers the core in Harsh. Three steps
(`IntoIter`, `RawVec`, `Drain`) and zero-sized types add no new Harsh, only
more of the same care; for them, read the original.

## 9.1 Layout

*Original: [Layout](https://doc.rust-lang.org/nomicon/vec/vec-layout.html)*

A pointer, a capacity and a length. The pointer is `NonNull<T>`, never null —
so `Option<MyVec<T>>` costs nothing extra — and the type owns its `T`s, so it
is `Send` and `Sync` when they are: two `unsafe impl` lines.

## 9.2 Allocating

*Original: [Allocating](https://doc.rust-lang.org/nomicon/vec/vec-alloc.html)*

Nothing is allocated until the first push: the pointer is `NonNull::dangling`.
`grow` allocates, or reallocates to twice the capacity, with a `Layout` built
from the element's size and alignment, and aborts through
`handle_alloc_error` if the allocator fails.

## 9.3 Push and Pop

*Original: [Push and Pop](https://doc.rust-lang.org/nomicon/vec/vec-push-pop.html)*

`push` *writes* into the slot past the end (`ptr::write`: the slot holds no
value to drop); `pop` *reads* the last element out (`ptr::read`), and the slot
is uninitialised again, past `len`.

## 9.4 Deallocating

*Original: [Deallocating](https://doc.rust-lang.org/nomicon/vec/vec-dealloc.html)*

`Drop` drops the `len` elements in place, then frees the allocation with the
same layout it was made with.

## 9.5 Deref

*Original: [Deref](https://doc.rust-lang.org/nomicon/vec/vec-deref.html)*

`Deref` and `DerefMut` to `[T]`, through `slice::from_raw_parts`: every slice
method — `sort`, `iter`, indexing — becomes the vector's.

## 9.6 Insert and Remove

*Original: [Insert and Remove](https://doc.rust-lang.org/nomicon/vec/vec-insert-remove.html)*

`ptr::copy` shifts the tail by one — `memmove`, since the ranges overlap — to
open a gap for `insert` or close one after `remove`.

## 9.7 IntoIter

*Original: [IntoIter](https://doc.rust-lang.org/nomicon/vec/vec-into-iter.html)*

A by-value iterator reading elements out from both ends, which takes over the
allocation. Not in this program.

## 9.8 RawVec

*Original: [RawVec](https://doc.rust-lang.org/nomicon/vec/vec-raw.html)*

The allocation logic factored out, shared by the vector and its `IntoIter`.
Not in this program.

## 9.9 Drain

*Original: [Drain](https://doc.rust-lang.org/nomicon/vec/vec-drain.html)*

A borrowing iterator that removes elements; it sets the length to zero before
starting, so a leaked `Drain` is safe (see *Leaking*). Not in this program.

## 9.10 Handling Zero-Sized Types

*Original: [Handling Zero-Sized Types](https://doc.rust-lang.org/nomicon/vec/vec-zsts.html)*

A zero-sized `T` needs no allocation and makes pointer offsets meaningless;
the original handles it throughout. This program refuses it, with an
`assert!` in `new`.

## 9.11 Final Code

*Original: [Final Code](https://doc.rust-lang.org/nomicon/vec/vec-final.html)*

```rust harsh
use std.alloc.( self, Layout)
use std.mem
use std.ops.( Deref, DerefMut)
use std.ptr.( self, NonNull)

pub struct MyVec<T>
    ptr: NonNull<T>
    cap: usize
    len: usize

// As Vec: it owns its Ts, so it is Send and Sync when they are.
unsafe impl<T: Send> Send for MyVec<T>
unsafe impl<T: Sync> Sync for MyVec<T>

impl<T> MyVec<T>
    pub fn new$ -> Self:
        assert!
            (mem.size_of<T>$ != 0)
            "zero-sized types are not handled here"
        Self\
            ptr = NonNull.dangling$
            cap = 0
            len = 0

    fn layout (cap: usize) -> Layout:
        let size = cap * mem.size_of<T>$
        Layout.from_size_align size (mem.align_of<T>$) <- unwrap$

    fn grow (&mut self):
        let new_cap = if self <- cap == 0: 4 else: 2 * self <- cap
        let new_layout = Self.layout new_cap
        let raw = do:
            if self <- cap == 0: unsafe: alloc.alloc new_layout
            else:
                let old = self <- ptr <- as_ptr$ as *mut u8
                unsafe: alloc.realloc old (Self.layout (self <- cap)) (new_layout <- size$)
        self <- ptr =
            match NonNull.new (raw as *mut T)\
                Some p => p
                None => alloc.handle_alloc_error new_layout
        self <- cap = new_cap

    pub fn push (&mut self) (elem: T):
        if self <- len == self <- cap:
            self <- grow$
        // Written, not assigned: the slot holds no value to drop.
        // `add` on a raw pointer is itself unsafe: the chain stays inside.
        let slot = unsafe:
            self <- ptr
                 <- as_ptr$
                 <- add (self <- len)
        unsafe: ptr.write slot elem
        self <- len += 1

    pub fn pop (&mut self) -> Option<T>:
        if self <- len == 0:
            return None
        self <- len -= 1
        // Read out: the slot is now uninitialised, and past `len`.
        // `add` on a raw pointer is itself unsafe: the chain stays inside.
        let slot = unsafe:
            self <- ptr
                 <- as_ptr$
                 <- add (self <- len)
        Some (unsafe: ptr.read slot)

    pub fn insert (&mut self) (index: usize) (elem: T):
        assert! (index <= self <- len) "index out of bounds"
        if self <- len == self <- cap:
            self <- grow$
        unsafe:
            let p =
                self <- ptr
                     <- as_ptr$
                     <- add index
            // Shift the tail right by one, then write into the gap.
            ptr.copy p (p <- add 1) (self <- len - index)
            ptr.write p elem
        self <- len += 1

    pub fn remove (&mut self) (index: usize) -> T:
        assert! (index < self <- len) "index out of bounds"
        self <- len -= 1
        unsafe:
            let p =
                self <- ptr
                     <- as_ptr$
                     <- add index
            let out = ptr.read p
            ptr.copy (p <- add 1) p (self <- len - index)
            out

impl<T> Drop for MyVec<T>
    fn drop (&mut self):
        if self <- cap != 0:
            unsafe:
                // Drop the elements, then free the memory.
                let elems = ptr.slice_from_raw_parts_mut (self <- ptr <- as_ptr$) (self <- len)
                ptr.drop_in_place elems
                alloc.dealloc
                    (self <- ptr <- as_ptr$ as *mut u8)
                    (Self.layout (self <- cap))

impl<T> Deref for MyVec<T>
    type Target = [T]
    fn deref (&self) -> &[T]:
        unsafe:
            std.slice.from_raw_parts (self <- ptr <- as_ptr$) (self <- len)

impl<T> DerefMut for MyVec<T>
    fn deref_mut (&mut self) -> &mut [T]:
        unsafe:
            std.slice.from_raw_parts_mut (self <- ptr <- as_ptr$) (self <- len)

fn main$:
    let mut v = MyVec.new$
    for word in ["delta", "alpha", "charlie"]:
        v <- push (word <- to_string$)
    v <- insert 1 (String.from "bravo")
    println! "{:?} len {} cap {}" (&v[..]) (v <- len$) (v <- cap)
    // The slice's methods, through Deref and DerefMut.
    v <- sort$
    println! "{:?}" (&v[..])
    println! "removed {}, popped {:?}" (v <- remove 0) (v <- pop$)
    println! "{:?}" (&v[..])
```

```text
["delta", "bravo", "alpha", "charlie"] len 4 cap 4
["alpha", "bravo", "charlie", "delta"]
removed alpha, popped Some("delta")
["bravo", "charlie"]
```

**In Harsh:** each unsafe step is an `unsafe:` block of a line or two, which
makes the unsafe surface easy to see and to review. The layout comes from
`Layout.from_size_align` with the size and alignment named, rather than
`Layout::array::<T>` — Harsh does not yet write a turbofish before an argument.


# Implementing Arc and Mutex

*Original: [Implementing Arc and Mutex](https://doc.rust-lang.org/nomicon/arc-mutex/arc-and-mutex.html)*

The original's chapter, despite its title, implements `Arc`; this companion
follows its sections, with the program under *Final Code*.

## 10.1 Arc

*Original: [Arc](https://doc.rust-lang.org/nomicon/arc-mutex/arc.html)*

A pointer to a heap allocation holding the value and an atomic count of
owners, shared across threads.

### 10.1.1 Layout

*Original: [Layout](https://doc.rust-lang.org/nomicon/arc-mutex/arc-layout.html)*

`MyArc<T>` holds a `NonNull<ArcInner<T>>` — the count and the data together —
and a `PhantomData<ArcInner<T>>`, telling the drop checker it owns an
`ArcInner<T>`. It is `Send` and `Sync` only when `T` is both.

### 10.1.2 Base Code

*Original: [Base Code](https://doc.rust-lang.org/nomicon/arc-mutex/arc-base.html)*

`new` boxes the inner value with a count of one and keeps the raw pointer;
`Deref` reaches the data through it.

### 10.1.3 Cloning

*Original: [Cloning](https://doc.rust-lang.org/nomicon/arc-mutex/arc-clone.html)*

Cloning increments the count, `Relaxed` — a new owner needs no
synchronisation with the others — and aborts if the count nears overflow.

### 10.1.4 Dropping

*Original: [Dropping](https://doc.rust-lang.org/nomicon/arc-mutex/arc-drop.html)*

Dropping decrements, `Release`; the owner that brings the count to zero
issues an `Acquire` fence, so it sees every other owner's writes, and frees
the allocation.

### 10.1.5 Final Code

*Original: [Final Code](https://doc.rust-lang.org/nomicon/arc-mutex/arc-final.html)*

```rust harsh
use std.marker.PhantomData
use std.ops.Deref
use std.ptr.NonNull
use std.sync.atomic.( self, AtomicUsize, Ordering)
use std.thread

struct ArcInner<T>
    rc: AtomicUsize
    data: T

pub struct MyArc<T>
    ptr: NonNull<ArcInner<T>>
    phantom: PhantomData<ArcInner<T>>

// Shared across threads: sound when T may be both sent and shared.
unsafe impl<T: Sync + Send> Send for MyArc<T>
unsafe impl<T: Sync + Send> Sync for MyArc<T>

impl<T> MyArc<T>
    pub fn new (data: T) -> MyArc<T>:
        let boxed = Box.new (ArcInner\ rc = AtomicUsize.new 1, data = data)
        MyArc\ ptr = NonNull.new (Box.into_raw boxed) <- unwrap$, phantom = PhantomData

    pub fn count (this: &Self) -> usize:
        let inner = unsafe: this <- ptr <- as_ref$
        inner <- rc <- load Ordering.Acquire

impl<T> Deref for MyArc<T>
    type Target = T
    fn deref (&self) -> &T:
        let inner = unsafe: self <- ptr <- as_ref$
        &inner <- data

impl<T> Clone for MyArc<T>
    fn clone (&self) -> MyArc<T>:
        let inner = unsafe: self <- ptr <- as_ref$
        // Relaxed: a new owner needs no synchronisation with the others.
        let old = inner <- rc <- fetch_add 1 Ordering.Relaxed
        if old >= isize.MAX as usize:
            std.process.abort$
        MyArc\ ptr = self <- ptr, phantom = PhantomData

impl<T> Drop for MyArc<T>
    fn drop (&mut self):
        let inner = unsafe: self <- ptr <- as_ref$
        let old = inner <- rc <- fetch_sub 1 Ordering.Release
        if old != 1:
            return
        // The last owner: see every other owner's writes, then free.
        atomic.fence Ordering.Acquire
        unsafe: drop (Box.from_raw (self <- ptr <- as_ptr$))

struct Payload
    text: String

impl Drop for Payload
    fn drop (&mut self):
        println! "payload freed"

fn main$:
    let shared = MyArc.new (Payload\ text = String.from "shared text")
    let mut handles = Vec.new$
    for i in 0..3:
        let mine = shared <- clone$
        let read = move || format! "thread {i} read {:?}" (mine <- text)
        handles <- push (thread.spawn read)
    for h in handles:
        println! "{}" (h <- join$ <- unwrap$)
    println! "owners left: {}" (MyArc.count (&shared))
```

```text
thread 0 read "shared text"
thread 1 read "shared text"
thread 2 read "shared text"
owners left: 1
payload freed
```

**In Harsh:** the counts' orderings read as they are named; the value is
shared by three threads, and freed once, after the last owner is gone.


# FFI

*Original: [FFI](https://doc.rust-lang.org/nomicon/ffi.html)*

Calling C from Rust means declaring its functions in an `extern "C"` block and
calling them in `unsafe` — the compiler cannot check C. Strings cross as
`CString` and `*const c_char`; callbacks cross as `extern "C" fn`. The
original's examples link to the `snappy` library; these use the C standard
library, which every program is linked with already.

```rust harsh
use std.ffi.( CStr, CString)
use std.os.raw.( c_char, c_int)

// Declarations of C functions: their bodies are in the C library.
extern "C"
    fn abs (x: c_int) -> c_int
    fn strlen (s: *const c_char) -> usize

// A safe interface: a `&CStr` is always valid and terminated, so the call
// is sound for every caller.
fn c_len (s: &CStr) -> usize:
    unsafe: strlen (s <- as_ptr$)

fn main$:
    let s = CString.new "harsh" <- unwrap$
    // The compiler cannot check C: each direct call is our promise.
    println! "{}" (unsafe: abs (-42))
    println! "{}" (c_len (&s))
```

```text
42
5
```

The original's next step is a **safe interface**: a Rust function that takes
Rust types, upholds C's requirements itself, and hides the `unsafe` from its
callers — `c_len` above, taking a `&CStr` that is sure to be valid and
terminated.

A callback: C's `qsort` sorting an array with a comparison written in Harsh.

```rust harsh
use std.ffi.c_void
use std.os.raw.c_int

extern "C"
    fn qsort (base: *mut c_void) (n: usize) (size: usize) (compare: extern "C" fn (*const c_void) (*const c_void) -> c_int)

// Written in Harsh, called by C.
extern "C" fn by_value (a: *const c_void) (b: *const c_void) -> c_int:
    let (x, y) = unsafe: (*(a as *const i32), *(b as *const i32))
    (x > y) as c_int - (x < y) as c_int

fn main$:
    let mut xs = [5, 3, 9, 1, 7]
    unsafe:
        qsort
            (xs <- as_mut_ptr$ as *mut c_void)
            (xs <- len$)
            (std.mem.size_of<i32>$)
            by_value
    println! "{:?}" xs
```

```text
[1, 3, 5, 7, 9]
```

**In Harsh:** the `extern "C"` block holds its declarations beneath its
header, each one line with no body; a function for C to call is `extern "C"
fn` with its body, like any function.


# Beneath std

*Original: [Beneath std](https://doc.rust-lang.org/nomicon/beneath-std.html)*

Everything so far used the standard library. Code for bare metal, kernels or
embedded devices uses `#![no_std]`: only `core` (and `alloc`, if an allocator
is provided). Such a program must define what happens on a panic — a
function marked `#[panic_handler]` — and, as a binary, its own entry point.

Harsh transpiles such code as it does any other — `#![no_std]` is an
attribute like the rest — but a `no_std` binary cannot be built and run the
way this book runs its programs, so this page has none. The original shows
the pieces; with them written in Harsh's spelling, `hrs build` for an embedded
target works as `cargo build` does.

## 12.1 #[panic_handler]

*Original: [#\[panic_handler\]](https://doc.rust-lang.org/nomicon/panic-handler.html)*

A `no_std` program has no panic machinery of its own: exactly one function in
the final binary must be marked `#[panic_handler]`, taking a
`&core::panic::PanicInfo` and never returning (`-> !`). It may loop, reset the
device, or report over a serial port. In Harsh it is written as any function —
`#[panic_handler]` above `fn panic (info: &PanicInfo) -> !:` — with its body
beneath.

That is the end of the dark arts. Use them sparingly, keep them in small
modules, and write down, next to every `unsafe:`, why it is sound.
