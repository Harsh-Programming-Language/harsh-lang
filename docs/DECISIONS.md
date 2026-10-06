# Harsh — settled decisions

The language decisions behind Harsh, each with its reason and its edge cases.
They are settled: a question answered here is not reopened. When something in
Harsh surprises you, look here first; when hand-written Harsh fails, suspect
the writing before the transpiler — check it against these rules, the guide
and the changelog.

Every Harsh example below transpiles with the current `hrs`.

## How Harsh maps to Rust

Harsh is a mapping of Rust's syntax, never a guess about it. Each Rust form
has one Harsh spelling; `hrs` translates token for token where the two
languages agree, and rustc judges the result. Harsh refuses a form only when
it can know, without types, that the form is wrong.

## Paths and generics: `.` is `::`, `.<` is `::<`

Rust's `::` is Harsh's `.` everywhere, the turbofish included. Rust's `<` is
Harsh's `<`. Nothing is inserted and nothing is guessed (decided 2026-10-03
and 2026-10-04; built in 0.4.0 and 0.5.0).

```rust harsh
let v = Vec.<u32>.new$                    // Vec::<u32>::new()
let n = std.mem.size_of.<u64>$            // std::mem::size_of::<u64>()
let x = "42" <- parse.<i32>$ <- unwrap$   // "42".parse::<i32>().unwrap()
let s = v <- iter$ <- sum.<u32>$          // v.iter().sum::<u32>()
```

A `<` opens a generic list in exactly these places, and is a comparison
everywhere else:

| Place | Example |
| --- | --- |
| A definition's header | `fn first<T> (v: &[T]) -> &T:`, `struct Pair<T>` |
| A type, after an annotation's `:` | `let v: Vec<Vec<u8>> = Vec.new$` |
| A type, after `->` | `fn f$ -> Option<u8>:` |
| A type, after `as` | `x as usize` |
| `impl … for` and `where` bounds | `impl From<u8> for Celsius` |
| After a `.` — the turbofish | `Vec.<u32>.new$` |
| Opening a qualified path | `<Celsius as Default>.default$` |

A type position runs up to the next boundary: `=`, `)`, `,`, `|`, `]` or
the block opener. A comparison cannot cross a boundary to borrow a `>`.

Edge cases:

- **`Vec<u32>.new$`** — the dot is missing. It is mapped as written, Rust's
  `Vec<u32>::new()`, and rustc rejects it. Write `Vec.<u32>.new$`.
- **`>>` in a type** closes two lists: `let v: Vec<Vec<u8>> = Vec.new$`. In an
  expression it is a shift: `let half = n >> 1`.
- **`x as usize < y`** — after `as` comes a type, so Rust reads `usize<y` as
  the start of a generic list and rejects the line. Harsh agrees with Rust:
  write `(x as usize) < y`.
- **A comparison beside a turbofish** — `v <- len$ < size_of.<u64>$` holds
  both, and each is read by its own rule.

## The arrow `<-`: a space on each side, a name after it

`<-` reaches into a value: a field or a method. It is Rust's `.` (decided
2026-10-04; built in 0.5.0 and 0.5.1).

```rust harsh
let n = words <- len$                     // words.len()
let first = pair.0                        // a tuple's item: a dot, not `<-`
let low = n < -1                          // a comparison with a negative
```

Two rules, both refused when broken:

1. **A space on each side:** `value <- method`. Written tight, `x<-y` reads as
   the comparison `x < -y` misspelt — and it compiled silently as the field
   `x.y` when `x` had a field `y` of the comparison's type.
2. **A name after it:** a field or a method, never a number, a negative or a
   bracket. A tuple's item is `t.0`, with a dot.

| Written | Read as |
| --- | --- |
| `x<3`, `x < 3` | a comparison |
| `x< -3`, `x < -3` | a comparison with a negative |
| `x <- len$` | a method |
| `x<-3`, `x<-y`, `x <-y`, `x<- y` | refused: no space around `<-` |
| `x <- 3`, `x <- -3`, `x <- (y)` | refused: no name after `<-` |

Rust lexes `<-` as one token too, which is why Rust itself writes `x < -3`
with a space.

## Statements and blocks

- **No `;` ends a line** (0.2.0). A block whose last value is to be discarded
  ends with a line `()`:

```rust harsh
for w in words:
    seen <- insert w
    ()
```

- **An inline block holds one expression.** Several statements on one line
  are Rust's braces, kept on purpose:

```rust harsh
let a = if c { f$; g$ } else { h$ }
let b = { let u = 3; u * u }
```

- **No mixed `if`** (2026-10-04): an inline branch takes an inline `else`,
  and a block `else` takes a block `if`. `if c: a else:` with the `else` block
  beneath is refused.

```rust harsh
let y = if n > 0: "pos" else: "neg"
let z =
    if n > 0:
        "pos"
    else:
        "neg"
```

- **`:` and `do:` both open a block.** A choice of style: `:` is compact,
  `do:` states the intent.
- **A body sits deeper than its header;** beyond that its depth is free. Under
  a signature broken over several lines, 4 or 8 both read clearly. `hrs fmt`
  has one standard layout and normalises it — an author who wants their own
  layout does not run the formatter.

## Application and isolation

Arguments are juxtaposed; parentheses isolate any argument that is not an
atom. Parentheses are Harsh's one isolation tool.

**The reason, which explains every case** (decided 2026-10-05): Harsh
separates arguments with spaces, so an argument of several tokens would read
as several arguments. Where spaces separate, a multi-token argument is
isolated; where nothing needs separating, nothing is. Taught whole in the
Book, §2.7 *One argument, one atom*.

```rust harsh
let s = add (n * 2) (-1)                  // add(n * 2, -1)
let r = show pair.0 pair.1                // a tuple's item is an atom
let q = f ({ let u = 3; u * u })          // a block passed as an argument
```

- **A tuple index applies to one atom.** To index a tuple produced by several
  tokens, isolate them: `(f src).0`, `(p <- pair$).1`. An atom needs nothing:
  `t.0`, `v[0].1`, `f$.0`. In `f src.0` the `.0` is `src`'s. As an argument,
  the indexed group is isolated in turn, `show ((f src).0)`; `show (f src).0`
  is refused, since its index would apply to `show`'s result (2026-10-06). The Rust drops
  the isolation around a call, `f(src).0`, and keeps it where Rust needs it,
  `(a + b).0` (decided 2026-10-05).
- **A lone parameter may drop its parentheses**, since nothing needs
  separating: `fn double n: i32 -> i32: n * 2`. Two or more are each isolated:
  `fn add (a: i32) (b: i32) -> i32: a + b`.
- **After a constructor, a single argument needs no parentheses**:
  `Some &x`, `Some &mut x`, `Some mut line`, `Some ref x`, in patterns and
  expressions alike. A constructor is a `struct` or enum variant the files
  declare, or the prelude's `Some`, `Ok`, `Err` -- known from the
  declarations, never from a name's case (Rust does not care how a name is
  spelt). After any other name `&` is bit-and, so a reference argument is
  isolated: `f (&x)`, `read_line (&mut buf)`. Several arguments are each
  isolated: `Rgb (mut r) g b` (decided 2026-10-05).
- **A leading `-` is isolated, everywhere**: `f (-1)`, `Celsius (-40)` --
  `f -1` is subtraction, and a pattern follows the same rule (decided
  2026-10-05).
- **A range of atoms is an atom**: `f 0..3` is `f(0..3)`, `g 0..=n`. Ends
  that are not atoms leave `..` an operator: `f (a + 1..n)`,
  `Celsius (-40..=0)`. Spacing never matters, so `f ..n` is the range from
  `f` to `n`; a range with no start, as an argument, is isolated: `h (..n)`
  (decided 2026-10-05; it changed `f 0..3` from `f(0)..3`).
- **Commas separate as spaces do**: a literal inside a tuple is isolated,
  `(1, (P\ x = 1, y = 2))`, or its fields would read as the tuple's items.
- **A block passed as an argument is isolated**, `f ({ … })`. Braces right
  after a name, `f { … }`, are refused: Rust reads them as a struct literal.
  An empty record keeps its braces, `Empty {}`.

## Struct literals: `P\` only

```rust harsh
let p = Point\ x = 1.0, y = 2.0          // inline
let q =
    Point\
        x = 1.0
        y = 2.0                            // one field per line
let r = Line\ from = (Point\ x = 0.0, y = 0.0), to = q   // nested: isolated
```

A pattern takes the same mark, `P\ x, y`, with a rename keeping Rust's `:`,
`P\ x: px, ..`; nested in another pattern it is isolated like a nested
literal:

```rust harsh
let Point\ x, y = p
for (i, (Point\ x, ..)) in points <- iter$ <- enumerate$:
    println! "{i}: {x}"
let xs: Vec<f64> = points <- iter$ <- map (|(Point\ x, ..)| *x) <- collect$
```

There is one spelling for each: a braced literal and a braced pattern are both
refused (the pattern since 0.6.0 — it had been "still accepted" in some places
and refused in others, ruled 2026-10-04). An empty record keeps its braces,
`Empty {}`.

Fields take `=`, so a literal can never borrow a `>` or a `:` from around it.
A brace form `P { x = 1 }` existed early and was dropped, and considered again
on 2026-10-04 and left dropped: a second spelling, a near-twin of Rust's
`P { x: 1 }`, and a third meaning for braces. `P { x: 1 }` is refused with the
`P\` forms in the message.

## Macros: three rules

1. **A Harsh macro's stream is Harsh.** A `~` macro is written in Harsh and
   expanded by `hrs`.
2. **A Rust macro's own language is Rust; Harsh goes in holes.** A `!` macro
   that takes a language of its own takes it in braces, as its documentation
   shows; Harsh reads none of it. Wherever that language takes Rust code, a
   hole `@: … :@` holds the Harsh for it — a value, a child, a format slot
   inside one of its strings. One rule; no positions to learn.

```rust harsh
rsx! {
    button {
        onclick: @: move |_| n <- set 0 :@,
        "reset"
    }
    strong { {@: n$ :@} }
    p { "count: {@: n$ :@}" }
}
```

3. **The one exception: a list of values is juxtaposed.** When a Rust
   macro's stream is just comma-separated expressions, it is written by
   Harsh's rule for application: `println! "{} {}" a (b + 1)`, `vec! 1 2 3`.

**A hole of several lines** (B2 and B7, decided 2026-10-04): three layouts
are accepted and mean the same — the code on the next line after `@:`, the
code on the `@:` line with `:@` on its own line, and the code on the `@:`
line with `:@` ending the last. Lines beneath align with the code as
anywhere in Harsh (`else:` under `if`); a closure's body under
`onclick: @: move |_|:` is indented from that line. `hrs fmt` writes the last
layout, one space inside each mark and none between a mark and the DSL's
brace — `{@: n$ + 1 :@}` — except that a `:@` stays on its own line when the
line above ends in a `//` comment. Editors' Enter and Tab follow the same
columns.

**What a hole is** (decided 2026-10-06): a place where one Rust expression is
written in Harsh. It is not a block: it adds nothing around its Rust — no
braces, no `;`. Whatever the macro's syntax needs around it — a `;` between
`thread_local!`'s items, a `,` between attributes, braces — is the macro's
text, written outside the hole as its documentation shows. Several statements
in a hole's place are an explicit block, as anywhere a value is expected:
`@: { let v = Cell.new false; v } :@`.

**How the converter finds Rust in a macro's body** (decided 2026-10-06). What
looks like Rust in a macro is the macro's own language, written in Rust's
syntax; the converter cannot know its meaning, only its shape. Its rule, one a
reader can apply by eye:

1. A hole may start after a top-level `:` or `=`, inside a brace pair by
   itself, or in a format string's slot — where macros put values.
2. It ends at the segment's end: the next top-level `,` or `;`, the end of the
   line, or the closing bracket.
3. It covers the longest stretch from such a start to that end that Rust's own
   parser reads as one expression.

| Rust | Harsh |
| --- | --- |
| `static FLAG: Cell<bool> = Cell::new(false);` | `static FLAG: Cell<bool> = @: Cell.new false :@;` |
| `onclick: move \|_\| n.set(0),` | `onclick: @: move \|_\| n <- set 0 :@,` |
| `p { {n() * 2} }` | `p { {@: n$ * 2 :@} }` |
| `quick_error!`'s `from()`, `display("…", err)` | unchanged: no `:` or `=` before them |

In the first row, `Cell<bool> = Cell::new(false)` after the `:` is not an
expression, so the hole starts after the `=`. A hole whose Rust would not come
back identical is never written: the converter copies that piece verbatim, so
a macro's body always survives the round trip.

Braces delimit a macro's token stream; parentheses isolate Harsh. Inside a
stream, the braces belong to the macro: in `strong { {@: n$ :@} }` the inner
braces are Dioxus's own, and the hole only replaces the Rust inside them.

## Matrices: a view is a value

```rust harsh
let a = m~ [1.0 2.0 3.0; 4.0 5.0 6.0; 7.0 8.0 9.0]
let x = a[0, 1]                           // one element
let w = a <- view (1..) (..=1)            // a view: borrowed, nothing copied
let c = a <- slice (0..2) (..)            // a copy: a matrix of its own
```

`a <- view_mut rows cols` writes through, `a <- view_mut 0 (..) <- fill 0.0`.
A view holds a reference to its matrix and its window, so the borrow checker
guards it as it guards `&v[1..3]`. `&a[0..2, ..]` is refused (rustc's error):
Rust's `Index` must return a reference, and a window is not something stored
(decided 2026-09-25, built 0.1.37). `hrs_std` has no `unsafe` code, and since
0.6.4 forbids it.

## Projects: `Hrs.toml`, and three intents

- **`Hrs.toml`** sits beside `Cargo.toml` and lists Harsh's dependencies,
  `hrs_std` included; `Cargo.toml` keeps Rust's. Procedural-macro crates stay
  in `Cargo.toml`. `hrs add` writes to `Hrs.toml`, `cargo add` to
  `Cargo.toml` (0.3.0).
- **Harsh crates** exist to be a level above Rust: like Rust's non-macro
  crates, they provide code — types and functions — to the main crate.
- **Three intents**, by how the project is made and where it is published:

| | Published to Harsh's registry | Published to crates.io |
| --- | --- | --- |
| **A Harsh project** (`hrs new`) | #1: a Harsh crate | #2: a Rust crate written in Harsh |
| **A Rust project** (`cargo new`) | — | #3: Harsh files in a Rust crate |

The commands (decided 2026-10-04):

| Step | Command |
| --- | --- |
| A program | `hrs new app` — `src/main.hrs`, `[[bin]]` |
| A library | `hrs new --lib geometry` (or `hrs new geometry --lib`) — `src/lib.hrs` with one public function and a test, `[lib]` |
| #1, to Harsh's registry | `hrs build`, then `hrs publish` |
| #2, to crates.io | `hrs export`, `cargo build`, then `cargo publish` |
| #3, Harsh files in a Rust crate | `hrs src/a.hrs src/b.hrs`, `hrs --check …` in CI, `cargo publish` — settled |

- **#1 or #2 is chosen when publishing, not when creating:** the same library
  can go to Harsh's registry and, as Rust, to crates.io.
- **Until the registry opens, `hrs publish` checks the package and stops:**
  `name`, `version`, `description` and `license` present, and every Harsh
  dependency by version rather than by path — then a message naming the two
  ways to share today.
- **`hrs export` keeps a library's `[lib]`** and warns when `description` or
  `license` is missing, which crates.io requires.

### Two passes; the generated project is `target/src/` (decided 2026-10-06)

`hrs` builds in two passes. **Pass 1** reads `Hrs.toml` and the `.hrs` files
and writes an ordinary Rust project into `target/src/`, mirroring `src/`
(`src/server/db.hrs` is `target/src/server/db.rs`), with its own
`Cargo.toml`. **Pass 2** is Cargo, or any Rust tool, working there; it never
sees `Hrs.toml`, which is pass 1's alone.

- `Hrs.toml` lists Harsh's dependencies only; everything Cargo installs stays
  in `Cargo.toml`. A crate named in both is refused with a message.
- The project's top-level folders (`assets/`, `migrations/`, …; not `src`,
  `target` or hidden ones) are linked into `target/src/`, so paths rooted at
  `CARGO_MANIFEST_DIR` -- `asset!`, `include_str!(concat!(env!(…)))` -- find
  the real files.
- `hrs dx …` runs pass 1, then Dioxus's `dx` inside `target/src/`.
- Before 0.7.0 the folder was `target/hrs/`; `hrs build` refuses a
  `Cargo.toml` still pointing there, and `hrs migrate` updates it once.

## Layout of a list after `=` (decided 2026-10-06)

A list that does not fit after `=` goes on the lines beneath, bracket
included, one unit in -- as `hrs fmt` places any value that does not fit
after `=`:

```rust harsh
let events =
    [
        Event.PageLoad,
        Event.Click 20 80,
    ]
```

## Open for review

- **`a[i, j]` beside `a[(i, j)]`.** As built, `a[i, j]` reads one element of a
  matrix — Julia's spelling — and becomes Rust's `a[(i, j)]`; `a[(i, j)]` gives
  the same Rust. A matrix is built with `m~ […]`; `[i, j]` is an array. This
  was designed without a ruling and stays as is until reviewed.
