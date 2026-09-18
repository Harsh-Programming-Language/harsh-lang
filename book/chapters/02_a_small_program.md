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

        let guess: u32 =
            match line <- trim$ <- parse$\
                Ok n => n
                Err _ => continue

        match guess <- cmp (&secret)\
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
    match guess <- cmp (&secret)\
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

> **Harsh —** A function is applied by writing its arguments after it, separated by spaces: `add 2 3`. Parentheses around an argument mean *this is one argument* — `add (n * 2) (nothing$)` — and never *these are the arguments*; an argument that is a single token needs none. Applying a function to nothing is `nothing$`, because `()` is a value, the unit, and only ever that. The same rule declares a function: `fn add (a: i32) (b: i32)` is one group per parameter, and a lone parameter may drop its parentheses, `fn greet name: &str`. A block's opener says how its entries end, and there are only four kinds. A header that ends itself — `struct Point`, `impl Point`, `mod geometry`, `extern "C"` — needs no mark at all, since nothing but a name can follow it and what comes deeper can only be the body. A `\` opens a comma-separated list — a literal's fields, or a `match`'s arms. A `:` or `do:` opens statements. And `#:` opens a grouping whose grammar belongs to someone else — a macro's DSL — where Harsh writes the braces and the lines and nothing between them. A construct that already has a spelling keeps it: `struct Point #:` is refused rather than read a second way.

An index written tight, `arr[1]`, is part of its atom, so `f arr[1]` passes the element — the same way `t.0` is part of `t`; brackets never apply, and an array passed as an argument is isolated, `f ([1, 2, 3])`. Two arrows share the work of reaching into things: `<-` reaches into a *value* — a field, a method, `v <- len$` — and `.` walks a *path* — a module, a type, an item, `String.from`. A macro applies like a function, with its `!` glued to its name: `vec! [1, 2, 3]`, `println! "{s}"`. And when an application and an arrow meet, **the application binds tighter**: in `greet "harsh" <- to_uppercase$` the function is applied first and the arrow takes its result, and the next arrow, or an operator, ends the arguments. The parentheses you will be tempted to write, `(greet "harsh") <- to_uppercase$`, are not wrong; they are not needed. Parentheses are needed the other way round — when the arrow belongs *inside* an argument, `greet (&(text <- to_uppercase$))`.

## 2.7 What you have met

Immutability by default and `mut` to opt out; `&` and `&mut` on arguments; `Result` for operations that can fail and `match` to take it apart; `loop`, `break`, `continue`; enums; type annotations that tell `parse` what to produce. Each has a chapter. The next one starts with the smallest pieces: variables, types, functions, and control flow.
