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
