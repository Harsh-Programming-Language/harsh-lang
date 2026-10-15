# Macros in Harsh — design

Decided in conversation before any code, from a real test: five notebooks of Leptos, Axum and Actix notes converted with `hrs-from`, whose `view!` bodies the transpiler mangled. The principle that came out of it: **the macro rules are Harsh's rules, on both sides.** A Harsh call is one group per argument; a Harsh matcher is one group per fragment; a macro body is Harsh where it is expressions and markup where it is markup; and the bracketed and braced forms are Rust's, exactly as brackets and braces are everywhere else in the language. Macros do not produce Harsh — `hrs` transpiles the file, then rustc expands — but the programmer never has to know: both sides of every macro are written in Harsh and meet in Rust.

## The three rules

Every macro call in Harsh follows one of three rules, decided by who wrote the
macro and what its stream is.

**1. A Harsh macro's stream is Harsh.** A macro marked `~` is written in Harsh
and expanded by `hrs` before anything is transpiled: its call, its matcher and
its expansion are all Harsh, and nothing of it reaches the Rust but what it
expands to.

```rust harsh
macro_rules~ twice
    ( ($x:expr) ) => do: $x * 2

let n = twice~ (a + 1)
```

**2. A Rust macro's stream is Rust; Harsh goes in holes.** A macro marked `!`
that takes a language of its own -- a framework's markup, a query,
`select!`'s arms -- takes it in braces, written exactly as its documentation
shows. Harsh reads none of it. Wherever that language takes Rust code, a hole
`@: … :@` holds the Harsh for it, and is transpiled in place:

```rust harsh
rsx! {                                       rsx! {
    button {                                     button {
        onclick: @: move |_| n <- set 0 :@,          onclick: move |_| n.set(0),
        "reset"                                      "reset"
    }                                            }
    strong { "{@: n$ :@}" }                      strong { "{n()}" }
}                                            }
```

A hole of several lines may start its code on the `@:` line or the next, and end with `:@` on its last line or its own; lines beneath align with the code. `hrs fmt` writes `{@: if n > 2:` with `else:` under `if` and `"few" :@}` closing the last line.

**3. The one exception: a list of values is juxtaposed.** When a Rust macro's
stream is just comma-separated expressions -- `println!`, `vec!`,
`assert_eq!` -- it is written by Harsh's rule for application instead:
arguments juxtaposed, any argument that is not an atom isolated in
parentheses, no commas. This is the only place Harsh decides how a macro's
stream is written, so that calling a macro keeps Harsh's feel:

```rust harsh
println! "{} {}" a (b + 1)          println!("{} {}", a, b + 1);
let v = vec! 1 2 3                  let v = vec!(1, 2, 3);
```

## Where macros stand

Macros are organised by who wrote them, in what, and whether they are yours:

- **Harsh macros (~)** -- your own, written in Harsh. *Declarative*
  (`macro_rules~`), and *procedural*, mirroring Rust's (ruling 17): custom
  derive, attribute-like and function-like -- the Rust Book's three kinds,
  in its order.
- **Rust macros (!)** -- your own, written in Rust inside a Harsh project:
  still supported. *Declarative* (`macro_rules!`): a zone of Rust, copied
  verbatim. *Procedural*: a Rust proc-macro crate beside your Harsh crates,
  depended on and brought in with `use` like any crate.
- **Harsh DSLs (~)** -- imported from Harsh libraries: Harsh throughout; the
  library's guide is the reference. The prelude is the first such library.
- **Rust DSLs (!)** -- imported from Rust libraries: called by Harsh's rules;
  a stream that is a language of its own is written in braces, with Harsh in
  holes (ruling 16). A call whose arguments carry a top-level operator is read
  as an application ending there, `my_macro! a | b | c` → `my_macro!(a) | b |
  c`, as `f a | b` is `f(a) | b`; isolate the stream, `my_macro! (a | b | c)`,
  or give it braces, `my_macro! { a | b | c }`.

Parsing and building for Harsh's procedural macros: a Harsh counterpart to
`syn` and `quote` ships with `hrs`: `hrs_syn` and `hrs_quote`. A macro may
also read and write its tokens directly with `hrs_proc_macro`.

## Harsh macros (~)

Your own macros, written in Harsh, expanded by `hrs` into Harsh before anything is transpiled.

### Declarative macros

`macro_rules~`: Harsh layout, Harsh matchers, a Harsh transcriber, calls written `twice~ 4`. What sets it apart from Rust's `macro_rules!` is rule 0, under *Rust macros* below.

#### Rule 4 — matchers: a metavariable is written in its parentheses

The user's ruling, 2026-09-25:

1. **A metavariable is written `($name:spec)`.** Its parentheses are part of the construct, as `$( … )*`'s are the repetition's -- Harsh's parameter shape, `fn f (a: T)`, and it keeps every metavariable one shape, with no drift such as `($k:     expr => $v:  expr)`. A bare `$name:spec` is refused, in a repetition too, and the message names the form.
2. **Every other token in a matcher is literal**, matched as written: `=>`, a keyword, and any further parentheses.
3. **A specifier gives its capture meaning.** `:expr` means an expression, and in an expression parentheses are grouping, so `(1+2)`, `((1+2))` and `(((1+2)))` are one expression to `($v:expr)`. `:ident`, `:lifetime` and `:literal` are single tokens, as in Rust: `(adding)` is no `:ident`.
4. **The transcriber stays bare**: `$v $( $x )*`, parentheses only where the output needs them.

```rust harsh
macro_rules~ apply
    (($v:ident) $( ($x:expr) )*) => do:
        $v $( $x )*

apply~ adding 4 5            // adding(4, 5)
apply~ adding (4) (1 + 2)    // (4) and (1 + 2) are expressions
apply~ (adding) 4 5          // refused: (adding) is no :ident
```

The arm's own parentheses come first, so an arm whose whole matcher is one metavariable is written `(($e:expr)) => do:`. Literal parentheses in a matcher require literal parentheses in the call:

| Matcher | Call | |
|---|---|---|
| `($k:expr) => ($v:expr)` | `"key" => (3+4)` | matches |
| `(($k:expr)) => ($v:expr)` | `"key" => (3+4)` | no: the outer `( )` is literal |
| `(($k:expr)) => ($v:expr)` | `("key") => (3+4)` | matches |
| `(($k:expr)) => ($v:expr)` | `(("key")) => (3+4)` | matches |

A pair in a repetition, each in literal parentheses of its own:

```rust harsh
macro_rules~ hashmap
    ( $( (($k:expr) => ($v:expr)) )* ) => do:
        …
let m = hashmap~ ("a" => 1) ("b" => 2)
```

#### Rule 5 — matchers: the bracketed and braced sides are Rust's

A matcher in `[ … ]` or `{ … }` is Rust's syntax, as the stream of a `~` call is: `filled~ [0u8; 4]` reaches the macro as written, so `([ ($elem:expr) ; ($n:expr) ])` is how a macro takes a `;`-separated pair -- the arm's own parentheses, then the bracket, whose tokens are literal, the metavariables written in theirs (rule 4).

```rust harsh
macro_rules~ filled
    ([ ($elem:expr) ; ($n:expr) ]) => do:
        vec! { @: $elem :@; @: $n :@ }
let v = filled~ [0u8; 4]
```

### Procedural macros

A procedural macro is a program: a function from tokens to the code that replaces or accompanies them. Harsh's are written in Harsh, in a crate of their own, and run by `hrs` when it transpiles the code that calls them; they mirror Rust's three forms.

#### Custom derive Macros

A derive mirrors Rust's. It is registered with its name, and may declare
helper attributes it reads:

```rust harsh
#[proc_macro_derive~ Describe (attributes describe)]
pub fn describe (input: TokenStream) -> TokenStream:
    …
```

and is called above a `struct`, an `enum` or a `union`, beside Rust's own
derives if you like:

```rust harsh
#[derive Debug]
#[derive~ Describe]
struct Point
    x: i32
    #[describe skip]
    secret: i32
    y: i32
```

- **It receives** the item as written: its outer attributes and doc
  comments, its header and every line beneath it, with every derive
  attribute removed -- Rust's and Harsh's -- and its helpers present.
- **Its output is added after the item**, which it cannot change. Several
  derives run in the order written.
- **Its helpers leave the item** once it has run: rustc would not know
  them. A helper no derive on the item declared is left for rustc to refuse.
- **Errors** name the derive and point at its attribute: "the derive
  `Describe` failed: …", or rustc's error with "in the expansion of
  `#[derive~ Describe]`".

#### Attribute-Like Macros

An attribute-like macro is placed on any item, and what it returns
*replaces* the item. As in Rust it takes two streams, the attribute's
arguments and the item -- a `proc_macro_attribute` with one parameter is
refused by rustc ("attribute proc macro has incorrect signature", checked
2026-09-24), and `#[route]` without arguments gets an empty first stream.
Harsh's own: a `pub fn` marked `#[proc_macro_attribute~]` taking
`(attr: TokenStream) (item: TokenStream)` -- one stream is refused, as rustc
refuses it -- called `#[name~]` or `#[name~ arguments]` on any item:
`#[route~ GET "/"]`, the arguments written after the name as a Rust
attribute's are (the user, 2026-09-24), and handed to the macro as written,
`GET "/"`. What shape of stream a macro expects is its author's decision, as
with any `~` macro; Harsh imposes none -- written with Harsh in mind it will
usually be `#[route~ GET "/" (some_attr = some_value)]`. The macro receives
the item with all its other attributes, before and after its own (checked
with rustc), and its output replaces the item; attribute-like macros run
before derives. A *Rust* attribute macro is different: there `hrs`
transpiles the call, and `#[route GET "/" (some_attr = some_value)]` and
`#[route (GET) ("/") (some_attr = some_value)]` both become Rust's
`#[route(GET, "/", some_attr = some_value)]` (checked). The Book's 23.5 has
the Rust Book's `route` example, written in Harsh.

#### Function-Like Macros

A proc macro is a Harsh function from a stream to a stream. The crate that
defines it:

```toml
# hello/Cargo.toml
[package]
name = "hello"
version = "0.1.0"
edition = "2021"

[dependencies]
hrs_proc_macro = "0.2"

[package.metadata.harsh]
proc-macro = true
```

```rust harsh
// hello/src/lib.hrs
use hrs_proc_macro.TokenStream

#[proc_macro~]
pub fn hello_macro (input: TokenStream) -> TokenStream:
    let input_str = input <- to_string$
    let output = format! "\"Hello, {}!\"" input_str
    output <- parse$ <- unwrap$
```

The crate that uses it lists it by path:

```toml
# app/Cargo.toml
[package.metadata.harsh]
proc-macros = ["../hello"]
```

```rust harsh
// app/src/main.hrs
fn main$:
    println! "{}" (hello_macro~ world)        // prints Hello, world!
```

- **The stream** is the call's tokens as written, as text: the rest of the
  line and every line indented beneath it (the `~` extent rule), comments
  dropped, a `do:` opener stripped, each line's indentation relative to the
  least-indented one. `hello_macro~ world` receives `world`.
- **The expansion** is Harsh text. `hrs` reads it as Harsh and splices it
  where the call stood, at the call's column; it then transpiles like any
  Harsh you wrote. Text that does not read as Harsh is an error naming the
  macro and the call; so is a macro that returns an error or panics.
- **Names** a proc macro introduces are visible to the caller (no hygiene:
  a Rust proc macro built from text has none either).
- **Who wins a name:** a file's own `macro_rules~`, then the project's proc
  macros, then the prelude; `hrs_std.NAME~` is always the prelude's.
- **Where it lives:** `#[proc_macro~]` marks a `pub fn` in the crate's root,
  `src/lib.hrs`. A crate marked `proc-macro = true` registers at least one;
  an unmarked crate registers none.
- **How it runs:** `hrs build`, `hrs run` and `hrs expand` transpile the
  listed crates and build a small runner under `.hrs/proc-macros/`
  (cargo's own freshness check makes later builds a no-op), then run it
  once per call. Changing a macro retranspiles its callers.
- **Not yet:** the attribute form; tokens with spans; a proc-macro crate
  that itself uses Harsh proc macros; diagnostics from rustc inside the
  macro crate mapped to its `.hrs`; proc macros inside `@: … :@` holes and
  in single-file `hrs FILE -o OUT`.

### Re-exporting a Harsh macro

As in Rust, a library can hand its users a macro from its proc-macro crate, so
they depend on the library alone (the user's ruling, 2026-09-24, option (a)):

```toml
# hello_macro/Cargo.toml
[package.metadata.harsh]
proc-macros = ["../hello_macro_derive"]

# hello_macro/src/lib.hrs
pub trait HelloMacro
    fn hello_macro$

pub use hello_macro_derive.HelloMacro        # resolved by hrs; absent from the Rust

# app/Cargo.toml -- depends on hello_macro only
[dependencies]
hello_macro = { path = "../hello_macro" }

# app/src/main.hrs
use hello_macro.HelloMacro

#[derive~ HelloMacro]
struct Pancakes
```

`hrs` blanks every `use` naming a macro crate the project lists -- it has no
Rust meaning -- and when it builds `app`, reads its path dependencies for
`pub use <macro crate>.<Name>` (or `.{A, B}`) and adds those macros, and only
those, to `app`'s runner.

## Rust macros (!)

Your own macros, written in Rust inside a Harsh project: still supported, and called by the rules of *Rust DSLs* below.

### Declarative macros



#### Rule 0 — `macro_rules!` is Rust, `macro_rules~` is Harsh

Harsh has two declarative macro systems, and the mark says which. Everything
in the rules below is about **`macro_rules~`**, Harsh's own: Harsh layout,
Harsh matchers, a Harsh transcriber, and calls written `twice~ 4`.

**`macro_rules!` is a zone of Rust.** The author is writing Rust and knows
it, so the transpiler reads none of it and rewrites none of it: the
definition goes out byte for byte as it came in, and `hrs-from` copies a Rust
one into a Harsh file the same way, which makes the round trip exact.

```rust harsh
macro_rules! my_vec {                      // copied verbatim, both directions
    ( $( $x:expr ),* ) => {
        { let mut v = Vec::new(); $( v.push($x); )* v }
    };
}

fn main$:
    let v = my_vec! 1 2 3                  // the call is Harsh: my_vec!(1, 2, 3)
```

`macro_rules! name` opens the zone; the `{` that follows and its matching `}`
delimit it. There is no Harsh opener — no `:`, no `do:`, no `raw:` — on
purpose: an opener would make the body's indentation meaningful, and the body
is Rust, laid out however its author likes. A `}` inside a string, a char, a
raw string or a comment is text, not a brace, so `println!("}")` closes
nothing.

Only the *definition* is foreign. A call to such a macro is an ordinary Harsh
call and is written like one.

### Procedural macros

A Rust proc-macro crate -- `proc-macro = true` under `[lib]`, Rust's own setting -- sits beside your Harsh crates; a Harsh crate depends on it as on any crate and brings its macros in with `use`: a function-like one is called `name! args`, a derive `#[derive Name]`, an attribute `#[name]` or `#[name (args)]`. The Book's 23.6 has the Rust Book's `HelloMacro` derive, in Rust, used from Harsh.

#### Custom derive Macros

A function marked with Rust's `proc_macro_derive`, naming the derive, called `#[derive Name]` after `use crate_name.Name`; its output is added beside the item.

#### Attribute-Like Macros

A function marked with Rust's `proc_macro_attribute`, taking two streams -- the attribute's arguments and the item -- and returning what replaces the item, called `#[name]` or `#[name (args)]`.

#### Function-Like Macros

A function marked with Rust's `proc_macro`, called `name! args`, or `name! { … }` when its stream is a language of its own.

## Harsh DSLs (~)

Macros imported from Harsh libraries: Harsh throughout, each call following the grammar its library's guide describes. Nothing here is special beyond that guide.

### The prelude: `g~`, `list~`, `set~`, `dict~`, `m~`, `v~`

Four macros need no definition and no `use`. `g~` is a comprehension:

```rust harsh
let pairs: Vec<(i32, i32)> =
    (g~ (x, y)
        for x in 1..4 if x > 1
        for y in 1..4 if y != x
    ) <- collect$
```

Each `if` belongs to the `for` it follows, so a condition sees its own level's
name and every outer one. `g~` is lazy; `list~`, `set~` and `dict~ k => v for …`
collect it. A file that defines its own `g` shadows the prelude's, and
`hrs_std.g~` always reaches the prelude's.

`m~ [1 2; 3 4]` and `v~ [1, 2, 3]` are Julia's matrix and vector literals — a
space is `hcat`, `;` or a line break is `vcat`, a comma makes a vector's
entries, and every entry is a block. They build `hrs_std::Matrix` and
`hrs_std::Vector`, so a project that uses them lists `hrs_std = "0.1"` among
its dependencies; the Book's chapter 16 and the Guide's "Matrices and linear
algebra" cover them.

## Rust DSLs (!)

Macros imported from Rust libraries, from `println!` to a framework's `view!`, called by Harsh's rules.

### Calling a macro: the shapes

Nothing here is special to macros. A call is a header, and what follows it is
a block of the kind the header's mark names — the same kit as everywhere else
in the language, which is why the forms can be guessed rather than recalled.

```rust harsh
m! x y z                             m!(x, y, z)          juxtaposed arguments
m! (x) (y) (z)                       m!(x, y, z)          the same, isolated

m! (do:                              m!({                 a block as one argument,
    stmt                                 stmt;            isolated like any
    value                                value            argument
)                                    })
m! { stream }                        m! { stream }        braces: the macro's own
                                                          stream, as Rust expects it
```

**A macro is a function of one argument — a token stream — returning a token stream.** The syntax and layout of that argument are the DSL's; Harsh only says where it ends. So a Harsh macro call (`name~`) is a stream, not an application. Everything
after the mark reaches the matcher as written: the rest of the call's line and
every following line indented deeper than the line the call sits on -- whether
those lines continue the call or are the body of a `do:` block it opens -- or
to the close of the group the call sits in, whichever comes first. Nothing in the stream is Harsh's — a comma is the DSL's token, and a
`(…)` is a group, one token, so a tuple is written once. A call inside a larger
expression is isolated as any application is: `((twice~ 4), 0)`.

```rust harsh
let v =
    lst~ 1 2                 the stream is `1 2 3 4`: the deeper line is the call's
        3 4

let n =
    (lst~ 1 2                isolated: the `)` ends the stream, so `<- len$`
        3 4) <- len$         chains on the result, not on the stream
```

A `\` after the mark is a token like any other: a matcher that wants it names
it, `(\ $a:expr)`. A Rust macro whose stream is not a list of values takes it
in braces, as Rust writes it: `hm! { 1 => "a", 2 => "b" }`. (`hm!\` with the
entries beneath was the spelling until 2026-09-22; it is retired, and refused
with the braces named.)

```rust harsh
macro_rules~ pair
    (($a:expr) ($b:expr)) => do:             juxtaposed arguments -- the Harsh way
        ($a, $b)                         the comma is the tuple's, in the output

macro_rules~ list
    ($( ($x:expr) ),* $(,)?) => do:        a DSL that owns commas, trailing one optional
        [$( $x ),*]

pair~ (1+3) "more"                       $a = `(1+3)`, $b = `"more"`
list~ 1, 2, 3,                           [1, 2, 3]
```

A fragment's extent follows the matcher's shape: to the next literal the
matcher names or the repetition's separator; one atom when another fragment
follows directly (`$a:expr $b:expr` is juxtaposition); the rest of the stream
when nothing follows. The matcher's outer parens are its own delimiter; a
DSL's brackets go inside them, `([ $e:expr ; $n:expr ])` for `filled~ [0u8; 2]`.

A repetition's grammar is Rust's exactly, in the matcher and in the
transcriber: `$( … )` is followed by `*`, `+` or `?`, with a separator before
`*` or `+` if the DSL wants one and none before `?`. A `$( … )` with no marker
is refused; what must appear once is written without it. A trailing comma
that may be there is `$(,)?`; one that must be is the literal `,`.

A Harsh-thinking author juxtaposes a macro's arguments and writes commas only
where the DSL spells a construct that owns them — an array, a tuple, a
specification block inline. The trailing-separator idiom is the tool for
exactly those.

Which one a given macro wants is a fact about that macro's grammar, not about
Harsh. For a declarative macro it follows from the matcher and needs no
declaration; for a procedural macro, whose layout can be anything, a crate's
`dsl.hrs` is where the answer will live.

### Derives and attributes

Rust's attribute macros take Harsh between their brackets, by the application and path rules: `#[tokio.main]`, `#[route "/api/:id"]` → `#[route("/api/:id")]`, `#[tokio.main (flavor = "multi_thread")]` → `#[tokio::main(flavor = "multi_thread")]`.

A derive by juxtaposition, `#[derive Debug Serialize]`; a helper attribute the same way, its arguments isolated.

### Rule 1 — a Rust macro's brace body is Rust; its holes are Harsh

A Rust macro whose stream is not a list of values takes it in braces, and
the braces hold **Rust** -- or the macro's own language, written as its
documentation shows. Harsh sets the body aside before reading the file, and
puts it back byte for byte into the generated Rust. The only Harsh inside is
what the author marks: a **hole** opens at `@:` and **always closes** at
`:@`, and holds ordinary Harsh, transpiled in place.

```rust harsh
view! {                                   view! {
    <p>{count}</p>                            <p>{count}</p>
    <button on:click={@: move |_|             <button on:click={move |_|
        inc$ :@}>"+"</button>                     inc()}>"+"</button>
}                                         }

tokio.select! {                           tokio::select! {
    n = @: slow "slow" :@ => n,               n = slow("slow") => n,
}                                         };
```

- A hole's baseline is the indentation of the line it opens on: one line, or
  a block beneath `@:` with `:@` where it ends.
- A hole may hold a macro call with braces of its own, and holes in those.
- `@@:` is a literal `@:` in the body's text; `@:` outside a body is refused.
- The body may span lines, as in Rust; the formatter never touches it.
- **A hole is one expression, not a block** (decided 2026-10-06). It adds
  nothing around its Rust: no braces, no `;`. What the macro's syntax needs
  around it -- `thread_local!`'s `;` between items, an attribute's `,` -- is
  written outside, `static FLAG: Cell<bool> = @: Cell.new false :@;`.
  Several statements in a hole's place are an explicit block:
  `@: { let v = Cell.new false; v } :@`.
- **A hole's code is read as Harsh**, so a mark inside one of its strings,
  characters or comments is text: `{@: format! "a:@b" :@}` closes at the
  last `:@`.
- Holes have two writers: the author, writing Harsh, and `hrs-from`,
  converting Rust. What looks like Rust in a macro is the macro's language
  written in Rust's syntax, so the converter judges only shape, by a rule a
  reader can apply by eye:
  1. a hole may start after a top-level `:` or `=`, inside a brace pair by
     itself, in a format string's slot, in a paren splice among the DSL's
     text, after the DSL's own `for … in` or `if`, or in an arm's pattern,
     future or handler -- wherever the library's documentation writes a
     Rust expression (the table below);
  2. it ends at the segment's end -- the next top-level `,` or `;`, the end
     of the line, or the closing bracket;
  3. it covers the longest stretch from such a start that Rust's own parser
     reads as one expression;
  4. and it is a hole only when that stretch is more than a single word and
     calls on a binding or uses an operator (2026-10-12; the catalogue
     below has the rule and the binding table).

  So `static FLAG: Cell<bool> = Cell::new(false);` gets its hole after the
  `=` (`Cell<bool> = …` is no expression), `onclick: move |_| n.set(0),`
  after the `:`, and `quick_error!`'s clauses -- `from()`,
  `display("…", err)` -- none. Everything else is the macro's text, copied
  as written; a hole that would not come back identical stops the
  conversion, named by line, so a macro's body always survives the round
  trip or is not converted at all.
- A `~` call's stream is its macro's too, but holes have no meaning in it: a
  Harsh macro's fragments land in a Harsh transcriber.
- **A `~` macro's author expands to valid Harsh** (the user's rule,
  2026-09-23). If the expansion calls a Rust macro with a brace body, the
  author writes its holes, as `filled~` does: `vec! { @: $elem :@; @: $n :@ }`.
  The transpiler reads an expansion exactly as it reads a file; anything
  that is not valid Harsh is an error, never repaired.

*Retired 2026-09-23, with this rule's arrival:* the old rules 1 (a `do:`
body of markup, "HSX"), 2 (a macro `do:` body as a Harsh block), 3 (a brace
body as Harsh on one line) and 6 (a brace tree written with Harsh's layout),
and the `#:` block. `m! do:` and `#:` are refused with the braces named. They
put Harsh inside a macro's language by guessing its grammar; Harsh no longer
guesses.

### Where the holes go: the DSLs of popular libraries

**A hole is a Rust expression in a macro's body that calls on something bound
outside the DSL's own language -- a variable, a function, a name the DSL
binds -- or uses an operator, which is a function too (`+` is `Add::add`),
the access operators `.` and `::` included. A single word is exempt** (the
user's rule, 2026-10-12): `{count}`, `(title)` and `=> n` read the same in
both languages. So `user.id`, `Duration::ZERO`, `3 + 5`, `&x` and
`items.len()` are holes; `count`, `x` and `4` are not.

Where to look -- the places a library takes a Rust expression, string slots
first:

| Where | Seen in | In Harsh |
|---|---|---|
| a format slot in the DSL's strings | `rsx!` | `"Twice: {@: count$ * 2 :@}"` |
| a brace pair of its own, `{…}` | `view!`, Yew's `html!`, `rsx!` | `{@: *counter :@}` |
| a value after `name:` or `name =` | `rsx!`, `view!`, `json!`, `thread_local!`, `lazy_static!`, `bitflags!` | `disabled: @: count$ <= 0 :@` |
| a splice in parentheses, `(…)` | Maud's `html!` | `(@: items <- len$ :@)` |
| the DSL's control flow: `for … in ITER {`, `if COND {` | `rsx!`, Yew, Maud (`@for`, `@if`) | `if @: count$ > 3 :@ {` |
| an arm: pattern, future, handler | `select!` | `@: Some v :@ = @: rx <- recv$ :@ => @: v * 2 :@,` |

Who is bound, for `hrs-from`, is a table, never a guess from a name's case:
the standard prelude, the file's items, the names its `use`s import, the
bindings in scope at the macro call (the function's parameters, its `let`s,
`for`s and patterns), the names the DSL binds itself (Leptos's `let:item`,
`for item in …`, `select!`'s `Some(v)`), and an expression's own closure
parameters. A glob import, `use leptos::prelude::*`, brings names nobody can
list without the crate: a stretch built only from them -- Leptos's
`fallback=|| view! { … }` below, Yew's `classes!("btn", "big")` -- is left
as written, and `hrs-from` says so, naming the line. An author may still
write a hole there. Text the DSL owns stays as written however Rust-like it
looks: `quick_error!`'s `display("…", err)` binds nothing outside; but SQL's
`WHERE total = price * 2` becomes a hole, through its operator.

A macro whose stream is a list of values is no DSL at all: it is juxtaposed,
`vec! 1 2 3`.

Each example below is the library's own Rust, compiled against the crate
named, and the Harsh an author writes for it, which `hrs` turns back into
that Rust; `hrs-from` puts its holes in exactly the same places. All of them
are in the repository, `examples/dsl/`, and tested
(`tests/dsl_libraries.rs`).

#### Leptos `view!`

*leptos 0.8.* Markup. Rust goes in an attribute's value after `=` (`on:click=`, `when=`), in a brace pair of its own (`{move || …}`), and in a `{…}` value that holds a comparison -- in markup a bare `>` ends the tag, so Leptos itself wants `class:big={move || count.get() > 10}`. `{count}` and `{double}` are single words; `fallback=|| view! { … }` names only the glob's `view!`, so `hrs-from` leaves it as written and says so.

```rust harsh
use leptos.prelude.*

#[component]
fn Counter initial: i32 -> impl IntoView:
    let (count, set_count) = signal initial
    let double = move || count <- get$ * 2
    view! {
        <button
            on:click=@: move |_| set_count <- update (|n| *n += 1) :@
            class:big={@: move || count <- get$ > 10 :@}
        >
            "Clicked " {count} " times"
        </button>
        <p>"Double: " {double} " / next: " {@: move || count <- get$ + 1 :@}</p>
        <Show when=@: move || count <- get$ % 2 == 0 :@ fallback=|| view! { <p>"odd"</p> }>
            <p>"even"</p>
        </Show>
    }

fn main$:
    mount_to_body (|| view! { <Counter initial=3/> })
```

```rust
use leptos::prelude::*;

#[component]
fn Counter(initial: i32) -> impl IntoView {
    let (count, set_count) = signal(initial);
    let double = move || count.get() * 2;
    view! {
        <button
            on:click=move |_| set_count.update(|n| *n += 1)
            class:big={move || count.get() > 10}
        >
            "Clicked " {count} " times"
        </button>
        <p>"Double: " {double} " / next: " {move || count.get() + 1}</p>
        <Show when=move || count.get() % 2 == 0 fallback=|| view! { <p>"odd"</p> }>
            <p>"even"</p>
        </Show>
    }
}

fn main() {
    mount_to_body(|| view! { <Counter initial=3/> })
}
```

#### Dioxus `rsx!`

*dioxus 0.6.* A tree. Rust goes after an attribute's `:` (`onclick:`, `disabled:`, `class:`), in a string's format slot (`"Twice: {count() * 2}"`), and in the tree's own control flow, after `for … in` and `if`, up to the body's `{`.

```rust harsh
use dioxus.prelude.*

fn main$:
    dioxus.launch App
    ()

#[component]
fn App$ -> Element:
    let mut count = use_signal (|| 0)
    let items = vec! "apple" "pear"
    rsx! {
        h1 { "High-five counter: {count}" }
        button { onclick: @: move |_| count += 1 :@, "Up high!" }
        button { onclick: @: move |_| count -= 1 :@, disabled: @: count$ <= 0 :@, "Down low!" }
        p { "Twice: {@: count$ * 2 :@}" }
        for item in @: items <- iter$ :@ {
            li { key: "{item}", "{item}" }
        }
        if @: count$ > 3 :@ {
            p { class: @: if count$ > 10: "hot" else: "warm" :@, "Getting big" }
        }
    }
```

```rust
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut count = use_signal(|| 0);
    let items = vec!["apple", "pear"];
    rsx! {
        h1 { "High-five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, disabled: count() <= 0, "Down low!" }
        p { "Twice: {count() * 2}" }
        for item in items.iter() {
            li { key: "{item}", "{item}" }
        }
        if count() > 3 {
            p { class: if count() > 10 { "hot" } else { "warm" }, "Getting big" }
        }
    }
}
```

#### Yew `html!`

*yew 0.21.* Markup with Rust in braces: an attribute's `{…}` value, a child `{ … }`, Yew's `{ for ITER }` (its `for` is Yew's, the iterator Rust) and `if COND { … }`. `{onclick}` and `{ "+1" }` are a name and a literal: no hole.

```rust harsh
use yew.prelude.*

#[function_component App]
fn app$ -> Html:
    let counter = use_state (|| 0)
    let onclick = do:
        let counter = counter <- clone$
        move |_| counter <- set (*counter + 1)
    html! {
        <div class={@: classes! "app" ((*counter > 5) <- then (|| "big")) :@}>
            <button {onclick}>{ "+1" }</button>
            <p>{@: *counter :@}</p>
            <p>{@: format! "Twice: {}" (*counter * 2) :@}</p>
            { for @: (0..*counter) <- map (|i| html! { <span key={i}>{ i }</span> }) :@ }
            if @: *counter > 3 :@ { <p>{ "big" }</p> }
        </div>
    }

fn main$:
    yew.Renderer.<App>.new$ <- render$
    ()
```

```rust
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    let counter = use_state(|| 0);
    let onclick = {
        let counter = counter.clone();
        move |_| counter.set(*counter + 1)
    };
    html! {
        <div class={classes!("app", (*counter > 5).then(|| "big"))}>
            <button {onclick}>{ "+1" }</button>
            <p>{ *counter }</p>
            <p>{ format!("Twice: {}", *counter * 2) }</p>
            { for (0..*counter).map(|i| html! { <span key={i}>{ i }</span> }) }
            if *counter > 3 { <p>{ "big" }</p> }
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
```

#### Maud `html!`

*maud 0.27.* Splices. Rust goes in a paren group among the text, `(items.len())`, as an attribute's value, `href=(…)` -- the parens are Maud's, the hole inside them -- and after Maud's `@for … in` and `@if`.

```rust harsh
use maud.(html, Markup)

fn page (title: &str) (items: &[&str]) -> Markup:
    html! {
        h1 { (title) }
        p.intro { "Items: " (@: items <- len$ :@) }
        a href=(@: format! "/items/{}" (items <- len$) :@) { "next" }
        ul {
            @for item in items {
                li { (@: item <- to_uppercase$ :@) }
            }
        }
        @if @: items <- is_empty$ :@ {
            p { "none" }
        }
    }

fn main$:
    println! "{}" (page "Fruit" (&["apple", "pear"]) <- into_string$)
```

```rust
use maud::{html, Markup};

fn page(title: &str, items: &[&str]) -> Markup {
    html! {
        h1 { (title) }
        p.intro { "Items: " (items.len()) }
        a href=(format!("/items/{}", items.len())) { "next" }
        ul {
            @for item in items {
                li { (item.to_uppercase()) }
            }
        }
        @if items.is_empty() {
            p { "none" }
        }
    }
}

fn main() {
    println!("{}", page("Fruit", &["apple", "pear"]).into_string());
}
```

#### serde_json `json!`

*serde_json 1.* JSON with Rust values: after each key's `:`. Literals, arrays of literals and nested objects stay as written; a value inside a nested object is a hole like any other.

```rust harsh
use serde_json.json

fn main$:
    let name = "Ada"
    let langs = vec! "Rust" "Harsh"
    let v = json! {{
        "name": @: name <- to_uppercase$ :@,
        "age": 36,
        "langs": langs,
        "count": @: langs <- len$ + 1 :@,
        "tags": [{ "id": 1, "main": @: langs <- first$ :@ }]
    }}
    println! "{v}"
```

```rust
use serde_json::json;

fn main() {
    let name = "Ada";
    let langs = vec!["Rust", "Harsh"];
    let v = json!({
        "name": name.to_uppercase(),
        "age": 36,
        "langs": langs,
        "count": langs.len() + 1,
        "tags": [{ "id": 1, "main": langs.first() }]
    });
    println!("{v}");
}
```

#### Tokio `select!`

*tokio 1.* Arms, `pattern = future => handler,`: the pattern (unless it is `_`), the future and the handler are each Rust. A handler that is a block is one value, an explicit block: `=> @: do: … :@`.

```rust harsh
use tokio.sync.mpsc
use tokio.time.(sleep, Duration)

#[tokio.main]
async fn main$:
    let (tx, mut rx) = mpsc.channel.<u32> 4
    tokio.spawn (async move:
        tx <- send 7 <- await <- unwrap$
        ()
    )
    let got = tokio.select! {
        @: Some v :@ = @: rx <- recv$ :@ => @: v * 2 :@,
        _ = @: sleep (Duration.from_secs 1) :@ => @: do:
            println! "timeout"
            0 :@
    }
    println! "got {got}"
```

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel::<u32>(4);
    tokio::spawn(async move {
        tx.send(7).await.unwrap();
    });
    let got = tokio::select! {
        Some(v) = rx.recv() => v * 2,
        _ = sleep(Duration::from_secs(1)) => {
            println!("timeout");
            0
        }
    };
    println!("got {got}");
}
```

#### std `thread_local!`

*the standard library.* Declarations: the value after `=`. The type before it, `RefCell<Vec<String>>`, is the macro's text -- `Cell<bool> = …` is no expression -- and so is the `;` after the hole.

```rust harsh
use std.cell.RefCell

thread_local! {
    static LOG: RefCell<Vec<String>> = @: RefCell.new (Vec.with_capacity 8) :@;
}

fn main$:
    LOG <- with (|l| l <- borrow_mut$ <- push ("start" <- to_string$))
    LOG <- with (|l| println! "{:?}" (l <- borrow$))
    ()
```

```rust
use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::with_capacity(8));
}

fn main() {
    LOG.with(|l| l.borrow_mut().push("start".to_string()));
    LOG.with(|l| println!("{:?}", l.borrow()));
}
```

#### `lazy_static!`

*lazy_static 1.* As `thread_local!`: the value after `=`, a block when it takes several statements.

```rust harsh
use lazy_static.lazy_static
use std.collections.HashMap

lazy_static! {
    static ref NAMES: HashMap<u32, &'static str> = @: do:
        let mut m = HashMap.new$
        m <- insert 0 "zero"
        m <- insert 1 "one"
        m :@;
    static ref COUNT: usize = @: NAMES <- len$ * 2 :@;
}

fn main$:
    println! "{} {}" (NAMES[&1]) (*COUNT)
```

```rust
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref NAMES: HashMap<u32, &'static str> = {
        let mut m = HashMap::new();
        m.insert(0, "zero");
        m.insert(1, "one");
        m
    };
    static ref COUNT: usize = NAMES.len() * 2;
}

fn main() {
    println!("{} {}", NAMES[&1], *COUNT);
}
```

#### `bitflags!`

*bitflags 2.* A declaration in the macro's own grammar: `struct Perms: u32 { const … }` and the `#[derive(…)]` above it are the macro's text; only a constant's value after `=` is Rust, and a literal stays as written.

```rust harsh
use bitflags.bitflags

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Perms: u32 {
        const READ = 0b001;
        const WRITE = 0b010;
        const ALL = @: Self.READ <- bits$ | Self.WRITE <- bits$ :@;
    }
}

fn main$:
    let p = Perms.READ | Perms.WRITE
    println! "{:?} {}" p (p == Perms.ALL)
```

```rust
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Perms: u32 {
        const READ = 0b001;
        const WRITE = 0b010;
        const ALL = Self::READ.bits() | Self::WRITE.bits();
    }
}

fn main() {
    let p = Perms::READ | Perms::WRITE;
    println!("{:?} {}", p, p == Perms::ALL);
}
```

#### `tracing`'s `info!` and `debug!`

*tracing 0.1.* Not a DSL in braces: the stream is a list -- fields, a message, its arguments -- so it is juxtaposed like any call, each field isolated: `(user = %user)`, `(?items)`, `(target: "app")`.

```rust harsh
use tracing.(debug, info)

fn main$:
    tracing_subscriber.fmt$
        <- without_time$
        <- with_ansi false
        <- with_max_level tracing.Level.DEBUG
        <- init$
    let user = "ada"
    let items = vec! 1 2 3
    info!
        (user = %user)
        (count = items <- len$)
        "processed {} items"
        (items <- len$)
    debug! (target: "app") (?items) "starting"
    ()
```

```rust
use tracing::{debug, info};

fn main() {
    tracing_subscriber::fmt().without_time().with_ansi(false).with_max_level(tracing::Level::DEBUG).init();
    let user = "ada";
    let items = vec![1, 2, 3];
    info!(user = %user, count = items.len(), "processed {} items", items.len());
    debug!(target: "app", ?items, "starting");
}
```

#### Lists: `vec!`, `format!`, `assert_eq!`, `matches!`

*the standard library.* Macros whose stream is a list of values are applied like functions -- juxtaposed, no braces, no holes. This is most macros.

```rust harsh
fn main$:
    let v = vec! 1 2 3
    let s = format! "{} items, first {:?}" (v <- len$) (v <- first$)
    assert_eq! (v <- iter$ <- sum.<i32>$) 6 "sum of {:?}" v
    let big = matches! (v <- last$) (Some n if *n > 2)
    println! "{s} {big}"
```

```rust
fn main() {
    let v = vec![1, 2, 3];
    let s = format!("{} items, first {:?}", v.len(), v.first());
    assert_eq!(v.iter().sum::<i32>(), 6, "sum of {:?}", v);
    let big = matches!(v.last(), Some(n) if *n > 2);
    println!("{s} {big}");
}
```

## What does not change

Calls are juxtaposed (`println! "{}" a`); `m! { … }` calls keep their braces, and a bracket after `!` is an array argument; the body of a `macro_rules~` transcriber may still be written in Rust's braces under rule 3 when that is wanted. Procedural macros are unaffected: they receive the transpiled tokens.
