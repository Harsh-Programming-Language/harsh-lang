# Macros in Harsh — design

Decided in conversation before any code, from a real test: five notebooks of Leptos, Axum and Actix notes converted with `hrs-from`, whose `view!` bodies the transpiler mangled. The principle that came out of it: **the macro rules are Harsh's rules, on both sides.** A Harsh call is one group per argument; a Harsh matcher is one group per fragment; a macro body is Harsh where it is expressions and markup where it is markup; and the bracketed and braced forms are Rust's, exactly as brackets and braces are everywhere else in the language. Macros do not produce Harsh — `hrs` transpiles the file, then rustc expands — but the programmer never has to know: both sides of every macro are written in Harsh and meet in Rust.

## Procedural macros and foreign DSLs: still in development

What is finished is **declarative** macros: Harsh's own (`macro_rules~`, which
unfolds into Harsh) and Rust's (`macro_rules!`, copied verbatim and expanded
by rustc).

**Procedural macros are not finished.** A proc-macro definition written in
Harsh transpiles to what looks like an ordinary Rust proc macro, and a call
with simple arguments reaches the macro as ordinary Rust tokens
(`echo! 1 2 3` arrives as `1, 2, 3`) -- but this is not yet verified end to
end, and one sharp edge is known: a call whose arguments carry a top-level
operator is read as an expression, so

```
my_macro! a | b | c        emits    my_macro!(a) | b | c
```

and the macro receives only `a`. Isolate the tokens until this is settled:
`my_macro! (a | b | c)`, `my_macro! [a | b | c]`, `my_macro!\ …` or
`my_macro! do:` all carry them through intact.

**Foreign DSLs** -- a `view!`, `rsx!` or `sql!` from someone else's crate --
work only as far as the shapes Harsh emits happen to match what that macro's
parser wants. The general answer is a spelling map shipped by the crate
(`dsl.hrs`), which is designed but not built.

Both are in progress; the shapes above may change.

## Rule 0 — `macro_rules!` is Rust, `macro_rules~` is Harsh

Harsh has two declarative macro systems, and the mark says which. Everything
in the rules below is about **`macro_rules~`**, Harsh's own: Harsh layout,
Harsh matchers, a Harsh transcriber, and calls written `twice~ 4`.

**`macro_rules!` is a zone of Rust.** The author is writing Rust and knows
it, so the transpiler reads none of it and rewrites none of it: the
definition goes out byte for byte as it came in, and `hrs-from` copies a Rust
one into a Harsh file the same way, which makes the round trip exact.

```
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

## Calling a macro: the four shapes

Nothing here is special to macros. A call is a header, and what follows it is
a block of the kind the header's mark names — the same kit as everywhere else
in the language, which is why the forms can be guessed rather than recalled.

```
m! x y z                             m!(x, y, z)          juxtaposed arguments
m! (x) (y) (z)                       m!(x, y, z)          the same, isolated

m!\                                  m! {                 `\`: entries separated
    spec                                 spec,            by `,` -- the newline
    spec                                 spec,            does what the comma does
                                     }
m!\ spec, spec                       m! { spec, spec }    the same, inline

m! do:                               m! {                 `do:`: statements,
    stmt                                 stmt;            separated by `;`
    stmt                                 stmt
                                     }
m! { stmt; stmt }                    m! { stmt; stmt }    braces hold a block on one line

m!\                                  m! {                 `#:`: entries separated
    Name #:                              Name {           by *nothing* -- for a
        attr                                 attr         grouping whose grammar
        attr                                 attr         is its author's
                                         },
                                     }
```

Which one a given macro wants is a fact about that macro's grammar, not about
Harsh. For a declarative macro it follows from the matcher and needs no
declaration; for a procedural macro, whose layout can be anything, a crate's
`dsl.hrs` is where the answer will live.

## Rule 1 — HSX: a macro `do:` block whose first line begins with `<`

Its lines are markup, copied through unchanged: tags, static attributes, quoted text, whitespace. The contents of every `{ … }` are Harsh, transpiled as an expression; a hole may span lines and holds layout inside it — a closure with a block body is `{move |_|:` with the body beneath and `}` closing it. Emitted as `name! { … }`.

```
view! do:
    <div class="app">
        <p>{count}</p>
        <button on:click={move |_| set_count <- update (|n| *n += 1)}>"+"</button>
        <Greeting name={"World" <- to_string$} />
    </div>
```

`on:click={expr}` is already valid RSX (rstml's block-valued attributes), so the braces need no translation; only their contents do. An attribute value may also be isolated in Harsh's parens, `on:click=(move |_| …)`: rstml parses a parenthesised expression after `=` (verified in its source, 0.13), the parens are optional grouping and are not emitted, and the bare `on:click=move |_| …` comes out. Braces are the preferred spelling, one with rule 6's holes, and are what `hrs-from` writes; the parens are accepted. The body is markup when it holds a tag at its top level; it may open with a hole or a string. The same rule holds in the brace form, `view! { … }`. This is the one place in Harsh where a brace is Harsh's and not Rust's, and it is defensible because markup is not Rust. Covers Leptos, Yew, Sycamore — the angle-bracket family.

## Rule 2 — a macro `do:` block otherwise is a Harsh block

All rules apply. Lines with a top-level `=>` are arms and get commas; the rest are statements and get semicolons. `$ident` is an atom. A repetition `$( … )*` that is the whole of its line holds what its block holds — statements, each with its `;`; arms or fields, each with its `,`. One embedded in an expression follows the brick of rule 4 read from the transcriber's side: `$( ($x) )*` is a list of arguments and becomes `$($x),*`; a repetition of bare tokens, `$($arg)*`, forwards `tt`s and is copied as it stands. A multi-line repetition is a paren block: `$(` line-final, the body beneath, `)*` as its own line. Emitted as `name! { … }`.

A transcriber `=> do:` emits Rust's delimiting braces, which are not a block: for the expansion to *be* a block with a value, the block is written — an inner `do:` — as Rust writes `{ { … } }`. A transcriber is a block or Rust's delimited group; a bare `=> $e * 2` is an error naming the forms. Arms take their `;` from the newline; a written one is rejected as a written `,` is after a match arm.

```
let winner = tokio.select! do:
    n = slow "slow" => n
    n = fast "fast" => n

macro_rules~ my_vec
    ( $( ($x:expr) )* ) => do:
        do:
            let mut v = Vec.new$
            $(v <- push $x)*
            v

macro_rules~ twice
    ($e:expr) => do: $e * 2
```

## Rule 3 — a macro `{ … }` body is Harsh in one line

*(As first written, rule 3 made a brace body "Rust's syntax with Harsh's tokens, nothing juxtaposing". That exemption was withdrawn with the space rule — one syntax for applying, everywhere — once rules 1 and 6 had given `view!` and `rsx!` their own forms and no DSL needed it.)*

A brace body is what a brace is anywhere in Harsh: the one-line form, layout off, the tokens Harsh's and juxtaposition on. `quote! { fn #name$ -> u32 { #body } }` emits `fn #name() -> u32 { #body }`; `tokio.select! { n = slow "slow" => n, }` emits the call. `f(x)` is an error here as everywhere. A DSL with bare adjacent words that are not an application has no brace form in Harsh; it has rule 1 or rule 6, or a `do:` body.

## Rule 4 — matchers: the parenthesised side is Harsh's

A parenthesised fragment `($x:expr)` is one parameter; a sequence of them is a comma list; a repetition `$( (…) )*` is a comma-separated repetition. Literal tokens *inside* a group stay inside it.

```
macro_rules~ hashmap                                  // Rust: ( $( $k:expr => $v:expr ),* )
    ( $( ($k:expr => $v:expr) )* ) => do:
        …
let m = hashmap~ ("a" => 1) ("b" => 2)                 // Rust: hashmap!("a" => 1, "b" => 2)
```

`fn f (a: T) (b: U)` → `fn f(a: T, b: U)`; `m! a b` → `m!(a, b)`; `( ($a:expr) ($b:expr) )` → `( $a:expr, $b:expr )`. The same brick, a third time. The mapping applies to a matcher, or a repetition body, with a group at its top level; a bare run of tokens beside a group is one item (`( add ($a:expr) ($b:expr) )` → `( add, $a:expr, $b:expr )`, matching the call `m! add 1 2`). A `tt` matcher — `( $( $arg:tt )* )` — is written the same in both languages, since it matches anything including the commas a Harsh call produces.

## Rule 5 — matchers: the bracketed and braced sides are Rust's

A matcher in `[ … ]` or `{ … }` is Rust's syntax, as its calls are: `vec! [0u8; 4]` is kept verbatim, so `[ $elem:expr ; $n:expr ]` is how a macro takes a `;`-separated pair.

```
macro_rules~ filled
    [ $elem:expr ; $n:expr ] => do:
        vec! [$elem; $n]
let v = filled~ [0u8; 4]
```

## Rule 6 — a brace tree: a macro `do:` block whose first line is `name:`

Dioxus's `rsx!` is not markup but a tree of braces, `div { class: "app", onclick: move |_| …, "Hello" }` — and a brace tree is what Harsh's layout is. So it is written as one, and as in rule 1 the Harsh is isolated in `{ … }`: everything outside a hole is the DSL's own syntax with Harsh's tokens (rule 3), everything inside is Harsh.

```
rsx! do:                                              rsx! {
    div:                                                  div {
        class = "app"                                         class: "app",
        onclick = {move |_| count <- set (count$ + 1)}        onclick: move |_| count.set(count() + 1),
        "Hello {count}"                                       "Hello {count}"
        for item in items:                                    for item in items {
            li: "{item}"                                          li { "{item}" }
        Button:                                               }
            onclick = {move |_| reset$}                       Button {
            "Reset"                                               onclick: move |_| reset(),
                                                                  "Reset"
                                                              }
                                                          }
                                                      }
```

- An element or component starts a block with `:`, `div:` / `Button:` / `li: "{item}"` inline. Its body is a brace tree again.
- An attribute takes its value through `=`, since `:` opens blocks here; it is emitted `name: value,`. A literal value stands bare; anything else is a hole, and the hole's braces are dropped on emission because `rsx!` recognises an event handler by its leading `move` or `|`.
- A child — a string, a `{ … }` hole, a `..spread` — is copied; a child hole keeps its braces.
- `for … in …:`, `if …:`, `else:` are the DSL's own control flow, not Harsh's, so their headers are copied with the token substitutions only and their bodies are brace trees. They are not isolated, though they resemble Harsh code.
- Commas: verified against `dioxus-rsx` 0.7's parser — attributes must be separated by commas (a trailing one is accepted), and a comma after an element or text node is flagged as unnecessary. So an attribute line takes `,`, no other line does.
- A hole may span lines as in rule 1: `onclick = {move |_|:` with the body beneath, `}` closing it.

Rules 1, 2 and 6 are told apart by the shape of the block's first line: `<` is markup, `name:` — a block opened by a non-keyword name, which means nothing else anywhere in Harsh — is a brace tree, anything else is a Harsh block. A tree begins with an element; `name =` is not the test, since `select!`'s arms have that shape.

## What does not change

Calls are juxtaposed (`println! "{}" a`); `vec! [ … ]` and `m! { … }` calls keep their brackets and braces; the body of a `macro_rules~` transcriber may still be written in Rust's braces under rule 3 when that is wanted. Procedural macros are unaffected: they receive the transpiled tokens.

## Order of work — done

Every step below has landed. Step 5, the notebooks, is regenerated outside this tree.

### The steps as planned

1. Transpiler: rule 3 (juxtaposition off inside macro braces — the fix the notebooks needed first; later withdrawn, see rule 3 above), rule 1 (HSX from the block's tokens, holes laid out as fragments), rule 2 (arm classification, `$ident` atoms, line-level repetition semicolons, `$(` paren block), rule 4 (matcher mapping), rule 5 (nothing to do — verify), rule 6 (the brace tree). A test per rule; round trip byte-exact.
2. Converter: Rust `view! { … }` → `view! do:` with attribute values and holes in `{ }` (the end-of-expression heuristic lives here — depth-zero `>` or the next `name=` — where permissiveness is fine); Rust matchers → rule 4 groups; other macro bodies → rule 3 with the author's whitespace kept. Round trip over the five notebooks.
3. Book: chapter 17's `select!` and chapter 20's `my_vec` in rule 2 form; HSX and the matcher rule in chapter 20's macro section.
4. Guide: a "Macros" section stating rules 1–5 with these examples, verified.
5. The five notebooks regenerated.
6. Tree-sitter and VSCode: HSX holes as code, markup as markup. Handover, changelog, archive.

Corpus for the mapping: the Book's macros, the guide's, `hashmap~`, `filled~`, `log!` above, and the notebooks' `view!` bodies.
