# Harsh by Example

Short programs, one idea each, every one of them compiled and run to produce
the output you see. If you know how to program, you can read this book from
the top and write Harsh by the end of it.

It is the *lookup* book: when you want to know how something is written, find
the page and copy the shape. *The Harsh Programming Language* is the one that
teaches the ideas — ownership, borrowing, traits, lifetimes — and is the place
to go when an example here works but you cannot see why. *The Harsh Language
Guide* is the third: it sets every Harsh spelling beside the Rust it stands
for, for readers who already know Rust.

**Rust without braces.**
**Rust with pipes, partial application, comprehensions and linear algebra.**
**Rust for functional programming, data science and machine learning.**

The last three lines are pages 7, 16 and 17: the pipes and partial
application, comprehensions, and matrices with linear algebra — what Harsh
adds to Rust, and the reason to use it.

There is no Rust in this book. Every program here is transpiled, compiled and
run before it reaches the page, so what you read is what the machine did.

The topics, and many of the example programs, follow *Rust by Example*,
rewritten in Harsh. `ATTRIBUTION.md` has the licence and the details.

# 1. Hello and printing

## 1.1 Hello, world!

The whole program:

```rust harsh
fn main$:
    println! "Hello, world!"
```

```text
Hello, world!
```

`fn main$` is the function every program starts from. The `$` says it takes no
arguments — Harsh writes *apply to nothing* that way, and you will see it
again on every call that takes none, like `String.new$`.

`println!` is a macro, which the `!` tells you, and its arguments are simply
written after it with spaces between them. No parentheses, no commas: that is
how every call in Harsh is written, macro or function.

## 1.2 Comments

```rust harsh
fn main$:
    // A line comment runs from the slashes to the end of the line.
    let x = 5

    /* A block comment can sit inside an expression, */
    let y = x + /* like this */ 5

    println! "x = {x}, y = {y}"
```

```text
x = 5, y = 10
```

Two kinds, both Rust's: `//` to the end of the line, and `/* … */` anywhere,
including in the middle of an expression. Block comments nest.

## 1.3 Printing

The string comes first, then the values that fill its holes:

```rust harsh
fn main$:
    // Each `{}` is filled by the next argument, juxtaposed after the string.
    println! "{} days" 31

    // A name in the braces takes it from a variable of that name.
    let actor = "the sun"
    println! "{actor} rises"

    // `{n}` picks the nth argument, counting from zero.
    println! "{0} is {1}, {1} is {0}" "this" "that"

    // A width after `:` pads the value; `>` right-aligns it.
    println! "[{:>5}]" 42

    // And a `{{` is a literal brace.
    println! "{{ not a hole }}"
```

```text
31 days
the sun rises
this is that, that is this
[   42]
{ not a hole }
```

The hole is `{}`. What goes between the braces is a small language of its own —
a name, a number, a width, an alignment — and all of it is Rust's, unchanged.
The thing to notice is what is *outside* the braces: the arguments are
juxtaposed, so `println! "{} {}" a b` needs no commas.

An argument that is not a single atom is isolated in parentheses, exactly as
it would be for a function:

    println! "{}" (a + 1)
    println! "{}" (p <- name)

That second one is a field access. `<-` is how Harsh reads into a value, and
it is an operator, so it needs the parentheses when used as an argument.

## 1.4 Debug

Most types cannot be printed until you say how. The quickest way is to derive
the programmer's spelling:

```rust harsh
// `Debug` is derived: it prints a value the way a programmer reads it.
#[derive Debug]
struct Point\ x: i32, y: i32

fn main$:
    let p = Point\ x = 3, y = 4

    // `{:?}` asks for the Debug spelling...
    println! "{:?}" p

    // ...and `{:#?}` for the same thing laid out over several lines.
    println! "{:#?}" p
```

```text
Point { x: 3, y: 4 }
Point {
    x: 3,
    y: 4,
}
```

`#[derive Debug]` is an attribute, and its arguments juxtapose like everything
else. `{:?}` asks for that spelling and `{:#?}` for the same thing over several
lines.

Note the struct declaration: `struct Point\ x: i32, y: i32`. The `\` opens a
*specification block* — Rust's `{ a, b }` groupings — and it always follows
the thing it specifies. The same `\` builds the value: `Point\ x = 3, y = 4`.
Written over several lines, the fields simply go beneath:

    struct Point
        x: i32
        y: i32

## 1.5 Display

`Display` is the spelling meant for whoever reads the output, and you write it
yourself:

```rust harsh
use std.fmt

struct Point\ x: i32, y: i32

// `Display` is written by hand: it is the spelling meant for a reader.
impl fmt.Display for Point
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        write! f "({}, {})" (self <- x) (self <- y)

fn main$:
    let p = Point\ x = 3, y = 4
    println! "{}" p
```

```text
(3, 4)
```

`impl fmt.Display for Point` takes no mark at all — an item body follows its
header directly, with the items beneath it. `.` is the path separator, so
`fmt.Display` is Rust's `fmt::Display`. Inside, `write!` takes the formatter
and the same string-and-holes as `println!`.

Once a type has `Display`, `{}` works on it, and `to_string$` comes free.
