# The Harsh Programming Language

## Why no braces

Here is what a brace language looks and reads like, once you notice it:

```text
I(would, like, to, go) {
    to the market so I(could buy) {
        some groceries
    }
    to the laundromat in order to(get) {
        clothes washed
    }
}
```

and here is the message it was trying to convey:

```text
I would like to go
    to the market so I could buy
        some groceries
    to the laundromat in order to get
        clothes washed
```

The second is already in the first — the indentation was there all along; the braces and parentheses only repeat it, in a form the eye has to skip past. That is sometimes hard to see, and because of it some very simple concepts become harder to learn than they are. Harsh weeds out the braces to let Rust shine.

Early in the work, Claude asked the author to justify removing braces from Rust at all. After a long enough exchange it put the answer in one line, and the line stayed:

> *Braces are for the compiler; indentation is for humans.* — Claude

## Who this is for

You have programmed before — in Python, JavaScript, Elixir, anything — and you have not programmed in Rust. This book teaches you Rust, using Harsh as the notation.

That sentence has a trap in it, so here it is on page one: **Harsh makes Rust quieter, not easier.** Harsh is a spelling of Rust — indentation for structure, `f a b` for applying a function, `<-` for reaching into a value — and it changes nothing about what Rust means. Ownership, borrowing, lifetimes, the type system, the compiler saying no: all of it arrives on schedule, unchanged, and this book is mostly about *that*. The notation just gets out of the way while you learn it.

If you already know Rust, *The Harsh Language Guide*, which teaches only the Harsh superset, will onboard you faster.

## How to read it

Every example is shown two ways: the **Harsh** you would write, and what it **prints** when run. Both are produced by the build from a single source file — the file is transpiled, compiled and run to make the page — so they cannot disagree. When an example is *meant* to fail, and learning Rust means learning what the compiler refuses and why, you see the Harsh and then the error the compiler reports, pointing at the line you wrote.

> **Harsh —** Harsh is one spelling of Rust, and not the usual one. Rust as you will meet it in crates and in answers on the internet is written differently — the same language, the same meanings, other punctuation. Nothing in this book depends on that other spelling and none of it appears here. When you want the two set side by side, *The Harsh Language Guide* is where that comparison lives, and `hrs export` turns any project of yours into the other spelling to read.

The chapters follow the order of *The Rust Programming Language* (the "Rust Book"), because that order has been refined over years to introduce each idea exactly when you need it; the prose and the examples here are original.


## Setting up

You need the Rust toolchain and the Harsh tools:

```text
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh     # Rust, if you do not have it
cargo install harsh-lang                                               # hrs, hrs-from, hrs-remap, hrs-lsp
hrs new hello && cd hello && hrs run                              # a first project
```

If the last line prints `Hello from Harsh`, you are ready. The tools are the crate [`harsh-lang`](https://crates.io/crates/harsh-lang); for the editor, install [Harsh](https://marketplace.visualstudio.com/items?itemName=harsh-lang.harsh-lang) from the VSCode Marketplace, and `hrs-lsp` — already on your `PATH` — gives it the layout-aware Enter, Tab and format-on-save the rest of this book assumes you have.

# 1. Getting started

## 1.1 A first program

Every program in this book is a complete file that you could save as `src/main.hrs` in a project made by `hrs new` and run with `hrs run`. Here is the smallest one:

```
fn main$:
    println! "Hello, world!"
```

```text
Hello, world!
```

Read the three blocks top to bottom.

- `fn main$:` declares a function named `main`. The `$` says it takes no parameters; the `:` at the end of the line says *the body follows, indented*.
- `println!` prints a line. The `!` marks it as a **macro**, not a function: a macro is expanded by the compiler into code before compilation proper. You do not need to know how yet; you need to know that `!` means "this is a macro" so that you are not surprised by what it can do, such as checking that `"{}"` has exactly one argument to fill it.
- `"Hello, world!"` is the argument. A function or macro is applied to its arguments by writing them after it, separated by spaces: `println! "…"`.
- `main` is where a program starts. Every executable has exactly one.

## 1.2 What `hrs run` did

`hrs run` is two steps that you will do a thousand times, so it is worth seeing them once.

1. **Transpile.** `hrs` reads `src/main.hrs` and writes `target/hrs/main.rs`, the same program in Rust's own spelling. It also writes a *source map*: which byte of the Rust came from which byte of the Harsh.
2. **Compile and run.** `cargo`, Rust's build tool, compiles the Rust and runs the binary. If the compiler complains, `hrs` uses the source map to turn every line number in the complaint into a line of your `.hrs` file.

You never read `target/hrs/main.rs` unless you want to; in daily use it is an intermediate file.

## 1.3 Arguments

Programs take input. The simplest input is what follows the program's name on the command line, which Rust hands you through the standard library:

```
use std.env

fn main$:
    let words: Vec<String> = env.args$ <- collect$
    println!
        "{} argument(s), the first being the program itself"
        (words <- len$)
```

```text
1 argument(s), the first being the program itself
```

- `use std.env` brings the `env` module of the standard library into scope. `std` is the standard library; `.` separates the parts of a path.
- `env.args$` calls the function `args` in that module. The `$` means *call with no arguments*, and it is the same `$` that said *no parameters* on `fn main$`: without it, `env.args` would name the function without calling it. Hold on to that distinction; it matters in Rust more than in most languages. (`()` is not this. In Harsh `()` is a value — the empty tuple, called *unit* — and you will meet it in chapter 3.)
- `<- collect$` is a **method call** on the value to its left. The arrow is how you reach into a value, and it is the token you will write most. Read `<-` as *then*: take the arguments, then collect them.
- `let words: Vec<String> = …` declares a variable and states its type: a vector (a growable list) of strings. Rust can usually infer types, but `collect` can produce many kinds of collection, so here you must say which. When this program is run with no arguments the vector holds exactly one item — the program's own name — which is why it prints `1`.

Two things about `let` that will matter soon: a variable declared with `let` cannot be changed afterwards unless you say `let mut`, and the type after the colon is a promise the compiler will hold you to. Both are chapter 3.

## 1.4 Comments

```
// A comment runs from `//` to the end of the line.
fn main$:
    // This one explains the next line.
    let answer = 42        // and this one sits after the code
    println! "{}" answer
```

```text
42
```

`//` starts a comment that runs to the end of the line. There are no block comments in this book, though they exist (`/* … */`). A comment on its own line, above the thing it describes, is the convention; a comment after code on the same line is fine for a short note.

You now have everything needed to read the next chapter, which builds a small program end to end before the language is explained piece by piece.

# 2. A small program

Before the language is taken apart piece by piece, here is one whole program: a guessing game. The computer holds a number; you type guesses; it says higher or lower until you hit it. It is fourteen lines, and it uses input, output, conversion, comparison, a loop, and Rust's way of dealing with things that can fail. Every one of those gets its own chapter later; here you meet them in the wild.

## 2.1 The whole thing

Reading input from the keyboard is where most languages' toy programs cheat, so this one does not. Run with the guesses `50`, `25`, `37`:

```
use std.io
use std.cmp.Ordering

fn main$:
    let secret = 37

    loop:
        let mut line = String.new$
        io.stdin$ <- read_line (&mut line) <- expect "read failed"

        let guess: u32 = match line <- trim$ <- parse$:
            Ok n => n
            Err _ => continue

        match guess <- cmp (&secret):
            Ordering.Less => println! "higher"
            Ordering.Greater => println! "lower"
            Ordering.Equal => do:
                println! "you have it: {}" guess
                break
```

```text
lower
higher
you have it: 37
```

Now the same program, walked through.

## 2.2 Bringing a name into scope

```
use std.io
```

The standard library is large and nothing in it is in scope by default except a small *prelude* (`println!`, `Vec`, `String`, `Option`, and a few others). `use` brings a path into scope; after this line `io` means `std.io` and `io.stdin$` is a valid call. You could write `std.io.stdin$` everywhere instead — `use` is a convenience, not a requirement.

## 2.3 The secret

```
let secret = 37
```

No type is written and none is needed: `37` is an integer literal, and Rust infers the type `i32` — a 32-bit signed integer, the default for integer literals. Chapter 3 lists the others.

Two things about this `let` will feel strange coming from most languages. First, `secret` is **immutable**: writing `secret = 12` later is a compile error. Rust makes every variable immutable unless you ask, with `let mut`, and the reason is the whole of chapter 4. Second, the name is in scope from this line to the end of the enclosing block and no further, which for `main` means the end of the program.

## 2.4 Loop, read, trim, parse

```
loop:
    let mut line = String.new$
    io.stdin$ <- read_line (&mut line) <- expect "read failed"
```

`loop:` opens a block that repeats forever; only `break` leaves it. Its body is the indented lines beneath.

`String.new$` calls the *associated function* `new` on the type `String`. It makes an empty string that can grow. The `mut` says this one will change: `read_line` is about to append to it.

The next line is a **method chain**, and it is worth reading slowly because you will write chains like it constantly.

- `io.stdin$` gets a handle on standard input.
- `<- read_line (&mut line)` calls `read_line` on that handle, passing `&mut line` — a *mutable reference* to the string, which is how a function is allowed to change a value that belongs to you. The `&mut` is not decoration; without it the call does not compile. That is chapter 4 again.
- `read_line` does not return the line. It returns a `Result` — a value that is either `Ok(bytes_read)` or `Err(some_error)` — because reading can fail. `<- expect "read failed"` says: if it is `Ok`, give me what is inside; if it is `Err`, stop the program and print this message. It is the bluntest way to handle a `Result`, fine for a toy, and chapter 9 replaces it.

```
    let guess: u32 = match line <- trim$ <- parse$:
        Ok n => n
        Err _ => continue
```

The line the user typed ends in a newline, so `<- trim$` removes it. `<- parse$` turns the text into a number — but which kind of number? The annotation `let guess: u32` tells it: an unsigned 32-bit integer. Rust reads the type you asked for and works backwards to what `parse` must produce. Parsing text can fail too (type `hello`), so `parse` also returns a `Result`, and this time the program handles both cases with `match`:

- `Ok n => n`: the parse worked; `n` is the number; the whole `match` is worth `n`, and that is what `guess` becomes.
- `Err _ => continue`: it failed; `_` means "I do not care what the error was"; `continue` jumps to the next iteration of the loop, asking again. `continue` never produces a value, so Rust is content that this arm does not yield a `u32`.

`match` is Rust's central control structure: it takes a value, lists the shapes it can have, and refuses to compile unless every shape is covered. `Result` has exactly two shapes and both are here.

## 2.5 Compare and decide

```
    match guess <- cmp (&secret):
        Ordering.Less => println! "higher"
        Ordering.Greater => println! "lower"
        Ordering.Equal => do:
            println! "you have it: {}" guess
            break
```

`cmp` compares two values and returns an `Ordering`, which is an **enum** — a type whose value is one of a fixed set of named alternatives: `Less`, `Greater`, `Equal`. The `use std.cmp.Ordering` at the top is what lets us write `Ordering.Less` instead of `std.cmp.Ordering.Less`.

The argument is `&secret`, a reference: `cmp` wants to *look at* the other number, not take it. You will see `&` on most arguments in Rust, and chapter 4 explains when you need it and when you do not.

Three arms, one per alternative, and the compiler checks that there are three. The `Equal` arm has two statements, so it is a block, opened by `do:`, indented beneath. `break` ends the `loop`, and since there is nothing after the loop in `main`, the program ends.

## 2.6 How a page of Harsh is laid out

You have now read a whole program, so this is the moment to say what the shape of it means. There is not much to say, and all of it is one idea: **where a line starts is what it belongs to.**

```
// The same function three times: one block, three spellings.

fn indented (t: bool) -> i32:
    if t:
        1
    else:
        2

fn inline (t: bool) -> i32:
    if t: 1 else: 2

fn braced (t: bool) -> i32:
    if t { 1 } else { 2 }

fn main$:
    // `do:` opens a block with nothing attached: a scope of its own
    let squared = do:
        let u = 3
        u * u

    // parameters: one group each, or bare when there is only one
    println!
        "{} {} {} {}"
        (indented true)
        (inline false)
        (braced true)
        squared
```

```text
1 2 1 9
```

> **Harsh —** A block opens at a `:` that ends its line, and its body is the lines indented beneath it; the body ends where the indentation does. That is the one form you will write nearly everywhere. The same block may be written *inline*, with the body after the colon on the same line, when it is one expression and fits; and it may be written in braces, on one line only, when several statements have to share a line — `{ let u = 3; u * u }` — which is also the spelling you reach for when pasting a line written the other way. All three produce exactly the same program. A block that belongs to nothing — a scope opened for its own sake — is `do:`. Two rules follow from the idea. A line ending is the end of a statement, so you never write `;` between statements; the one `;` you write yourself goes at the end of a block's last line, to say *discard this value* (chapter 3 shows what that does). And a line indented deeper than the one above it, without a `:` to open a block, *continues* it — which is how a long line is broken.

There is one more shape on that page, and it is the one that looks least like other languages:

```
fn greet name: &str -> String:
    format! "hello, {name}"

fn add (a: i32) (b: i32) -> i32:
    a + b

fn nothing$ -> i32:
    42

fn main$:
    let s = greet "harsh"            // one argument: the name follows the function
    let n = add 2 3                  // two: juxtaposed, no commas, no parens
    let m = add (n * 2) (nothing$)   // an expression is one argument once it is in parens
    let z = nothing$                 // applied to nothing: `$`, never `()`
    let u = ()                       // `()` is a value -- the unit -- and only ever that

    let v = vec! [1, 2, 3]           // a macro applies the same way
    let len = v <- len$              // `<-` reaches into a value: a method, a field
    let text = String.from "abc"     // `.` walks a path: a module, a type, an item

    // an application binds tighter than `<-`: the arrow takes the result
    let big = greet "harsh" <- to_uppercase$        // greet first, then the method on it
    let total = String.from "abc" <- len$ + len     // the same, then the operator on that
    let inner = greet (&(text <- to_uppercase$))    // the arrow *inside* an argument: isolated; `&` is an operator, so its operand is too

    println! "{s} {n} {m} {z} {u:?} {len} {text} {big} {total} {inner}"
```

```text
hello, harsh 5 52 42 () 3 abc HELLO, HARSH 6 hello, ABC
```

> **Harsh —** A function is applied by writing its arguments after it, separated by spaces: `add 2 3`. Parentheses around an argument mean *this is one argument* — `add (n * 2) (nothing$)` — and never *these are the arguments*; an argument that is a single token needs none. Applying a function to nothing is `nothing$`, because `()` is a value, the unit, and only ever that. The same rule declares a function: `fn add (a: i32) (b: i32)` is one group per parameter, and a lone parameter may drop its parentheses, `fn greet name: &str`. An index written tight, `arr[1]`, is part of its atom, so `f arr[1]` passes the element — the same way `t.0` is part of `t`; brackets never apply, and an array passed as an argument is isolated, `f ([1, 2, 3])`. Two arrows share the work of reaching into things: `<-` reaches into a *value* — a field, a method, `v <- len$` — and `.` walks a *path* — a module, a type, an item, `String.from`. A macro applies like a function, with its `!` glued to its name: `vec! [1, 2, 3]`, `println! "{s}"`. And when an application and an arrow meet, **the application binds tighter**: in `greet "harsh" <- to_uppercase$` the function is applied first and the arrow takes its result, and the next arrow, or an operator, ends the arguments. The parentheses you will be tempted to write, `(greet "harsh") <- to_uppercase$`, are not wrong; they are not needed. Parentheses are needed the other way round — when the arrow belongs *inside* an argument, `greet (&(text <- to_uppercase$))`.

## 2.7 What you have met

Immutability by default and `mut` to opt out; `&` and `&mut` on arguments; `Result` for operations that can fail and `match` to take it apart; `loop`, `break`, `continue`; enums; type annotations that tell `parse` what to produce. Each has a chapter. The next one starts with the smallest pieces: variables, types, functions, and control flow.

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

# 4. Ownership

This is the chapter the book exists for. Ownership is the idea that is Rust's alone: the reason it needs no garbage collector and no manual `free`, the reason the compiler will refuse programs that every other language accepts, and the reason those programs, once accepted, do not have the bugs the others do. Harsh changes nothing here. There is no notation to learn in this chapter beyond what you have, and every example is a conversation with the compiler about who is responsible for a value. Read the errors as carefully as the programs; they are the lesson.

## 4.1 What ownership is

### The stack, the heap, and the question

A running program keeps values in two places. The **stack** holds values whose size is known at compile time — an `i32`, a `bool`, a fixed array — and it is fast because it is simple: push on the way in, pop on the way out, in strict order. The **heap** holds values whose size is not known in advance or may change — the text of a string typed by the user, a list that grows — and it is slower because someone has to find a free region, record where it is, and later give it back.

That "give it back" is the whole problem. Give the memory back too early and something still using it reads garbage; give it back twice and the allocator's bookkeeping is corrupted; never give it back and the program leaks. Garbage-collected languages solve this by never letting you free anything and periodically finding what is unreachable. C solves it by trusting you. Rust solves it with a rule the compiler can check.

### The rules

- Every value has exactly one **owner**: the variable that holds it.
- When the owner goes out of scope, the value is **dropped** — its memory is freed, immediately and deterministically.
- There is only ever one owner at a time, so a value is freed exactly once.

Scope is the ordinary thing: a variable is valid from where it is declared until the end of the block that declares it.

```
fn main$:
    do:
        let s = "hello"          // s is valid from here
        println! "{}" s          // and usable until the block ends
    // the block is over: s is gone, and there is nothing to free

    let t = String.from "hello"  // a String owns memory on the heap
    println! "{}" t
    // main ends here: t goes out of scope and its memory is freed
```

```text
hello
hello
```

`s` holds a string *literal*, text baked into the program, which lives on the stack and costs nothing to drop. `t` holds a `String`, which owns a buffer on the heap. `String.from` allocates it; when `t` goes out of scope at the end of `main`, Rust frees it. You wrote no `free`, and none was needed, because the compiler knows exactly where `t`'s scope ends and puts the call there itself. That is ownership from the value's point of view: an owner, and a drop at the end of the owner's scope.

`String` is the type you reach for when text is not fixed at compile time — it can grow:

```
fn main$:
    let mut s = String.from "hello"
    s <- push_str ", world"       // append to the heap buffer
    s <- push '!'                 // append one character
    println! "{}" s
    println! "{} bytes" (s <- len$)
```

```text
hello, world!
13 bytes
```

`push_str` and `push` append to the buffer, reallocating if it is full. `s <- len$` is isolated in parentheses because it is an argument to `println!` and is not a single atom — the same rule chapter 3 gave for an index expression.

### Move

Now the rule bites. What happens when a `String` is assigned to a second variable?

```
fn main$:
    let s1 = String.from "hello"
    let s2 = s1                   // s1 is moved into s2
    println! "{}, world" s1       // s1 is no longer valid
```

```text
error[E0382]: borrow of moved value: `s1`
  --> move.hrs:4:26
   |
 2 |     let s1 = String.from "hello"
   |         -- move occurs because `s1` has type `String`, which does not implement the `Copy` trait
 3 |     let s2 = s1                   // s1 is moved into s2
   |              -- value moved here
 4 |     println! "{}, world" s1       // s1 is no longer valid
   |                          ^^ value borrowed here after move
   = help: consider cloning the value if the performance cost is acceptable (hrs 3:16)
```

The line `let s2 = s1` did not copy the string. A `String` is, on the stack, three words — a pointer to the heap buffer, a length, and a capacity — and assignment copies those three words, so that `s1` and `s2` would both point at the same buffer. If both were owners, both would free it when they went out of scope, and that is the double-free the rules exist to prevent. So Rust does the only safe thing: it declares that the assignment **moved** the value, that `s2` is now the owner, and that `s1` is no longer valid. Using `s1` afterwards is the error you see — and read it, because it tells you where the move happened, why (`String` does not implement `Copy`), and what to do if you meant to keep both.

This is the moment people come to Rust from anywhere else and feel the floor shift: assignment is not a copy. It is a transfer of responsibility. Once you have it, most of the chapter follows from it.

### Clone

When you actually want two strings, say so:

```
fn main$:
    let s1 = String.from "hello"
    let s2 = s1 <- clone$       // a second, independent copy of the heap data
    println! "s1 = {}, s2 = {}" s1 s2
```

```text
s1 = hello, s2 = hello
```

`clone` allocates a second buffer and copies the bytes into it; now there are two owners of two values, and each will be freed once. It is a method call, so it is visible in the code, which is the point — anything that copies heap memory is expensive enough that Rust wants you to write it down. When you see `<- clone$` you know something possibly costly is happening; when you do not, you know nothing is.

### Copy

Integers did not behave this way in chapter 3, and they still do not:

```
fn main$:
    let x = 5
    let y = x                     // an integer is copied, not moved
    println! "x = {}, y = {}" x y
```

```text
x = 5, y = 5
```

An `i32` is a value that lives entirely on the stack, has no buffer to free, and is as cheap to copy as to move — the two operations are the same four bytes. Types like that implement a trait called `Copy`, and for a `Copy` type assignment copies and the original stays valid. All the scalars are `Copy`, and so are tuples and arrays made only of `Copy` types. Nothing that owns heap memory is, and a type cannot be `Copy` if dropping it does work. The error above named this exactly: `String` does not implement `Copy`, so assignment moved it.

### Ownership and functions

Passing a value to a function is the same as assigning it to the parameter, and follows the same rule:

```
fn takes_ownership some_string: String:
    println! "{}" some_string
    // some_string goes out of scope here and is freed

fn makes_copy some_integer: i32:
    println! "{}" some_integer

fn main$:
    let s = String.from "hello"
    takes_ownership s             // s moves into the function…

    let x = 5
    makes_copy x                  // x is copied into the function…
    println! "{}" x               // …so x is still here
    println! "{}" s               // …but s is not
```

```text
error[E0382]: borrow of moved value: `s`
  --> fn_move.hrs:15:19
   |
 9 |     let s = String.from "hello"
   |         - move occurs because `s` has type `String`, which does not implement the `Copy` trait
10 |     takes_ownership s             // s moves into the function…
   |                     - value moved here
15 |     println! "{}" s               // …but s is not
   |                   ^ value borrowed here after move
   = note: consider changing this parameter type in function `takes_ownership` to borrow instead if owning the value isn't necessary (hrs 1:33)
   = help: consider cloning the value if the performance cost is acceptable (hrs 10:22)
```

`s` moved into `takes_ownership`, was printed, and was dropped when that function returned — so by the time `main` tries to print it, it is gone, and the compiler says so. `x` was copied into `makes_copy`, so `main` still has it. Notice the compiler's note: it has already worked out that `takes_ownership` did not need to own the string, and suggests borrowing instead. That suggestion is the next section; for now, see that the rule is uniform. There is no special case for function calls. A value goes where its owner goes.

Returning a value moves it too — out of the function and into whatever receives it:

```
fn gives_ownership$ -> String:
    let some_string = String.from "yours"
    some_string                   // moved out to the caller

fn takes_and_gives_back a_string: String -> String:
    a_string                      // moved in, moved back out

fn main$:
    let s1 = gives_ownership$
    let s2 = String.from "hello"
    let s3 = takes_and_gives_back s2
    println! "{} {}" s1 s3
```

```text
yours hello
```

`gives_ownership` creates a `String` and hands it to `s1`. `s2` moves into `takes_and_gives_back` and comes out again as `s3`; `s2` is gone, and the string it named now belongs to `s3`. The value was never copied, only passed along. Nothing was freed until `main` ends, when `s1` and `s3` go out of scope and each drops what it owns.

This is correct and it is also tedious. A function that only wants to *look* at a `String` — measure it, say — has to give it back or the caller loses it, and giving it back means returning it alongside the real answer:

```
fn calculate_length s: String -> (String, usize):
    let length = s <- len$
    (s, length)                   // hand the String back along with the answer

fn main$:
    let s1 = String.from "hello"
    let (s2, len) = calculate_length s1
    println! "the length of '{}' is {}" s2 len
```

```text
the length of 'hello' is 5
```

It works. It is also nobody's idea of a good time, and Rust has a better one.

## 4.2 References and borrowing

A **reference** lets a function use a value without owning it. Write `&s1` and you get a reference to `s1`; the function receives `&String` — "a reference to a String" — and when its parameter goes out of scope nothing is freed, because the parameter never owned anything:

```
fn calculate_length s: &String -> usize:
    s <- len$
    // s goes out of scope, but it never owned the String, so nothing is freed

fn main$:
    let s1 = String.from "hello"
    let len = calculate_length (&s1)
    println! "the length of '{}' is {}" s1 len
```

```text
the length of 'hello' is 5
```

Creating a reference is called **borrowing**, and the word is chosen with care: you have the value for a while, you give it back, and the owner was the owner throughout. `s1` is still valid after the call because it never left. The `&` appears once in the signature, `s: &String`, and once at the call, `(&s1)` — where it is isolated in parentheses because `&s1` contains an operator and an argument must be one atom. Chapter 3's index rule again: if it has an operator in it, wrap it.

### Mutable references

A borrowed value cannot be changed through the borrow:

```
fn change some_string: &String:
    some_string <- push_str ", world"

fn main$:
    let s = String.from "hello"
    change (&s)
```

```text
error[E0596]: cannot borrow `*some_string` as mutable, as it is behind a `&` reference
  --> borrow_mut_err.hrs:2:5
   |
 2 |     some_string <- push_str ", world"
   |     ^^^^^^^^^^^ `some_string` is a `&` reference, so the data it refers to cannot be borrowed as mutable
   = help: consider changing this to be a mutable reference (hrs 1:25)
```

A plain `&` reference is a promise to only read. To change something through a reference you need a **mutable reference**, `&mut`, and the value you borrow from has to be `mut` in the first place:

```
fn change some_string: &mut String:
    some_string <- push_str ", world"

fn main$:
    let mut s = String.from "hello"
    change (&mut s)
    println! "{}" s
```

```text
hello, world
```

Three `mut`s: the variable is declared mutable, the borrow is taken mutably, and the parameter's type says so. That is not redundancy; each one is a statement at a different place — the declaration, the call, the signature — and each is what its reader needs to know.

### The one rule about mutable references

Here is the restriction that makes borrowing safe, and it is the compiler's most-argued-with error:

```
fn main$:
    let mut s = String.from "hello"
    let r1 = &mut s
    let r2 = &mut s
    println! "{}, {}" r1 r2
```

```text
error[E0499]: cannot borrow `s` as mutable more than once at a time
  --> two_mut.hrs:4:14
   |
 3 |     let r1 = &mut s
   |              ------ first mutable borrow occurs here
 4 |     let r2 = &mut s
   |              ^^^^^^ second mutable borrow occurs here
 5 |     println! "{}, {}" r1 r2
   |                       -- first borrow later used here
```

**At any one time, a value may have either one mutable reference or any number of immutable ones — never both, never two mutable.** The reason is *data races*: two paths that can write the same memory, or one that writes while another reads, with nothing to order them. In a garbage-collected language that is a bug you find at runtime, sometimes. Rust makes it a compile error, always, by refusing to let two mutable references to one value exist at the same time.

The scope of a reference is what matters, so the borrows only conflict while both are alive. Give the first one a block of its own and the second is fine:

```
fn main$:
    let mut s = String.from "hello"

    do:
        let r1 = &mut s
        r1 <- push_str " there"
    // r1 is gone, so a new mutable borrow is fine

    let r2 = &mut s
    r2 <- push_str "!"
    println! "{}" s
```

```text
hello there!
```

Mixing is refused for the same reason. Readers were promised nothing would change under them; a writer breaks that promise:

```
fn main$:
    let mut s = String.from "hello"
    let r1 = &s
    let r2 = &s
    let r3 = &mut s
    println! "{}, {}, and {}" r1 r2 r3
```

```text
error[E0502]: cannot borrow `s` as mutable because it is also borrowed as immutable
  --> mixed_borrow.hrs:5:14
   |
 3 |     let r1 = &s
   |              -- immutable borrow occurs here
 5 |     let r3 = &mut s
   |              ^^^^^^ mutable borrow occurs here
 6 |     println! "{}, {}, and {}" r1 r2 r3
   |                               -- immutable borrow later used here
```

A reference's scope, though, is not its enclosing block. It runs from where the reference is created to **the last place it is used**. So this compiles, because `r1` and `r2` are not used after the first `println!`, and the compiler can see that:

```
fn main$:
    let mut s = String.from "hello"
    let r1 = &s
    let r2 = &s
    println! "{} and {}" r1 r2

    // r1 and r2 are not used after this point, so their borrow is over
    let r3 = &mut s
    r3 <- push_str "!"
    println! "{}" r3
```

```text
hello and hello
hello!
```

The compiler tracks where each borrow ends by use, not by block, and this is what makes the rule livable rather than merely correct. When you get one of these errors, look at the last use of the earlier borrow, because that is where its scope ends, and often the fix is to reorder two lines so the borrows do not overlap.

### Dangling references

In languages with pointers it is possible to hand out a pointer to memory that is then freed — a *dangling* pointer, the classic source of crashes and worse. Rust guarantees this cannot happen: a reference is never allowed to outlive the value it points to. Try to return a reference to a local and see:

```
fn dangle$ -> &String:
    let s = String.from "hello"
    &s                            // a reference to s…

fn main$:
    let reference_to_nothing = dangle$
    // …but s was freed when dangle returned
```

```text
error[E0106]: missing lifetime specifier
  --> dangle.hrs:1:15
   |
 1 | fn dangle$ -> &String:
   |               ^ expected named lifetime parameter
   = help: this function's return type contains a borrowed value, but there is no value for it to be borrowed from
   = help: consider using the `'static` lifetime (hrs 1:16)
```

`s` is created inside `dangle`, so it is dropped when `dangle` returns; a reference to it would point at freed memory. The compiler's message is about *lifetimes*, a word chapter 10 explains in full — for now, read the first `help`: the return type is a borrowed value and there is nothing in this function it could be borrowed *from*. The fix is to return the `String` itself, moving it out to the caller:

```
fn no_dangle$ -> String:
    let s = String.from "hello"
    s                             // move the String out instead

fn main$:
    let s = no_dangle$
    println! "{}" s
```

```text
hello
```

Ownership moves out; nothing dangles; the caller owns the string and will drop it.

To sum up this section in two lines: at any time, either one mutable reference or any number of immutable ones; and a reference must always point to a valid value. Everything the compiler said above is one of those two rules, applied.

## 4.3 Slices

A **slice** is a reference to a contiguous part of a collection rather than the whole thing. It exists to solve a problem that references alone leave open, so here is the problem first. Say we want the first word of a string. Without slices, the natural answer is an *index* — the position where the first word ends:

```
fn first_word s: &String -> usize:
    let bytes = s <- as_bytes$

    for (i, &item) in bytes <- iter$ <- enumerate$:
        if item == b' ':
            return i

    s <- len$

fn main$:
    let mut s = String.from "hello world"
    let word = first_word (&s)    // word = 5
    s <- clear$                 // s is now "", but word is still 5
    println! "{}" word            // 5 is the answer to a question nobody can ask now
```

```text
5
```

This compiles and it prints `5`, and it is wrong in a way the compiler cannot see: `word` is a `usize`, a plain number with no connection to `s`, so when `s` is cleared, `word` goes on holding a meaning that no longer means anything. The bug is that we have two values that must agree — the string and an offset into it — and nothing enforces the agreement.

Two things about the function itself. `as_bytes` gives the string's bytes, `iter$ <- enumerate$` walks them with their indices, and `(i, &item)` in the `for` destructures each pair — the `&` in the pattern takes the byte out of the reference the iterator yields. And the `if` returns early with `return i`; the function's last line, `s <- len$`, is the value when no space is found.

### String slices

A string slice is a reference to part of a `String`:

```
fn main$:
    let s = String.from "hello world"
    let hello = &s[0..5]
    let world = &s[6..11]
    let from_start = &s[..5]      // same as 0..5
    let to_end = &s[6..]          // same as 6..s.len()
    let whole = &s[..]
    println! "{} {} {} {} {}" hello world from_start to_end whole
```

```text
hello world hello world hello world
```

`&s[0..5]` is a reference to the five bytes starting at index 0 — the range's end is excluded, as in chapter 3. Under the hood it is a pointer into `s`'s buffer plus a length. The start may be dropped when it is 0, the end when it is the length, and both when it is the whole string. The type of every one of these is `&str`, pronounced "string slice", and it is the type of a string literal too: `"hello"` is a `&str` pointing into the program's own binary.

Now `first_word` can return the word itself, tied to the string it came from:

```
fn first_word s: &str -> &str:
    let bytes = s <- as_bytes$

    for (i, &item) in bytes <- iter$ <- enumerate$:
        if item == b' ':
            return &s[0..i]

    &s[..]

fn main$:
    let s = String.from "hello world"
    let word = first_word (&s)    // a &String coerces to &str
    println! "{}" word

    let literal = "hello world"   // a literal is already a &str
    println! "{}" (first_word literal)
    println! "{}" (first_word (&literal[6..]))
```

```text
hello
hello
world
```

The signature takes `&str`, not `&String`, and that is the more useful signature: a `&String` converts to a `&str` automatically — you see it happen at `first_word (&s)` — so the function accepts a `String`, a literal, and a slice of either. Write your functions over `&str` and everyone can call them. (The last call slices a literal: `&literal[6..]` is itself a `&str`, and it is isolated as an argument by the usual rule.)

And now the bug from the start of the section is caught, by the borrowing rule we already have:

```
fn first_word s: &str -> &str:
    let bytes = s <- as_bytes$

    for (i, &item) in bytes <- iter$ <- enumerate$:
        if item == b' ':
            return &s[0..i]

    &s[..]

fn main$:
    let mut s = String.from "hello world"
    let word = first_word (&s)
    s <- clear$                 // error: word still borrows s
    println! "{}" word
```

```text
error[E0502]: cannot borrow `s` as mutable because it is also borrowed as immutable
  --> slice_stale.hrs:13:5
   |
12 |     let word = first_word (&s)
   |                            -- immutable borrow occurs here
13 |     s <- clear$                 // error: word still borrows s
   |     ^^^^^^^^^^^ mutable borrow occurs here
14 |     println! "{}" word
   |                   ---- immutable borrow later used here
```

`word` is a slice of `s`, which is an immutable borrow of `s`; `clear` needs a mutable borrow to empty the string; the two cannot coexist while `word` is still used. The index version compiled and lied. The slice version does not compile, and the error names the line. This is the pattern of the whole chapter: an idea that was a bug in one form becomes a type error in another, and the compiler catches at build time what the earlier form would have caught in production, or not at all.

### Other slices

Slices are not only for strings. Part of an array is `&[i32]`, a slice of integers, and it works the same way:

```
fn main$:
    let a = [1, 2, 3, 4, 5]
    let middle: &[i32] = &a[1..4]
    println! "{:?}" middle
    println! "{}" (middle <- len$)
```

```text
[2, 3, 4]
3
```

`&[T]` is the slice type for any element type, and it is what you take as a parameter when a function wants "some contiguous `T`s" without caring whether they came from an array or a `Vec`.

## 4.4 What you have

One owner per value, dropped at the end of its scope. Assignment and function calls **move** unless the type is `Copy`; `clone` when you want a real copy. `&` borrows without owning, `&mut` borrows to change, and at any time there is either one `&mut` or any number of `&` — measured from creation to last use. A reference can never outlive what it refers to. And a slice is a borrow of part of something, which is why the compiler can tell when the something changes underneath it.

Nothing in this chapter was Harsh. That is the thesis: the notation was quiet, and Rust was all there was to learn. Next, a way to give your own types a shape — structs.

# 5. Structs

A *struct* is a type you define: a name for a bundle of values that belong together — a user, a rectangle, a request. Chapter 4's rules apply to a struct exactly as to a `String`: a struct owns its fields, is moved on assignment unless it is `Copy`, and is borrowed with `&`. This chapter is the definitions, the ways of building a value, and the methods that give a type its behaviour.

There are three forms of struct, and they are three equals, not one standard form and two odd ones. A **record struct** has named fields — Rust's own documentation calls this one simply "a struct", so that is what you will see it called elsewhere. A **tuple struct** has fields by position, with no names of their own. A **unit-like struct** has no fields at all. Every one of them can be declared, built and taken apart, and chapter 6 will show that each variant of an enum is one of these three.

This is also the first chapter where Harsh has a shape of its own to show. A record struct's fields are lines under the heading, one per line, with no commas and no braces, the way a function's body sits under its signature; a tuple struct's field types follow its name the way arguments follow a function; a unit-like struct is its name and nothing more. A *value* of a record struct is laid out as its declaration is, with a `\` after the name to say that a field list follows — the one mark this chapter adds.

## 5.1 The three forms

```
// A record struct: named fields, one per line...
struct Point
    x: f64
    y: f64

// ...or on the declaration's line, after the mark
struct Size\ w: f64, h: f64

// A tuple struct: fields by position, the name applied to their types
struct Meters f64

// A unit-like struct: a name and nothing else
struct Origin

fn main$:
    let p = Point\ x = 1.0, y = 2.0
    let s =
        Size\
            w = 3.0
            h = 4.0
    let d = Meters 5.5
    let o = Origin

    println! "{} {} {} {}" (p <- x) (s <- h) (d.0) (matches! o Origin)
```

```text
1 4 5.5 true
```

> **Harsh —** A record struct's fields may sit beneath the name, one per line, or on the declaration's own line after a `\` — `struct Size\ w: f64, h: f64` — and the same choice exists for a value: `Point\ x = 1.0, y = 2.0` on one line, or `Size\` with a field per line beneath. Neither is the preferred form; the block reads better when the fields are many or their names long, the inline form when the type is small, and this book uses whichever the example reads best in. A tuple struct is the name *applied* to its field types, `struct Meters f64`, exactly as `Meters 5.5` applies it to a value. A unit-like struct is a name, and `Origin` is also its only value.

### Record structs

```
struct User
    active: bool
    username: String
    email: String
    sign_in_count: u64

fn main$:
    let mut user1 =
        User\
            active = true
            username = String.from "someusername123"
            email = String.from "someone@example.com"
            sign_in_count = 1
    user1 <- email = String.from "anotheremail@example.com"
    println!
        "{} ({}), active: {}, sign-ins: {}"
        (user1 <- username)
        (user1 <- email)
        (user1 <- active)
        (user1 <- sign_in_count)
```

```text
someusername123 (anotheremail@example.com), active: true, sign-ins: 1
```

The definition is `struct User` and four indented fields, each `name: Type`. Nothing marks the body: only a name can follow `struct`, so a deeper line can only be a field. An *instance* is the struct's name, a `\`, and every field given a value with `=` — a line each here, or all on one line after the mark, `User\ active = true, …`. Order does not matter, but every field must be present, and the `\` is what says the name is not finished: a bare `User` is already a complete expression. Inline, the field list reaches the end of its line, or the `)` of the group it is written in — so `Point\ x = 1, msg` is one literal with the shorthand field `msg`, with or without parens around it, and the compiler is the one to say whether `msg` is a field `Point` has. When a literal is one element of a tuple, give it its own parens: `((Point\ x = 1), msg)`. A field is read with `<-`: `user1 <- email`. The whole instance is mutable or it is not — `let mut user1` — and then a field is assigned with `<-` on the left of `=`. There is no way to mark a single field mutable, and this is deliberate: mutability is a property of the binding, and a reader who sees `let user1` knows the entire value is fixed.

Notice the parentheses around `(user1 <- username)` when it is handed to `println!`. A field access has an operator in it, so, like every other operator expression, it is isolated when it is an argument; `println! "{}" user1 <- username` would read as `println!` applied to `user1`, followed by an arrow the macro does not want. The rule is chapter 3's and chapter 4's, applied again.

### Building a struct in a function

A function that returns a struct is the ordinary way to construct one with defaults:

```
struct User
    active: bool
    username: String
    email: String
    sign_in_count: u64

fn build_user (email: String) (username: String) -> User:
    User\
        active = true
        username            // shorthand: the field and the variable share a name
        email
        sign_in_count = 1

fn main$:
    let u =
        build_user
            (String.from "someone@example.com")
            (String.from "someusername123")
    println! "{} <{}>" (u <- username) (u <- email)
```

```text
someusername123 <someone@example.com>
```

`username,` and `email,` inside the literal are the *field init shorthand*: when the variable has the same name as the field, the field name alone does both jobs. Not required, but universal in real code, and it is why parameters in constructors tend to be named after fields.

### Building one instance from another

Often a new instance is an old one with a few fields changed. The *struct update syntax*, `..user1`, says "and every field I did not mention comes from `user1`":

```
struct User
    active: bool
    username: String
    email: String
    sign_in_count: u64

fn main$:
    let user1 =
        User\
            active = true
            username = String.from "someusername123"
            email = String.from "someone@example.com"
            sign_in_count = 1
    let user2 =
        User\
            email = String.from "another@example.com"
            ..user1             // every other field comes from user1
    println!
        "{} <{}> {}"
        (user2 <- username)
        (user2 <- email)
        (user2 <- sign_in_count)
    println! "{}" (user1 <- active)
```

```text
someusername123 <another@example.com> 1
true
```

It must be last in the literal. And it moves: `..user1` copies the `Copy` fields (`active`, `sign_in_count`) and *moves* the rest (`username`), exactly as `let x = user1.username` would. After it, `user1` is partly gone — `user1 <- active` is still usable, `user1 <- username` is not:

```
struct User
    active: bool
    username: String
    email: String
    sign_in_count: u64

fn main$:
    let user1 =
        User\
            active = true
            username = String.from "someusername123"
            email = String.from "someone@example.com"
            sign_in_count = 1
    let user2 =
        User\
            email = String.from "another@example.com"
            ..user1
    println! "{}" (user1 <- username)    // username was moved into user2
    println! "{}" (user2 <- username)
```

```text
error[E0382]: borrow of moved value: `user1.username`
  --> update_move.hrs:18:20
   |
15 |         User\
   |         ----- value moved here
18 |     println! "{}" (user1 <- username)    // username was moved into user2
   |                    ^^^^^^^^^^^^^^^^^ value borrowed here after move
   = note: move occurs because `user1.username` has type `String`, which does not implement the `Copy` trait
```

The compiler tracks moves per field. That is the chapter 4 rule at a finer grain, and the error says which field went where.

### Tuple structs

A tuple struct has a name and positional fields with no names of their own — a tuple with a type:

```
struct Color i32 i32 i32
struct Point i32 i32 i32

fn main$:
    let black = Color 0 0 0
    let origin = Point 0 0 0
    println! "{} {} {}" black.0 black.1 black.2

    let Point x y z = origin
    println! "{} {} {}" x y z
```

```text
0 0 0
0 0 0
```

`struct Color i32 i32 i32` declares: the field types follow the name, juxtaposed, the way arguments follow a function. `Color 0 0 0` constructs, by juxtaposition, three arguments — the same rule as a function call, because in Rust a tuple struct's name *is* a constructor function. Fields are `black.0`, `black.1`: a tuple index keeps its dot and is part of the name, so `black.0` is an atom and needs no parentheses as an argument. And a value is taken apart with a pattern of the same shape as the constructor, `let Point x y z = origin`. `Color` and `Point` have the same fields and are different types; a function taking a `Color` will not accept a `Point`, which is the point of naming them.

### Unit-like structs

A struct can have no fields at all:

```
struct AlwaysEqual

fn main$:
    let subject = AlwaysEqual
    let _ = subject
    println! "made one"
```

```text
made one
```

`struct AlwaysEqual` is the whole definition — nothing after the name — and `AlwaysEqual` is also its only value. Such a type exists to carry behaviour (chapter 10 puts traits on one) rather than data.

### Ownership of a struct's data

Every `String` in `User` is owned by the instance: when the instance is dropped, its strings are. That is why the fields were `String` and not `&str`. Try the reference:

```
struct User
    username: &str
    email: &str

fn main$:
    let user1 =
        User\
            username = "someusername123"
            email = "someone@example.com"
    println! "{}" (user1 <- username)
```

```text
error[E0106]: missing lifetime specifier
  --> str_field.hrs:2:15
   |
 2 |     username: &str
   |               ^ expected named lifetime parameter
   = help: consider introducing a named lifetime parameter (hrs 1:12)

error[E0106]: missing lifetime specifier
  --> str_field.hrs:3:12
   |
 3 |     email: &str
   |            ^ expected named lifetime parameter
   = help: consider introducing a named lifetime parameter (hrs 1:12)
```

A struct holding a reference must say how long the referenced data lives — the *lifetime* the error asks for — so that the compiler can check the struct never outlives what it points at. That is chapter 10; until then, structs own their data, which is the common case and the one that needs no annotation.

## 5.2 An example program

Here is a small program built three times, to show what a struct does for readability. Compute the area of a rectangle, first with two separate variables:

```
fn area (width: u32) (height: u32) -> u32:
    width * height

fn main$:
    let width1 = 30
    let height1 = 50
    println!
        "The area of the rectangle is {} square pixels."
        (area width1 height1)
```

```text
The area of the rectangle is 1500 square pixels.
```

It works, and `area` takes two parameters that are related — they are one rectangle's width and height — without saying so. Group them in a tuple:

```
fn area dimensions: (u32, u32) -> u32:
    dimensions.0 * dimensions.1

fn main$:
    let rect1 = (30, 50)
    println!
        "The area of the rectangle is {} square pixels."
        (area rect1)
```

```text
The area of the rectangle is 1500 square pixels.
```

Now one argument, but `dimensions.0` and `dimensions.1` are worse than `width` and `height`: the reader has to know which is which, and so does the next person who edits it. Name them:

```
struct Rectangle
    width: u32
    height: u32

fn area rectangle: &Rectangle -> u32:
    rectangle <- width * rectangle <- height

fn main$:
    let rect1 = Rectangle\ width = 30, height = 50
    println!
        "The area of the rectangle is {} square pixels."
        (area (&rect1))
```

```text
The area of the rectangle is 1500 square pixels.
```

`area` now takes `&Rectangle` — a borrow, so `main` keeps `rect1` — and reads `rectangle <- width * rectangle <- height`. The code says what it means. Application binds tighter than `<-`, so the two accesses are the operands of `*` and nothing needs isolating on that line; only the call `(area (&rect1))` in the `println!` does, twice, for the two operators in it.

### Printing a struct

`println!` with `{}` knows how to print numbers and strings; it does not know how to print a `Rectangle`, and it says so:

```
struct Rectangle
    width: u32
    height: u32

fn main$:
    let rect1 = Rectangle\ width = 30, height = 50
    println! "rect1 is {:?}" rect1
```

```text
error[E0277]: `Rectangle` doesn't implement `Debug`
  --> no_debug.hrs:7:30
   |
 7 |     println! "rect1 is {:?}" rect1
   |                              ^^^^^ `Rectangle` cannot be formatted using `{:?}`
   = help: the trait `Debug` is not implemented for `Rectangle`
   = note: add `#[derive(Debug)]` to `Rectangle` or manually `impl Debug for Rectangle`
   = help: consider annotating `Rectangle` with `#[derive(Debug)]` (hrs 1:1)
```

`{}` uses the `Display` trait, which is for user-facing output, and Rust will not guess what a user-facing rectangle looks like. The `{:?}` in the message is the *debug* format, which can be derived automatically — read the compiler's note, which tells you exactly what to add:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

fn main$:
    let rect1 = Rectangle\ width = 30, height = 50
    println! "rect1 is {:?}" rect1
    println! "rect1 is {:#?}" rect1
```

```text
rect1 is Rectangle { width: 30, height: 50 }
rect1 is Rectangle {
    width: 30,
    height: 50,
}
```

`#[derive Debug]` is an *attribute*: a line above the definition that asks the compiler to generate an implementation of `Debug` for the type. Inside its brackets an attribute is an application like any other — `derive` applied to `Debug`, and to more, `#[derive Debug Clone PartialEq]`; an attribute that takes a keyed value isolates it, `#[cfg (feature = "fast")]`. `{:?}` prints on one line; `{:#?}` pretty-prints with each field on its own. You will derive `Debug` on almost every struct you write. There is also `dbg!`, a macro that prints an expression *and its value* to standard error along with the file and line, and hands the value back — `dbg! (30 * scale)` inside a larger expression is how you look at an intermediate result without restructuring the code.

## 5.3 Methods

A *method* is a function that belongs to a type. It is defined inside an `impl` block and its first parameter is `self`, the instance it is called on:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

impl Rectangle:
    fn area (&self) -> u32:
        self <- width * self <- height

    fn width (&self) -> bool:
        self <- width > 0

fn main$:
    let rect1 = Rectangle\ width = 30, height = 50
    println!
        "The area of the rectangle is {} square pixels."
        (rect1 <- area$)

    if rect1 <- width$:
        println!
            "The rectangle has a nonzero width; it is {}"
            (rect1 <- width)
```

```text
The area of the rectangle is 1500 square pixels.
The rectangle has a nonzero width; it is 30
```

`impl Rectangle:` opens the block that holds the type's methods. `fn area (&self) -> u32:` is `area` with one parameter, `&self` — a borrow of the instance, in its own group — and the body reads the fields through it. The call is `rect1 <- area$`: the arrow selects the method on `rect1`, `$` applies it to nothing, and `self` is `&rect1`. `rect1 <- area$` is the whole call, so as an argument it is isolated like any expression with operators in it.

`&self` is the choice you make most: read, do not consume. `&mut self` when the method changes the instance; plain `self` — taking ownership — rarely, for a method that transforms the value into something else and does not want the old one used again. The three are chapter 4's three ways of passing a value, applied to the receiver.

A method may have the same name as a field. `rect1 <- width$` is the method, `rect1 <- width` the field; the `$` tells them apart. The usual reason to do this is a *getter*: the field private, the method public, so the value can be read but not set from outside the module — chapter 7.

### More parameters

After `self`, a method's parameters are groups like any function's:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

impl Rectangle:
    fn area (&self) -> u32:
        self <- width * self <- height

    fn can_hold (&self) (other: &Rectangle) -> bool:
        self <- width > other <- width && self <- height > other <- height

fn main$:
    let rect1 = Rectangle\ width = 30, height = 50
    let rect2 = Rectangle\ width = 10, height = 40
    let rect3 = Rectangle\ width = 60, height = 45

    println! "Can rect1 hold rect2? {}" (rect1 <- can_hold (&rect2))
    println! "Can rect1 hold rect3? {}" (rect1 <- can_hold (&rect3))
```

```text
Can rect1 hold rect2? true
Can rect1 hold rect3? false
```

`fn can_hold (&self) (other: &Rectangle)` — the receiver, then one parameter. The call, `rect1 <- can_hold (&rect2)`, applies the method to one argument, isolated because of its `&`.

### Associated functions

A function in an `impl` block that does *not* take `self` is an *associated function*: it belongs to the type but not to an instance. Constructors are the classic case:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

impl Rectangle:
    fn square size: u32 -> Self:
        Self\ width = size, height = size

impl Rectangle:
    fn area (&self) -> u32:
        self <- width * self <- height

fn main$:
    let sq = Rectangle.square 3
    println! "{:?} has area {}" sq (sq <- area$)
```

```text
Rectangle { width: 3, height: 3 } has area 9
```

`fn square size: u32 -> Self:` — `Self` inside an `impl` is an alias for the type, so this returns a `Rectangle`. It is called with the path form, `Rectangle.square 3`: a dot, because this is a path into the type's namespace, not a method on a value. `String.from` and `String.new$` are the same thing, and now you know what they are. A type may have several `impl` blocks, as here; there is no reason to split them in a program this size, but it is legal, and chapter 10 shows when it is useful.

## 5.4 What you have

A struct is named fields under a heading; an instance is the name, a `\`, and its fields with `=`, with shorthand and `..other` to fill it. Tuple structs are constructed and matched by juxtaposition; a unit struct is a name alone. Structs own their data until chapter 10 says how they may borrow it. `#[derive Debug]` and `{:?}` to see one. `impl` holds the methods, `&self` reads, `&mut self` changes, `self` consumes, and `rect1 <- area$` calls; a function without `self` is associated with the type and called through its path, `Rectangle.square 3`.

Next: enums — types whose value is one of several named shapes — and `match`, which you met in chapter 2 and can now understand.

# 6. Enums and pattern matching

A struct says "all of these, together". An *enum* says "exactly one of these". A value of an enum type is one of a fixed set of named *variants*, and each variant may carry data of its own — so an enum is both a set of alternatives and, when the variants have payloads, a way of saying "this value is one of several shapes, and here is which". Rust programs are built on two of them, `Option` and `Result`, and on `match`, the construct that takes an enum apart and refuses to let you forget a case.

## 6.1 Defining an enum

An IP address is version four or version six — never both, never neither. That is an enum:

```
#[derive Debug]
enum IpAddrKind
    V4
    V6

fn route ip_kind: IpAddrKind:
    println! "routing {:?}" ip_kind

fn main$:
    let four = IpAddrKind.V4
    let six = IpAddrKind.V6
    route four
    route six
```

```text
routing V4
routing V6
```

`enum IpAddrKind` and two variants, one per line, no commas, the same layout as a struct's fields, and markless for the same reason: only a name can follow `enum`. A variant is named through the type, `IpAddrKind.V4`, with a dot, because it is a path: the variant lives inside the type's namespace. Both values have the type `IpAddrKind`, so one function takes either.

### Variants with data

The kind alone is not much use; an address has a value. A variant can carry one:

```
#[derive Debug]
enum IpAddr
    V4 u8 u8 u8 u8
    V6 String

fn main$:
    let home = IpAddr.V4 127 0 0 1
    let loopback = IpAddr.V6 (String.from "::1")
    println! "{:?} {:?}" home loopback
```

```text
V4(127, 0, 0, 1) V6("::1")
```

`V4 u8 u8 u8 u8` is a variant with four fields and `V6 String` a variant with one — the variant applied to its payload's types, exactly as a tuple struct is declared. Constructing is application: `IpAddr.V4 127 0 0 1` applies the variant to four arguments, and `IpAddr.V6 (String.from "::1")` to one, isolated because it is a call. Each variant is a constructor function for the enum, and the two variants may carry different types — something no struct can express.

Variants can take any shape a struct can — and that sentence is the whole of what an enum is. An enum is a *union* of variants, and each variant is one of chapter 5's three forms: a unit-like struct, a record struct, or a tuple struct, spelled exactly as the struct of that form is spelled, with the word `struct` and the name's own line taken away. (A type built this way — a choice among bundles — is what Haskell and its relatives call an *algebraic data type*; Rust's enums are that, with the word left out.)

```
#[derive Debug]
enum Message
    Quit                          // a unit-like struct

    Move                          // a record struct
        x: i32
        y: i32

    Write String                  // a tuple struct
    ChangeColor i32 i32 i32       // a tuple struct

impl Message:
    fn call (&self):
        println! "calling {:?}" self

fn main$:
    let msgs =
        [
            Message.Quit,
            (Message.Move\ x = 1, y = 2),
            (Message.Write (String.from "hi")),
            (Message.ChangeColor 0 160 255),
        ]

    for m in &msgs:
        m <- call$
```

```text
calling Quit
calling Move { x: 1, y: 2 }
calling Write("hi")
calling ChangeColor(0, 160, 255)
```

> **Harsh —** `Quit` is a unit-like struct; `Move` with `x` and `y` beneath it is a record struct; `Write String` and `ChangeColor i32 i32 i32` are tuple structs, the name applied to the payload's types. Each is written as chapter 5 wrote it, and each has the inline form chapter 5 had too — `Move\ x: i32, y: i32` on one line — so the same enum reads either way, and which to use is the same judgement as for a struct: what the names and their lengths make clearest.

```
enum Message
    Quit
    Move\ x: i32, y: i32
    Write String
    ChangeColor i32 i32 i32

fn main$:
    let m = Message.Move\ x = 1, y = 2
    if let Message.Move\ x, y = m:
        println! "moved to {x},{y}"
```

```text
moved to 1,2
```

`Quit` carries nothing; `Move` opens a record variant with named fields beneath it, exactly as a struct's are laid out; `Write String` and `ChangeColor i32 i32 i32` are tuple variants, their payload types juxtaposed after the name. A record variant's value is a literal like any other, `Message.Move\ x = 1, y = 2`. Without the enum this would be four separate struct types, and no function could take "a message" — with it, `impl Message:` gives all four a method, and `m <- call$` works on any of them. (`for m in &msgs` borrows the array so the messages are not moved out of it; chapter 8 makes that habit.)

### `Option`

Rust has no null. Where another language returns a value that might be null, Rust returns a value that might be *absent*, and says so in the type:

```
fn main$:
    let some_number = Some 5
    let some_char = Some 'e'
    let absent_number: Option<i32> = None
    println! "{:?} {:?} {:?}" some_number some_char absent_number
```

```text
Some(5) Some('e') None
```

`Option<T>` is an enum with two variants, `Some T` — there is a value, here it is — and `None`. It is so central that both variants are in scope without a prefix: you write `Some 5` and `None`, not `Option.Some 5`. The type of `absent_number` has to be written, because `None` alone does not say what kind of value is absent.

What `Option` buys you is that an `Option<i8>` is *not* an `i8`, and the compiler will not let you treat it as one:

```
fn main$:
    let x: i8 = 5
    let y: Option<i8> = Some 5
    let sum = x + y
    println! "{}" sum
```

```text
error[E0277]: cannot add `Option<i8>` to `i8`
  --> option_add.hrs:4:17
   |
 4 |     let sum = x + y
   |                 ^ no implementation for `i8 + Option<i8>`
   = help: the trait `Add<Option<i8>>` is not implemented for `i8`
   = help: the following other types implement trait `Add<Rhs>`:
  <i8 as Add>
  <i8 as Add<&i8>>
  <&'a i8 as Add<i8>>
  <&i8 as Add<&i8>>
```

In a language with null, `x + y` compiles and fails at runtime when `y` happens to be null. Here it does not compile, because a value that might be absent cannot be added until you have said what happens when it is. To use the `i8` inside, you have to take the `Option` apart, and handling the `None` case is part of taking it apart. That is the whole idea, and `match` is how it is done.

## 6.2 `match`

`match` takes a value and a list of *arms*, each a pattern and the code to run when the pattern fits. The first arm whose pattern matches wins:

```
enum Coin
    Penny
    Nickel
    Dime
    Quarter

fn value_in_cents coin: Coin -> u8:
    match coin:
        Coin.Penny => do:
            println! "Lucky penny!"
            1
        Coin.Nickel => 5
        Coin.Dime => 10
        Coin.Quarter => 25

fn main$:
    println! "{}" (value_in_cents Coin.Penny)
    println! "{}" (value_in_cents Coin.Quarter)
```

```text
Lucky penny!
1
25
```

`match coin:` opens the arms; each is `pattern => expression`, one per line. An arm's expression is its value, and the `match` is worth whichever arm ran — so `value_in_cents` returns the `u8` from the arm that matched. An arm that needs several statements opens a block with `=> do:`, and the block's last line is its value, as everywhere.

> **Harsh —** One arm per line, and no comma after it: the line ending is what separates arms, and a `,` at the end of one is an error naming the newline. Commas separate arms only when several share a line.

### Patterns that bind

When a variant carries data, the pattern names it and the arm can use it:

```
#[derive Debug]
enum UsState
    Alabama
    Alaska

enum Coin
    Penny
    Nickel
    Dime
    Quarter UsState

fn value_in_cents coin: Coin -> u8:
    match coin:
        Coin.Penny => 1
        Coin.Nickel => 5
        Coin.Dime => 10
        Coin.Quarter state => do:
            println! "State quarter from {:?}!" state
            25

fn main$:
    println! "{}" (value_in_cents (Coin.Quarter UsState.Alaska))
```

```text
State quarter from Alaska!
25
```

`Coin.Quarter state` — the pattern has the shape of the constructor, with a variable where the value goes. When a `Quarter` matches, `state` is bound to the `UsState` inside it for the length of the arm. This is the same juxtaposition that constructs: `Coin.Quarter UsState.Alaska` builds one, `Coin.Quarter state` takes one apart. A pattern is an application throughout — `Some n`, `Ok value`, `Circle r` — and a record variant is taken apart with the same mark that builds it, `Move\ x, y`, as chapter 5 did for a struct.

### Matching `Option`

Now the `Option` from the previous section can be used:

```
fn plus_one x: Option<i32> -> Option<i32>:
    match x:
        None => None
        Some i => Some (i + 1)

fn main$:
    let five = Some 5
    let six = plus_one five
    let none = plus_one None
    println! "{:?} {:?}" six none
```

```text
Some(6) None
```

`None => None` and `Some i => Some (i + 1)`: two arms, two variants, and inside the second `i` is the `i32`, so `i + 1` is ordinary arithmetic; `Some (i + 1)` wraps the result back up, its argument isolated because of the `+`. Combining `match` and enums this way is how most Rust code handles values that might not be there; after a while you stop noticing it.

### Matches are exhaustive

Leave a case out and the compiler stops you:

```
fn plus_one x: Option<i32> -> Option<i32>:
    match x:
        Some i => Some (i + 1)

fn main$:
    println! "{:?}" (plus_one (Some 5))
```

```text
error[E0004]: non-exhaustive patterns: `None` not covered
  --> non_exhaustive.hrs:2:11
   |
 2 |     match x:
   |           ^ pattern `None` not covered
   = note: `Option<i32>` defined here
   = note: the matched value is of type `Option<i32>`
   = help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown (hrs 3:31)
```

Every possible value must be covered by some arm. This is what makes `Option` safe: you cannot forget the `None` case, because forgetting it is a compile error. The same holds for your own enums — add a variant, and every `match` that does not handle it fails to build, which is exactly the list of places you needed to look.

### Catch-all patterns

When only some values matter, the last arm can catch the rest:

```
fn add_fancy_hat$:
    println! "fancy hat"
fn remove_fancy_hat$:
    println! "no hat"
fn move_player spaces: u8:
    println! "move {} spaces" spaces

fn main$:
    for dice_roll in [3, 7, 4]:
        match dice_roll:
            3 => add_fancy_hat$
            7 => remove_fancy_hat$
            other => move_player other

    // when the value is not needed, `_` matches anything and binds nothing

    let roll = 9

    match roll:
        3 => add_fancy_hat$
        7 => remove_fancy_hat$
        _ => ()
```

```text
fancy hat
no hat
move 4 spaces
```

`other => move_player other` binds whatever did not match `3` or `7` to `other` and uses it. When the value is not needed, `_` matches anything and binds nothing, and `_ => ()` says "and do nothing" — `()` is the unit value, and a `match` whose arms are all unit is a statement. The catch-all must be last, since arms are tried in order and nothing after it could ever run.

## 6.3 `if let` and `let … else`

A `match` with one arm that matters and a `_ => ()` is a lot of lines for "if this is a `Some`, do this":

```
fn main$:
    let config_max = Some 3u8

    // The match way: one arm that matters, one that does nothing.
    match config_max:
        Some max => println! "The maximum is configured to be {max}"
        _ => ()

    // The `if let` way: the same, in one line.

    if let Some max = config_max:
        println! "The maximum is configured to be {max}"
```

```text
The maximum is configured to be 3
The maximum is configured to be 3
```

`if let Some max = config_max:` is a condition that is also a pattern: if `config_max` matches `Some max`, bind `max` and run the block. It is `match` with one arm and no exhaustiveness check — you are choosing to ignore the other cases, and the syntax says so. Use it when one case is all you want; use `match` when forgetting a case would be a bug.

`if let` takes an `else`, which runs for everything the pattern did not match:

```
#[derive Debug]
enum UsState
    Alabama
    Alaska

enum Coin
    Penny
    Quarter UsState

fn main$:
    let coins = [Coin.Penny, (Coin.Quarter UsState.Alabama), Coin.Penny]
    let mut count = 0

    for coin in coins:
        if let Coin.Quarter state = coin:
            println! "State quarter from {:?}!" state

        else:
            count += 1

    println! "{} non-quarters" count
```

```text
State quarter from Alabama!
2 non-quarters
```

Sometimes the pattern is a guard at the top of a function: if the value has the right shape, carry on with its contents; otherwise leave. `let … else` is that, without nesting the rest of the function inside an `if`:

```
#[derive Debug]
enum UsState
    Alabama
    Alaska

impl UsState:
    fn existed_in (&self) (year: u16) -> bool:
        match self:
            UsState.Alabama => year >= 1819
            UsState.Alaska => year >= 1959

enum Coin
    Penny
    Quarter UsState

fn describe_state_quarter coin: Coin -> Option<String>:
    let Coin.Quarter state = coin else:
        return None

    if state <- existed_in 1900:
        Some (format! "{state:?} is pretty old, for America!")
    else:
        Some (format! "{state:?} is relatively new.")

fn main$:
    println!
        "{:?}"
        (describe_state_quarter (Coin.Quarter UsState.Alabama))
    println!
        "{:?}"
        (describe_state_quarter (Coin.Quarter UsState.Alaska))
    println! "{:?}" (describe_state_quarter Coin.Penny)
```

```text
Some("Alabama is pretty old, for America!")
Some("Alaska is relatively new.")
None
```

`let Coin.Quarter state = coin else:` — if `coin` is a `Quarter`, `state` is bound *for the rest of the function*, not just for a block; if it is not, the `else` runs, and the `else` must leave (here with `return`), because there is no `state` to continue with. The happy path stays flat, which is the reason this form exists. The `else` block may also be inline, `else: return None`, when it is one expression.

## 6.4 What you have

An enum is one of several named variants, each carrying nothing, a tuple, or named fields; variants are constructed and matched by juxtaposition, `IpAddr.V4 127 0 0 1`, `Coin.Quarter state`. `Option<T>` is `Some T` or `None` and is Rust's null, made safe by the type. `match` takes a value apart with patterns, binds what is inside, and must cover every case; `_` and `other` catch the rest. `if let` for one case, `let … else` to guard and continue.

Next: modules — how a program larger than one file is organised, and what `pub` and `use` actually do.

# 7. Packages, crates and modules

Every program so far fitted in one file. Real ones do not, and Rust's answer to "where does this code live and who may call it" has three layers: a *package* is what `hrs new` makes — a `Cargo.toml` and a `src/` directory; a *crate* is what the compiler builds from it, a binary or a library; and inside a crate, *modules* group items and decide which of them are visible from outside. This chapter is the module system: how to declare a module, how to name a thing inside one, what `pub` opens, what `use` shortens, and how a module moves into its own file.

## 7.1 Packages and crates

A crate is the unit of compilation: `rustc` is given one file — the *crate root*, `src/main.hrs` for a binary or `src/lib.hrs` for a library — and everything the crate contains is reached from there through `mod` declarations. A package holds at most one library crate and any number of binaries, and `Cargo.toml` names them; `hrs new hello` writes a package with one binary whose root is `src/main.hrs`, and `hrs run` transpiles the tree under `src/` and hands `cargo` the result. You met the arrangement in chapter 1; this chapter is what goes *in* the tree.

## 7.2 Modules

A module is a named scope for items — functions, structs, enums, other modules — declared with `mod` and a block:

```
mod front_of_house:
    mod hosting:
        fn add_to_waitlist$:
            println! "added to waitlist"

        fn seat_at_table$:
            println! "seated"

    mod serving:
        fn take_order$:
            println! "order taken"

        fn serve_order$:
            println! "served"

        fn take_payment$:
            println! "paid"

fn main$:
    println! "the restaurant is open"
```

```text
the restaurant is open
```

`mod front_of_house:` opens a module; `mod hosting:` inside it opens a nested one, and the functions sit inside that. The layout is the same as everywhere: a heading, and the contents indented beneath. The result is a *module tree* rooted at the crate:

```text
crate
 └── front_of_house
     ├── hosting
     │   ├── add_to_waitlist
     │   └── seat_at_table
     └── serving
         ├── take_order
         ├── serve_order
         └── take_payment
```

Modules are for organising code the way directories organise files, and — the part that matters — for *privacy*: everything inside a module is private to that module and its descendants unless it is marked otherwise.

## 7.3 Paths

To call something in a module you name its path, the way you name a file in a directory. A path is either *absolute*, starting from `crate`, or *relative*, starting from the current module; the segments are joined with `.`:

```
mod front_of_house:
    mod hosting:
        fn add_to_waitlist$:
            println! "added to waitlist"

fn eat_at_restaurant$:
    crate.front_of_house.hosting.add_to_waitlist$    // absolute path
    front_of_house.hosting.add_to_waitlist$          // relative path

fn main$:
    eat_at_restaurant$
```

```text
error[E0603]: module `hosting` is private
  --> private_path.hrs:7:26
   |
 7 |     crate.front_of_house.hosting.add_to_waitlist$    // absolute path
   |                          ^^^^^^^ private module
   |                                  --------------- function `add_to_waitlist` is not publicly re-exported
   = note: the module `hosting` is defined here (hrs 2:5)

error[E0603]: module `hosting` is private
  --> private_path.hrs:8:20
   |
 8 |     front_of_house.hosting.add_to_waitlist$          // relative path
   |                    ^^^^^^^ private module
   |                            --------------- function `add_to_waitlist` is not publicly re-exported
   = note: the module `hosting` is defined here (hrs 2:5)
```

Both paths are correctly spelled and the program does not build, because `hosting` is private. The rule: a child module can see everything in its ancestors, but a parent cannot see inside a child unless the child says so. `eat_at_restaurant` lives in the crate root, which is `front_of_house`'s parent, so it can see `front_of_house` (siblings are visible); but `hosting` is inside `front_of_house` and was not made public, so the path stops there. The error names the exact segment. The reason for the default is that a module's insides are its implementation; what it chooses to expose is its interface, and changing an interface should be a decision, not an accident.

`pub` is that decision:

```
mod front_of_house:
    pub mod hosting:
        pub fn add_to_waitlist$:
            println! "added to waitlist"

fn eat_at_restaurant$:
    crate.front_of_house.hosting.add_to_waitlist$    // absolute path, from the crate root
    front_of_house.hosting.add_to_waitlist$          // relative path, from here

fn main$:
    eat_at_restaurant$
```

```text
added to waitlist
added to waitlist
```

`pub mod hosting` lets the parent see the module; `pub fn add_to_waitlist` lets it call the function. Both are needed — making a module public does not make its contents public, only reachable. Now the absolute path `crate.front_of_house.hosting.add_to_waitlist$` and the relative `front_of_house.hosting.add_to_waitlist$` both work, and end in `$` because they are calls. Prefer the absolute path when the caller and the callee are likely to move independently; prefer the relative one when they will move together.

### `super`

A path may also start from the *parent* module, with `super`:

```
fn deliver_order$:
    println! "delivered"

mod back_of_house:
    pub fn fix_incorrect_order$:
        cook_order$
        super.deliver_order$      // one level up: the crate root

    fn cook_order$:
        println! "cooked"

fn main$:
    back_of_house.fix_incorrect_order$
```

```text
cooked
delivered
```

`super.deliver_order$` from inside `back_of_house` reaches the crate root, where `deliver_order` lives. It is the module system's `..`: use it when a child depends on something in its parent and the two will stay together, so the relation "one level up" is the stable one to write down.

### Public structs and enums

`pub` on a struct makes the *type* public, and each field is still private unless it too is marked:

```
mod back_of_house:
    pub struct Breakfast
        pub toast: String
        seasonal_fruit: String

    impl Breakfast:
        pub fn summer toast: &str -> Breakfast:
            Breakfast\
                toast = String.from toast
                seasonal_fruit = String.from "peaches"

fn main$:
    // A public constructor is the only way in, since one field is private.
    let mut meal = back_of_house.Breakfast.summer "Rye"
    meal <- toast = String.from "Wheat"       // public field: readable and settable
    println! "I'd like {} toast please" (meal <- toast)
```

```text
I'd like Wheat toast please
```

`toast` is public and `seasonal_fruit` is not, so outside `back_of_house` a `Breakfast` can be read and written through `toast` alone — and cannot be constructed with a literal at all, since a literal has to give every field. That is why `summer` exists: a public associated function is the way to build a struct that has a private field, and it is where the module gets to choose the default. Try the private field from outside:

```
mod back_of_house:
    pub struct Breakfast
        pub toast: String
        seasonal_fruit: String

    impl Breakfast:
        pub fn summer toast: &str -> Breakfast:
            Breakfast\
                toast = String.from toast
                seasonal_fruit = String.from "peaches"

fn main$:
    let mut meal = back_of_house.Breakfast.summer "Rye"
    meal <- seasonal_fruit = String.from "blueberries"
```

```text
error[E0616]: field `seasonal_fruit` of struct `Breakfast` is private
  --> private_field.hrs:14:13
   |
14 |     meal <- seasonal_fruit = String.from "blueberries"
   |             ^^^^^^^^^^^^^^ private field
```

An enum is the opposite: `pub enum` makes every variant public, because an enum with hidden variants would be one you could not match on:

```
mod back_of_house:
    #[derive Debug]
    pub enum Appetizer
        Soup
        Salad

fn main$:
    let order1 = back_of_house.Appetizer.Soup     // variants are public with the enum
    let order2 = back_of_house.Appetizer.Salad
    println! "{:?} {:?}" order1 order2
```

```text
Soup Salad
```

## 7.4 `use`

Writing the full path at every call is tedious, and `use` brings a path into scope once:

```
mod front_of_house:
    pub mod hosting:
        pub fn add_to_waitlist$:
            println! "added to waitlist"

use crate.front_of_house.hosting          // bring the module in, not the function

fn eat_at_restaurant$:
    hosting.add_to_waitlist$              // the parent says where it came from

fn main$:
    eat_at_restaurant$
```

```text
added to waitlist
```

After `use crate.front_of_house.hosting`, the name `hosting` is in scope in this module and `hosting.add_to_waitlist$` works. This is also the idiomatic *depth* to import at for a function: bring in the parent module, not the function, so that every call says where it came from — `hosting.add_to_waitlist$` reads as "hosting's", where a bare `add_to_waitlist$` would look local. For structs, enums and traits the convention is the opposite — import the type itself, `use std.collections.HashMap` — since a type's name is meant to be used bare.

A `use` is scoped to the module it appears in. It does not reach into a child module:

```
mod front_of_house:
    pub mod hosting:
        pub fn add_to_waitlist$:
            println! "added to waitlist"

use crate.front_of_house.hosting

mod customer:
    pub fn eat_at_restaurant$:
        hosting.add_to_waitlist$          // `use` above is in the parent, not here

fn main$:
    customer.eat_at_restaurant$
```

```text
error[E0433]: failed to resolve: use of undeclared crate or module `hosting`
  --> use_scope.hrs:10:9
   |
10 |         hosting.add_to_waitlist$          // `use` above is in the parent, not here
   |         ^^^^^^^ use of undeclared crate or module `hosting`
   = help: consider importing this module through its public re-export (hrs 9:5)
```

The `use` is in the crate root; `customer` is a child; the name is not in scope there. Either move the `use` into `customer` or write `super.hosting` from inside it.

### `as`

When two imports would have the same name, rename one:

```
use std.fmt.Result
use std.io.Result as IoResult             // two `Result`s: rename one

fn function1$ -> Result:
    Ok ()

fn function2$ -> IoResult<()>:
    Ok ()

fn main$:
    println! "{:?} {:?}" (function1$) (function2$ <- is_ok$)
```

```text
Ok(()) true
```

Both `std.fmt` and `std.io` define a `Result`. `use std.io.Result as IoResult` brings the second in under a different name; the `as` is Rust's, unchanged.

### Re-exporting with `pub use`

`use` brings a name in for *this* module. `pub use` brings it in and passes it on, so that users of this module see it as if it had been defined here:

```
mod restaurant:
    mod front_of_house:
        pub mod hosting:
            pub fn add_to_waitlist$:
                println! "added to waitlist"

    pub use front_of_house.hosting        // re-export: visible to users of `restaurant`

fn main$:
    restaurant.hosting.add_to_waitlist$   // the internal `front_of_house` is not mentioned
```

```text
added to waitlist
```

`restaurant`'s user calls `restaurant.hosting.add_to_waitlist$` and never learns that `hosting` is really inside `front_of_house`. Re-exporting is how a library presents a public structure different from its internal one — the way the code is organised for its authors and the way it is presented to its users need not be the same tree.

### Nested paths and globs

Several imports from one place can share their prefix:

```
use std.(cmp.Ordering, collections.HashMap)   // two paths sharing a prefix
use std.io.(self, Write)                      // the module itself, and one item from it
use std.collections.*                         // everything: the glob, for tests and preludes

fn main$:
    let mut m: HashMap<&str, i32> = HashMap.new$
    m <- insert "a" 1

    let s: HashSet<i32> = HashSet.new$        // from the glob
    let ord = 1 <- cmp (&2)
    let mut out = io.stdout$
    writeln! out "{:?} {:?} {}" ord (m <- get "a") (s <- len$)
        <- unwrap$

    let _ = Ordering.Less
```

```text
Less Some(1) 0
```

`use std.(cmp.Ordering, collections.HashMap)` is two imports; the parentheses group the branches of a `use` tree, which is one of the four things parentheses do in Harsh. `use std.io.(self, Write)` imports the module `io` itself *and* one item from it — `self` in a `use` tree means "the thing before the parentheses". And `use std.collections.*` imports everything the module exports: the *glob*. Globs make it hard to see where a name came from, so they are for two places — tests, which import everything from the module under test, and *preludes*, modules designed to be imported whole.

## 7.5 Separating modules into files

So far every module has had its body inline. As a program grows, a module goes into a file of its own, and the tree of modules becomes a tree of files:

`src/main.hrs`

```
use crate.garden.vegetables.Asparagus

pub mod garden      // the body is in src/garden.hrs

fn main$:
    let plant = Asparagus
    println! "I'm growing {plant:?}!"
```

`src/garden.hrs`

```
pub mod vegetables  // the body is in src/garden/vegetables.hrs
```

`src/garden/vegetables.hrs`

```
#[derive Debug]
pub struct Asparagus
```

```text
$ cargo run
I'm growing Asparagus!
```

Three files. `src/main.hrs` declares `pub mod garden` with no body — just the declaration on a line — and the compiler looks for the body in `src/garden.hrs`. That file declares `pub mod vegetables` the same way, and the body of *that* is found in `src/garden/vegetables.hrs`: a module's children go in a directory named after it. The `use` at the top of `main.hrs` is then the ordinary thing, a path through the tree to the item wanted.

Only the crate root ever needs to know the tree's shape; a file that declares a child module does not say where the child's file is, because there is only one place it can be. And nothing in the calling code changes when a module moves from inline to its own file — the path `crate.garden.vegetables.Asparagus` is the same either way. Moving code into files is a filing decision, and the module system keeps it from being anything more.

> **Harsh —** The file layout is not Harsh's invention and Harsh does not touch it: `hrs build` transpiles every file under `src/` into `target/hrs/`, tree for tree and name for name, so the compiler finds each module exactly where the declaration says it is.

## 7.6 What you have

A package is what `hrs new` makes and a crate is what the compiler builds from its root. `mod` opens a module; everything in it is private until `pub` says otherwise, and `pub` on a struct still leaves each field private. Paths are `crate.` from the root, or relative, or `super.` from the parent, with `.` between segments. `use` shortens a path for one module — import the parent for a function, the type for a type — `as` renames, `pub use` re-exports, `(…)` groups, `*` takes all. A bodiless `mod name` puts the body in `name.hrs`, and its children in `name/`.

Next: the collections the standard library gives you — `Vec`, `String` and `HashMap` — which is where ownership starts to earn its keep in daily code.

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

    match third:
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
        match cell:
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

# 9. Error handling

Rust has two kinds of error and refuses to blur them. An *unrecoverable* error is a bug — an index past the end, an invariant broken — and the program *panics*: it prints a message and stops. A *recoverable* error is an event the program should expect — a file that is not there, text that is not a number — and it is a *value*, of type `Result<T, E>`, which the caller has to look at. There are no exceptions, and no error can be silently ignored, because an ignored `Result` is an unused value the compiler warns about and an unhandled one does not type-check. This chapter is the two kinds, the `?` operator that makes the recoverable kind pleasant, and the judgement of which to use.

## 9.1 Unrecoverable errors with `panic!`

You can panic on purpose:

```
fn main$:
    println! "about to fail"
    panic! "crash and burn"
```

```text
about to fail
thread 'main' panicked at panic.hrs:3:
crash and burn
```

The program printed its first line, reached `panic!`, printed the message with the file and line, and exited. That is the whole mechanism. By default the panic *unwinds*: it walks back up the stack dropping every value on the way, so files close and memory is freed, and then the process ends. (A backtrace — the chain of calls that led here — is available with `RUST_BACKTRACE=1` in the environment; the book leaves it off.)

Most panics are not written; they are hit:

```
fn main$:
    let v = vec! [1, 2, 3]
    v[99];                          // the `;` discards the value; the index still runs
```

```text
thread 'main' panicked at panic_index.hrs:3:
index out of bounds: the len is 3 but the index is 99
```

Chapter 8's index rule, seen from the other side. In C this reads whatever happens to be at that address — a *buffer over-read*, the root of a large fraction of security holes. Rust checks, and panics, and the panic is not the bug: the index was. The panic is the report.

## 9.2 Recoverable errors with `Result`

Opening a file can fail for reasons that are nobody's bug. So `File.open` returns a `Result`:

```
enum Result<T, E>
    Ok T
    Err E
```

`Ok` carries the value when the operation worked, `Err` the error when it did not, and the caller takes it apart with `match`, exactly as it would an `Option`:

```
use std.fs.File

fn main$:
    let greeting_file_result = File.open "hello.txt"

    let greeting_file = match greeting_file_result:
        Ok file => file
        Err error => panic! "Problem opening the file: {error:?}"

    let _ = greeting_file
```

```text
thread 'main' panicked at result_match.hrs:8:
Problem opening the file: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

There is no `hello.txt`, so the `Err` arm ran and the program panicked with the error inside it. This is a recoverable error handled by choosing not to recover — which is fine for a program that cannot do anything without the file — and the point is that the choice was made in the code, visibly, rather than by the runtime.

### Matching on the kind of error

An `io.Error` says what went wrong, and a program can act on that:

```
use std.fs.File
use std.io.ErrorKind

fn main$:
    let greeting_file = match File.open "hello.txt":
        Ok file => file
        Err error => match error <- kind$:
            ErrorKind.NotFound => match File.create "hello.txt":
                Ok fc => fc
                Err e => panic! "Problem creating the file: {e:?}"
            _ => panic! "Problem opening the file: {error:?}"

    println!
        "{:?}"
        (greeting_file <- metadata$ <- map (|m| m <- is_file$))
    std.fs.remove_file "hello.txt" <- unwrap$
```

```text
Ok(true)
```

`error <- kind$` is an `ErrorKind`, an enum: on `NotFound`, create the file; on anything else, panic. Three `match`es nested, one inside the arm of the last — each `=> match … :` opens its arms on the lines beneath, and the layout keeps them apart where braces would have been a thicket. (The last line deletes the file the example created, so the book stays tidy.)

### `unwrap` and `expect`

`match` on every `Result` is verbose, and there are two shortcuts for "give me the value or panic":

```
use std.fs.File

fn main$:
    let f = File.open "hello.txt" <- unwrap$
    let _ = f
```

```text
thread 'main' panicked at unwrap_expect.hrs:4:
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

`unwrap$` is the `match` above with a generic message. `expect` is the same with *your* message:

```
use std.fs.File

fn main$:
    let f =
        File.open "hello.txt"
            <- expect "hello.txt should be included in this project"

    let _ = f
```

```text
thread 'main' panicked at expect.hrs:6:
hello.txt should be included in this project: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

Prefer `expect`: when it fires, the message says what you were assuming, and that is what you need to know first. Both are right when the `Err` case really cannot happen — a literal that you know parses — or in a quick program where stopping is the correct response. In library code they are a smell, since the library does not know what the caller wants done.

### Propagating errors

The usual answer is that the function that hit the error is not the one that knows what to do about it; it should hand the error up. That is *propagating*, and written by hand it looks like this:

```
use std.fs.File
use std.io.(self, Read)

fn read_username_from_file$ -> Result<String, io.Error>:
    let username_file_result = File.open "hello.txt"

    let mut username_file = match username_file_result:
        Ok file => file
        Err e => return Err e

    let mut username = String.new$

    match username_file <- read_to_string (&mut username):
        Ok _ => Ok username
        Err e => Err e

fn main$:
    println!
        "{:?}"
        (read_username_from_file$ <- map_err (|e| e <- kind$))
```

```text
Err(NotFound)
```

The function returns `Result<String, io.Error>`; each step that can fail is matched, and on `Err` the function returns it. The caller then decides. This is correct and it is half `match`; Rust has an operator for it.

### The `?` operator

`?` after a `Result` means: if it is `Ok`, give me the value; if it is `Err`, return it from this function right now.

```
use std.fs.File
use std.io.(self, Read)

fn read_username_from_file$ -> Result<String, io.Error>:
    let mut username = String.new$
    File.open "hello.txt"? <- read_to_string (&mut username)?
    Ok username

fn main$:
    println!
        "{:?}"
        (read_username_from_file$ <- map_err (|e| e <- kind$))
```

```text
Err(NotFound)
```

The same function in three lines. `File.open "hello.txt"?` is the file or an early return; `<- read_to_string (&mut username)?` likewise. The `?` binds tighter than `<-`, as it does in Rust, so it applies to the `open` before the chain continues — and the chain runs across the `?`, which reads naturally once you see it as "open, if that worked read into `username`, if that worked carry on". `?` also converts the error on the way out, through the `From` trait: a function returning `Result<_, MyError>` can use `?` on any error that `MyError` knows how to be made from, and chapter 10 shows `From`.

`?` works on `Option` too, returning `None` early:

```
fn last_char_of_first_line text: &str -> Option<char>:
    text <- lines$
         <- next$?
         <- chars$
         <- last$

fn main$:
    println!
        "{:?}"
        (last_char_of_first_line "Hello, world\nHow are you today?")
    println! "{:?}" (last_char_of_first_line "")
    println! "{:?}" (last_char_of_first_line "\nhi")
```

```text
Some('d')
None
None
```

`text <- lines$ <- next$?` is the first line, or `None` if there are no lines; then `<- chars$ <- last$` is the last character of it, or `None` if the line is empty. Two ways to have nothing, one expression. What `?` cannot do is mix: a `?` on an `Option` inside a function returning `Result` is an error, and the other way round. Convert first.

`?` can only be used in a function whose return type it fits — which means not in a `main` that returns nothing:

```
use std.fs.File

fn main$:
    let greeting_file = File.open "hello.txt"?
    let _ = greeting_file
```

```text
error[E0277]: the `?` operator can only be used in a function that returns `Result` or `Option` (or another type that implements `FromResidual`)
  --> question_in_main.hrs:4:46
   |
 3 | fn main$:
   | -------- this function should return `Result` or `Option` to accept `?`
 4 |     let greeting_file = File.open "hello.txt"?
   |                                              ^ cannot use the `?` operator in a function that returns `()`
   = help: the trait `FromResidual<Result<Infallible, std::io::Error>>` is not implemented for `()`
```

The compiler says exactly what is missing: `main` returns `()`, and `?` needs somewhere to send the error. `main` may return a `Result`:

```
use std.error.Error
use std.fs.File

fn main$ -> Result<(), Box<dyn Error>>:
    let greeting_file = File.open "hello.txt"
    println! "opened: {}" (greeting_file <- is_ok$)

    let n: i32 = "42" <- parse$?
    println! "{n}"
    Ok ()
```

```text
opened: false
42
```

`fn main$ -> Result<(), Box<dyn Error>>:` — a `main` that either succeeds with nothing or fails with some error. `Box<dyn Error>` is "any error at all", a trait object (chapter 18) that `?` can convert any error into, so this `main` can use `?` on a file operation and a parse in the same body. `Ok ()` at the end is the success value — `()` is the unit value, and it is passed to `Ok` as an ordinary argument. If `main` returns an `Err`, the program prints it and exits with a nonzero code, which is what a shell expects of a failed program.

## 9.3 To panic or not

The rule of thumb: panic when the program has reached a state it was not written to be in — a bug — and return a `Result` when the failure is something the caller can be expected to handle. A few sharper edges of that rule:

- In examples, prototypes and tests, `unwrap` and `expect` are fine; a `Result` that you handle carefully in a ten-line demo obscures the demo. In tests, a panic is *how* a test fails.
- When you have information the compiler does not — you know the literal parses, you know the index is in range because you just checked the length — `expect` with a message saying why is the honest spelling.
- When the failure is expected in normal operation — a missing file, bad input, a network that is down — it is a `Result`, and the type signature is the documentation that it can happen.
- When a function's *contract* is broken — it was called with an argument it documented as invalid — panic. The caller has a bug, and continuing would spread it.

That last point is what types are for, and a type can carry a check so that no caller has to repeat it:

```
pub struct Guess
    value: i32

impl Guess:
    pub fn new value: i32 -> Guess:
        if value < 1 || value > 100:
            panic! "Guess value must be between 1 and 100, got {value}."

        Guess\ value

    pub fn value (&self) -> i32:
        self <- value

fn main$:
    let g = Guess.new 50
    println! "{}" (g <- value$)

    let bad = Guess.new 200
    println! "{}" (bad <- value$)
```

```text
50
thread 'main' panicked at guess_validate.hrs:8:
Guess value must be between 1 and 100, got 200.
```

`Guess.new` checks the range once; `value` is private, so a `Guess` cannot be made any other way; every function that takes a `Guess` can therefore trust it without checking. Making an invalid state impossible to construct is the strongest form of error handling, because the error has nowhere to occur.

## 9.4 What you have

`panic!` for bugs; a panic unwinds, prints, and stops. `Result<T, E>` for failures the caller should handle: `match` it, `unwrap$` or `expect "why"` to panic on `Err`, or `?` to return it up — on `Result` or `Option`, in any function whose return type it fits, including a `main` that returns `Result<(), Box<dyn Error>>`. Choose by asking whether the failure is the program's fault or the world's, and let a type's constructor make the invalid states unreachable.

Next: generics, traits and lifetimes — the three mechanisms by which one piece of code serves many types, and the chapter where the lifetime error from chapter 4 finally gets its answer.

# 10. Generic types, traits and lifetimes

Three mechanisms let one piece of code serve many types, and this chapter is all three. *Generics* are placeholders for types: a `Vec<T>` is one definition that is a vector of anything. *Traits* say what a type can do, so a generic function can require it: "any `T` that can be compared". And *lifetimes* are generics over how long a reference is valid, which is the question chapter 4 left open. They arrive together because they are used together, and because the last of them is the one people fear, unnecessarily, so it is best met right after the first two make the shape familiar.

## 10.1 Generics

### Removing duplication

Two functions that differ only in a type:

```
fn largest_i32 list: &[i32] -> &i32:
    let mut largest = &list[0]

    for item in list:
        if item > largest:
            largest = item

    largest

fn largest_char list: &[char] -> &char:
    let mut largest = &list[0]

    for item in list:
        if item > largest:
            largest = item

    largest

fn main$:
    println! "{}" (largest_i32 (&[34, 50, 25, 100, 65]))
    println! "{}" (largest_char (&['y', 'm', 'a', 'q']))
```

```text
100
y
```

Same body, twice, for `i32` and for `char`, and there would be a third for `f64`. The duplication is in the *type*, so the fix is a parameter for the type:

```
fn largest<T> list: &[T] -> &T:
    let mut largest = &list[0]

    for item in list:
        if item > largest:
            largest = item

    largest

fn main$:
    println! "{}" (largest (&[34, 50, 25, 100, 65]))
```

```text
error[E0369]: binary operation `>` cannot be applied to type `&T`
  --> largest_generic_err.hrs:5:17
   |
 5 |         if item > largest:
   |            ---- &T
   |                 ^
   |                   ------- &T
   = help: consider restricting type parameter `T` (hrs 1:13)
```

`fn largest<T> list: &[T] -> &T` — the `<T>` after the name declares a type parameter, and then `T` is used where the type would be. It does not compile, and the error is the chapter's first lesson: `>` is not defined for *every* type, so the compiler will not let a function that takes *any* `T` compare two of them. It has to be told which `T`s are allowed, and the help says how: restrict the type parameter.

```
fn largest<T: PartialOrd> list: &[T] -> &T:
    let mut largest = &list[0]

    for item in list:
        if item > largest:
            largest = item

    largest

fn main$:
    println! "{}" (largest (&[34, 50, 25, 100, 65]))
    println! "{}" (largest (&['y', 'm', 'a', 'q']))
```

```text
100
y
```

`<T: PartialOrd>` — `T` may be any type that implements the trait `PartialOrd`, which is the trait that provides `>`. Now the body may compare, and the function works for `i32` and `char` and anything else that can be ordered, with one definition. Generics in Rust are checked at the definition, not at each use: a generic function has to make sense for *every* type its bounds allow, which is why the unbounded version was rejected and why, once it compiles, no call can break it. (At compile time each use is *monomorphized* — a copy is generated per concrete type — so there is no runtime cost for the abstraction.)

### Generic structs and methods

A struct's fields can be generic too:

```
#[derive Debug]
struct Point<T>
    x: T
    y: T

impl<T> Point<T>:
    fn x (&self) -> &T:
        &self <- x

impl Point<f32>:
    fn distance_from_origin (&self) -> f32:
        (self <- x <- powi 2 + self <- y <- powi 2) <- sqrt$

#[derive Debug]
struct Mixed<T, U>
    x: T
    y: U

fn main$:
    let integer = Point\ x = 5, y = 10
    let float = Point\ x = 1.0, y = 4.0
    println! "{:?} {:?} {}" integer float (integer <- x$)
    println! "{}" (float <- distance_from_origin$)

    let m = Mixed\ x = 5, y = 4.0
    println! "{m:?}"
```

```text
Point { x: 5, y: 10 } Point { x: 1.0, y: 4.0 } 5
4.1231055
Mixed { x: 5, y: 4.0 }
```

`struct Point<T>` with fields of type `T`; `Point\ x = 5, y = 10` is a `Point<i32>` and `Point\ x = 1.0, y = 4.0` a `Point<f64>`, inferred. Methods on a generic type are declared in `impl<T> Point<T>:` — the `<T>` after `impl` says *this block is generic over T* — and a method for one specific type is `impl Point<f32>:`, so `distance_from_origin` exists only on `Point<f32>`. When two fields may differ in type, two parameters, `Mixed<T, U>`.

Both fields of `Point<T>` are the same `T`, and the compiler holds you to it:

```
struct Point<T>
    x: T
    y: T

fn main$:
    let wont_work = Point\ x = 5, y = 4.0
    let _ = wont_work
```

```text
error[E0308]: mismatched types
  --> generic_mismatch.hrs:6:39
   |
 6 |     let wont_work = Point\ x = 5, y = 4.0
   |                                       ^^^ expected integer, found floating-point number
```

## 10.2 Traits

A trait is a set of methods a type may implement — an interface, a protocol, whichever word your last language used. It is defined with `trait`, and a type implements it with `impl … for`:

```
pub trait Summary:
    fn summarize (&self) -> String

pub struct NewsArticle
    pub headline: String
    pub location: String
    pub author: String

impl Summary for NewsArticle:
    fn summarize (&self) -> String:
        format!
            "{}, by {} ({})"
            (self <- headline)
            (self <- author)
            (self <- location)

pub struct Tweet
    pub username: String
    pub content: String

impl Summary for Tweet:
    fn summarize (&self) -> String:
        format! "{}: {}" (self <- username) (self <- content)

fn main$:
    let tweet =
        Tweet\
            username = String.from "horse_ebooks"
            content = String.from "of course, as you probably already know, people"
    println! "1 new tweet: {}" (tweet <- summarize$)

    let article =
        NewsArticle\
            headline = String.from "Penguins win the Stanley Cup Championship!"
            location = String.from "Pittsburgh, PA, USA"
            author = String.from "Iceburgh"
    println! "New article available! {}" (article <- summarize$)
```

```text
1 new tweet: horse_ebooks: of course, as you probably already know, people
New article available! Penguins win the Stanley Cup Championship!, by Iceburgh (Pittsburgh, PA, USA)
```

`pub trait Summary:` declares one method by its signature, `fn summarize (&self) -> String`, with no body — that line ends at the return type. `impl Summary for NewsArticle:` gives the body for one type, `impl Summary for Tweet:` for another, and after that `tweet <- summarize$` and `article <- summarize$` are ordinary method calls. The trait is what makes the two types interchangeable to any code that only needs `summarize`.

One rule to know: you may implement a trait for a type only if the trait or the type is defined in your crate. `Summary` for `Vec<T>` — fine, `Summary` is yours; `Display` for `Tweet` — fine, `Tweet` is yours; `Display` for `Vec<T>` — not allowed, since both belong to someone else and two crates could disagree. It is called the *orphan rule* and it is what keeps trait implementations unambiguous across the whole ecosystem.

### Default implementations

A trait method may have a body, used by any type that does not supply its own:

```
pub trait Summary:
    fn summarize_author (&self) -> String

    fn summarize (&self) -> String:
        format! "(Read more from {}...)" (self <- summarize_author$)

pub struct Tweet
    pub username: String
    pub content: String

impl Summary for Tweet:
    fn summarize_author (&self) -> String:
        format! "@{}" (self <- username)

fn main$:
    let tweet =
        Tweet\
            username = String.from "horse_ebooks"
            content = String.from "of course, as you probably already know, people"
    println! "1 new tweet: {}" (tweet <- summarize$)
```

```text
1 new tweet: (Read more from @horse_ebooks...)
```

`summarize` has a default that calls `summarize_author`, which has none; `Tweet` implements only the required one and gets the other free. A default may call the required methods, which is how a trait can offer a lot of behaviour and demand a little.

### Traits as parameters

A function that takes "anything summarizable" has four spellings, all in this example:

```
use std.fmt.Display

pub trait Summary:
    fn summarize (&self) -> String

pub struct Tweet
    pub username: String

impl Summary for Tweet:
    fn summarize (&self) -> String:
        format! "@{}" (self <- username)

impl Display for Tweet:
    fn fmt (&self) (f: &mut std.fmt.Formatter) -> std.fmt.Result:
        write! f "tweet by {}" (self <- username)

// `impl Trait` in a parameter: any type that implements Summary

pub fn notify item: &impl Summary:
    println! "Breaking news! {}" (item <- summarize$)

// The same, spelled out as a bound on a type parameter
pub fn notify2<T: Summary> item: &T:
    println! "Breaking news! {}" (item <- summarize$)

// Two bounds with `+`
pub fn notify3 item: &(impl Summary + Display):
    println! "{item} — {}" (item <- summarize$)

// A `where` clause, for when the bounds get long
pub fn notify4<T> item: &T
    [where T: Summary + Display]:
    println! "{item} — {}" (item <- summarize$)

fn main$:
    let t = Tweet\ username = String.from "horse_ebooks"
    notify (&t)
    notify2 (&t)
    notify3 (&t)
    notify4 (&t)
```

```text
Breaking news! @horse_ebooks
Breaking news! @horse_ebooks
tweet by horse_ebooks — @horse_ebooks
tweet by horse_ebooks — @horse_ebooks
```

`item: &impl Summary` is the short form: a reference to some type that implements `Summary`. `notify2<T: Summary> item: &T` is the same thing with the type parameter named, which you need when two parameters must be the *same* type, or the name is used elsewhere. `&(impl Summary + Display)` requires two traits — `+` joins bounds, and the group is parenthesised so the `+` cannot be read as part of the parameter. And `[where T: Summary + Display]` moves the bounds out of the signature to a line of their own, which is what you do when they get long. The brackets are Harsh's: a `where` bound has a `:` in it, and the brackets keep that colon from opening a block; they are stripped on the way out.

`Tweet` also implements `Display` here, by writing `fmt` — that is the trait behind `{}`, the one chapter 5's error said `Rectangle` lacked. `write! f "…" args` writes into the formatter, and the method's two groups are `(&self)` and `(f: &mut std.fmt.Formatter)`.

> **Harsh —** A `where` clause on one line needs nothing: `fn f<T> (x: T) -> T where T: Clone:`. Over several lines it is written in brackets, `[where T: Summary + Display]` on its own line under the signature, because a bound's `:` would otherwise be read as opening a block; the brackets keep the clause part of the header and are not emitted, and the body indents as it would without them.


### Returning a trait

`impl Trait` works in return position too:

```
pub trait Summary:
    fn summarize (&self) -> String

pub struct Tweet
    pub username: String

impl Summary for Tweet:
    fn summarize (&self) -> String:
        format! "@{}" (self <- username)

fn returns_summarizable$ -> impl Summary:
    Tweet\ username = String.from "horse_ebooks"

fn main$:
    println! "{}" (returns_summarizable$ <- summarize$)
```

```text
@horse_ebooks
```

"This function returns *some* type that implements `Summary`" — the caller can call `summarize$` and nothing else. Useful when the concrete type is long or unnameable (closures and iterators, chapter 13). One limit: the function must return one concrete type; a function that returns a `Tweet` on one branch and a `NewsArticle` on another cannot use `impl Summary`, and needs a trait object (chapter 18).

### Conditional methods

A method can exist only when the type parameter meets a bound:

```
use std.fmt.Display

struct Pair<T>
    x: T
    y: T

impl<T> Pair<T>:
    fn new (x: T) (y: T) -> Self:
        Self\ x, y

// Only a Pair whose T can be compared and displayed gets this method.

impl<T: Display + PartialOrd> Pair<T>:
    fn cmp_display (&self):
        if self <- x >= self <- y:
            println! "The largest member is x = {}" (self <- x)

        else:
            println! "The largest member is y = {}" (self <- y)

fn main$:
    let p = Pair.new 3 7
    p <- cmp_display$

    // `to_string` exists on every type that implements Display — a blanket impl in std.
    let s: String = 42 <- to_string$
    println! "{s}"
```

```text
The largest member is y = 7
42
```

`impl<T> Pair<T>:` gives every `Pair` a `new`; `impl<T: Display + PartialOrd> Pair<T>:` gives `cmp_display` only to pairs whose elements can be compared and printed. The standard library uses the same device on a larger scale: `impl<T: Display> ToString for T` implements `ToString` for *every* type that implements `Display`, which is why `42 <- to_string$` works — a *blanket implementation*, and a large part of why traits compose.

## 10.3 Lifetimes

Every reference has a *lifetime*: the region of the program during which it is valid. Usually the compiler works it out and you write nothing — every `&` so far had a lifetime you never saw. It has to be written only when the compiler cannot tell how the lifetimes of several references relate, and the classic case is a function that returns one of two borrowed arguments:

```
fn longest (x: &str) (y: &str) -> &str:
    if x <- len$ > y <- len$: x else: y

fn main$:
    println! "{}" (longest "abcd" "xyz")
```

```text
error[E0106]: missing lifetime specifier
  --> longest_err.hrs:1:35
   |
 1 | fn longest (x: &str) (y: &str) -> &str:
   |                ----
   |                          ----
   |                                   ^ expected named lifetime parameter
   = help: this function's return type contains a borrowed value, but the signature does not say whether it is borrowed from `x` or `y`
   = help: consider introducing a named lifetime parameter (hrs 1:12)
```

Read the help: the return type is a borrowed value, and the signature does not say whether it is borrowed from `x` or from `y`. The compiler needs to know, because the caller's borrow checker needs to know how long the returned reference may be used — as long as `x` lives, as long as `y` lives, or only as long as both do. The signature has to say. The syntax for saying it is a *lifetime parameter*:

```
fn longest<'a> (x: &'a str) (y: &'a str) -> &'a str:
    if x <- len$ > y <- len$: x else: y

fn main$:
    let string1 = String.from "abcd"
    let string2 = "xyz"
    let result = longest (string1 <- as_str$) string2
    println! "The longest string is {result}"
```

```text
The longest string is abcd
```

`fn longest<'a> (x: &'a str) (y: &'a str) -> &'a str` — `'a` is declared after the name like a type parameter (the apostrophe marks it as a lifetime), and then `&'a str` means "a reference with lifetime `'a`". Putting the same `'a` on both inputs and the output says: the returned reference lives as long as the *shorter* of the two inputs. That is the whole meaning. Lifetime annotations do not change how long anything lives; they describe the relationship, so that the checker can verify calls against it.

And it does verify:

```
fn longest<'a> (x: &'a str) (y: &'a str) -> &'a str:
    if x <- len$ > y <- len$: x else: y

fn main$:
    let string1 = String.from "long string is long"
    let result

    do:
        let string2 = String.from "xyz"
        result = longest (string1 <- as_str$) (string2 <- as_str$)

    println! "The longest string is {result}"
```

```text
error[E0597]: `string2` does not live long enough
  --> longest_scope.hrs:10:48
   |
 9 |         let string2 = String.from "xyz"
   |             ------- binding `string2` declared here
10 |         result = longest (string1 <- as_str$) (string2 <- as_str$)
   |                                                ^^^^^^^ borrowed value does not live long enough
12 |     println! "The longest string is {result}"
   |     - `string2` dropped here while still borrowed
   |                                     -------- borrow later used here
```

`string2` lives only inside the `do:` block; `result` is used after it; and `longest`'s signature says `result` cannot outlive the shorter of its inputs. So the compiler rejects the program, and — this is the part to appreciate — it does so *at the call*, from the signature alone, without looking inside `longest`. That is what the annotation bought: a function's borrowing behaviour is part of its interface, checked at each use.

### Lifetimes in structs

Chapter 5's `&str` field had this error. A struct that holds a reference declares a lifetime, and the struct cannot outlive what it borrows:

```
#[derive Debug]
struct ImportantExcerpt<'a>
    part: &'a str

impl<'a> ImportantExcerpt<'a>:
    fn level (&self) -> i32:
        3

    fn announce_and_return_part (&self) (announcement: &str) -> &str:
        println! "Attention please: {announcement}"
        self <- part

fn main$:
    let novel = String.from "Call me Ishmael. Some years ago..."
    let first_sentence =
        novel <- split '.'
              <- next$
              <- expect "Could not find a '.'"
    let i = ImportantExcerpt\ part = first_sentence

    println! "{i:?} {}" (i <- level$)
    println! "{}" (i <- announce_and_return_part "hi")
```

```text
ImportantExcerpt { part: "Call me Ishmael" } 3
Attention please: hi
Call me Ishmael
```

`struct ImportantExcerpt<'a>` with `part: &'a str` means an `ImportantExcerpt` is valid only while the text it points into is. `impl<'a> ImportantExcerpt<'a>:` declares the lifetime for the methods, which then mostly do not mention it — `level` takes `&self` and returns an `i32`; `announce_and_return_part` returns a `&str` whose lifetime the compiler works out from `&self`, by the rules below.

### Elision

If every reference needed a written lifetime the language would be unusable, so three rules fill them in when they can, and you write one only when they cannot:

```
// One input lifetime: the output gets it. Written out, this is
// fn first_word<'a> s: &'a str -> &'a str.
fn first_word s: &str -> &str:
    let bytes = s <- as_bytes$

    for (i, &item) in bytes <- iter$ <- enumerate$:
        if item == b' ':
            return &s[0..i]

    &s[..]

// A reference that lives for the whole program.
static GREETING: &'static str = "I have a static lifetime."

fn main$:
    println! "{}" (first_word "hello world")
    println! "{GREETING}"
```

```text
hello
I have a static lifetime.
```

1. Each reference parameter gets its own lifetime.
2. If there is exactly one input lifetime, the output gets it — which is why chapter 4's `first_word s: &str -> &str` needed nothing.
3. If one of the inputs is `&self`, the output gets `self`'s lifetime — which is why `announce_and_return_part` needed nothing.

`longest` had two inputs and no `self`, so no rule applied and the signature had to say. That is the entire theory of when you write lifetimes. `'static` is the one named lifetime you will see in the wild: the whole program, which every string literal has, since the text lives in the binary.

### All three at once

```
use std.fmt.Display

fn longest_with_an_announcement<'a, T> (x: &'a str) (y: &'a str) (ann: T) -> &'a str
    [where T: Display]:
    println! "Announcement! {ann}"
    if x <- len$ > y <- len$: x else: y

fn main$:
    println! "{}" (longest_with_an_announcement "abcd" "xyz" "today")
```

```text
Announcement! today
abcd
```

A lifetime, a type parameter, a bound in a `[where …]` clause, and three parameters: everything in this chapter in one signature, and nothing in it is new.

## 10.4 What you have

`<T>` after a name makes a function, struct or `impl` generic; `T: Trait` restricts it, and the body may only do what the bounds allow. `trait` declares methods, with or without defaults; `impl Trait for Type` supplies them; `&impl Trait`, `T: Trait`, `+`, and `[where …]` take them as parameters and `impl Trait` returns one. The orphan rule keeps implementations unambiguous. A lifetime `'a` names how long a reference is valid, is written only when elision's three rules cannot fill it in, and lets the checker verify each call from the signature alone.

Next: writing automated tests — which is the natural use of everything so far, and where `panic` is a feature.

# 11. Writing automated tests

The compiler checks a great deal — types, ownership, exhaustive matches — and none of it says whether `add 2 2` is `4`. That is what tests are for: functions that call your code and check what comes back, run by `cargo test` (or `hrs test`, which transpiles and then does the same), and reported. This chapter is how to write them, what the assertion macros do, and how to run some of them. Every example here is a test file, and the output shown is the test runner's report.

## 11.1 Anatomy of a test

A test is a function with `#[test]` above it. It passes if it returns, and fails if it panics:

```
pub fn add (left: u64) (right: u64) -> u64:
    left + right

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn it_works$:
        let result = add 2 2
        assert_eq! result 4
```

```text
$ cargo test
running 1 test
test tests::it_works ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Everything after `#[cfg test]` is the conventional shape. `mod tests:` is an ordinary module; the attribute above it says *compile this only when testing*, so the tests cost nothing in the real build. `use super.*` brings the enclosing module's items — `add` — into the test module, which is a child and would otherwise see nothing. Then `#[test]` on a function makes it a test, and `assert_eq! result 4` checks that its two arguments are equal, panicking with both values if they are not. The report names each test by its module path and tallies at the end.

Here is what failure looks like:

```
#[cfg test]
mod tests:
    #[test]
    fn exploration$:
        assert_eq! (2 + 2) 4

    #[test]
    fn another$:
        panic! "Make this test fail"
```

```text
$ cargo test
running 2 tests
test tests::another ... FAILED
test tests::exploration ... ok
failures:
---- tests::another stdout ----
thread 'tests::another' panicked at failing.hrs:10:
Make this test fail
failures:
    tests::another
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

`another` panicked, so it failed; the runner prints each failing test's output and message under `failures:`, then the tally. One failure fails the run — `cargo test` exits nonzero — which is what a build pipeline wants.

## 11.2 The assertion macros

`assert!` takes something that must be true:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

impl Rectangle:
    fn can_hold (&self) (other: &Rectangle) -> bool:
        self <- width > other <- width && self <- height > other <- height

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn larger_can_hold_smaller$:
        let larger = Rectangle\ width = 8, height = 7
        let smaller = Rectangle\ width = 5, height = 1
        assert! (larger <- can_hold (&smaller))

    #[test]
    fn smaller_cannot_hold_larger$:
        let larger = Rectangle\ width = 8, height = 7
        let smaller = Rectangle\ width = 5, height = 1
        assert! (!smaller <- can_hold (&larger))
```

```text
$ cargo test
running 2 tests
test tests::larger_can_hold_smaller ... ok
test tests::smaller_cannot_hold_larger ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Both tests call a method and assert on the result: the first that it holds, the second that it does not — `!smaller <- can_hold (&larger)` negates the call, and the whole expression is isolated as the macro's one argument. `assert!` on a `false` panics with the text of the expression.

`assert_eq!` and `assert_ne!` compare two values and, on failure, print both — which is why they beat `assert! (a == b)`, whose failure would only say "false":

```
pub fn add_two a: u64 -> u64:
    a + 3        // a bug

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn it_adds_two$:
        let result = add_two 2
        assert_eq! result 4
```

```text
$ cargo test
running 1 test
test tests::it_adds_two ... FAILED
failures:
---- tests::it_adds_two stdout ----
thread 'tests::it_adds_two' panicked at assert_eq_fail.hrs:12:
assertion `left == right` failed
  left: 5
 right: 4
failures:
    tests::it_adds_two
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

`left: 5, right: 4` — the function under test has a bug, and the report shows what it produced and what was expected without a `println!` in sight. The two arguments must implement `PartialEq` (for `==`) and `Debug` (for the printing); for your own types that is `#[derive PartialEq Debug]`.

### A message of your own

Every assertion macro takes extra arguments after the required ones, and they are a format string and its values, printed on failure:

```
pub fn greeting name: &str -> String:
    String.from "Hello!"      // forgot the name

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn greeting_contains_name$:
        let result = greeting "Carol"
        assert!
            (result <- contains "Carol")
            "Greeting did not contain name, value was `{result}`"
```

```text
$ cargo test
running 1 test
test tests::greeting_contains_name ... FAILED
failures:
---- tests::greeting_contains_name stdout ----
thread 'tests::greeting_contains_name' panicked at custom_message.hrs:12:
Greeting did not contain name, value was `Hello!`
failures:
    tests::greeting_contains_name
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

When a test fails a week after it was written, "assertion failed" is not enough; "did not contain name, value was `Hello!`" says what to look at. The message is worth writing whenever the failing values alone would not explain the failure.

### Testing that something panics

Sometimes the correct behaviour *is* a panic — chapter 9's `Guess.new` on a bad value — and a test asserts that it happens:

```
pub struct Guess
    value: i32

impl Guess:
    pub fn new value: i32 -> Guess:
        if value < 1:
            panic! "Guess value must be greater than or equal to 1, got {value}."

        else if value > 100:
            panic! "Guess value must be less than or equal to 100, got {value}."

        Guess\ value

#[cfg test]
mod tests:
    use super.*

    #[test]
    #[should_panic (expected = "less than or equal to 100")]
    fn greater_than_100$:
        Guess.new 200;
```

```text
$ cargo test
running 1 test
test tests::greater_than_100 - should panic ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`#[should_panic]` inverts the meaning of a panic: the test passes if the body panics and fails if it returns. On its own it is imprecise — any panic passes, including one from a bug in the wrong place — so `expected = "…"` narrows it to a panic whose message contains that text. The report marks it `- should panic`.

### `Result` in tests

A test may return a `Result` instead of panicking:

```
#[cfg test]
mod tests:
    #[test]
    fn it_works$ -> Result<(), String>:
        let result = 2 + 2

        if result == 4:
            Ok ()

        else:
            Err (String.from "two plus two does not equal four")
```

```text
$ cargo test
running 1 test
test tests::it_works ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`Ok ()` passes and `Err` fails with the error printed. The point is `?`: a test that calls fallible code can use `?` on every step and fail cleanly on the first `Err`, instead of unwrapping each one. (Such a test cannot be `#[should_panic]`; to assert that something returns an `Err`, use `assert! (value <- is_err$)`.)

## 11.3 Running tests

`cargo test` builds the test binary and runs every test, by default in parallel on several threads — so tests must not depend on each other or on shared state such as a file they all write; `cargo test -- --test-threads=1` runs them one at a time when they must. Output printed by a passing test is captured and hidden; `-- --show-output` shows it.

A name after `cargo test` runs only the tests whose path contains it:

```
pub fn add_two a: u64 -> u64:
    a + 2

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn add_two_and_two$:
        assert_eq! 4 (add_two 2)

    #[test]
    fn add_three_and_two$:
        assert_eq! 5 (add_two 3)

    #[test]
    fn one_hundred$:
        assert_eq! 102 (add_two 100)

    #[test]
    #[ignore]
    fn expensive_test$:
        // takes an hour; run with `cargo test -- --ignored`
        assert_eq! 2 (add_two 0)
```

```text
$ cargo test add
running 2 tests
test tests::add_three_and_two ... ok
test tests::add_two_and_two ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out
```

Three tests match `add`, `one_hundred` does not and is *filtered out*, and `expensive_test` was never a candidate: `#[ignore]` excludes a test from the default run, and `cargo test -- --ignored` runs only those. That is the tool for the slow tests you want to keep but not wait for on every save.

## 11.4 Where tests live

Rust distinguishes *unit* tests, which live in the file with the code and test it in detail, from *integration* tests, which live outside the crate and use its public API the way a user would.

Unit tests are what this chapter has shown: a `#[cfg test] mod tests:` at the bottom of each file. Because it is a child module of the code it tests, it can see private items:

```
pub fn add_two a: u64 -> u64:
    internal_adder a 2

fn internal_adder (left: u64) (right: u64) -> u64:
    left + right

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn internal$:
        // a private function is still visible to its own module's tests
        assert_eq! 4 (internal_adder 2 2)
```

```text
$ cargo test
running 1 test
test tests::internal ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`internal_adder` is not `pub` and the test calls it anyway — child modules see their ancestors, and privacy is about the boundary of a crate, not about hiding code from its own tests. Whether private functions *should* be tested is a matter of taste; Rust does not make the choice for you.

Integration tests are files in a `tests/` directory beside `src/`, each its own crate that `use`s your library like an external one; `cargo test` builds and runs every file there, and only the public API is reachable, which is the point. A binary crate — one with `src/main.hrs` and no library — cannot be tested this way, since there is nothing to `use`; the usual arrangement is a thin `main.hrs` calling into a `lib.hrs` that holds the logic, and the integration tests target the library. Harsh changes none of this: `hrs test` transpiles `src/` and `tests/` alike and hands `cargo test` the result.

## 11.5 What you have

`#[test]` makes a function a test; it passes by returning and fails by panicking, which `assert!`, `assert_eq!` and `assert_ne!` do with useful messages and your own on top. `#[should_panic (expected = "…")]` for the cases that must panic; a `Result`-returning test for `?`. `#[cfg test] mod tests:` with `use super.*` at the bottom of a file, seeing everything in it; `tests/` for the public API. `cargo test name` filters, `#[ignore]` defers, `-- --test-threads=1` serialises.

Next: an I/O project — a small `grep` — that uses everything so far in one program, and is the first thing in this book you might actually run.

# 12. An I/O project: a small grep

Everything so far, in one program. `grep` searches a file for lines containing a string; this chapter builds a small one, `minigrep`, the way a real program gets built: a first version that works, then a refactor into something you would not be embarrassed by, then tests, then a feature. Along the way: command-line arguments, reading a file, error output on the right stream, an exit code, an environment variable, and the split between a library that does the work and a binary that talks to the shell. Each stage is a complete project; the output under it is the program run with the arguments shown.

## 12.1 Reading arguments

`poem.txt`

```text
I'm nobody! Who are you?
Are you nobody, too?
Then there's a pair of us - don't tell!
They'd banish us, you know.

How dreary to be somebody!
How public, like a frog
To tell your name the livelong day
To an admiring bog!
```

`src/main.hrs`

```
use std.env

fn main$:
    let args: Vec<String> = env.args$ <- collect$
    let query = &args[1]
    let file_path = &args[2]

    println! "Searching for {query}"
    println! "In file {file_path}"
```

```text
$ cargo run -- searchstring example-filename.txt
Searching for searchstring
In file example-filename.txt
```

`env.args$` is an iterator over the arguments, and `<- collect$` gathers them into a `Vec<String>` — the type annotation tells `collect` which collection to build, as in chapter 1. Element `0` is the program's own name, so the query is `args[1]` and the file is `args[2]`; `&args[1]` borrows the string out of the vector rather than moving it. (The `$ cargo run -- …` line above the output shows the invocation: the `--` separates cargo's arguments from the program's.)

## 12.2 Reading the file

`src/main.hrs`

```
use std.env
use std.fs

fn main$:
    let args: Vec<String> = env.args$ <- collect$
    let query = &args[1]
    let file_path = &args[2]

    println! "Searching for {query}"
    println! "In file {file_path}"

    let contents =
        fs.read_to_string file_path
            <- expect "Should have been able to read the file"

    println! "With text:\n{contents}"
```

```text
$ cargo run -- the poem.txt
Searching for the
In file poem.txt
With text:
I'm nobody! Who are you?
Are you nobody, too?
Then there's a pair of us - don't tell!
They'd banish us, you know.

How dreary to be somebody!
How public, like a frog
To tell your name the livelong day
To an admiring bog!
```

`fs.read_to_string file_path` reads the whole file into a `String`, or fails, and for now the failure is an `expect`. The program works. It is also a `main` doing three jobs — parsing, reading, printing — with two unlabelled positional arguments and an error strategy of "crash". Programs written like this are how big `main` functions happen, so the next section takes it apart before it grows.

## 12.3 Refactoring

Two things at once: give the configuration a type, and give errors a path that is not a panic. First the type:

`src/main.hrs`

```
use std.env
use std.fs

struct Config
    query: String
    file_path: String

impl Config:
    fn new args: &[String] -> Config:
        if args <- len$ < 3:
            panic! "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        Config\ query, file_path

fn main$:
    let args: Vec<String> = env.args$ <- collect$
    let config = Config.new (&args)
    println! "Searching for {}" (config <- query)
    println! "In file {}" (config <- file_path)

    let contents =
        fs.read_to_string (config <- file_path)
            <- expect "Should have been able to read the file"

    println! "With text:\n{contents}"
```

```text
$ cargo run
thread 'main' panicked at src/main.hrs:12:
not enough arguments
```

`Config` names the two values, and `Config.new` builds one from the argument list — cloning the strings, which costs a little and keeps the ownership story simple: the `Config` owns its fields, and `args` is untouched. The check for too few arguments is in the one place the arguments are looked at. Run with none and the check fires — as a panic, which is the right *check* with the wrong *response*: a panic is for bugs, and a user forgetting an argument is not a bug.

`src/main.hrs`

```
use std.env
use std.fs
use std.process

struct Config
    query: String
    file_path: String

impl Config:
    fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        Ok (Config\ query, file_path)

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        println! "Problem parsing arguments: {err}"
        process.exit 1

    println! "Searching for {}" (config <- query)
    println! "In file {}" (config <- file_path)

    let contents =
        fs.read_to_string (config <- file_path)
            <- expect "Should have been able to read the file"

    println! "With text:\n{contents}"
```

```text
$ cargo run
Problem parsing arguments: not enough arguments
```

Now `Config.build` returns `Result<Config, &'static str>` — an `Err` with a message, in the case the caller should handle — and `main` handles it with `unwrap_or_else`: given the `Ok`, the value; given the `Err`, run this closure with it. The closure prints the message and calls `process.exit 1`, which ends the program with a nonzero code and *no* panic output. Compare the two reports: the second is what a user should see.

The closure is a *trailing closure with a block body*: `<- unwrap_or_else |err|:` opens a block whose two lines are the closure's body, and the argument list closes after them. It is the form you will write for every callback of more than one line, and it is chapter 13's subject.

### Into a library

The last refactoring moves the logic out of `main.hrs` into `lib.hrs`, so that it can be tested — a binary's `main` cannot be called from a test, but a library's functions can. Here is the library, tests first, in the shape the finished program will have:

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config:
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
$ cargo test
running 2 tests
test tests::case_insensitive ... ok
test tests::case_sensitive ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Two tests, two functions. `search` walks `contents <- lines$` and keeps each line that `<- contains query`; `search_case_insensitive` lowercases both first. The signature `fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>` is chapter 10's lesson in use: the result holds slices *of `contents`*, so the output lifetime is tied to that parameter and not to `query`, and the compiler holds the caller to it. `run` reads the file, chooses a search, and prints; it returns `Result<(), Box<dyn Error>>` so that `?` on the read works and any error reaches `main` as a value. The test strings use `"\` at the end of a line — Rust's line continuation inside a string, which drops the newline and the leading spaces of the next line — so the expected text starts on the line below.

The tests were written before the search functions worked; that order — a failing test, then the code that passes it — is test-driven development, and `cargo test` makes it cheap.

## 12.4 The finished program

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config:
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
$ cargo run -- frog poem.txt
How public, like a frog
```

`main.hrs` is now short. `use minigrep.Config` imports from the library crate, which has the package's name; `Config.build` and `minigrep.run` do the work, and `main` handles the two ways they can fail. The output is one line, the one containing `frog`. A different query:

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config:
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
$ cargo run -- body poem.txt
I'm nobody! Who are you?
Are you nobody, too?
How dreary to be somebody!
```

`body` matches `nobody` and `somebody` and not `Body` — the search is case-sensitive, and the way to change that is an environment variable, read by `Config.build`:

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config:
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
IGNORE_CASE=1 $ cargo run -- to poem.txt
Are you nobody, too?
How dreary to be somebody!
To tell your name the livelong day
To an admiring bog!
```

`env.var "IGNORE_CASE"` is `Ok` if the variable is set, to anything, and `<- is_ok$` turns that into the `bool`. Set it and every line with `to` or `To` comes back. An environment variable is the conventional place for an option a user sets once for a session; a command-line flag would be the place for one that changes per run, and either is a small change to `build`.

### Errors go to standard error

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let args: Vec<String> = env.args$ <- collect$

    let config = Config.build (&args) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config:
    pub fn build args: &[String] -> Result<Config, &'static str>:
        if args <- len$ < 3:
            return Err "not enough arguments"

        let query = args[1] <- clone$
        let file_path = args[2] <- clone$
        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- contains query:
            results <- push line

    results

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$
    let mut results = Vec.new$

    for line in contents <- lines$:
        if line <- to_lowercase$ <- contains (&query):
            results <- push line

    results

#[cfg test]
mod tests:
    use super.*

    #[test]
    fn case_sensitive$:
        let query = "duct"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Duct tape."
        assert_eq!
            (vec! ["safe, fast, productive."])
            (search query contents)

    #[test]
    fn case_insensitive$:
        let query = "rUsT"
        let contents = "\

Rust:
safe, fast, productive.
Pick three.
Trust me."
        assert_eq!
            (vec! ["Rust:", "Trust me."])
            (search_case_insensitive query contents)
```

```text
$ cargo run
Problem parsing arguments: not enough arguments
```

Run with no arguments the program prints its message and exits with code 1, and the message went to *standard error* — `eprintln!` instead of `println!` — so that `cargo run > output.txt` would leave the error on the terminal and the file empty, rather than writing the complaint into the results. Output is for results and standard error is for everything else; a program that gets this right composes with every other tool on the system, and one that gets it wrong quietly does not.

## 12.5 What you have

A whole program: `env.args$ <- collect$` for arguments, `fs.read_to_string` for a file, a `Config` type built by a `Result`-returning function, `unwrap_or_else` with a block closure and `process.exit` for a clean failure, `Box<dyn Error>` and `?` in `run`, a `lib.hrs` with the logic and tests and a `main.hrs` that only talks to the shell, `env.var` for an option, and `eprintln!` for errors. That is the shape of most command-line programs, and you have written one.

Next: closures and iterators — the two features the functional languages lend Rust, which this program used twice without explaining, and which turn most `for` loops into something shorter.

# 13. Closures and iterators

Two features Rust took from the functional languages, and the two that most change how daily code reads. A *closure* is a function you write inline, that can use the variables around it. An *iterator* is a value that yields a sequence one item at a time, with a library of *adaptors* — `map`, `filter` and the rest — that take a closure each. Together they replace most `for` loops with a chain that says what is wanted rather than how to loop for it. Harsh has layout for both, and this is the chapter where its chain and closure forms earn their keep.

## 13.1 Closures

A closure captures its environment. Here one reads a struct field it was not passed:

```
#[derive Debug PartialEq Copy Clone]
enum ShirtColor
    Red
    Blue

struct Inventory
    shirts: Vec<ShirtColor>

impl Inventory:
    fn giveaway (&self) (user_preference: Option<ShirtColor>) -> ShirtColor:
        user_preference <- unwrap_or_else (|| self <- most_stocked$)

    fn most_stocked (&self) -> ShirtColor:
        let mut num_red = 0
        let mut num_blue = 0

        for color in &self <- shirts:
            match color:
                ShirtColor.Red => num_red += 1
                ShirtColor.Blue => num_blue += 1

        if num_red > num_blue: ShirtColor.Red else: ShirtColor.Blue

fn main$:
    let store = Inventory\ shirts = vec! [ShirtColor.Blue, ShirtColor.Red, ShirtColor.Blue]
    let user_pref1 = Some ShirtColor.Red
    println!
        "The user with preference {:?} gets {:?}"
        user_pref1
        (store <- giveaway user_pref1)

    let user_pref2 = None
    println!
        "The user with preference {:?} gets {:?}"
        user_pref2
        (store <- giveaway user_pref2)
```

```text
The user with preference Some(Red) gets Red
The user with preference None gets Blue
```

`unwrap_or_else` takes a closure that produces the default: `(|| self <- most_stocked$)`, no parameters, a body that uses `self` from the enclosing method. Nothing was passed in; the closure *captured* `self`. A function could not do this — `fn` cannot see the variables of the function that defines it — and that is the difference: a closure is a function plus the environment it was written in. The closure is isolated in parentheses here because it is an argument with an inline body; it is the one case in the argument rule where the parentheses mark "this whole thing, closure and body, is one argument".

### Forms

A closure is `|parameters| body`, and the body is an expression or a block:

```
fn add_one_v1 x: u32 -> u32:
    x + 1

fn main$:
    let add_one_v2 = |x: u32| -> u32:      // fully annotated, block body
        x + 1

    let add_one_v3 = |x| x + 1               // inferred, expression body

    let add_one_v4 = |x|:                    // inferred, block body
        x + 1

    println!
        "{} {} {} {}"
        (add_one_v1 1)
        (add_one_v2 2)
        (add_one_v3 3)
        (add_one_v4 4)
```

```text
2 3 4 5
```

Four spellings of `x + 1`. `add_one_v2` annotates everything and has a block body under a `:`; `v3` is the short form, inferred types and an expression; `v4` is inferred with a block. The `:` after the parameter list opens the body's block exactly as it does after a function signature, and the body indents from the closure. Annotations are optional because a closure is usually short and used once, close to where its types are obvious; the compiler infers them from the first use, and holds the closure to it:

```
fn main$:
    let example_closure = |x| x
    let s = example_closure (String.from "hello")
    let n = example_closure 5
    println! "{s} {n}"
```

```text
error[E0308]: mismatched types
  --> closure_infer.hrs:4:29
   |
 4 |     let n = example_closure 5
   |             --------------- arguments to this function are incorrect
   |                             ^ expected `String`, found integer
   = note: expected because the closure was earlier called with an argument of type `String` (hrs 3:30)
   = note: closure parameter defined here (hrs 2:28)
   = help: try using a conversion method (hrs 4:30)
```

`example_closure` was called with a `String`, so that is its parameter type, and the second call with `5` is a type error. A closure has *one* signature, inferred once; it is not generic.

### Capturing

A closure captures each variable it uses in the least demanding way that works — by shared reference, by mutable reference, or by value — and the borrow checker treats the closure as holding that borrow from its creation to its last use:

```
fn main$:
    let list = vec! [1, 2, 3]
    println! "Before defining closure: {list:?}"

    let only_borrows = || println! "From closure: {list:?}"
    println! "Before calling closure: {list:?}"
    only_borrows$
    println! "After calling closure: {list:?}"
```

```text
Before defining closure: [1, 2, 3]
Before calling closure: [1, 2, 3]
From closure: [1, 2, 3]
After calling closure: [1, 2, 3]
```

`only_borrows` reads `list`, so it holds `&list`, and `list` can still be printed before and after the call, since shared borrows coexist. Now a closure that writes:

```
fn main$:
    let mut list = vec! [1, 2, 3]
    println! "Before defining closure: {list:?}"

    let mut borrows_mutably = || list <- push 7
    borrows_mutably$
    println! "After calling closure: {list:?}"
```

```text
Before defining closure: [1, 2, 3]
After calling closure: [1, 2, 3, 7]
```

`borrows_mutably` pushes, so it holds `&mut list` from `let` to the last call — and notice there is no `println!` between the two, because one would need `&list` while the mutable borrow is live, and chapter 4's rule would refuse it. The closure is `let mut` because calling it mutates its capture.

To make a closure take ownership of what it uses — needed when it will outlive the current function, as a closure handed to a new thread will — write `move`:

```
use std.thread

fn main$:
    let list = vec! [1, 2, 3]
    println! "Before defining closure: {list:?}"

    thread.spawn (move || println! "From thread: {list:?}")
        <- join$
        <- unwrap$
```

```text
Before defining closure: [1, 2, 3]
From thread: [1, 2, 3]
```

`move ||` moves `list` into the closure; the thread may then run after `main`'s frame is gone and still own its data. Without `move`, the closure would borrow `list`, and the compiler would reject the program because the thread might outlive the borrow. (Threads are chapter 16; this is a preview of why `move` exists.)

### `Fn`, `FnMut`, `FnOnce`

How a closure captures determines which of three traits it implements, and every function that takes a closure says which it needs:

- `FnOnce`: can be called at least once. Every closure implements it; a closure that *moves* a captured value out of its body implements *only* it, since after one call the value is gone.
- `FnMut`: can be called repeatedly and may mutate its captures.
- `Fn`: can be called repeatedly and touches nothing mutably — or captures nothing at all.

The type of a closure is written as the trait applied to its parameter types, the return after `->`:

```
// A closure's type is written as the trait applied to its parameter types,
// with `->` for the return: `Fn i32 -> i32` takes one i32 and returns one.

fn apply_twice (f: impl Fn i32 -> i32) (x: i32) -> i32:
    f (f x)

fn make_adder n: i32 -> impl Fn i32 -> i32:
    move |x| x + n

// Two parameters: each is one atom, a group when it is more than a name
fn combine (f: impl Fn (i32) (i32) -> i32) (a: i32) (b: i32) -> i32:
    f a b

// No parameters: `$`, exactly as a call with none
fn run_once (f: impl FnOnce$ -> String) -> String:
    f$

// Boxed, for a closure stored in a struct or a Vec
fn boxed$ -> Box<dyn Fn i32 -> i32>:
    Box.new (|x| x * 10)

fn main$:
    let add5 = make_adder 5
    println! "{}" (apply_twice add5 1)
    println! "{}" (combine (|a, b| a * b) 6 7)
    let s = String.from "moved out"
    println! "{}" (run_once (move || s))
    println! "{}" ((boxed$) 4)
```

```text
11
42
moved out
40
```

> **Harsh —** `Fn i32 -> i32` is a closure taking one `i32` and returning one; `Fn (i32) (i32) -> i32` takes two, each parameter one atom, a group when it is more than a bare name; `FnOnce$` takes none, exactly as a call with none is written. The same spelling serves `impl Fn` in a parameter or a return type and `dyn Fn` inside a `Box`. There is nothing else to learn here: a closure type is an application like every other.

`unwrap_or_else` takes `FnOnce`, the most permissive, because it calls the closure at most once. `sort_by_key` calls its closure once per comparison, so it asks for `FnMut`:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

fn main$:
    let mut list =
        [
            (Rectangle\ width = 10, height = 1),
            (Rectangle\ width = 3, height = 5),
            (Rectangle\ width = 7, height = 12),
        ]

    list <- sort_by_key (|r| r <- width)
    println! "{list:#?}"
```

```text
[
    Rectangle {
        width: 3,
        height: 5,
    },
    Rectangle {
        width: 7,
        height: 12,
    },
    Rectangle {
        width: 10,
        height: 1,
    },
]
```

`(|r| r <- width)` reads a field and returns it: it is `Fn`, which is also `FnMut`, so it qualifies. Here is one that does not:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

fn main$:
    let mut list =
        [
            (Rectangle\ width = 10, height = 1),
            (Rectangle\ width = 3, height = 5),
        ]

    let mut sort_operations = vec! []
    let value = String.from "closure called"

    list <- sort_by_key |r|:
        sort_operations <- push value       // moves `value` out: can only happen once
        r <- width

    println! "{list:#?}"
```

```text
error[E0507]: cannot move out of `value`, a captured variable in an `FnMut` closure
  --> fn_once_err.hrs:17:33
   |
14 |     let value = String.from "closure called"
   |         ----- captured outer variable
16 |     list <- sort_by_key |r|:
   |                         --- captured by this `FnMut` closure
17 |         sort_operations <- push value       // moves `value` out: can only happen once
   |                                 ^^^^^ move occurs because `value` has type `String`, which does not implement the `Copy` trait
```

The closure pushes `value` — a `String`, moved out of the capture — onto a vector. That can happen once, so the closure is `FnOnce` only, and `sort_by_key` needs to call it many times; the error names both facts. Mutating a capture *without* moving it is fine:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

fn main$:
    let mut list =
        [
            (Rectangle\ width = 10, height = 1),
            (Rectangle\ width = 3, height = 5),
            (Rectangle\ width = 7, height = 12),
        ]

    let mut num_sort_operations = 0

    list <- sort_by_key |r|:
        num_sort_operations += 1            // mutates a capture: FnMut, fine
        r <- width

    println! "{list:#?}, sorted in {num_sort_operations} operations"
```

```text
[
    Rectangle {
        width: 3,
        height: 5,
    },
    Rectangle {
        width: 7,
        height: 12,
    },
    Rectangle {
        width: 10,
        height: 1,
    },
], sorted in 6 operations
```

`num_sort_operations += 1` mutates a captured counter, so the closure is `FnMut`, which is what was asked for, and the sort reports how many times it looked. The block-bodied closure is the trailing form from chapter 12 — `list <- sort_by_key |r|:` and the body beneath — with the argument list closing after the block.

## 13.2 Iterators

An iterator produces a sequence of values, one at a time, on request. The whole of the `Iterator` trait that matters is one method:

```
pub trait Iterator:
    type Item
    fn next (&mut self) -> Option<Self.Item>
```

`next` returns `Some item` until the sequence is finished and `None` after; `Item` is an *associated type* (chapter 20) naming what it yields. Everything else is built on `next`:

```
fn main$:
    let v1 = vec! [1, 2, 3]
    let mut v1_iter = v1 <- iter$            // nothing happens yet
    assert_eq! (v1_iter <- next$) (Some (&1))
    assert_eq! (v1_iter <- next$) (Some (&2))
    assert_eq! (v1_iter <- next$) (Some (&3))
    assert_eq! (v1_iter <- next$) None

    for val in v1 <- iter$:                 // `for` calls next until None
        println! "Got: {val}"
```

```text
Got: 1
Got: 2
Got: 3
```

`v1 <- iter$` makes an iterator over references to the vector's elements — `Some (&1)`, a reference, because `iter` borrows — and each `next$` advances it, which is why `v1_iter` is `mut`. A `for` loop is exactly this: it calls `next` until `None`, and does not need the `mut` because it takes the iterator by value and does its own advancing. Three ways to make an iterator from a collection: `iter$` yields `&T`, `iter_mut$` yields `&mut T`, and `into_iter$` consumes the collection and yields `T`.

### Consumers and adaptors

Methods on iterators come in two kinds. A *consumer* calls `next` until the end and produces something:

```
fn main$:
    let v1 = vec! [1, 2, 3]
    let total: i32 = v1 <- iter$ <- sum$     // a consumer: drives the iterator to the end
    println! "{total}"
```

```text
6
```

`sum$` drives the iterator to exhaustion and adds. `collect$` gathers into a collection (the type must be written or inferable); `count$`, `max$`, `min$`, `last$`, `find`, `any`, `all` and `fold` are consumers too. An *adaptor* takes one iterator and returns another that transforms its items as they pass:

```
fn main$:
    let v1: Vec<i32> = vec! [1, 2, 3]
    let v2: Vec<_> =
        v1 <- iter$
           <- map (|x| x + 1)
           <- collect$
    println! "{v2:?}"
```

```text
[2, 3, 4]
```

`map (|x| x + 1)` yields each item plus one; `collect$` at the end consumes the result. That last step is not optional:

```
fn main$:
    let v1: Vec<i32> = vec! [1, 2, 3]
    v1 <- iter$ <- map (|x| x + 1);         // an adaptor alone does nothing
```

An adaptor alone does nothing, because iterators are *lazy*: `map` builds an iterator that will add one *when asked*, and nothing asks. The compiler warns that the value is unused ("iterators are lazy and do nothing unless consumed" — the warnings are off in the book's build, but you will see it). Every chain therefore ends in a consumer, or a `for`.

### Closures that capture their environment

An adaptor's closure can use variables from outside, which is most of their power:

```
#[derive PartialEq Debug]
struct Shoe
    size: u32
    style: String

fn shoes_in_size (shoes: Vec<Shoe>) (shoe_size: u32) -> Vec<Shoe>:
    shoes <- into_iter$
          <- filter (|s| s <- size == shoe_size)
          <- collect$

fn main$:
    let shoes =
        vec! [
            (Shoe\ size = 10, style = String.from "sneaker"),
            (Shoe\ size = 13, style = String.from "sandal"),
            (Shoe\ size = 10, style = String.from "boot"),
        ]

    let in_my_size = shoes_in_size shoes 10
    println! "{in_my_size:#?}"
```

```text
[
    Shoe {
        size: 10,
        style: "sneaker",
    },
    Shoe {
        size: 10,
        style: "boot",
    },
]
```

`filter (|s| s <- size == shoe_size)` keeps the shoes whose size equals a variable of the enclosing function; `into_iter$` consumes the vector so the kept shoes are moved into the result rather than copied. The function is three links on three lines — the vertical chain from the language guide's layout section, every `<-` aligned under the first — and reads top to bottom as "the shoes, filtered, collected".

### Chains

Adaptors compose, and this is where the chain layout matters:

```
fn main$:
    let words = ["apple", "banana", "cherry", "date", "elderberry", "fig"]

    let long_upper: Vec<String> =
        words <- iter$
              <- filter (|w| w <- len$ > 4)
              <- map (|w| w <- to_uppercase$)
              <- collect$

    println! "{long_upper:?}"

    let total_len: usize =
        words <- iter$
              <- map (|w| w <- len$)
              <- sum$
    println! "{total_len}"

    let first_long = words <- iter$ <- find (|w| w <- len$ > 5)
    println! "{first_long:?}"

    let any_fig = words <- iter$ <- any (|&w| w == "fig")
    println! "{any_fig}"

    for (i, w) in words <- iter$
        <- enumerate$
        <- skip 4:
        println! "{i}: {w}"
```

```text
["APPLE", "BANANA", "CHERRY", "ELDERBERRY"]
34
Some("banana")
true
4: elderberry
5: fig
```

Filter, map, collect; map then sum; `find` for the first match; `any` for a yes-or-no; `enumerate$ <- skip 4` to number the items and drop the first four. Each closure is one atom in parentheses, and when the chain grows past a line it goes vertical. `any (|&w| w == "fig")` uses a `&w` pattern in the closure's parameter to take the `&&str` the iterator yields down to a `&str` — a pattern, as in a `for` or a `match`. Read the chains aloud and they are English.

### Laying a chain out

The layout has one rule for chains, and the formatter applies it, so you rarely decide it yourself:

```
fn main$:
    let words = ["apple", "banana", "cherry", "date"]

    // one or two links: on the line, when the line fits
    let n = words <- len$
    let first = words <- iter$ <- next$

    // three or more: vertical, every arrow under the first
    let caps: Vec<String> =
        words <- iter$
              <- map (|w| w <- to_uppercase$)
              <- collect$

    // a block-bodied closure is written as a paren block: the body
    // beneath its parameters, the `)` on its own line at the closure's column
    let lengths: Vec<usize> =
        words <- iter$
              <- map (
                     |w|:
                         let l = w <- len$
                         l * 2
                 )
              <- collect$

    println! "{n} {first:?} {caps:?} {lengths:?}"
```

```text
4 Some("apple") ["APPLE", "BANANA", "CHERRY", "DATE"] [10, 12, 12, 8]
```

> **Harsh —** One or two links stay on their line when the line fits in 72 columns; three or more go vertical, one `<-` per line, every arrow under the first. A `=` always ends its line before a vertical chain begins, never starts one. A closure whose body is a block is written as a *paren block*: the `(` ends the arrow's line, the parameters sit on their own line, the body beneath them, and the `)` closes on a line of its own at the closure's column — the parentheses are transparent to the layout, and the block inside ends where they end. `hrs fmt` puts all of this where it belongs; write it however it comes out of your fingers, save, and read the result.

## 13.3 The pipes

Everything so far has reached into a value with `<-`: a method on the left of the arrow's data. The pipes go the other way. `|>` takes the value on its left and hands it to the *function* on its right; `<|` does the same from the other side. Where a chain says *this value, then this method on it*, a pipe says *this value, into this function*:

```
fn tokenize text: &str -> Vec<String>:
    text <- split_whitespace$
         <- map (|w| w <- to_lowercase$)
         <- collect$

fn count words: &Vec<String> -> usize:
    words <- len$

fn shout text: &str -> String:
    text <- to_uppercase$

fn main$:
    let text = "the cat sat on the mat"

    // a value flows left to right, into one function after another
    let n = text |> tokenize |> (|w| count (&w))
    println! "{n} words"

    // the same, right to left
    let m = (|w| count (&w)) <| tokenize <| text
    println! "{m} words"

    // a method reaches into a value; a pipe hands a value to a function
    let a = text <- to_uppercase$
    let b = text |> shout
    println! "{a} / {b}"
```

```text
6 words
6 words
THE CAT SAT ON THE MAT / THE CAT SAT ON THE MAT
```

> **Harsh —** `text |> tokenize` is `tokenize` applied to `text`, and the chain reads on: the next `|>` applies the closure to what came out. `<|` is the mirror — `f <| g <| x` applies `g` to `x` and `f` to the result. Both sides of a pipe are *atoms*: a value, a name, an isolated group. That is the one place the pipes and the arrow part company. Chapter 2 said an application binds tighter than `<-` — `tokenize text <- len$` applies `tokenize` first — but `tokenize text |> count` does *not* apply it first: every atom on a pipe's side is an argument, so that line hands `count` two of them, `tokenize` and `text`. To pipe a *result*, isolate it: `(tokenize text) |> count`. The reason is the pipes' own feature — a pipe may carry several values, `2.0 0.5 |> scale` — and a rule that let one of them be an application would have to guess where it ended. So a chain like `raw <- clone$` is isolated before it goes in, `(raw <- clone$) |> trim_ws`, and a closure is isolated the same way, `|> (|w| count (&w))` — the function a pipe applies is one atom too.

### Partial application

The pipes do one more thing, and it is the thing the arrow cannot do. A pipe may carry several values — `2.0 0.5 3.0 |> scale` — and when it carries *fewer* than the function takes, the missing ones are **deferred**: the result is a closure waiting for the rest.

```
fn scale (factor: f64) (offset: f64) (x: f64) -> f64:
    x * factor + offset

fn main$:
    // every parameter given: a plain call
    let y = 2.0 0.5 3.0 |> scale
    println! "{y}"

    // fewer given: what is missing is deferred, and the result is a closure
    let f = 2.0 0.5 |> scale          // waits for x
    println! "{}" (f 3.0)

    let g = scale <| 10.0             // filled from the right: waits for factor and offset
    println! "{}" (g 2.0 0.5)

    let h = 2.0 |> scale <| 10.0      // both sides: the hole is in the middle
    println! "{}" (h 0.5)

    // a partial is a value like any other
    let doubled: Vec<f64> =
        vec! [1.0, 2.0, 3.0]
            <- into_iter$
            <- map (2.0 0.0 |> scale)
            <- collect$
    println! "{doubled:?}"
```

```text
6.5
6.5
20.5
20.5
[2.0, 4.0, 6.0]
```

> **Harsh —** `|>` fills a function's parameters from the left, `<|` from the right, and the two together leave a hole in the middle. The result is one flat closure over whatever was not filled — a value like any other: bind it, pass it to `map`, return it from a function. There is no placeholder token; the hole is what is left. Harsh knows how many parameters `scale` takes because `scale` is declared in your project; for a function it cannot see into — the standard library, a crate you depend on — a pipe is a plain call with the values you gave, and the compiler says so if the count was wrong. Give a project function *more* than it takes and Harsh itself refuses:

```
fn sub (a: i32) (b: i32) (c: i32) -> i32:
    a - b - c

fn main$:
    let n = 1 2 3 4 |> sub
    println! "{n}"
```

```text
error: `sub` takes 3 parameter(s) and 4 were piped in
  --> pipe_too_many.hrs:5:21
   |
  5|     let n = 1 2 3 4 |> sub
   |                     ^^
```

### Pipelines

A pipe's result is a value, so pipes chain: each stage's output is the next stage's input, left to right, and a partial application makes a stage out of a function that needed more than one argument:

```
fn trim_ws s: String -> String:
    s <- trim$ <- to_string$

fn shout s: String -> String:
    s <- to_uppercase$

fn wrap (left: &str) (right: &str) (s: String) -> String:
    format! "{left}{s}{right}"

fn main$:
    let raw = String.from "   hello, harsh   "

    // a pipeline: each stage's result is the next stage's argument
    let out = (raw <- clone$) |> trim_ws |> shout |> ("[" "]" |> wrap)
    println! "{out}"

    // the same pipeline as a value, with the argument left out
    let banner = |s: String|:
        s |> trim_ws |> shout |> ("<" ">" |> wrap)
    println! "{}" (banner raw)
```

```text
[HELLO, HARSH]
<HELLO, HARSH>
```

`("[" "]" |> wrap)` is `wrap` with its first two parameters filled — a function of one `String` — and so it is a stage like `trim_ws` and `shout`. `banner` is the whole pipeline as a closure: the same stages, with the argument left for later. Read the line aloud: *raw, trimmed, shouted, wrapped*.

### When a pipe beats a closure

A closure that only forwards its argument is a partial application spelled the long way:

```
fn discount (rate: f64) (price: f64) -> f64:
    price * (1.0 - rate)

fn main$:
    let prices = vec! [10.0, 25.0, 40.0]

    // a closure that only forwards its argument...
    let a: Vec<f64> =
        prices <- iter$
               <- map (|p| discount 0.2 (*p))
               <- collect$
    // ...says nothing a partial does not say shorter
    let b: Vec<f64> =
        prices <- iter$
               <- copied$
               <- map (0.2 |> discount)
               <- collect$
    println! "{a:?} {b:?}"

    // where the closure earns its place: the argument is transformed on the way in
    let c: Vec<f64> =
        prices <- iter$
               <- map (|p| discount 0.2 (p + 5.0))
               <- collect$
    println! "{c:?}"
```

```text
[8.0, 20.0, 32.0] [8.0, 20.0, 32.0]
[12.0, 24.0, 36.0]
```

`|p| discount 0.2 (*p)` names `p` twice to say what `0.2 |> discount` says once. When the closure transforms its argument on the way in — `p + 5.0` — it is doing work the pipe cannot, and it stays. That is the whole rule: reach for a partial when the argument passes through untouched, and a closure when it does not.

## 13.4 Improving `minigrep`

Chapter 12's program used `clone` in `Config.build` and a `for` loop with a `push` in `search`; iterators remove both:

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let config = Config.build (env.args$) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config:
    // Take the iterator itself: no clones, and the argument list is consumed as it is read.
    pub fn build (mut args: impl Iterator<Item = String>) -> Result<Config, &'static str>:
        args <- next$                                   // the program name

        let Some query = args <- next$ else:
            return Err "Didn't get a query string"

        let Some file_path = args <- next$ else:
            return Err "Didn't get a file path"

        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

// One expression: lines, filtered, collected.
pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    contents <- lines$
             <- filter (|line| line <- contains query)
             <- collect$

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$

    contents <- lines$
             <- filter (|line| line <- to_lowercase$ <- contains (&query))
             <- collect$
```

```text
$ cargo run -- body poem.txt
I'm nobody! Who are you?
Are you nobody, too?
How dreary to be somebody!
```

`Config.build` now takes `impl Iterator<Item = String>` — the argument iterator itself, straight from `env.args$` — and pulls from it with `next$`: the program name is skipped, then `let Some query = args <- next$ else:` takes the query or returns the error. No vector, no indexes, no clones: each `String` is moved out of the iterator into the `Config`. And `search` is one expression, `contents <- lines$ <- filter (…) <- collect$`, three lines that say exactly what the loop did in six. Same output, same tests.

### Performance

The natural worry is that a chain of closures must be slower than a loop. It is not: Rust compiles adaptors and their closures down to the same machine code as the hand-written loop — sometimes better, since the compiler can see the whole chain — and the standard library's own benchmarks show no difference. This is the *zero-cost abstraction* Rust promises: what you do not use costs nothing, and what you do use costs what it would have cost by hand. Write the chain; it is the clearer of the two and not the slower.

## 13.5 What you have

Closures capture by reference, mutable reference or value, `move` to force the last; they are `Fn`, `FnMut` or `FnOnce` by what they do with their captures, and a function taking one says which it needs. Iterators produce items through `next`; `iter$`, `iter_mut$`, `into_iter$` make them; adaptors like `map` and `filter` are lazy and take closures; consumers like `sum$` and `collect$` run the chain; `for` is a consumer too. Harsh lays a chain out vertically with the arrows aligned and a closure's block body under its parameters. `|>` and `<|` hand a value to a function, from the left or the right; with fewer values than the function takes they defer the rest as a flat closure, which is how a partial application is written.

Next: cargo — profiles, documentation, publishing, workspaces — before the second half of the book turns to smart pointers and concurrency.

# 14. More about cargo

Cargo has done the building so far, and it does more: release builds, documentation, publishing, and workspaces for a project of several crates. This chapter is short and mostly about commands, because there is little to show that compiles — but one thing, documentation comments, is code in your files and worth seeing through the transpiler.

## 14.1 Build profiles

`cargo build` (and `hrs build`) uses the *dev* profile: fast to compile, unoptimised, with debug assertions and overflow checks on. `cargo build --release` uses the *release* profile: slow to compile, optimised, checks off, and this is the binary you ship or benchmark. The profiles are configurable in `Cargo.toml`:

```text
[profile.dev]
opt-level = 0

[profile.release]
opt-level = 3
```

`opt-level` runs from 0 to 3; those are the defaults, and you change them when you have a reason — a dev build of a program that is too slow to test unoptimised, for instance. Harsh adds nothing here: `hrs build --release` passes the flag through.

## 14.2 Documentation comments

Rust has a comment form that becomes documentation. `///` documents the item that follows; `//!` documents the enclosing item, and at the top of a file that is the crate or module itself:

`src/lib.hrs`

```
//! # My Crate
//!
//! `my_crate` is a collection of utilities to make performing certain
//! calculations more convenient. This comment documents the crate itself.

/// Adds one to the number given.
///
/// # Examples
///
/// ```
/// let arg = 5
/// let answer = my_crate.add_one arg
///
/// assert_eq! 6 answer
/// ```
pub fn add_one x: i32 -> i32:
    x + 1
```

```text
$ hrs test
   Doc-tests my_crate

running 1 test
test target/hrs/lib.rs - add_one (line 10) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
```

`cargo doc --open` renders every `///` and `//!` in the crate as HTML, with the Markdown inside them — headings, code blocks, links — laid out. The conventional sections are `# Examples`, `# Panics` (when the function can), `# Errors` (what `Err`s it returns) and `# Safety` (for `unsafe` functions). The example in the `///` is Harsh, like everything else in the file, and `cargo test` *runs it* — every code block in a doc comment is a test, so documentation cannot drift from the code without failing the build.

> **Harsh —** A fenced block in a doc comment is the one kind of comment the transpiler reads. The documentation tool lifts such a block out, compiles it and runs it — it is code, not prose — so it is written in Harsh and the transpiler writes out the Rust that tool expects, the same translation it does everywhere else. A fence tagged with another language — `text`, `toml` — is prose and is left exactly as you wrote it. A line beginning `#` inside the fence is the documentation tool's own mark for *compile this but do not show it*, and it survives the translation.

### Re-exports for a public API

Chapter 7 mentioned `pub use` for presenting a different structure to users than the code has. In a library it is what makes the docs usable:

```
//! Art: a library for modelling artistic concepts.

pub use self.kinds.PrimaryColor
pub use self.kinds.SecondaryColor
pub use self.utils.mix

pub mod kinds:
    /// The primary colors according to the RYB color model.
    #[derive Debug Clone Copy]
    pub enum PrimaryColor
        Red
        Yellow
        Blue

    /// The secondary colors according to the RYB color model.

    #[derive Debug]
    pub enum SecondaryColor
        Orange
        Green
        Purple

pub mod utils:
    use crate.kinds.*

    /// Combines two primary colors in equal amounts to create a secondary color.
    pub fn mix (c1: PrimaryColor) (c2: PrimaryColor) -> SecondaryColor:
        match (c1, c2):
            (PrimaryColor.Red, PrimaryColor.Yellow) | (PrimaryColor.Yellow, PrimaryColor.Red) => SecondaryColor.Orange
            (PrimaryColor.Yellow, PrimaryColor.Blue) | (PrimaryColor.Blue, PrimaryColor.Yellow) => SecondaryColor.Green
            _ => SecondaryColor.Purple

fn main$:
    // With the re-exports, a user writes `art::mix` and `art::PrimaryColor`,
    // never `art::utils::mix`. Inside the crate they are reachable both ways.
    let red = PrimaryColor.Red
    let yellow = PrimaryColor.Yellow
    println! "{:?}" (mix red yellow)
```

```text
Orange
```

Inside, `PrimaryColor` lives in `kinds` and `mix` in `utils`, which is a sensible arrangement for the authors. A user does not care, and without the re-exports would have to write `art.utils.mix` and `art.kinds.PrimaryColor` and know which was where. The three `pub use` lines put all of them at the top of the crate; `cargo doc` lists the re-exports on the front page, and the user's `use art.(mix, PrimaryColor)` is the whole of what they learn about the layout.

## 14.3 Publishing

`crates.io` is the registry `cargo` fetches dependencies from, and publishing to it is `cargo publish` after `cargo login` once with a token from the site. `Cargo.toml` needs `name` (unique on the registry), `version`, `description` and `license` before the registry will accept it; a publish is permanent, since other crates may depend on it — versions can be *yanked* (`cargo yank --vers 1.0.1`) to stop new projects picking them up, but never deleted. For a Harsh crate, `hrs export` writes the transpiled Rust as a plain crate under `target/export`, which is what you publish: the registry, and the people who depend on you, see Rust.

## 14.4 Workspaces

A *workspace* is a set of packages sharing one `Cargo.lock` and one `target/` directory, for a project that has grown to several crates — a binary and the libraries it is split into, say. The root `Cargo.toml` lists the members:

```text
[workspace]
members = ["adder", "add_one"]
```

and each member is an ordinary package in its own directory, depending on its siblings by path (`add_one = { path = "../add_one" }`). `cargo build` at the root builds them all; `cargo test -p add_one` tests one. `hrs` walks each member's `src/` in turn.

## 14.5 Installing binaries

`cargo install crate_name` fetches a crate from the registry, builds its binary and puts it in `~/.cargo/bin`, which is on your `PATH` once Rust is installed — it is how command-line tools written in Rust get distributed, and how `hrs` itself is installed from its repository (`cargo install --path .`). Any binary named `cargo-something` on the `PATH` also becomes a subcommand, `cargo something`, which is how cargo's own set of commands is extended.

## 14.6 What you have

`--release` for the optimised build; `///` and `//!` for documentation that `cargo doc` renders and `cargo test` runs; `pub use` to give a crate a public shape; `cargo publish` for the registry, from `hrs export`'s Rust; workspaces for several crates; `cargo install` for tools. None of it is Harsh's, and all of it works on the transpiled tree.

Next: smart pointers — `Box`, `Rc` and `RefCell`, which are how Rust does the data structures a garbage-collected language takes for granted.

# 15. Smart pointers

A reference, `&T`, points at a value it does not own. A *smart pointer* is a struct that points at a value and *does* own it, with some rule about that ownership: `Box<T>` puts a value on the heap; `Rc<T>` lets several owners share one; `RefCell<T>` moves the borrow rules from compile time to run time. `String` and `Vec` are smart pointers too, by this definition — they own heap memory and know its length. What makes the three in this chapter worth a chapter is that between them they build the data structures that seem impossible under chapter 4's rules: recursive lists, shared graphs, values changed through a shared reference.

## 15.1 `Box<T>`

A `Box` is the simplest: one value, on the heap, owned by the box:

```
fn main$:
    let b = Box.new 5           // an i32 on the heap
    println! "b = {b}"
```

```text
b = 5
```

`Box.new 5` allocates and returns the box; `b` is used like the `i32` it holds, and freed when `b` goes out of scope. On its own that is pointless — an `i32` is happier on the stack — and boxes are for three situations: a type whose size is not known at compile time, a large value you want to move without copying, and a value you own but only care about the trait it implements (chapter 18). The first is the classic:

```
enum List
    Cons i32 List
    Nil

fn main$:
    let list = List.Cons 1 (List.Cons 2 (List.Cons 3 List.Nil))
    let _ = list
```

```text
error[E0072]: recursive type `List` has infinite size
  --> cons_err.hrs:1:1
   |
 1 | enum List
   | ^^^^^^^^^
 2 |     Cons i32 List
   |              ---- recursive without indirection
   = help: insert some indirection (e.g., a `Box`, `Rc`, or `&`) to break the cycle (hrs 2:14)
```

A *cons list* — each element holds a value and the rest of the list — is the recursive structure of the functional languages. Written directly it is a type error: a `List` contains a `List` contains a `List`, so the compiler cannot say how many bytes one takes. The help says exactly what to do: put the recursion behind a pointer, whose size is known.

```
#[derive Debug]
enum List
    Cons i32 (Box<List>)
    Nil

use List.(Cons, Nil)

fn main$:
    let list =
        Cons
            1
            (Box.new (Cons 2 (Box.new (Cons 3 (Box.new Nil)))))
    println! "{list:?}"
```

```text
Cons(1, Cons(2, Cons(3, Nil)))
```

`Cons (i32, Box<List>)` is an `i32` and a pointer, a fixed size, and the rest of the list is on the heap behind it. `use List.(Cons, Nil)` brings the variants in so the construction reads as `Cons 1 (Box.new (Cons 2 …))` — each `Cons` applied to a value and a boxed tail, each argument that is a call isolated. `Box` does nothing but own and point, which is why it is the pointer to reach for first.

## 15.2 `Deref`: treating a pointer like a reference

`*b` on a `Box` gives the value inside, as `*r` on a reference does. That works because `Box` implements the `Deref` trait, and you can implement it for a type of your own:

```
use std.ops.Deref

struct MyBox<T> T

impl<T> MyBox<T>:
    fn new x: T -> MyBox<T>:
        MyBox x

impl<T> Deref for MyBox<T>:
    type Target = T

    fn deref (&self) -> &Self.Target:
        &self.0

fn hello name: &str:
    println! "Hello, {name}!"

fn main$:
    let x = 5
    let y = MyBox.new x
    assert_eq! 5 x
    assert_eq! 5 (*y)            // *y is *(y.deref())

    let m = MyBox.new (String.from "Rust")
    hello (&m)                   // &MyBox<String> -> &String -> &str: deref coercion
    hello (&(*m)[..])            // what the coercion saves you writing
```

```text
Hello, Rust!
Hello, Rust!
```

`MyBox<T>` is a tuple struct with one field; `impl Deref for MyBox<T>` says what `*` does: `deref` returns a reference to the field, and `*y` is sugar for `*(y.deref())`. The `type Target = T` line is an associated type — the trait needs to know what a `MyBox<T>` derefs *to*.

The second half is *deref coercion*: `hello` takes `&str`, and is passed `&m`, a `&MyBox<String>`. The compiler applies `Deref` as many times as needed to make the types meet — `&MyBox<String>` to `&String` to `&str` — so the call compiles as written. The last line spells out what the coercion did: `&(*m)[..]`. This is the mechanism behind every `&String` that was passed where a `&str` was wanted since chapter 4; it was never a special case, only `Deref`.

## 15.3 `Drop`: code that runs on cleanup

A smart pointer's other half is what happens when it goes away. The `Drop` trait is a method the compiler calls when a value goes out of scope:

```
struct CustomSmartPointer
    data: String

impl Drop for CustomSmartPointer:
    fn drop (&mut self):
        println!
            "Dropping CustomSmartPointer with data `{}`!"
            (self <- data)

fn main$:
    let c = CustomSmartPointer\ data = String.from "my stuff"
    let d = CustomSmartPointer\ data = String.from "other stuff"
    println! "CustomSmartPointers created."
    drop c                       // std::mem::drop: early, explicit
    println! "CustomSmartPointer dropped before the end of main."

    let _ = d
```

```text
CustomSmartPointers created.
Dropping CustomSmartPointer with data `my stuff`!
CustomSmartPointer dropped before the end of main.
Dropping CustomSmartPointer with data `other stuff`!
```

`impl Drop for CustomSmartPointer` with `fn drop (&mut self)` — the body runs at the end of the owner's scope, in reverse order of creation, which is why `d` is dropped after `c` would have been. `Box` uses `Drop` to free its heap memory, `File` to close the file, a lock guard to release the lock. You cannot call `x <- drop$` yourself — that would leave `x` in scope, to be dropped again — but you can call the function `drop x`, which takes the value and ends it now; the example does, and `c`'s message appears before the last `println!`. Deterministic cleanup with no `finally` and no garbage collector is what `Drop` gives, and it is most of the reason Rust needs no `defer`.

## 15.4 `Rc<T>`: shared ownership

Chapter 4 said a value has exactly one owner. Sometimes that is the wrong model — a node in a graph belongs to every edge that reaches it — and `Rc<T>`, the *reference-counted* pointer, is how Rust expresses it:

```
#[derive Debug]
enum List
    Cons i32 (Rc<List>)
    Nil

use List.(Cons, Nil)
use std.rc.Rc

fn main$:
    let a = Rc.new (Cons 5 (Rc.new (Cons 10 (Rc.new Nil))))
    println! "count after creating a = {}" (Rc.strong_count (&a))

    let b = Cons 3 (Rc.clone (&a))
    println! "count after creating b = {}" (Rc.strong_count (&a))

    do:
        let c = Cons 4 (Rc.clone (&a))
        println! "count after creating c = {}" (Rc.strong_count (&a))

        let _ = c

    println!
        "count after c goes out of scope = {}"
        (Rc.strong_count (&a))

    let _ = b
```

```text
count after creating a = 1
count after creating b = 2
count after creating c = 3
count after c goes out of scope = 2
```

`a` is a list held in an `Rc`; `b` and `c` are lists that each *share* `a` as their tail, by `Rc.clone (&a)`. `Rc.clone` does not copy the list; it increments a count and returns another pointer to the same allocation, and `Rc.strong_count` shows the count going 1, 2, 3 and back to 2 when `c` is dropped. The value is freed when the count reaches zero — when the last owner is gone — and that is the whole rule. Two things to know: `Rc.clone` is the conventional spelling precisely because it is *not* a deep copy, so a reader can tell the cheap clones from the expensive ones; and `Rc` is for a single thread — chapter 16 has `Arc` for the rest.

An `Rc<T>` only hands out *shared* references to its value. Several owners and one of them writing would be the data race of chapter 4, so through an `Rc` the value is read-only. Which leaves the question of how to change something that is shared.

## 15.5 `RefCell<T>`: borrowing checked at run time

The borrow rules — one `&mut` or many `&`, never both — are enforced by the compiler, on what it can prove:

```
fn main$:
    let x = 5
    let y = &mut x
    *y += 1
    println! "{x}"
```

```text
error[E0596]: cannot borrow `x` as mutable, as it is not declared as mutable
  --> refcell_err.hrs:3:13
   |
 3 |     let y = &mut x
   |             ^^^^^^ cannot borrow as mutable
   = help: consider changing this to be mutable (hrs 2:9)
```

`RefCell<T>` enforces the same rules *at run time*: `borrow$` gives a shared reference and `borrow_mut$` a mutable one, each counted while it lives, and a violation is a panic rather than a compile error. The value may then be mutated through something that is itself only shared — an `Rc`, say:

```
use std.cell.RefCell
use std.rc.Rc

#[derive Debug]
enum List
    Cons (Rc<RefCell<i32>>) (Rc<List>)
    Nil

use List.(Cons, Nil)

fn main$:
    let value = Rc.new (RefCell.new 5)
    let a = Rc.new (Cons (Rc.clone (&value)) (Rc.new Nil))
    let b = Cons (Rc.new (RefCell.new 3)) (Rc.clone (&a))
    let c = Cons (Rc.new (RefCell.new 4)) (Rc.clone (&a))

    *value <- borrow_mut$ += 10          // change the 5 that a, b and c all share
    println! "a after = {a:?}"
    println! "b after = {b:?}"
    println! "c after = {c:?}"
```

```text
a after = Cons(RefCell { value: 15 }, Nil)
b after = Cons(RefCell { value: 3 }, Cons(RefCell { value: 15 }, Nil))
c after = Cons(RefCell { value: 4 }, Cons(RefCell { value: 15 }, Nil))
```

`value` is an `Rc<RefCell<i32>>` shared by three lists, and `*value <- borrow_mut$ += 10` writes through it: `borrow_mut$` returns a guard, `*` reaches the number, and the guard's `Drop` releases the borrow at the end of the statement. All three lists see `15`. This is *interior mutability*: a value that looks immutable from outside — `a`, `b`, `c` are not `mut` — and is mutated inside, under the borrow rules, checked as it happens.

And checked they are:

```
use std.cell.RefCell

fn main$:
    let cell = RefCell.new 5
    let one = cell <- borrow_mut$
    let two = cell <- borrow_mut$       // two mutable borrows: the rule, checked at runtime
    println! "{one} {two}"
```

```text
thread 'main' panicked at refcell_panic.hrs:6:
already borrowed: BorrowMutError
```

Two `borrow_mut$` guards alive at once is exactly what the compiler forbids for `&mut`, and `RefCell` panics with `already borrowed`. The rules did not change; only when they are checked did. Use `RefCell` when you know the code respects the rules and the compiler cannot see it — a value mutated through shared handles, a mock object recording calls in a test — and accept that the check has moved from build time to the first run that violates it.

## 15.6 Reference cycles

`Rc` frees when the count reaches zero; two `Rc`s that point at each other never reach zero, and the memory leaks. Rust does not prevent this — a leak is safe, just wasteful — so structures with pointers in both directions use a `Weak<T>` for one direction. A weak pointer does not count toward ownership, and to use it you `upgrade$` it into an `Option<Rc<T>>` that is `None` if the value is gone:

```
use std.cell.RefCell
use std.rc.(Rc, Weak)

#[derive Debug]
struct Node
    value: i32
    parent: RefCell<Weak<Node>>
    children: RefCell<Vec<Rc<Node>>>

fn main$:
    let leaf =
        Rc.new (Node\
            value = 3
            parent = RefCell.new (Weak.new$)
            children = RefCell.new (vec! [])
        )
    println!
        "leaf strong = {}, weak = {}"
        (Rc.strong_count (&leaf))
        (Rc.weak_count (&leaf))

    do:
        let branch =
            Rc.new (Node\
                value = 5
                parent = RefCell.new (Weak.new$)
                children = RefCell.new (vec! [Rc.clone (&leaf)])
            )
        *leaf <- parent <- borrow_mut$ = Rc.downgrade (&branch)
        println!
            "branch strong = {}, weak = {}"
            (Rc.strong_count (&branch))
            (Rc.weak_count (&branch))
        println!
            "leaf strong = {}, weak = {}"
            (Rc.strong_count (&leaf))
            (Rc.weak_count (&leaf))

    println!
        "leaf parent = {:?}"
        (leaf <- parent
              <- borrow$
              <- upgrade$
              <- map (|p| p <- value))
    println!
        "leaf strong = {}, weak = {}"
        (Rc.strong_count (&leaf))
        (Rc.weak_count (&leaf))
```

```text
leaf strong = 1, weak = 0
branch strong = 1, weak = 1
leaf strong = 2, weak = 0
leaf parent = None
leaf strong = 1, weak = 0
```

A tree: a `Node` owns its children (`Rc`) and *knows* its parent (`Weak`). `leaf` starts with no parent; inside the block `branch` is made with `leaf` as a child, and `leaf`'s parent is set with `Rc.downgrade (&branch)` — a weak pointer, so `branch`'s strong count stays 1 and its weak count becomes 1. When the block ends `branch` is dropped, the strong count hits zero, the node is freed, and `leaf`'s `upgrade$` afterwards returns `None`: the parent is gone and the child knows it. No cycle, no leak, and the counts printed at each step show exactly why.

## 15.7 What you have

`Box<T>` owns one heap value and makes a recursive type finite. `Deref` makes a pointer usable as a reference and drives the coercion from `&String` to `&str`. `Drop` runs cleanup at the end of scope, in order, and `drop x` runs it early. `Rc<T>` gives a value several owners and frees it when the last is gone; `RefCell<T>` moves the borrow rules to run time so a shared value can be changed; together they make shared mutable structures; and `Weak<T>` breaks the cycles that would otherwise leak.

Next: concurrency — threads, channels and the `Mutex`, and how the ownership rules make data races a compile error.

# 16. Fearless concurrency

Concurrent code — several things happening at once, on threads — is where most languages' worst bugs live: data races, where two threads touch one value with no ordering; deadlocks; values used after the thread that owned them is gone. Rust makes the first of those a compile error, using nothing new: the ownership and borrowing rules of chapter 4, plus two traits that say which types may cross a thread boundary. The result is what the Rust community calls *fearless* concurrency — not that it is easy, but that the compiler catches the class of mistake that is otherwise found in production at three in the morning. This chapter is threads, message passing, shared state, and the two traits.

## 16.1 Threads

`thread.spawn` runs a closure on a new thread and returns a handle:

```
use std.thread
use std.time.Duration

fn main$:
    let handle = thread.spawn ||:
        for i in 1..4:
            println! "hi number {i} from the spawned thread!"
            thread.sleep (Duration.from_millis 1)

    println! "main is waiting"
    handle <- join$ <- unwrap$      // block until the spawned thread finishes
    println! "main is done"
```

```text
hi number 1 from the spawned thread!
main is waiting
hi number 2 from the spawned thread!
hi number 3 from the spawned thread!
main is done
```

The spawned thread prints three lines while the main thread carries on — the two are running at once, and without the `join` the main thread would reach its end and the process would exit, taking the spawned thread with it, however far it had got. `handle <- join$` blocks until the thread finishes, and `<- unwrap$` handles the case where the thread panicked. The closure is `||:` with its body beneath, the trailing block form; here it captures nothing.

Usually it captures something, and this is where ownership meets threads:

```
use std.thread

fn main$:
    let v = vec! [1, 2, 3]

    let handle = thread.spawn ||:
        println! "Here's a vector: {v:?}"

    handle <- join$ <- unwrap$
```

```text
error[E0373]: closure may outlive the current function, but it borrows `v`, which is owned by the current function
  --> move_err.hrs:6:31
   |
 6 |     let handle = thread.spawn ||:
   |                               ^^ may outlive borrowed value `v`
 7 |         println! "Here's a vector: {v:?}"
   |                                     - `v` is borrowed here
   = note: function requires argument type to outlive `'static` (hrs 6:18)
   = help: to force the closure to take ownership of `v` (and any other referenced variables), use the `move` keyword (hrs 6:31)
```

The closure borrows `v`, and the compiler asks a question the language forces: how long will the thread run? It cannot know — the thread might outlive `main`, and then the borrow would dangle. So a closure passed to `spawn` must own everything it uses, and the help says how:

```
use std.thread

fn main$:
    let v = vec! [1, 2, 3]

    let handle = thread.spawn move ||:
        println! "Here's a vector: {v:?}"

    handle <- join$ <- unwrap$
```

```text
Here's a vector: [1, 2, 3]
```

`move ||:` — chapter 13's `move`, moving `v` into the closure, which then owns it for as long as the thread runs. This is the one place `move` is not optional, and the error when it is missing is precise.

## 16.2 Message passing

One way to share data between threads is not to share it: send it. A *channel* has a transmitter and a receiver, and a value sent down it is moved from one thread to the other:

```
use std.sync.mpsc
use std.thread

fn main$:
    let (tx, rx) = mpsc.channel$

    thread.spawn move ||:
        let val = String.from "hi"
        tx <- send val <- unwrap$

    let received = rx <- recv$ <- unwrap$
    println! "Got: {received}"
```

```text
Got: hi
```

`mpsc.channel$` — *multiple producer, single consumer* — returns the pair `(tx, rx)`. The spawned thread moves `tx` in, makes a `String`, and `tx <- send val` moves the string into the channel; `rx <- recv$` blocks the main thread until something arrives and hands it over. The string was on the spawned thread and is now on the main one, and no lock was taken because at no moment did both have it.

That "moved" is enforced:

```
use std.sync.mpsc
use std.thread

fn main$:
    let (tx, rx) = mpsc.channel$

    thread.spawn move ||:
        let val = String.from "hi"
        tx <- send val <- unwrap$
        println! "val is {val}"        // val was moved into the channel

    let received = rx <- recv$ <- unwrap$
    println! "Got: {received}"
```

```text
error[E0382]: borrow of moved value: `val`
  --> channel_moved.hrs:10:26
   |
 8 |         let val = String.from "hi"
   |             --- move occurs because `val` has type `String`, which does not implement the `Copy` trait
 9 |         tx <- send val <- unwrap$
   |                    --- value moved here
10 |         println! "val is {val}"        // val was moved into the channel
   |                          ^^^^^ value borrowed here after move
   = help: consider cloning the value if the performance cost is acceptable (hrs 9:23)
```

`send` took `val`; using it afterwards is chapter 4's error, and it is the right error, because the receiving thread may have modified or dropped the value by now. In a language where sending is a copy of a pointer, this compiles and the two threads quietly share a string.

Several producers, one consumer:

```
use std.sync.mpsc
use std.thread
use std.time.Duration

fn main$:
    let (tx, rx) = mpsc.channel$
    let tx1 = tx <- clone$

    thread.spawn move ||:
        for val in ["hi", "from", "the", "thread"]:
            tx1 <- send (String.from val) <- unwrap$
            thread.sleep (Duration.from_millis 1)

    thread.spawn move ||:
        for val in ["more", "messages", "for", "you"]:
            tx <- send (String.from val) <- unwrap$
            thread.sleep (Duration.from_millis 1)

    let mut received: Vec<String> = rx <- iter$ <- collect$    // ends when every sender is dropped
    received <- sort$
    println! "{received:?}"
```

```text
["for", "from", "hi", "messages", "more", "the", "thread", "you"]
```

`tx <- clone$` makes a second transmitter for the second thread. `rx <- iter$` yields each message as it arrives and ends when *every* transmitter has been dropped — both threads finish, their `tx`s go out of scope, the iterator stops, and `collect$` has all eight words. (They are sorted before printing because the two threads' messages interleave in an order the scheduler decides, and the book's output has to be the same every build.)

## 16.3 Shared state

The other way is to share the data and take turns. A `Mutex` — *mutual exclusion* — holds a value and gives out access to one thread at a time:

```
use std.sync.Mutex

fn main$:
    let m = Mutex.new 5

    do:
        let mut num = m <- lock$ <- unwrap$   // blocks until the lock is free
        *num = 6
    // the guard was dropped at the end of the block, releasing the lock

    println! "m = {m:?}"
```

```text
m = Mutex { data: 6, poisoned: false, .. }
```

`m <- lock$` blocks until the lock is free and returns a *guard* that derefs to the value: `*num = 6` writes through it. The guard implements `Drop`, and dropping it releases the lock — which happens at the end of the `do:` block here, and would happen at the end of any scope, so a lock cannot be forgotten. This is chapter 15's `Deref` and `Drop` doing the work: the API that makes a mutex hard to misuse is made of two traits.

To share a mutex between threads, it needs several owners, and chapter 15's answer to that was `Rc`:

```
use std.rc.Rc
use std.sync.Mutex
use std.thread

fn main$:
    let counter = Rc.new (Mutex.new 0)
    let mut handles = vec! []

    for _ in 0..10:
        let counter = Rc.clone (&counter)

        let handle = thread.spawn move ||:
            let mut num = counter <- lock$ <- unwrap$
            *num += 1

        handles <- push handle

    for handle in handles:
        handle <- join$ <- unwrap$

    println! "Result: {}" (*counter <- lock$ <- unwrap$)
```

```text
error[E0277]: `Rc<Mutex<i32>>` cannot be sent between threads safely
  --> mutex_rc_err.hrs:12:35
   |
12 |         let handle = thread.spawn move ||:
   |                      ------------ required by a bound introduced by this call
   |                                   ^^^^^^^^ `Rc<Mutex<i32>>` cannot be sent between threads safely
   |                                   ------- within this `{closure@mutex_rc_err.hrs:12:36: 12:43}`
   = help: within `{closure@mutex_rc_err.hrs:12:36: 12:43}`, the trait `Send` is not implemented for `Rc<Mutex<i32>>`
   = note: required because it's used within this closure (hrs 12:35)
   = note: required by a bound in `spawn`
```

Refused. `Rc` counts references without any synchronisation — two threads incrementing the count at once would corrupt it — and so `Rc<T>` is not `Send`, and `spawn` requires `Send`. The type system knows which types are safe to move across threads, and this one is not. The thread-safe reference count is `Arc`, *atomic* `Rc`, with the same API:

```
use std.sync.(Arc, Mutex)
use std.thread

fn main$:
    let counter = Arc.new (Mutex.new 0)
    let mut handles = vec! []

    for _ in 0..10:
        let counter = Arc.clone (&counter)

        let handle = thread.spawn move ||:
            let mut num = counter <- lock$ <- unwrap$
            *num += 1

        handles <- push handle

    for handle in handles:
        handle <- join$ <- unwrap$

    println! "Result: {}" (*counter <- lock$ <- unwrap$)
```

```text
Result: 10
```

`Arc.new (Mutex.new 0)`, `Arc.clone (&counter)` for each thread, `move` the clone into the closure, lock, increment; join all ten; read the result. Ten threads incremented one counter and the answer is ten, and any attempt to write the program without the lock — an `Arc<i32>` and `+= 1` — would not compile, because `Arc` only hands out shared references. The pattern `Arc<Mutex<T>>` is the standard shape of shared mutable state across threads, and every piece of it is a chapter you have read.

## 16.4 `Send` and `Sync`

The rejection of `Rc` came from two *marker traits* — traits with no methods, that a type implements to make a claim:

- `Send`: the type may be *moved* to another thread. Almost everything is `Send`; `Rc<T>` is the standard exception, and raw pointers.
- `Sync`: the type may be *referenced* from several threads at once — `&T` is `Send`. `Mutex<T>` is `Sync`; `RefCell<T>` is not, since its borrow counting is not thread-safe.

Both are implemented automatically for any type made entirely of `Send`/`Sync` parts, so you rarely write them; you meet them as the bound `spawn` puts on its closure, and as the error when a type does not qualify. They are the whole of Rust's concurrency guarantee: the rules of chapter 4 decide who may touch what, and these two traits decide which types may cross the line, and the compiler checks both.

## 16.5 What you have

`thread.spawn` with a `move` closure and `join$` on the handle. Channels — `mpsc.channel$`, `send` moves a value across, `recv$` and `rx <- iter$` receive, clone the transmitter for more producers — for sharing by not sharing. `Mutex` for taking turns, with a guard that unlocks on drop; `Arc` for owning one across threads; `Arc<Mutex<T>>` as the pattern. `Send` and `Sync`, which are what make the compiler refuse the unsafe versions.

Next: `async` — concurrency without threads, for the programs that wait on the network more than they compute.

# 17. Async

Chapter 16's threads are the right tool when the work is computation. When the work is *waiting* — for a network reply, a file, a timer — a thread spends its time blocked, and a program with ten thousand connections cannot have ten thousand threads. `async` is Rust's answer: functions that can pause at the points where they would wait, and a *runtime* that runs many of them on a few threads, switching between them at those points. This chapter is `async`/`await`, tasks, joining and racing futures, and the one rule about blocking. The examples use the `tokio` runtime, because the language provides the syntax and the `Future` trait but deliberately no runtime; each example is a small project with a dependency, run with `hrs run`.

## 17.1 `async` and `await`

`Cargo.toml`

```text
[dependencies]
tokio = { version = "1.40", features = ["rt", "macros", "time", "sync"] }
```

`src/main.hrs`

```
use tokio.time.(sleep, Duration)

async fn say_hello name: &str -> String:
    sleep (Duration.from_millis 10) <- await     // yields to the runtime, does not block a thread
    format! "hello, {name}"

#[tokio.main (flavor = "current_thread")]
async fn main$:
    let greeting = say_hello "async" <- await
    println! "{greeting}"
```

```text
$ hrs run
hello, async
```

`async fn say_hello` returns not a `String` but a *future* — a value that will produce a `String` when driven to completion. `<- await` on a future drives it: if it is ready, the value; if it must wait, the current function *pauses*, the runtime runs something else, and this function resumes later, on the same line. `sleep (…) <- await` is the waiting point here, and while it waits no thread is blocked. `#[tokio.main]` turns `main` into the runtime's entry: it builds the runtime and runs `main`'s future to completion. The attribute's path is written with Harsh's dot, `tokio.main`, like any path.

Two things to hold: an `async fn` does nothing until awaited — calling it builds the future and returns immediately — and `await` can only appear inside an `async` function or block, because only those can pause.

## 17.2 Tasks

A *task* is the async counterpart of a thread: a future handed to the runtime to run independently:

`Cargo.toml`

```text
[dependencies]
tokio = { version = "1.40", features = ["rt", "macros", "time", "sync"] }
```

`src/main.hrs`

```
use tokio.time.(sleep, Duration)

#[tokio.main (flavor = "current_thread")]
async fn main$:
    let handle = tokio.spawn async:
        for i in 1..4:
            println! "hi number {i} from the first task!"
            sleep (Duration.from_millis 1) <- await

    for i in 1..3:
        println! "hi number {i} from the second task!"
        sleep (Duration.from_millis 1) <- await

    handle <- await <- unwrap$
```

```text
$ hrs run
hi number 1 from the second task!
hi number 1 from the first task!
hi number 2 from the second task!
hi number 2 from the first task!
hi number 3 from the first task!
```

`tokio.spawn async:` — `spawn` takes a future, and `async:` opens a block that is one, with its body beneath, laid out as a closure's would be. The two loops run interleaved on one thread: each `sleep <- await` is a point where the runtime switches to the other. `handle <- await` waits for the task, as `join$` did for a thread. The order of the lines is fixed here because a `current_thread` runtime switches only at awaits, deterministically; on a multi-threaded runtime it would vary, as chapter 16's did.

### Joining futures

Two futures can also be run to completion *together*, without spawning:

`Cargo.toml`

```text
[dependencies]
tokio = { version = "1.40", features = ["rt", "macros", "time", "sync"] }
```

`src/main.hrs`

```
use tokio.time.(sleep, Duration)

#[tokio.main (flavor = "current_thread")]
async fn main$:
    let fut1 = async:
        for i in 1..4:
            println! "hi number {i} from the first task!"
            sleep (Duration.from_millis 1) <- await

    let fut2 = async:
        for i in 1..3:
            println! "hi number {i} from the second task!"
            sleep (Duration.from_millis 1) <- await

    tokio.join! fut1 fut2;             // run both to completion, alternating at each await
```

```text
$ hrs run
hi number 1 from the first task!
hi number 1 from the second task!
hi number 2 from the second task!
hi number 2 from the first task!
hi number 3 from the first task!
```

`async:` blocks assigned to `let` are futures that have not started. `tokio.join! fut1 fut2` polls both, alternating whenever one waits, and finishes when both have — returning a tuple of their results, discarded here with a `;`. The difference from `spawn`: a joined future runs inside the current task and can borrow from it, where a spawned task is independent and must own what it uses (`move`, chapter 16 again).

## 17.3 Message passing

Chapter 16's channel has an async twin, whose `recv` is a future:

`Cargo.toml`

```text
[dependencies]
tokio = { version = "1.40", features = ["rt", "macros", "time", "sync"] }
```

`src/main.hrs`

```
use tokio.sync.mpsc
use tokio.time.(sleep, Duration)

#[tokio.main (flavor = "current_thread")]
async fn main$:
    let (tx, mut rx) = mpsc.unbounded_channel$
    let tx1 = tx <- clone$

    let tx1_fut = async move:
        for val in ["hi", "from", "the", "future"]:
            tx1 <- send (String.from val) <- unwrap$
            sleep (Duration.from_millis 5) <- await

    let rx_fut = async:
        while let Some value = rx <- recv$ <- await:
            println! "received '{value}'"

    let tx_fut = async move:
        for val in ["more", "messages", "for", "you"]:
            tx <- send (String.from val) <- unwrap$
            sleep (Duration.from_millis 5) <- await

    tokio.join! tx1_fut tx_fut rx_fut;
```

```text
$ hrs run
received 'hi'
received 'more'
received 'messages'
received 'from'
received 'the'
received 'for'
received 'future'
received 'you'
```

Three futures joined: two producers sending with delays and a receiver looping `while let Some value = rx <- recv$ <- await:`. The producers are `async move:` blocks — `move` so that each owns its transmitter — and the receiver ends when both transmitters have been dropped, which happens when the producer blocks finish. The messages interleave at the awaits, and because the runtime is single-threaded and the delays are equal, the order is fixed. With threads this program needed `Arc` for nothing and a `join` for each thread; here it is three blocks and one `join!`.

## 17.4 Racing and timeouts

Sometimes the first future to finish is the one you want:

`Cargo.toml`

```text
[dependencies]
tokio = { version = "1.40", features = ["rt", "macros", "time", "sync"] }
```

`src/main.hrs`

```
use tokio.time.(sleep, Duration)

async fn slow name: &str -> &str:
    sleep (Duration.from_millis 50) <- await
    name

async fn fast name: &str -> &str:
    sleep (Duration.from_millis 5) <- await
    name

#[tokio.main (flavor = "current_thread")]
async fn main$:
    let winner = tokio.select! do:        // a macro body written in Harsh\ arms, no commas
        n = slow "slow" => n
        n = fast "fast" => n
    println! "{winner} finished first"

    let result =
        tokio.time.timeout (Duration.from_millis 10) (slow "slow")
            <- await
    println! "{:?}" (result <- is_err$)
```

```text
$ hrs run
fast finished first
true
```

`tokio.select!` polls its arms and returns the first to complete, dropping the others — `fast` wins, and `slow` is cancelled mid-sleep. Its body is written like any Harsh block, `tokio.select! do:` with the arms beneath: a line with a `=>` in it is an arm and takes the comma Rust wants, and `slow "slow"` is a call as everywhere else. `tokio.time.timeout` is the common special case — a future and a limit, `Err` if the limit comes first — and here it does.

## 17.5 The one rule

An async program shares its threads among all its tasks, so a task that *blocks* — a CPU-bound loop, a `std.thread.sleep`, a synchronous file read — stalls every other task on that thread. The rule is: never block in async code. When the work is blocking, hand it to a thread:

`Cargo.toml`

```text
[dependencies]
tokio = { version = "1.40", features = ["rt", "macros", "time", "sync"] }
```

`src/main.hrs`

```
use std.time.Duration

#[tokio.main (flavor = "current_thread")]
async fn main$:
    // CPU work or a blocking call goes to a thread, so the runtime keeps turning.
    let sum = tokio.task.spawn_blocking ||:
        std.thread.sleep (Duration.from_millis 10)
        (1..=100u64) <- sum<u64>$

    let answer = sum <- await <- unwrap$
    println! "{answer}"
```

```text
$ hrs run
5050
```

`spawn_blocking` runs the closure on a thread from a pool kept for the purpose, and returns a future that resolves to the closure's result; the runtime keeps turning while the thread works. The closure is a trailing `||:` with its body beneath, as in chapter 13. Everything in the standard library that blocks — `std.fs`, `std.thread.sleep`, `Mutex` held across an await — has this shape in async code: either an async version from the runtime (`tokio.fs`, `tokio.time.sleep`, `tokio.sync.Mutex`) or a `spawn_blocking`.

## 17.6 What you have

`async fn` returns a future; `<- await` drives one and pauses the caller; a runtime — `#[tokio.main]` — drives `main`. `tokio.spawn async:` for an independent task, `async:` and `async move:` blocks as values, `tokio.join!` to run several to completion together, `tokio.select! do:` to take the first, `timeout` for a limit, async channels for messages between tasks, and `spawn_blocking` for anything that would block. Harsh lays out an `async:` block as it does a closure's body; everything else is Rust's.

Next: the object-oriented features Rust has and the ones it deliberately lacks — and trait objects, which is how a `Vec` holds values of different types.

# 18. Object-oriented features

Is Rust object-oriented? It has objects in the sense that matters — data with methods, encapsulated behind a public interface — and it has polymorphism, through traits. It does not have inheritance, on purpose, and this chapter is about what it offers instead: *trait objects*, which let a collection hold values of different types and call the same method on each, and the design consequences of choosing that over a class hierarchy.

## 18.1 Encapsulation

A struct with private fields and public methods is an object in the classic sense:

```
pub struct AveragedCollection
    list: Vec<i32>
    average: f64

impl AveragedCollection:
    pub fn new$ -> Self:
        Self\ list = vec! [], average = 0.0

    pub fn add (&mut self) (value: i32):
        self <- list <- push value
        self <- update_average$

    pub fn remove (&mut self) -> Option<i32>:
        let result = self <- list <- pop$

        match result:
            Some value => do:
                self <- update_average$
                Some value
            None => None

    pub fn average (&self) -> f64:
        self <- average

    fn update_average (&mut self):
        let total: i32 =
            self <- list
                 <- iter$
                 <- sum$
        self <- average = total as f64 / self <- list <- len$ as f64

fn main$:
    let mut c = AveragedCollection.new$
    c <- add 3
    c <- add 5
    println! "{}" (c <- average$)
    c <- remove$
    println! "{}" (c <- average$)
```

```text
4
3
```

`list` and `average` are private; `add`, `remove` and `average` are the interface; `update_average` is an implementation detail no caller can see. The cached average can never be stale, because every path that changes the list goes through a method that recomputes it — and the representation can change (a `HashSet`, say) without any caller noticing. This is chapter 7's privacy doing the job classes do elsewhere, and it needs no new feature.

## 18.2 Trait objects

Chapter 8 held several types in one `Vec` by wrapping them in an enum, which works when the set of types is known when you write the enum. A GUI library cannot know what components its users will define. It needs "a vector of anything that can draw itself":

```
pub trait Draw:
    fn draw (&self)

pub struct Screen
    pub components: Vec<Box<dyn Draw>>      // any type that implements Draw, boxed

impl Screen:
    pub fn run (&self):
        for component in self <- components <- iter$:
            component <- draw$

pub struct Button
    pub width: u32
    pub height: u32
    pub label: String

impl Draw for Button:
    fn draw (&self):
        println!
            "button {}x{} '{}'"
            (self <- width)
            (self <- height)
            (self <- label)

struct SelectBox
    width: u32
    height: u32
    options: Vec<String>

impl Draw for SelectBox:
    fn draw (&self):
        println!
            "select box {}x{} {:?}"
            (self <- width)
            (self <- height)
            (self <- options)

fn main$:
    let screen =
        Screen\
            components =
                vec! [
                    (Box.new (SelectBox\
                        width = 75
                        height = 10
                        options = vec! [String.from "Yes", String.from "Maybe", String.from "No"]
                     )),
                         (Box.new (Button\
                             width = 50
                             height = 10
                             label = String.from "OK"
                          )),
                          ]
    screen <- run$
```

```text
select box 75x10 ["Yes", "Maybe", "No"]
button 50x10 'OK'
```

`Vec<Box<dyn Draw>>` is that vector. `dyn Draw` is a *trait object*: some type, unknown at compile time, that implements `Draw`. It must sit behind a pointer — `Box`, `&`, `Rc` — because different types have different sizes and the vector's elements must all be the same size, and a pointer is. The `for` loop calls `component <- draw$` on each, and the right implementation is found *at run time* through a table of method pointers the trait object carries — *dynamic dispatch*, as against the compile-time dispatch of generics. `Button` is defined here and `SelectBox` could be defined by a user of the library; `Screen` needs to know neither.

The compiler still checks that every element can draw:

```
pub trait Draw:
    fn draw (&self)

pub struct Screen
    pub components: Vec<Box<dyn Draw>>

fn main$:
    let screen =
        Screen\
            components = vec! [Box.new (String.from "Hi")]
    let _ = screen
```

```text
error[E0277]: the trait bound `String: Draw` is not satisfied
  --> not_draw.hrs:10:32
   |
10 |             components = vec! [Box.new (String.from "Hi")]
   |                                ^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Draw` is not implemented for `String`
   = help: this trait has no implementations, consider adding one (hrs 1:1)
   = note: required for the cast from `Box<String>` to `Box<dyn Draw>`
```

A `String` does not implement `Draw`, so it cannot become a `Box<dyn Draw>`, and the error says so at the point of the cast. Duck typing with a type checker: anything goes in, provided it has the method, and "provided" is verified.

### Trait objects or generics

The same screen could be written with a type parameter:

```
pub trait Draw:
    fn draw (&self)

// Generic: one T for the whole screen, chosen at compile time, no boxing.
pub struct Screen<T: Draw>
    pub components: Vec<T>

impl<T> Screen<T> [where T: Draw]:
    pub fn run (&self):
        for component in self <- components <- iter$:
            component <- draw$

struct Label String

impl Draw for Label:
    fn draw (&self):
        println! "label '{}'" (self.0)

fn main$:
    let screen = Screen\ components = vec! [Label (String.from "a"), Label (String.from "b")]
    screen <- run$
```

```text
label 'a'
label 'b'
```

`Screen<T: Draw>` holds a `Vec<T>` — a vector of *one* type that implements `Draw`, chosen per screen at compile time, with no boxing and static dispatch. That is the faster and the more restrictive choice: a `Screen<Button>` cannot hold a `SelectBox`. Use generics when all the elements are the same type, which is the common case and the compiler will optimise it fully; use `dyn` when they are not, and pay the pointer and the indirect call, which is small. The choice is one word in the type and it is a real design decision, not a style one.

One limit: not every trait can be a trait object. The method signatures must not mention `Self` as a return type or have type parameters — the compiler needs to know the method's calling convention without knowing the concrete type. `Clone` is the usual casualty (`fn clone (&self) -> Self`); a trait like that is *not object safe* and the error says so when you try.

## 18.3 The state pattern

An object-oriented design pattern, done in Rust to see what fits and what does not. A blog post is a draft, then pending review, then published; its behaviour — what `content` returns, what `approve` does — depends on which. In the classic pattern each state is an object, and the post delegates to whichever it holds:

```
pub struct Post
    state: Option<Box<dyn State>>
    content: String

impl Post:
    pub fn new$ -> Post:
        Post\ state = Some (Box.new Draft), content = String.new$

    pub fn add_text (&mut self) (text: &str):
        self <- content <- push_str text

    pub fn content (&self) -> &str:
        self <- state
             <- as_ref$
             <- unwrap$
             <- content self

    pub fn request_review (&mut self):
        if let Some s = self <- state <- take$:
            self <- state = Some (s <- request_review$)

    pub fn approve (&mut self):
        if let Some s = self <- state <- take$:
            self <- state = Some (s <- approve$)

trait State:
    fn request_review (self: Box<Self>) -> Box<dyn State>
    fn approve (self: Box<Self>) -> Box<dyn State>

    fn content<'a> (&self) (_post: &'a Post) -> &'a str:
        ""

struct Draft

impl State for Draft:
    fn request_review (self: Box<Self>) -> Box<dyn State>:
        Box.new PendingReview

    fn approve (self: Box<Self>) -> Box<dyn State>:
        self

struct PendingReview

impl State for PendingReview:
    fn request_review (self: Box<Self>) -> Box<dyn State>:
        self

    fn approve (self: Box<Self>) -> Box<dyn State>:
        Box.new Published

struct Published

impl State for Published:
    fn request_review (self: Box<Self>) -> Box<dyn State>:
        self

    fn approve (self: Box<Self>) -> Box<dyn State>:
        self

    fn content<'a> (&self) (post: &'a Post) -> &'a str:
        &post <- content

fn main$:
    let mut post = Post.new$
    post <- add_text "I ate a salad for lunch today"
    println! "draft: '{}'" (post <- content$)
    post <- request_review$
    println! "pending: '{}'" (post <- content$)
    post <- approve$
    println! "published: '{}'" (post <- content$)
```

```text
draft: ''
pending: ''
published: 'I ate a salad for lunch today'
```

`Post` holds `Option<Box<dyn State>>`; `Draft`, `PendingReview` and `Published` are unit structs implementing `State`; `request_review` and `approve` each *consume* the current state (`self: Box<Self>` — a method that takes the box by value, which is how a state can hand back a different one) and return the next. `Post`'s methods use `take$` to move the state out of the `Option`, call the transition, and put the result back — the `Option` exists so that the state can be moved out of a `&mut self` without leaving a hole. `content` has a default that returns `""`, overridden only by `Published`. `Post` knows nothing about the transitions; adding a state means adding a type, not editing a `match`.

It works, and it has the pattern's usual costs: the states know about each other, a transition that is invalid for a state has to be written as "return `self`", and nothing stops a caller from asking a draft for its content. Rust offers a different shape — encode the states as *types*:

```
// The same workflow with the states as types: an invalid transition does not compile.
pub struct Post
    content: String

pub struct DraftPost
    content: String

pub struct PendingReviewPost
    content: String

impl Post:
    pub fn new$ -> DraftPost:
        DraftPost\ content = String.new$

    pub fn content (&self) -> &str:
        &self <- content

impl DraftPost:
    pub fn add_text (&mut self) (text: &str):
        self <- content <- push_str text

    pub fn request_review self -> PendingReviewPost:
        PendingReviewPost\ content = self <- content

impl PendingReviewPost:
    pub fn approve self -> Post:
        Post\ content = self <- content

fn main$:
    let mut post = Post.new$
    post <- add_text "I ate a salad for lunch today"

    let post = post <- request_review$     // a DraftPost has no `content` method to misuse
    let post = post <- approve$
    println! "{}" (post <- content$)
```

```text
I ate a salad for lunch today
```

Now `Post.new$` returns a `DraftPost`, which has `add_text` and `request_review` and *no* `content` method; `request_review` consumes it and returns a `PendingReviewPost`, which has only `approve`; and only a `Post` has `content`. Each transition is `let post = post <- …` — shadowing, chapter 3 — because each is a new type. An invalid transition is not a no-op or a runtime error; it does not compile, because the method does not exist on that type. The workflow is in the type signatures, where a reader finds it and the compiler enforces it.

That is the chapter's real lesson. Rust can express the object-oriented patterns, and sometimes they are right; but a pattern that exists to make invalid states unrepresentable at run time is often better expressed as types that make them unrepresentable at compile time. Chapter 9 said the same about `Guess`. Reach for the type first.

## 18.4 What you have

Structs with private fields and public methods are encapsulated objects. `Box<dyn Trait>` is a trait object — any type implementing the trait, dispatched at run time, sized by the pointer — for collections of mixed types, checked at the cast. Generics for one type chosen at compile time. `self: Box<Self>` for a method that consumes a boxed value and returns a replacement. And the state pattern, done twice: as trait objects, and as types that make the invalid transitions uncompilable.

Next: patterns — every place they appear and every form they take, since you have been using them since chapter 2.

# 19. Patterns and matching

You have been writing patterns since chapter 2: `Ok n`, `Some max`, `(key, value)`, `Point { x, y }`. A *pattern* is a shape that a value is tested against and, when it fits, taken apart into names. This chapter collects every place a pattern can appear and every form one can take, so that the next time a value has a shape you can write the shape directly instead of reaching for it with methods. Harsh's contribution is the one you know: a variant with fields is matched by juxtaposition, `Coin.Quarter state`, the way it is constructed.

## 19.1 Where patterns appear

```
fn main$:
    // match arms
    let x = Some 3

    match x:
        None => println! "none"
        Some i => println! "some {i}"
    // if let, with else if and else if let

    let favorite_color: Option<&str> = None
    let is_tuesday = false
    let age: Result<u8, _> = "34" <- parse$

    if let Some color = favorite_color:
        println! "Using your favorite color, {color}, as the background"
    else if is_tuesday:
        println! "Tuesday is green day!"
    else if let Ok age = age:
        if age > 30: println! "Using purple as the background color" else: println! "Using orange"
    else:
        println! "Using blue as the background color"
    // while let

    let mut stack = vec! [1, 2, 3]

    while let Some top = stack <- pop$:
        println! "{top}"
    // for

    let v = vec! ['a', 'b', 'c']

    for (index, value) in v <- iter$ <- enumerate$:
        println! "{value} is at index {index}"
    // let

    let (a, b, c) = (1, 2, 3)
    println! "{a} {b} {c}"

    // function parameters
    fn print_coordinates (&(x, y): &(i32, i32)):
        println! "Current location: ({x}, {y})"

    print_coordinates (&(3, 5))
```

```text
some 3
Using purple as the background color
3
2
1
a is at index 0
b is at index 1
c is at index 2
1 2 3
Current location: (3, 5)
```

Six places. `match` arms, where every case must be covered. `if let`, for one case, with `else if` and `else if let` chaining conditions and patterns freely — the example mixes a plain boolean between two pattern tests. `while let`, looping while a pattern keeps matching — popping a stack until it is empty. `for`, whose loop variable is a pattern: `(index, value)` takes apart the pairs `enumerate$` yields. `let`, whose left side is a pattern: `let (a, b, c) = …` is three bindings at once, and the plain `let x = 5` you have written a thousand times is a pattern too, one that matches anything. And function parameters, which are patterns as well: `&(x, y): &(i32, i32)` takes a reference to a tuple apart in the signature. The nested `fn` is legal Rust — an item inside a function, visible only there.

## 19.2 Refutability

A pattern that can fail to match is *refutable*; one that always matches is *irrefutable*. `Some x` is refutable, `x` and `(a, b)` are not. `let`, `for` and function parameters need an irrefutable pattern, because they have nothing to do when it fails; `if let`, `while let` and `match` arms accept a refutable one, because failing is what their `else`, their end, and their next arm are for:

```
fn main$:
    let some_option_value: Option<i32> = None
    let Some x = some_option_value
    println! "{x}"
```

```text
error[E0005]: refutable pattern in local binding
  --> refutable.hrs:3:9
   |
 3 |     let Some x = some_option_value
   |         ^^^^^^^^ pattern `None` not covered
   = note: `let` bindings require an "irrefutable pattern", like a `struct` or an `enum` with only one variant
   = note: for more information, visit https://doc.rust-lang.org/book/ch18-02-refutability.html
   = note: the matched value is of type `Option<i32>`
   = help: you might want to use `let else` to handle the variant that isn't matched (hrs 3:35)
```

`let Some x = value` has nowhere to go when `value` is `None`, and the help offers the fix from chapter 6, `let … else`. The other direction is a warning rather than an error: `if let x = 5` is legal and pointless, and the compiler says so.

## 19.3 The forms

### Literals, names, ranges, alternatives

```
fn main$:
    let x = 1

    match x:
        1 => println! "one"
        2 => println! "two"
        3 => println! "three"
        _ => println! "anything"
    // named variables shadow inside an arm

    let x = Some 5
    let y = 10

    match x:
        Some 50 => println! "Got 50"
        Some y => println! "Matched, y = {y}"     // a new y, bound to 5
        _ => println! "Default case, x = {x:?}"

    println! "at the end: x = {x:?}, y = {y}"

    // or-patterns and ranges
    let x = 5

    match x:
        1 | 2 => println! "one or two"
        3..=5 => println! "three through five"
        _ => println! "anything"

    let c = 'c'

    match c:
        'a'..='j' => println! "early ASCII letter"
        'k'..='z' => println! "late ASCII letter"
        _ => println! "something else"
```

```text
one
Matched, y = 5
at the end: x = Some(5), y = 10
three through five
early ASCII letter
```

A literal matches itself. A name matches anything and binds it — and inside an arm it is a *new* variable that shadows any outer one, which is the trap in the second `match`: `Some y` does not compare against the outer `y`, it binds a fresh `y` to `5`, and the message says so. (The guard in 19.4 is how to compare against an outer variable.) `|` gives alternatives; `..=` matches an inclusive range of numbers or characters — ranges are the one pattern form that is not just a literal or a structure, and they are allowed only for those two types, where the compiler can check that a set of ranges covers everything.

### Destructuring

```
struct Point
    x: i32
    y: i32

enum Color
    Rgb i32 i32 i32
    Hsv i32 i32 i32

enum Message
    Quit

    Move\
        x: i32
        y: i32

    Write String
    ChangeColor Color

fn main$:
    let p = Point\ x = 0, y = 7
    let Point\ x: a, y: b = p           // fields into new names
    println! "{a} {b}"

    let Point\ x, y = p                 // shorthand: same names
    println! "{x} {y}"

    match p:
        Point\ x, y: 0 => println! "On the x axis at {x}"
        Point\ x: 0, y => println! "On the y axis at {y}"
        Point\ x, y => println! "On neither axis: ({x}, {y})"

    let msgs =
        [
            Message.Quit,
            (Message.Move\ x = 1, y = 2),
            (Message.Write (String.from "hi")),
            (Message.ChangeColor (Color.Hsv 0 160 255)),
        ]

    for msg in msgs:
        match msg:
            Message.Quit => println! "The Quit variant has no data to destructure."
            Message.Move\ x, y => println! "Move in the x direction {x} and in the y direction {y}"
            Message.Write text => println! "Text message: {text}"
            Message.ChangeColor (Color.Rgb r g b) => println! "Change color to red {r}, green {g}, blue {b}"
            Message.ChangeColor (Color.Hsv h s v) => println! "Change color to hue {h}, saturation {s}, value {v}"

    // nested, all at once

    let ((feet, inches), Point\ x, y) = ((3, 10), Point\ x = 3, y = -10)
    println! "{feet} {inches} {x} {y}"
```

```text
0 7
0 7
On the y axis at 7
The Quit variant has no data to destructure.
Move in the x direction 1 and in the y direction 2
Text message: hi
Change color to hue 0, saturation 160, value 255
3 10 3 -10
```

A struct pattern is a field list like any other, marked with `\`: `Point\ x: a, y: b` binds the fields to new names — a rename keeps the colon, since here the field is not being given a value but matched — `Point\ x, y` is the shorthand for the same names, and a field may hold a literal to test it: `Point\ x, y: 0` matches only points on the x axis and binds `x`. Enum variants are matched by the shape that built them: `Message.Quit` bare, `Message.Move { x, y }` with the record's fields, `Message.Write text` with one juxtaposed name, and `Message.ChangeColor (Color.Hsv h s v)` with the inner variant's pattern *isolated in parentheses*, because it is one argument and it has structure. The isolating parentheses are Harsh's argument rule, applied to a pattern — the same rule as `Some (i + 1)` on the constructing side. And patterns nest to any depth: the last `let` takes a tuple of a tuple and a struct apart in one line.

### Ignoring

```
fn foo (_: i32) (y: i32):                // an unused parameter, by name
    println! "This code only uses the y parameter: {y}"

fn main$:
    foo 3 4

    // `_` in a nested position
    let mut setting_value = Some 5
    let new_setting_value = Some 10

    match (setting_value, new_setting_value):
        (Some _, Some _) => println! "Can't overwrite an existing customized value"
        _ => setting_value = new_setting_value

    println! "setting is {setting_value:?}"

    let numbers = (2, 4, 8, 16, 32)

    match numbers:
        (first, _, third, _, fifth) => println! "Some numbers: {first}, {third}, {fifth}"
    // `_x` binds and silences the warning; `_` does not bind at all

    let s = Some (String.from "Hello!")

    if let Some _ = s:                     // `Some _s` here would move the String out
        println! "found a string"

    println! "{s:?}"

    // `..` for the rest
    struct Point
        x: i32
        y: i32
        z: i32

    let origin = Point\ x = 0, y = 0, z = 0

    match origin:
        Point\ x, .. => println! "x is {x}"

    match numbers:
        (first, .., last) => println! "Some numbers: {first}, {last}"
```

```text
This code only uses the y parameter: 4
Can't overwrite an existing customized value
setting is Some(5)
Some numbers: 2, 8, 32
found a string
Some("Hello!")
x is 0
Some numbers: 2, 32
```

`_` matches anything and binds nothing. In a parameter it is a value the function ignores (useful when a trait signature requires it); nested, it tests a shape without taking it apart — `(Some _, Some _)` is "both set" without caring what to; in a tuple it skips positions. `_` and a name starting with `_` differ in one way that matters: `_x` *binds* (and only silences the unused-variable warning), so `if let Some _s = s` would move the `String` out of `s`, while `if let Some _ = s` does not, and `s` is still printable after. `..` ignores *all remaining* parts: `Point { x, .. }` for a struct, `(first, .., last)` for a tuple, and it must be unambiguous — `(.., second, ..)` is an error, since the compiler cannot tell which position `second` means.

## 19.4 Guards and bindings

```
enum Message
    Hello\
        id: i32

fn main$:
    // a match guard: an extra condition after the pattern
    let num = Some 4

    match num:
        Some x if x % 2 == 0 => println! "The number {x} is even"
        Some x => println! "The number {x} is odd"
        None => ()
    // a guard solves the shadowing problem from the literals example

    let x = Some 5
    let y = 10

    match x:
        Some 50 => println! "Got 50"
        Some n if n == y => println! "Matched, n = {n}"
        _ => println! "Default case, x = {x:?}"
    // a guard applies to the whole or-pattern

    let x = 4
    let y = false

    match x:
        4 | 5 | 6 if y => println! "yes"
        _ => println! "no"
    // `@` binds a value while also testing it against a range

    let msg = Message.Hello\ id = 5

    match msg:
        Message.Hello { id: id_variable @ 3..=7 } => println! "Found an id in range: {id_variable}"
        Message.Hello { id: 10..=12 } => println! "Found an id in another range"
        Message.Hello\ id => println! "Found some other id: {id}"
```

```text
The number 4 is even
Default case, x = Some(5)
no
Found an id in range: 5
```

A *match guard* is an `if` after the pattern: the arm matches only if the pattern fits *and* the condition holds. `Some x if x % 2 == 0` tests the bound value; `Some n if n == y` compares against the outer `y` — the answer to the shadowing trap, since a guard is an expression and sees the enclosing scope. A guard applies to the whole of an or-pattern, `4 | 5 | 6 if y`, not just the last alternative. Guards are not counted for exhaustiveness — the compiler cannot see through an arbitrary condition — so a `match` whose arms all have guards still needs a catch-all.

`@` binds a name to a value *while* testing it: `id_variable @ 3..=7` matches ids from 3 to 7 and gives the arm the actual id, where `3..=7` alone would test without binding and `id` alone would bind without testing. It is the form for "I want to know it is in this range, and I want the value".

## 19.5 What you have

Patterns appear in `match`, `if let`, `while let`, `for`, `let` and parameters; `let`, `for` and parameters need irrefutable ones. The forms: literals, names (which shadow), `|`, `..=` ranges, tuple and struct and variant destructuring to any depth, `_` and `_name` and `..` to ignore, `if` guards, and `x @ pattern` to bind and test. Harsh writes a variant's payload by juxtaposition and isolates a nested pattern in parentheses, exactly as it writes the constructing expression.

Next: the advanced features — `unsafe`, the corners of traits and types, function pointers and returned closures, and macros — which most programs never need and every Rust programmer eventually meets.

# 20. Advanced features

Everything in this chapter is something a Rust program can go a long way without. Each is here because sooner or later you meet it in a library's source or an error message, and because knowing the edges of the language is part of knowing the language. In order: `unsafe`, the parts of the trait system chapter 10 skipped, the corners of the type system, functions as values, and macros.

## 20.1 Unsafe Rust

Every guarantee so far — no dangling references, no data races, no out-of-bounds reads — is enforced by the compiler *refusing programs it cannot prove safe*. Some correct programs cannot be proved safe: talking to the operating system, implementing a data structure with raw pointers, calling C. For those, `unsafe` marks a region where the programmer takes over the proof:

```
fn main$:
    let mut num = 5
    let r1 = &num as *const i32          // raw pointers can be made in safe code
    let r2 = &mut num as *mut i32

    unsafe:                              // ...but only dereferenced in an unsafe block
        println! "r1 is: {}" (*r1)
        *r2 = 6
        println! "r2 is: {}" (*r2)
```

```text
r1 is: 5
r2 is: 6
```

A *raw pointer* — `*const T` or `*mut T` — may be created anywhere; it is *dereferencing* one that is unsafe, since nothing guarantees it points at a live value, so that happens inside `unsafe:`. Five things need the keyword: dereferencing a raw pointer, calling an unsafe function, accessing a mutable static, implementing an unsafe trait, and accessing a union's fields. Everything else — the borrow checker, the type checker — stays on inside the block. `unsafe` does not turn Rust off; it turns off five checks, and marks where.

```
use std.slice

unsafe fn dangerous$:
    println! "this function is unsafe to call"

// A safe function wrapping unsafe code: the contract is checked, then trusted.

fn split_at_mut (values: &mut [i32]) (mid: usize) -> (&mut [i32], &mut [i32]):
    let len = values <- len$
    let ptr = values <- as_mut_ptr$
    assert! (mid <= len)

    unsafe:
        (slice.from_raw_parts_mut ptr mid, slice.from_raw_parts_mut
                                               (ptr <- add mid)
                                               (len - mid))

fn main$:
    unsafe:
        dangerous$

    let mut v = vec! [1, 2, 3, 4, 5, 6]
    let (a, b) = split_at_mut (&mut v) 3
    a[0] = 10
    b[0] = 40
    println! "{v:?}"
```

```text
this function is unsafe to call
[10, 2, 3, 40, 5, 6]
```

`unsafe fn dangerous$` is a function whose *caller* must uphold some contract, and may only be called from an `unsafe:` block. `split_at_mut` is the pattern that matters: a *safe* function that uses unsafe code inside, after checking the contract itself. Two mutable slices into one vector cannot be expressed to the borrow checker, so the function takes a raw pointer, asserts that `mid` is in range, and builds the slices from raw parts inside `unsafe:` — and its callers never see the keyword, because the function has done the reasoning and stands behind it. This is how the standard library is written, and the discipline to copy: keep `unsafe` small, wrap it in a safe interface, and write down the invariant it depends on.

```
extern "C":                             // a foreign block: a layout block like any other
    fn abs (input: i32) -> i32           // a foreign function has no body; the `;` is supplied

static mut COUNTER: u32 = 0             // a mutable global: reading or writing it is unsafe

fn add_to_count inc: u32:
    unsafe:
        COUNTER += inc

fn main$:
    unsafe:
        println! "Absolute value of -3 according to C: {}" (abs (-3))

    add_to_count 3

    unsafe:
        println! "COUNTER: {COUNTER}"
```

```text
Absolute value of -3 according to C: 3
COUNTER: 3
```

`extern "C":` declares functions from another language — a layout block like any other, each foreign signature on its own line with no body — and calling one is unsafe, since Rust cannot check what C does. A `static mut` is a mutable global, and every access to one is unsafe, because two threads could race on it; a `static` without `mut` is safe, and is the usual form. (The `(-3)` is isolated: a negative literal is an operator expression.)

## 20.2 Advanced traits

### Associated types

Chapter 13 showed `Iterator`'s `type Item`. Here is a type implementing it:

```
struct Counter
    count: u32

impl Iterator for Counter:
    type Item = u32                       // the associated type: what `next` yields

    fn next (&mut self) -> Option<Self.Item>:
        if self <- count < 5:
            self <- count += 1
            Some (self <- count)

        else:
            None

fn main$:
    let sum: u32 =
        (Counter\ count = 0)
            <- zip ((Counter\ count = 0) <- skip 1)
            <- map (|(a, b)| a * b)
            <- filter (|x| x % 3 == 0)
            <- sum$

    println! "{sum}"
```

```text
18
```

`type Item = u32` inside the `impl` fixes what this iterator yields, and `next` returns `Option<Self.Item>`. An associated type is like a type parameter that the *implementation* chooses once, rather than the caller choosing at each use: `Counter` is an iterator of `u32` and nothing else, and callers never write `Iterator<u32>`. That is the difference from generics, and the reason `Iterator` uses one — a type is an iterator over one thing. Having implemented `next`, `Counter` gets every adaptor for free, and the chain in `main` (`zip`, `map`, `filter`, `sum$`) is the proof.

### Operator overloading

`+` is a trait, `Add`, and a type implements it to be addable:

```
use std.ops.Add

#[derive Debug Copy Clone PartialEq]
struct Point
    x: i32
    y: i32

impl Add for Point:
    type Output = Point

    fn add (self) (other: Point) -> Point:
        Point\ x = self <- x + other <- x, y = self <- y + other <- y

struct Millimeters u32
struct Meters u32

// The default type parameter, `Rhs = Self`, overridden: add a different type.
impl Add<Meters> for Millimeters:
    type Output = Millimeters

    fn add (self) (other: Meters) -> Millimeters:
        Millimeters (self.0 + (other.0 * 1000))

fn main$:
    println! "{:?}" ((Point\ x = 1, y = 0) + (Point\ x = 2, y = 3))

    let Millimeters total = Millimeters 500 + Meters 2
    println! "{total}"
```

```text
Point { x: 3, y: 3 }
2500
```

`impl Add for Point` supplies `add` and `type Output`, and `Point + Point` calls it. `Add<Rhs = Self>` has a *default type parameter* — the right-hand side is the same type unless you say otherwise — and `impl Add<Meters> for Millimeters` says otherwise: a millimetre value plus a metre value. All the operators are traits in `std.ops`, and this is the whole mechanism.

### Same name, several traits

Two traits may define a method with the same name, and a type may implement both, and have an inherent method of that name as well:

```
trait Pilot:
    fn fly (&self)

trait Wizard:
    fn fly (&self)

struct Human

impl Pilot for Human:
    fn fly (&self):
        println! "This is your captain speaking."

impl Wizard for Human:
    fn fly (&self):
        println! "Up!"

impl Human:
    fn fly (&self):
        println! "*waving arms furiously*"

trait Animal:
    fn baby_name$ -> String

struct Dog

impl Dog:
    fn baby_name$ -> String:
        String.from "Spot"

impl Animal for Dog:
    fn baby_name$ -> String:
        String.from "puppy"

fn main$:
    let person = Human
    person <- fly$                       // the inherent method wins
    Pilot.fly (&person)                  // a trait's method, by its path
    Wizard.fly (&person)
    println! "A baby dog is called a {}" (Dog.baby_name$)
    println! "A baby dog is called a {}" (<Dog as Animal>.baby_name$)   // fully qualified
```

```text
*waving arms furiously*
This is your captain speaking.
Up!
A baby dog is called a Spot
A baby dog is called a puppy
```

`person <- fly$` calls the inherent method — the type's own wins. A trait's version is called through the trait's path with the receiver as the first argument, `Pilot.fly (&person)`. When the method has no `self` — `baby_name$` is an associated function — there is no receiver to go by, and the *fully qualified* form names both the type and the trait: `<Dog as Animal>.baby_name$`, "Dog's implementation of Animal's baby_name". The angle brackets are Rust's and pass through; the `.` is Harsh's path dot. You will write this rarely and read it in error messages often.

### Supertraits and the newtype pattern

A trait may require another:

```
use std.fmt

// A supertrait: OutlinePrint requires Display, and may use it.
trait OutlinePrint: fmt.Display:
    fn outline_print (&self):
        let output = self <- to_string$
        let len = output <- len$
        println! "{}" ("*" <- repeat (len + 4))
        println! "*{}*" (" " <- repeat (len + 2))
        println! "* {output} *"
        println! "*{}*" (" " <- repeat (len + 2))
        println! "{}" ("*" <- repeat (len + 4))

struct Point
    x: i32
    y: i32

impl fmt.Display for Point:
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        write! f "({}, {})" (self <- x) (self <- y)

impl OutlinePrint for Point {}
// The newtype pattern: a local wrapper lets us implement a foreign trait on a foreign type.
struct Wrapper (Vec<String>)

impl fmt.Display for Wrapper:
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        write! f "[{}]" (self.0 <- join ", ")

fn main$:
    (Point\ x = 1, y = 3) <- outline_print$

    let w = Wrapper (vec! [String.from "hello", String.from "world"])
    println! "w = {w}"
```

```text
**********
*        *
* (1, 3) *
*        *
**********
w = [hello, world]
```

`trait OutlinePrint: fmt.Display:` — the first `:` declares the *supertrait*, the second opens the block. Any type implementing `OutlinePrint` must implement `Display`, and `outline_print`'s default may therefore call `to_string$`. `Point` implements `Display` and then `OutlinePrint` with an empty `impl … {}` — Rust's braces for an empty body, since there is nothing to lay out.

The second half is the *newtype* pattern, the answer to chapter 10's orphan rule. `Display` and `Vec` are both foreign, so `impl Display for Vec<String>` is not allowed; wrap the vector in a local tuple struct, `Wrapper (Vec<String>)`, and implement `Display` for that. The wrapper costs nothing at run time and `self.0` reaches the vector. It is also the way to give a value a distinct type — `Millimeters` and `Meters` above — so the compiler keeps units apart that are the same `u32` underneath.

## 20.3 Advanced types

```
use std.fmt

// A type alias: a synonym, not a new type.
type Kilometers = i32
type Thunk = Box<dyn Fn$ + Send + 'static>

fn takes_long_type f: Thunk:
    f$

// The never type: `!` for something that does not return.
fn bar$ -> !:
    panic! "never returns"

fn main$:
    let x: i32 = 5
    let y: Kilometers = 5
    println! "x + y = {}" (x + y)              // the same type, so they add

    let f: Thunk = Box.new (|| println! "hi")
    takes_long_type f

    // `continue`, `panic!` and `loop` have type `!`, so a match arm may use them
    // where a value of any type is expected:
    let guess = "3"

    let n: u32 = match guess <- trim$ <- parse$:
        Ok num => num
        Err _ => bar$

    println! "{n}"

    // Dynamically sized types must sit behind a pointer: `str` is one, `&str` is
    // pointer plus length. A generic `T` is implicitly `T: Sized`; relax it with `?Sized`:
    fn generic<T: ?Sized + fmt.Debug> t: &T:
        println! "{t:?}"

    generic "a str, unsized"
```

```text
x + y = 10
hi
3
"a str, unsized"
```

A *type alias* is a name for an existing type, not a new one: `Kilometers` *is* `i32`, so the two add, and the alias buys nothing but readability — which is exactly what `Thunk` buys for a long trait-object type written many times. The *never type* `!` is the type of an expression that does not return: `panic!`, `continue`, `loop` without `break`, `process.exit`. It is why a `match` arm can be `Err _ => bar$` next to `Ok num => num` — `!` coerces to any type, so the arms agree — and why `continue` worked in chapter 2's guessing game.

A *dynamically sized type* is one whose size is not known at compile time: `str` (not `&str`), `[T]`, `dyn Trait`. They can only be used behind a pointer that carries the size — `&str` is a pointer and a length, `Box<dyn Trait>` a pointer and a vtable. Every generic `T` is implicitly `T: Sized`; `T: ?Sized` relaxes that, and then `T` must be used through a reference, as `generic` does.

## 20.4 Functions and closures as values

```
fn add_one x: i32 -> i32:
    x + 1

// `fn i32 -> i32` is a function pointer type: takes any fn or non-capturing closure.
fn do_twice (f: fn i32 -> i32) (arg: i32) -> i32:
    f arg + f arg

// Returning a closure: an opaque `impl Fn`, or a boxed trait object when the
// concrete type varies.
fn returns_closure$ -> impl Fn i32 -> i32:
    |x| x + 1

fn returns_initialized_closure init: i32 -> Box<dyn Fn i32 -> i32>:
    if init > 0: Box.new (move |x| x + init) else: Box.new (move |x| x - init)

#[derive Debug]
enum Status
    Value u32
    Stop

fn main$:
    println! "{}" (do_twice add_one 5)

    // A tuple-struct or variant constructor is a function too: pass it where a closure is wanted.
    let strings: Vec<String> =
        [1, 2, 3] <- iter$
                  <- map (ToString.to_string)
                  <- collect$
    println! "{strings:?}"

    let statuses: Vec<Status> =
        (0u32..3) <- map Status.Value <- collect$
    println! "{statuses:?} {:?}" Status.Stop

    let handlers = vec! [returns_closure$, returns_closure$]

    for h in handlers:
        println! "{}" (h 1)

    let f = returns_initialized_closure 10
    println! "{}" (f 1)
```

```text
12
["1", "2", "3"]
[Value(0), Value(1), Value(2)] Stop
2
2
11
```

`fn(i32) -> i32` is a *function pointer* type, and `do_twice` takes one: any named function, or a closure that captures nothing. Prefer the `impl Fn` bounds of chapter 13 in your own signatures, since they accept capturing closures too; `fn` is for interfacing with code that needs a plain pointer, C among it. The two `map` calls show a useful fact: every tuple-struct or enum-variant constructor *is* a function, so `map Status.Value` builds a `Status` from each number, and `map (ToString.to_string)` applies a trait method by its path.

Returning a closure needs a type for it, and a closure's type has no name. `impl Fn(i32) -> i32` in return position is the usual answer: the caller gets *some* closure. When different calls return different closures — the `if` returns one of two, with different captures — they are different types and `impl Fn` cannot name both; box them as `Box<dyn Fn(i32) -> i32>`, a trait object, and the type is the same either way. Chapter 18's rule, applied to closures.

## 20.5 Macros

A macro is code that writes code at compile time. `println!`, `vec!` and `#[derive]` are macros; the `!` and the `#[…]` are how you tell. A *declarative* macro is defined with `macro_rules!` as a set of patterns and what each expands to:

```
// A declarative macro: pattern-matched at compile time. Both sides are
// written in Harsh. The matcher is a parameter list -- one group per fragment,
// a repetition of groups for a list -- and the transcriber is a `do:` block.
#[macro_export]
macro_rules! my_vec:
    ( $( ($x:expr) )* ) => do:
        do:
            let mut temp_vec = Vec.new$
            $(temp_vec <- push $x)*
            temp_vec

// Two arms, and a repetition that spans lines. The inner `do:` makes the
// expansion a block with a value, as the Rust `{ { .. } }` would.
macro_rules! sum:
    () => do: 0
    ($h:expr) => do: $h
    ( ($h:expr) $( ($t:expr) )* ) => do:
        $h + sum! $( ($t) )*

fn main$:
    let v: Vec<u32> = my_vec! 1 2 3
    println! "{v:?}"
    println! "{}" (sum! 1 2 3 4)
```

```text
[1, 2, 3]
10
```

Both sides of the macro are written in Harsh, and both follow rules you already know. The matcher `( $( ($x:expr) )* )` is a parameter list: one group per fragment, and a repetition of groups is a list of them — the same brick as `fn f (a: T) (b: U)`, applied a third time. The call `my_vec! 1 2 3` is an ordinary application, one atom per argument. The transcriber is a `do:` block; its statements take their `;` from the layout, `$x` is an atom like any other so `push $x` is a call, and a repetition `$( … )*` that is the whole of its line repeats a statement. The inner `do:` is there because a macro's expansion is a run of tokens, not a block: for the expansion to *be* a block with a value, the block must be written, as Rust writes `{ { … } }`. `sum!` shows an arm with nothing to match and a repetition that spans lines, `$(` on one line, its body beneath, `)*` back under it. On the call side, `$( ($t) )*` in an argument position is a list of arguments, the mirror of the matcher; a repetition of bare tokens, `$($arg)*`, is copied as it stands and is how a macro forwards `tt`s.

> **Harsh —** A matcher written in brackets or braces rather than parentheses — `[ $elem:expr ; $n:expr ]` — is left exactly as written, because that is the shape the call side has too: `vec! [0u8; 4]`. The same licence covers a transcriber whose output is a language of its own rather than Harsh.

A macro whose body is markup — the `view!` of a web framework — is written the same way, with one rule of its own: when the first line of the `do:` block begins with `<`, the lines are markup and are copied through, and the contents of every `{ … }` are Harsh:

```
// A stand-in for a UI framework's `view!`: it swallows the markup and yields
// unit, so this file compiles without a dependency. Only the spelling of the
// call is the point here.
macro_rules! view:
    ( $($t:tt)* ) => do: ()

fn main$:
    let count = 3
    let items = vec! [1, 2, 3]
    view! do:
        <div class="app">
            <p>{count}</p>
            <button on:click={move |_| println! "{}" (count + 1)}>"+"</button>
            <ul>
                <For each={move || items <- clone$} let:item>
                    <li>{item * 2}</li>
                </For>
            </ul>
        </div>
    println! "rendered {count}"
```

```text
rendered 3
```

A hole may hold a whole expression, `{move |_| …}`, and it may span lines: `{move |_|:` with the closure's body beneath and `}` closing it. The framework's own rules for markup are unchanged, because Harsh never reads it; only the holes are its business.

Dioxus writes its interface not as markup but as a tree of braces — an element is a name and a brace body holding its attributes and children — and a tree of braces is what Harsh's layout is. So it is written as one. When the first line of a macro's `do:` block is a name followed by `:`, the block is a brace tree: `div:` opens an element, an attribute takes its value through `=` (since `:` opens blocks here) and is emitted `class: "app",`, a string or a `{ … }` is a child, and `for` and `if` are the framework's own, their bodies trees again. As in markup, the Harsh is in the holes:

```
// A stand-in for Dioxus's `rsx!`, as `view!` above: it swallows the tree and
// yields unit.
macro_rules! rsx:
    ( $($t:tt)* ) => do: ()

fn main$:
    let count = 3
    let items = vec! [1, 2, 3]
    rsx! do:
        div:
            class = "app"
            onclick = {move |_| println! "{}" (count + 1)}
            "Hello {count}"
            for item in items:
                li: "{item}"
            Button\
                onclick = {move |_|:
                    let n = count * 2
                    println! "{n}"
                }
                "Reset"
    println! "rendered {count}"
```

```text
rendered 3
```

What the tree becomes follows what `rsx!` itself accepts, read from its source: attributes are separated by commas and elements and text nodes are not, and an event handler is recognised by its leading `move` or `|` — which is why an attribute's hole loses its braces on the way out while a child's keeps them.

The other kind, *procedural* macros — `#[derive Debug]`, attribute macros, function-like macros that parse arbitrary tokens — are functions that take a token stream and return one, and live in a crate of their own with `proc-macro = true` in its `Cargo.toml`. Writing one is a project rather than a page: the `syn` crate parses the tokens into a syntax tree, `quote` turns a template back into tokens, and the function in between is ordinary Rust. When you need one, those two crates and their examples are the place to start.

### Parentheses, once and for all

Every use of parentheses in this book has been one of a short list, and the list is worth stating now that all of it has been seen:

```
fn double x: i32 -> i32:
    x * 2

fn main$:
    let a = (1 + 2) * 3          // precedence: needed, and kept
    let b = (1 + 2)              // grouping: optional, and harmless
    let c = (double 4)           // the same: a whole value in parens is just the value
    let t = (1, 2)               // a tuple: the comma is what makes it one
    let u = ()                   // the unit value
    let d = double (a + b)       // isolating one argument
    println! "{a} {b} {c} {t:?} {u:?} {d}"
```

```text
9 3 8 (1, 2) () 24
```

> **Harsh —** Parentheses do three things. They **make a tuple** — the comma does it, and `()` with nothing inside is the unit value. They **set precedence** — `(1 + 2) * 3` — which is mandatory and kept. And they **group** — around a whole value, a whole statement, or one argument of an application — which is optional, costs nothing, and is how an argument that is more than one token is marked as one: `double (a + b)`. Nothing else: parentheses never pass an argument list, and a name written tight against a `(` is an error naming the space. Inside a macro's one-line braces the same rules apply, since a macro's body is Harsh — the one earlier exemption for brace bodies was withdrawn when it proved to be a second syntax in disguise.

## 20.6 What you have

`unsafe:` for five operations the compiler cannot check, wrapped in safe functions that do the checking; `extern "C"` for foreign functions. Associated types for a trait with one choice per implementation; operator traits in `std.ops` with default type parameters; disambiguation by trait path and `<Type as Trait>`; supertraits; the newtype pattern for the orphan rule and for distinct types. Aliases, `!`, and `?Sized`. `fn` pointers, constructors as functions, `impl Fn` and `Box<dyn Fn>` for returned closures. `macro_rules!` written in Harsh on both sides, markup and brace-tree macros with Harsh holes, and where procedural macros come from.

Next, and last: a multithreaded web server, built from the standard library alone — the book's closing project.

# 21. Final project: a multithreaded web server

The last chapter builds a web server from the standard library alone: a TCP listener, HTTP by hand, a thread pool, and a graceful shutdown. It uses nearly everything in the book — closures, trait objects, channels, `Arc<Mutex<T>>`, `Drop`, `Option.take` — and is the kind of program that in most languages would be a framework. Each stage here is a complete project that runs to completion: the server spawns its own clients, serves a fixed number of requests, and stops, so that the book can show the run. Point a browser at the listening address instead and it is a real server.

## 21.1 A single-threaded server

`404.html`

```text
<!DOCTYPE html>
<html lang="en">
  <head><meta charset="utf-8"><title>Hello!</title></head>
  <body><h1>Oops!</h1><p>Sorry, I don't know what you're asking for.</p></body>
</html>
```

`hello.html`

```text
<!DOCTYPE html>
<html lang="en">
  <head><meta charset="utf-8"><title>Hello!</title></head>
  <body><h1>Hello!</h1><p>Hi from Rust</p></body>
</html>
```

`src/main.hrs`

```
use std.fs
use std.io.(prelude.*, BufReader)
use std.net.(TcpListener, TcpStream)
use std.thread
use std.time.Duration

fn main$:
    let listener = TcpListener.bind "127.0.0.1:0" <- unwrap$     // port 0: any free port
    let addr = listener <- local_addr$ <- unwrap$

    // A client, so the program drives itself: three requests, one after another.
    let client = thread.spawn move ||:
        for path in ["/", "/sleep", "/nope"]:
            let mut stream = TcpStream.connect addr <- unwrap$
            write! stream "GET {path} HTTP/1.1\r\n\r\n" <- unwrap$

            let status =
                BufReader.new (&stream)
                    <- lines$
                    <- next$
                    <- unwrap$
                    <- unwrap$
            println! "client: {path} -> {status}"

    // The server: one connection at a time, three of them, then done.

    for stream in listener <- incoming$ <- take 3:
        let stream = stream <- unwrap$
        handle_connection stream

    client <- join$ <- unwrap$

fn handle_connection mut stream: TcpStream:
    let buf_reader = BufReader.new (&stream)
    let request_line =
        buf_reader <- lines$
                   <- next$
                   <- unwrap$
                   <- unwrap$

    let (status_line, filename) =
        match &request_line[..]:
            "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html")
            "GET /sleep HTTP/1.1" => do:
                thread.sleep (Duration.from_millis 50)          // a slow request
                ("HTTP/1.1 200 OK", "hello.html")
            _ => ("HTTP/1.1 404 NOT FOUND", "404.html")

    let contents = fs.read_to_string filename <- unwrap$
    let length = contents <- len$
    let response = format! "{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}"
    stream <- write_all (response <- as_bytes$) <- unwrap$
```

```text
$ cargo run
client: / -> HTTP/1.1 200 OK
client: /sleep -> HTTP/1.1 200 OK
client: /nope -> HTTP/1.1 404 NOT FOUND
```

`TcpListener.bind "127.0.0.1:0"` — port 0 means "any free port", and `local_addr$` says which; a real server binds `127.0.0.1:7878` and you open it in a browser. `listener <- incoming$` is an iterator of connections, `take 3` stops it after three, and each is handed to `handle_connection`.

`handle_connection` reads the first line of the request — `GET / HTTP/1.1` — through a `BufReader`, matches it against the three paths it knows, and writes a response: a status line, a `Content-Length` header, a blank line, the file. That is HTTP, or enough of it. The `/sleep` path sleeps first, to stand in for a slow request. `(status_line, filename)` comes out of a `match` whose arms are tuples, with the slow arm a `do:` block; and the request line is matched as a `&str` slice, `&request_line[..]`, because string literals are `&str` and the patterns must have the value's type.

The client is a thread that connects three times in a row and prints each status. Watch the order of the log at the end: the third request could not start until the second's sleep was over, because a single thread served them one at a time. A browser tab waiting behind someone else's slow request is what that feels like.

## 21.2 A thread pool

Spawning a thread per connection would fix that and open a denial-of-service hole — a thousand connections, a thousand threads. A *thread pool* is a fixed number of threads that take jobs from a queue, and it is the shape of most servers:

`src/main.hrs`

```
use std.fs
use std.io.(prelude.*, BufReader)
use std.net.(TcpListener, TcpStream)
use std.sync.(Arc, Mutex)
use std.thread
use std.time.Duration

use pool.ThreadPool

fn main$:
    let listener = TcpListener.bind "127.0.0.1:0" <- unwrap$
    let addr = listener <- local_addr$ <- unwrap$
    let served = Arc.new (Mutex.new (Vec.new$))
    // Four clients at once: the slow request no longer holds up the others.
    let clients: Vec<_> =
        ["/sleep", "/", "/nope", "/"] <- iter$
            <- map (
                   |path|:
                       thread.spawn move ||:
                           let mut stream =
                               TcpStream.connect addr <- unwrap$
                           write! stream "GET {path} HTTP/1.1\r\n\r\n"
                               <- unwrap$
                           BufReader.new (&stream)
                               <- lines$
                               <- next$
                               <- unwrap$
                               <- unwrap$

               )
            <- collect$
    let pool = ThreadPool.new 4

    for stream in listener <- incoming$ <- take 4:
        let stream = stream <- unwrap$
        let served = Arc.clone (&served)

        pool <- execute move ||:
            handle_connection stream served

    for c in clients:
        println! "client got: {}" (c <- join$ <- unwrap$)

    drop pool                                            // graceful shutdown: finish the jobs, join the workers

    let mut log =
        served <- lock$
               <- unwrap$
               <- clone$
    log <- sort$
    println! "{log:?}"

fn handle_connection (mut stream: TcpStream) (served: Arc<Mutex<Vec<String>>>):
    let buf_reader = BufReader.new (&stream)
    let request_line =
        buf_reader <- lines$
                   <- next$
                   <- unwrap$
                   <- unwrap$

    let (status_line, filename) =
        match &request_line[..]:
            "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html")
            "GET /sleep HTTP/1.1" => do:
                thread.sleep (Duration.from_millis 50)
                ("HTTP/1.1 200 OK", "hello.html")
            _ => ("HTTP/1.1 404 NOT FOUND", "404.html")

    let contents = fs.read_to_string filename <- unwrap$
    let length = contents <- len$
    let response = format! "{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}"

    stream <- write_all (response <- as_bytes$) <- unwrap$
    served <- lock$
           <- unwrap$
           <- push (format! "{request_line} -> {status_line}")
```

`src/lib.hrs`

```
use std.sync.(mpsc, Arc, Mutex)
use std.thread

pub struct ThreadPool
    workers: Vec<Worker>
    sender: Option<mpsc.Sender<Job>>

type Job = Box<dyn FnOnce$ + Send + 'static>

impl ThreadPool:
    /// Create a new ThreadPool. `size` is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new size: usize -> ThreadPool:
        assert! (size > 0)

        let (sender, receiver) = mpsc.channel$
        let receiver = Arc.new (Mutex.new receiver)     // one receiver, shared by every worker
        let mut workers = Vec.with_capacity size

        for id in 0..size:
            workers <- push (Worker.new id (Arc.clone (&receiver)))

        ThreadPool\ workers, sender = Some sender

    pub fn execute<F> (&self) (f: F)
        [where F: FnOnce$ + Send + 'static]:
        let job = Box.new f
        self <- sender
             <- as_ref$
             <- unwrap$
             <- send job
             <- unwrap$

impl Drop for ThreadPool:
    fn drop (&mut self):
        drop (self <- sender <- take$)                    // close the channel: workers see Err and stop

        for worker in &mut self <- workers:
            if let Some thread = worker <- thread <- take$:
                thread <- join$ <- unwrap$
                println! "worker {} shut down" (worker <- id)

struct Worker
    id: usize
    thread: Option<thread.JoinHandle<()>>

impl Worker:
    fn new (id: usize) (receiver: Arc<Mutex<mpsc.Receiver<Job>>>) -> Worker:
        let thread = thread.spawn move ||:
            loop:
                let message =
                    receiver <- lock$
                             <- unwrap$
                             <- recv$   // the lock is released here, before the job runs

                match message:
                    Ok job => job$
                    Err _ => break                                    // the sender is gone: shut down

        Worker\ id, thread = Some thread
```

```text
$ cargo run
client got: HTTP/1.1 200 OK
client got: HTTP/1.1 200 OK
client got: HTTP/1.1 404 NOT FOUND
client got: HTTP/1.1 200 OK
worker 0 shut down
worker 1 shut down
worker 2 shut down
worker 3 shut down
["GET / HTTP/1.1 -> HTTP/1.1 200 OK", "GET / HTTP/1.1 -> HTTP/1.1 200 OK", "GET /nope HTTP/1.1 -> HTTP/1.1 404 NOT FOUND", "GET /sleep HTTP/1.1 -> HTTP/1.1 200 OK"]
```

`ThreadPool` is in `lib.hrs`, so it can be tested and reused; `main.hrs` uses it. Reading the library top to bottom:

- A `Job` is a boxed closure, `Box<dyn FnOnce() + Send + 'static>`: a trait object, because each job is a different closure type; `FnOnce`, because it runs once; `Send` and `'static`, because it crosses to another thread and may outlive the caller. A type alias names it once.
- `ThreadPool.new` makes a channel, wraps the *receiver* in `Arc<Mutex<…>>` — one receiver, shared by every worker, locked to take a job — and spawns `size` workers each holding a clone of the `Arc`. `assert! (size > 0)` is the documented panic.
- `execute` boxes the closure and sends it. The bound is in a `[where …]` clause; `self <- sender <- as_ref$ <- unwrap$` reaches the sender inside the `Option`.
- A `Worker` is a thread in a `loop`: lock the receiver, `recv$` a job, *release the lock* (the guard is dropped at the end of the `let` statement — which is why `recv$` is a separate statement from the `match`, so that other workers can take jobs while this one runs), then run it. `Err` from `recv$` means the sender is gone, and the loop ends.
- `Drop for ThreadPool` is the graceful shutdown: `take$` the sender out of its `Option` and drop it, so every worker's next `recv$` returns `Err` and it exits its loop; then `join$` each worker's thread, taken out of *its* `Option`. The two `Option`s exist because `drop` has only `&mut self` and cannot move a field out of it — `take$` moves the value and leaves `None`, which is chapter 18's trick with the post's state.

`main` now starts four clients *at once*, hands each connection to `pool <- execute move ||:`, and `drop pool` at the end waits for the workers. The slow request no longer delays the others — the clients' statuses arrive in the order the threads finish, which is why the served log is sorted before printing. The four `worker N shut down` lines are the graceful shutdown doing exactly what it says.

Everything in this program is a chapter you have read: channels and `Arc<Mutex<T>>` from 16, `Box<dyn FnOnce>` from 18, `Drop` and `take$` from 15 and 18, the `[where …]` bound from 10, `move ||:` from 13. That is the book's argument, made one last time: the notation stayed out of the way, and Rust was what you learned.

## 21.3 Where to go

The Rust Book this one follows has a chapter of appendices — keywords, operators, derivable traits, the tools — which are Rust's and not repeated here. For Harsh itself, the *language guide* is the reference for every construct in every form, and it is short, because Harsh is short. For Rust, the standard library documentation is the next book: `std` is large and well written, and after twenty-one chapters you can read any page of it. Build something. The compiler will tell you when you are wrong, and you now know how to read what it says.

# Harsh at a glance

Everything of Harsh's own, in one place, each rule with the chapter that teaches it. There is little of it, because Harsh states rules only for what it changes; wherever this page is silent, what you learned of Rust in the chapters holds unchanged. One program first, with most of the page in it, built and run like every other in this book:

```
use std.collections.HashMap          // `.` walks a path

#[derive Debug Clone PartialEq]      // an attribute applies inside its brackets
struct Point                         // a record struct: fields beneath...
    x: f64
    y: f64
#[derive Debug]
struct Size\ w: f64, h: f64          // ...or inline after `\`
struct Meters f64                    // a tuple struct: the name applied to its types
struct Origin                        // a unit-like struct

enum Shape                           // a union of the three forms
    Empty                            // unit-like
    Circle f64                       // tuple
    Rect\ w: f64, h: f64             // record, inline (or with the fields beneath)

trait Area:                          // a trait is a block
    fn area (&self) -> f64

impl Area for Shape:
    fn area (&self) -> f64:
        match self:                  // arms: one per line, no commas
            Shape.Empty => 0.0
            Shape.Circle r => 3.14159 * r * r
            Shape.Rect\ w, h => w * h

fn add (a: i32) (b: i32) -> i32:     // one group per parameter
    a + b

fn twice (f: impl Fn i32 -> i32) -> impl Fn i32 -> i32:  // a closure type; a bare parameter ends at `->`, so this one is grouped
    move |x| f (f x)

fn describe<T> (label: &str) (item: T) -> String
    [where T: std.fmt.Debug]:        // a multi-line where clause is bracketed
    format! "{label}: {item:?}"

fn main$:                            // applied to nothing: `$`
    let p = Point\ x = 1.0, y = 2.0  // a literal: `\`, then `field = value`
    let s =
        Size\
            w = 3.0
            h = 4.0
    let shapes = [Shape.Empty, (Shape.Circle 1.0), (Shape.Rect\ w = 2.0, h = 3.0)]

    let total: f64 =                 // a chain of three or more links: vertical
        shapes <- iter$
               <- map (|s| s <- area$)
               <- sum$

    let n = add 2 3                  // application by juxtaposition
    let m = add (n * 2) (add 1 1)    // an expression is one argument in parens
    let inc = 1 |> add               // a partial: one parameter left
    let k = 5 |> inc |> (twice inc)  // pipes: a value into a function, then the next

    let kind = if n > 4: "big" else: "small"     // inline blocks
    let mut counts = HashMap.new$
    for w in ["a", "b", "a"]:
        *counts <- entry w <- or_insert 0 += 1

    let z = do:                      // a bare block as a value
        let t = 2
        t * t
    let d = Meters 1.5               // a tuple index keeps its dot and is part of the name

    println!
        "{p:?} {s:?} {} {total:.2} {n} {m} {k} {kind} {} {z} {}"
        d.0
        (describe "counts" (counts <- len$))
        (matches! Origin Origin)
```

```text
Point { x: 1.0, y: 2.0 } Size { w: 3.0, h: 4.0 } 1.5 9.14 5 12 8 big counts: 2 4 true
```

## Layout — chapter 2

- A block opens at a `:` that ends its line; its body is the lines indented beneath it, and ends where the indentation does. (§2.6)
- The same block may be written inline, the body after the colon on the same line when it is one expression; or in braces on one line only, `{ let u = 3; u * u }`, when several statements must share a line. A `{` and its `}` on different lines is an error. (§2.6)
- `do:` opens a block that belongs to nothing — a scope of its own, a value, an operand in parentheses. (§2.6, §3.1)
- A line ending ends a statement; `;` is written only at the end of a block's last line, to discard its value. (§2.6, §3.4)
- A line indented deeper than the one above it, with no `:` to open a block, continues it. (§2.6)
- A construct that opens mid-line indents its body from its own column; when that reaches too far right, `=` ends its line and the opener starts the next. (§3.5)
- Every line is aligned with an open block or a continuation; a column that matches nothing is an error, never a silent move between scopes. (§3.5)

## Applying — chapter 2

- A function is applied by writing its arguments after it: `add 2 3`. Parentheses around an argument mean *one argument*, never *the arguments*: `add (n * 2) (add 1 1)`. A single token needs none. (§2.6)
- Applied to nothing: `f$`, `fn main$:`, `s <- len$`. `()` is the unit value and only ever that. (§2.6)
- An application binds tighter than `<-`: `greet "a" <- to_uppercase$` applies `greet` first, and the next arrow or an operator ends the arguments; parentheses go round an argument that holds an arrow, `f (x <- g$)`, never round the application. A pipe is the exception: its sides are atoms, so `(f x) |> g`. (§2.6, §13.3)
- A name written tight against a `(` is an error. (§20.5)
- Parentheses make a tuple (the comma does it), set precedence, or group — and nothing else. (§20.5)
- Brackets index, never apply: `arr[1]` is part of its atom (`f arr[1]` passes the element); `arr [1]` is the same index but is refused inside an application; an array argument is isolated, `f ([1, 2])`. (§2.6)
- `<-` reaches into a value: a field, a method. `.` walks a path: a module, a type, an item. A tuple index keeps its dot, `d.0`. (§2.6, §5.1)
- A macro applies like a function, its `!` glued to its name: `vec! [1, 2]`, `println! "{x}"`. (§2.6)

## Declaring — chapters 3, 5, 6, 10

- `fn add (a: i32) (b: i32) -> i32:` — one group per parameter; a lone parameter may be bare, `fn greet name: &str`. A bare parameter ends at `->` or `[where …]`, so a parameter whose type carries an arrow is grouped. (§3.4)
- Three struct forms, three equals: a **record struct**, `struct Point` with `x: f64` beneath or `struct Size\ w: f64, h: f64`; a **tuple struct**, `struct Meters f64`; a **unit-like struct**, `struct Origin`. Nothing marks a declaration's body. (§5.1)
- An enum is a union of variants, each one of the three forms spelled as that struct is: `Empty`, `Circle f64`, `Rect` with fields beneath or `Rect\ w: f64, h: f64`. (§6.1)
- A literal is the name, `\`, and `field = value` — inline to the end of its line or its group's `)`, or a field per line beneath. `..base` last. A literal that is one element of a tuple has its own parentheses. (§5.1)
- `impl Point:`, `trait Area:`, `mod geometry:`, `extern "C":` — all blocks. (§5.3, §10.2, §7.2, §20.1)
- A `where` clause on one line needs nothing; over several lines it is bracketed, `[where T: Debug]`, on its own line under the signature. (§10.2)
- Attributes apply inside their brackets: `#[derive Debug Clone]`, `#[cfg (feature = "x")]`. (§5.2)

## Patterns — chapters 6, 19

- A pattern is an application: `Some n`, `Ok value`, `Circle r`, `Coin.Quarter state`. (§6.2)
- A record is taken apart with the mark that builds it: `Point\ x, y`, `Point\ x: a, y: b`, `Point\ x, ..`. (§19.1)
- `match x:` with one arm per line, `pattern => body`, no commas; several arms on one line are comma-separated. An arm with several statements is `=> do:`. (§6.2, §3.5)

## Closures, chains and pipes — chapter 13

- `|x| x + 1` inline; `|x|:` with the body beneath; `|x: i32| -> i32:` fully annotated; `move ||:` capturing by value. (§13.1)
- A closure's type is the trait applied to its parameter types: `Fn i32 -> i32`, `Fn (i32) (i32) -> i32`, `FnOnce$`. (§13.1)
- A chain of one or two links stays on its line within 72 columns; three or more go vertical, every `<-` under the first, and `=` ends its line before one begins. A block-bodied closure in a chain is a paren block: `(`, the parameters, the body, `)` on its own line. (§13.2)
- `x |> f` hands `x` to `f`; `f <| x` the same from the right; both sides are atoms, and a chain or a closure is isolated first. Fewer values than the function takes defer the rest as one flat closure — `|>` fills from the left, `<|` from the right, both leave a hole in the middle — and the result is a value. A project function given too many is an error. (§13.3)

## Macros and markup — chapter 20

- `macro_rules! twice:` holds arms; a matcher is a parameter list, `( ($x:expr) )`; a transcriber is a block, `=> do:`. (§20.5)
- A macro with a body is `name! do:`; markup inside it is copied and its `{ … }` holes are Harsh; a Dioxus tree is written with the layout, `div:` opening an element. (§20.5)
- A fenced block in a `///` comment is Harsh, and the tool that runs it gets the Rust it expects. (§14.2)

## The tools — chapters 1, 14

- `hrs new`, `hrs run`, `hrs build`, `hrs test`, `hrs check`, `hrs lint`, `hrs watch`; `hrs fmt` lays a file out by these rules and is a no-op on one already laid out; `hrs export` writes the project as a plain Rust crate; `hrs-from` brings Rust in. (§1.2, §14.1)
- Errors point at your `.hrs` line, whether Harsh raised them or the compiler did. (§1.2, §3.4)
