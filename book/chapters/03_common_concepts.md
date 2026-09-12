# 3. Common concepts

Variables, types, functions, control flow: every language has them, and Rust's versions each have one thing that is not like the others. This chapter is those things.

## 3.1 Variables and mutability

A variable is immutable unless declared otherwise. This is the first Rust decision that surprises people, so here is the compiler enforcing it:

```
fn main$:
    let x = 5
    println! "x is {}" x
    x = 6
    println! "x is {}" x
```

```text
error[E0384]: cannot assign twice to immutable variable `x`
  --> immutable.hrs:4:5
   |
 2 |     let x = 5
   |         - first assignment to `x`
 4 |     x = 6
   |     ^^^^^ cannot assign twice to immutable variable
   = help: consider making this binding mutable (hrs 2:9)
```

Read the error the way Rust means it. It is not "you cannot change variables"; it is "you did not say this one would change". The fix is one word:

```
fn main$:
    let mut x = 5
    println! "x is {}" x
    x = 6
    println! "x is {}" x
```

```text
x is 5
x is 6
```

Why the default? Because most values in most programs are never reassigned, and a reader who sees `let` without `mut` knows — without reading further — that the value is what it was. When a value *does* change, `mut` at the declaration is the announcement. The habit pays for itself the first time you track down where something was modified and the answer is "only where it says `mut`".

### Shadowing

You may declare a new variable with the name of an old one. The new one *shadows* the old for the rest of its scope:

```
fn main$:
    let x = 5
    let x = x + 1

    do:
        let x = x * 2
        println! "inner x is {}" x

    println! "outer x is {}" x

    // Shadowing may change the type; `mut` may not.
    let spaces = "   "
    let spaces = spaces <- len$
    println! "{} spaces" spaces
```

```text
inner x is 12
outer x is 6
3 spaces
```

Shadowing is not mutation. Each `let x` is a fresh variable; the earlier one is merely hidden. That is why the inner block's `x` does not leak out, and why the last example is allowed to turn a string into a number under the same name — a `mut` variable must keep its type, but a new variable can be anything. Use shadowing when a value goes through stages under one natural name (raw text, then parsed) and `mut` when one value genuinely changes.

`do:` in that example opens a bare block — a scope with no `if` or `loop` attached. `do` is a keyword Rust reserved and never used, so it was free to mean this.

## 3.2 Scalar types

Rust is statically typed: every value has a type known at compile time. Usually the compiler infers it and you write nothing; when it cannot, or when you want a specific size, you annotate.

```
fn main$:
    let a: i32 = -7            // signed 32-bit, the default integer
    let b: u8 = 255            // unsigned 8-bit: 0 to 255
    let c: i64 = 9_000_000_000 // underscores are ignored, for reading
    let d = 0xff               // hex; also 0o77 octal, 0b1010 binary
    let e = 2.5                // f64, the default float
    let f: f32 = 1.0
    let t = true
    let ch = 'Z'               // a char is one Unicode scalar, in single quotes
    let heart = '❤'

    println! "{} {} {} {} {} {} {} {} {}" a b c d e f t ch heart
    println! "{}" (a + 10 * 2 - 3 / 2 % 2)
    println! "{}" (7 / 2)      // integer division truncates
    println! "{}" (7.0 / 2.0)
```

```text
-7 255 9000000000 255 2.5 1 true Z ❤
12
3
3.5
```

Integers come in signed (`i8`, `i16`, `i32`, `i64`, `i128`) and unsigned (`u8` … `u128`) widths, plus `isize`/`usize` sized to the machine — `usize` is what indexes and lengths use. `i32` is the default when nothing constrains a literal. Floats are `f64` (default) and `f32`. Division of integers truncates toward zero; there is no automatic promotion to float, and `7 / 2.0` is a type error, not `3.5`.

### What happens at the edges

Adding one to a `u8` holding 255 cannot fit. In a debug build Rust *panics* — stops the program with a message — rather than silently wrapping, which is what most languages do. In a release build it wraps. When you mean wrapping, say so; when you want to know, ask:

```
fn main$:
    let big: u8 = 255
    let bigger = big <- wrapping_add 1
    let checked = big <- checked_add 1
    println! "{} {:?}" bigger checked
```

```text
0 None
```

`checked_add` returns an `Option`: `Some(value)` if it fit, `None` if it did not — the `{:?}` in the format string is *debug* formatting, which is how you print an `Option`. `Option` is Rust's answer to null, and chapter 6 is about it.

## 3.3 Compound types

```
fn main$:
    // A tuple holds a fixed number of values of any types.
    let pair: (i32, &str) = (1, "one")
    let (n, name) = pair                 // destructure into two variables
    println! "{} is {}" n name
    println! "{} {}" pair.0 pair.1       // or index by position

    // An array holds a fixed number of values of one type, on the stack.
    let days = ["mon", "tue", "wed"]
    let zeros = [0; 4]                   // four zeros
    println! "{} {} {}" (days[0]) (days <- len$) (zeros <- len$)
```

```text
1 is one
1 one
mon 3 4
```

A tuple is fixed-length and heterogeneous; you take it apart by destructuring or by `.0`, `.1`. An array is fixed-length and homogeneous, indexed with `days[0]` — and note the parentheses around `(days[0])` when it is an argument: Harsh applies functions by juxtaposition, so an index expression is isolated to keep `f a [0]` from being read as `f` applied to `a` and to an array. An array, and its length is part of its type: `[i32; 4]` is a different type from `[i32; 5]`. Indexing past the end is checked at runtime and panics — it never reads memory it should not. For a list that grows you want `Vec`, in chapter 8.

## 3.4 Functions

```
fn greet name: &str:
    println! "hello, {}" name

fn add (a: i32) (b: i32) -> i32:
    a + b

fn square x: i32 -> i32:
    let y = x * x
    y

fn main$:
    greet "world"
    println! "{}" (add 2 3)
    println! "{}" (square 9)
```

```text
hello, world
5
81
```

Three things to notice.

- **Parameters are typed, always.** Rust does not infer parameter types; the signature is the contract, and the compiler checks every call against it. Each parameter is written in its own group, `(a: i32) (b: i32)`; a single parameter may drop its parentheses, `name: &str`.
- **The return type follows `->`.** No annotation means the function returns `()`, the *unit* type — the empty tuple, the value that carries no information — so `greet` returns unit. This is the `()` chapter 1 promised: a value, never a call. Calling with no arguments is `$`.
- **The last expression is the return value.** `square` computes `y` and then has `y` as its final line, with no `return`. That last line is an *expression*, and a function's body is worth its final expression. `return` exists for leaving early; it is not needed at the end.

### Statements and expressions

That last point has a sharp edge. A *statement* does something and produces no value; an *expression* produces a value. `let y = x * x` is a statement. `y` alone is an expression. A block's value is its final expression — **unless that expression has a semicolon after it**, which turns it into a statement, and then the block is worth `()`. A line ending is normally the end of a statement and you never write `;`, so you rarely think about this. But you may write one, and the rule is: a `;` at the end of a block's last line means *discard this value*.

Here is what a stray one does to a function that meant to return a value:

```
fn plus_one x: i32 -> i32:
    x + 1;

fn main$:
    println! "{}" (plus_one 5)
```

```text
error[E0308]: mismatched types
  --> statement_value.hrs:1:23
   |
 1 | fn plus_one x: i32 -> i32:
   |    -------- implicitly returns `()` as its body has no tail or `return` expression
   |                       ^^^ expected `i32`, found `()`
   = help: remove this semicolon to return this value (hrs 2:10)
```

The error is Rust's most-quoted: the signature promises an `i32`, the body ends in a statement, and so the body is worth `()`. Remove the `;` and it is correct. You will meet this error on purpose exactly once, and then you will know what it means forever.

## 3.5 Control flow

### `if`

```
fn main$:
    let n = 7

    if n % 2 == 0:
        println! "even"
    else:
        println! "odd"
    // `if` is an expression: it produces a value, and both arms must agree on its type.

    let kind = if n > 5: "big" else: "small"
    println! "{}" kind

    let sign =
        if n > 0:
            1

        else if n < 0:
            -1

        else:
            0

    println! "{}" sign
```

```text
odd
big
1
```

The condition must be a `bool` — `if n:` with an integer `n` is a type error, not a truthiness test. And `if` is an expression, so it can sit on the right of a `let`; when it does, every arm must produce the same type, because `kind` has to be *some* one type:

```
fn main$:
    let n = 7
    let kind = if n > 5: "big" else: 0
    println! "{}" kind
```

```text
error[E0308]: `if` and `else` have incompatible types
  --> if_mismatch.hrs:3:38
   |
 3 |     let kind = if n > 5: "big" else: 0
   |                ----------------------- `if` and `else` have incompatible types
   |                          ----- expected because of this
   |                                      ^ expected `&str`, found integer
```

Notice the shape of the multi-line `if` in the previous example: the `if` begins its own line after `let sign =`, so its arms indent from *it*. That is the rule everywhere a construct opens in the middle of a line: a block's lines are indented past the column of the construct that opened it, not merely past the statement, and when that would push the block too far right, `=` ends its line instead and the opener starts its own. Every control-flow construct also has the inline shape chapter 2 showed for blocks:

```
fn main$:
    let n = 7

    // a construct whose body fits on the line: the colon is followed by the body
    let parity = if n % 2 == 0: "even" else: "odd"
    let size = match n:
        0 => "none"
        1..=5 => "few"
        _ => "many"
    let sign = match n: 0 => 0, _ if n > 0 => 1, _ => -1

    // `else` answers the `if` above it, and may sit under the `if`...
    let kind =
        if n > 100:
            "huge"
        else if n > 10:
            "big"
        else:
            "small"

    // ...or under the line that holds the `if`, when it opened mid-line
    let label = if n > 10:
        "big"
    else:
        "small"

    let mut count = 0
    while count < 3: count += 1
    for i in 0..2: println! "i = {i}"

    println! "{parity} {size} {sign} {kind} {label} {count}"
```

```text
i = 0
i = 1
odd many 1 small small 3
```

> **Harsh —** Inline, the body follows the colon on the same line, and the arms of a `match` written on one line are separated by commas — the only place a comma separates arms, since on their own lines the line ending does that. An `else` answers the nearest `if` above it, and it may sit at the column of any line of that `if`: under the `if` itself, or under the `let` that holds it when the `if` opened on the `let`'s line. A `while` or `for` whose body is one statement takes the inline form as readily as an `if` does.

### Loops

```
fn main$:
    // `loop` repeats until `break`, and `break` can carry the loop's value.
    let mut counter = 0

    let result =
        loop:
            counter += 1

            if counter == 10:
                break counter * 2

    println! "{}" result

    // `while` repeats while a condition holds.
    let mut n = 3

    while n != 0:
        println! "{}" n
        n -= 1
    // `for` walks anything iterable: an array, a range.

    let a = [10, 20, 30]

    for x in a:
        println! "{}" x

    for i in (1..4) <- rev$:
        println! "{}" i
```

```text
20
3
2
1
10
20
30
3
2
1
```

`loop` with `break value` is how you compute something by repetition and hand it out — the loop is an expression too. `while` is what you expect. `for` is the loop you will write most: it walks an iterator, never an index, so it cannot run off the end of the array, and `1..4` is a *range* (1, 2, 3 — the end is excluded; `1..=4` would include it). `<- rev$` reverses it; ranges and arrays are both things you can iterate, which is chapter 13's topic.

## 3.6 What you have

Immutable by default, `mut` and shadowing; the scalar and compound types and their fixed sizes; typed parameters, the final expression as the return value, and the semicolon that discards it; `if` and the loops as expressions. Next: the one idea that is Rust's alone, and the reason this book exists — ownership.
