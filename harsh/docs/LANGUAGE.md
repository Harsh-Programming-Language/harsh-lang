# Harsh — language guide

This describes the language as implemented, not as designed. Everything documented here has been transpiled, compiled, and run.

Harsh is a layout transformation over Rust. The transpiler understands block structure and applies a small set of local token substitutions; it has no semantic model, no type knowledge, and no name resolution. Everything not mentioned in this guide is passed through to Rust unchanged, which means generics, lifetimes, closures, `impl Trait`, `async`/`.await`, macros, patterns, operators, and every future Rust syntax addition work without the transpiler knowing they exist.

Source files use the `.hrs` extension. The package is `harsh-lang`; the binaries are `hrs`, `hrs-from`, `hrs-remap` and `hrs-lsp`.

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

## The mark

```
#[Ha<rs>.h]
     │  │
     │  └── .h
     └───── rs
```

Eleven characters, five readings, none contradicting the others.

- **A word.** Read straight through it spells `Hars.h`.
- **An attribute.** `#[…]` is Rust's attribute syntax, so the mark reads as *this code is Harsh*.
- **A generic.** `Ha<rs>` parses as a type instantiation before the eye notices it spells a word.
- **A C header.** `hars.h` is the thing you drop into any project and it works, which is close to Harsh's actual claim about Rust crates.
- **The file extension.** Read `.h` and `rs` as one unit and it is `.hrs` — the real extension, hiding inside a spelling that looks like C.
    - This is why the orange is structural rather than decorative: it marks the half of `.hrs` that was separated from the dot to make the word work, and it is what lets a reader find `rs` again once the letters have been rearranged.

Typographic rules for the mark are in `docs/BRAND.md`.

## How to read this guide

The guide is in two parts. **The tutorial** builds one program from a single function to a small application, introducing each construct where it is first needed. **The reference** then covers every construct with all the forms it can take — indented, inline, braced, and any alternative spellings — with what each transpiles to.

Every snippet in both parts was transpiled, compiled and run before it was written down; the programs they were taken from are in `examples/guide/`, one per section, and the tutorial's finished program is `examples/guide/30_tutorial.hrs`. Where a form has a limit, the limit is stated where the form is described rather than collected at the end, though a summary of the limits found while writing this guide sits near the end for anyone checking before they rely on one.

Three rules do most of the work in Harsh, and everything else follows from them:

- A colon ending a logical line opens a block; a more-indented line anywhere else continues the current line.
- Application is by juxtaposition, `f a b`, and parentheses isolate **one** argument — a parenthesised group with commas is a tuple, always.
- `.` is the path separator and `<-` is member access, both directions, no exceptions.

## Setting up

Everything below was run, as written, before it was written down. Harsh needs one thing you may already have and one thing you build.

### What you need

- **A Rust toolchain** — `rustc` and `cargo`, installed with [rustup](https://rustup.rs). `hrs lint` also needs clippy, which rustup installs as a component: `rustup component add clippy`. Check with `cargo --version`.
- **The Harsh repository** — the archive or a clone. It is an ordinary Cargo package.

### Building the tools

From the published crate:

```
cargo install harsh-lang
```

or, from the repository root, `cargo install --path .` to get the tree as it is. Either installs four executables into `~/.cargo/bin`, which rustup already put on your `PATH`:

| Executable | What it does |
|---|---|
| `hrs` | The transpiler and the build driver: `hrs new`, `hrs build`, `hrs run`, and the rest below |
| `hrs-from` | The converter, Rust to Harsh |
| `hrs-remap` | Maps cargo's diagnostics back to `.hrs` lines; `hrs` calls it for you |
| `hrs-lsp` | The language server; the editor extensions launch it for you |

Check with `hrs` alone, which prints its usage. (If you would rather not install, `cargo build --release` leaves the same four under `target/release/`.)

### Your first project

```
hrs new hello
cd hello
hrs run
```

prints `Hello from Harsh`. The project `hrs new` creates has three files:

```
hello/
  Cargo.toml
  .gitignore
  src/main.hrs
```

`Cargo.toml` is a normal manifest with one addition — the binary target points at the *generated* tree, not at `src/`:

```toml
[[bin]]
name = "hello"
path = "target/hrs/main.rs"
```

That is the whole arrangement. You write `src/**.hrs`; `hrs build` transpiles each one to `target/hrs/**.rs` beside a source map; cargo compiles what is in `target/hrs/`; errors come back pointing at your `.hrs` line and column. `target/` is ignored by git, so the generated Rust is never committed.

To convert an existing cargo project, add that `[[bin]]` section (with the package's name) and move `src/main.rs` aside. If the `[[bin]]` entry is missing, `hrs` refuses to build and says so — otherwise cargo would silently compile `src/main.rs` and run the wrong program.

### The commands

```text
hrs build [cargo args]     transpile, then cargo build
hrs run   [cargo args]     transpile, then cargo run
hrs test  [cargo args]     transpile, then cargo test
hrs check [cargo args]     transpile, then cargo check
hrs lint  [cargo args]     transpile, then cargo clippy
hrs watch [subcommand]     rebuild on every save (default: check)
hrs new   <name>           create a project laid out for Harsh
hrs export [dir]           write the project as a plain Rust crate,
                           formatted with cargo fmt (default: target/export)
hrs <input.hrs> [-o out.rs] [--map out.map.json]
                           transpile a single file
```

Anything after the subcommand goes to cargo unchanged: `hrs run --release`, `hrs test -- --nocapture`. Each command first reports what the transpile step did — `hrs: transpiled 1 of 3 file(s)` or `hrs: 3 file(s) up to date` — so a build that does nothing says so.

The generated Rust under `target/hrs/` is not meant to be read. It keeps the `.hrs` file's line breaks so that every diagnostic maps back to the right line, and it is not formatted, because formatting would move every byte the source map depends on. When a Rust reader needs the code — a reviewer, a collaborator without `hrs`, a crates.io upload — `hrs export` writes the project as an ordinary cargo crate: `Cargo.toml` with its targets under `src/`, every module transpiled, no source maps, and `cargo fmt` run over the result (when rustfmt is installed; `rustup component add rustfmt`). The exported crate builds with `cargo build` alone. Nothing in it points back at the `.hrs` files, so it is a copy, not a link: export again after changes.

### Bringing existing Rust in

```
hrs-from src/old.rs -o src/old.hrs
```

The converter writes Harsh from Rust: braces become indentation, `::` becomes `.`, `.method()` becomes `<- method$`, a `fn`'s parameter list becomes one group per parameter, `use a::{b, c}` becomes `use a.(b, c)`. Its output is valid Harsh that transpiles back to token-identical Rust — the transpiler's own source goes through this round trip in the test suite — and it is the fastest way to see what a file of your own looks like in Harsh.

### Editors

- **VSCode** — `code --install-extension harsh-lang.harsh-lang`, or search for *Harsh* in the Extensions view ([the listing](https://marketplace.visualstudio.com/items?itemName=harsh-lang.harsh-lang)); to run the tree's own version instead, `npm install` then `npx @vscode/vsce package` in `editors/vscode-harsh` and install the `.vsix`. Highlighting for `.hrs`, folding by indentation, and — with `hrs-lsp` on your `PATH`, which `cargo install --path .` puts there — layout-aware Enter and Tab: Enter after a chain line lands under the previous arrow, after `let d =` one level in, after a block opener inside the block; Tab on a fresh line moves to the next column the layout allows, and past the last of them keeps indenting a unit at a time; Shift-Tab comes back a unit at a time to column 0. Set `harsh.serverPath` if the binary lives elsewhere. Marketplace listing to come.
- **Zed** — `editors/zed-harsh` is a Zed extension backed by the tree-sitter grammar in `editors/tree-sitter-harsh`; install it as a dev extension from the Zed command palette once the grammar repository it names is reachable. It launches `hrs-lsp` too, so Enter behaves as in VSCode; Zed has no way to bind Tab to a language-server request, so Tab there stays Zed's own.
- **Anything with tree-sitter** — `editors/tree-sitter-harsh` is the grammar; it parses every example in this repository with no error nodes.

None of this is required. A `.hrs` file is plain text and `hrs` does not care which editor wrote it.

### What is not there yet

The language server, `hrs-lsp`, does two things: on-type formatting, as described under Editors, and formatting on save — `hrs fmt` on the buffer. Types on hover, go-to-definition and inline errors are not available in `.hrs` files; errors appear when you run `hrs check` or `hrs watch`, mapped to the right line. `hrs fmt` applies the layout style this guide follows: `hrs fmt --check` lists the files that would change, `hrs fmt` rewrites them in place; it changes no token and never joins lines. Design in `docs/FMT.md`.

# Part one — a tutorial

The program counts words in a text and prints the most frequent ones in a chosen format. It is around sixty lines when finished and uses most of the language on the way there.

## A first function

Harsh has no braces. A colon at the end of a line opens a block, and the block is whatever is indented beneath it.

```
fn tokenize text: &str -> Vec<String>:
    text <- split_whitespace$ <- map (|w| w <- to_lowercase$) <- collect$
```

Three things are visible already.

- `fn tokenize text: &str` declares one parameter without parentheses. Parameters are one group each, `fn f (a: T) (b: U)`; a single parameter may drop its parentheses.
- `<-` is method call and field access — Rust's `.`. Reading left to right, `text <- split_whitespace$` is `text.split_whitespace()`.
- `split_whitespace$` applies the function to nothing. `$` is the zero-argument call, written tight against the name like the `!` of a macro; a bare name is a *value* — the function itself — rather than a call. Rust's `()` is not used for this because in Harsh `()` is only ever the unit value.

The Rust it becomes:

```rust
fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace().map(|w| w.to_lowercase()).collect()
}
```

## Calling it

Arguments are juxtaposed: the function, then each argument separated by spaces.

```
fn main$:
    let words = tokenize "the cat sat on the mat"
    println! "{} words" (words <- len$)
```

The second line shows the one rule about parentheses that matters most. `words <- len$` is not an atom — it contains an operator — so to pass it as an argument it must be **isolated** in parentheses. Parentheses never group several arguments; they mark the boundary of one. `println! "{} words" (words <- len$)` is a macro applied to two arguments, a string and an isolated expression.

Macros and functions follow the same rule. `println! "{} words" x` is `println!("{} words", x)`.

## Counting

```
use std.collections.HashMap

fn count (words: &[String]) -> HashMap<String, usize>:
    let mut m = HashMap.new$
    for w in words:
        *m <- entry (w <- clone$) <- or_insert 0 += 1
    m
```

- `use std.collections.HashMap` — the path separator is `.`. Rust's `::` is a syntax error in Harsh.
- `HashMap.new$` — a path to an associated function, then a zero-argument call.
- The `for` loop opens a block with its colon, like a function does.
- The last line of a block is its value. `m` is returned because it is last and has no `;`. Semicolons are inserted for you on every other line.

## A chain across lines

`sorted` turns the map into a list ordered by count. The chain of method calls is long, so it is broken across lines: a more-indented line continues the current one, and the leading `<-` makes each link visible.

```
fn sorted counts: HashMap<String, usize> -> Vec<(String, usize)>:
    let mut v: Vec<(String, usize)> = counts <- into_iter$ <- collect$
    v <- sort_by
        |a, b|:
            b.1 <- cmp (&a.1) <- then_with (|| a.0 <- cmp (&b.0))
    v
```

The closure passed to `sort_by` is written with its body **indented** rather than in braces: `|a, b|:` opens a block, and the block is the closure's body. When a closure ends a line this way, the transpiler closes the argument list after the block, producing `v.sort_by(|a, b| { … })`. The closure starts its own continuation line so that its body indents from *it*, not from `v` — the header and the body then form one shape, which is the layout style this guide follows throughout.

The whole of `v <- sort_by |a, b|: …` is one statement — the closure is an expression inside it — so it ends with `;` in the Rust because another statement follows it, exactly as any statement does. The `v` on the last line is the function's value.

## Pipes

The three functions compose into a pipeline. `|>` passes the values on its left to the function on its right — and if fewer are given than the function takes, the rest is deferred as a closure, which is how the pipes double as partial application. Here every function takes one value, so each step is a plain call.

```
fn pipeline text: &str -> Vec<(String, usize)>:
    text |> tokenize |> (|w: Vec<String>| count (&w)) |> sorted
```

- `text |> tokenize` is `tokenize(text)`; the chain reads left to right.
- The middle step is a closure in isolating parentheses, because the function a pipe applies must be **one atom**, and a closure with a body is not one until it is isolated.
- `count (&w)` — `&w` is an operator expression, so it too is isolated. `count &w` would be bitwise-and.

The backward pipe `<|` exists as well: `tokenize <| text`. It is right-associative, so `f <| g <| x` is `f(g(x))`.

The pipes do one more thing, and it is the thing that makes them Harsh's rather than borrowed. A pipe may carry **several values** — `3 5 |> sub` — and if it carries *fewer* than the function takes, the rest is **deferred**: the result is a closure waiting for what is missing. `|>` fills parameters from the left, `<|` from the right, and both on one function leave a hole in the middle.

```fragment
fn scale (factor: f64) (offset: f64) (x: f64) -> f64:
    x * factor + offset

let f = 2.0 0.5 |> scale       →  move |__hrs1| scale(2.0, 0.5, __hrs1)
let g = scale <| 10.0          →  move |__hrs1, __hrs2| scale(__hrs1, __hrs2, 10.0)
f 3.0                          →  6.5
g 2.0 0.5                      →  20.5
```

- `f` is a closure over the last parameter; `g` over the first two. Both are values: bind them, pass them to `map`, return them.
- With every parameter supplied the closure disappears and it is a plain call: `2.0 0.5 3.0 |> scale` is `scale(2.0, 0.5, 3.0)`.
- Harsh knows how many parameters `scale` takes because `scale` is declared in this project. For a function it cannot see into — the standard library, a crate — a pipe is a plain call with the values given.

The reference section "Pipes" and "Partial application with the pipes" have every form and every error.

## A type for the output

```
#[derive Debug Clone PartialEq]
enum Format
    Plain
    Ranked
    Boxed
        width: usize
```

An enum body lists its variants one per line, and nothing marks the body: only a name can follow `enum`, so the deeper lines can only be variants. Each variant is one of the three struct forms: `Plain` is unit-like, a tuple variant would be `Circle f64` — the payload types juxtaposed after the name — and `Boxed` is a record variant whose fields are indented beneath it, or written `Boxed\ width: usize` on one line when that reads better.

```
struct Report
    counts: Vec<(String, usize)>
    format: Format

impl Report
    fn new (counts: Vec<(String, usize)>) (format: Format) -> Self:
        Report\ counts, format

    fn top (&self) (n: usize) -> Vec<(String, usize)>:
        self <- counts <- iter$ <- take n <- cloned$ <- collect$
```

Struct fields go one per line with no commas, and the declaration's body follows its name with no mark. A struct is built with `\`: `Report\ counts, format` — see "Building a struct".

## Matching

```
    fn render_line (&self) (idx: usize) (word: &str) (n: usize) -> String:
        match &self <- format\
            Format.Plain => format! "{word} {n}"
            Format.Ranked => format! "{}. {word} {n}" (idx + 1)
            Format.Boxed\ width => format! "|{:<w$}|{n:>3}|" word (w = *width)
```

- `match` opens a block; each arm is one line, pattern `=>` body, and the commas Rust needs are inserted.
- `Format.Plain` in a pattern uses the same `.` path separator as everywhere else.
- `(w = *width)` — a named format argument contains `=`, which terminates an application run, so it is isolated like any other non-atom.

An arm whose body needs several lines ends with `=>` and indents:

```
    fn render (&self) (n: usize) -> String:
        let mut out = Vec<String>.new$
        for (i, (w, c)) in self <- top n <- iter$ <- enumerate$:
            out <- push (self <- render_line i w (*c))
        out <- join "\n"
```

`Vec<String>.new$` needs no turbofish; the transpiler inserts `::<>` because the generic list is followed by a path step. `(*c)` is isolated because `*c` is an operator expression.

## Putting it together

```
fn main$:
    let text = "the cat sat on the mat. The mat sat still; the cat did not."
    let counts = pipeline text
    let boxed = Report.new (counts <- clone$) (Format.Boxed\ width = 6)
    let ranked = Report.new counts Format.Ranked
    println! "{}" (boxed <- render 3)
    println! "{}" (ranked <- render 2)
```

```
|the   |  4|
|cat   |  2|
|mat   |  2|
1. the 4
2. cat 2
```

`Report.new counts Format.Ranked` is three atoms — a path, a name, a path — and needs no parentheses at all. `Report.new (counts <- clone$) (Format.Boxed { width: 6 })` isolates its two non-atom arguments. That contrast is the whole of the application rule.

## What the tutorial skipped

Partial application (the pipes), inline blocks on one line, `while let`, labelled loops, `let … else`, the three closure spellings, and the full set of match patterns. Each is in the reference below with every form it takes.

# Part two — the language

## Blocks

### The four kinds, and what opens each

A block's opener says how its entries end. There are four kinds, and every
block in Harsh is one of them:

| opener | the body | separator Harsh writes |
|---|---|---|
| *(no mark)* | items, after a header that ends itself — `struct`, `enum`, `union`, `impl`, `trait`, `mod`, `extern` | none (or `,` for a declaration's fields) |
| `\` | a comma-separated list: a literal's fields, an inline declaration, a `match`'s arms, a macro's entries | `,` |
| `:` / `do:` | statements, and a tail value | `;` |
| `#:` | a grouping whose grammar is its author's | none |

`impl`, `trait`, `mod` and `extern` take no mark because nothing but a name
can follow them, exactly as for `struct` and `enum`: the header ends itself,
so what follows deeper can only be the body.

```
impl Point                          impl Point {
    fn x (&self) -> f64:                fn x(&self) -> f64 {
        self <- x                           self.x
                                        }
                                    }
```

`#:` is for a grouping Harsh does not define — a macro's DSL, most often. A
construct that already has a spelling keeps it: `struct Point #:` is refused,
naming the spelling it should have used, since there is one spelling per
construct.

### Opening a block

A logical line whose last significant token is a colon opens a block. The colon is replaced by an opening brace, the following more-indented lines become the body, and a closing brace is emitted when indentation returns to the header's level.

```
fn greet (name: &str) -> String:
    format! "Hello, {name}"
```

```rust
fn greet(name: &str) -> String {
    format!("Hello, {name}")
}
```

A `match` is a specification block in use, like a literal's fields, so it is opened by `\` — `match value\` with the arms beneath, or `match value\ p => e, q => f` inline — and its arms are comma-separated in the Rust. Each arm's body opens with a fat arrow, since an arm already has one:

```
match value\
    Some n =>
        log n
        n * 2
    None => 0
```

- The colon must be the last significant token on the **logical** line, which is why it never collides with type annotations — those are always mid-line.
- Trailing comments do not count, so `fn f$:  // note` still opens a block.
- A block opener with no indented body following it is an error, reported against the colon.

### The three ways to write any block

- Every block can be written indented, inline on one line, or with Rust's braces, and all three emit identical Rust.

```
fn indented (t: bool) -> i32:
    if t:
        1
    else:
        2

fn inline (t: bool) -> i32:
    if t: 1 else: 2

fn braced (t: bool) -> i32:
    if t { 1 } else { 2 }
```

- The inline form is a colon that is **not** the last token on its line; it is described under Inline blocks.
- The braced form is Rust's, **on one line only**: `{ let u = 3; u * u }`. A `{` and its `}` on different lines is an error naming `:` / `do:` — two syntaxes for one block across files is the drift the language refuses. Inside a one-line brace pair, Rust's rules apply unchanged.
- Two places keep multi-line braces because there they are not a block: a hole in markup or a tree (`on:click={move |_|:` with the body beneath and `}` closing it — the brace supplies an expression to an attribute, and the layout inside is the hole's), and a brace group handed as an argument, `json! ({ … })`, an object literal.
- Which to use is a style question. The indented form is the idiom; the inline form suits one-line conditionals; braces are the one-line spelling, and what you reach for when pasting a line of Rust.
- A block in operand position — `a && { … }` in Rust — is a `do:` block isolated in parens: `a && (do:` with the body beneath and `)`; the parens are optional grouping and are not emitted. A block as a value is `let x = do:` with the body beneath. An empty function body is written as the unit: `fn main$:` / `()`, with any comment above it.

### The two block openers

- `:` and `do:` open a block interchangeably, on any construct that takes one. (A declaration takes none — `struct`, `enum`, `union`, `impl`, `trait`, `mod`, `extern`: the body follows the header, and `struct P do` is refused like `struct P:`; a literal is marked `\`.)

```
fn f (x: i32) -> i32 do:
    x + 1

fn g$:
    if true do:
        print! "a"
    let h = |x| do:
        x * 2
```

- They emit identical Rust. `do` is reserved in Rust and collides with nothing.
- The reason for the second spelling is the eye: `-> String:` can be read as a type annotation for a moment, and `-> String do:` cannot. `do` is familiar from several languages and never looks like a declaration, since nothing precedes it the way `let` precedes a binding.
- Bare `do:` — with nothing before it — is the standalone block, described under Standalone blocks.

### Continuation lines

A more-indented line that does *not* follow a block opener is a continuation of the current logical line rather than a new block. This is what makes multi-line method chains work with no leading-dot rule and no explicit line-continuation marker.

```
let listener =
    TcpListener.bind "127.0.0.1:8089"
        <- await
        <- unwrap$

let d =
    a_name_too_long_to_chain_from
        <- iter$
        <- map (|n| n * 2)
        <- sum<i32>$
```

- A logical line ends when the next line is at the same indentation or shallower.
- Continuation applies to any construct, including multi-line `where` clauses before a block-opening colon.

```
fn parse_all<'a, I> (items: I) -> Result<Vec<i64>, ParseErr>
        where I: Iterator<Item = &'a str>:
    // body
```

### Bracketed regions

Inside `(`, `[`, or `{`, layout is suppressed entirely. Lines break freely and indentation carries no meaning until the bracket closes.

- This is how multi-line argument lists, array literals, and brace-form struct literals work.
- It is also why a closure body written inside a call cannot currently use indentation — use the brace form there.

### Layout style

`hrs fmt` applies everything in this section — `hrs fmt --check` lists the files that would change, `hrs fmt` rewrites them, the editors format on save — and it changes no token and never joins lines: it adds the breaks the style asks for and puts every line at its column.

The layout rules require a block to be indented past its *statement*. The recommended style is stricter: **a block's lines are indented past the column of the construct that opened it**, so a header and its body form one shape. When the opener is the first token on its line — `fn`, `if`, a `match` at the margin — the two rules coincide. They differ only when the opener is mid-line, and that is where the weaker form reads badly: the header far right, the body far left, nothing joining them.

The way to keep the opener at the start of its line is to break after `=`, so the closure or `match` begins the continuation and its body indents from there:

```fragment
let a =                           not      let a = |x: i32| -> i32:
    |x: i32| -> i32:                          let d = x * 2
        let d = x * 2                         d + 1
        d + 1

let f =                           not      let f = |x: i32| -> i32:
    |x: i32| -> i32:                          x * 2
        x * 2

let g =                           not      let g = |x: i32| do:
    |x: i32| do:                              x * 4
        x * 4

let z = || 7                      not      let z = ||:
                                              7
```

A chain's shape follows its length. **One or two links sit on one line; three or more take one link per line, the arrows aligned under the first.** The one-line form gives way when the line would pass 72 columns of code: then every link stands on its own line. A chain that is the value of a `=` starts on the line after it — `=` ends its line — and if it is one or two links that fit there, it stays horizontal on that line. A block-bodied closure never sits inside a horizontal chain, since its dedent breaks the line the chain is drawing:

```fragment
let message =                          not      let message = receiver <- lock$ <- unwrap$ <- recv$
    receiver <- lock$
             <- unwrap$
             <- recv$

let statuses: Vec<Status> =            not      let statuses: Vec<Status> = (0u32..3) <- map Status.Value <- collect$
    (0u32..3) <- map Status.Value <- collect$

v <- iter$                             not      v <- iter$ <- map (|x|:
  <- map (|x| x * 2)                                  x * 2)
  <- filter (|x| x > &2)                            <- filter (|x| x > &2)
  <- count$                                       <- count$
```

In a chain with one link per line the arrows align under the first arrow when it follows a single name, whatever column that is — alignment outranks the indentation unit, which governs only the first line of a construct. When the receiver is a longer expression the links sit one unit past its line instead, so they do not drift to the right of a long receiver:

```
["/sleep", "/", "/nope", "/"] <- iter$
    <- map (|path| fetch path)
    <- collect$
```

A chain is a run of arrows each applied to the result of the one before. The four arrows of `self <- width > other <- width && self <- height > other <- height` are four chains of one link each, on four operands, and the rule leaves that line alone. `hrs fmt` applies all of this.

When the chain has a left-hand side, break after `=`; the `=` always ends its line and never starts one. A block-bodied closure, if one is needed, is isolated in parentheses with the `)` on its own line:

```fragment
let d: Vec<i32> =                 not      let d: Vec<i32> = v <- iter$ <- map (|x|:
    v <- iter$                                   x * 10
      <- map (|x| x * 10)                      ) <- collect$
      <- collect$

let d: Vec<i32> =                 not      let d: Vec<i32>
    v <- iter$                               = v <- iter$
      <- map (                                     <- map (|x| x * 10)
             |x|:                                  <- collect$
                 x * 10
         )
      <- collect$
```

Every bracket group — the parentheses isolating an argument, a tuple's, a list's brackets, a macro's — has an **anchor**: the callee when a name precedes the bracket (`map (`, `foo.bar (`, `vec![`), the bracket itself when nothing does (`let t = (`). The group's contents are one unit past the anchor, and its closer stands on its own line under the anchor. The group then has one shape wherever it sits, and the shape survives a rename, since no column depends on the length of a name. Inside the brackets, Rust's rules hold as always — a tuple or array spelled one element per line keeps its commas:

```fragment
let t =                           let xs =                          let ys =
    (                                 [                                 vec![
        1,                                1,                                1,
        2                                 2                                 2
    )                                 ]                                 ]
```

A call with several arguments does not use parentheses this way — a parenthesised group is one argument — so it breaks by juxtaposition, each argument a continuation line one unit past the callee, and there is no closer:

```
let t =
    compute
        1
        2
```

Breaking after `=` is again what puts the anchor at the start of its line; `let xs = [` with the elements one unit past the `[` and the `]` under it is accepted and the same rule, only wider. Braces are not in this sentence: `{` is Rust's, and what follows it is laid out by Rust's rules.

One compact form is accepted: when the nested construct begins on the same line as the bracket and its body is a single line, the closer ends that line rather than taking one of its own —

```
v <- iter$
  <- map (|x|:
              x * 10)
  <- collect$
```

— the body still one unit past the construct that opened it (`|x|`, not the line). With a body of more than one line the closer goes back under the anchor; a closer at the end of an arbitrary last statement is the trailing-comma problem in a new coat.

When the receiver is long — a long name, or a path call — it takes the continuation line alone and the arrows hang under it, one level in:

```
let listener =
    TcpListener.bind "127.0.0.1:8089"
        <- await
        <- unwrap$
```

None of this is enforced by the transpiler; all of it is what the formatter will produce. Every form on this page, on both sides, is valid Harsh.

### Blank lines and comments

- Blank lines are insignificant. Up to two consecutive blank lines are preserved in the output.
- Comment-only lines float to the next real line, so their indentation never opens or closes a block accidentally.
- Comments are preserved in the generated Rust, including doc comments, which rustdoc then consumes normally.

### Exercises

[Harshlings](https://gitlab.com/bahiminin.benoit.dah.opensource/harshlings) — fifty small exercises, Rustlings-style, in Harsh: `cargo install harsh-lang`, clone, `hrs run`. The topics follow *The Harsh Programming Language*, and Harsh's own rules — the layout, `$`, the `\` literal, the pipes, a spaced `[` inside an application — sit where the construct is taught.

### Doc examples

A fenced block inside a `///` or `//!` comment is the one part of a comment the transpiler reads, because rustdoc reads it too: it lifts the block out, compiles it and runs it as a test. It is code, so it is Harsh, and the transpiler writes the Rust rustdoc expects.

```
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

```rust
/// Adds one to the number given.
///
/// # Examples
///
/// ```
/// let arg = 5;
/// let answer = my_crate::add_one(arg);
///
/// assert_eq!(6, answer);
/// ```
pub fn add_one(x: i32) -> i32 {
    x + 1
}
```

- Which fences: exactly the ones rustdoc compiles — an empty info string, `rust`, or the doctest attributes (`ignore`, `no_run`, `should_panic`, `compile_fail`, `edition2021`, ...). A fence that names another language (`text`, `toml`) is prose and is copied through untouched.
- An example is a list of statements: it is transpiled inside a wrapper, as rustdoc also wraps it, so its last line keeps its `;` rather than becoming a block's tail expression.
- rustdoc's hidden lines (`# ` inside the fence) are its mark, not Harsh's `#`: the marker is taken off before the line is read and put back after.
- An error in an example is a transpile error, reported on the line of the `.hrs` file where you wrote it.
- `hrs-from` converts in the other direction, so a crate brought in from Rust has its doc examples in Harsh like the rest of it.

## Function parameters

A function has one parameter syntax: **one parenthesised group per parameter.** The groups concatenate into Rust's comma list. A single parameter may drop its parentheses — that is not a second form, only the one-group case with nothing to separate.

```
fn greet2 (name: &str) (age: i32) -> String:
    format! "Hello, {name}, age {}" age

fn greet name: &str -> String:
    format! "Hello, {name}"
```

```rust
fn greet2(name: &str, age: i32) -> String { .. }
fn greet(name: &str) -> String { .. }
```

- Adjacent parenthesised groups concatenate, with commas inserted between them.
- A group holds exactly one parameter. Rust's comma list, `fn f (a: T, b: U)`, is a syntax error at the comma — it would be a second spelling of the same declaration. `hrs-from` writes the groups for you when bringing Rust in. Commas *inside* a parameter are not separators and are fine: a tuple pattern `((a, b): (i32, i32))`, a payload `(i32, i32)`. A function pointer type is an application, `(g: fn i32 i32 -> i32)`.
- A function with no parameters is `fn f$` — the same `$` that applies a name to nothing at a call. An empty group `()` in a declaration is an error, since a group holds a parameter and `()` is the unit value; a parameter *of type* unit is an ordinary group, `(u: ())`.
- The rewrite is bounded: it applies only between the function name (after any generics) and the `->` or the block-opening colon, so no expression parsing is involved.

The bare form has one restriction. A parameter type may itself contain `->`, and there is then no way to tell which arrow ends the parameter, so a type containing a top-level `->` must be parenthesised:

```
fn apply (g: fn i32 -> i32) (v: i32) -> i32:
    g v
```

Arguments juxtapose in the same way; see Application.

### Every parameter form

- All of these are legal and produce the Rust shown.

```fragment
fn curried (a: i32) (b: i32) -> i32:       fn curried (a: i32, b: i32) -> i32
fn bare name: &str -> String:              fn bare (name: &str) -> String
fn tuple ((a, b): (i32, i32)) -> i32:      fn tuple ((a, b): (i32, i32)) -> i32
fn unit$ -> i32:                         fn unit() -> i32
fn takes_unit (u: ()) -> i32:            fn takes_unit(u: ()) -> i32
fn generic<T: std.fmt.Debug> (label: &str) (item: T) -> String:
fn fnptr (g: fn i32 -> i32) (v: i32) -> i32:
```

- `takes_unit (u: ())` — the unit type in a parameter is Rust's, untouched; only the group around the parameter is Harsh's.
- `fnptr (g: fn i32 -> i32)` — the parameter's type contains `->`, so it must be parenthesised; the bare form would not know which arrow ends the parameter.
- Generic parameters go where Rust puts them, between the name and the first group.
- `self` is a parameter like any other under these rules: alone it may be bare or grouped, `fn top &self:` or `fn top (&self):`; with others it is a group, `fn top (&self) (n: usize)`. A bare parameter followed by a group — `fn top &self (n: usize)` — is an error naming the grouped form.

## Inline blocks

A colon that is **not** the last token on a line opens an inline block. It ends at the end of the logical line, at `else`, or at a separator that is not its own. Items inside carry the separator the indented form would have inserted.

| Block kind | Separator | Inline form |
|---|---|---|
| match arms | `,` | `match x: A => 1, B => 2` |
| fields | `,` | `struct P: x: i32, y: i32` |
| statements | — | one expression: `do: a * 2`, `if c: f$ else: g$` |

An inline block holds **one expression** — one rule, no separator. Several statements on one line are Rust's braces, `if c { f$; g$ } else { h$ }` or `{ let a = 1; a * 2 }` on its own, and a `;` inside an inline colon-block is an error. The reason is that an inline statement block nested in a comma-block has no end the layout can find: `A => do: f(); g(), B => 2` would put `g()` outside the arm. Braces say where a block ends; the colon form does not, on one line. The two are not combined: `if c: { .. }` is an error, since the braces open the block by themselves and the colon would open a second one inside it. Braces are for short blocks — Rust's line and Rust's judgement; anything longer takes the indented form. (A `;` that ends the whole line still discards the tail value, as everywhere.)

```
if t: 1 else: 2              →  if t { 1 } else { 2 }
for i in 0..3: f i           →  for i in 0..3 { f(i) }
```

Each `else` binds to the nearest open `if`. Braces select the other reading:

```
if a: if b: 1 else: 2 else: 3        →  if a { if b { 1 } else { 2 } } else { 3 }
if a: if b { 1 } else { 0 } else: 2  →  if a { if b { 1 } else { 0 } } else { 2 }
```

The inline and indented forms of the same program emit identical Rust; this is enforced by a test.

### Inline forms of each construct

- Every block-opening construct has an inline form, and what separates its items is whatever the indented form would have inserted.

```
if t: 1 else: 2
if let Some n = v: n else: -1
match n: 0 => "zero", 1 | 2 => "small", _ => "big"
for i in 0..3: s += i
while let Some top = st <- pop$: popped += top
{ let a = 1; a * 2 }
struct P: x: i32, y: i32
```

- A `let` binding may take an inline block as its value: `let r = if let Some n = v: n * 2 else: 0`.
- An inline block ends at the end of its logical line. Writing `else: if …` on one line and `else:` on the next therefore orphans the second `else`, and it is rejected; both the fully inline and the fully indented chain work.

```fragment
if n == 1: 10 else: if n == 2: 20 else: 30      ✓ one line

if n == 1:                                       ✓ indented
    10
else if n == 2:
    20
else:
    30

if n == 1: 10                                    ✗ rejected
else: if n == 2: 20
else: 30
```

## Closures

A closure body may be indented, using either `|x|:` or `|x| do:` — the two are identical, because `do:` is the existing bare-block marker and a closure body is a brace block.

```
let f =
    |x: i32| -> i32:
        x * 2
let g =
    |x: i32| do:
        x * 4
let z = || 7
```

As a trailing argument, the closure's argument list closes after the block. If the chain continues past it, the closure is isolated in parentheses so the `)` says where it ends:

```
v <- iter$                        v.iter()
  <- map (|n|:                        .map(|n| {
       n * 2                              n * 2
     )                                })
  <- collect$                         .collect()
```

Closure parameters are written as in Rust: a comma list inside the pipes, typed or not — `|a, b|`, `|a: i32, b: i32|`, `|a: &i32, b: &i32| -> bool:` with a block body. There is no curried closure form, because Rust already gives `|a| |b| body` a meaning — a closure returning a closure — and Harsh never changes what Rust means. So the asymmetry is Rust's: `fn` parameters are one group each; closure parameters are a comma list. When what you want is a closure over the remaining parameters of a function, a pipe gives it without writing one.

```
let add = |a: i32, b: i32| a + b    // two parameters, comma list
let curried = |a: i32| move |b: i32| a * b    // a closure returning a closure, as in Rust
let sub2 = 10 |> sub    // a closure over the remaining parameters
```

A closure is recognised only where it is unambiguous — parameters ending a block header. Elsewhere `|` and `||` remain operators, and a closure needs isolating parens: `f (|x| body)`.

### Every closure form

```
let a =
    |x: i32| -> i32:    // indented body, with return type
        let d = x * 2
        d + 1

let b =
    |x: i32| do:    // indented body, `do:` marker — identical to `|x|:`
        x * 3

let z = || 7    // zero parameters, inline body

let e =
    move |x: i32| -> i32:    // capturing by move
        x + base

let c = apply (|x| x + 100) 1    // inline body, isolated as an argument
```

- `|x|:` and `|x| do:` are the same thing. `do:` is the ordinary bare-block marker and a closure body is a brace block.
- A closure with an inline body inside a call **must** be isolated in parentheses, because `|` is also an operator and only the block-header position is unambiguous.

### Closures with indented bodies as arguments

- When a closure's parameter list ends a line, the indented block beneath is its body, and the statement containing it is an ordinary statement — it ends where its block ends and takes a `;` if another statement follows.

```
v <- sort_by                       v.sort_by(|a, b| {
    |a, b|:                             a.cmp(b)
        a <- cmp b                 });
println! "{:?}" v                  println!("{:?}", v)
```

- Whether a following line is inside the closure or after it is decided by indentation alone, like any block: indent it under the body and it is part of the closure.
- **If a chain continues after the closure, the closure must be isolated in parens.** The `)` is what says where the closure ends and the chain resumes; without it the chain has nothing to attach to, and it is rejected with a message naming this fix.

```
v <- iter$                               v.iter()
  <- map (|x|:                               .map(|x| {
       x * 2                                       x * 2
     )                                         })
  <- filter (|x| x > &2)                     .filter(|x| x > &2)
  <- count$                                  .count()
```

- The `)` sits on a line of its own at the closure's column, as above; it may also end the body's last line, `x * 2)`, which the layout accepts but the style does not use.

```
let d: Vec<i32> =
    v <- iter$
      <- map (|x|:
           x * 10
         )
      <- collect$
```

- Parens are transparent to layout: the `:` opens a block inside them exactly as it would outside, and the body is governed by the same rules — several statements, nested blocks, its own closures. Parens isolate, or raise precedence; they neither open nor close anything. A block written inside a group simply owns no token outside it, so it ends where the group ends.
- The same holds on one line. A closure's prototype `|p|` followed by `:` opens an inline block, and the block ends at the `)` of its group; whatever follows the `)` carries on the statement, and may open another such block:

```
xs <- iter$ <- map (|p|: if *p > 1: *p else: 0) <- collect$      xs.iter().map(|p| { if *p > 1 { *p } else { 0 } }).collect()
xs <- iter$ <- map (|p: &i32|: *p + 1) <- sum$                     xs.iter().map(|p: &i32| { *p + 1 }).sum()
xs <- iter$ <- fold 0 (|acc, x|: acc + x)                          xs.iter().fold(0, |acc, x| { acc + x })
let f = |x|: x + 1                                                let f = |x| { x + 1 };
```

- A line inside an open `(` indents past the line that opened it, or is the `)` that closes it; a line at the opener's column is an error, as it would be outside a group.
- `|x|:` and `|x| do:` are the same closure: a prototype followed by a block, in either of the block's spellings. `do:` emits one pair of braces wherever it stands.
- The bare form and the isolated form emit the same Rust for the closure; the isolated form additionally lets the statement carry on.

## Statements and semicolons

Semicolons are optional. The transpiler inserts them from the layout, and an explicit semicolon is always honoured.

- Every logical line in a statement block gets a semicolon except the last, so the last line is a tail expression exactly as in Rust.
- A line beginning with `let`, `use`, `const`, `static`, `type`, `mod`, `extern`, or `return` always gets a semicolon, including when it is last, because those are statements rather than tail values.
- An explicit semicolon you write is preserved and suppresses the last-line exemption. This is the escape hatch when you want to discard a tail value.
- A nested block gets a semicolon after its closing brace only when its header began with `let`, `const`, `static`, `type`, or `use`. An `if` or `match` used as a statement does not.
- Attribute lines never take a separator and never count as a block's tail.

```
fn total (xs: &[i64]) -> i64:
    let mut sum = 0
    for x in xs:
        sum += x
    sum
```

```rust
fn total(xs: &[i64]) -> i64 {
    let mut sum = 0;
    for x in xs {
        sum += x
    }
    sum
}
```

### The three rules

- A statement ends at a newline, unless an unclosed bracket or a deeper-indented next line carries it on. `let x = vec![1,` followed by `2]` on the next line is one statement.
- A `;` is written in Harsh in exactly one place: at the end of a block's last statement, where it means *emit one*. That discards the tail value, as it does in Rust. A `;` anywhere else is rejected, since the newline already ended that statement.
- In the Rust, every statement ends with `;` except the last in its block, which gets one only if the Harsh had one. This applies at every nesting level independently — a closure body is a block and is governed by the same rule at its own level.

```fragment
parent_statement                 parent_statement;
my_func                          my_func(|| {
    || do:                           closure_statement;
        closure_statement            last_closure_statement
        last_closure_statement   });
parent_statement                 parent_statement;
last_parent_statement            last_parent_statement
```

```fragment
parent_statement                 parent_statement;
my_func                          my_func(|| {
    || do:                           closure_statement;
        closure_statement            last_closure_statement;
        last_closure_statement;  });
parent_statement                 parent_statement;
last_parent_statement;
                                 last_parent_statement;
```

- Going the other way, `hrs-from` removes every `;` except one ending a block's last statement, which is why the round trip is exact.

### Worked forms

```
fn tail$ -> i32:            fn tail() -> i32 {
    let a = 1                     let a = 1;
    a + 1                         a + 1
                              }

fn discard$ -> ():          fn discard() -> () {
    let a = 1                     let a = 1;
    a;                            a;
                              }

fn ret$ -> i32:             fn ret() -> i32 {
    if true:                      if true {
        return 9                      return 9
    0                             }
                                  0
                              }
```

- `return 9` gets no `;` even though it is not the block's last line in the source, because `{ return 9 }` is valid Rust and the line is last *in its block*.
- `let v = if true: 1 else: 2` — a block used as a value gets its `;` after the closing brace, because the `let` needs one.

## Paths and member access

This is the largest departure from Rust, and the one that costs paste-compatibility.

### The path separator

A dot is the path separator, replacing `::`.

```
use std.collections.HashMap
let v = geometry.Vec2\ x = 1.0, y = 2.0
let e = ParseErr.BadDigit c
```

`::` is not valid in Harsh. Writing it is a syntax error, because allowing both spellings for one thing is exactly the drift that a permissive rule produces:

```text
error: `::` is not valid in Harsh; the path separator is `.`
  --> general.hrs:30:9
   |
 30| impl fmt::Display for Circle:
   |         ^^
```

### Member access

A left arrow is field access and method call, replacing Rust's dot.

```
self <- x * self <- x + self <- y * self <- y
let name = params <- name <- as_deref$ <- unwrap_or "World"
```

- The arrow binds tightly, so whitespace around it is dropped in the output while line breaks are kept. This is what allows the vertical chain style.
- `<-` is already a reserved token in rustc, so it never collides with a comparison against a negative value — Rust itself requires a space in `x < -1`.

The mapping is a plain substitution in both directions, with no context and no exceptions:

| Rust | Harsh |
|---|---|
| `::` | `.` |
| `.` | `<-` |

```
"empty" <- to_string$
String.from "hi" <- len$
s <- len$
params <- name
self <- field
```

### Chain forms

- Spacing around `<-` is insignificant: spaced, tight and mixed produce identical Rust.

```
let a = p <- x
let b = p<-y
let c = v <- len$<-to_string$
```

- A horizontal chain and a vertical one are the same expression; the vertical form is the continuation rule with `<-` leading each line.

```
let e = v <- iter$ <- max$ <- copied$ <- unwrap_or 0

let d =
    v <- iter$
      <- map (|n| n * 2)
      <- sum<i32>$
```

- `sum<i32> ()` — a generic method call needs no turbofish; after `<-` the `<` is always generics.
- A tuple index or float keeps its dot — `t.0`, `t.1`, `2.5` — and a tuple index is part of its atom, so `show t.0 t.1` is two arguments: `show(t.0, t.1)`.

### Where the dot is not a path separator

Three cases, and they are different tokens rather than exceptions to the rule above:

- **After a numeric literal** it is part of a float: `1.0`, `2.5e3`.
- **Before a digit** it is a tuple index: `t.0`, `t.0.1`. Safe because a path segment can never begin with a digit.
- **Doubled** it is a range or rest pattern: `0..3`, `1..=5`, `..base`, `Foo { x, .. }`.

Everywhere else a `.` becomes `::` and a `<-` becomes `.`, unconditionally.

Writing `s.len()` in Harsh therefore produces `s::len()`, which rustc rejects with a diagnostic that remaps to the right column and suggests an instance method. The transpiler does not try to catch this itself: a `.` following a value could only come from `::` following a value in Rust, which is not valid Rust either, so the converter can never produce one. The only source is a typo, and rustc already reports those.

There is no accommodation for pasted Rust. Use `hrs-from`, which converts completely and predictably.

## Generics

Generic argument lists pass through unchanged in type position and are automatically turbofished when followed by a path separator.

```
let v = Vec<i32>.new$
let m = HashMap<String, usize>.new$
let counts: Vec<(String, usize)> = tally (&words) <- into_iter$ <- collect$
```

```rust
let v = Vec::<i32>::new();
let m = HashMap::<String, usize>::new();
let counts: Vec<(String, usize)> = tally(&words).into_iter().collect();
```

- You never write turbofish yourself. `Vec<i32>::new()` is a parse error in Rust's expression position, so the transpiler inserts the `::`.
- The substitution is safe without knowing type from expression position, because turbofish is legal in both.

### Generics in every position

```fragment
fn generic<T: std.fmt.Debug> (label: &str) (item: T) -> String:     function
struct Generic<T>:                                                   struct
    inner: T
impl<T: std.fmt.Debug> Generic<T>:                                   impl
    fn show (&self) -> String:
        format! "{:?}" (self <- inner)
let m = HashMap<String, i32>.new$                                  call — turbofish inserted
let parsed = "42" <- parse<i32>$ <- unwrap$                      method — turbofish inserted
let v: Vec<i32> = q$                                               type — untouched
```

- The turbofish is inserted only where a generic list is followed by a path step or a call. In type position it is left alone, so `let v: Vec<i32>` emits exactly that.

## Items

### Use declarations

A dot followed by a parenthesised group is a use-tree group. It is the only grouped form: `use` never opens an indented block, and `use a.b:` with entries below is a syntax error. Inside the parentheses the commas are yours, as in Rust.

```
use axum.(
        serve,
        Router,
        routing.get,
        response.(Html, IntoResponse),
        extract.(Query, Path)
    )
use tokio.net.TcpListener
```

```rust
use axum::{
        serve,
        Router,
        routing::get,
        response::{Html, IntoResponse},
        extract::{Query, Path}
    };
use tokio::net::TcpListener;
```

- The group spans lines freely because parentheses suppress layout.
- `.(` is not valid Rust in any position, so the rewrite needs no context.

- Nested groups and `self` work inside a group:

```
use std.io.(self, Write)
use std.collections.(
        BTreeMap,
        HashSet,
    )
```

### Structs and enums

A declaration's body follows its name with no mark: `struct` and `enum` take exactly one name (and its generics, and at most a `[where …]`), so what stands deeper can only be the body. Fields and variants are one per line; the commas are supplied.

```
#[derive Debug Clone]                        #[derive(Debug, Clone)]
struct Point                                 struct Point {
    x: f64                                       x: f64,
    y: f64                                       y: f64,
                                             }
enum Shape                                   enum Shape {
    Empty                                        Empty,
    Circle f64                                   Circle(f64),
    Rect Point Point                             Rect(Point, Point),
    Named                                        Named {
        name: String                                 name: String,
        sides: u8                                    sides: u8,
                                                 },
                                             }
```

There are three forms of struct — **record** (named fields; Rust's docs call this one simply a struct), **tuple** (fields by position), **unit-like** (no fields) — and an enum is a union of variants each of which is one of the three, spelled as that struct is spelled with the `struct` and the name's line taken away. A tuple variant's payload is an application — the variant applied to its types, one atom each, a generic type isolated: `Boxed (Box<List>)`. A record variant is its bare name with the fields beneath, or `Named\ name: String, sides: u8` on its own line inside the block: the block and inline forms mix freely, and which reads better depends on the names and their lengths.

**Inline**, the field list is marked with `\` and its items comma-separated. The `\` says where the header ends and the parts begin, as the newline does in the block form:

```
struct Point\ x: f64, y: f64
enum Shape\ Empty, Circle f64, Rect Point Point, (Named\ name: String, sides: u8)
struct Wrapper<T> [where T: Display]\ field: T, other: u32
```

**Every struct form:**

```
struct Unit                             struct Unit;                       // unit-like struct
struct Meters f64                       struct Meters(f64);                // tuple struct: the name applied to its payload
struct Pair i32 i32                     struct Pair(i32, i32);
struct Wrapper<T> (Vec<T>)              struct Wrapper<T>(Vec<T>);         // a generic payload, isolated
struct Point                            struct Point {                     // record struct
    x: f64                                  x: f64,
    y: f64                                  y: f64,
                                        }
struct Inline\ x: f64, y: f64           struct Inline {                    // record struct, inline
                                            x: f64,
                                            y: f64,
                                        }
struct Bounded<T> [where T: Display]    struct Bounded<T>                  // with a where clause: brackets on the header
    field: T                            where
                                            T: Display,
                                        {
                                            field: T,
                                        }
```

`struct Point:` — a colon on the header — is an error naming the markless form: braces build nothing in Harsh, and a colon would be the one block opener that is not a keyword or a prototype.

### Building a struct — the literal

A value is always marked `\`. In expression position a bare `Point` is a complete expression already (a unit struct's value, a constant, a function), so the mark is what says "the name is the head, and fields follow". Fields take `=` — the `:` is the declaration's — a bare `field` is the shorthand, `..base` is last:

```
let p =                                      let p = Point {
    Point\                                       x: 5.0,
        x = 5.0                                  y: 7.0,
        y = 7.0                              };

let p = Point\ x = 5.0, y = 7.0             let p = Point { x: 5.0, y: 7.0 };          // inline: to the end of the line, or the `)` isolating it
let q = Point\ x, y                         let q = Point { x, y };                    // shorthand
let r = Point\ x = 1.0, ..base              let r = Point { x: 1.0, ..base };          // update syntax
let m = Message.Move\ x = 1, y = 2          let m = Message::Move { x: 1, y: 2 };      // a record variant
```

An inline literal reads to the end of its line or to the `)` that isolates it, which is what lets it stand anywhere an expression can — in a header, in a list, as an operand:

```fragment
match (Point\ x = 1.0, y = 4.0)\            match (Point { x: 1.0, y: 4.0 }) {
    Point\ x, .. => x
if p == (Point\ x = 1.0):                   if p == (Point { x: 1.0 }) {
let v = vec! [(Point\ x = 1), (Point\ x = 2)]
(Point\ x = 1, y = 0) + (Point\ x = 2, y = 3)
```

Because the field list reaches its group's `)`, a name after the last `=` field is a shorthand field, never something outside the literal — the transpiler reads no type to decide, so the line says what it means on its own, and rustc says whether the fields fit:

```
struct Point                                struct Point {
    x: i32                                      x: i32,
    msg: String                                 msg: String,
                                            }

fn a msg: String -> Point:                  fn a(msg: String) -> Point {
    Point\ x = 1, msg                           Point { x: 1, msg }        // preferred: no parens to wonder about
                                            }

fn b (msg: String) -> Point:                fn b(msg: String) -> Point {
    (Point\ x = 1, msg)                         Point { x: 1, msg }        // the same literal, grouped
                                            }

fn c msg: String -> (Point, String):        fn c(msg: String) -> (Point, String) {
    ((Point\ x = 1, msg = m$), msg)              (Point { x: 1, msg: m() }, msg)    // a tuple: the literal has its own group
                                            }
```

Written `(Point\ x = 1, msg)` when `Point` has no field `msg`, it is still one literal, and rustc names the missing field. Whether a literal is satisfied is rustc's question; where it ends is the layout's, and that never depends on a definition. `hrs-from` isolates a literal that is one element of a Rust tuple for the same reason.

`Point { x: 1 }` — a brace literal — is an error, on one line or several: one spelling per construct.

### Patterns

A pattern destructures with the same mark: `Point\ x, y`, `Point\ x: px, ..` (a rename keeps Rust's `:`). The braced pattern is still accepted.

```
let Point\ x, y = p
match q\
    Point\ x: 0, y => println! "on the y axis at {y}"
    Some (Point\ x, ..) => x
```

### Traits and impls

Item bodies take no separator, except that a declaration without a body gets a semicolon.

```
pub trait Shape
    fn area (&self) -> f64

    fn describe (&self) -> String:
        format! "shape with area {:.2}" (self <- area$)

impl Shape for Circle
    fn area (&self) -> f64:
        3.14159 * self <- r * self <- r
```

- `fn area (&self) -> f64` has no body, so it is a required method and terminates with a semicolon.
- `fn describe` opens a block and becomes a default method.
- The same rule covers associated types, associated constants, `use` inside an impl, and item macros.

- A generic `impl` and a trait `impl` for a tuple struct:

```
impl<T: std.fmt.Debug> Generic<T>
    fn show (&self) -> String:
        format! "{:?}" (self <- inner)

impl Area for Tuple
    fn area (&self) -> f64:
        (self.0 * self.1) as f64
```

### Modules

```
mod geometry
    pub struct Vec2
        pub x: f64
        pub y: f64

    impl Vec2
        pub fn dot (self) (o: Vec2) -> f64:
            self <- x * o <- x + self <- y * o <- y
```

A module declaration without a body is just `mod name` and gets its semicolon automatically.

### Constants and statics

```
const MAX_DEPTH: usize = 8
static GREETING: &str = "tokenizing"
```

### Type aliases

```
type Pair = (i32, i32)
let p: Pair = (8, 9)
```

- A `type` line always terminates with `;`, like `const` and `static`.

## Where clauses

- Two forms, both legal. The single-line bare form needs nothing:

```
fn one<T> (x: T) -> T where T: Clone:
    x
```

- The multi-line form is bracketed, so that the clause reads as part of the header rather than as a sibling of the body — which lets the body sit at the same indent as the clause instead of one level further right:

```
fn three<T, U> (a: T) (b: U) -> String
    [where
        T: std.fmt.Debug,
        U: std.fmt.Display]:
    format! "{a:?} {b}"
```

- The brackets are stripped on emission. They are a reading aid, not a parsing necessity.

A `where` clause that spans lines must be bracketed, because the `:` in a bound would otherwise be read as a block opener. Brackets suppress layout, so the bound is safe inside them; they are stripped on emission.

```
fn f<T> (x: T) -> T
    [where T: Clone + Send]:
    x

fn g<T, U> (a: T) (b: U) -> String
    [where
        T: std.fmt.Debug,
        U: std.fmt.Display]:
    format! "{a:?} {b}"
```

- The body may be indented to any depth greater than the `fn`; both `    x` and `        x` work, since the bracketed line is a continuation of the signature.
- A single-line `where` needs no brackets: `fn f<T> (x: T) -> T where T: Clone:` is fine.

## Application

A run of atoms is a call. Parentheses **isolate one argument** and never group several, so a group with commas is always a tuple, and an empty group `()` is the unit value. Applying to *nothing* is `$`, a suffix.

```
f a b            →  f(a, b)          two arguments
f (a, b)         →  f((a, b))        one tuple argument
f$               →  f()              zero arguments: `$` applies a name to nothing
f ()             →  f(())            one argument, the unit value
f (a + 1) (g b)  →  f(a + 1, g(b))    a non-atom argument needs isolating parens
```

An atom is a non-keyword identifier (with optional path, generics and macro bang), a literal, a tuple index like `t.0`, a parenthesised group, or any of these followed by `$` — `f$` is one atom, so `g f$ 5` is `g(f(), 5)` and `f$ 5` applies the result of `f()` to `5`. An index written tight, `a[i]`, is part of its atom: `f a[i]` is `f(a[i])`, as `f t.0` is `f(t.0)`, and `a[i].0[j]` is one atom too. Brackets never apply — `a [i]` is the same index — but inside an application the spaced form is refused, since it could only mean the index of the wrong thing: write `a[i]`, or `(a [i])` to make it one argument. An array passed as an argument is isolated: `f ([1, 2, 3])`.

Application binds tighter than `<-`: `f x <- g$` is `(f x) <- g$`. To apply the arrow to the argument instead, isolate it.

```
show "abc" <- to_string$     →  show("abc").to_string()
show ("abc" <- trim$)        →  show("abc".trim())
```

Macros follow the identical rule:

```
println! "{}" (greet "Ben")   →  println!("{}", greet("Ben"))
format! "Hello, {name}"       →  format!("Hello, {name}")
```

- Rust call syntax written into a macro — `println!("{}", greet("Ben"))` — is rejected, since under these rules it passes a single tuple.
- A macro with a pattern argument juxtaposes like anything else, the pattern isolated as one argument: `matches! x (Some n if n > 1)`. The bracket and brace forms are Rust's and pass through: `vec! [0; 4]`, `quote! { … }`.
- Braces are opaque to application.

### What is and is not an atom

- An application run is a head followed by atoms. The run ends at the first thing that is not an atom, and everything after that is emitted as written.

| Atom | Not an atom — isolate it |
|---|---|
| `x`, `42`, `"s"`, `'c'` | `a + 1`, `*c`, `&w`, `!flag` |
| `Some`, `Point.new`, `std.io.stdout` | `w = *width` (named format argument) |
| `(a + 1)`, `(g b)`, `t.0` — a group, or a tuple index | `\|x\| body` — a closure with an inline body |
| `m~` — a macro name | `f(a, b)` — an argument list, which is an error |
| `f$`, `s <- len$`, `(add 10)$` — an application to nothing | `f $` — the `$` is a suffix and is never spaced |

- The rule that decides every row: **if it contains an operator, isolate it.** `count &w` is bitwise-and; `count (&w)` is a call. `render i w *c` is a multiplication; `render i w (*c)` is a call.

```
let b = two (a + 1) (one 2)      →  let b = two(a + 1, one(2))
let e = takes_unit ()            →  let e = takes_unit(())
let f = (one) 3                  →  let f = (one)(3)
```

- A parenthesised group may itself be the **head** of an application: `(add 10) 7` applies the closure that `add 10` returns, and `(add 10)$` applies it to nothing.
- `$` is the one suffix. It means *apply to nothing* in a call — `main$`, `String.new$`, `v <- iter$ <- collect$`, `m~$` — and *no parameters* in a declaration, `fn main$:`. The two are the same rule read from either side. It is tight, like `!`; `f $` with a space is an error naming `f$`. Because `$` carries this meaning, `()` never has to: `f ()` passes the unit value, `Ok ()` is `Ok(())`, `tx <- send ()` sends unit, and there is no `(())`. (A macro transcriber's `$a` is a prefix, a metavariable, and is an atom there.)

### The four uses of parentheses

- Parentheses have exactly four jobs in Harsh. Anything else is an error.
    - **A tuple.** A parenthesised group containing top-level commas is a tuple, always: `(2, "good", 4.0)`, and `f (a, b)` passes one tuple argument. The empty group `()` is the empty tuple, unit, and nothing else.
    - **One parameter group in a declaration.** `fn f (a: T) (b: U)` — each group holds one parameter, or several with commas in Rust's form.
    - **Isolating one argument.** A multi-token argument is wrapped so that its boundary is unambiguous: `f (a + 1) (g b)`. This is needed whenever an argument is more than one token, and for a closure whenever its body is on the same line — `map (|x| x * 2)` — or whenever a chain follows an indented one.
    - **Grouping in a `use` tree.** `use axum.(serve, Router)`.
- Passing an argument list is not one of them. `f(a, b)` reads under these rules as a call with a single tuple argument, and since that is almost never what was meant, a macro written that way is rejected outright; a function written that way is a type error in the Rust. `f()` reads as `f ()` — spacing never matters — and passes unit; rustc reports the wrong count on the right line. The zero-argument call is `f$`.
- Parens change precedence and isolate. They never change the layout or the meaning of what they surround: a block opened inside them is still a block, governed by the same rules.
- Square brackets have one job outside Rust's own arrays and indexing: `[where …]`, which visually separates a multi-line where clause from the body beneath it so the body need not indent further. The bare single-line form `fn f<T> (x: T) -> T where T: Clone:` is equally legal.

### Pipes

Two operators, and one idea: a pipe **fills a function's parameters** — `|>` from the left, `<|` from the right. When the count reaches the function's arity the result is a plain call; when it falls short, what is missing is **deferred** as one flat closure. That makes the pipes both the way to apply without parentheses and the way to partially apply.

```fragment
fn sub (a: i32) (b: i32) (c: i32) -> i32:
    a - b - c

3 |> sub            →  move |__hrs1, __hrs2| sub(3, __hrs1, __hrs2)
3 5 |> sub          →  move |__hrs1| sub(3, 5, __hrs1)
3 5 10 |> sub       →  sub(3, 5, 10)

sub <| 3            →  move |__hrs1, __hrs2| sub(__hrs1, __hrs2, 3)
sub <| 3 5          →  move |__hrs1| sub(__hrs1, 3, 5)
sub <| 3 5 10       →  sub(3, 5, 10)

1 |> sub <| 3       →  move |__hrs1| sub(1, __hrs1, 3)        the middle hole
```

- **Each side is a sequence of atoms, all arguments.** `f a |> g` is `g(f, a)`, not `g(f(a))`; to pipe the *result* of an application, isolate it: `(f a) |> g`. The same on the right of `<|`: `merge <| (f a)`. An empty call is one atom, so `merge <| routes_hello$` needs no isolation; and `()` is the unit value, an atom like any other: `() |> send` is `send(())`.
- **The function is one atom**, and `a |> f <| b` is one application: `a` fills from the left, `b` from the right.
- `|>` is left-associative, `<|` right-associative, and a chained result is one value: `x |> f |> g` is `g(f(x))`, `f <| g <| x` is `f(g(x))`.

```
double <| 21                →  double(21)
show <| double <| 3         →  show(double(3))
5 |> double                 →  double(5)
5 |> double |> show         →  show(double(5))
show (double <| 4)          →  show(double(4))       a pipe inside an argument
(double 4) |> show          →  show(double(4))       a result, isolated
```

### Partial application with the pipes

A pipe that supplies fewer values than the function takes does not fail; it **defers**. The result is a closure over the parameters that were not filled, and that closure is an ordinary value.

```fragment
fn sub (a: i32) (b: i32) (c: i32) -> i32:
    a - b - c

let s3 = 3 |> sub          →  move |__hrs1, __hrs2| sub(3, __hrs1, __hrs2)     two left
let s3_5 = 3 5 |> sub      →  move |__hrs1| sub(3, 5, __hrs1)                  one left
let last = sub <| 1        →  move |__hrs1, __hrs2| sub(__hrs1, __hrs2, 1)     filled from the right
let mid = 3 |> sub <| 1    →  move |__hrs1| sub(3, __hrs1, 1)                  the middle hole
s3 5 1                     →  s3(5, 1)                                        = -3
```

- **Which parameters are filled** is decided by the side: `|>` fills the first ones, `<|` the last ones, and the two together leave a hole in the middle. There is no placeholder token; the hole is what is left.
- **Arity** is what tells a call from a partial. Harsh knows it for a `fn` declared anywhere in the project, whatever declaration form was used, and for a partial bound by `let`. It does not know it for an imported function, a method, or a closure parameter — those live in Rust's type checker. **Unknown arity: the atoms given are taken as all of them, and it is a plain call**, so `text |> String.from` and `x |> println!` work as they read; if the count was wrong, rustc says so. Known arity with too many arguments is a Harsh error naming the function and both counts.
- A partial is a **value**: bind it, pass it, store it, return it, apply it by juxtaposition like any function. Bound by `let`, its remaining arity is known, so it composes.

```
fn sub (a: i32) (b: i32) (c: i32) -> i32:     fn sub(a: i32, b: i32, c: i32) -> i32 {
    a - b - c                                     a - b - c
                                              }
fn apply2 (f: impl Fn (i32) (i32) -> i32)     fn apply2(f: impl Fn(i32, i32) -> i32, a: i32, b: i32) -> i32 {
    (a: i32) (b: i32) -> i32:
    f a b                                         f(a, b)
                                              }
fn main$:                                     fn main() {
    let sub3 = 3 |> sub                           let sub3 = move |__hrs1, __hrs2| sub(3, __hrs1, __hrs2);
    let sub3_5 = 5 |> sub3                        let sub3_5 = move |__hrs1| sub3(5, __hrs1);
    let a = sub3_5 100                            let a = sub3_5(100);                        // = -102
    let b = 5 |> (3 |> sub)                       let b = move |__hrs1| (move |__hrs1, __hrs2| sub(3, __hrs1, __hrs2))(5, __hrs1);    // the same as sub3_5, inline: a partial's remaining arity is known
    let c = apply2 sub3 5 100                     let c = apply2(sub3, 5, 100);
    let d = 7 |> (30 40 |> sub)                   let d = (move |__hrs1| sub(30, 40, __hrs1))(7);    // arity met: a plain call, the closure gone
    println! "{a} {} {c} {d}" (b 100)             println!("{a} {} {c} {d}", b(100))
                                              }
```

- The closure is flat — one closure with all the remaining parameters, not a chain of one-parameter closures — which is what lets `sub3 5 100` apply it in one step. It costs nothing at runtime: the compiler aliases `(move |p| f(a, p))(b)` to `f(a, b)`, verified in the generated assembly.
- A function with nothing supplied is already a value in Rust, so there is nothing to defer: pass `one`, not a pipe.
- Errors, each naming the fix:

```fragment
1 2 3 4 |> sub          `sub` takes 3 parameter(s) and 4 were piped in
x |> f y                after a pipe's function only `<|` may follow; isolate: `x |> (f a)`
1 + 2 |> f              a pipe's arguments are atoms; isolate an expression in parentheses
f $                     `$` applies a name to nothing and is written tight against it: `f$`
fn f ():                `()` is the unit value; a function with no parameters is `fn f$`
```

- A prefix `$a` in a `macro_rules~` body is Rust's metavariable sigil and is an atom.

### Functions returning closures

Written as a function returning a closure, with no special support needed — the ordinary rules apply the result:

```fragment
fn add a: i32 -> impl Fn i32 -> i32:
    move |b| a + b

let inc = add 1        →  let inc = add(1);
let r = inc 5          →  let r = inc(5);
let s = (add 10) 7     →  let s = (add(10))(7);
let t = 3 |> (add 100) →  let t = (add(100))(3);
```

A parenthesised group may head an application, which is what makes `(add 10) 7` work.

## Match

Arms are separated by line breaks rather than commas. An arm body may be inline after the fat arrow, or an indented block when the arrow ends the line.

```
match parse_all items\
    Ok ns =>
        let total: i64 = ns <- iter$ <- sum$
        println! "parsed {:?} total {}" ns total
    Err e => println! "error: {:?}" e
```

- Commas are inserted after every arm, including the last.
- Guards and or-patterns pass through unchanged.
- A `;` at the end of an arm is rejected. `;` discards a block's tail value, and an arm body is not a block. The same applies to struct fields, enum variants and use-tree entries.

A struct literal may appear in a scrutinee or condition, which Rust itself forbids. Harsh has no ambiguity there, since the `:` ends the expression, so the expression is parenthesised on emission:

```
match P { x: 1, y: 2 }\
    P { x, y } => x + y
```

```rust
match (P { x: 1, y: 2 }) {
    P { x, y } => x + y,
}
```

### Every match form

```
match s\    // indented arms
    Shape.Empty => "empty" <- to_string$
    Shape.Circle r => format! "circle {r}"
    Shape.Rect w h =>    // block-bodied arm
        let area = w * h
        format! "rect {area}"
    Shape.Named { name, sides } => format! "{name}/{sides}"

match n: 0 => "zero", 1 | 2 => "small", _ => "big"    // inline arms

match n { 0 => "zero", _ => "other" }    // braced
```

### Patterns

- Patterns juxtapose with the same grammar as application: `Shape.Circle r`, `Shape.Rect w h`, `Some x`, `Ok (a, b)` for a tuple payload.
- Record patterns keep braces: `Shape.Named { name, sides }`, `E.Rec { a, b }`.

```
match n\    // guards
    x if x < 0 => "neg"
    0 => "zero"
    x if x % 2 == 0 => "even"
    _ => "odd"

match p\    // tuple patterns
    (0, y) => y
    (x, 0) => x
    (x, y) => x * y

match n\    // ranges
    0..=9 => "digit"
    b'a'..=b'z' => "lower"
    _ => "other"

match n\    // bindings
    v @ 1..=5 => format! "low {v}"
    v => format! "hi {v}"

match n: 0 => "zero", 1 | 2 => "small", _ => "big"    // or-patterns
```

### Match as a value and nested matches

```
let r = match n\
    1 => 10
    _ => 20

match a\
    Some x =>
        match b\
            Some y => x + y
            None => x
    None => 0
```

- A `match` used as a `let` initialiser gets its `;` after the closing brace.
- A nested `match` in an arm body is an ordinary block-bodied arm; indentation does the rest.

## If

### Every if form

```fragment
if n < 0:                                       indented, with else-if chain
    "neg"
else if n == 0:
    "zero"
else:
    "pos"

if n < 0: "neg" else: if n == 0: "zero" else: "pos"      inline

if n < 0 { "neg" } else if n == 0 { "zero" } else { "pos" }   braced

if a: if b: 1 else: 2 else: 3                   else binds to the nearest if   → 2 when a && !b
if a: if b { 1 } else { 0 } else: 2             braces select the outer binding → 0 when a && !b
```

### If let

```
if let Some n = v:    // indented
    n
else:
    -1

if let Some n = v: n else: -1    // inline

let r = if let Some n = v: n * 2 else: 0    // as a value
```

- The pattern juxtaposes: `Some n`, not `Some(n)`.
- A struct literal is allowed in the condition, which Rust forbids; the condition is parenthesised on emission.

## Control flow

Every Rust control-flow construct opens its block with a colon.

```
for i in 0..3:
    if i % 2 == 0:
        println! "even {i}"
    else:
        println! "odd {i}"

while let Some top = stack <- pop$:
    print! "{top} "

if let Some first = words <- first$:
    println! "first = {first}"
```

`else` currently emits on its own line after the closing brace. This is valid Rust; rustfmt would join it.

### Loops

```
for i in 0..4:    // for, indented
    s += i

for i in 0..3: s += i    // for, inline

for (a, b) in &pairs: s += a * b    // for over a pattern

for i in 0..2 { w += i }    // braced

while n < 3:    // while
    n += 1

while let Some top = st <- pop$: popped += top    // while let

let found = loop:    // loop with a break value
    k += 1
    if k == 5: break k * 10

'outer: for i in 0..3:    // labelled loops
    for j in 0..3:
        if i * j == 2: continue 'outer
        hits += 1
```

- `break k * 10` inside an inline `if` works because `break` terminates an application run, so the value is not read as arguments to something.
- The label `'outer:` is a lifetime-shaped token followed by a colon, and the colon is mid-line, so it does not open a block; the `for` that follows does.

## Let bindings

### Every let form

```
let a = 1    // plain
let b: i64 = 2    // typed
let mut c = 3    // mutable
let (d, e) = (4, 5)    // destructuring a tuple
let P\ x, y = P\ x = 6, y = 7    // destructuring a struct
let Some f = get$    // let-else, indented
    else:
        return
let a = a + 100    // shadowing
let g =    // a block as the value
    do:
        let t = 2
        t * t
let h = { let u = 3; u * u }    // several statements on one line: Rust's braces
```

- `let … else:` opens a block like any other; the body must diverge, as in Rust.
- The `else` block may also be inline: `let Some f = get$ else: return`.

## Standalone blocks

A bare block is written `do:`. The keyword is a marker only and never reaches the output — Rust reserves `do` but has no syntax for it, which is exactly why it was chosen: it collides with no real identifier.

```
let scoped =
    do:
        let a = 10
        a * 2
```

```rust
let scoped = {
    let a = 10;
    a * 2
};
```

`do` is therefore not available as an identifier in hrs. It is the only keyword the language adds.

## Struct literals

Struct literals keep Rust's brace form. Braces remain legal everywhere they are in Rust, and inside them layout is suppressed.

```
let c = Circle\ r = 2.0
let p = Point\ x = 3, y = 4
```

Struct literals: `Point\` with the fields beneath, or inline; the mark is what distinguishes a value from a bare name (see "Building a struct").

## Error handling

- Rust's `?` operator, `Result`, `Option` and `let … else` all pass through, since none of them involve braces or the tokens Harsh rewrites.

```
fn fallible (n: i32) -> Result<i32, String>:
    if n < 0: Err ("negative" <- to_string$) else: Ok (n * 2)

fn uses_question (n: i32) -> Result<i32, String>:
    let v = fallible n ?
    Ok (v + 1)
```

- `fallible n ?` — the `?` terminates the application run, so it applies to the whole call: `fallible(n)?`.
- `Err ("negative" <- to_string$)` and `Ok (n * 2)` isolate their non-atom arguments in the usual way.

## Unsafe and async

```
fn unsafe_block$ -> i32:
    let x = 5
    let p = &x as *const i32
    unsafe:
        *p

#[tokio.main]
async fn main$:
    let listener =
        TcpListener.bind "127.0.0.1:8089"
            <- await
            <- unwrap$
```

- `unsafe:` opens a block like any other keyword.
- `.await` is written `<- await`, since it is a postfix on a value; it fits into a vertical chain like any method.

## Macros

### Invocation forms

```
println! "{} {}" a (twice~ 4)      →  println!("{} {}", a, twice!(4))      juxtaposed
let s = format! "value={a}"        →  let s = format!("value={a}")                 a single argument
let v = vec! [1, 2, 3]             →  let v = vec![1, 2, 3]                        bracket form, untouched
let z = vec! [0u8; 4]              →  let z = vec![0u8; 4]                         `;` form, untouched
let m1 = matches! a (1..=9)        →  let m1 = matches!(a, 1..=9)                   a pattern is one argument
let m2 = matches! a (n if n > 3)   →  let m2 = matches!(a, n if n > 3)              a guarded pattern, isolated
let m3 = matches! r (Ok 7)         →  let m3 = matches!(r, Ok(7))                   the pattern juxtaposes inside
assert_eq! a 5                     →  assert_eq!(a, 5)
```

- Macros follow the function rule exactly, with no exemption: one atom per argument, isolate anything that is not one. A pattern with a guard or an or-pattern is a multi-token argument like any other, so it is isolated — `matches! a (n if n > 3)` — and inside the isolating parens the ordinary rules apply, which is why `(Ok 7)` becomes `Ok(7)`.
- This is the reason Rust and Julia give macros the same syntax as the language, where C gives them a foreign one: a macro call written in Harsh is checked against Harsh's rules, and whether the result means what the macro wants is the macro's business in Rust. The transpiler removes the parens and commas and puts them back; it does not need to know what `matches!` expects.
- Rust call syntax written into a macro — `println!("{}", 1)` — is rejected, since under these rules it passes a single tuple. So is `matches! (a, n if n > 3)`, for the same reason.
- The bracket and brace forms are Rust's and pass through: `vec! [0; 4]`, `quote! { … }`; inside a brace body nothing juxtaposes. A macro with a body of its own is written `name~ do:` — see "Macro bodies" below.

### Definitions

**Procedural macros and foreign DSLs are still in development.** Declarative macros are finished — Harsh's own and Rust's both. A proc macro *defined* in Harsh transpiles to an ordinary Rust proc macro, and a call with simple arguments reaches it as ordinary Rust tokens, but this is not verified end to end; and a call whose arguments carry a top-level operator is read as an expression (`my_macro! a | b | c` emits `my_macro!(a) | b | c`), so isolate the tokens — `my_macro! (a | b | c)` — until it is settled. A DSL from someone else's crate works only where the shapes Harsh emits match what its parser wants; the general answer, a spelling map shipped by the crate, is designed and not built.

A `macro_rules!` — with Rust's `!` — is a zone of Rust: the transpiler copies it out byte for byte and `hrs-from` copies one in the same way. It carries no Harsh opener, since its `{` … `}` delimit it and its body is laid out as Rust, not as Harsh. Its calls are still Harsh calls (`my_vec! 1 2 3` → `my_vec!(1, 2, 3)`); only the definition is foreign.

Both sides of a `macro_rules~` are written in Harsh, and both follow rules already stated: a matcher is a parameter list, a transcriber is a block. The macro unfolds **into Harsh** before anything is transpiled, so the generated Rust holds no macro at all — only what the expansion came to.

```
macro_rules~ twice                                   fn main() {
    (($e:expr)) => do:                                   let v = {
        $e * 2                                               let mut tmp = Vec::new();
                                                             tmp.push(1);
macro_rules~ my_vec                                          tmp.push(2);
    ( $( ($x:expr) )* ) => do:                               tmp.push(3);
        do:                                                  tmp
            let mut tmp = Vec.new$                       };
            $(tmp <- push $x)*
            tmp                                          println!("{}", 21 * 2)
                                                     }
fn main$:
    let v = my_vec~ 1 2 3
    println! "{}" (twice~ 21)
```

- `macro_rules~ name:` opens a block whose lines are arms; each arm ends where the next begins, and the `;` Rust wants between them comes from the layout. A written `;` or `,` after an arm is rejected, as a written `,` after a match arm is.
- **The matcher is a parameter list.** A parenthesised fragment `($x:expr)` is one parameter; two or more are all grouped, `( ($a:expr) ($b:expr) )` → `( $a:expr, $b:expr )`; a repetition of groups is a comma-separated repetition, `( $( ($k:expr => $v:expr) )* )` → `( $( $k:expr => $v:expr ),* )`. It is the rule for `fn f (a: T) (b: U)`, applied a third time, and it is what makes the call `m~ a b` and the matcher meet in Rust. The mapping applies to a matcher with a group at its top level; `( $x:expr )` and a `tt` matcher `( $( $arg:tt )* )` are the same in both languages, since `tt` matches anything, the commas a Harsh call produces included. A bracketed or braced matcher — `[ $elem:expr ; $n:expr ]` — is Rust's, exactly as `vec! [0u8; 4]` is on the call side.
- **The transcriber is a block**, `=> do:` with the body beneath or `=> do: expr` inline. Its lines follow the block rules: statements take `;`, a line with a top-level `=>` is an arm and takes `,`, and `$x` is an atom, so `push $x` is a call. A transcriber's braces are Rust's *delimiters*, not a block: an expansion that is several statements with a value writes its block, the inner `do:` above, as Rust writes `{ { … } }`. A transcriber may also be Rust's delimited group, `=> { … }`, for a DSL of its own (`quote!`'s output, say); that form is for such transcribers, not a place to write Rust's `if` and `match`. A bare `=> $e * 2` is an error naming the forms.
- **A repetition** `$( … )*` that is the whole of its line repeats what its block holds — statements, each with its `;` — and may span lines: `$(` line-final, the body beneath, `)*` as its own line. One inside an expression follows the matcher's brick from the other side: `$( ($x) )*` is a list of arguments and becomes `$($x),*`; a repetition of bare tokens, `$($arg)*`, forwards `tt`s and is copied as it stands. A written separator, `$( … );*`, is an error.

### Macro bodies

A macro invoked with a body — `tokio.select!`, a framework's `view!` or `rsx!` — is written `name~ do:` with the body beneath, and is emitted `name~ { … }`. What the body is depends on the shape of its first line.

```
let winner = tokio.select! do:           let winner = tokio::select! {
    n = slow "slow" => n                      n = slow("slow") => n,
    n = fast "fast" => n                      n = fast("fast") => n,
                                          };
```

- **A Harsh block** is the default: statements and arms, as in a transcriber. `select!`'s arms are lines with `=>`.
- **Markup**, when the body is markup — its first tag may follow a hole or a string, `{panel}` / `<button …>` (Leptos, Yew, Sycamore): the lines are copied through as written — tags, attributes, quoted text, whitespace — with only the token substitutions applied (a `.` in a component path is a path separator), and only the Rust in it is Harsh: the contents of every `{ … }` — a child `{count}`, an attribute value `on:click={move |_| …}`. **Braces are the spelling**, the same as a Dioxus tree's holes; they are RSX's own block form and reach the Rust. An attribute value may also be isolated in parens, `on:click=(|| body)` — parens are grouping and don't change the nature of what they hold — and those are not emitted, so the bare `on:click=|| body` comes out; accepted, not preferred. A hole may span lines and holds layout, `{move |_|:` with the body beneath and `}` closing it. The same holds in the one-line brace form, `view! { <p>{count}</p> }`, inside a closure's braces or anywhere else.

```
view! do:
    <div class="app">
        <p>{count}</p>
        <button on:click={move |_| set_count <- update (|n| *n += 1)}>"+"</button>
        <Greeting name={"World" <- to_string$} />
    </div>
```

- **A brace tree**, when the first line is a name followed by `:` (Dioxus): `div:` opens an element and its block is a tree again; an attribute takes its value through `=`, since `:` opens blocks here, and is emitted `class: "app",`; a string, a `{ … }` or a `..spread` is a child; `for … in …:`, `if …:` and `else:` are the framework's own, their headers copied with the token substitutions and their bodies trees. As in markup, the Harsh is in the holes. Only attributes take commas, which is what the framework's parser accepts; an attribute's hole loses its braces on emission, so the framework sees the `move |_|` that marks an event handler.

```
rsx! do:                                              rsx! {
    div:                                                  div {
        class = "app"                                         class: "app",
        onclick = {move |_| count <- set (count$ + 1)}        onclick: move |_| count.set(count() + 1),
        "Hello {count}"                                       "Hello {count}"
        for item in items:                                    for item in items {
            li: "{item}"                                          li { "{item}" }
        Button:                                               }
            onclick = {move |_|:                              Button {
                let n = count * 2                                 onclick: move |_| {
                reset n                                               let n = count * 2;
            }                                                         reset(n)
            "Reset"                                               },
                                                                  "Reset"
                                                              }
                                                          }
                                                      }
```

- **Braces**, `name~ { … }`, are the one-line form, as braces are everywhere: the body is Harsh, layout off, and it juxtaposes — `quote! { fn #name$ -> u32 { #body } }` emits `fn #name() -> u32 { #body }`, and `tokio.select! { n = slow "slow" => n, }` the call it names. There is no exemption: `f(x)` is an error inside a macro's braces as anywhere.

## Attributes and comments

An attribute is an application inside its brackets, exactly as a macro call is after its bang: a name, then its arguments one atom each, parens only to isolate what is not one atom. Dots are path separators inside it as everywhere.

```
#[derive Debug Clone]                              #[derive(Debug, Clone)]
#[cfg (feature = "hydrate")]                       #[cfg(feature = "hydrate")]
#[cfg (not (feature = "ssr"))]                     #[cfg(not(feature = "ssr"))]
#[cfg_attr (feature = "ssr") (derive Serialize)]     #[cfg_attr(feature = "ssr", derive(Serialize))]
#[serde (rename = "id") default]                   #[serde(rename = "id", default)]
#[tokio.main (flavor = "current_thread")]          #[tokio::main(flavor = "current_thread")]
#[cfg test]                                        #[cfg(test)]
#[doc = "a single token is one argument"]           #[doc = "a single token is one argument"]
```

- Attribute lines never take a separator and never count as a block's tail expression.
- Rust's `#[derive Debug Clone]` is an error, as `f(x)` is anywhere: see "One syntax for applying".
- Doc comments on items and on fields both survive into the Rust, so `cargo doc` documents your `.hrs` source.

```
/// A doc comment survives into the Rust, so rustdoc sees it.
#[derive Debug Default]
struct Config
    /// Field docs too.
    verbose: bool
```

## One syntax for applying

Harsh has one way to apply a name to arguments, and it is juxtaposition. A parenthesised group after a name isolates one argument and is **separated from the name by a space**; when the argument is a single token the group is not needed. Written tight, `f(x)` reads as Rust's call, and Harsh has no call syntax — so it is an error, everywhere:

```fragment
f (a + b)   /   f x                 f(a + b)   /   f(x)
Some (a + b)   /   Some n           patterns and constructors alike
m~ (a + b)   /   m~ a               macros
#[cfg (feature = "x")]              attributes
fn f (a: T) (b: U)                  declarations
Circle (f64)                        a variant's payload — the group is Rust's payload list and keeps its commas
```

The parenthesised types follow the same rule. `Fn`, `FnMut`, `FnOnce` and a `fn` in type position are applications, and `$` applies to nothing there as everywhere, so that `()` is only ever the unit value:

```
type A = fn (x: i32) (y: f64) -> &str       type A = fn(x: i32, y: f64) -> &str;
type B = Box<dyn Fn i32 -> i32>              type B = Box<dyn Fn(i32) -> i32>;
type C = Box<dyn Fn (i32) (f64) -> i32>      type C = Box<dyn Fn(i32, f64) -> i32>;
type D = Box<dyn FnOnce$ + Send>             type D = Box<dyn FnOnce() + Send>;
```

**Documented parameters.** A parameter that carries a doc comment or an attribute — a Leptos component's props — is written with its doc lines above its group and its attributes inside it, one group per line, the return type on the line after. The Rust that comes out is the one `#[component]` expects, the `(` opening before the first parameter's doc:

```text
#[component]
pub fn ServiceCard
    /// The service to render.
    (service: Service)
    /// Stagger for the entry animation, in seconds.
    (#[prop (default = 0.0)] delay: f32)
    -> impl IntoView:
    …
```

**A chain hanging off a block expression** — `if … {} else {}.into_iter()` — is isolated in a group, the `(` on its own line, the chain continuing after the `)`; the parens are mandatory grouping (the chain follows them) and reach the Rust:

```text
let brand_name =
    (
        if s <- brand_lines <- is_empty$:
            vec! [s <- name <- clone$]
        else:
            s <- brand_lines <- clone$
    ) <- into_iter$ <- map (…) <- collect_view$
```

A paren block's tail may itself open the statement's block — `if xs <- iter$ <- any (|x|:` / body / `):` / the `if`'s body — and paren blocks nest.

A foreign block is a layout block, its bodiless functions taking their `;` from the layout:

```
extern "C":                         extern "C" {
    fn abs (input: i32) -> i32          fn abs(input: i32) -> i32;
                                    }
```

**Parens do three things, and the transpiler tells them apart by what they hold.** A group with a top-level comma is a tuple — a construct, kept. A group that shares its expression with other tokens raises precedence — `(a + b) * c`, `(a + b) <- abs$` — and is kept, since the transpiler cannot tell precedence from habit and must not guess. A group that is the whole of a statement or the whole of a value after `=` or `=>` encloses nothing it could bind tighter than: it is optional grouping and is not emitted, so `let s = (1 + 2)` is `let s = 1 + 2;` and `quote! do: (fn #name$ -> u32 do: #body)` emits the same tokens as without the parens — which matters inside `quote!`, where the tokens are the value. A macro's brace body is Harsh in one line like any other brace: `quote! { fn #name$ -> u32 { #body } }`.

## Everything else is Rust

Because the transpiler substitutes tokens rather than reconstructing a tree, the following need no support and simply work: lifetimes and lifetime bounds, `where` clauses, generic constraints, `impl Trait` in argument and return position, closures, iterator chains, `async` and `.await`, the `?` operator, all operators and their precedence, all patterns, ranges, slices, tuples, references and raw pointers, `unsafe`, macro invocations of every kind, raw strings, byte strings, and numeric suffixes.

## Not supported

- **Mixing inline and multi-line `else`.** An inline block closes at the end of its line, so `else: if …` on one line followed by `else:` on the next orphans the second. Both the fully inline chain and the fully indented chain work; the mixed form is rejected with an error.

## Not yet supported


## Known limits

- **`hrs-from`** produces Harsh in the recommended layout (its output goes through `hrs fmt`), but without pipes: a chain stays a chain.

## What is rejected

- Each of these is a syntax error with a message naming the fix.

```fragment
let x = a::b                           `::` is not valid in Harsh; the path separator is `.`
println!("{}", 1)                      a macro takes juxtaposed arguments, not a parenthesised list
matches! (a, n if n > 3)               the same — a parenthesised list is a tuple
match n:                               `;` is not valid at the end of a match arm
    1 => 10;
let a = x |> f y                       after a pipe's function only `<|` may follow; isolate: `x |> (f a)`
let a = 1 + 2 |> f                     a pipe's arguments are atoms; isolate an expression in parentheses
let a = 1 2 3 4 |> sub                 `sub` takes 3 parameter(s) and 4 were piped in
let a = f $                            `$` applies a name to nothing and is written tight against it: `f$`
fn f ():                               `()` is the unit value; a function with no parameters is `fn f$`
fn f (a: i32, b: i32) -> i32:          a parameter group holds one parameter; write `(a: T) (b: U)`
use std.io:                            `use` does not open a block; group imports in parentheses
match n:                               `,` is not written at the end of a match arm
    1 => 10,
if n == 1: 10                          this `else` has no `if` to attach to
else: if n == 2: 20
else: 30
let a = 1;                             `;` is only written after a block's last statement
let b = 2
v <- map |x|:                          a chain cannot continue after a bare closure block;
    x * 2                              isolate the closure in parens
<- count$
```

## Command line

```
hrs input.hrs -o out/main.rs --map out/main.map.json
cargo build --message-format=json | hrs-remap --map out/main.map.json
hrs-from existing.rs -o existing.hrs
```

`hrs-from` converts Rust source to Harsh, for bringing existing code in. It matches braces from the lexer's bracket depth, so nesting is exact; the only judgement is whether each `{` becomes indentation, and when that is not clear-cut the braces are left alone, which is always valid Harsh.

The source map records one entry per token as byte offsets. The remapper rewrites every span in every diagnostic and sub-diagnostic, then re-renders against the original source, so errors report the correct line and column in your `.hrs` file rather than in the generated Rust.

## Quick reference

| Construct | hrs | Rust |
|---|---|---|
| Block | `:` then indent | `{ … }` |
| Match arm block | `=>` then indent | `=> { … }` |
| Path separator | `.` | `::` |
| Field or method | `<-` | `.` |
| Use group | `.( … )` | `::{ … }` |
| Generic call | `Vec<i32>.new()` | `Vec::<i32>::new()` |
| Tuple index | `t.0` | `t.0` |
| Range | `0..3`, `1..=5` | same |
| Bare block | `do:` | `{ … }` |
| Struct literal | `Point { x: 1 }` | same |
| Curried parameters | `(a: T) (b: U)` | `(a: T, b: U)` |
| Bare parameter | `name: &str` | `(name: &str)` |
| Application | `f a b` | `f(a, b)` |
| Tuple argument | `f (a, b)` | `f((a, b))` |
| Pipe back | `f <\| x` | `f(x)` |
| Pipe forward | `x \|> f` | `f(x)` |
| Pipe, several values | `3 5 10 \|> sub` | `sub(3, 5, 10)` |
| Partial, from the left | `3 \|> sub` | `move \|p1, p2\| sub(3, p1, p2)` |
| Partial, from the right | `sub <\| 3` | `move \|p1, p2\| sub(p1, p2, 3)` |
| Partial, middle hole | `1 \|> sub <\| 3` | `move \|p1\| sub(1, p1, 3)` |
| Pipe a result | `(f a) \|> g` | `g(f(a))` |
| Inline block | `if t: 1 else: 2` | `if t { 1 } else { 2 }` |
| Trailing closure | `map \|n\|:` + indent | `map(\|n\| { … })` |
| Statement end | newline | `;` |
| Discard tail value | explicit `;` | `;` |

### Extended quick reference

| Construct | Harsh | Rust |
|---|---|---|
| Bare parameter | `fn f name: &str` | `fn f (name: &str)` |
| Curried parameters | `fn f (a: T) (b: U)` | `fn f (a: T, b: U)` |
| No parameters | `fn f$:` | `fn f() {` |
| Zero-argument call | `g$` | `g()` |
| Unit argument | `g ()` | `g(())` |
| Group as head | `(add 10) 7` | `(add(10))(7)` |
| If-let | `if let Some n = v: …` | `if let Some(n) = v { … }` |
| Let-else | `let Some f = g$ else: return` | `let Some(f) = g() else { return }` |
| While-let | `while let Some t = s <- pop$: …` | `while let Some(t) = s.pop() { … }` |
| Loop value | `let x = loop: … break v` | `let x = loop { … break v }` |
| Label | `'outer: for …:` | `'outer: for … {` |
| Await | `f$ <- await` | `f().await` |
| Try | `f n ?` | `f(n)?` |
| Unsafe | `unsafe:` | `unsafe {` |
| Type alias | `type P = (i32, i32)` | `type P = (i32, i32);` |
| Tuple struct | `struct T (i32, i32)` / `T 3 4` | `struct T(i32, i32);` / `T(3, 4)` |
| Record variant | `Rec:` + fields | `Rec { … }` |
| Macro, bracket form | `vec! [0; 4]` | `vec![0; 4]` |
| Macro definition | `macro_rules~ m:` | `macro_rules~ m {` |
| Pipe into closure | `x \|> (\|w\| f w)` | `(\|w\| f(w))(x)` |
| Isolated closure, chained | `map (\|x\|:` + body + `) <- f$` | `map(\|x\| { … }).f()` |
| Block opener synonym | `fn f$ do:` | `fn f() {` |
| Tuple index argument | `f t.0 t.1` | `f(t.0, t.1)` |
| Index argument | `f a[i]` | `f(a[i])` |
| Array argument | `f ([1, 2])` | `f([1, 2])` |
