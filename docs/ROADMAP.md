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

## Open, in order (2026-09-25)

**Where things stand.** 0.1.53 is delivered (2026-09-29); 0.1.32 is on
GitLab, and the website is live at https://harsh-lang.com/. `RELEASE.md` has
publishing and installing every product, in order; publishing is the
user's. 0.1.30 and 0.1.31 were handed over and are spent. The design questions
are gathered in the development notes and wait on his use of what exists -- he decides
after a few days with it. What needs no decision is done, or listed last below.

### Waiting on the user, after his testing

- **`hrs fmt` applies the countable rules -- built 2026-09-29 (0.1.52).** Remaining from the list below: (1) and (3).
- **Three layout bugs found applying the handwriting rule (2026-09-29).**
  (1) `let x =` then, on the next line, `if c: a` with its `else:` beneath:
  the inline block is not transpiled (a `:` reaches the Rust). The books use
  `let x = do:` instead. (2) `hrs fmt` flattens a nested `else` in an `if`
  bound by `let`, changing its layout wrongly. (3) `hrs fmt` indents a
  `match`'s arms under its head when the head is a broken chain. Also: the
  companions' programs have never been through `hrs fmt`; formatting them is
  his decision.

- **Real Harsh colours on GitLab and GitHub.** Since 0.1.50 Harsh blocks are
  fenced `rust harsh` and coloured as Rust there. For Harsh's own colours:
  GitLab highlights with Rouge, which accepts new lexers from contributors (a
  Harsh lexer in Ruby, from our TextMate grammar; then GitLab's upgrade of
  Rouge); GitHub's Linguist admits a language used in about 200 repositories.
  Then the tag could become `harsh`.

- ~~**The Harshonomicon**~~ -- delivered in 0.1.45 (13 chapters, 35
  programs), after *Harsh Design Patterns* in 0.1.44.
- **A generic function applied to arguments gets no turbofish.**
  `Layout.array<u32> 4` stays `Layout::array<u32> 4` -- a Rust parse error --
  while `mem.size_of<u32>$` and `sum<u32>$` are turbofished. The books work
  around it (`Layout.from_size_align`, a typed binding for `transmute`); the
  transpiler should turbofish a generic list followed by an argument too.
  Found 2026-09-28.
- **Books quote the generated Rust in error excerpts.** The books' builders
  remap an error's positions with `hrs-remap`, but its excerpt still quotes
  the generated Rust (`names.push(String::from(..))`) -- in *Harsh by
  Example* and *Harsh Design Patterns* alike -- while `hrs run` quotes the
  Harsh. The builders should render errors as `hrs run` does. Found
  2026-09-28.

- ~~**Hover, go-to-definition through rust-analyzer**~~ -- **built 2026-09-27
  (0.1.38)** with his three rulings. To try on the Mac with the real
  rust-analyzer: a type on hover, a function's definition, and a project
  using `hrs_std` (whether rust-analyzer takes the distribution through
  `cargo.extraArgs` -- not verifiable here). **Completion** next, same path.
- ~~**The website's static generation**~~ -- **the deploy, 2026-09-27**: tried
  on his Mac ("blazingly fast"), then the `build` job made the static build
  with the page checks, the trial job retired (`site/DEPLOY.md`, part D).
- **Completion** -- built 2026-09-27 (0.1.39), on the text being typed.

- **Matrix views as values** -- `VIEWS-DESIGN.md`: the spelling (methods
  `view`/`view_mut`, or a `view~` macro), `&a[range, range]` then refused,
  the release it goes in. The views' undefined behaviour under Stacked
  Borrows is the known issue of 0.1.30.
- **Hover, go-to-definition, completion through rust-analyzer** --
  `LSP-RA-DESIGN.md`: how rust-analyzer sees the distribution, answers while
  a file does not transpile, first-release scope.
- **B7's outside of the marks** (the inside is built) and **B2** --
  `DSL-REVIEW.md`.
- **Zed** -- the two Enter causes in `ZED-FINDINGS.md`, then the registry PR:
  his editor.
- **sr-auto** -- the hand-written rewrite: his to run.
- **Publishing**, always his. The Jupyter kernel is on PyPI (`harsh-kernel` 0.1.3, 2026-09-27); its page still says `python3 -m` -- the tree's README says `python -m`, for the next kernel version.
- **`try_solve` and `try_inv`** in `hrs_std`, if he wants them: `solve` and
  `inv` stay Julia's (a panic on a singular matrix, as Julia throws), and the
  twins would return a `Result` for `unwrap`, `expect` or `?` -- no breakage,
  `hrs_std` 0.1.3. Discussed 2026-09-27; he found nothing to fix, so it waits.

### In 0.1.31 and 0.1.32 (delivered 2026-09-26; the changelog lists it)

A Harsh app using a Harsh library by path (it never worked); re-exports of
Harsh macros; `hrs export` carrying no Harsh crate (`hrs_std` as the crate's
own module, module level); the Jupyter kernel's errors at the cell's lines
(checked against evcxr's real output); `hrs fmt` spacing inside a hole's
marks (B7); the converter's field-value gap (in the transpiler's brace
check); the matcher ruling's leftovers (the kernel's test).

- **The website, `site/`** -- built 2026-09-26, written by hand in Harsh on
  Dioxus by Claude Fable 5.1 (the user's ruling: never authored in Rust and converted), part of
  the tree and the bundles, published to GitHub Pages from the GitHub mirror
  by `.github/workflows/pages.yml`; built and served on the user's Mac the
  same day. The Converter (Harsh to Rust in the browser) has its own page;
  the Playground is an editor and a console, running the Rust on the Rust
  Playground's servers, `hrs_std` sent along with it (`driver::single_file`);
  running it all in the browser is deferred, not planned. Publishing guide:
  `site/DEPLOY.md`. Next: his changes; the first CI deploy; static generation
  (`dx build --ssg`) for SEO; a custom domain; the logo as an SVG.

### Open, needing no decision

- ~~**Item-level shaking of `hrs_std`**~~ -- **built 2026-09-25**
  (`src/shake.rs`): 18 real programs (the Book's and *By Example*'s matrix
  examples) exported, built with cargo alone, identical to `hrs run`, zero
  warnings. It keeps ~780 of `hrs_std`'s 846 lines for the simplest matrix
  program: the crate is small and knit together (operators, display,
  indexing and through it the views), so the minimum set is close to the
  whole.
- ~~**Two `if`/`else` arguments in a row**~~ -- **fixed 2026-09-25**: the
  emitter already continued an argument list across a tail after a literal
  (`continues_args`), but required the call's `(` in the block's own header,
  which after an `else:` branch it is not; and the `else` check now looks
  into a block's tail. Both converter gaps of this family are closed.
- ~~**The converter's width-reflowed layout**~~ -- **explained and fixed
  2026-09-25:** not a second bug. The formatter guards its changes only for a
  file that transpiles; handed `hrs-from`'s output when that was broken by a
  converter gap, it reflowed the broken file, and the gap looked like two
  problems. `hrs-from` now formats only a conversion that transpiles
  (`format_if_sound`), so a gap's error points at the converter's own layout.
- ~~**Reconvert the sr-auto site**~~ -- **done 2026-09-25**: 22 files, 2,426 lines, 235 holes, 0 conversion errors; every file reads back the same program (19 token for token, 3 by design); the same picture as 0.1.26, no regression. Building it with Leptos needs Rust 1.76 (Leptos 0.8's minimum).
- ~~**The editor grammars' scopes for holes**~~ -- **done 2026-09-25.**
  VS Code (extension 0.1.5): a `name! { … }` body is the DSL's text
  (`meta.embedded.dsl`), a hole `@: … :@` Harsh inside it, nested DSL bodies
  in holes too, `@@:` an escape -- checked with VS Code's own tokenizer.
  tree-sitter: `hole_open`, `hole_close`, `hole_escape` tokens, highlighted;
  parser regenerated; over the corpus (359 files) error nodes fell from 879
  to 281 -- every hole used to be one. **Zed** takes it once
  `tree-sitter-harsh` is pushed and `zed-harsh/extension.toml`'s `rev` set to
  that commit (the user's). Left: the guide's DSL chapter and its precedence
  table.
- **A real Leptos build**, and source-map offsets for a re-read expansion.

### Last

- **Review: `&a[range, range]` as a view** (the user, 2026-09-27: an error
  for now, reviewed last). Since 0.1.37 it does not compile -- a part of a
  matrix is `a <- view …` (borrowed) or `a <- slice …` (copied), and the
  index is one element. To decide once he has used matrices in Harsh:
  whether the index form should come back, and only if a sound way is found
  (Rust's `Index` must return a reference to something stored; a window is
  not -- the forged reference behind it until 0.1.36 was undefined
  behaviour). Also possible then: a clearer message than rustc's for it.
- **A Harsh crate registry** -- deliberately last; `PACKAGING.md` section 6.

*(The detailed list this replaces -- the DSL build plan of 2026-09-22 and the
items done since -- is kept in the handover and the changelog.)*

## Known bugs, open — fix before the next release that touches the converter

- ~~**Matrix views undefined behaviour under Stacked Borrows**~~ -- **fixed
  2026-09-27 (0.1.37, `hrs_std` 0.1.3)**: views are values (`View`,
  `ViewMut`, `VectorView`, `VectorViewMut`), `a <- view …` / `a <- view_mut
  …`; no `unsafe` left in `hrs_std`. `&a[range, range]` is an error (rustc's:
  no `Index` for a range pair); its review is in *Last*, below. Miri's final
  word is his to run (`RELEASE.md`, step 1).
- ~~**The converter's empty records**~~ -- **fixed 2026-09-27 (0.1.37)**: an
  empty variant stays `Home {}`; an empty `struct Empty {}` keeps its line
  (the backward walk took its `}` for part of the next item's line). `v[0].1`
  and an `if` as a macro argument convert and read back: pinned
  (`converter_keeps_*`, `converter_reads_a_tuple_field_after_an_index_and_an_if_as_an_argument`).
  Left, cosmetic: the converter drops spaces around comparisons (`n> 2`,
  `i<cs <- len$`) -- valid Rust back, only its look.

- ~~**A statement at the top level went through untouched**~~ -- **fixed
  2026-09-27 (0.1.36).** `let x = f 3 2` outside a function came out as `let
  x = f 3 2;` -- Harsh left in the Rust, no word of it. Now an error that
  says Rust's top level holds items only, at the line
  (`layout::check_top_level`, at the three places a file is transpiled).
  Tests: `a_statement_at_the_top_level_is_refused`,
  `the_top_level_keeps_every_item`, `a_statement_inside_a_function_is_fine`.
  Chosen over reading the Converter's input as a notebook cell (the user,
  2026-09-27): strict, and Rust's own rule.

- ~~**A round trip on the website changed numbers**~~ -- **found and fixed
  2026-09-27.** The tools were not at fault: driven every way (Rust → Harsh
  → Rust, the formatter, one argument per line, Harsh read as Rust), `3, 2`
  stayed `3, 2`, and `every_website_sample_round_trips_both_ways` now pins
  every sample of the site both ways. The cause was the website's editor:
  Enter and Tab read the text, then wait for the wasm's columns; a change
  meanwhile (a re-render, a fast key) left the edit at stale positions,
  scrambling the text -- reproduced in the editor's harness (`f 3 2 (y)` came
  out `f 3 2` / `(y)`). Every wait now drops a stale edit; the harness has
  the case (32 behaviours), which fails without the guards. Cosmetic, left:
  the converter writes a deref as `* counts` (the same Rust as `*counts`).

- **Converter, found converting the website back (2026-09-26):** an empty
  record variant `Home {}` converts to `Home` over a `()` line, which
  transpiles to `Home { (), }` -- invalid Rust, the round trip broken; a
  single-token attribute argument is isolated, `#[route ("/")]`; spaces
  around `<` are dropped (`i < cs.len()` → `i<cs <- len$`); `&text(x)` →
  `& text (x)`. None pinned yet.

- **`hrs-from`: a tuple field right after an index, `v[0].1`**, does not read
  back (found 2026-09-25 by the self-host, in a test; worked around with a
  destructuring `let`). Not pinned yet.

- ~~**A Harsh app could not use a Harsh library by path**~~ -- **found and
  fixed 2026-09-25** while building re-exports: `hrs` never transpiled a
  Harsh library the project depends on by path, so cargo found no Rust for
  it -- though the Book's chapter 17 said `hrs` walks each member. Now
  `Project::transpile` transpiles such libraries first, recursively, each
  only when stale (`a_harsh_app_uses_a_harsh_library_by_path`); chapter 17
  corrected to say what `hrs` does. `hrs` at a workspace's root, without a
  `[package]`, is still not a command -- run it in a member.

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
  *Traced 2026-09-25, the transpiler's half:* the approved form -- `(` on its
  own line, the `if`/`else` beneath, `)` -- transpiles for **one** argument.
  With two in a row, the layout pass absorbs the first group's closing `)`,
  the next `(`, its `if c:` and body into the tail of the first group's
  `else:` block, so only the second `else:` surfaces, with no `if` before it
  (node dump: `Block format!…( if c:`, `Block else:`, `Block else:`). The fix
  is in the layout: a block's tail ends at the `)` closing its group, and the
  next `(` starts a sibling group. Then the converter writes that form.
  *Tried 2026-09-25:* the `else` check was taught to look into a block's
  tail -- and the input was accepted but the Rust was wrong, `format!("{} {}",
  if … ) (if … )`: the second group became a call on the macro's result.
  Juxtaposition does not carry an argument list across a block's tail.
  Reverted (refusing beats miscompiling). The real fix is in the emitter's
  juxtaposition; low priority -- the converter works around it with bindings,
  and hand-written Harsh rarely meets it.
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

```rust harsh
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

```rust harsh
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

```rust harsh
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

```rust harsh
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
**Done 2026-09-25: errors point at the cell's Harsh lines.** evcxr 0.17
(built in the container with `cargo install evcxr_repl --version 0.17.0
--locked`, its own lock file keeping it on Rust 1.75) prints its own report,
`╭─[command:1:1]` over a numbered gutter, not rustc's `-->`, so the old
remapping never matched. The kernel now asks `hrs` for the source map, maps
each line of the Rust it sends to the cell's line, and rewrites the gutter
(`6 │` → `3 │` where a one-line struct literal was four lines of Rust),
colours kept; tested on evcxr's real report. **Left:** completion via
`hrs-lsp`, when the language server has types.

## Next week (as of mid-September -- kept for the record; see the open list above)

- ~~**A turbofish head does not apply to a juxtaposed argument.**~~ -- found
  **fixed** on 2026-09-25 (`from_str.<Person> "x"` → `from_str::<Person>("x")`). `from_str.<Person> "…"` and its multi-line form emit `from_str::<Person> "…"` — no call, and rustc's message is about the string, not about the missing application; only `from_str.<Person> ("…")` works. Cause (found 2026-09-12): `juxt::atom_end` accepts a generic list only when the matching `>` is followed by `(` or `.`, which was written for `Vec<i32>` versus the comparison `a < b && c > d`; a juxtaposed argument after `>` fails that test, so the head is never an atom and nothing applies. The rule Harsh states is that a name with its generics is a callee like any other, so this is the rule failing in one of its forms (GOVERNANCE: "a rule holds in every form it applies to"). Fix: accept the list when the `>` belongs to a path segment written `.<…>` — that spelling is unambiguous, since a comparison never follows a dot — and leave the bare `name<…>` case as it is. Pin both directions plus `a < b && c > d`. **And the error**: when a name with generics is followed by something that is not an application, say so — "`from_str.<Person>` is a name with its generics; write `from_str.<Person> arg` to apply it" beats rustc's complaint about the string that follows.


- **Harshlings: an exercise for every edge case decided this week**, so the language's own rules are the ones a learner practises first: the tight index as an atom (`add a[0] a[1]`, `add a[0] (a[1] * 2)`, a tuple of elements), the refused `f arr [1]` and its two spellings, `f ([1, 2])` for an array argument, the written `;` in an inline `do:`, the `\` literal in a tuple, application binding tighter than `<-`, a pipe's sides as atoms, `$` versus `()`, a bare parameter before `->`. Each with the error it produces today as the exercise and the decided spelling as the solution.
- **The same edge cases into the Book**, where each construct is taught — chapter 2's applying section for the index atom and the refused spaced form, chapter 5 for the literal in a tuple, chapter 13 for the pipes — as verified snippets, so a reader meets the rule where the construct is introduced and not only in the reference. (The user's request, 2026-09-11.)

## Before publishing

- **Formatter: a block literal's shape — done 2026-09-10** (the rule below landed in `fmt.rs` and `columns.rs`; the corpus moved with one `hrs fmt`).
- Was: `let user1 = User\` with the fields one unit past the *statement* is legal and is what the Book's snippets write today, but it hides what belongs to what. The author's rule (2026-09-10): the fields sit one unit past the literal's own column, so either `=` ends its line and `User\` starts the next one with the fields beneath it — the guide's stated style, and what `hrs-from` already writes — or the fields align one unit past `User` on the `=` line. Today `hrs fmt` leaves the first form alone (`fmt --check` on `book/src/05_structs/define.hrs` is a no-op), so this is a re-breaking rule for `fmt.rs`, the same brick as "break after `=` before a multi-line group": once it lands, the Book's snippets move with one `hrs fmt`, and Enter after a line ending in `\` should land one unit past the literal's name (`columns.rs`), not the statement. Verified the same day the Book's literals are not in the formatter's style.

## Procedural macros — paused 2026-09-18; built since (0.1.28-0.1.30) -- kept for the record

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
- ~~**The transpiler panicked on a non-ASCII character inside a hole**~~ — **fixed 2026-09-26**: `dslzone::restore`'s scan for a hole's `:@` stepped one byte at a time and sliced `text[j..]`, panicking inside a multi-byte character (`view! { <p>{@: "…" :@}</p> }`); it compares bytes now, which find the same (ASCII) marks. Pinned by `a_hole_carries_characters_of_several_bytes`. Found writing the website, whose Converter and Playground run this code in the browser, where a panic stops the page.
- ~~**Converter: a `while … matches! …` condition inside a block-bodied arm of a nested `match`**~~ — **fixed by 2026-09-25**'s brace look-back stopping at an arm's `=>`: it converts and reads back exactly (`a_while_matches_inside_a_nested_match_converts_and_reads_back`). It was never updated and does nothing; noted so it isn't mistaken for load-bearing.

## Last: a Harsh crate registry

**Chosen (the user, 2026-09-27): Harsh's own registry, served as static
files** -- cargo's sparse alternative-registry index on GitHub Pages
(`harsh-lang.com/registry/`), packages (`.hrs` sources and `Cargo.toml`) as
GitHub release downloads with checksums in the index, `hrs publish` with a
GitHub token, `hrs` transpiling a registry dependency's Harsh before cargo
builds it, so its `~` macros reach the user's code. Why not crates.io: `hrs
export` unfolds `~` macros away, so a Harsh library shared as Rust loses them.
`hrs export` to crates.io stays the way to make *Rust* crates from Harsh.
Design document first (`REGISTRY-DESIGN.md`), then the build.


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
