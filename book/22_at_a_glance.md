# Harsh at a glance

Everything of Harsh's own, in one place, each rule with the chapter that teaches it. There is little of it, because Harsh states rules only for what it changes; wherever this page is silent, what you learned of Rust in the chapters holds unchanged. One program first, with most of the page in it, built and run like every other in this book:

@@ glance

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
