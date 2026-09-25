# 8. Ownership and borrowing

This is the part of Rust that Harsh keeps entirely. Nothing here is a Harsh
idea; the only thing to learn is how the spellings look.

## 8.1 Moving

```
fn consume (s: String) -> usize:
    s <- len$

fn main$:
    // A `String` owns its text. Passing it hands the ownership over.
    let a = String.from "hello"
    let n = consume a
    println! "{n}"
    // `a` is gone here: it was moved into `consume`.

    // `clone` makes a second owner.
    let b = String.from "world"
    let c = b <- clone$
    println! "{b} {c}"

    // Small copyable values are copied instead of moved.
    let x = 5
    let y = x
    println! "{x} {y}"
```

```text
5
world world
5 5
```

A value that owns something — a `String` owns its text — has exactly one
owner. Passing it to a function hands the ownership over, and the old name
cannot be used afterwards. `clone$` makes a second owner when you want one,
and small copyable values such as integers are copied rather than moved.

Using a name after it has been moved is refused before the program runs:

```
fn consume (s: String):
    println! "{s}"

fn main$:
    let a = String.from "hello"
    consume a
    println! "{a}"
```

```text
error[E0382]: borrow of moved value: `a`
 --> use_after_move.hrs:8:15
  |
6 |     let a = String::from("hello");
  |         - move occurs because `a` has type `String`, which does not implement the `Copy` trait
7 |     consume(a);
  |             - value moved here
8 |     println!("{a}")
  |               ^^^ value borrowed here after move
  |
note: consider changing this parameter type in function `consume` to borrow instead if owning the value isn't necessary
 --> use_after_move.hrs:1:15
  |
1 | fn consume(s: String) {
  |    -------    ^^^^^^ this parameter takes ownership of the value
  |    |
  |    in this function
  = note: this error originates in the macro `$crate::format_args_nl` which comes from the expansion of the macro `println` (in Nightly builds, run with -Z macro-backtrace for more info)
help: consider cloning the value if the performance cost is acceptable
  |
7 |     consume(a.clone());
  |              ++++++++

error: aborting due to previous error

For more information about this error, try `rustc --explain E0382`.
```

The error arrives on the Harsh line that caused it, not on the generated Rust.

## 8.2 Borrowing

```
// A borrow reads without taking ownership.
fn length (s: &String) -> usize:
    s <- len$

// A mutable borrow may change what it points at.
fn shout (s: &mut String):
    s <- push_str "!"

fn main$:
    let mut greeting = String.from "hi"

    // `&` is an operator, so a borrowed argument is isolated.
    println! "{}" (length (&greeting))

    shout (&mut greeting)
    println! "{greeting}"

    // Many readers, or one writer, never both at once.
    let r1 = &greeting
    let r2 = &greeting
    println! "{r1} {r2}"

    let w = &mut greeting
    w <- push_str "?"
    println! "{greeting}"
```

```text
2
hi!
hi! hi!
hi!?
```

`&x` borrows, `&mut x` borrows so it can change. The rule is one writer *or*
any number of readers, never both, and the compiler checks it.

The Harsh part is only the parentheses: `&` is an operator, so a borrowed
argument is isolated — `length (&greeting)`, `shout (&mut greeting)`.

## 8.3 Slices

```
// A slice borrows a run of something rather than the whole of it.
fn first_word (s: &str) -> &str:
    let bytes = s <- as_bytes$
    for (i, &b) in bytes <- iter$ <- enumerate$:
        if b == b' ':
            return &s[0..i]
    s

fn main$:
    let sentence = String.from "hello wide world"
    println! "{}" (first_word (&sentence))

    let numbers = [1, 2, 3, 4, 5]
    let middle = &numbers[1..4]
    println! "{:?} {}" middle (middle <- len$)
```

```text
hello
[2, 3, 4] 3
```

A slice borrows a run of a value: `&s[0..i]` of a string, `&numbers[1..4]` of
an array. Note `&s[0..i]` needs no extra parentheses when it is a `return`
value, but would as an argument, by the same operator rule.
