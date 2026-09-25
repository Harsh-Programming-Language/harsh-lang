# Roadmap

Sizing is relative: XS, S, M, L. "Blocked by" matters more than the estimate.

## Done

- Language frozen and self-hosting: transpiler, converter, remapper, driver, watch mode
- `hrs-lsp`: a name-blind language server with layout-aware on-type formatting (Enter under the previous arrow, one level in after `=`, into the block after an opener) and a `harsh/columns` request the VSCode extension binds to Tab / Shift-Tab; launched by both extensions. `src/columns.rs` is the shared line-structure answer the formatter will reuse. Design in `docs/LSP.md`
- Five layout fixes — tuple index as atom, closure statements take `;`, parens transparent to layout, bare-closure chain rejected, mid-block `;` rejected
- Macro DSL exemption removed: a macro is applied exactly like a function, both directions
- The pipes as the one deferral mechanism: `|>` fills from the left, `<|` from the right, the closure vanishes when the arity is met — flat closures, composable, values — with project-wide arity collected by the driver. `$` removed.
- VSCode grammar; tree-sitter grammar and Zed extension, matching the transpiler across all 47 example files with zero error nodes
- Licence (MPL 2.0), trademark, governance, contributing
- Harsh's own proc macros, the function-like form (0.1.28, ruling 17): a Harsh function marked `#[proc_macro~]`, called `name~ stream`, run by a generated runner when the caller is transpiled; `hrs_proc_macro` 0.1.0
- Harsh derives (0.1.29), mirroring Rust's: `#[proc_macro_derive~ Name]` with optional helper attributes, called `#[derive~ Name]`
- Language guide doubled — tutorial plus every construct in every form — with every snippet transpiled, compiled and run
- Driver verified on a Mac with rustc 1.97 and current cargo: `new`, `build`, `run`, remapping. Manifest guard (refuses to build without a target under `target/hrs/`), always-on transpile report, equal-mtime staleness, progress bar suppressed; remapper locates zero-width insertion spans; application-vs-`<-` precedence stated in the guide and pinned by a test

## Open, in order (2026-09-22)

**0. Foreign DSLs and the macro rulings of 2026-09-22 — the build plan.**
Ruled that day (ruling 16, the map of the macro system and the A–E rulings
in the macro design notes); nothing built. Not a rewrite: the layout rules,
the application rule, `~` macros, the emitter, pipes, comprehensions and
matrices are untouched. What changes, in the order to do it, one change at
a time with the gates between:

1. **The `vec!` migration — done 2026-09-23 (0.1.23).** A bracket after `!` becomes an
   ordinary argument (A1), so `vec! [1, 2, 3]` would mean `vec!([1, 2, 3])`:
   the idiom becomes `vec! 1 2 3` and `vec! { 0; 10 }`. 78 corpus lines,
   the Book, Harshlings, *by Example*, the guide and `hrs-from` move in one
   step, with a diagnostic for `vec!` applied to a single array literal. A
   language change more than a transpiler change; its own diff.
2. **Local rule changes — done 2026-09-23 (0.1.24):** `m!\`
   retired (A2); "a literal cannot be applied" (A3); `@:` refused outside a
   body (B1); the turbofished-function application bug
   (`parse_kv.<String> "x"` loses its parens, C1).
3. **The prototype — merged into 4 (the user: everything in the tree):** a zone scanner for `m! { … }` bodies
   with `@: … :@` holes, run on the sketches in ruling 16 and on the sr-auto
   Rust (`sr-auto-leptos.zip`), with a stub `view!` and a real Leptos build.
   Bring the runs before building.
4. **Verbatim bodies with holes — built 2026-09-23 (0.1.24, `src/dslzone.rs`); the old DSL modes retired with it; expansions read as source (0.1.26).** Open: the real Leptos build; source-map offsets for a re-read expansion (a hole's are right since 0.1.27). A brace macro call
   is a zone the transpiler does not lex as Harsh -- the `macro_rules!`
   zone mechanism, generalised -- with a scanner for `@: … :@` that hands
   each hole to the pipeline as a fragment with its own baseline (B2),
   splices the Rust back, and carries the source map through (E2); `hrs fmt`
   leaves the zone alone (E1). Estimated 500–800 lines across `lex`,
   `layout`, `emit`, `fmt` and the map, mostly new. Then `@@:` (B8),
   comments skipped in a body (B5).
5. **The converter — done 2026-09-23 (0.1.25-0.1.26): writes the holes (markup, trees, `select!` arms, `json!` objects by shape), writes a DSL stream in braces, an error for what it cannot write:** `hrs-from` classifies every
   macro call by its stream -- comma list to juxtaposition, else braces
   (D1) -- and drops the stray `;` it leaves after `sql!(…)`. `unbrace.rs`
   is where the subtle regressions come from; `round_trip_own_source` is
   the guard.
6. **Reconvert the sr-auto site** from its Rust with the new `hrs-from`; lay
   out its holes by hand where Harsh is wanted.
7. **Outside the transpiler:** the VSCode and tree-sitter grammars (a scope
   for holes, one for verbatim bodies, E3); the precedence table of
   operators and constructs in the guide (A4); the guide's DSL chapter.

Ruled 2026-09-23: **always close** -- every hole ends in `:@`, delimited
or not (removes "whichever first" and B6). Still to rule before 4: B7,
spacing around the marks, with B2's fresh-line question. Sessions: one
for 3, two or three for 1–2 and 4–6. The old items 6 and 7 below (the
`dsl.hrs` spelling map, the `#:` formatter rule) are superseded by this
item.

1. **Harsh by Example — done 2026-09-20.** `by-example/`: 15 pages, 43
   programs, one idea each, every one transpiled, compiled and run by its
   harness; the lookup book between the Book and the Guide. Written by hand
   rather than converted (the user's call, and the reason it can carry a page
   on what Harsh adds); the topics follow *Rust by Example*, MIT OR
   Apache-2.0, attributed in `by-example/ATTRIBUTION.md`. In the public tree,
   excluded from the crate, as the Book is. Left for a later pass, pending the
   user's reading: lifetimes beyond slices, iterators as their own topic,
   trait objects past one example, async.
2. **Generator comprehensions — done 2026-09-21.** `g~ e for p in it if c …`
   in Harsh's prelude, with `list~`, `set~`, `dict~`; the design and every
   alternative rejected on the way are recorded as rulings 11-13 in the
   macro design notes, and *Harsh by Example* has a page on it. Building it
   fixed two emitter bugs every user macro had (an expanded `$` pasting
   source text, and no turbofish inside an expansion). Open: a real
   `hrs_std` crate, which waits on `~` macros travelling between crates.
3. **Matrices and linear algebra, Julia's — done 2026-09-21 (0.1.18).**
   `hrs_std` (nalgebra underneath), `m~`/`v~` in the prelude with Julia's
   grammar, `*` dispatched by the operand types, `solve` for Julia's `\`
   (least squares when tall), `inv`, `det`, `UniformScaling`, `I`,
   concatenation; the Book's chapter 16. Every decision and its reason is
   ruling 14 in the macro design notes. **Slicing and broadcasting — done
   2026-09-21 (0.1.21)**, ruling 15: `slice`, views through a comma index,
   the dotted operators, `f<>`. **Open:** a Miri run over `hrs_std`'s views
   (the user's Mac) before `hrs_std` 0.1.1 is published; `.^` and dotted
   comparisons as operators and in-place `.=` (closed unless the user asks:
   `f64.powf<> a 2.0` and a closure under `<>` cover them, and neither can
   be a token substitution), an optional `bc~` (`@.`) macro. Taught in the
   Book (16.7, 16.8) and Harshlings (`matrices05`–`07`) since 0.1.22;
   block concatenation with `UniformScaling`. (A panic naming the Harsh line: done in 0.1.19.)
4. **The user's macro testing** — how `expr` absorbs constructs and isolation
   parens under the stream rule. Any adjustment to the extent rule waits on
   his examples. Two things 0.1.14 found and left for it: an `expr` fragment
   accepts *any* single token as an atom (`<-`, `\`), so a stream that does
   not fit fails late and with a poor message; and a fragment has no notion of
   a line, so in the `do:` form nothing stops a `stmt` at a newline.
5. **Procedural macros** — paused for his reading. Verified: a proc macro
   defined in Harsh transpiles to a Rust one; simple calls reach it as
   ordinary Rust tokens. Untested: a Harsh-written proc-macro crate built and
   called; derive and attribute on Harsh items; Harsh marks inside a macro
   body; diagnostics mapped back. Open ruling: `m! a | b | c`.
6. ~~**Foreign DSLs**, the `dsl.hrs` spelling map~~ and 7. ~~**the `#:`
   formatter rule**~~ -- **superseded** by item 0, the `@: … :@` holes, built
   0.1.24-0.1.27 (item 0 said so; this list had not caught up -- corrected
   2026-09-24, the user). What is still open for DSLs:
   - **B7, one ruling, deferred by the user:** spacing around `@:` and `:@`,
     with B2's question (may code start on the `@:` line of a multi-line
     hole?). Laid out for him in the development notes (`DSL-REVIEW.md`).
   - a real Leptos build; source-map offsets for a re-read expansion;
   - reconverting the sr-auto site with the current `hrs-from`;
   - the editor grammars' scopes for holes and verbatim bodies, the guide's
     DSL chapter, its precedence table;
   - C1's transpiler bug: `parse_kv.<String> "x"` loses its application.
8. **Zed** — the two Enter causes in `ZED-FINDINGS.md`, then the registry PR.
9. **Kernel follow-ups** — rustc line numbers through evcxr's wrapping back to
   the cell's Harsh lines; completion via `hrs-lsp` once it has types.
10. **The rest of the language server** — hover, go-to-definition, inline
   errors — and Open VSX.
11. **Still the user's to run** — the hand-written sr-auto rewrite; the
   refinement corpus of real crates. (The rewrite needs the site's original
   *Rust*, reconverted with the current `hrs-from`: the 2026-09-07 conversion
   predates the block kit and `match x\`, and 9 of its 22 files are refused
   today.)
12. **Last: a Harsh crate registry** — deliberately last; `~` macros travel as
   `.hrs` inside a crates.io package already.

## Known bugs, open — fix before the next release that touches the converter

- **Matrix views are undefined behaviour under Stacked Borrows** (Miri on the
  user's Mac, 2026-09-25; released so in 0.1.30 by his decision). `&a[0..2,
  ..]` returns a reference to a zero-sized slice pointing at the matrix, its
  window in the slice's length -- `bitvec`'s technique -- and widening that
  reference back to the matrix is rejected by Stacked Borrows (Tree Borrows
  accepts it; the rest of `hrs_std` is clean under both). **First work after
  0.1.30, the user's choice (2026-09-25): (c), a view as a value** --
  `View<'a, T>` holding `&Matrix` and the window, sound with no trick -- which
  needs a spelling of its own, since Rust's `Index` must return a reference
  and the transpiler has no types to route `&a[..]` elsewhere. Design first.

- **`hrs-from`: an `if` as a struct literal's field value, with a string
  continued over lines in a branch** (found 2026-09-24 by the self-host in
  `src/procmac.rs`, worked around there with a `let`). The converter writes
  the `if` in inline braces that span two lines, which the transpiler
  refuses. Pinned by the ignored test
  `converter_writes_an_if_with_a_continued_string_as_a_field_value` in
  `tests/roundtrip.rs` -- every `cargo test` reports it as ignored until
  it is fixed. The user asked that this not be forgotten.
  *Traced 2026-09-24:* inside a struct literal, Rust's own braces suspend
  the layout, and a field's `if` keeps inline braces there on purpose
  (`E\ m = if a {1 } else {2 }` -- a block on one line, which Harsh allows);
  the converter never asks whether the block really is on one line, and a
  string continued with `\` makes it two. The `if`-as-a-macro-argument gap
  below is the same family (expression-position blocks inside a group).
  A proper fix is a rule for when such a block may keep its braces and when
  it must take block form inside the group -- a converter design change,
  left for a session of its own.
- ~~**An inline attribute before a `struct` with fields beneath**~~ --
  **fixed 2026-09-24**: `layout::decl_header` skips the line's leading
  `#[…]` groups; `an_inline_attribute_before_a_struct_keeps_its_fields` is a
  regular test now.
- ~~**`hrs-from`: a `match` as an arm's value after a guard arm**~~ --
  **fixed 2026-09-24**: the segment that decides a brace's kind stopped at
  `;` and `}` but not at an arm's `=>`, so the guard's `if` of an earlier arm
  was taken for the brace's keyword; it stops there now
  (`converter_writes_a_match_after_a_guard_arm`, a regular test).
- ~~**A newer `hrs` does not retranspile**~~ -- *misdescribed when found:*
  `is_stale` compared the `hrs` binary's time with each generated file's, so
  after an upgrade a file whose Rust did not change -- and so was not
  rewritten -- was retranspiled on *every* build. **Fixed 2026-09-24**:
  `target/hrs/.hrs-stamp` records the `hrs` (version and binary time) that
  last transpiled a project; a different one makes every file stale once
  (`a_different_hrs_retranspiles_once`). A stamp rather than file times,
  since `hrs` rewrites a generated file only when its content changes.
- **`hrs-from`: an `if` expression as a macro argument** (found 2026-09-24 by
  the self-host in `procmac.rs`, worked around there): `format!("{}", if c
  { a } else { b })` becomes block `if`s inside the parentheses, and the
  `else` loses its `if`. Pinned by the ignored test
  `converter_writes_an_if_as_a_macro_argument`.
- ~~**A `struct`'s `[where …]` clause emitted as a field**~~ -- **fixed
  2026-09-24.** Half a misreading: Harsh writes a declaration's where clause
  on its header line, `struct W<T> [where T: Clone]`, which was always
  right. The other half was real: the function's own-line form beneath a
  struct's header was emitted as a field, invalid Rust, silently; it is now
  refused, naming the header-line form
  (`a_structs_where_clause_stands_on_its_header_line`).
- ~~**`hrs-from`: an attribute between two runs of doc comments**~~ --
  **fixed 2026-09-24**: a comment counted as mid-statement when anything but
  `#` preceded it, so an attribute's `[`, name and `]` did, and the item after
  a following doc comment went one level deeper; now only whole attributes
  and comments before it leave the statement unbegun (`only_attributes`;
  `converter_keeps_an_item_after_an_attribute_between_doc_comments`, a regular
  test). The width-reflowed layout noted with it is the formatter's
  `CHAIN_WIDTH`, the likely mechanism, still not isolated.
- **The retrospective — `docs/RETROSPECTIVE.md`.** Opens with the README's two paragraphs on how the work was run (the credit, and the disagreements that held). When the formatter has landed and the tooling is published, a technical document written to draw lessons from: every step from the first session to the last, in order, with what was decided and why and what was undone; every feature of the language and every tool built to implement and support it — transpiler, converter, remapper, driver, language server, editor grammars, book harness, docs pipeline — each with the source files it lives in (`src/lex.rs`, `src/layout.rs`, `src/juxt.rs`, `src/emit.rs`, `src/unbrace.rs`, `src/columns.rs`, `src/driver.rs`, `src/bin/*`, `editors/*`, `book/build.py`, `docs/build.py`, `check.sh`); the oracles and what each one caught; the bugs the Book found; the principles that held and the ones that were amended, with the amendments dated. Sources: every `HANDOVER.md` delivered (each one records a session), `CHANGELOG.md`, `TUTORIAL.md`, `GOVERNANCE.md`, and the test names, which are the change log the compiler enforces. Opening: the README's paragraph on disagreement — the decisions the author made against Claude's first opinion, and the rule that let a disagreement end with an example rather than with whoever spoke last. Written once, at the end, from those records — so keep them complete until then.

## Blocks, real and pseudo: `#…:`, and no mark on an item body — decided 2026-09-12, not yet built

**The framing, the user's, and it is the reason the rest follows.** `:` and
`do:` are *openers*, not header terminators: a construct's own shape says
where its header ended, which is why `struct Point` and `enum Message` lost
their colon and why `do:` can stand alone as a value. And Rust's `{}` is
overloaded: some braces hold a sequence of statements or expressions with
Rust's separators -- a **real block** -- and the rest are groupings that
merely share the delimiter: a struct's fields, an enum's variants, a `use`
tree, a match arm list, a macro's DSL, an `impl`'s items. Those are **pseudo
blocks**. The distinction is about producing the right Rust syntax and
nothing else: scope, lifetimes and visibility stay rustc's, exactly as the
transpiler never reads a type to parse a line. (For `GOVERNANCE.md`.)

**Three marks, and a rule to tell them apart:**

    no mark     a body of items       impl, trait, mod, struct, enum, extern
    #x:         a grouping whose separator you name -- Harsh cannot know it
    :  do:      a real block: statements, Rust's separators, a tail value

A macro's body is not Rust: it is whatever its author invented, and Harsh's
statement rule — a line ends a statement, the transpiler writes the `;` —
is wrong there. `quick_error!` shows it plainly: its attribute lists take no
separator at all, and Harsh writes `from();` where the DSL wants `from()`.

**The decision (the user's design).** A third kind of block, whose opener
states how its entries end:

    #:      Harsh writes the braces; the entries end with nothing
    #,:     ... with `,`
    #;:     ... with `;`
    #<p>:   ... with `<p>`, whatever punctuation stands between `#` and `:`

written at the end of the line that opens the block, exactly where `:` and
`do:` go. `:` and `do:` keep their meaning: real blocks, with Rust's
separators. The example that motivated it:

```
quick_error!:
    #[derive Debug]
    pub enum DocumentServiceError #,:
        RateLimitExceeded #:
            description "You've exceeded the allowed number of documents per minutes"
        Io (err: io.Error) #:
            from$
            cause err
            description "I/O error"
```

**The rules, as decided:**

- **Only how the entries end, never what they mean.** Inside a pseudo block
  the lines are Harsh — `from$` is applied to nothing, `cause err`
  juxtaposes, `description "…"` is an application. Harsh supplies braces and
  a terminator; the meaning is Rust's once translated.
- **An entry is a line and whatever is indented under it**, as everywhere
  else in Harsh; the terminator goes after the entry, not after every
  physical line. (Reuses the comma-block machinery.)
- **Applied recursively, never inherited.** Each opener states its own kind;
  a plain `:` inside a `#:` block is an ordinary statement block again,
  which is what a DSL that embeds real Rust code needs.
- **No trailing separator.** A grammar that accepts `a, b, c` accepts it
  without a final comma by definition; the other direction is not
  guaranteed. If a DSL is found that requires the trailing form, the fix is
  additive (`#,,:` or similar) and does not change what `#,:` means.
- **Allowed anywhere a block may open, except after a construct that already
  has a spelling**: `fn`, `struct`, `enum`, `impl`, `trait`, `mod`,
  `extern`, `macro_rules!`, and the openers `if`, `match`, `loop`, `while`,
  `for`, `do`, `unsafe`. `struct Point #,:` is refused, naming `:` — one
  spelling per construct. A local check at the line, like the dedent rule;
  no context to carry, no "inside a macro" to define.
- **The terminator is general, not a list of three**: punctuation between
  `#` and a line-ending `:`, copied verbatim, so a DSL that separates with
  `|` or `=>` works the day someone meets it. Refused: an identifier or a
  literal (`#x:`), a terminator beginning `[` (that is an attribute), and
  one that would leave the output un-lexable (a lone quote). The guide
  teaches the three common forms and states that any punctuation is legal.

**`impl`, `trait`, `mod` and `extern` lose their `:`.** Their bodies are
items, always, with no separator, always -- pseudo blocks with an empty
terminator, which is `#:`. But a mark that never varies carries no
information, and the header already ends itself: only a name can follow
`impl`, `trait` or `mod`. So they take no mark at all, as `struct` and
`enum` already do. `do:` is refused there for the reason it reads well --
it says "statement block", which an item body is not, and it would teach
that fluently and wrongly.

```
mod garden                          mod garden;                 // no deeper line: Rust's file module

mod geometry                        mod geometry {              // a deeper line: an inline module
    struct Point                        struct Point {
        x: f64                              x: f64,
        y: f64                              y: f64,
                                        }
    impl Point                          impl Point {
        fn origin$ -> Point:                fn origin() -> Point {
            Point\ x = 0.0, y = 0.0             Point { x: 0.0, y: 0.0 }
                                            }
                                        }
                                    }
```

The two cases bare has to survive, and both do:

- **`mod garden` with a body and without** is the lookahead `struct Marker`
  versus `struct Point` already requires, and it was accepted there for the
  same reason.
- **A multi-line `where`** would otherwise read as the body's first item;
  the clause is bracketed, so the layout pass and the reader both see it:

```
impl<T> Summary for Wrapper<T>      impl<T> Summary for Wrapper<T>
    [where T: Display]                  where
    fn summarize (&self) -> String:         T: Display,
        format! "{}" (self <- 0)        {
                                            fn summarize(&self) -> String {
                                                format!("{}", self.0)
                                            }
                                        }
```

  `fn` keeps its `:` -- a statement block -- while `impl` has none.

**The trailing terminator.** `#,:` writes no separator after the last entry;
`#,:,` writes one. The mark after the `:` repeats the terminator (`#=>:=>`,
`#**:**`), so it needs no table and cannot collide with a terminator whose
own spelling repeats a character -- the reason `#,,:` was dropped. The
lexer's rule: `#` opens it, everything to the *first* `:` is the terminator,
and what follows on that line is either nothing or the terminator again.
Anything else is an error naming the two shapes. Note that the line does not
end in `:` here: `#` opens the pseudo block and `:` closes the terminator
specification -- the line-final `:` was a rule about real blocks, not about
these.

**Macros** — the design conversation of
2026-09-12 in full, and the state to resume from. In short: the two kinds of
macro were separated (one you write, one you import); imported DSLs are
**delegated to the library author**, who ships a definition file with the
crate (`harsh/dsl.hrs`) stating its grammar, with Harsh providing the
generator, the editor tooltip, and definition files for the crates that
matter — `view!` and `rsx!` to be extracted from the transpiler into two
such files as the test of the format; `#…:` shrank from a family of
terminators to a single `#:` for the no-separator exception, because `\`
already opens a comma-separated block and needs no new mark;
`macro_rules!` loses its `:` like `impl`/`mod`/`extern` and its arms are
separated by `;`, written by Harsh. **Open**: how a matcher is spelled in
Harsh without losing the repetition's separator (the user is working it out
against real matchers), whether `quote!`'s holes want a Harsh spelling, and
`hrs expand`.

**The earlier direction for library DSLs (2026-09-12, superseded in part by
the delegation above): let `hrs-from` say
what the Harsh version is.** Two kinds of macro were being treated as one,
and they pull opposite ways:

1. **A macro the programmer writes.** He is entitled to the illusion that it
   is a pattern over *Harsh* -- that what he writes inside is what he would
   write outside -- even though the expansion is Rust and Harsh maps both
   ways. Today that illusion holds for the transcriber's shape and breaks at
   the matcher, which is Rust's (`$x:expr`, the fragment specifiers, the
   repetition syntax). How far the illusion can honestly be carried is an
   open question; `hrs expand` -- a user's macro expanded and shown back in
   Harsh -- would carry it a long way, since the loop would then stay in one
   language even though the middle is Rust.

2. **A macro from a library.** Someone else's grammar, which Harsh cannot
   know. Everything proposed so far -- `#…:`, a declaration table, deriving
   the separator from the matcher -- asks the *user* to produce Harsh that
   the DSL will accept, which means looking the answer up in the Rust. That
   relocates the Rust rather than removing it, and a language of lookups is
   the block of granite, not the bricks.

**The inversion (the user's).** Do not guess what a DSL might want: let the
converter say what the Harsh version *is*. `hrs-from` over a crate's
documentation and doc examples produces that crate's DSL in Harsh -- the
Harsh version of its docs, written by the same mapping for every crate in
the ecosystem. The programmer learns one thing, the converter's mapping, and
infers every DSL from it; nobody asserts a fact about someone else's macro,
because the Rust is the truth, the converter is the map, and the round trip
is the check. This is the `view!` hole rule generalised: Harsh imposes one
shape and carries the translation, and the user recognises a pattern instead
of recalling a rule.

**What that makes `#…:`:** the spelling the *converter emits*, not a choice
the user makes. `hrs-from` meets a brace group inside a macro call, reads
the separators that are visible in the text it is converting, and writes
`#,:` or `#:` at the right depth. The user reads the result and copies the
shape. Writing one by hand stays possible and honest -- an escape hatch,
rare -- and `raw:` sits under that for bodies that do not lex as Harsh.

**The work this implies**, when it is taken up: `hrs-from` recognising a
brace group inside a macro call and reproducing its entries as layout plus
the right opener (a mechanical rule over tokens -- the separators are in the
text, not guessed -- so it is uniform across crates); the round trip as the
oracle, crate by crate; and a tool that runs a crate's README and doc
examples through it, which is the same machinery as the notebook
converter and the doc-example pass, pointed at other people's code. **Before any of
it**, the user is looking at real `macro_rules!` and proc-macro sources to
answer: how often a separator is visible in a repetition rather than baked
into hand-written arms; how many of the DSLs a Harsh user meets are
proc-macros, which state nothing; and whether a separator is uniform through
a body or varies by depth, as `quick_error!`'s does.

**Two questions left open (2026-09-12) — proposals only, nothing decided,
nothing changed.** The user will read them rested and decide.

*Question A — where `#…:` sits beside the macro rules, and whether anything
in `docs/MACROS.md` has to move.* The case that raised it, a `quick_error!`
variant whose entries are applications with no separator and whose second
entry runs onto a continuation line:

```
Io (filename: &str) (cause: io.Error) #:      Io(filename: &str, cause: io::Error) {
    display "I/O error: {} for filename {}"       display("I/O error: {} for filename {}",
        cause filename                                    cause, filename)
    context (filename: &str) (cause: io.Error)    context(filename: &str, cause: io::Error)
        -> (filename <- to_string$, cause)            -> (filename.to_string(), cause)
                                                  }
```

Everything in it is ordinary Harsh -- the parameter groups juxtapose, the
continuation line belongs to its entry, the terminator (none here) goes
after the entry and not after every physical line. **Checked, and the answer
is that nothing has to move:** rule 3 once turned juxtaposition off inside a
macro's braces, but that exemption was *withdrawn* with the space rule, and
today a brace body is what a brace is anywhere -- layout off, juxtaposition
on. So Leptos and Dioxus are untouched by any of this: `view!` has rule 1
(markup from the block's tokens) and `rsx!` has rule 6 (the brace tree), and
neither depends on juxtaposition being off. What `#…:` adds is the one thing
rules 1, 3 and 6 do not provide -- a brace body whose *entries are
terminated as the opener says* -- and it answers the sentence at the end of
rule 3 ("a DSL with bare adjacent words ... has rule 1 or rule 6, or a `do:`
body") for the DSLs whose entries are applications but whose separators are
not Rust's. Proposal: add `#…:` to `MACROS.md` as the fourth answer and
leave rules 1--6 as they stand. *(An earlier idea -- making `#…:` "the mark
that says these lines are Harsh", with unmarked brace bodies copied through
-- was withdrawn once rule 3's current wording was read: it solved a problem
that no longer exists.)*

*Question B — `raw:`, the escape hatch, for bodies that are not Harsh at
all.* `#…:` assumes the entries lex as Harsh. Some DSLs do not: `sql! {
SELECT * FROM t WHERE a <> b }`, a matcher full of `$x:expr`, anything whose
punctuation Harsh's lexer would reject or reinterpret. For those no
terminator helps; what is wanted is *brace these lines by indentation, copy
them verbatim, touch nothing*. That is a different feature -- the layout
supplies the braces, the lexer holds its tongue -- and it would need its own
mark. Proposal: record `raw:` as a companion item, unbuilt, and build it
only when a real crate needs it. The pair would then read: `#…:` for DSLs
whose lines are Harsh, `raw:` for DSLs that are not.

**Order of work.** The framing and the three marks into `GOVERNANCE.md` and
the guide; the rules into `GOVERNANCE.md` and the guide; the opener
in the layout pass with the entry rule reusing the comma block; the refusal
lint with its message; `hrs-from` recognising a brace group whose entries
carry `,` or nothing and writing the right opener (without it the round trip
breaks on the crates that motivated this); then `quick_error!` and one or
two other DSL crates through the round trip as the proof; a Harshlings
exercise, since a learner meets this in someone else's macro; the Book's
macro chapters and `docs/MACROS.md` updated — rule 3 (nothing juxtaposes
inside a macro's braces) is what this completes.

**The cost is the corpus, not the parser.** Dropping the `:` from `impl`,
`trait`, `mod` and `extern` touches every one of them: the transpiler's own
source (through `round_trip_own_source`), the 266-file corpus, the Book's
221 snippets, the guide, the 50 Harshlings exercises, the converted Leptos
site. `hrs fmt` can do most of it and `hrs-from` must stop writing the
colon; the round trip is the oracle. Land it **with** `#…:` so the corpus
moves once, not twice.

## A Jupyter kernel for Harsh — built 2026-09-19

`kernel/` in the tree: a Python wrapper kernel (`harsh_kernel`) that transpiles
each cell with `hrs` and proxies it to the evcxr Rust kernel over the Jupyter
protocol, relaying output, values and errors. Harsh errors point at the cell's
own line; evcxr's `:dep`/`:vars`/… commands pass through. Installed with
`pip install ./kernel && python3 -m harsh_kernel.install`; needs `hrs` and
`evcxr_jupyter` on the machine. `docs/JUPYTER.md` is the guide. Tested here
for the transpile half (`kernel/test_transpile.py`); the end-to-end run
against evcxr is verified on the Mac, since the container's Rust 1.75 cannot
build a current evcxr (edition 2024) and an old one took the session's disk.
**Left**: rustc line numbers through evcxr's wrapping back to the cell's Harsh
lines; completion via `hrs-lsp` when it has types. *(2026-09-24: both need
the user's Mac -- the first reads evcxr's error output, and evcxr cannot be
built in the container, so writing the mapping here would mean guessing its
format; the second waits on the language-server design below.)*

## Next week

- **A turbofish head does not apply to a juxtaposed argument.** `from_str.<Person> "…"` and its multi-line form emit `from_str::<Person> "…"` — no call, and rustc's message is about the string, not about the missing application; only `from_str.<Person> ("…")` works. Cause (found 2026-09-12): `juxt::atom_end` accepts a generic list only when the matching `>` is followed by `(` or `.`, which was written for `Vec<i32>` versus the comparison `a < b && c > d`; a juxtaposed argument after `>` fails that test, so the head is never an atom and nothing applies. The rule Harsh states is that a name with its generics is a callee like any other, so this is the rule failing in one of its forms (GOVERNANCE: "a rule holds in every form it applies to"). Fix: accept the list when the `>` belongs to a path segment written `.<…>` — that spelling is unambiguous, since a comparison never follows a dot — and leave the bare `name<…>` case as it is. Pin both directions plus `a < b && c > d`. **And the error**: when a name with generics is followed by something that is not an application, say so — "`from_str.<Person>` is a name with its generics; write `from_str.<Person> arg` to apply it" beats rustc's complaint about the string that follows.


- **Harshlings: an exercise for every edge case decided this week**, so the language's own rules are the ones a learner practises first: the tight index as an atom (`add a[0] a[1]`, `add a[0] (a[1] * 2)`, a tuple of elements), the refused `f arr [1]` and its two spellings, `f ([1, 2])` for an array argument, the written `;` in an inline `do:`, the `\` literal in a tuple, application binding tighter than `<-`, a pipe's sides as atoms, `$` versus `()`, a bare parameter before `->`. Each with the error it produces today as the exercise and the decided spelling as the solution.
- **The same edge cases into the Book**, where each construct is taught — chapter 2's applying section for the index atom and the refused spaced form, chapter 5 for the literal in a tuple, chapter 13 for the pipes — as verified snippets, so a reader meets the rule where the construct is introduced and not only in the reference. (The user's request, 2026-09-11.)

## Before publishing

- **Formatter: a block literal's shape — done 2026-09-10** (the rule below landed in `fmt.rs` and `columns.rs`; the corpus moved with one `hrs fmt`).
- Was: `let user1 = User\` with the fields one unit past the *statement* is legal and is what the Book's snippets write today, but it hides what belongs to what. The author's rule (2026-09-10): the fields sit one unit past the literal's own column, so either `=` ends its line and `User\` starts the next one with the fields beneath it — the guide's stated style, and what `hrs-from` already writes — or the fields align one unit past `User` on the `=` line. Today `hrs fmt` leaves the first form alone (`fmt --check` on `book/src/05_structs/define.hrs` is a no-op), so this is a re-breaking rule for `fmt.rs`, the same brick as "break after `=` before a multi-line group": once it lands, the Book's snippets move with one `hrs fmt`, and Enter after a line ending in `\` should land one unit past the literal's name (`columns.rs`), not the statement. Verified the same day the Book's literals are not in the formatter's style.

## Procedural macros — paused 2026-09-18, by the user

Declarative macros are finished. Procedural macros are **paused while the user
reads into `TokenStream`**; the careful approach is his, and the state is:

**Group 2, a proc macro the programmer writes in Harsh.** The definition
transpiles to what looks like an ordinary Rust proc macro (checked by eye, not
built and called). A call with simple arguments reaches the macro as ordinary
Rust tokens -- verified with an echo macro: `echo! 1 2 3` arrives as
`1, 2, 3`, `echo!\ a = 1, b = 2` as `a : 1, b : 2`, `echo! do:` as
`let x = 1 ; x + 1`. **One sharp edge found, and it is silent**: a call whose
arguments carry a top-level operator is read as an expression, because
application binds tighter than operators --

    my_macro! a | b | c        emits    my_macro!(a) | b | c

so the macro receives `a` alone and `| b | c` applies to the expansion. The
grouped forms carry operators through intact (`(a | b | c)`, `[a | b | c]`,
`\`, `do:`). **Open question for the user**: refuse the bare form, naming the
fix, or let a macro call take everything to the end of its line as arguments
-- which costs `let x = m! 1 + 2`, where `+ 2` is the caller's arithmetic
today.

**Still untested in group 2**: a proc-macro crate actually written in Harsh,
built and called; the derive and attribute forms applied to Harsh items; a
macro body carrying Harsh's own marks (`<-`, `$`, a `\` literal); and whether
`hrs-remap` carries a macro's own diagnostics back to `.hrs` lines.

**Group 3, a foreign DSL** (`view!`, `rsx!`, `sql!` from someone else's
crate): works only where the shapes Harsh emits match what that parser wants.
The general answer is the crate shipping a spelling map (`dsl.hrs`), designed
in the development tree and not built. Untouched.

## Further out

- ~~Two converter bugs (a brace group after a name)~~ — fixed and pinned, 0.1.12.

- **Publish on crates.io — first of the two below.** `cargo install <crate>` gives anyone the three binaries with one line and no repository; it is packaging what exists, not new code, and it is gated only by the name. The package name stays **`hrust`** (decided: the `h` and the `rust` tell a reader what the package is to Rust; `harsh` is taken on crates.io; the Rust Foundation's policy permits "Rust" in a crate name that refers to compatibility, and the *language* is named Harsh everywhere, which is what the policy is about). **Reversed the evening of 2026-09-10, after `hrust 0.1.0` had been published**: the repository, the mirror, the Marketplace publisher and the extension had all become `harsh-lang`, and one project with one name in four places and another in the fifth was worse than either name; a name without "rust" in it also needs no tolerance from the policy. The crate is **`harsh-lang`** (library crate `harsh_lang`; the binaries stay `hrs`, `hrs-from`, `hrs-remap`, `hrs-lsp`), `hrust 0.1.0` deleted from crates.io the same night, with zero downloads. Then, manifest metadata (`description`, `license = "MPL-2.0"`, `repository`, `readme`, `keywords`, `categories`, `rust-version`), an `exclude` list so editor grammars, notebooks and examples stay in the repository rather than the package, and `cargo publish --dry-run` as the test.
- **Build-script integration.** The transpiler is already a library. A small `build.rs` that walks `src/**.hrs`, writes the Rust into `OUT_DIR` and lets `main.rs` `include!` it would make a Harsh project a dependency-only affair: `cargo build` alone, no `hrs` binary, for every collaborator and every CI. `include!` is known not to break span mapping. Short of the language server, the largest adoption lever there is. Caveat: a build script transpiles but cannot sit between cargo and the terminal, so on a plain `cargo build` rustc names the generated file; the driver stays the way to get errors on `.hrs` lines, and `hrs` should learn the build-script layout alongside the `[[bin]]` one.
- **Language server, the rest of it** -- **ruled 2026-09-24: (a), rust-analyzer
  behind `hrs-lsp`**, the server staying name-blind: transpile, map a `.hrs`
  position to the generated Rust through the source maps, ask rust-analyzer,
  map the answer back (the last good transpile kept for a file that does not
  transpile). Its design document comes first. `hrs-lsp` exists and formats on type. What rust-analyzer gives that `.hrs` files still don't: types on hover, go-to-definition, inline errors. Decides adoption beyond one person. Needs error recovery in the layout pass (first error currently stops the run) and the source map in both directions. Largest item by far; deserves its own design conversation. Smaller first steps in `hrs-lsp`: `)` as a second on-type trigger so the closing paren of an isolated closure snaps into place; `else` likewise.
- **Remove the dead `depth` field** from `scanner.c`.
- **Converter: a `while … matches! …` condition inside a block-bodied arm of a nested `match`** leaves a `,` on the next arm and drops the outer `},` — reproduction in the handover (2026-09-10). Not pinned. It was never updated and does nothing; noted so it isn't mistaken for load-bearing.

## Last: a Harsh crate registry

An online repository of Harsh crates, the user's request of 2026-09-17,
placed last deliberately. It is not a prerequisite for anything above:
a Harsh crate published to crates.io through `hrs export` is a Rust crate
that can also ship its `.hrs` sources in the package; cargo vendors them,
and `hrs` finds a crate's `macro_rules~` definitions (and any `harsh/dsl.hrs`)
there. So Harsh crates live on crates.io as Rust crates with their Harsh
alongside, and a registry of Harsh's own is a convenience for the day the
ecosystem is large enough to want one.

## Settled

- **`<-` stays member access.** Considered and declined: `~`, `\`, `#`, `->`, `-<`, taking `.` back from paths. A future stream-binding feature, when it has a problem statement, takes a symbol that is not a Rust token — `<<-` or `=<` are the candidates (`<<-` cannot be mistyped into `<=`).
- **Brackets index; parentheses isolate.** `v[0]` and `v [0]` are the same index; an array passed as an argument is `f ([1, 2])`. A spaced `[` never applies. Decided 2026-09-11 when Harshlings' runner met `<- args [..]`. The same day: a *tight* `[..]` is part of its atom, so `f arr[1]` is `f(arr[1])` (it used to be `f(arr)[1]`, silently), and a spaced `[` inside an application is refused with both spellings named.
- **The arity table is for the pipes alone.** Decided, and recorded as a rule in `docs/GOVERNANCE.md`. Proposals to read it from any other feature are declined.
- **An inline `\` field list reads to its group's `)`; the transpiler never reads a type to parse a line.** `Point\ x = 1, msg` — grouped or not — is one literal with the shorthand field `msg`; whether `msg` is a field `Point` has is rustc's question, and rustc answers it by name. A rule that ended the literal "once the fields are satisfied" would need the definition of `Point`, which may be in another crate, and is not decidable even then (`..base`). Considered and declined 2026-09-10. A literal that is one element of a tuple takes its own parens, `((Point\ x = 1), msg)`, and `hrs-from` writes that form. The editor's Enter after a trailing macro `!` follows the same discipline: name-blind, from the line alone.

## Principles that decide new items

- Harsh is a superset of Rust: it states rules only for what it changes, and Rust's rules hold everywhere else.
- Rules are simple bricks applied recursively, not big rules with exceptions.
- Strict over permissive. There is a converter for translating; the language does not need to be forgiving.
- Verify against a real compiler. Keep the round trip byte-exact. One change at a time; archive after each green.
