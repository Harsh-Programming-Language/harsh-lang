# Changelog

## 0.2.5 — 2026-10-03  (`hrs_std` 0.1.4 and the Jupyter kernel 0.1.3, unchanged)

- **The deploy's link check** (the user's report: 0.2.4's site built, then the check failed): it read the cheat sheet's inline logo, a `data:image/png;base64,…` URL, as a relative link. `data:` URLs are no longer taken for links, and the new document pages, `/cheatsheet/` and `/glance/`, join the guide and the books among the pages the check leaves to `check.sh`. No change to `hrs` itself.

## 0.2.4 — 2026-10-03  (`hrs_std` 0.1.4 and the Jupyter kernel 0.1.3, unchanged)

- **The website's deploy, fixed** (the user's report from the GitHub mirror): a toolchain update made the linker's strip remove every custom section of the WebAssembly, `__wasm_bindgen_unstable` included, and wasm-bindgen failed. `site/Cargo.toml` gains a `wasm-release` profile, inheriting `release`, with `strip = false`; `wasm-opt` shrinks the file after wasm-bindgen instead.
- **The converter tells a struct literal in a `match` arm from a pattern.** In `Some(s) => E { a: s }, _ => ..`, its scan for a `=>` ran past the arm's ending comma to the next arm's arrow and read the literal as a pattern -- copied through raw, `E\ a: s` with Rust's `:` and `.into()` (found converting our own layout.rs). The scan now stops at a comma outside the groups around the brace.
- **The converter isolates a struct pattern inside a tuple** -- `(P { a }, x) =>` becomes `((P\ a), x)`, as a literal there already was; unparenthesised, the list took `x` as a field. A keyword before a paren (`let (..)`) no longer counts as a call. And a `for` pattern ends at `in`: the literal after it (`in [(P { a: 1 }, 2)]`) is a literal. A test covers each.

## 0.2.3 — 2026-10-03  (`hrs_std` 0.1.4 and the Jupyter kernel 0.1.3, unchanged)

Fixes from Docreview's issues file and the cheat sheet's check. Each turned broken Rust into working Rust or into a clear Harsh error; none of the code they touch ever compiled, so nothing working breaks.

- **H2:** parentheses isolating a pattern after `let` -- `if let (P\ x, ..) = v`, `let (P\ x, ..) = v else:` -- are dropped in the Rust; rustc warned `unused_parens`.
- **H7, second case:** a line starting with `<-` at its statement's own column became a Rust statement starting with `.`; now an error: indent it under the statement it continues.
- **H8:** a name Rust 2021 reserves for its future (`become`, `yield`, `try`, `box`, …) is written as a raw identifier, `r#become`, as rustc's hint says (the user's choice over refusing it).
- **An inline body after a bare parameter or a bounded generic** -- `fn double n: i32 -> i32: n * 2`, `fn apply<F: Fn (i32) -> i32> (f: F) -> i32: f 1` -- took the header's own `:` for the opener and became broken Rust; now right. The cheat sheet shows the bare-parameter form again.

## 0.2.2 — 2026-10-03  (`hrs_std` 0.1.4 and the Jupyter kernel 0.1.3, unchanged)

- **The Book's *Harsh at a glance* brought up to the language** (the user's request): it still taught the `;` that 0.2.0 replaced with `()`, wrote `impl Point:`, `mod geometry:` and `macro_rules! twice:` with a colon (all refused since declarations and macro headers take no mark), and named the `#:` retired on 2026-09-23. Rewritten, each checked against the transpiler; added: the `()` line, the parenthesised literal inside a literal, `hrs add`, Harsh's `macro_rules~`.
- **Harsh at a glance on the website**, at `/glance/`, with a card on the Learn page: rendered from the chapter the Book's build writes, so page and chapter never disagree. The deploy checks the page exists.

## 0.2.1 — 2026-10-03  (`hrs_std` 0.1.4 and the Jupyter kernel 0.1.3, unchanged)

- **The Fast Track to Harsh** (the user's request): a cheat sheet in the manner of Julia's, by task -- basics, operators, strings, numbers, collections, comprehensions, control flow, patterns, functions and closures, structs and enums, methods and traits, ownership, errors, iterators, modules, testing, concurrency and async, linear algebra, macros, the tools, the three ways to use Harsh, layout style. `docs/CHEATSHEET.md`, rendered like the guide, published at `/cheatsheet/` with a card on the Learn page. **Every example is checked** by `docs/check-cheatsheet.py`, run by `check.sh`: transpiled, and the Rust parsed by `rustfmt` -- so an example the transpiler mangles is caught, not only one it refuses (214 examples, 9 blocks).
- Found by that check, on the roadmap: an inline function body (`fn f …: body` on one line) after a bare parameter or a bounded generic is mis-read -- the parameter's `:` is taken for the opener -- and becomes broken Rust without an error.

## 0.2.0 — 2026-09-30  (`hrs_std` 0.1.4 and the Jupyter kernel 0.1.3, unchanged)

**A breaking release, numbered as one** (the user's decision): code that 0.1.x accepted -- a `;` ending a line -- is now refused, and Harsh is public, on its website, the Book and crates.io. Under Cargo's rules for versions below 1.0 the middle number is the breaking one, so a dependency on `harsh-lang = "0.1"` stays on 0.1.55 until its owner moves. (This release was first built as 0.1.56, never pushed or published.)


- **A block's discarded value is written `()`, not `;`** (the user's rule, from building Docreview): a block whose last value is to be thrown away ends with a line `()` -- `seen <- insert word`, and `()` beneath it -- which says so where the block's value goes. The Rust is written as Rust is: `seen.insert(word);`, no `()`. **A `;` ending a line is now refused**, with that fix in the message; `;` inside brackets (`[u8; 4]`, `m~ [1 2; 3 4]`) and in `macro_rules!` arms is untouched. A call whose last argument is an indented block takes its `()` on the line after the block. The converter writes the `()` -- except after a `let` and after a macro that produces no value (`println!`, `assert!`, `panic!` …), whose `;` changes nothing. Taught in the guide (*Statements and semicolons*), the Book (chapter 3: the lesson rewritten, a positive example added), *By Example* 13 (`()` instead of `let _ =`, and what `let _` really means).
- **Fixed, found building Docreview:** H5, a generic bound closing with `>>` left a signature's parameter groups unjoined; H6, a `const` whose value is a multi-line block lost its `;`; H7, a closure written `|a: A| (b: B)|` became broken Rust -- now an error saying a closure's parameters are separated by commas. Tests for each.
- **Breaking:** code using the `;` is refused, the error pointing at each `;` with the fix.

## 0.1.55 — 2026-09-29  (`hrs_std` 0.1.4 and the Jupyter kernel 0.1.3, unchanged)

- **`hrs add`** (the user's design): `hrs add hrs_std` writes `hrs_std = "0.1"` into the project's `[dependencies]` -- making the table if there is none, saying so, and doing nothing when the crate is listed already. It adds Harsh's crates only: the four of the distribution now, the registry's when it is built; a Rust crate is `cargo add`'s, and `hrs add serde` says so. One command per registry, so a name is never looked up in the wrong one.
- The message for a project using `m~` without `hrs_std` now leads with `hrs add hrs_std`; code that names `hrs_std` without `m~` or `v~` gets the same advice instead of rustc's "undeclared crate".
- Taught: the Book (chapter 16's opening; chapter 17, *Adding dependencies*, with Cargo's rename for a name in both worlds), the guide, `hrs_std`'s README, `hrs --help`. The registry design updated; `RELEASE.md` gains reserving the four names on crates.io.

## 0.1.54 — 2026-09-29  (**`hrs_std` 0.1.4**; the Jupyter kernel 0.1.3, unchanged)

- **`try_solve` and `try_inv`** (the user's request): `solve` and `inv` stay Julia's -- an answer, or a panic with Julia's `DimensionMismatch` or `SingularException` -- and their `try_` twins return a `Result<_, LinAlgError>`, the error saying which failure it was, for a matrix from data the caller has not checked. Rust's own pairs are the precedent: `RefCell`'s `borrow` and `try_borrow`, `Vec`'s `reserve` and `try_reserve`. `solve` and `inv` are now built on their twins, their messages unchanged word for word. Taught in the Book (16.5) and *By Example* (17.5), with a program using both forms; `hrs_std`'s README. Tests: the failures as values, the panics as before.

## 0.1.53 — 2026-09-29  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **A literal inside a literal on one line is refused unparenthesised** (the user's proposal): in `Dog\ inner = Animal\ name = x, good = true` the reader cannot see where `Animal` ends -- `good` went to `Dog`, and our own docs had said the opposite. The error offers both readings: `inner = (Animal\ name = x), good = true`, or `inner = (Animal\ name = x, good = true)`. The vertical form, the inner literal alone on its field line, needs nothing. **A trailing comma** at the end of a field list is refused too (`inner = Animal\ name = x,` on a field line was accepted); a comma ending a line inside brackets remains the group's separator.
- **Fixed:** a parenthesised literal as a field's value kept its `(` and lost its `)` in the Rust. **The converter** (`hrs-from`, the website's Converter) now parenthesises a one-line literal inside a one-line literal, as it already did inside a tuple.
- The Book's *Precedence* traps and the guide's rules corrected: they said an inline inner literal takes the outer one's remaining fields.

## 0.1.52 — 2026-09-29  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **`hrs fmt` applies the rules for breaking long lines** (the user's agreement): inline lists of three items or more -- literals, inline `struct`s and `enum`s, inline `match`es -- go one item per line, commas dropped (and a declaration's `\`); `if` chains of three clauses or more go on several lines, each `else` under its `if`; a call goes vertical only when an argument is long (20 columns or more; a string does not count), and after `=>`, `=` or a block's `:` it first moves to a fresh line one unit in -- never aligned far to the right. What asks for judgment stays the writer's: a literal inside a literal, a `let`'s three-clause `if`, a call deep in another. Six tests; the formatter's corpus test (361 files) finds every book program already formatted; the guide's two inline-form examples now follow the rules. `docs/FMT.md` records it.
- The formatter's safety check treats an arm whose body is a one-expression block as the bare arm (`p => { e }` = `p => e,`), and never breaks after a field's `=`.
- **The Book, as the user wrote it:** chapter 3's `sign` example in its compact form (`if n > 0: 1`, then `else` with the nested `if` beneath it), the sentence describing its shape reworded to match; and no blank line between an `if`'s body and its `else` anywhere in the Book (ten places, among them `cmp_display` in chapter 10). All 249 programs rebuilt and run.

## 0.1.51 — 2026-09-29  (no change to the transpiler)

- **Long lines broken, by the user's rule for handwritten Harsh:** a chain of more than two steps, a struct literal of more than two fields, more than two `if`/`else` clauses, and a `match` on a long chain go on several lines. So does a call or macro call made long by its arguments -- by the writer's judgment, not a count: a long list of short arguments stays on its line -- and a literal holding another literal. The rules ask for judgment and no tool enforces them; where a break would have been mechanical (arguments aligned far to the right), the code names an intermediate value instead. The companions' programs keep outputs byte-for-byte unchanged. Applied to the four books' programs, the website's source, the README and the docs (57 lines; `examples/`, the transpiler's test fixtures, untouched), and stated in the guide's *Layout style*, *Breaking long lines*. Where a line taught an inline form, the lesson now teaches it within the rule (two arms, two variants) or shows the binding by indentation. All four books rebuilt.
- Found on the way, recorded on the roadmap: an inline `if` block on the line after `let x =` is not transpiled; `hrs fmt` flattens a nested `else` in an `if` bound by `let`, and pushes a `match`'s arms far right when its head is a broken chain; the companions' programs were never formatted by `hrs fmt`.

## 0.1.50 — 2026-09-29  (no change to the transpiler)

- **Harsh in colour on GitLab and GitHub.** Neither knows a language called Harsh, and our Harsh blocks were untagged, so both showed them plain (the user's screenshot of the README). Every Harsh block in the Markdown they display -- the README, the guide, the tutorial, the macros guide, the other docs, the four books -- is now fenced ```` ```rust harsh ````: they colour it as Rust by the first word, and Harsh's own tools read the second and treat it as Harsh, highlighting it as before in the books, the notebooks and the website. Real Rust stays ```` ```rust ````; the guide's fragments are ```` ```rust fragment ````; shell commands, which were untagged too, are ```` ```sh ````; the wordmark and file listings ```` ```text ````. The four books' builders write the new tag; the guide's checker refuses an untagged fence, and `check.sh` any in the repository's Markdown. All four books rebuilt: 386 programs.
- Real Harsh colours on GitLab would need a Harsh lexer in Rouge, GitLab's highlighter (a contribution upstream); GitHub's needs a language used in 200 repositories. Recorded on the roadmap.

## 0.1.49 — 2026-09-28  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **Precedence, documented** (the user's request). *The Harsh Programming Language* gains an appendix, *Precedence*: the eighteen levels from atoms to assignment, dereference case by case, the pipes' rules, the traps (`f -1` and `f *x` are arithmetic, `f x |> g` pipes two values, `f a.b` is a path, `*p <- field` dereferences the field), and a program showing each rule with its output -- in words, as the Book shows no Rust. *The Harsh Language Guide* gains a *Precedence* section with the same rules beside the Rust they become, every line matched against the transpiler by `check.sh`. *Harsh by Example* gains a *Precedence* page in chapter 7; *The Harshonomicon* states the dereference rule where raw pointers are dereferenced; chapter 2 and *Harsh at a glance* point to the appendix.
- **Fixed, a regression of 0.1.44:** a `trait`, `impl` or `enum` written with Rust's braces already, `impl Error for E {}`, gained a second pair, `{} {}`. Found rebuilding *Harsh by Example* (its custom-error program); pinned by a test. All four books rebuilt: 386 programs, every one built and run.

## 0.1.48 — 2026-09-28  (no change to the transpiler)

- **Fixed: the Learn page's links to the books.** Its cards linked relatively (`book/`), which resolved to `/book/` while the site was one page at `/learn`; with static generation each page is a folder, `/learn/`, and the links became `/learn/book/` -- "Not found", on every book (the user's screenshots). They are now absolute, `/book/`. Two guards: `check.sh` refuses a relative link in the site's source, and the deploy follows every internal link on every pre-rendered page and fails, before publishing, if one leads nowhere.

## 0.1.47 — 2026-09-28  (no change to the transpiler: `hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, as before)

- ***The Harshonomicon*, checked against the original.** The user supplied the current *Rustonomicon*, and the companion now follows its table of contents one for one: its numbering (chapters 1 to 12, the introduction unnumbered), its titles, *References* and *Aliasing* as two sections (with a new program: the compiler refusing a second `&mut`), *Implementing Vec*'s eleven sections and *Implementing Arc and Mutex*'s *Arc* with its five, each pointing into the program given under *Final Code*, and *Beneath std*'s *#[panic_handler]*. Details the original adds, now stated: its list of undefined behaviours, `repr(align(n))`, drop check's `#[may_dangle]`, leaking's `Drain` and `thread::scoped`, `BinaryHeap::sift_up`'s guard, and FFI's safe interface (the program gains one). 36 programs, every one built and run.
- Both companions are on the website's Learn page (since 0.1.44 and 0.1.45), each with its card, published by the next push.

## 0.1.46 — 2026-09-28  (no change to the transpiler: `hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, as in 0.1.45)

- ***Harsh Design Patterns*, checked against the original.** The user supplied the current *Rust Design Patterns* site, and the companion now follows it exactly: its order (anti-patterns before functional programming; *Additional Resources* last), its page names (*Constructor*, *RAII Guards*, *Compose Structs*, *Object-Based APIs*, …), and its FFI idioms as three pages with a program each (errors -- flat enums, structured enums; accepting strings; passing strings). Four pages corrected in substance: *Avoid complex type bounds with custom traits* shows the original's technique, an associated type implemented for closures (the supertrait bundle kept as a variant); *Functional Optics* shows the iso, the poly iso and the prism; *On-Stack Dynamic Dispatch* uses deferred conditional initialisation; *Design principles* lists all fourteen. 38 programs, every one built and run.
- The books' retired-spelling guard no longer mistakes a bound inside a struct's generics (`struct Value<G: Getter>`) for the retired `struct P:`.

## 0.1.45 — 2026-09-28  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **A new book: *The Harshonomicon***, a companion to *The Rustonomicon* (Apache-2.0): 13 chapters in its order -- safe and unsafe, data layout, ownership and lifetimes, conversions, uninitialised memory, resource management, unwinding, concurrency, a `Vec` and an `Arc` built from scratch, FFI (C's `qsort` with a Harsh callback), beneath std -- 35 programs, every one well-defined, built and run; prose of its own, shorter than the original on purpose, each section linked. On the website's Learn page, at `/nomicon/`.
- **Fixed, found writing it:** an empty `enum` (`enum Void`, a type with no values) came out `enum Void;` -- now `{}`; a declaration with no body, several parameters and no return type (a function in an `extern "C"` block, a trait method) kept its parameter groups apart when a parameter's type held a `:`; and a function-pointer type with qualifiers, `extern "C" fn (A) (B) -> C` or `unsafe fn …`, was not joined into Rust's `fn(A, B) -> C`. Each pinned by a test.

## 0.1.44 — 2026-09-28  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **A new book: *Harsh Design Patterns***, a companion to *Rust Design Patterns* (MPL-2.0): its idioms, design patterns and anti-patterns in the same order -- 9 chapters, 35 programs in Harsh, every one built and run -- with prose of its own, what Harsh changes on each page (pipes and partial application in *Strategy* and *Command*, comprehensions in *Programming paradigms*), and a link to each original page. On the website's Learn page, at `/patterns/`.
- **Fixed: a declaration with no body whose header holds a `:`.** `trait Loggable: Debug + Display` and `impl<T: Debug> Loggable for T` -- a marker trait and its blanket implementation -- were misread, the `:` taken for an inline block that swallowed the rest of the line. A declaration's `:` is now its header's own, and an empty `trait` or `impl` becomes Rust's `{}`. The guide states it. Found writing the book.

## 0.1.43 — 2026-09-27  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **Hover in Harsh.** rust-analyzer's answer is Rust; `hrs-lsp` now converts its signatures and paths with Harsh's own converter (the one behind `hrs-from` and the website's Converter) and shows them as Harsh, coloured as Harsh: `fn adding (x: i32) (y: i32) -> i32`, `std.slice.Iter`. A line the converter cannot take, and a long documentation example, stay Rust -- never half translated.

## 0.1.42 — 2026-09-27  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **Hover, coloured.** The first working hover on the user's Mac showed the signature in plain text: `hrs-lsp` told rust-analyzer nothing of what the editor can show, so it answered plain text. It now asks for Markdown, and a hover's Rust comes in a code block the editor highlights; completion's documentation too.

## 0.1.41 — 2026-09-27  (no change of code: `hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, as in 0.1.40)

- **The version, aligned.** 0.1.40's tree was pushed under the message and tag 0.1.39; a version is never reused, so the next is 0.1.41 -- the same code as 0.1.40, to be committed and tagged `v0.1.41` (`RELEASE.md`, step 2).

## 0.1.40 — 2026-09-27  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **Hover, go-to-definition and completion answer on a real rust-analyzer.** It loads nothing under a project's `target/` from disk -- the generated Rust included -- so every question came back empty (found on the user's Mac with `HRS_LSP_LOG`: each hover mapped right, each answer `null`). `hrs-lsp` now hands it the text of every generated file, as an editor hands it an open file, when it starts and after each save; completion updates that text in place. The suite's fake rust-analyzer now ignores unopened `target/` files as the real one does, so the tests catch this.

## The website, 2026-09-27, later  (no new version: the workflow and `site/` are not in the crate)

- **harsh-lang.com is published with every page pre-rendered as HTML** (static generation): the deploy builds `dx bundle --web --ssg`, checks each page's content is in its HTML, and publishes that. Tried first on the user's Mac. The separate trial job is retired.
- The first deploy found `dx` 0.7.10 building both sides and pre-rendering nothing (and in debug): the build is now `--release`, and a step does the pre-rendering by hand when `dx` does not -- the site's own server, asked for every page. Nothing was published by the failed run. Two compiler warnings in the site fixed (an unused import, an unneeded `mut`).

## 0.1.39 — 2026-09-27  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **Completion in `.hrs` files**, through rust-analyzer, on the text as you type it.
- **Hover and go-to-definition, fixed after the first try on a Mac**: a name passed to a macro (`apply~ adding (4)`) now maps; `hrs-lsp` waits for rust-analyzer to finish loading instead of answering nothing; Harsh's distribution is handed to it only when a project uses it; and the rust-analyzer inside VS Code's extension is used when none is installed with rustup.
- **`HRS_LSP_LOG`**: set it to a file, and `hrs-lsp` writes every question to rust-analyzer and every answer there.
- **The website's static generation**, tried by a CI job that deploys nothing (`site/DEPLOY.md`, part D); the site gains an inert `ssg` feature.
- Taught: the Book's *Setting up*, the guide, the language-server page, the Install page.

## 0.1.38 — 2026-09-27  (`hrs_std` 0.1.3 and the Jupyter kernel 0.1.3, unchanged)

- **Types on hover and go-to-definition in `.hrs` files**, in every editor that runs `hrs-lsp` (VS Code, Zed, Neovim, Helix …). The server asks rust-analyzer about the Rust a file became and maps positions both ways through the source maps -- `SourceMap::find` (`.hrs` to `.rs`), new, beside `locate`. rust-analyzer starts on the first request, one per project; answers follow the last save, and a line changed since is answered after the next save rather than wrongly; no rust-analyzer installed is said once (`rustup component add rust-analyzer`). Tested end to end against a fake rust-analyzer (`tests/lsp.rs`); the real one is the user's to try. Completion comes next, by the same path.
- The user's rulings (2026-09-27): the distribution shown to rust-analyzer by `hrs`, not a file in the project; answers from the last good translation for unchanged lines; hover and definition first.
- `&a[range, range]` stays an error; its review is last on the roadmap.

## 0.1.37 — 2026-09-27  (`hrs_std` 0.1.3; the Jupyter kernel 0.1.3, unchanged)

- **Matrix views are values, and sound.** `a <- view (1..) (..=1)` borrows a window onto a matrix, as Julia's `view(a, 2:3, 1:2)`: nothing copied, guarded by the borrow checker like any borrow, printed, used in `.*`, and `<- copy$` for a matrix of its own. `a <- view_mut 2 (..) <- fill 0` writes through. `hrs_std` has no `unsafe` any more: views were references forged over a zero-sized slice, which Miri found undefined behaviour under Stacked Borrows (the known issue since 0.1.30).
- **Breaking: `&a[range, range]` no longer compiles.** An index is one element, `a[i, j]`; a part of a matrix is a `slice` (a copy) or a `view` (a borrow). Accepted for now, to be reviewed once the user has used matrices in Harsh -- the index form may come back if a sound way is found.
- **The converter keeps empty records**: an empty variant stays `Home {}` (it came back `Home { (), }`), and an empty `struct Empty {}` keeps its line (the next item was glued onto it, `struct Empty { } fn g$`, and misread). `v[0].1` and an `if` as a macro argument, recorded as gaps, convert and read back: now pinned by tests.
- **Taught**: the Book's chapter 16, *By Example*'s matrices, the guide's slicing table, `hrs_std`'s README; the Book's chapter 17 on sharing Harsh code. Harshlings 0.1.7: `matrices05` uses `view`.
- The Book's build removes each program's temporary project once its output is taken (the 248 of them filled the disk).

## 0.1.36 — 2026-09-27  (the Jupyter kernel 0.1.3, unchanged)

- **A statement at a file's top level is an error that says so.** Rust's top level holds items only; a `let x = f 3 2` there used to go through untouched -- Harsh's call syntax and all -- and the website's Converter showed broken Rust. Now: *a `let` at the top level: Rust's top level holds only items (`fn`, `struct`, `enum`, `impl`, `use`, `const` …); it belongs inside a function -- `fn main$:`, for one; or, for a value of the whole program, `const` or `static`*, pointing at the line. (rustc caught a plain `let x = 3` there too, in a project; but not in the Converter, which has no rustc, and not clearly when the line held Harsh, which it passed on untransformed.) Items, attributes (`#![…]` included) and macro calls stay allowed, as in Rust; notebook cells, holes and doc examples are wrapped in a function, so statements stay at home there.
- **The website's editor drops a stale edit**: Enter or Tab waited for the columns, and a change to the text meanwhile left the edit at old positions, scrambling it (`3, 2` once became `3 3 32`). Every sample of the site now round-trips both ways in a test.
- The home page's boxes, in one form: *Functional Programming* -- Comprehensions, Pipes, and Partial Functions; *Linear Algebra* -- Vectors, Matrices, and Matrix Arithmetic.

## The website, 2026-09-27, later  (no new version: the crate is as in 0.1.35)

- **A copy button on every code block** -- on the site and in the books -- discreet, shown on hovering the block.
- **Fixed: the editor could scramble text**: Enter and Tab wait for the page's layout answer, and a change to the text meanwhile left their edit at stale positions (a round trip once showed `3, 2` as `3 3 32`). A stale edit is now dropped. The language's tools were never at fault; a new test round-trips every sample of the site both ways.

## 0.1.35 — 2026-09-27  (the Jupyter kernel 0.1.3, unchanged)

- **The Converter works both ways, Harsh ⇄ Rust.** Rust to Harsh gives exactly what `hrs-from` writes: both call the new **`driver::convert_str`** -- the converter, the formatter when the result transpiles, the doc examples -- so they cannot disagree (`hrs-from` keeps `--raw`). Two buttons choose the direction: choosing the other one turns the output into the input, so the text always matches the direction (and while the output is an error, the other direction waits); the samples load in either language; Run opens the Rust side. In Rust, the editor indents as editors do for braces: four spaces.
- **Code blocks keep their layout** on the website: a line too wide scrolls inside its box, instead of wrapping.
- **`Home`** leads the navbar.
- **The books link back to https://harsh-lang.com/learn**, the page that lists them.
- The home page's last box reads *Artificial Intelligence & Data Science*.

## The website, 2026-09-27  (no new version: `site/` is not in the crate, so 0.1.34 is unchanged)

- **A new home page**: a jumbotron across the top, 75% of the screen's height -- a night-time desk with Harsh on its screens -- where four boxes take turns, typing themselves and erasing: *Rust without the braces*; *Functional Programming* (Partial Function Application, Pipes); *Linear Algebra* (matrices and vectors, matrix arithmetic); *Artificial Intelligence & Data Science*. Titles in Rust's orange, the lines under them white.
- **The navbar**, the same on every page, lies over the image on the home page, translucent and blurring it; on phones its links fold behind a three-line menu button, on every page.

## 0.1.34 — 2026-09-26  (the Jupyter kernel 0.1.3; everything else as in 0.1.33)

- **Matrices in Jupyter notebooks.** A cell using `v~`, `m~` or anything else of `hrs_std` now just works: the first time a cell's Rust names `hrs_std`, the kernel writes Harsh's standard distribution into its own folder and gives `hrs_std` to evcxr, once per session. The first such cell takes a minute or two while nalgebra compiles. Needs kernel 0.1.3 and `hrs` 0.1.34.
- **`hrs dist <dir>`** writes Harsh's standard distribution -- `hrs_std`, `hrs_proc_macro`, `hrs_quote`, `hrs_syn` -- into a folder, as `hrs build` does inside a project, for whatever builds outside one.

## 0.1.33 — 2026-09-26  (the Jupyter kernel 0.1.2, and everything else, as in 0.1.32)

- **The website is at https://harsh-lang.com/**, its own domain; the old github.io address redirects there. `site/Dioxus.toml` has no `base_path` any more, and `dx serve` serves at the root.
- **The Book, *By Example* and the guide show Harsh's logo in the browser's tab, and link back to harsh-lang.com** above every page -- the same pages in the repository, on the website, and offline.
- **A discreet back-to-top arrow**, in the corner of every page of the website and of the books, once a page has scrolled a screen or so.
- **The website's Install page lists the whole toolset**: VS Code, Jupyter, Zed (coming), other editors through `hrs-lsp`, Harshlings.
- **Installing the Jupyter kernel**: `python -m` for every Python command, `--no-deps` to reinstall the kernel alone, and away from the checkout (`docs/JUPYTER.md`, `RELEASE.md`). The old commands could install into one Python and run another, or upgrade Jupyter's own packages under other tools.
- `site/DEPLOY.md` records how the domain was set up, and the mirror's token as it must be.

## 0.1.32 — 2026-09-26  (the Jupyter kernel 0.1.2; everything else as in 0.1.31)

0.1.31 was handed over and never published; its number is spent, and 0.1.32 carries all of it (below) with these additions:

- **The Jupyter kernel is ready for PyPI**, as `harsh-kernel` 0.1.2: a `README.md` for its PyPI page, the licence as an SPDX string (`license = "MPL-2.0"`, which setuptools will require from 2027), `setuptools>=77`.
- **`RELEASE.md`**, at the root: publishing and installing every product -- the crate, the Jupyter kernel, the editor extensions, Harshlings, the website -- in order, in one file.

## 0.1.31 — 2026-09-26  (`hrs_std` 0.1.2, `hrs_proc_macro` 0.2.0, `hrs_quote` 0.1.0, `hrs_syn` 0.1.0, VS Code extension 0.1.5, Harshlings 0.1.6, all unchanged; the Jupyter kernel 0.1.1)

- **The website, in the repository**: `site/`, a Dioxus app written in Harsh -- the home page, the Book, *By Example* and the guide, a Converter (Harsh to Rust in the browser) and a Playground (an editor that indents, closes brackets and highlights as Harsh does, and a console; the program runs on the Rust Playground, with the part of `hrs_std` it uses sent along). Published to GitHub Pages from the GitHub mirror by `.github/workflows/pages.yml`.
- **A Harsh app can use a Harsh library by path.** `hrs` never transpiled the library, though the Book's chapter 17 said it did; `hrs build` now transpiles Harsh path dependencies first, recursively, only what is stale.
- **Re-exports of Harsh macros**, as in Rust: a library writes `pub use hello_macro_derive.HelloMacro`, and its users call the macro through it. The `use` is resolved by `hrs` and leaves no trace in the Rust.
- **`hrs export` carries `hrs_std` as the crate's own code**, pure Rust: the items of `hrs_std` the program reaches, and only those, become the crate's module `matrix`, with nalgebra and num-traits as its dependencies in place of `hrs_std`.
- **The Jupyter kernel, 0.1.1, reports errors at the cell's lines**, read from evcxr's real output (its gutter numbers, not rustc's `-->`, which evcxr does not print).
- **`hrs fmt` writes one space inside a hole's marks**, `@: x :@`; a multi-line hole's layout and `@@:` are left as written.
- **Fixed: an `if` with a string continued over lines, as a struct literal's field value**, was refused after conversion; the transpiler's brace check asks whether a token begins a line, not whether the line number changed.
- **`hrs-from` formats only a conversion that transpiles**, so a converter gap is reported as it is instead of reflowed.
- **Fixed: a character of several bytes inside a hole panicked the transpiler** -- `view! { <p>{@: "…" :@}</p> }` stopped `hrs` with "byte index … is not a char boundary". The scan for a hole's closing `:@` compares bytes now, which find the same ASCII marks. Found writing the website, whose Converter and Playground run the transpiler in the browser, where a panic stops the page.
- **`driver::single_file`**: a Harsh program's Rust as one file, with the part of `hrs_std` it uses following it as an inline `mod hrs_std { … }` -- shaken as `hrs export` shakes it, the program's lines where they were. For places that take one file and have no `hrs_std`, such as the Rust Playground, to which the website's Playground sends its code. `dist` and `shake` are part of the library without its features now (they depend on nothing).

## 0.1.30 — 2026-09-25  (new: Harsh's standard distribution -- `hrs_std` 0.1.2, `hrs_proc_macro` 0.2.0, `hrs_quote` 0.1.0, `hrs_syn` 0.1.0 -- shipped inside `hrs`; Harshlings 0.1.6)

- **Harsh's standard distribution ships inside `hrs`**, as Rust's `std` and `proc_macro` ship with the toolchain: `hrs_std`, `hrs_proc_macro`, `hrs_quote` and `hrs_syn`. A `Cargo.toml` names them as Rust names `syn` and `quote` -- `hrs_quote = "0.1"` -- and `hrs` serves them from what it carries, never from crates.io, always at its own version. Only `harsh-lang` is published.

- **Harsh's own proc macros are complete, mirroring Rust's in all three kinds** -- custom derive, attribute-like, function-like -- and written with Harsh's `syn` and `quote`. The Rust Book's `HelloMacro` and `route` examples, written in Harsh, run end to end.
- **`hrs export` is pure Rust** (the user's rule: the Rust translation carries nothing of Harsh): no `[package.metadata.harsh]` in the exported manifest; a Harsh macro crate is refused, having no Rust meaning. The design for the rest -- `hrs_std` tree-shaken into the export, re-exports, two kinds of package -- is in the development notes (`PACKAGING.md`), for the releases after this one.
- **Harsh declarative macros, the matcher** (the user's ruling): a metavariable is written in its parentheses, `($name:spec)`, part of the construct as `$( … )*`'s are the repetition's; a bare `$name:spec` is refused, naming the form. Every other token in a matcher is literal, further parentheses included; a specifier gives its capture meaning -- to `($v:expr)`, `(1+2)` and `((1+2))` are one expression, while `:ident` is one identifier token, as in Rust. An arm whose matcher is one metavariable is written `(($e:expr))`. The prelude, the Book, the guide, *By Example* and Harshlings are in the new form. **This changes existing macros:** a bare metavariable must now be written in its parentheses.
- **Known issue -- matrix views and Stacked Borrows.** A borrowed view, `&a[0..2, ..]`, packs its window into a reference to a zero-sized slice that points at the matrix, and widens it back when read. Miri reports that as undefined behaviour under Stacked Borrows, its default model; Tree Borrows accepts it, and every other part of `hrs_std` is clean under both. Released knowingly; making views sound is the first work after this release.
- **Fixed:** an inline attribute before a `struct` with fields beneath; a `struct`'s `[where …]` on its own line beneath the header, silently emitted as a field, is now refused, naming the header-line form; after an upgrade, unchanged files were retranspiled on every build (a stamp now records which `hrs` last ran); three `hrs-from` converter gaps (a `match` after a guard arm, an attribute between doc comments, an `if` as a macro argument worked around); a macro call at item level; the navigation of every rendered document; Harshlings' `verify.py` leaving a folder behind per program.

- **Macros documented by who wrote them** (the user's architecture): **Harsh macros (~)**, your own in Harsh; **Rust macros (!)**, your own in Rust, still supported; **Rust DSLs (!)**, imported from Rust libraries (calling rules, braces and holes); **Harsh DSLs (~)**, imported from Harsh libraries -- the first two each with declarative and procedural (function-like, derive, attribute). The Book's chapter 23 gives each a numbered section (23.5-23.8; parentheses and the summary move to 23.9-23.10), with two new verified examples: a `macro_rules!` inside a Harsh file, and the Rust Book's `HelloMacro` -- a Rust derive crate used from Harsh. The Language Guide, `docs/MACROS.md` and *Harsh by Example* 7.3 follow the same architecture. Removed on the way: the guide's paragraph saying procedural macros were "still in development". Harshlings 0.1.6 maps its macro exercises onto it, fixes its stale topic list, and `verify.py` no longer leaves a 13 MB folder behind per program.
- **`hrs_proc_macro` 0.2.0** (step 1 of Harsh's `syn`/`quote`): Rust's token types, with where each token stands, lexed by the transpiler's own lexer; `to_string` and `parse` as before.
- **Errors at the programmer's token** (step 2): a token a proc macro copies from its input keeps its origin, so rustc's error on it lands on the programmer's code, as in Rust; a macro rejects code with `compile_error!` and an input token's span (`set_span`). A macro's own `parse` gives `call_site` spans, as Rust's does.
- **Fixed: a macro call at item level** -- `compile_error! "msg"` outside a function -- was left unapplied, invalid Rust.
- **`quote~`** (step 3): `hrs_quote` 0.1.0, new. In a macro crate that says `use hrs_quote.quote`, `quote~ do:` takes a template laid out as the output reads, with `#name` interpolating anything that implements `ToTokens`; a multi-line value is indented to where its `#name` stands, and an interpolated input token keeps its span. The Rust Book's `HelloMacro` derive, written in Harsh with `quote~`, runs end to end. Repetition (step 4): `#( … )*` and `#( … ),*` iterate every `#name` inside, side by side; a repetition that begins a line repeats as lines at that column, one elsewhere runs inline. Columns are measured when the macro runs, from the text actually assembled.
- **`hrs_syn`** (step 5): `hrs_syn` 0.1.0, new. `parse_macro_input! { input as DeriveInput }` parses a Harsh item into `syn`'s shapes -- `DeriveInput` with its attributes, visibility, name, `Generics` (`split_for_impl`) and `Data` (a struct's `Fields`, named, unnamed or unit; an enum's `Variant`s; a union) -- from Harsh's item grammar; `Error::to_compile_error` points at the programmer's token. The Rust Book's `HelloMacro`, written in Harsh line for line, runs on a plain and a generic struct.
- **Harsh attribute-like macros**, mirroring Rust's: `#[proc_macro_attribute~]` on a `pub fn` taking two streams (one is refused, as rustc refuses it), called `#[route~ GET "/"]` on any item; the macro receives the arguments as written and the item with all its other attributes, and its output replaces the item. `hrs_syn` gains `ItemFn`. The Rust Book's `route` example, written in Harsh, runs end to end.
- **Navigation:** every rendered document lists its headings to four levels -- the Book's chapters were missing -- with one H1 per page and no broken links.
- **The library builds without its default features**, the pure language with no dependencies; `check.sh` keeps it so.

## 0.1.29 — 2026-09-24  (`hrs_proc_macro` 0.1.0, `hrs_std` 0.1.2, Harshlings 0.1.5, all unchanged)

- **Harsh derives, mirroring Rust's** (the user's rule: for all Harsh proc macros, mirror Rust's). A macro crate registers `#[proc_macro_derive~ Describe]` on a `pub fn` -- `#[proc_macro_derive~ Show (attributes show)]` to declare a helper attribute, Rust's `attributes(…)`. `#[derive~ Describe]` above a `struct`, an `enum` or a `union` runs it on that item: it receives the item with its attributes and doc comments and without any derive attribute, Rust's included; what it returns is added after the item, which it cannot change; its helpers leave the item once it has run. It sits beside Rust's `#[derive Debug]`; several run in the order written. Errors name the derive at its attribute; rustc's say "in the expansion of `#[derive~ Describe]`".
- **Fixed:** `#[proc_macro~] pub fn f` written inline was refused as over-indented.
- **The Book** teaches Harsh's proc macros in 23.5, function-like and derives, with two-crate examples (its harness now builds crates side by side).
- **Known bugs recorded** (each an ignored test, reported by every `cargo test` until fixed): an inline attribute before a `struct` with fields beneath; `hrs-from` writing a `match` after a guard arm; `hrs-from` writing an `if` with a continued string as a field value.

## 0.1.28 — 2026-09-24  (`hrs_proc_macro` 0.1.0, new; `hrs_std` 0.1.2, Harshlings 0.1.5, both unchanged)

- **Harsh's own proc macros, the function-like form** (ruling 17). A proc macro is an ordinary Harsh `pub fn` marked `#[proc_macro~]`, from `hrs_proc_macro.TokenStream` to `TokenStream`; a crate says `proc-macro = true` under `[package.metadata.harsh]`, and a crate that uses it lists `proc-macros = ["../path"]`. `hello_macro~ world` runs the macro when `hrs` transpiles the caller, and its expansion -- Harsh -- is spliced where the call stood, so the generated Rust holds no macro. `hrs` builds a small runner under `target/hrs/proc-macros/` with cargo; later builds are a no-op, and changing a macro retranspiles its callers.
- **Errors name the macro and the call**, whoever finds them: the macro's own error or panic (`` `boom~` failed: panicked: not yet ``), an expansion that does not read as Harsh, or rustc (mapped back to the call, "in the expansion of `name~`").
- **`hrs expand`** finds the file's project from the file, expands its proc macros, and names any `name~` nothing defines instead of printing it back silently.
- **`#[name~ (…)]` and `#[derive~ Name]` are refused** until they are built: they had reached rustc as `#[name!(…)]`, Rust nobody wrote. `#[proc_macro~]` is Harsh's and never reaches rustc.
- **`hrs_proc_macro` 0.1.0**, the runtime: `TokenStream` (text) and the runner's entry point.

## 0.1.27 — 2026-09-24  (`hrs_std` 0.1.2, Harshlings 0.1.5, both unchanged)

- **A diagnostic inside a hole points at the `.hrs` line and column.** Each restored body maps piece by piece: a DSL segment to itself, a hole's Rust to its Harsh code. `error[E0277] --> src/main.hrs:8:18` is the `+` of `@: count + "x" :@`.
- **The map stayed right after a body, and after a `macro_rules!` zone.** `dslzone::restore` and `rawzone::restore` now move the map by what they put back (the latter had never done so: any file with a `macro_rules!` zone mapped wrong after it), and a placeholder that outgrew its body's first line no longer pushes every source offset after it.

## 0.1.26 — 2026-09-23  (`hrs_std` 0.1.2, Harshlings 0.1.5, both unchanged)

- **A macro's DSL stream is written in braces** (the user's rule: parens isolate Harsh code, braces delimit a whole stream). `hrs-from` writes `sql!(SELECT name FROM t)` as `sql! {SELECT name FROM t}`, `html!(…)` and `json!(…)` likewise, with their Rust in holes; only the delimiter changes, and `hrs` keeps the braces. Until now such a stream was written in isolating parens and came back `sql!(SELECT(name, FROM, t))`, silently. A comma-separated list of expressions stays juxtaposed, `vec! 1 2 3`, `matches! x (Some 1 | None)`.
- **`json!` objects and arrays are structure**: only their Rust values become holes, `"text": @: question :@`.
- **A closure's body is written beneath its prototype** whatever its return type: `-> Option<Vec<String>>:` (the `>>` token had made the converter isolate the body as an argument, which transpiled to a turbofish).
- **An expansion is read as source** (the user's rule: a `~` macro's author expands to valid Harsh, holes included; anything else is an error). A brace body produced by an expansion is Rust with holes, like a typed one. The guide's `filled~` writes its holes.

## 0.1.25 — 2026-09-23  (`hrs_std` 0.1.2, Harshlings 0.1.5, both unchanged)

- **`hrs-from` writes the holes** (the user's algorithm, 2026-09-23). Converting a Rust file keeps each DSL body as written — `view!` markup, an `rsx!` tree, `select!`'s arms — and turns every piece of Rust code in it into a hole of Harsh: `{@: active :@}`, `class:active=@: move || current <- get$ == i :@`, `onclick: @: move |_| reset$ :@,`, `for item in @: items <- iter$ :@ {`, `@: Some v :@ = @: rx <- recv$ :@ => …`. A literal stays as written. A hole nests, and its body's lines follow the converter's layout.
- **No silent fallback:** a piece that cannot be written as Harsh stops `hrs-from` with the line named (the user's ruling). Each hole is checked by transpiling it back, and must be a complete expression.
- The sr-auto site (22 files, 2,693 lines of Leptos): 233 holes, 0 errors, no Rust left outside a hole, every file the same program after the round trip.

## 0.1.24 — 2026-09-23  (`hrs_std` 0.1.2 unchanged; Harshlings 0.1.5)

- **A Rust macro's brace body is Rust, and its holes are Harsh** (ruling 16, the user's principles of 2026-09-23). `view! { … }`, `rsx! { … }`, `tokio.select! { … }`, `quote! { … }`: the body is copied byte for byte, one line or many; a hole `@: … :@` — always closed — holds Harsh, transpiled in place, and may nest. `@@:` is a literal `@:`. `hrs-from` copies brace bodies as Rust; `hrs fmt` never touches them.
- **Retired:** the old DSL modes — `view! do:` markup ("HSX"), `rsx! do:` brace trees, a macro `do:` body, `#:` blocks, `m!\\` — each refused with the braces named. A block passed as an argument is `m! (do: …)`.
- **A literal cannot be applied:** `assert_eq! a + 1 b` was `assert_eq!(a) + 1(b)`, silently; it is refused. **`@:` outside a body** is refused. **A turbofished function applies:** `parse_kv.<String> "x"` is `parse_kv::<String>("x")`.
- **Fixed:** the guide example `(twice~ 4, 0)`; `examples_transpile` now expands macros.

## 0.1.23 — 2026-09-23  (`hrs_std` 0.1.2 unchanged; Harshlings 0.1.4)

- **A bracket after a macro's bang is an argument, an array** (ruled 2026-09-22, A1): `m! [a, b] c` is `m!([a, b], c)`. Rust's bracket call is written as Harsh writes any call — `vec! 1 2 3`, `vec!$`, and `vec! { 0; 4 }` for a stream that is not a list. Written the Rust way, `vec![1, 2, 3]` would now mean a vector of one array, so it is refused with the Harsh spelling named; `vec! ([1, 2, 3])` passes one array on purpose. The corpus, the Book, *by Example*, Harshlings, the guide and the prelude's `m~` and `v~` moved in one step (94 sites).
- **`hrs-from`** writes a macro's bracket list as juxtaposition and a `;` stream in braces.
- **Fixed: a block argument followed by another block argument ended the call** — `f (P\ x = 1) (Q\ y = 2)` was `f(P { .. })(Q { .. })`. Plain arguments after a block always worked.
- **Fixed: a `~` call's stream was judged by rules for Rust's `!` macros** on the path the editor and the formatter's guard read, so the guide's own `pair~ (1, 2)` showed as an error. A `~` stream is its macro's.
- **Fixed: a crash in the emitter** when an expansion's tokens carried spans past the end of a short file.

## 0.1.22 — 2026-09-21  (`hrs_std` 0.1.2)

- **Ruled, not built (2026-09-22):** foreign DSLs — `view! { … }` and `rsx! { … }` bodies verbatim, Harsh holes `@: … :@`. Ruling 16 in the design notes; first on the roadmap.
- **The Book teaches slicing and broadcasting:** chapter 16 gains *Taking a part* and *Element by element* (242 snippets). Harshlings 0.1.3 adds `matrices05`–`07`.
- **`f<>` takes up to three arguments** (`f64.clamp<> a 0.0 1.0`, `f64.mul_add<> a b c`), stretched together.
- **`hrs` names the missing crate** for a file that uses `.*` or `f<>` without `hrs_std` in its manifest, as it did for `m~` and `v~`.
- **Fixed: `hrs fmt` could change a matrix.** Past its width it listed the entries of `m~ [ … ]` one per line, and inside `m~` a line break is a row: a row of ten became a column of ten, silently. The formatter now never adds or removes a line break inside a `~` call's stream — the stream's syntax belongs to the macro. An over-long `~` line stays as its author wrote it.
- **Three converter fixes**, all found by the round trip over the transpiler's own source: a `match` used as an operand with more of the expression after it (`a && match k { .. } && b`) had its tail written as a separate statement, *silently*; a bracket inside a character literal before a `match` (`Tk::Open('[')`) made `hrs-from` write the retired `match x:`; and a struct literal as a closure's body across lines was written as a block.

## 0.1.21 — 2026-09-21  (`hrs_std` 0.1.1)

- **Slicing, Julia's `a[1:2, :]` in Harsh's ranges.** `a <- slice (0..2) (..)` copies; `&a[0..2, ..]` borrows — a view, as `&v[1..3]` is of a `Vec`, with `<- copy$` for the copy. `..` alone is Julia's `:`; an axis taken by a number is dropped, so `a[1, ..]` is a vector; and what is read can be written: `a[1, 1] = 50`, `a[2, ..] <- fill 0`.
- **A top-level comma in an index makes a tuple:** `a[i, j]` is `a[(i, j)]`. A rule of the language, for any type indexed by a tuple.
- **Broadcasting, Julia's dotted operators:** `a .* b`, `.+`, `.-`, `./`, shapes stretched as Julia stretches them. Written to Rust as `a * hrs_std::DOT * b`, so precedence is Rust's own and nothing is parsed. A `use` glob is untouched.
- **`f<>` — apply to each**, Julia's `f.(a)`: `relu<> a`, `f64.powf<> a 2.0`, `(|x, y| x > y)<> a b`, `a |> relu<> |> f64.sqrt<>`. It borrows its arguments. Beneath it, `a <- map f`. Rust's empty generic list (`f::<>(x)`, `P<>`) means nothing and is dropped by `hrs-from`: the one place Harsh is not a superset of Rust.
- **Fixed in `hrs_std`:** `&a + &a * &b` did not compile (no `&Matrix + Matrix`; every borrowing form of `+`, `-`, `*` now exists), and an element could not be assigned (no `IndexMut`).
- *Harsh by Example* 17.7 and 17.8; the Guide's matrix section.

## 0.1.20 — 2026-09-21

- **`g~` is the *generator comprehension*,** and `list~`, `set~`, `dict~` the list, set and dict comprehensions — Python's names, and the `g` explained. The Book's chapter 15 is "Generator comprehensions".
- **Where a condition may go,** taught everywhere: every `for` makes one more name available, and a condition sees only the names bound before it — with the annotated example, and the error a misplaced test gives (*cannot find value `b`*, on the Harsh line).
- **What `g~` writes, shown in Harsh:** one `flat_map` per `for`, the level's `if`s folded under `then`, the outer levels flattened — and a runnable proof in the Book that the expansion and the comprehension agree. `flatten$` is part of the generator and never written by hand; `collect$` is not, because a generator is lazy.
- **Fixed: a comment inside a `~` call's lines broke the generated Rust.** It was captured into a fragment and re-emitted in the one-line expansion, commenting out the rest. A macro now never sees a plain comment — Rust's own rule; doc comments still pass.
- Harshlings 0.1.2: a sixth comprehension exercise, the misplaced condition.

## 0.1.19 — 2026-09-21

- **A panic names the Harsh line.** `hrs run` and `hrs test` read the program's stderr and put every `target/hrs/main.rs:4:20` back on its source, `src/main.hrs:4:19` — in the panic message and in backtrace frames, for every file of the project — through the same source maps as the compiler's errors. The column is the token Rust named. A `DimensionMismatch` from `hrs_std` now points at the Harsh line of the product that failed. Cargo keeps its colours on a terminal.

## 0.1.18 — 2026-09-21

**Harsh's selling points, put forth.** *Rust without braces. Rust with pipes, partial application, comprehensions and linear algebra. Rust for functional programming, data science and machine learning.*

- **`hrs_std`, Harsh's standard library, a second crate.** `Matrix<T>` and `Vector<T>` over nalgebra, Julia's model: `*` is the matrix product, scaling or matrix-times-vector, chosen by the compiler from the operand types; `solve` is Julia's `A \ b` — exact for a square matrix, least squares for a tall one, so linear regression is one line; `inv`, `det`, `transpose`, `dot`, `norm`, indexing from 0; `UniformScaling` and `I`; `hcat`/`vcat`. Sizes are values: `DimensionMismatch` and `SingularException` in Julia's words, reported at the caller's line. Publish it before this `harsh-lang`.
- **`m~` and `v~` in the prelude**, with Julia's grammar: a space is `hcat`, `;` or a line break is `vcat`, a comma makes a vector, every entry is a block — so `m~ [(&a) (&b)]` concatenates. `m~ [[1 2], [3 4]]` and `v~ [1 2 3]` are refused, as Julia refuses them. `hrs` names the missing dependency when a project uses them without `hrs_std`; `hrs new` does not add it.
- **The docs lead with the four features.** A "What Harsh adds" showcase — Harsh beside the Rust it replaces, both run — after the braces comparison in the README, the Guide, the Book and START-HERE. The Book gains a part: chapter 14 Pipes and partial application, 15 Comprehensions, 16 Matrices and linear algebra (the old 14–22 are now 17–25). The Guide gains sections on comprehensions and matrices; *Harsh by Example* gains partial application, and pages on comprehensions and matrices grow; Harshlings gains 9 exercises.
- **The formatter aligns a bracket's lines under its first entry** — a matrix's rows, an array's continuation — as Julia lays them out. Parens and `[where …]` keep their rules.
- **Fixed, general:** a separator (`,`, `;`, `=>`) or a closer is never a macro atom; a spanning `expr`, `ty`, `pat` or `path` stops at a top-level `,` or `;`; a repetition without a separator takes one atom per round and stops where what follows begins; the converter isolates an array-literal argument, `f ([1, 2])`, which it wrote as an index.

## 0.1.17 — 2026-09-21

- **A comma inside a type's generic list belongs to the list.** `fn f (r: Result<i32, String>)` and `fn f (a: i32) (m: HashMap<K, V>)` were refused with "a parameter group holds one parameter", because the comma inside `<…>` was read as a separator between parameters; only the bare single-parameter form worked. Worse, `hrs-from` silently split Rust's `fn f(r: Result<i32, String>)` into `(r: Result<i32) (String>)`, so valid Rust became broken Harsh. One shared function served both directions and counted brackets but not generics; it now counts `<…>` too (`>>` closing two), spaced or not. A comma list of parameters, `(a: i32, b: i32)`, is still refused.

## 0.1.16 — 2026-09-21

- **`g~`, a comprehension, in Harsh's prelude.** `g~ x * 2 for x in xs if x > 1`, with as many `for`s as you like; each `if` belongs to the `for` it follows, so a condition sees every name bound up to that point. It is lazy -- a plain iterator -- and `for` takes its iterable by value, as everywhere. `list~`, `set~` and `dict~ k => v for …` collect it into a `Vec`, a `HashSet`, a `HashMap`. The prelude needs no definition and no `use`: a file's own macro of the same name shadows it, and `hrs_std.g~` always reaches the prelude's. A file that calls none of them is untouched, and a Rust `g!` beside `g~` is not refused.
- **A fragment stops at what the matcher names next, including a repetition's first literal.** `$it:expr $(if $c:expr)*` cut `0..4` to `0`; it now stops at the `if`.
- **Fixed, in every macro, not only the prelude's:** a `$` written in a transcriber kept a span inside the macro's definition, so an expansion with `(|| $x)$` followed by more code pasted the source between the definition and the call into the output -- the expansion, then the original line again; and an expanded turbofish was dropped, so `collect<Vec<_>>$` in a transcriber came out `collect<Vec<_>>()`, which rustc reads as two comparisons.
- *Harsh by Example* gains a page on comprehensions: 16 pages, 46 programs.
- Correction: 0.1.15's notes said it built with zero compiler warnings. It had one, an unused variable in `tests/fmt.rs`, now removed.

## 0.1.15 — 2026-09-20

- **A second book: *Harsh by Example*.** Fifteen pages, 43 programs, one idea each, every one transpiled, compiled and run by its harness before it reaches the page — the lookup book, between the Book (which teaches the ideas) and the Guide (which sets each spelling beside its Rust). It follows the topic order of *Rust by Example* (MIT OR Apache-2.0, see `by-example/ATTRIBUTION.md`) but every program and every line of prose is written for Harsh, which is why it can carry a page on what Harsh adds: the pipes and partial application, `macro_rules~` and its token stream, `$`, `do:`, `\` and one group per parameter. `by-example/HARSH-BY-EXAMPLE.md`, a file per page in `by-example/pages/`, and the rendered `.html` and `.ipynb` beside them.
- **The corpus guards were walking a directory that does not exist.** `tests/fmt.rs` looked in `book/src`, which the Book's sources left on 2026-09-11, so for nine days they covered 43 files instead of 278 and the Book's 223 snippets were invisible to them. (`hrs fmt` *was* a no-op over all of them — the guard was blind, not wrong.) The walks now point at the real trees, take in the new book, and assert the corpus is over 250 files so a future move fails loudly. Which snippets a guard may build alone is read from each page's own `@@ … !error` / `!test` / `!doc` markers rather than a hand-kept list, and the test helper now runs the same macro pass the binary runs.

## 0.1.14 — 2026-09-20

This release removes spellings and adds none.

- **A retired spelling is refused on one line too.** `match x:`, `match x do:`, `struct P:`, `enum E:` and `union U:` were refused across lines and still accepted inline, so `match x: a => 1` and `match x\ a => 1` emitted the same Rust. Each is now refused by name, inside parens as well, with the `\` spelling in the message. A test holds every retired spelling in both forms, so a migration cannot leave half of itself behind again.
- **`\` follows the construct it specifies.** It opens a specification block -- Rust's `{ a, b }` groupings: a declaration's fields, a literal's or a pattern's, a match's arms -- and alone it means nothing. `let x = \ 4 * 2` used to emit `{ 4 * 2, }`; it is an error. It has nothing to do with calls, and it is not `do:`, which opens a block of statements.
- **A Harsh macro's stream is bounded by indentation.** The stream of a `~` call is the rest of its line *and every following line indented deeper than the line the call sits on*; an unmatched closing bracket -- the closer of a group the call was written inside -- ends it sooner. So `lst~ 1 2` with `3 4` deeper beneath it passes four tokens, `(lst~ 1 2` … `3 4) <- len$` chains on the result, and `(m~ do:` … `)` works. Until now the stream stopped at the physical newline: a continued call emitted `[1, 2]3(4)` with no Harsh error. A `\` after the mark is a token of the stream, as any other.
- **For Rust's `!` macros, `\` is a brace call.** `m!{ a => x, b => y }` is `m!\ a => x, b => y`, or `m!\` with one entry per line. The inline form was broken -- it emitted `hm! { 1 } => "a", …` -- and `hrs-from` now writes the `\` form for a flat comma list, keeping Rust's braces for a `;` body, a nested brace, a trailing comma (which `m!\` cannot write and some matchers require), or an entry with two atoms side by side -- a DSL's own tokens, which juxtaposition would misread.
- Fixed: an inline `match n\` with an or-pattern, `1 | 2 => …`, closed its braces at the `|` -- the `=>` after a match's own `\` had it read as a record pattern's, so the inline form only ever worked by accident. `union U\ a: u32, b: f32` inline emitted `union(U) {`.

## 0.1.13 — 2026-09-20

- **A repetition takes its marker, on both sides of a macro.** `$( … )` is followed by `*`, `+` or `?` — Rust's grammar exactly, as decided on the 19th. The matcher had been accepting a bare `$( … )` as "exactly once", and the transcriber accepted it too, running every round without being told to. Both are now refused, with the caret on the `$(`; a multi-line repetition whose `)` lacks its marker says so, on the `)`, instead of claiming it "is never closed". A group that must appear once is written without the `$( … )`.
- **A transcriber no longer eats the author's tokens.** Whatever followed a repetition's `)` was taken for a separator, and the token after it skipped unchecked: `[$( $a ) x y]` silently produced `[1(x, 2, x, 3)]`. A token is a separator only when a marker follows it. `?` takes no separator in a transcriber, as in a matcher.
- An error on a call with no arguments (`one~` alone) is reported at the call, not at line 1.
- Housekeeping: four functions left behind by the 0.1.9 delimiter rule are gone, with two comments that still described it; a test that was registered twice runs once.

## 0.1.12 — 2026-09-19

- **A Jupyter kernel.** `kernel/` holds `harsh_kernel`, a wrapper kernel: each cell is transpiled with `hrs` and evaluated by the evcxr Rust kernel, with output, values and errors relayed back. A function defined in one cell is callable in the next; a Harsh macro unfolds inside a cell; evcxr's `:dep`-style commands pass through; a Harsh error points at the cell's own line, gutter and caret included. `pip install ./kernel && python3 -m harsh_kernel.install`; needs `hrs` and `evcxr_jupyter`. `docs/JUPYTER.md` is the guide.
- **The source map through expansion.** An error inside a Harsh macro's expansion now points at the transcriber's line *and* carries a note naming the call that produced it — *in the expansion of `bad~` called at file:6:13* — with the call's line shown. The map records each expansion and the context of every expanded token; `hrs-remap` reads both. A diagnostic at inserted punctuation is blamed on the token before it, not the one after.
- **Two converter fixes** (`hrs-from`): a brace group in an open `if let`/`while let` pattern is the pattern's, not a block's (`if let E::G { body, .. } = &m {` converts to `if let E.G\ body, .. = &m:`); a struct literal that is the sole one-line element of a bracket group is not isolated in parens (`vec![Arm { m: 1 }]` round-trips as itself), while two in one group still are.

## 0.1.11 — 2026-09-19

- **A Harsh macro call is a stream, not an application.** Everything after `name~` — to the end of the line, to the close of the group the call sits in, or to the end of a `do:` block it opens — reaches the matcher exactly as written. A comma is the DSL's token; a `(…)` is a group, one token, so a tuple is written once, `pair~ (1, 2)`; a call inside a larger expression is isolated as any application is, `((twice~ 4), 0)`. This replaces 0.1.10's `name~\` rule, which decided who owned a comma from a mark — an inference the language does not make.
- **A fragment's extent follows the matcher's shape.** To the next literal the matcher names or the repetition's separator; one atom when another fragment follows directly (`$a:expr $b:expr` is juxtaposition, as in any Harsh application); the rest of the stream when nothing follows. The Rust idiom `$( … ),* $(,)?` works for DSLs that own commas.
- **The matcher's outer parens are its own delimiter; a DSL's brackets go inside them**: `([ $e:expr ; $n:expr ])` for `filled~ [0u8; 2]`. The group-per-argument matchers (`(($a:expr) ($b:expr))`) are gone from the corpus, the Book, the guide and the exercises: a Harsh macro juxtaposes, `($a:expr $b:expr)`.
- Arms are tried in order and the first that fits wins; the more specific arm comes first.

(0.1.10 was built and never published; 0.1.11 supersedes it.)

## 0.1.9 — 2026-09-18

(0.1.8 was published from an earlier state of the same day's work, before the audit fixes below and before `match v\`; 0.1.9 supersedes it.)

- **A `match` is opened by `\`**, in both forms: `match v\` with the arms beneath, or `match v\ p => e, q => f` inline. A match's arms are a specification block in use — comma-separated in the Rust, like a literal's fields — so they take the specification marker. `match v:` and `match v do:` are refused, naming the spelling. `hrs fmt` lays a multi-line match out as it does a literal — `let r =` on its line, `match n\` beneath, the arms one unit past it. The corpus, the Book, the guide, the exercises and `hrs-from` all moved.

**From an audit of the macro work**: hygiene now covers every binding a transcriber can write — closure parameters, `for` bindings, `match` arm patterns and `fn` parameters, not `let` alone (each of those had silently captured the caller's expression); `#![recursion_limit = "…"]` is read; **`hrs expand <file>`** prints a file with its Harsh macros unfolded, as Harsh; `neg~ -5` matches a `literal`; `struct P do` and `impl P do` are refused by name, since those constructs take no opener; `match x do:` is a match; a `\` literal as a `match` scrutinee is parenthesised; and every example in the corpus is now compiled by a test, not only transpiled.

**Six fixes found by running the guide's own macro corpus** (`examples/guide/17_macro_bodies.hrs`), all in the new expander: a matcher's delimiters may be `[` or `{` as well as `(`, and a call written `m~ [ … ]` hands its contents to the matcher; a fragment stops at the next literal token the matcher names, so `$k:expr => $v:expr` works; a repetition may span lines (`$(` on its own line, the body beneath, `)*` closing it), and its lines land where the `$(` stood; a call's arguments stop at a `,` or `;` that belongs to whatever encloses the call, so `(twice~ 4, 0)` passes `4`; a `$` in an expansion is still Harsh's call marker, so `Vec.new$` becomes a call; and an expansion's out-of-order spans no longer panic the emitter.

A `~` call may hand its arguments as a **block** — `m~ do:` with the body beneath, or `m~\` with its entries — as a `!` call does. The opener belongs to the call; what the matcher sees is the block's own tokens, written as written (a `\` block's entries arrive one per line, with no comma, since nothing has been emitted yet).

- **Harsh expands its own macros.** A `macro_rules~` definition unfolds *into Harsh* before anything is transpiled: the definition leaves no trace, a call becomes what its transcriber says, and the generated Rust holds no macro of ours at all. A repetition on its own line yields one line per round; inside a line it yields its rounds in place with the separator you wrote. Nothing is inserted on your behalf.
- **Hygiene, as in Rust.** Every token carries a syntax context: a name the macro introduces and a name the caller passed in are different names even when spelled alike. Where both would stand in one scope, the macro's own local is respelled — and only then, so an expansion reads as its author wrote it. A name the caller is to use must be passed in as `$name:ident`.
- **Fifteen fragments in Harsh's terms**: `ident`, `lifetime`, `literal`, `tt`, `expr`, `stmt`, `line`, `block`, `pat`, `ty`, `path`, `item`, `entry`, `meta`, `vis`. Rust's `pat_param` and `expr_2021` are refused by name — they are edition history. A fragment ends at its logical line, a dedent, its group's close, or the next literal token the matcher names, so `(($a:expr) ($b:expr))` is ordinary here.
- **Rust's recursion ceiling replicated** (128 passes, raised with `#![recursion_limit = "…"]`), and a call whose mark does not match its definition is refused by name: *`adding` is a Harsh macro, so it is called `adding~`*.
- **`macro_rules~ name` takes no mark.** Its arms are a specification block separated by layout alone — no `;`, no `,` — as `struct Point` and `impl Foo` are written. The old `macro_rules~ name:` is refused, naming the spelling.
- **The Harsh macro system, stage 1**: a definition is read into a tree (`src/mac.rs`) with the fifteen Harsh fragments — `ident`, `lifetime`, `literal`, `tt`, `expr`, `stmt`, `line`, `block`, `pat`, `ty`, `path`, `item`, `entry`, `meta`, `vis` — repetitions with a separator and `*`/`+`/`?`, nesting depth, and errors on your own lines. Rust's `pat_param` and `expr_2021` are refused by name: they are edition history. Nothing expands yet; expansion is the next stage.

## 0.1.7 — 2026-09-17

- **The block kit is complete, and item bodies lose their `:`.** A block's opener says how its entries end: **no mark** after a header that ends itself — now `impl`, `trait`, `mod` and `extern` as well as `struct`, `enum`, `union` — **`\`** for a comma-separated list, **`:`** or **`do:`** for statements, and the new **`#:`** for a grouping whose grammar is its author's, where Harsh writes the braces and the lines and nothing between them. `impl Foo:` is refused, naming the spelling; so is `#:` on a construct that already has one (`struct Point #:`).
- A macro call written `m!\` no longer ends its last entry with a comma: a matcher of fixed arity refuses a trailing separator. Struct literals and declarations keep theirs.
- `hrs-from` writes the bare item header, and the whole corpus, the Book, the guide and the exercises are written that way.

## 0.1.6 — 2026-09-17

- **`macro_rules!` is a zone of Rust**, copied verbatim in both directions: the transpiler reads none of it and rewrites none of it, and `hrs-from` copies a Rust one into a Harsh file unchanged, so the round trip through a definition is byte-exact. `macro_rules! name` opens the zone and its `{` … `}` delimit it — no Harsh opener, since the body is laid out as Rust. A `}` inside a string, a char, a raw string or a comment is text, not a brace. Only the definition is foreign: `my_vec! 1 2 3` is still a Harsh call and becomes `my_vec!(1, 2, 3)`.
- `hrs-from` no longer converts a Rust `macro_rules!` into Harsh — it copies it.

## 0.1.5 — 2026-09-17

- **Harsh's own declarative macros are marked `~`**: `macro_rules~ twice:` at the definition, `twice~ 4` at the call. Rust's `macro_rules!` and its calls (`println!`, `vec!`, `view!`) are unchanged. Today the two behave identically — the mark is normalised once, at the door — so this release is the spelling and nothing else; it is the fork where Harsh's own expansion will happen. A `~` with a space before it is not a mark and is left alone.
- The Book's macro chapter, the guide, `docs/MACROS.md` and the examples are written in the new spelling; every two-column block is still checked against its Rust.

## 0.1.4 — 2026-09-11

- A spaced `[` after a macro's argument is refused too (`assert_eq! v [1, 2]` used to emit `assert_eq!(v) [1, 2]`); an array argument to a macro is isolated, `assert_eq! v ([1, 2])`. `vec! [1, 2]`, with nothing before the `[`, is the bracket body as before. Found by a Harshlings exercise.

## 0.1.2 — 2026-09-11

- A tight index is part of its atom: `f arr[1]` is `f(arr[1])` — it used to emit `f(arr)[1]`, silently indexing the result. A spaced `[` inside an application (`f arr [1]`) is now refused, naming `arr[..]` and `(arr [..])`. Brackets never apply; `arr [1]` alone is the index it always was.

## 0.1.1 — 2026-09-11

Every fix is published from here on.

- Fixed: a written `;` ending an inline block (`if c: f$;`, `let y = do: f$;`, an arm's `do: f$;`) was dropped; it now reaches the Rust as `{ f(); }`, as in the indented form. Found by the Harshlings runner. Settled: brackets always index, spaced or not; `f ([1, 2])` passes an array.
- `GOVERNANCE.md`: a rule holds in every form it applies to.

- **The crate is `harsh-lang`** (library crate `harsh_lang`); the binaries stay `hrs`, `hrs-from`, `hrs-remap`, `hrs-lsp`. `hrust 0.1.0`, published a few hours earlier, was deleted. The VSCode extension is `harsh-lang.harsh-lang` on the Marketplace, with the drawn mark as its icon (0.1.1); the docs, the guide and the Book link the crate and the extension.

- Guide code blocks: a second column is always Rust and is compared with the transpiler's output; notes are a third column or a `//` comment. All 24 two-column blocks now match; two Rust columns in the partial-application section were wrong and are corrected from the transpiler's own output.

- **Formatter: a block literal's shape** (the user's rule). `let u = User\` with the fields beneath is re-broken to `let u =` / `User\` / the fields, one unit past the literal; nine corpus files moved. Editor: Enter after a line ending in `\` lands one unit past the literal's name.

- The Book states that an application binds tighter than `<-` (chapter 2, with the lines that show it) and that a pipe's sides are atoms, so a result is isolated before `|>` (chapter 13) — the rule was pinned by a test and stated in the guide, never told to the reader.

- Fixed: the rendered Book stopped at §14.2 — a ```` ```text ```` span in a sentence opened a fence in the renderer and swallowed the page to the next block. Fences now open and close only at a line start, and the Book's prose guard refuses three backticks inside a sentence.

- **The Book is complete as planned**: closure types in chapter 13, the where-clause rule in 10, `extern "C":` and "Parentheses, once and for all" in 20, and chapter 22 "Harsh at a glance" — one verified program and every rule of Harsh's own with the section that teaches it. `book/PLAN.md` is marked done.
- **The three struct forms are three equals** in the Book and the guide — record, tuple, unit-like — each shown declared both ways, block and inline; an enum is explained as a union of variants each of which is one of the three (an algebraic data type), with `Move\ x: i32, y: i32` shown beside the block form.
- **The Book states the layout itself**: §2.6 "How a page of Harsh is laid out" (the three block spellings, `do:`, statement ends, continuation lines, application, `$`, parameter groups, `<-` versus `.`) and chapter 3's inline forms and `else` placement, with three new verified snippets.
- **The Book has a pipes chapter** (§13.3): `|>` and `<|` beside `<-`, partial application from either side and the middle hole, a partial passed to `map`, pipelines, and when a pipe beats a closure — six verified snippets, one of them Harsh's own arity refusal. The chain rule and paren blocks are stated under "Chains". The harness's `!error` now also shows an error Harsh itself raises.

- **The guide's code is verified** (`docs/check-guide.py`, run by `docs/build.py` before anything renders): every untagged block transpiles, statement blocks inside a `fn main$:` wrapper and lone methods inside an `impl`; a Harsh-beside-Rust block, in either notation (aligned columns or `harsh → rust`), has its Rust column compared token for token with what the transpiler writes. First run: 23 of the guide's 61 single-column blocks and the whole opening tutorial were on retired spellings — `enum Format:`, `struct Report:`, `Report { counts, format }`, `format!(…)`, `Some(n)`, `g(v)`, `#[cfg(test)]`, `println!(…)`, a comma parameter list — and one Rust column claimed `#[derive Debug Clone]` came out unchanged. All corrected; 99 blocks transpile, 18 are matched against their Rust; 12 catalogues of forms are tagged ```` ```fragment ```` and not checked.
- **The Book's prose is guarded** (`book/build.py`): retired spellings inside backticks — `struct X:`, brace literals, `#[cfg(`, `::`, `m!(`, `.f()`, `Write (String)` — fail the build. First run: fifteen more in chapters 10, 11, 17, 20 and 21.
- Editor: Enter after a line ending in a macro's `!` lands on the first argument's column and then on each argument's sibling; name-blind, no arity. VSCode: `indentationRules` and `onEnterRules` removed and `editor.autoIndent: keep` set, so Move Line Up/Down no longer re-indents by Rust's braces. Zed's dev-install route, with the grammar as a local repository, is in `docs/INSTALL.md`.
- The Book is *The Harsh Programming Language*; the `\` field-list rule is settled in the roadmap, the guide and chapter 5.

- **Doc examples are Harsh.** A fenced block in a `///` or `//!` comment is code — rustdoc lifts it out, compiles it and runs it — so it is written in Harsh and `hrs` writes the Rust rustdoc expects (`src/docex.rs`, a pass over the emitted text, with the source map shifted by each rewrite); `hrs-from` converts in the other direction, so a crate brought in from Rust has its examples in Harsh like the rest of it. Which fences: exactly the ones rustdoc compiles (empty, `rust`, or the doctest attributes); a fence naming another language is prose and is untouched. Hidden `#` lines keep their marker, an example's last line keeps its `;` (it is transpiled inside a wrapper, as rustdoc wraps it too), and a mistake in an example is a transpile error on the `.hrs` line that holds it. `hrs export` translates them as well, since the exported crate is built by cargo alone. Eight tests (`tests/docex.rs`); the guide's "Doc examples"; the Book's chapter 14 example is Harsh and its doc test is run by the harness (`!doc`).
- Fixed, found by `round_trip_own_source` when the above was written: an inline struct literal inside a tuple, `(Point { x: 1 }, msg)`, converted to `(Point\ x = 1, msg)`, whose field list ran to the tuple's `)` and took `msg` as a shorthand field — a valid literal, and so a silent wrong program rather than an error. The converter now isolates a literal that is one element of a tuple, `((Point\ x = 1), msg)`, as it already did inside brackets; a call's argument list is excluded, since each argument is isolated there already.
- The book is one edition and contains no Rust: `book/HARSH-BOOK.md` → `book/harsh-book.{html,ipynb}`, the two-column edition retired, all 37 `@rust` paragraphs folded in (the concept kept and restated, the Rust spelling gone), `@harsh` now a rendered callout, and `@rust` and `@guide` refused by the harness. The generated Rust of every snippet is still built and run, and stripped from the page with the `.rs` path label that introduced it — 28 of those labels were standing over nothing in the old Harsh-only edition.
- Fixed: every rendered page carried the language guide's title and tagline, the book and the roadmap included — `render()` took both and used neither.
- The struct and enum redesign reached the book's prose: chapters 5, 6, 9, 11 and 19 still taught `struct User:`, `Move:`, `Write (String)`, `enum Result<T, E>:`, brace literals, brace patterns and `#[cfg(test)]`, all of which their own snippets had stopped using.

- Formatter: **arguments beneath the callee** (the user's rule) — a call whose line passes the width, or has a block argument beside another, lists every argument one unit past the callee, recursively; a block argument is `(do:` .. `)` and the converter writes it; chains recurse into arguments and a paren block's tail continues its header's chain; one link is never broken. Editor: Enter after a declared function applied to fewer arguments than its parameters lands on the next argument's column.
- Converter: argument braces (`json!({ .. })`) are data and kept; attributed bare blocks are `do:`; holes and transcribers are Harsh inside; the Leptos site regenerates on the new spelling, 20 of 22 files token-identical.
- Formatter: **the receiver decides a vertical chain's shape** — the first link stays on the receiver's line and the links align under its arrow when that arrow sits within 12 columns of the line's start; a longer receiver stands alone with every link one unit in. Six corpus files moved.
- Editor: Enter after an argument listed beneath its callee lands on the argument's column (a sibling), one more unit a Tab away — no staircase, and no fall back to the statement. The line after a line-final `=` keeps the long-receiver hang (one unit in, for the first link).
- **Braces hold a block on one line.** A `{ .. }` over several lines is an error naming `:` / `do:` (a hole in markup or a tree, and a brace group as an argument, are the exemptions). A block in operand position is `(do:` .. `)`, a block as a value `= do:`, an empty function body `()`.
- **Structs and enums, redesigned with the user.** A declaration's body follows its name with no mark — `struct Point` / `enum Truth` with the body beneath, record variants by a bare name — and inline the field list is marked `\`: `struct Point\ x: f64, y: f64`, `enum Shape\ Empty, Circle f64, (Named\ name: String)`. Tuple payloads are applications: `struct Meters f64`, `Tuple i32 String`. A literal is always `\`: `Point\` with `field = value` lines beneath, or `Point\ x = 5.0, y = 7.0` inline — anywhere an expression stands, headers included (`match (Point\ x = 1):`). Patterns: `Point\ x, ..`. `Point { x: 1 }`, `struct Point:`, `Point:` are errors naming the new forms. The corpus (252 files, Book 208/208) is on the new spelling; `hrs-from` writes it, including `\` patterns, `= do:` values, `(do:` operands, `=> do:` transcribers with `$(..)*` repetitions, and `()` empty bodies; `#[cfg …]` is written tight.
- **First program built by the real framework:** the converted Leptos + Axum site compiles end to end with `hrs cargo leptos build` (ssr binary, hydrate wasm, wasm-bindgen).
- From the crate's first real `cargo leptos build`: a paren block's tail may open a markup (or any) block — `) <- into_iter$ <- map (|line| view! do:` — with the next tail attaching to the innermost open block; and **`include_str!` / `include_bytes!` / `include!` paths are re-based** for the generated tree (`"../content/x"` in `src/` becomes `"../../content/x"` in `target/hrs/`; `hrs export` and a single file beside its source keep them).
- `hrs cargo <sub>` runs external cargo subcommands (`cargo leptos build`) plainly — they do not take `--message-format`. `--version` carries the bundle's date as semver build metadata (`0.1.0+2026.09.07`) so a reinstall is visible. Fixed: a `pub const`'s value was neither converted nor juxtaposed (`pub` hid the `const`; `const` is also a modifier).
- **A real Leptos + Axum crate (20 files, 2,700 lines) round-trips.** What it taught, all fixed and pinned (`leptos_shapes`, `converter_leptos_shapes`): documented parameters (`///` and `#[prop …]` on props — doc lines above the group, attributes inside, the Rust `#[component]` expects); rule 1 in brace form and a markup body that opens with a hole or a comment; an attribute value's Harsh in braces, `on:click={move |_| …}` (the preferred spelling, one with a Dioxus tree's holes, written by `hrs-from`), or isolated in parens, accepted and not emitted; hyphenated attributes; a markup block's tail inside a group (`).collect_view()` was dropped); **nested paren blocks** (the baseline was counted from the line, not the absolute depth); a `):` tail that opens the header's own block; a chain off a block expression (`if … {} else {}.into_iter()`) isolated in a group with `(` on its own line, and `else` seen inside an open group; closure bodies inside argument groups converted to layout blocks (`<- map (` / `|item|:` / body / `)`), prototypes written tight (`|item|`), their commas not read as tuples; bare `{ … }` → `do:`; `json!({…})` and block arguments isolated; a comment between links continues the chain; generic type arguments isolated in `Fn` applications; `hrs-from --raw`; `hrs cargo <sub>` (`hrs cargo leptos build`). **`fmt` enforces rule 0 at run time** and leaves a file unchanged rather than change its meaning.
- `book/PLAN.md`: the plan for amplifying the Harsh Book into the one document for readers who do not know Rust, decided with the user; the Language Guide stays the comparison for Rust programmers.
- Fixed: `hrs build`/`run`/`check` kept the generated Rust from an older transpiler when the source had not changed ("up to date" by mtime). A generated file is now stale whenever the `hrs` executable is newer than it, so `cargo install --path .` retranspiles everything on the next build.
- Fixed: `hrs build`/`run`/`check` kept the Rust an older transpiler had generated when the `.hrs` source was unchanged (`1 file(s) up to date`), so a newly installed `hrs` seemed to reject nothing and change nothing. A generated file is now also stale when the `hrs` executable is newer than it.
- Fixed: a bare parameter followed by `[where ..]` had its `[` counted as a group — the arm that ended the parameter there was unreachable behind `Tk::Open(_)` (a build warning said so). Optional grouping now peels `((x))` fully. The build is warning-free.
- `docs/INSTALL.md`: the commands to install and update, on one page. `--version` on all four binaries.
- **One syntax for applying, everywhere.** A name and the group isolating its argument are separated by a space; `f(x)` tight is an error naming `f (x)` / `f x` — in patterns (`Some n`), declarations (`fn f (a: T)`, `Circle (f64)`), macros (`m! (a)`), attributes and types. Attributes are applications inside their brackets: `#[derive Debug Clone]`, `#[cfg (feature = "x")]`, `#[cfg_attr (feature = "ssr") (derive Serialize)]`. The parenthesised types are applications: `fn (x: i32) (y: f64) -> &str`, `Fn i32 -> i32`, `Fn (i32) (i32)`, `FnOnce$`. `extern "C":` is a layout block. A macro's brace body is Harsh in one line and juxtaposes (`quote! { fn #name$ -> u32 { #body } }`); the earlier rule-3 exemption is withdrawn. Optional grouping is not emitted: `let s = (1 + 2)` → `let s = 1 + 2;`, a whole-statement `(f x)` → `f(x)`; tuples, the unit and precedence parens are kept. `hrs-from` writes all of these forms, converts `let`/`if let`/`while let` patterns as applications, and spaces `if !x` correctly. Corpus: 58 attribute lines, 15 `Fn`-type lines, four old example files and the `extern` Book file rewritten; Book 208/208.
- **Macros are Harsh on both sides** (`docs/MACROS.md`, six rules). A macro's `{ … }` body is Rust's syntax with Harsh's tokens and nothing inside juxtaposes (`m! { a b c }` no longer becomes `a(b, c)`). A macro invoked with a body is `name! do:`: a Harsh block (`tokio.select! do:` with `=>` arms), markup when its first line begins with `<` (Leptos: lines copied, `{ … }` holes Harsh, a hole may span lines), a brace tree when it begins with `name:` (Dioxus: `div:` opens an element, `class = "app"` an attribute, holes Harsh, `for`/`if` the framework's own). `macro_rules! name:` holds arms `( matcher ) => do:`; the matcher is a parameter list — `( ($a:expr) ($b:expr) )`, a repetition of groups `$( ($x:expr) )*` — and the transcriber a block with `$x` an atom, `$( … )*` alone on its line repeating statements (multi-line as `$(` / body / `)*`), and `$( ($x) )*` in an expression a list of arguments while `$($arg)*` forwards `tt`s. Bracketed and braced matchers are Rust's. A bare `=> $e * 2`, a written `;` after an arm, and a written repetition separator are errors. `hrs-from` converts `macro_rules!` bodies, `view! { … }` and `rsx! { … }` to the new forms; the round trip is pinned.
- `docs/GOVERNANCE.md`: "no second spelling" was a prescription, not a rule, and now reads so; and a new principle, leave to rustc what rustc rejects. `docs/FMT.md`: all four decisions recorded; decision 4 (parens transparent to layout) is the formatter session's first step.
- **`hrs fmt`** — the formatter (`docs/FMT.md`). Two passes: re-breaking — a chain of three or more links goes vertical, one or two links sit on one line within 72 columns (a `=` value is measured on its own line first), `=` ends its line before a multi-line group or a chain, a paren block's closure prototype gets its own line — then indentation: every physical line gets its column from the layout's rules — block bodies one unit past the line holding their construct, bracket contents one unit past the anchor and closers under it, chain links under the first arrow or one unit past a long receiver, continuations keeping their relative nesting, braces and markup untouched. It changes no token and no line break; over the 252-file corpus it is a no-op, idempotent, and the Rust out is token-identical (`tests/fmt.rs`). `hrs fmt --check` for CI; `hrs-from` writes formatted output; `hrs-lsp` answers `textDocument/formatting`, so format-on-save works in VSCode and Zed.
- **Parens are transparent to layout.** A block may be written inside a group and ends where the group ends: `map (|p|: if *p > 1: *p else: 0) <- collect$`, `fold 0 (|acc, x|: acc + x)`, `let f = |x|: x + 1`; a chain may open several. A line inside an open `(` must indent past the line that opened it or be the `)` — a line at the opener's column is an error. Fixed: a paren block after an earlier argument emitted `fold(0) (|acc, x| {…})`.
- Raw identifiers (`r#type`) are one token; they used to emit `r)#type`.
- The tree-sitter grammar parses a line-continued string (`"\` at the end of a line); the corpus loop now covers 252 files with zero error nodes.
- `examples_transpile` now walks `examples/guide/` too; four scratch files it had never seen (bare-closure chains, rejected on purpose) are removed and the retired inline statement form `do: let u = 3; u * u` is corrected in `11_let.hrs` and the guide.
- **The Harsh Book** — 21 chapters, 205 verified snippets, in a Harsh-only and a Harsh + Rust edition.
- **`$` applies a name to nothing.** `f$` is `f()`, `fn main$:` declares no parameters, `s <- len$ <- collect$` chains, `m!$` for a macro. `()` is now only ever the unit value: `f ()` is `f(())`, `Ok ()` is `Ok(())`; the `(())` spelling is gone. A `()` group in a `fn` declaration, a spaced `f $`, and `$` beside parameter groups are errors that name the fix. `hrs-from` writes the new spelling. The tree-sitter grammar gains an `apply_nothing` node; the VSCode grammar highlights the suffix.
- A bare parameter followed by a group (`fn add &self (k: i32)`) is now an error naming `(&self) (k: i32)`; it used to emit invalid Rust silently.
- A trailing comment on a block-opening line (`Move:  // fields`) no longer swallows the `{`.
- `hrs run` / `hrs test` no longer swallow a program output line that happens to be valid JSON (`5050`, `true`, `"text"`): only an object with a `reason` is taken as cargo's.
- `hrs new` writes `fn main$:`.
- A chain continues after an isolated closure whose body has a nested block; the `)` tail now rejoins the closure's header.
- A one-token bare parameter (`fn f self`) is parenthesised correctly; it used to emit `fn f self)`.
- `move ||:` and `async:` / `async move:` are recognised as block-owning trailing arguments (`thread.spawn move ||:`, `tokio.spawn async:`).
- A bare parameter now ends at a `[where …]` clause as it does at `->`.

## 0.1.0

First public release.

- `hrs`: the transpiler and build driver — `new`, `build`, `run`, `test`, `check`, `lint`, `watch`, and single-file mode with a source map.
- `hrs-from`: Rust to Harsh; the transpiler's own source round-trips through it byte-exact.
- `hrs-remap`: cargo diagnostics mapped back to `.hrs` lines and columns, including zero-width suggestion spans.
- `hrs-lsp`: a language server with layout-aware on-type formatting and a `harsh/columns` request; no names, no types, no arity.
- Editors: VSCode extension (highlighting, folding, Enter and Tab through `hrs-lsp`), tree-sitter grammar, Zed extension.
- Documentation: the language guide with every snippet transpiled, compiled and run; the specification; the start of the Harsh Book.
