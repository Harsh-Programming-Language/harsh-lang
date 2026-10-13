# Harsh: a retrospective

## How the work was run

The README closes with two paragraphs on how Harsh was made. They open this
record because everything after them follows from the method they describe.

> Harsh was designed by Bahiminin Benoit Dah and built with [Claude](https://claude.ai) (Anthropic): the design decisions are the author's, most of the code is Claude's, and every claim about the language was checked against a real compiler before it was believed — a habit that found several bugs on the way.

> The two disagreed often, which was the point. The `\` mark, the three struct forms as equals, the pipes as the one deferral mechanism, the shape of a block literal, the name — each was the author's call, several against Claude's first opinion, and each held. The rule that made that work: every proposal had to be argued before it was built, and every claim checked against a compiler before it was believed — so a disagreement ended with an example either could point at, not with whoever spoke last.

### What this document is

A technical record, written once, at the end, to draw lessons from. It covers
the work from the first dated session (2026-09-07) to the release in hand on
2026-10-13 (0.8.4): what was decided, in what order, why, and what was later
undone; every feature of the language and every tool, with the files it lives
in; the oracles that checked the work and what each one caught; the bugs the
books and the application projects found; and the principles, those that held
and those that were amended.

Its sources are the session notes delivered with each release (the
development notes, kept outside the public tree), `CHANGELOG.md`,
`docs/TUTORIAL.md`, `docs/GOVERNANCE.md`, `docs/DECISIONS.md`, `docs/FMT.md`,
`docs/LSP.md`, `docs/MACROS.md`, `docs/ROADMAP.md`, the module headers of
`src/`, and the names of the tests. The test names are the change log the
compiler enforces, and they are cited as evidence: "pinned by
`a_different_hrs_retranspiles_once`" means a test of that name exists and
fails if the behaviour returns.

Where the sources disagree, the later one is taken. Where something is not
recorded, it is left out.

### Disagreements, as the notes record them

The README names the author's calls. The session notes record the shape of
several disagreements and how each ended. A sample, in order:

- **Why remove braces at all.** Claude asked the author to justify it. After
  a long enough exchange the answer was put in one line, *braces are for the
  compiler; indentation is for humans*, which the README carries under the
  tagline. The notes add that Claude does not remember the exact words and
  would sign them.
- **The parameter forms (before 0.1.0).** The author proposed, then withdrew,
  `fn add |x, y|`, `fn add /x, y/`, `fn add ||x, y||` and `fn add \x, y\`:
  each was a second spelling. The comma list was removed instead.
- **Pseudo blocks (2026-09-12).** From a `quick_error!` body, Claude proposed
  three designs and argued for the wrong one: inside a macro's braces, insert
  nothing and let the author write the DSL's punctuation. The author refused
  it because macro bodies must read like Harsh, and reframed the body as a
  grouping rather than a block. The design that followed (`#…:`) was built and
  later retired (below); the reframing held.
- **The `name~\` rule (2026-09-19).** Claude's rule for who owns a comma in a
  `~` call lasted an hour. The author called it guessing intent from a mark.
  Four rounds of discussion produced the stream rule that replaced it.
- **Comprehensions (2026-09-21).** Claude argued twice from reasoning and was
  wrong twice: that an early `return None` short-circuits better than an
  `&&` fold (measured: byte-identical machine code), then that early return
  composes across levels better (it does not nest at all). Both were settled
  by writing the two-level case in each shape and running it.
- **Matrices (2026-09-21).** Claude first built `m~` as a list of row shapes,
  Matlab's `[1, 2; 3, 4]` included. The author pointed to Julia's actual rule
  (a space is `hcat`, `;` or a newline is `vcat`, a comma makes a vector,
  brackets are blocks), which is what shipped.
- **A second manifest table (2026-09-29).** The author proposed a
  `[dependencies_hrs]` table in `Cargo.toml`. Claude declined it with reasons
  (Cargo warns on unknown tables; rust-analyzer and cargo tools read the real
  manifest). Four days later the author's own design, a separate `Hrs.toml`
  beside `Cargo.toml`, settled the question.
- **The handwriting rules (2026-09-29).** Claude first made `hrs fmt` the
  judge of the author's rules for long lines and added a gate forcing it on
  the books. The author corrected it: the rules ask for judgment, and a long
  `v~` list of short numbers must not explode. The gate was removed.
- **The turbofish (2026-10-03).** Claude called `Layout.array<u32> 4` a
  missing turbofish. The author corrected it: Rust's `::` is Harsh's `.`
  everywhere, so the turbofish is `.<T>`, and the old form was never a gap but
  the wrong syntax.
- **The audit of exceptions (2026-10-04).** The author named the failure
  behind the turbofish: when Claude's hand-written Harsh failed, Claude patched
  the transpiler instead of the syntax. `DECISIONS.md` opens with the working
  rule that came out of it: when hand-written Harsh fails, suspect the writing
  before the transpiler.

## The chronology

The sessions ran from 2026-09-07 to 2026-10-13. The release table at the end
of this part is an index; the sections before it follow the work in order.

### Before the first release (to 2026-09-10)

The earliest sessions are undated in the notes. Their order is recoverable
from the test counts each one recorded (29, 65, 69, 72, 80, 81, 85, 89, 103).

**The architecture.** The first transpiler was a conventional front end:
lexer, recursive-descent parser, semantic tree, an emitter rebuilding Rust
from the tree. It handled a subset of Rust and would have had to grow without
end. It was replaced with a **layout transformation**: the transpiler
understands which lines are nested inside which, and copies every token it
does not explicitly rewrite (`docs/TUTORIAL.md`, part one). That is why
generics, lifetimes, `impl Trait`, `async`, macros and future Rust syntax pass
through without the transpiler knowing them.

**The five changes (29 tests).** Landed one at a time, each with the round
trip over the transpiler's own source byte-exact: a tuple index is part of its
atom (`show t.0 t.1`); a statement holding an indented closure takes `;`;
parens transparent to layout, a block's tail rejoining after `)`; a chain
after a bare closure block refused; a `;` anywhere but a block's last
statement refused. With them: the macro DSL exemption removed (a macro is
applied exactly like a function, both ways), and `$` redesigned as
partial application with a project-wide arity table. The round trip caught
`Option<Vec<T>> = x` converted as `>>= x`, a shift-assignment.

**The pipes replace `$`.** In the same period the author settled the pipes as
the final deferral design: `|>` fills a function's parameters from the left,
`<|` from the right; with fewer arguments than the arity, the result is one
flat closure over what is missing (`1 |> sub <| 3` is
`move |__hrs1| sub(1, __hrs1, 3)`, the middle hole that `$` never had). `$` in
its old postfix position became an error pointing at the pipes. The arity
table was declared the pipes' alone. This is one of the README's named calls:
"the pipes as the one deferral mechanism".

**The language server and the editors (65 tests).** `src/columns.rs` and
`src/bin/hrs-lsp.rs` were designed in `docs/LSP.md` before the code: a server
that answers one question, where may the next line start, from token kinds and
the two layout rules, and never compares identifiers. The VS Code extension
gained a client; Zed a WASM shim. Around the same time: `hrs export` (a project
as a plain Rust crate); the call-paren style of generated Rust (tight after a
callee, pinned by `call_parens_follow_rust_spacing`); and indentation
discipline without a linter: three new layout errors, each replacing a silent
wrong program (a line matching no open block, a continuation starting with a
statement keyword, a `;` inside an inline block). The last retired the inline
statement form `do: let a = 1; a * 2`: one expression per inline block, and
several statements on one line are Rust's braces.

**The Harsh Book (69 to 72 tests).** Chapters 1 to 21 were written in order,
205 snippets, every one transpiled, compiled and run by `book/build.py` (now
`book/sources/build.py`). The bugs it found are listed in their own part
below. In the same sessions:

- **`$` came back with a new meaning: apply to nothing.** `f$` is `f()`,
  `fn main$:` declares no parameters, and `()` is only ever the unit value
  (`f ()` is `f(())`); the old `(())` spelling went. Measured before deciding:
  397 calls and 118 declarations in the corpus against zero real `(())`.
  Pinned by `dollar_applies_to_nothing`.
- **One parameter per group.** Counting the parameter forms found three that
  produced the same Rust. The curried group became the one syntax, the bare
  single parameter its one-group corollary, and `fn f (a: T, b: U)` a syntax
  error at the comma (`rejects_comma_parameter_lists`). The notes record this
  as an amendment to "the syntax is frozen" that removes a spelling and adds
  none.
- `use std.io:` with entries beneath had never worked; the author confirmed
  it was never intended, and it became an error naming the paren tree
  (`rejects_use_block`). A written `,` in a comma block was refused
  (`rejects_comma_in_comma_blocks`).

**Macros, first design (80 tests).** `docs/MACROS.md`, six rules, decided in
conversation with a code sample per decision, out of a real test: five
notebooks of Rust notes converted with `hrs-from`, whose `view!` bodies the
transpiler mangled. Markup bodies (rule 1) and brace trees (rule 6, for
Dioxus) were given Harsh forms, `name! do:`. Both forms were retired on
2026-09-23.

**Parens transparent to layout (81 tests).** In the author's terms: parens
isolate or raise precedence, and neither open nor close anything. A block
opened inside a group ends where the group ends; `map (|p|: if *p > 1: *p else: 0)`
works. A pre-existing bug fell out: `fold 0 (|acc, x|:` as a paren block had
emitted `fold(0) (…)`. Pinned by `blocks_inside_groups`. From the same
conversation, two lines entered `GOVERNANCE.md`: one spelling is *preferred,
not enforced*, and *leave to rustc what rustc rejects*.

**`hrs fmt` (85 tests).** Designed in `docs/FMT.md` with four decisions taken
before code (no inserted pipes, no expanding or collapsing inline blocks, one
fixed style, parens transparent). Built in two steps: indentation only, then
re-breaking. The no-op test over the corpus flagged 24 files; seven were the
formatter's bugs and the rest were the guide's wording losing to the Book's
practice. The chain rule (three links or more vertical; one or two on a line
within 72 columns) took its number from measuring the corpus. Rule 0 (the same
Rust before and after) was checked in tests and, from the Leptos work on,
enforced at run time.

**One syntax for applying (85 tests).** The author named an inconsistency:
attributes were still written `#[derive(Debug, Clone)]`. The principle, in
his words: remove anything that implies using parens to pass arguments to a
function, anywhere in Harsh. `f(x)` tight became an error everywhere, types
and attributes included: `#[derive Debug Clone]`, `Fn (i32) (i32)`,
`fn (x: i32) (y: f64) -> &str`. Rule 3's brace-body exemption was withdrawn.
Optional grouping parens stopped being emitted.

**A real crate.** The author supplied `sr-auto-leptos` (20 files, 2,700 lines,
Leptos 0.8 and Axum 0.8) with the brief "rewrite an existing project in
Harsh". Leptos did not build on the container's rustc 1.75, so the method was
token comparison: convert each file, transpile it back, compare with the
original. First run: 10 of 22 identical; last: 21. What real code found is
listed with the oracles. The first `hrs cargo leptos build` on the author's
Mac finished both halves (server binary and WebAssembly).

**2026-09-09: braces on one line; structs and enums redesigned.** The
converter had written `pub fn main$ { // … }`. The author's rule: braces hold
a block on one line, and over several lines a block is `:` or `do:`. That left
struct literals as the one construct in braces, and the morning's redesign
followed. Declarations open by grammar with no mark (`struct Point` with
fields beneath); `\` marks a field list wherever a mark is needed, inline
declarations and every literal (`Point\ x = 5.0, y = 7.0`); literal fields
take `=`; patterns are `Point\ x, ..`; tuple payloads are applications
(`struct Meters f64`). The choice of `\`: it has no meaning in Rust or Harsh
outside a string. A `Point:` literal form from the same morning was retired by
the afternoon. The corpus moved (85 files by script, a dozen by hand).

**2026-09-10: the first publication.** Doc examples became Harsh
(`src/docex.rs`): a fence rustdoc would compile is code, so it is written in
Harsh and transpiled. The Book became one edition with no Rust in it; its
two-column edition and the `@rust` marker were retired. Two guards were added:
the Book's prose guard against retired spellings, and `docs/check-guide.py`,
whose first run found the guide's tutorial and many of its blocks on retired
spellings. The block-literal shape (`let u =` / `User\` / fields one
unit past it) became a formatter rule. Then publishing: the home on GitLab, a
mirror on GitHub, the crate on crates.io as `hrust` 0.1.0, then renamed the
same night to `harsh-lang` (`hrust` was the one name of five that differed)
and `hrust` deleted within the 72-hour window with zero downloads; the
extension on the Marketplace as `harsh-lang.harsh-lang`. State recorded that
night: 103 tests, the round trip byte-exact, the Book at 221 snippets.

### 2026-09-11: first fixes published; a rule for every form

`harsh-lang` 0.1.1 and 0.1.2 went to crates.io. 0.1.1 carried a fix found by
Harshlings' runner, the first Harsh program written from scratch: a written
`;` ending an inline block (`if c: f$;`) was dropped. The author's reason for
fixing it as a bug rather than documenting it became a governance principle,
**a rule holds in every form it applies to**
(`inline_block_keeps_its_written_semicolon`). 0.1.2 made a tight index part of
its atom: `f arr[1]` had emitted `f(arr)[1]`, indexing the result silently;
a spaced `[` inside an application became an error naming both spellings
(`tight_index_is_an_atom_and_spaced_index_in_an_application_is_refused`).

Two hours were lost between the two releases to things that were not code: a
bundle packed while its copy step was broken, archive timestamps older than
the clone's last build (cargo ran a stale binary), and an attempt to re-publish
a spent version. The rule written that day: a version is spent the moment it
is uploaded.

Harshlings reached fifty exercises the same day, each verified to fail as
written and pass when solved. Zed was paused with two Enter causes found and
not fixed.

### 2026-09-12 to 2026-09-19: blocks and the two macro worlds (0.1.5 to 0.1.12)

**2026-09-12** was a day of design with nothing built. The author reframed `:`
as an opener rather than a header terminator: a construct's own shape says
where its header ended. From that, Rust's braces split into real blocks
(statements) and pseudo blocks (fields, variants, arms, item bodies, DSLs).
`impl`, `trait`, `mod` and `extern` were to lose their `:` like `struct` and
`enum`. The `#…:` pseudo block, whose opener names its separator, was
designed in full. For library DSLs the author rejected every mechanism that
asked the programmer to produce Harsh a DSL would accept: the converter should
say what the Harsh version is.

**2026-09-13.** Harsh would expand its own declarative macros rather than
translate them into Rust's `macro_rules!`. The author's argument: translation
must decide at the definition what Rust punctuation a repetition maps to,
before knowing where the expansion lands, and so guesses intent.

**2026-09-17.** The two macro worlds were sealed. `macro_rules!` stays Rust's,
a zone copied verbatim in both directions (`src/rawzone.rs`, 0.1.6);
`macro_rules~` is Harsh's, called `twice~ 4` (0.1.5). The block kit landed
(0.1.7): no mark after a header that ends itself, `\` for comma lists, `:` or
`do:` for statements, and `#:` for a grouping with no separator. The same day
the VS Code extension learned to find `hrs-lsp` itself when launched from the
Dock, without a login shell's `PATH` (extension 0.1.2), after the author lost
block indentation and said to fix it in the plugin rather than in a setting.

**2026-09-17 to 2026-09-18.** The Harsh macro system was built in stages in
`src/mac.rs`: the definition read into a tree with fifteen fragment kinds;
matching and substitution with syntax contexts on every token; then wired
into the transpile path, so a `macro_rules~` unfolds into Harsh before
anything is transpiled and leaves no trace in the Rust. A third-party audit
the author asked for that day found seven faults, among them hygiene correct
for `let` only (three probes gave 6, 3 and 2 where Rust gives 11, 300 and
100) and `hrs expand`, promised by the Book, missing. Two guards came out of
it: `every_corpus_file_transpiles` and `every_example_compiles`.

**`match v\` (0.1.9, 2026-09-18).** Decided on the 12th and never built: the
tree still wrote `match v:` 56 times in the corpus. The author's taxonomy of
statement blocks and specification blocks went into `GOVERNANCE.md`; a match's
arms are a specification block in use, opened by `\`. `match v:` and
`match v do:` were refused by name. 0.1.8 had been published earlier that
day from an earlier state of the day's work, so 0.1.9 superseded it. The same
day found the cause of an extraction gone wrong: every release sheet since 0.1.0 wrote
`tar xzf` without its dash, which macOS's tar reads in the old bundled-flags
mode, ignoring `--strip-components`.

**2026-09-19.** A `~` call became a stream, not an application (0.1.11): it
ends at the newline, the end of the block it opens, or the close of the group
it sits in, and everything inside reaches the matcher as written. The Jupyter
kernel was built (0.1.12), and the source map was carried through expansion so
an error inside an expansion names the call.

**2026-09-20.** The author stated the principle the week's macro rulings had
been circling: a macro is a function of one argument, a token stream; the
syntax of that argument belongs to the DSL, and Harsh only says where it ends.
It went into `GOVERNANCE.md`.

### 2026-09-20 to 2026-09-22: retired spellings, a second book, Harsh's selling points (0.1.13 to 0.1.22)

**0.1.13** removed a bare `$( … )` that the code accepted as "exactly once"
though the author had ruled it out; the record was right and the code wrong.
It also fixed a transcriber that ate the author's tokens: `[$( $a ) x y]`
had produced `[1(x, 2, x, 3)]` without an error.

**0.1.14** began from the author's report that the Book's generation seemed to
have errors. One was Claude's: `check.sh` and the harness run `target/release`
while `cargo test` runs `target/debug`, and 0.1.13's last `check.sh` ran
before its final change. The other was the language's: every migration had
refused a retired header that *ends* in its old mark and left the inline form
accepted, so `match x: a => 1` still worked. Four fixtures in
`tests/roundtrip.rs` asserted that it did. Every retired spelling is now
refused in both forms, pinned row by row in
`retired_spellings_are_refused_in_both_forms`. A `~` stream became bounded by
indentation, and a `\` with no construct before it was refused.

**0.1.15** added *Harsh by Example*, written by hand after a measured
conversion of *Rust by Example* (147 of 171 programs round-tripped) read like
converted Rust; the author confirmed. Wiring it into the gates found that the
corpus guards had walked `book/src`, empty since 2026-09-11, for nine days:
43 files checked instead of 278.

**0.1.16 to 0.1.22** built what the author named Harsh's selling points:
partial application, pipes, comprehensions and linear algebra, with the
tagline *Rust without braces. Rust with pipes, partial application,
comprehensions and linear algebra. Rust for functional programming, data
science and machine learning.* `g~` and `list~`, `set~`, `dict~` went into a
prelude (0.1.16). `hrs_std` arrived as a second crate with `Matrix<T>` and
`Vector<T>` over nalgebra and Julia's model, `m~` and `v~` in the prelude
(0.1.18). A panic names the Harsh line (0.1.19). Slicing, broadcasting and
`f<>` followed (0.1.21), each prototyped before any design at the author's
request: nine candidates ran outside the tree before he ruled. The borrowed
view `&a[0..2, ..]` packed its window into a zero-sized reference; Claude
recorded that it had not been run under Miri.

On 2026-09-22 the foreign-DSL problem was ruled in one conversation with
nothing built (ruling 16 of the design notes): a Rust macro's brace body
verbatim, Harsh holes marked `@: … :@`. The marker was chosen from three
candidates and the ruling records why the other two were rejected.

### 2026-09-23 to 2026-09-25: holes, procedural macros, the distribution (0.1.23 to 0.1.30)

**0.1.23** made a bracket after a macro's bang an argument, an array, and
moved 94 sites (`vec! 1 2 3`). **0.1.24** built ruling 16: `src/dslzone.rs`
sets each `name! { … }` body aside, as `rawzone` does, and transpiles only
its holes. The old DSL modes were retired with the compiler's dead-code
warnings as the checklist: markup bodies, brace trees, `#:` blocks and `m!\\`.
**0.1.25** made the converter write the holes, after the author corrected a
misreading: `@:` exists so the converter can find the Rust inside a DSL. The
author chose an error over a warning when a piece cannot be written as Harsh:
no silent fallback. The sr-auto site converted with 233 holes and 0 errors.

**Procedural macros (0.1.28 to 0.1.30).** The author ruled the direction and
spelling and rejected a shortcut. A proc macro is an ordinary Harsh `pub fn`
marked `#[proc_macro~]`, run by a generated runner when its caller is
transpiled (`src/procmac.rs`, `hrs_proc_macro`). His rule for the rest: "For
all the Harsh proc macros, mirror Rust's!" Derives followed (0.1.29), then
Harsh counterparts to `syn` and `quote` (`hrs_syn`, `hrs_quote`), and
attribute-like macros. The Rust Book's `HelloMacro` and `route`, written in
Harsh, run end to end (`the_rust_books_hello_macro_with_hrs_syn`,
`an_attribute_macro_written_in_harsh_replaces_its_function`). The
declarative matcher was re-ruled: a metavariable is written in its
parentheses, `($name:spec)`, and every other matcher token is literal
(`a_metavariable_is_written_in_its_parentheses`).

**0.1.30 (2026-09-25)** closed the batch. The author's words: "I want to test
it today, publish it, then move to a place of improving instead of designing
the language." His first test failed with "no matching package named
`hrs_proc_macro` found"; the four support crates were then shipped inside
`hrs`, as Rust's `std` ships with the toolchain, and served to cargo through
`--config` patches (`the_standard_distribution_needs_no_registry`). The
author's first `cargo test` on his Mac failed three tests, all in the tests:
macOS's temporary folder is a symlink. Miri on his Mac found undefined
behaviour in the matrix views under Stacked Borrows. He chose to release 0.1.30
with the issue stated, and to make views values afterwards.

### 2026-09-26 to 2026-09-29: the website, rust-analyzer, companions, long lines (0.1.31 to 0.1.55)

**The website.** A Dioxus app written by hand in Harsh, `site/`, part of the
tree. On 2026-09-26 it was switched to Rust and back in one session: the
author had noticed `::` on a page, which turned out to be page content (the
samples' Rust columns), and the Harsh site was restored byte for byte. He
then ruled that the site is written by hand in Harsh and never converted:
"Harsh is a proven concept". The site went live at harsh-lang.com that
evening (0.1.33), and to static generation the next day. Writing its
Converter found a transpiler panic on a multi-byte character inside a hole
(`a_hole_carries_characters_of_several_bytes`).

**The language server's second half.** The author ruled on 2026-09-25 for
rust-analyzer behind `hrs-lsp`, the server staying name-blind. Hover and
go-to-definition came in 0.1.38, completion in 0.1.39. Every answer on the
author's Mac was `null` until `HRS_LSP_LOG` showed why: rust-analyzer loads
nothing under `target/` from disk, so the server now hands it the generated
files as open documents (0.1.40). Hover answers in Harsh from 0.1.43.

**Views made sound (0.1.37).** A view is a value, `a <- view rows cols`,
guarded by the borrow checker; `hrs_std` has no `unsafe` left, and since 0.6.4
forbids it. `&a[range, range]` stopped compiling; its review is the last item
on the roadmap.

**The companions.** *Harsh Design Patterns* (0.1.44) and *The Harshonomicon*
(0.1.45), each checked against the original's table of contents when the
author supplied it (0.1.46, 0.1.47). Each found transpiler bugs (listed
below).

**Long lines.** *Precedence* was documented from the transpiler's actual
behaviour (0.1.49). Harsh blocks were fenced `rust harsh` so GitLab and
GitHub colour them as Rust (0.1.50). The author's handwriting rules for long
lines were applied to the books by judgment (0.1.51), then the countable ones
became formatter rules (0.1.52). A literal inside a one-line literal was
refused unparenthesised, the author's proposal (0.1.53): Claude's own docs had
said an inner literal takes the outer one's remaining fields, and the opposite
was true. `try_solve` and `try_inv` (0.1.54) and `hrs add` (0.1.55) closed
the month.

### 2026-09-30 to 2026-10-03: `()` and `Hrs.toml` (0.2.0 to 0.3.1)

**0.2.0, 2026-09-30.** From building Docreview, the author's rule: a block
whose value is discarded ends with a line `()`, and a `;` ending a line is
refused. Built as 0.1.56 and renumbered before it was pushed: the release is
breaking and Harsh was now public, so the middle number moved.
`RELEASE.md` gained the version rule.

**2026-10-01 to 2026-10-03.** Docreview's nested Harsh crate was not
transpiled by `hrs build`. The author generalised the problem into two modes
(pure Harsh, `hrs` driving; Harsh inside Rust projects) and settled the design
on 2026-10-03: a separate `Hrs.toml` beside `Cargo.toml` for Harsh's
dependencies, the project's own `Cargo.toml` never edited, a generated one for
Cargo, and no `build.rs` mode (0.3.0). The procedural-macro crates stay in
`Cargo.toml`, by his principle "except the macro crates". 0.3.1 did the open
items: `hrs export` taking Harsh crates along, workspace inheritance, and
several `.hrs` files in a Rust project with a `--check` drift test. The cheat
sheet (0.2.1) and its gate, which parses every example with `rustfmt`, came
the same days.

### 2026-10-04: the audit of exceptions (0.4.0 to 0.6.4)

Eight releases in one day. **0.4.0**: the turbofish is `.<T>`, and the old
no-dot form refused; no mixed `if` ("No mixed form", the author). Then the
audit: read-only, it found one exception (the turbofish) and five second
spellings. The author ruled four deliberate (braces for several statements on
one line, `:` or `do:` by style, a body's depth the author's, a lone parameter
bare). The fifth, `a[i, j]` beside `a[(i, j)]`, had been made without him and
is still open for review.

**0.5.0** built the rulings from the audit: `<-` takes a space on each side
(tight, `t<-below` had compiled silently as the field `t.below`); generics are
mapped, never guessed, so 0.4.0's refusal of the no-dot form, itself a guess,
was removed; a block passed as an argument is isolated. **0.5.1**: a name
always follows `<-`. **0.6.0**: a struct pattern has one spelling, `P\ x, y`,
and `docs/DECISIONS.md` was written, the settled decisions with their reasons.
**0.6.1 to 0.6.4**: no guess about a plain `<` in juxtaposition, errors in a
file re-read from its expansion mapped to the right line, the hole layouts (B2,
B7), and `hrs_std` forbidding `unsafe`.

### 2026-10-05 to 2026-10-06: ShopBill (0.7.0 to 0.7.2)

Most of 0.7.0 came from building ShopBill and the rulings of 2026-10-05 it led
to. **One argument, one atom**, taught whole in the Book's §2.7: spaces
separate arguments, so a multi-token argument is isolated, and where nothing
needs separating nothing is. After a constructor the files declare, a single
argument goes bare (`Some &x`); ranges of atoms are atoms (`f 0..3` is
`f(0..3)`, breaking); a tuple index applies to one atom. The generated
project moved from `target/hrs/` to `target/src/` with its own `Cargo.toml`,
and `hrs migrate` updates a project once. A hole became one expression, not a
block. **0.7.1** fixed a release sheet that kept 0.6.4's commands and the tag
`v0.2.1`, which had been run; packing now refuses a sheet naming another
release. **0.7.2**: the constructor table, which only grew, was made to
belong to the files being read.

### 2026-10-12 to 2026-10-13: the applications and the developer book (0.7.3 to 0.8.4)

Seven releases on 2026-10-12, each driven by what the one before left open.
The inputs were Docreview's issues file, ShopBill's, and the developer book,
a separate deliverable written by reading every line of the code, whose
chapter of traps listed bugs.

- **0.7.3**: Docreview's H11 to H13 and the developer book's findings,
  pinned in `tests/docreview.rs` and `tests/book_findings.rs`. The
  self-host gate, which printed `DIFFERS` and went on, now fails.
- **0.7.4**: the author's rulings on what 0.7.3 left open. H9, a long chain of
  one link; Enter after a call still waiting for arguments, which made a
  second reader of the arity table official in `GOVERNANCE.md`; format slots
  in DSL strings as holes ("if it looks like Rust, make it a hole").
- **0.7.5**: a catalogue of where the holes go in eleven libraries' DSLs, each
  library's own Rust compiled against the real crate in `examples/dsl/`,
  checked three ways by `tests/dsl_libraries.rs`.
- **0.7.6**: what a hole is, by the author's rule, decided by a binding table
  in `src/bindings.rs`. The transpiler keeps no such table.
- **0.8.0**: `hrs dx`, built in 0.7.0, ran Deno's `dx` when it was first on
  `PATH`. The author ruled that Harsh arbitrates no conflict between two
  programs of one name: `hrs -- <program>` runs any program in
  `target/src/` as the shell finds it, `hrs which` lists them, and `hrs dx` is
  refused.
- **0.8.1**: Docreview's H14 to H16, among them `hrs fmt` leaving 18 of the
  app's 94 files as written; a second run changed 13 files, so the formatter
  now repeats its passes to a fixed point.
- **0.8.2**: ShopBill's S2 and S7, a chain in an operand of `||`, and a
  process-wide constructor table made one per thread after a test failed once
  in 300 runs.
- **0.8.3, 2026-10-13**: documentation only.
- **0.8.4, in hand on 2026-10-13**: the author's ruling on the layout of a
  multi-line literal argument, `hrs` at a workspace's root
  (`a_workspace_root_runs_each_member`), and a Rouge lexer for GitLab's
  highlighter. The same day SR-Auto was dropped from the project ("enough
  working test projects").

### What was undone

Each row is a form that existed, was used, and was retired or reversed. The
date is when it stopped.

| Form or decision | Replaced by | When |
| --- | --- | --- |
| A conventional parser and semantic tree | A layout transformation | Before 0.1.0 |
| `$` as a currying split, then with project-wide arity | The pipes `\|>` and `<\|` | Before 0.1.0 |
| `()` for "no arguments", `(())` for unit, an empty group vanishing beside others | `$` applies to nothing; `()` is unit | Before 0.1.0 |
| Rust's comma parameter list | One group per parameter | Before 0.1.0 |
| `use std.io:` with entries beneath | The paren tree `use std.io.(Read, Write)` | Before 0.1.0 |
| The inline statement form `do: let a = 1; a * 2` | One expression per inline block; braces for several | Before 0.1.0 |
| The macro DSL exemption | A macro applied like a function | Before 0.1.0 |
| Rule 3's "nothing juxtaposes inside a macro's braces" | Withdrawn with the space rule | Before 0.1.0 |
| `f(x)` tight, `#[derive(Debug)]` | `f (x)`, `#[derive Debug]` | Before 0.1.0 |
| `struct Point:`, `Move:`, `Point { x: 1 }`, `Point:` literals | `struct Point`, `P\` | 2026-09-09 |
| The Book's two-column edition and `@rust` | One edition, no Rust | 2026-09-10 |
| The crate name `hrust` | `harsh-lang` | 2026-09-10 |
| `impl Foo:`, `mod m:`, `trait T:`, `extern:` | No mark on an item body | 2026-09-17 (0.1.7) |
| `macro_rules~ name:` | `macro_rules~ name`, arms by layout | 2026-09-17 (0.1.8) |
| `match x:`, `match x do:` | `match x\` | 2026-09-18 (0.1.9) |
| `name~\` deciding who owns a comma | A `~` call is a stream | 2026-09-19 (0.1.11) |
| Group-per-argument matchers, `(($a:expr) ($b:expr))` | Juxtaposed matchers | 2026-09-19 (0.1.11) |
| A bare `$( … )` meaning once | A repetition takes `*`, `+` or `?` | 2026-09-20 (0.1.13) |
| Retired spellings still accepted inline | Refused in both forms | 2026-09-20 (0.1.14) |
| The planned `dsl.hrs` spelling maps | Holes (ruling 16) | 2026-09-22 |
| `vec! [1, 2, 3]` | `vec! 1 2 3` | 2026-09-23 (0.1.23) |
| `view! do:` markup, `rsx! do:` trees, `#:` blocks, `m!\\` | Brace bodies with holes | 2026-09-23 (0.1.24) |
| A bare metavariable `$name:spec` in a Harsh matcher | `($name:spec)` | 2026-09-25 (0.1.30) |
| `&a[range, range]`, a borrowed view by `Index` | `a <- view …`, a value | 2026-09-27 (0.1.37) |
| A `;` ending a line | A line `()` | 2026-09-30 (0.2.0) |
| `hrs_std` in `Cargo.toml` | `Hrs.toml` | 2026-10-03 (0.3.0) |
| The turbofish inserted for `Vec<u32>.new$` | `.<T>` written; the no-dot form refused | 2026-10-04 (0.4.0) |
| 0.4.0's refusal of the no-dot form | Mapped as written; rustc judges | 2026-10-04 (0.5.0) |
| `<-` with any spacing; `t <- 3` for a tuple item | `x <- name`, spaced; `t.3` | 2026-10-04 (0.5.0, 0.5.1) |
| The braced struct pattern `P { x, y }` | `P\ x, y` | 2026-10-04 (0.6.0) |
| `f 0..3` meaning `f(0)..3` | `f(0..3)` | 2026-10-06 (0.7.0) |
| `target/hrs/` | `target/src/` | 2026-10-06 (0.7.0) |
| `hrs dx` | `hrs -- dx …` | 2026-10-12 (0.8.0) |
| Symbolic links into `target/src/` | A mirror of hard links | 2026-10-12 (0.8.2) |
| A struct literal as a bare argument | Isolated, `push (Card\ …)` | 2026-10-12 (0.8.2) |

Two decisions were undone inside a session and never shipped: the website
switched to Rust and back (2026-09-26), and B2 (code on a hole's `@:` line),
which reached the tree before the author ruled and was removed the same day
(2026-09-24). It was ruled for, in a different form, on 2026-10-04 (0.6.3).

### The releases, as an index

Dates are the changelog's. A number handed over or pushed is never reused
(`RELEASE.md`, *Version numbers*): 0.1.10 and 0.1.31 were built or handed over
and never published, 0.1.56 became 0.2.0, 0.1.8 was superseded by 0.1.9, and
0.1.41 re-released 0.1.40's code under a correct tag. The changelog's 0.1.1
entry gathers the work of the sessions before 2026-09-11, most of which the
session notes date to 2026-09-09 and 2026-09-10, before 0.1.0 was published.
0.1.3 has no entry of its own; its fix is listed under 0.1.4.

| Version | Date | What it carried |
| --- | --- | --- |
| 0.1.0 | 2026-09-10 | First public release: `hrs`, `hrs-from`, `hrs-remap`, `hrs-lsp`, the VS Code and tree-sitter grammars, the guide |
| 0.1.1 | 2026-09-11 | The inline `;` kept; "a rule holds in every form"; the sessions before it |
| 0.1.2 | 2026-09-11 | A tight index is part of its atom |
| 0.1.4 | 2026-09-11 | A spaced `[` after a macro's argument refused |
| 0.1.5 | 2026-09-17 | `~` marks Harsh's own macros |
| 0.1.6 | 2026-09-17 | `macro_rules!` is a zone of Rust |
| 0.1.7 | 2026-09-17 | The block kit; item bodies lose their `:` |
| 0.1.9 | 2026-09-18 | `match v\`; Harsh expands its own macros, with hygiene |
| 0.1.11 | 2026-09-19 | A `~` call is a stream |
| 0.1.12 | 2026-09-19 | The Jupyter kernel; the source map through expansion |
| 0.1.13 | 2026-09-20 | A repetition takes its marker |
| 0.1.14 | 2026-09-20 | Retired spellings refused on one line; the stream bounded by indentation |
| 0.1.15 | 2026-09-20 | *Harsh by Example*; the corpus guards repointed |
| 0.1.16 | 2026-09-21 | `g~` and the prelude |
| 0.1.17 | 2026-09-21 | A comma inside generics belongs to the list |
| 0.1.18 | 2026-09-21 | `hrs_std`; `m~`, `v~`; the four selling points |
| 0.1.19 | 2026-09-21 | A panic names the Harsh line |
| 0.1.20 | 2026-09-21 | Generator comprehensions named; comments in a `~` call |
| 0.1.21 | 2026-09-21 | Slicing, broadcasting, `f<>` |
| 0.1.22 | 2026-09-21 | `hrs fmt` never breaks a line inside a `~` stream |
| 0.1.23 | 2026-09-23 | A bracket after a bang is an array argument |
| 0.1.24 | 2026-09-23 | Brace bodies verbatim, holes `@: … :@`; the old DSL modes retired |
| 0.1.25 | 2026-09-23 | `hrs-from` writes the holes; no silent fallback |
| 0.1.26 | 2026-09-23 | DSL streams in braces; an expansion read as source |
| 0.1.27 | 2026-09-24 | A diagnostic inside a hole points at it |
| 0.1.28 | 2026-09-24 | Harsh's proc macros, the function-like form |
| 0.1.29 | 2026-09-24 | Harsh derives |
| 0.1.30 | 2026-09-25 | `hrs_quote`, `hrs_syn`, attribute macros; the distribution inside `hrs` |
| 0.1.32 | 2026-09-26 | The website in the tree; `single_file`; the kernel for PyPI |
| 0.1.33 | 2026-09-26 | harsh-lang.com |
| 0.1.34 | 2026-09-26 | Matrices in notebooks; `hrs dist` |
| 0.1.35 | 2026-09-27 | The Converter both ways, `driver::convert_str` |
| 0.1.36 | 2026-09-27 | A statement at the top level refused |
| 0.1.37 | 2026-09-27 | Matrix views are values; `hrs_std` without `unsafe` |
| 0.1.38 | 2026-09-27 | Hover and go-to-definition through rust-analyzer |
| 0.1.39 | 2026-09-27 | Completion; `HRS_LSP_LOG` |
| 0.1.40 | 2026-09-27 | rust-analyzer handed the generated files |
| 0.1.41 | 2026-09-27 | 0.1.40's code under its right tag |
| 0.1.42 | 2026-09-27 | Hover in Markdown |
| 0.1.43 | 2026-09-27 | Hover in Harsh |
| 0.1.44 | 2026-09-28 | *Harsh Design Patterns* |
| 0.1.45 | 2026-09-28 | *The Harshonomicon* |
| 0.1.46 | 2026-09-28 | *Design Patterns* checked against the original |
| 0.1.47 | 2026-09-28 | *The Harshonomicon* checked against the original |
| 0.1.48 | 2026-09-28 | Absolute links on the Learn page |
| 0.1.49 | 2026-09-28 | *Precedence*; a 0.1.44 regression fixed |
| 0.1.50 | 2026-09-29 | Harsh fenced `rust harsh` |
| 0.1.51 | 2026-09-29 | The handwriting rules applied to the books |
| 0.1.52 | 2026-09-29 | `hrs fmt` applies the countable rules |
| 0.1.53 | 2026-09-29 | A literal inside a literal refused unparenthesised |
| 0.1.54 | 2026-09-29 | `try_solve`, `try_inv` |
| 0.1.55 | 2026-09-29 | `hrs add` |
| 0.2.0 | 2026-09-30 | `()` instead of `;` (breaking) |
| 0.2.1 | 2026-10-03 | The cheat sheet, checked |
| 0.2.2 | 2026-10-03 | *Harsh at a glance* brought up to the language |
| 0.2.3 | 2026-10-03 | Docreview's H2, H7, H8; inline bodies after a bare parameter |
| 0.2.4 | 2026-10-03 | The site's deploy; the converter's arm literals |
| 0.2.5 | 2026-10-03 | The deploy's link check |
| 0.3.0 | 2026-10-03 | `Hrs.toml` (breaking) |
| 0.3.1 | 2026-10-03 | `hrs export` with Harsh crates; Harsh files in a Rust project |
| 0.4.0 | 2026-10-04 | `.<T>` (breaking); no mixed `if` |
| 0.5.0 | 2026-10-04 | `<-` spaced; generics mapped, never guessed (breaking) |
| 0.5.1 | 2026-10-04 | A name always follows `<-` |
| 0.6.0 | 2026-10-04 | `P\` patterns only (breaking); `DECISIONS.md` |
| 0.6.1 | 2026-10-04 | No guess about `<` in juxtaposition |
| 0.6.2 | 2026-10-04 | Errors through a `~` expansion's brace body on the right line |
| 0.6.3 | 2026-10-04 | The hole layouts, B2 and B7 |
| 0.6.4 | 2026-10-04 | `hrs_std` forbids `unsafe` |
| 0.7.0 | 2026-10-06 | One argument, one atom; ranges as atoms; `target/src/` (breaking) |
| 0.7.1 | 2026-10-06 | The release sheet names its release; the layout of `&` |
| 0.7.2 | 2026-10-06 | A constructor's single argument bare in `hrs fmt`; the constructor table per file set |
| 0.7.3 | 2026-10-12 | Docreview's H11 to H13; the developer book's findings |
| 0.7.4 | 2026-10-12 | Rulings: H9, Enter after a waiting call, format slots |
| 0.7.5 | 2026-10-12 | The catalogue of library DSLs |
| 0.7.6 | 2026-10-12 | What a hole is, by a binding table |
| 0.8.0 | 2026-10-12 | `hrs --`, `hrs which`; `hrs dx` retired (breaking) |
| 0.8.1 | 2026-10-12 | Docreview's H14 to H16; `hrs fmt` in one run |
| 0.8.2 | 2026-10-12 | ShopBill's S2 and S7; the constructor table per thread |
| 0.8.3 | 2026-10-13 | Documentation only |
| 0.8.4 | in hand, 2026-10-13 | A literal argument's layout; `hrs` at a workspace's root; a Rouge lexer |

The test count is a second index. The session notes record it at each step:
29 at the five changes, 103 at 0.1.0, 162 at 0.1.12, 202 at 0.1.27, 259 at
0.1.30, 275 with none ignored on 2026-09-25, 357 at 0.7.3, 379 at 0.8.0, and
390 at 0.8.3.

## The language, feature by feature

Each feature below gives the rule, the files it lives in, and the ruling
behind it. The pipeline order is `lex.rs`, then `layout.rs` (which also runs
`juxt.rs`'s token passes per logical line), then `emit.rs`; `rules.rs` holds
the keyword tables both directions share.

### Layout: two rules

A logical line whose last significant token is `:` or `=>` opens a block; a
more-indented line in any other position continues the current logical line.
The second rule is what makes multi-line chains work with no continuation
marker, and why a type annotation's `:` (always mid-line) never opens a block.
`src/layout.rs` says of itself that it "understands exactly one thing: which
lines are nested inside which". `src/lex.rs` records, for each token, whether
it begins a physical line and that line's indentation, and nothing about
blocks.

Inside a group, layout follows the same rules: a line inside an open `(`
indents past the line that opened it or is the `)` (`check_group_lines`).
Indentation discipline is enforced by errors rather than a linter: a line
matching no open block is refused with the columns it could have used
(`rejects_misaligned_lines`), and an `else` may sit at the column of any
physical line of the header it answers (`accepts_aligned_continuations`).

### Blocks: statement blocks and specification blocks

The author's taxonomy (2026-09-18, in `GOVERNANCE.md`): a statement block
holds statements separated by `;`, opened by `:` or `do:`; a specification
block holds entries separated by `,` (fields, variants, arms), opened by `\`
in use and by nothing in a declaration's multi-line form. Item bodies
(`impl`, `trait`, `mod`, `extern`) take no mark: their headers end
themselves. `macro_rules~` is the one exception, arms separated by layout.

- An inline block holds one expression. Several statements on one line are
  Rust's braces, `{ let u = 3; u * u }`, kept on purpose.
- Braces hold a block on one line; over several lines a block is `:` or `do:`
  (2026-09-09).
- No mixed `if` (2026-10-04, 0.4.0): an inline branch takes an inline `else`
  (`no_inline_if_with_a_block_else`).
- A `\` follows the construct it specifies; alone it means nothing (0.1.14,
  `a_backslash_follows_the_construct_it_specifies`).
- A statement at the top level is refused (0.1.36): Rust's top level holds
  items only, and Harsh's call syntax was passed through untransformed there
  (`a_statement_at_the_top_level_is_refused`).

Files: `layout.rs` (`classify`, `build_block`, `check_old_marks`,
`check_inline_old_marks`, `check_backslash_head`, `check_top_level`),
`emit.rs` (separators).

### Statements end at the newline; `()` discards

Until 0.2.0 a `;` was optional and an explicit one honoured. From 2026-09-30
a `;` ending a line is refused, and a block whose last value is discarded ends
with a line `()` (`layout::check_trailing_semi`). The Rust is written as Rust
is, `seen.insert(word);`, with no `()`. The converter writes no `()` after a
`let` or after a macro that produces no value; `rules.rs` holds that list as
`UNIT_MACROS`. Brackets keep their `;` (`[u8; 4]`, `m~ [1 2; 3 4]`).

### Substitutions: `.` for `::`, `<-` for `.`

Rust's `::` is Harsh's `.`, and Rust's `.` is Harsh's `<-`. `::` is a syntax
error in Harsh. Floats, tuple indexes and ranges keep their dots because they
are different tokens. The lexer has two modes, differing only in what `.`
means (`src/lex.rs`), so the converter can read Rust.

- **`<-`** is spaced on both sides and followed by a name (0.5.0, 0.5.1;
  `arrow_has_a_space_on_each_side`, `a_name_follows_the_arrow`). Rust itself
  lexes `<-` as one token and asks for `x < -3`.
- **Generics in an expression** are `.<T>`, Rust's `::<T>` (0.4.0,
  `the_turbofish_is_written_with_a_dot`). A plain `<` is mapped as written and
  rustc judges it (0.5.0, 0.6.1). `DECISIONS.md` lists the seven places `<`
  opens a generic list.
- **Use trees** are `use std.io.(self, Write)`.

### Application: juxtaposition and isolation

`f a b` is `f(a, b)`; parentheses isolate one argument and never group
several, so `(a, b)` is always a tuple. The rule behind every case
(2026-10-05): spaces separate arguments, so a multi-token argument is
isolated, and where nothing needs separating nothing is.

- `$` applies a name to nothing: `f$` is `f()`, `fn main$:` (before 0.1.0,
  `dollar_applies_to_nothing`).
- Application binds tighter than `<-`: `f x <- g$` is `(f x) <- g$`
  (`application_binds_tighter_than_arrow`).
- A tight index is part of its atom, `f arr[1]`; a spaced `[` inside an
  application is refused (2026-09-11).
- A tuple index applies to one atom; a call's result is indexed isolated,
  `(f src).0`, `(f src)[i]` (0.7.0, 0.7.3; `a_tuple_index_on_a_calls_result`,
  `an_indexed_group_applies_its_index_once`).
- A range of atoms is an atom, `f 0..3` (0.7.0, `ranges_of_atoms_are_atoms`);
  a call right after a range's `..` is applied (0.8.1,
  `h14_a_call_after_a_ranges_dots_is_applied`).
- After a constructor the files declare, or `Some`, `Ok`, `Err`, a single
  argument goes bare, `Some &x` (0.7.0). The constructors are read from the
  declarations, never from a name's case (`s5_constructors_take_a_single_argument`,
  `h13_a_record_variants_fields_are_not_constructors`).
- A literal cannot be applied: `assert_eq! a + 1 b` had been
  `assert_eq!(a) + 1(b)` silently (0.1.24, `a_literal_cannot_be_applied`).
- A block passed as an argument is isolated, `f ({ … })` (0.5.0,
  `a_block_after_a_name_is_refused`).
- The layout of `&`: a reference tight, bit-and spaced (2026-10-06, the
  author's convention; `the_layout_of_ampersand`). The transpiler accepts any
  spacing.
- The space rule: `f(x)` written tight is an error everywhere, in patterns,
  declarations, attributes and types (before 0.1.0).

Files: `juxt.rs` (`regions`, `atom_end`, `apply_region`, `check`,
`rewrite_dollar`, `range_groups`, `grouping_fixups`, the constructor table).
`juxt.rs` computes expression regions per line rather than transforming
everywhere and carving out exceptions: adjacent names
mean application in an expression, and something else in a type
(`&mut Formatter`) or a declaration (`let mut sum`).

### Parameters: one group each

`fn add (a: i32) (b: i32) -> i32:`; a lone parameter may drop its group,
`fn double n: i32 -> i32:`. Rust's comma list is refused at the comma
(`rejects_comma_parameter_lists`); a bare parameter followed by a group is
refused, since `fn add &self (k: i32)` once emitted valid, wrong Rust
(`rejects_bare_parameter_followed_by_group`). A comma inside a type's generic
list belongs to the list (0.1.17,
`a_comma_inside_generics_belongs_to_the_generic_list`); the shared function,
`rules::param_list_commas`, is safe only in a parameter list, where `<` cannot
be a comparison. Documented parameters put the doc lines above each group, as
Leptos's `#[component]` needs. Files: `rules.rs` (`fn_param_groups`),
`layout.rs` (`check_fn_params`, `check_bare_param`), `emit.rs`
(`param_fixups`).

### Structs, enums, literals and patterns

Three struct forms, named as equals in the Book and the guide (the author's
decision, 2026-09-10): record, tuple and unit-like, each declared in block or
inline form. An enum is a union of variants, each one of the three. Literals
are `P\`: `Point\ x = 1.0, y = 2.0` inline, or `Point\` with one field per
line beneath; fields take `=` so a literal can never borrow a `>` or a `:`
from around it. Patterns take the same mark, `P\ x, y`, one spelling since
0.6.0. Nesting is isolated: a literal inside a one-line literal (0.1.53,
`a_literal_inside_a_literal_is_parenthesised`), a literal in a tuple
(`a_literal_in_a_tuple_is_isolated`), a literal as an argument (0.8.2,
`s2_a_bare_literal_argument_is_refused`). Where clauses stand on the header
line, bracketed: `struct W<T> [where T: Clone]`
(`a_structs_where_clause_stands_on_its_header_line`).

Files: `layout.rs` (`literal_at`, `decl_header`, the literal checks),
`juxt.rs` (`tuple_struct_fixups`), `emit.rs` (`backslash_fixups`, and
`scrutinee_fixups`: a literal as a `match` scrutinee is parenthesised, which
Rust requires).

### Closures, and parens transparent to layout

A closure is a prototype `|x|` followed by a body in any block form, `:`,
`do:` or `{ … }`, inline or indented. As a trailing argument its body may sit
beneath (`thread.spawn move ||:`, `move_closure_as_trailing_argument`); inside
a group, the paren block (`<- map (` / `|x|:` / body / `)`) with the chain
continuing after the `)` (`paren_block_tail_after_nested_block`,
`blocks_inside_groups`). A chain after a bare closure block is refused
(`rejects_chain_after_bare_closure`). Files: `layout.rs`
(`logical_lines`, `expand_item`, `inline_colon`), `juxt.rs`
(`closure_params_end`).

### The pipes and partial application

`|>` fills parameters from the left, `<|` from the right. When the count
equals the arity, a plain call; fewer, one flat closure over what is missing;
too many, a Harsh error; unknown arity (std, other crates), a plain call. Each
side is a sequence of atoms, so a result is isolated before it is piped,
`(f a) |> g`. Files: `juxt.rs` (`rewrite_pipes`, `collect_arities`), with the
arity table collected across a project by the driver. Pinned by `pipes` and
`rejects_bad_pipes`. Partial application of a function returning a closure
needs no support: `(add 10) 7` is `(add(10))(7)`.

### Attributes and types as applications

Attributes are applications inside their brackets: `#[derive Debug Clone]`,
`#[cfg (feature = "x")]`, `#[cfg_attr (feature = "ssr") (derive Serialize)]`.
Function types are applications too: `Fn (i32) (i32)`, `FnOnce$`,
`fn (x: i32) (y: f64) -> &str`. Files: `juxt.rs` (`regions` for attribute
lines, `type_app_fixups`). The author's attribute rulings of 2026-09-24:
`#[route GET "/"]`, parentheses only around an argument of several tokens.

### Macros

Four kinds, documented by who wrote them (the author's architecture,
2026-09-24): Harsh macros (`~`), Rust macros (`!`) written in a Harsh project,
Rust DSLs (`!`) from Rust libraries, and Harsh DSLs (`~`) from Harsh
libraries. `DECISIONS.md` reduces them to three rules: a Harsh macro's stream
is Harsh; a Rust macro's own language is Rust, with Harsh in holes; and a list
of values is juxtaposed (`vec! 1 2 3`).

- **`macro_rules~`** (`src/mac.rs`): expanded by `hrs` into Harsh before
  transpiling, so the Rust holds no macro of Harsh's. Fifteen fragment kinds;
  hygiene by syntax contexts, with a macro's local respelled only when it
  would clash (`a_clash_moves_the_macros_local_not_the_callers`); Rust's
  recursion ceiling of 128, raised by `#![recursion_limit]`; a call's mark
  checked against its definition (`adding!` against `macro_rules~ adding` is
  refused by name). A call is a stream bounded by its line, deeper lines, and
  its group (`a_harsh_macros_stream_is_bounded_by_indentation_and_by_its_group`).
  A metavariable is written in its parentheses, `($name:spec)` (0.1.30).
- **`macro_rules!`** (`src/rawzone.rs`): a zone of Rust, blanked to a marker of
  the zone's exact shape before lexing and written back byte for byte
  (`a_rust_macro_rules_is_copied_verbatim`).
- **Rust DSLs** (`src/dslzone.rs`): a `name! { … }` body is set aside, as a
  zone is, and put back with each hole `@: … :@` transpiled in place; holes
  nest, `@@:` is a literal `@:`. A hole is one expression, not a block (0.7.0).
  Diagnostics inside a hole point at the hole (0.1.27,
  `an_error_inside_a_hole_points_at_the_hole`).
- **Procedural macros** (`src/procmac.rs`, the crates `hrs_proc_macro`,
  `hrs_quote`, `hrs_syn`): `#[proc_macro~]`, `#[proc_macro_derive~ Name]` and
  `#[proc_macro_attribute~]` on a `pub fn`, mirroring Rust's three kinds. The
  mark is blanked byte for byte so offsets and map entries stay put. A
  generated runner under the project's target folder runs the macros when a
  caller is transpiled; each expansion lives in a span region past any file,
  so no source-gap copy can reach it, and a copied input token keeps its
  origin, so rustc's error lands on the programmer's token
  (`an_error_lands_on_the_programmers_token`).
- **The prelude** (`mac::PRELUDE`): `g~`, `list~`, `set~`, `dict~`, `m~`,
  `v~`, injected only into files that call them, shadowed by a file's own
  macro, always reachable as `hrs_std.NAME~`
  (`the_prelude_comprehension_and_its_shorthands`,
  `a_files_own_macro_shadows_the_prelude`).

### Doc examples are Harsh

A fence in a `///` or `//!` comment that rustdoc would compile is code, so it
is written in Harsh and transpiled; a fence naming another language is prose
and untouched (before 0.1.0). Hidden `#` lines keep rustdoc's marker; a
mistake is an error on the `.hrs` line that holds it
(`a_broken_example_is_an_error_on_its_own_line`). File: `src/docex.rs`, a pass
over the emitted text, shifting the map by each rewrite. Eight tests in
`tests/docex.rs`.

### `hrs_std`: matrices, broadcasting, `f<>`

`hrs_std` is Harsh's standard library, a crate of its own: `Matrix<T>` and
`Vector<T>` over nalgebra, on Julia's model. Harsh has no types, but the
trait system does, so `A * B`, `2.0 * A` and `A * x` pick their meaning at
compile time. The syntax needs no types either:

- `m~ [1 2; 3 4]`, Julia's grammar (`a_matrix_literal_follows_julias_grammar`).
- `a .* b` is written `a * hrs_std::DOT * b`, so Rust's own precedence applies
  (`a_dotted_operator_is_a_pair_of_operators_round_a_dot`).
- `f<> a` is `hrs_std::each!(f, a)`, Julia's `f.(a)`
  (`a_function_marked_each_becomes_the_each_macro`). Rust's empty generic
  list `f::<>` is dropped by `hrs-from`: the changelog calls it the one place
  Harsh is not a superset of Rust.
- A top-level comma in an index makes a tuple, `a[i, j]` is `a[(i, j)]`
  (`a_comma_in_an_index_makes_a_tuple`).
- `solve` and `inv` panic with Julia's messages; `try_solve` and `try_inv`
  return a `Result` (0.1.54), after Rust's own pairs (`borrow` and
  `try_borrow`).

Files: `juxt.rs` (`rewrite_dotted`, `rewrite_comma_index`, `rewrite_each`),
`hrs_std/`, `src/dist.rs` (the distribution's sources as data, kept current by
`the_distribution_is_current`), `src/shake.rs` (only the items an export
reaches).

### Smaller rules

- A name Rust 2021 reserves (`become`, `yield`, `try`, `box`, …) is written as
  a raw identifier (0.2.3, the author's choice over refusing it).
- A file's `include_str!` paths are re-based for the generated tree.
- Raw identifiers (`r#type`) are one token; they had emitted `r)#type`.

## The tools

### The transpiler: `hrs`

`src/main.rs` is the binary; the library is `src/lib.rs`, whose header states
why both directions live in one crate: they must agree on the carve-out list
and the block-kind table. The pipeline:

| Stage | File | What it does |
| --- | --- | --- |
| Zones | `rawzone.rs`, `dslzone.rs`, `procmac.rs` | Set aside `macro_rules!` bodies, Rust DSL bodies and proc-macro marks, blanked to their exact shape so every offset stays put |
| Lexing | `lex.rs` | Tokens with byte spans, line starts and indentation |
| Expansion | `mac.rs` | Unfold `~` macros, declarative and procedural, into Harsh tokens |
| Layout | `layout.rs` | Physical lines to logical lines to a tree of blocks; every layout error |
| Application | `juxt.rs` | Expression regions; `$`, the pipes, juxtaposition, the matrix rewrites |
| Emission | `emit.rs` | Rust out, and one source-map entry per token |
| Doc examples | `docex.rs` | Fences in doc comments transpiled, the map shifted |

`emit.rs` copies the whitespace between tokens from the source instead of
regenerating it. That decision made the source map close to an identity: `>>`
never becomes `> >`, chains keep their shape, and an error lands on the right
column. The map is keyed on bytes, because rustc's `column_start` counts
characters and diverges from its byte offsets on any non-ASCII line
(`docs/TUTORIAL.md`, verified rather than assumed).

The single-file form, `hrs in.hrs -o out.rs --map out.map.json`, came first.
Since 0.3.1 it takes several files that see each other's functions, writes a
header naming each source, and `--check` fails when a `.rs` is not what its
`.hrs` makes, for continuous integration.

### The converter: `hrs-from`

`src/unbrace.rs`, Rust to Harsh. Its header gives its method: the lexer tracks
bracket depth, so every `{` has a known partner, and the whole problem reduces
to one question asked once per brace, does this brace become indentation or
stay a brace? Only braces at paren and bracket depth zero are considered.

- It writes the holes in a DSL's body (0.1.25) and decides which Rust becomes a
  hole by a binding table (`src/bindings.rs`, 0.7.6): the prelude, the file's
  items, its imports, the bindings in scope at the call, the names the DSL
  binds. A stretch of glob-imported names with no operator is left as written
  with a warning (`a_stretch_of_glob_names_is_left_with_a_warning`).
- A piece it cannot write as Harsh stops it with the line named; each hole is
  checked by transpiling it back (0.1.25, "no silent fallback").
- It formats only a conversion that transpiles (`format_if_sound`,
  2026-09-25), so a converter gap is reported as it is and not reflowed.
- `driver::convert_str` holds its whole path, so `hrs-from`, the website's
  Converter and the language server's hover give the same bytes (0.1.35,
  `convert_str_is_what_hrs_from_writes`).

### The remapper: `hrs-remap`

`src/remap.rs` and `src/bin/hrs-remap.rs`. It reads cargo's JSON, discards
rustc's `rendered` text (which has the generated file's paths baked in) and
re-renders each diagnostic from its spans against the Harsh source. A
zero-width span at a token's end, where rustc's suggestions point, maps to the
end of the token it follows (`SourceMap::remap_insert`). Through an expansion,
it adds *in the expansion of `name~` called at …* (0.1.12). Since 0.1.19 the
driver rewrites panic locations in the program's stderr the same way
(`a_panic_is_put_back_on_its_harsh_line`).

### The driver

`src/driver.rs`, behind `hrs build`, `run`, `test`, `check`, `lint`,
`watch`, `fmt`, `expand`, `export`, `new`, `add`, `migrate`, `dist`,
`publish`, `which` and `--`.

- **Layout of a project.** Harsh sources in `src/**.hrs`; generated Rust in
  `target/hrs/` until 0.7.0, `target/src/` since, with its own `Cargo.toml`.
  The first try on a `cargo new` project built the wrong program (Cargo found
  `src/main.rs` itself), so the driver refuses a manifest with no target under
  the generated tree.
- **Staleness.** By modification times, an equal time counting as stale. A
  newer `hrs` made every build retranspile until 2026-09-24; now a stamp
  records the `hrs` that last transpiled a project
  (`a_different_hrs_retranspiles_once`).
- **Two passes** (0.3.0, 0.7.0). Pass 1 reads `Hrs.toml` and the `.hrs` files,
  transpiles Harsh dependencies first, recursively
  (`a_harsh_app_uses_a_harsh_library_by_path`), and writes a generated
  `Cargo.toml`; pass 2 is Cargo working in `target/src/`. The project's
  top-level entries are mirrored there, real folders with hard-linked files
  (0.8.2, `s7_the_projects_folders_are_mirrored_not_linked`).
- **The distribution** (0.1.30): `hrs_std`, `hrs_proc_macro`, `hrs_quote` and
  `hrs_syn` are written from `src/dist.rs` and served through cargo `--config`
  patches, only for the crates the manifests name by version.
- **`hrs export`** writes a plain Rust crate; since 2026-09-25 it carries
  nothing of Harsh, `hrs_std`'s reached items becoming the crate's own module
  (`export_is_pure_rust`, `export_takes_hrs_std_in_as_its_own_code`).
- **Watch mode** polls every 250 ms with a settle delay: no dependency, no
  platform differences, and editors write in stages.
- **`hrs -- <program>`** and **`hrs which`** (0.8.0); **a workspace's root**
  runs each member in turn (0.8.4).

### The formatter: `hrs fmt`

`src/fmt.rs`, designed in `docs/FMT.md`. It moves tokens between lines and
columns and changes no token. Its structure comes from `layout::lines()`, the
part of the layout pass that never fails, and from `src/columns.rs`, which it
shares with the editor so the two cannot disagree on a chain's shape
(`the_editor_and_the_formatter_share_the_anchor`).

- **Rule 0**: the transpiled Rust before and after must be the same. Checked
  in the tests and, since the Leptos work, at run time: on a mismatch the file
  is left as written, and since 0.7.0 `--check` counts that as a failure,
  naming the file.
- **Re-breaking** (`rebreak`): chains, arguments beneath the callee, the
  block literal's shape, and since 0.1.52 the countable handwriting rules.
- **Bodies** (`place_bodies`, 0.8.1): a DSL body is set aside and put back
  where its call now stands, its text unchanged.
- **Fixed point** (`format_harsh_checked`, 0.8.1): passes repeat until the
  text stops changing, four at most, each checked by Rule 0 against the one
  before (`h16_one_run_is_enough`).
- It never adds or removes a line break inside a `~` call's stream (0.1.22,
  `fmt_never_breaks_a_line_inside_a_tilde_stream`).

### The language server: `hrs-lsp`

`src/bin/hrs-lsp.rs` over `lsp-server` and `lsp-types`, pinned to versions
that build on rustc 1.75 so the container and the author's Mac build the same
graph. Two halves.

- **Its own**, from `src/columns.rs`: on-type formatting on `\n`, `)` and `]`,
  the custom request `harsh/columns` behind Tab and Shift-Tab, and
  `textDocument/formatting` through `fmt`. Name-blind by construction: token
  kinds, spans and the two layout rules, through the same
  `layout::logical_lines` the transpiler runs. The arity table is its one
  exception, for the cursor only (2026-10-12).
- **Through rust-analyzer** (0.1.38 to 0.1.43): hover, go-to-definition and
  completion, asked about the generated Rust and mapped both ways through the
  source maps (`SourceMap::find`, `locate`), the answers shown as Harsh.
  `tests/lsp.rs` drives the real binary against a fake rust-analyzer, which
  since 0.1.40 ignores unopened files under `target/` as the real one does.

Positions are counted in UTF-16 units since 0.7.3
(`columns_convert_between_utf16_and_characters`).

### The editor grammars

| Grammar | Files | Notes |
| --- | --- | --- |
| VS Code | `editors/vscode-harsh/` (`syntaxes/`, `client/extension.js`, `language-configuration.json`) | The TextMate grammar includes Rust's and adds Harsh's operators, and since extension 0.1.5 DSL bodies with holes. The client launches `hrs-lsp` and binds Enter, Tab and Shift-Tab. `indentationRules` were removed and `autoIndent` set to `keep`: the server owns every column. |
| tree-sitter | `editors/tree-sitter-harsh/` (`grammar.js`, `src/scanner.c`) | Required by Zed, where a parse error removes highlighting. Deliberately looser than the transpiler. The external scanner emits INDENT, DEDENT and NEWLINE, with a `pending` field because NEWLINE consumes the line break. |
| Zed | `editors/zed-harsh/` | The extension and a WASM shim returning `hrs-lsp`. Unpublished: `tree-sitter-harsh` must be pushed and its commit set as `rev`; the two Enter causes found on 2026-09-11 remain. |
| Rouge | `editors/rouge-harsh/` | GitLab's highlighter. Rust's lexer with Harsh's tokens added; its spec and Rouge's general suite pass against Rouge's `main` of 2026-10-13. The pull request is the author's to open. |
| Linguist | `editors/linguist/README.md` | A drafted entry. Linguist takes an extension once GitHub Search indexes 2,000 files of it in a year, across many repositories. |

The zero-error-nodes loop over the corpus is the tree-sitter grammar's check.
Its first run was a false pass: `npx tree-sitter` was absent and the count of
error lines was zero on an npm error, so the loop now asserts each file
produced a tree.

### The Jupyter kernel

`kernel/harsh_kernel`, a Python wrapper (0.1.12): each cell is transpiled by
`hrs` and evaluated by the evcxr Rust kernel. Errors are mapped to the cell's
own lines; evcxr prints its own report with a numbered gutter rather than
rustc's `-->`, so the first remapping never matched anything, and the kernel
now rewrites the gutter from the source map (kernel 0.1.1, tested on evcxr's
real coloured output). Since kernel 0.1.3 a cell naming `hrs_std` gets the
standard distribution through `hrs dist`. On PyPI as `harsh-kernel`.

### The books, their harnesses and the docs pipeline

| Book | Harness | Programs (latest recorded) |
| --- | --- | --- |
| *The Harsh Programming Language* | `book/sources/build.py` | 249 (0.1.49) |
| *Harsh by Example* | `by-example/sources/build.py` | 63 (0.1.49) |
| *Harsh Design Patterns* | `patterns/sources/build.py` | 38 (0.1.46) |
| *The Harshonomicon* | `nomicon/sources/build.py` | 36 (0.1.47) |

Every program is transpiled, compiled and run before it reaches the page, and
the page shows its real output. The Book's harness grew by need: `!error`
(the diagnostic through `hrs-remap`), `!panic`, `!test` and `!test!fail`,
`!doc` (doc tests run through `hrs test`), directory projects, cargo-backed
projects sharing one target folder, and crates side by side for proc macros.
It also guards the prose: retired spellings inside backticks fail the build,
and since 2026-09-10 three backticks inside a sentence do too.

`docs/build.py` renders every Markdown document to HTML and a notebook, with
the page chrome in `docs/page-head.html` and `docs/page-tail.html`. It runs
`docs/check-guide.py` first, which transpiles every block of the guide and
compares each two-column block's Rust with the transpiler's, token for token.
`docs/check-cheatsheet.py` transpiles every cheat-sheet example and parses the
Rust with `rustfmt`, because transpiling alone let mangled Rust through.

### The website

`site/`, a Dioxus app written by hand in Harsh, published from the GitHub
mirror to GitHub Pages at harsh-lang.com, every page pre-rendered. Its
Converter and Playground run the transpiler in the browser
(`driver::transpile_str`, `convert_str`); the Playground sends the Rust to the
Rust Playground with the part of `hrs_std` it uses inline
(`driver::single_file`, `single_file_carries_the_hrs_std_it_uses`). Its
editor indents with `columns` and `fmt`, the code `hrs-lsp` and `hrs fmt` use.
Four gates in `check.sh` cover it: every `.hrs` transpiles and is formatted,
the highlighter matches the books' byte for byte, the editor's keys behave
(`site/check-editor.py`), and sample lines fit 52 characters.

### `check.sh`

The gates, in order: a release build; the library without default features
and the macro crates' own tests; the site's four gates; four examples
transpiled, compiled and run; the round trip Rust to Harsh to Rust; the
self-host; a planted type error remapped; `hrs export` built with cargo alone
and compared with the example's output; the guide's checker; the cheat
sheet's; every Markdown fence tagged; every site link absolute; the companion
books' programs transpiled. GitLab's pipeline runs `cargo test` then
`./check.sh` on every push.

### Harshlings

A separate repository: a runner written in Harsh (the first Harsh program
written from scratch, 2026-09-11) and exercises with hints and solutions in
the Book's order, 65 at the latest count. `verify.py`, run by its CI,
requires every exercise to fail as written, every solution to pass and every
hint to exist. Its standing rule: never run `hrs fmt` over `exercises/`,
which are broken on purpose; doing so once repaired `layout01`, and
`verify.py` caught it.

## The oracles

An oracle here is anything that decided, without anyone's opinion, whether
the work was right. Each one below caught real faults; the examples are from
the records.

| Oracle | Checks | Since |
| --- | --- | --- |
| rustc | The generated Rust compiles and runs | The start |
| The round trip over the transpiler's own source | Rust to Harsh to Rust, token for token, byte-exact | The start |
| The self-host | The transpiler converted to Harsh, transpiled, rebuilt, and producing identical output | The start; a gate since 0.7.3 |
| The corpus guards (`tests/fmt.rs`) | Every `.hrs` transpiles, every example compiles, `fmt` is a no-op and token-preserving | 2026-09-18 for the first two |
| Rule 0 | `hrs fmt` changes no Rust | Before 0.1.0; at run time from the Leptos work |
| The books' harnesses | Every program built and run, the output shown | The Book's first chapters |
| The guide's and cheat sheet's checkers | Every block transpiles; Rust columns match; the Rust parses | 2026-09-10; 2026-10-03 |
| Real projects | sr-auto, Docreview, ShopBill, the website, Harshlings | 2026-09-07 onward |
| The author's Mac | rust-analyzer, `dx`, VS Code, a current cargo, macOS paths, Miri | Every delivery from 0.1.8 |
| Reading the code whole | An audit (2026-09-18); the developer book's editions | 2026-09-18; 0.7.3 onward |

### rustc

Transpiling proves nothing about the Rust; compiling and running it does. The
2026-09-18 audit found four corpus files that transpiled to invalid Rust
unnoticed, because `check.sh` transpiled without compiling; among them
`struct P do`, passed through from before item bodies lost their opener. The
same audit found `match x do:` emitting arms separated by `;`. `every_example_compiles` was
added the same day. The same lesson returned on 2026-10-03: the cheat sheet's
gate parses the Rust with `rustfmt`, and its first run found an inline body
after a bare parameter mis-read into broken Rust with no error.

### The round trip over the transpiler's own source

`round_trip_own_source` converts every module to Harsh, transpiles it back and
compares the token streams. The notes credit it with roughly twenty to
twenty-five bugs the examples never reached, four of them in code written
minutes earlier. `docs/TUTORIAL.md` lists a sample from the early sessions:
`pub const X = …` losing its `;`; `return match x:` classified as a statement
block; `if !flag {` read as a macro call; a duplicated `ALWAYS_SEMI` constant
shadowing the shared one, so edits to the shared table did nothing. Later
catches:

- `Option<Vec<T>> = x` converted as `>>= x`, a shift-assignment (the five
  changes).
- `(Span { lo: at, hi: at + 1 }, msg)` converted to a literal whose field list
  swallowed `msg` as a shorthand field: a valid program with another meaning
  (2026-09-10).
- `a && match k { .. } && b`, its tail written as a statement that `hrs`
  transpiled without complaint into `}; && b;` (0.1.22).
- A struct literal in a match arm read as a pattern, found converting
  `layout.rs` (0.2.4).
- `d < -1` converted as `d<- 1` everywhere, found by 0.5.0's spacing rule in
  the transpiler's own source (0.6.0); a literal behind a reference left in
  Rust's braces, found by 0.5.0's block-argument refusal (0.5.0).
- It also refused a change: H14's first fix, reading `f (x)..(y)` as one
  range, failed the round trip of the transpiler's own source and was
  withdrawn (0.8.1).

Its blind spot was recorded too: a normalisation that drops any comma after a
`}`, meant for block-bodied arms, let a converted match guard through that
the self-host build then caught (0.1.23).

### The self-host

`check.sh` converts every source file of the transpiler to Harsh, transpiles
it back, rebuilds `hrs` from the result, and requires that `hrs` to write the
same Rust for the examples. The notes record it catching converter bugs in
code written minutes earlier: `if let E::G { body, .. } = &m {` (2026-09-17,
"the third time"), `impl<'a> Emitter<'a>` (0.1.7), a `match` after a guard
arm, an `if` with a continued string as a field value, an `if` as a macro
argument, an attribute between doc comments, and `v[0].1` (all 2026-09-24 to
2026-09-25). Until 0.7.3 it printed `DIFFERS` and carried on; the developer
book flagged it and it now fails.

### The corpus guards

`every_corpus_file_transpiles`, `every_example_compiles`,
`fmt_is_a_no_op_over_the_corpus`, `fmt_changes_no_token` and
`fmt_is_idempotent`. They caught the formatter changing a matrix: inside
`m~` a line break is a row, and past its width the formatter listed a row of
ten as a column of ten, which ran without a word (2026-09-21). They also
failed silently once: for nine days they walked a directory the Book had left,
43 files instead of 278 (fixed 0.1.15). The walk now asserts how many files it
found.

### Rule 0 in the formatter

Rule 0 kept every formatter fault from reaching a program: a misread shape
leaves the file alone. It was itself wrong twice. Its comparison stripped all
whitespace, so a space added inside a string passed (fixed 0.7.4, now token
for token); and it read `&=` as two tokens, as the lexer did, so `b & = x`
looked like `b &= x` (H15, fixed in the lexer in 0.8.1). A file left alone
was silent until 0.7.0; the 2.3 edition of the developer book notes that a
file that never settles within four passes is still dropped without a word.

### The books' harnesses

Every program in four books is built and run, and the notes require all four
rebuilt before a transpiler change is delivered, because neither `cargo test`
nor `check.sh` compiles them. The rule came from 0.1.49: 0.1.44's "an empty
`impl` gets `{}`" doubled the braces of `impl Error for E {}`, unnoticed until
*Harsh by Example* was rebuilt. What the books found is listed in the next
part.

### The checkers of the guide and the cheat sheet

`docs/check-guide.py`'s first run (2026-09-10) found many of the guide's
blocks failing, its tutorial written in retired spellings (`enum Format:`,
`Report { counts, format }`, `format!(…)`, `#[cfg(test)]`), and a Rust column
claiming `#[derive Debug Clone]` passed through unchanged. Its comparison of
Rust columns then caught a wrong column Claude had just written for a partial
application. It was not run from `check.sh` until 2026-09-27, so 0.1.36's
top-level check broke a guide block nobody saw.

### Real projects

- **sr-auto** (a Leptos and Axum site, before 0.1.0): documented parameters
  had no form; markup bodies opening with a hole; nested paren blocks, whose
  depth was measured from the line rather than absolutely; a chain off a block
  expression; `include_str!` paths broken once the Rust moved to the generated
  tree. Pinned in `leptos_shapes`, `converter_leptos_shapes` and
  `leptos_build_findings`. Its conversion with holes (0.1.25) gave 233 holes
  and no errors, and 0.1.26 found nested JSON taken for one hole.
- **Docreview**, the author's document review app written in Harsh, kept an
  issues file: H2 and H5 to H8 (0.2.0, 0.2.3), H9 (ruled 0.7.4), H11 to H13
  (0.7.3), H14 to H16 (0.8.1), and round 76's `||` (0.8.2). The `()` rule
  itself came from building it, and its nested crate led to `Hrs.toml`.
- **ShopBill** kept S1 to S8. Most of 0.7.0 came from it; S2 and S7 were fixed
  in 0.8.2 (`tests/shopbill.rs`, every test failing on the release before its
  fix).
- **The website** found the multi-byte panic inside a hole (0.1.31), the
  stale-edit scrambling in its own editor (`3, 2` shown as `3 3 32`, traced to
  the editor and pinned by `every_website_sample_round_trips_both_ways`), and
  four converter bugs when its Rust was converted back.
- **Harshlings**: the dropped inline `;` (0.1.1), `assert_eq! v [10, 20, 30]`
  emitting `assert_eq!(v) [10, 20, 30]` (0.1.4), and a bodiless foreign
  signature whose bare parameter swallowed the return type.

### The author's Mac

The container had rustc 1.75 and no rust-analyzer, `dx`, VS Code or nightly.
From 0.1.8 every delivery was a full bundle so the author could test what
Claude could not run. The Mac found: rust-analyzer answering `null` for every
question (0.1.40); three tests assuming the temporary folder was not a symlink (2026-09-25); the
VS Code extension not finding `hrs-lsp` from the Dock (extension 0.1.2); and,
under Miri, undefined behaviour in the matrix views under Stacked Borrows,
the crate's own safety note having called it "not established" (2026-09-25).
Miri also caught a fragile test comparing a `powf` result exactly.

### Reading the code whole

The audit of 2026-09-18 treated every claim in the notes as unverified and
ran something to confirm each; its seven findings are in the chronology. The
developer book, written by reading every line, listed fourteen likely bugs in
its first edition, all fixed in 0.7.3 and twelve pinned in
`tests/book_findings.rs`:
among them a byte string `b"\""` ending at its escaped quote
(`a_byte_string_has_escapes`), a `macro_rules!` with a one- or two-letter
name moved to the top of the output (`a_short_macro_name_stays_in_place`),
and the tree shaker dropping `#![forbid(unsafe_code)]` and the `use` after it
(`the_shaker_keeps_an_inner_attribute`). Its edition 2.3 found
`take (1)..(split n) 2` becoming `take(1)..(split(n), 2)`, which waits on the
author's ruling.

Two practices strengthened the oracles. Proc-macro tests that passed on their
first run were each checked for the ability to fail by a mutation; one, which
looked for `main.hrs:3:` anywhere in the output, stayed green with the map
relocation removed and was rewritten to check the error's own line
(2026-09-24). And the *Rust by Example* measurement (146 programs round-trip,
21 genuine failures) was read by its distribution, not its total: a converter
change once moved `macros/variadics` from refused to compiling and printing
`(1) + 2 = 3`, while the headline stayed the same (0.1.14).

## The bugs the books found

Writing a teaching book was a stronger test than the suite. The notes on
*Harsh by Example* put it so: each thing it taught was a sentence Claude would
have written wrongly. The list below is in order; each item was fixed and
pinned unless it says otherwise.

**The Harsh Book, chapters 1 to 21 (before 0.1.0).**

- A trailing comment on a statement swallowed the inserted `;`; on a
  block-opening header it commented out the `{`
  (`trailing_comment_keeps_separator`).
- `move ||:` and `async:` were not recognised as block-owning trailing
  arguments (`move_closure_as_trailing_argument`).
- A bare parameter's region swallowed a `[where …]` clause, and a one-token
  bare parameter, `fn f self`, lost its `(` (both in `curried_parameters`).
- The `)` tail of an isolated closure with a nested block took its indent from
  the wrong line (`paren_block_tail_after_nested_block`).
- `hrs run` dropped program output lines that parse as JSON (`5050`, `true`):
  a tokio example printed `5050` and the Book showed nothing.
- `hrs new` still wrote the old `fn main ():`.
- `fn add &self (k: i32)` emitted invalid Rust silently; now a Harsh error.
- The converter's `atom_end` never handled `>>`.

**Rebuilding and amplifying the Book (2026-09-10).** The rendered Book stopped
at §14.2, found by the author: an unanchored fence pattern read three
backticks inside a sentence as a fence and swallowed the page. Every rendered
page carried the guide's title. Twenty-eight path labels stood over nothing in
the Harsh-only edition. Five chapters still taught the struct spellings their
own snippets had left; the new prose guard then found fifteen more stale
spellings in chapters 10, 11, 17, 20 and 21.

**The audit (2026-09-18).** `hrs expand`, promised by the Book, did not exist.

***Harsh by Example* (2026-09-20).** Six behaviours were written into the book
as the language has them, not as assumed: `abs -3` is a subtraction; a bound
juxtaposes, so `impl Fn(i32) -> i32` is refused; a block-bodied closure may
end a chain only at statement level; attribute arguments juxtapose; an `impl`
with no items keeps Rust's braces; `let _ = thread.spawn …`. Wiring the book
into the gates found the corpus guards walking nothing.

**Chapter 16's matrices (2026-09-21).** A 75-column `m~` line exposed the
formatter turning rows into columns. The comprehension chapter's annotated
example found a comment inside a multi-line `~` call commenting out the rest
of the expansion (0.1.20). An example written to show `dict~` found a comma
inside generics refused as a parameter separator (0.1.17): the notes record
that ordinary Rust, `Result<T, E>` as a parameter, had never been written in
Harsh by anyone.

**The proc-macro chapter (2026-09-24 to 2026-09-25).** The harness itself,
which truncated each manifest before reading it; two macro parts whose
headings shared anchors, so every link under 23.6 led to 23.5; and an example
asking for `hrs_proc_macro = "0.1"`, hidden by the harness's redirect to the
tree until the redirect was removed, which a reader copying it would have met
as "no matching package".

***Harsh Design Patterns* (0.1.44, 2026-09-28).** A declaration with no body
whose header holds a `:`, `trait Loggable: Debug + Display`, was read as an
inline block that swallowed the line (`a_marker_trait_and_its_blanket_impl`).

***The Harshonomicon* (0.1.45, 2026-09-28).** An empty `enum` emitted
`enum Void;`; a body-less declaration with several parameters kept its groups
apart when a type held a `:` (`a_bodiless_declaration_with_several_parameters`);
a function-pointer type with `extern "C"` or `unsafe` was not joined.

**Rebuilding *Harsh by Example* for *Precedence* (0.1.49).** The `{} {}`
regression from 0.1.44.

**The handwriting rules applied to the books (0.1.51).** An inline `if` on
the line after `let x =` not transpiled; `hrs fmt` flattening a nested `else`
in a `let`-bound `if`; a `match`'s arms pushed far right under a broken chain.
Recorded then; the last was fixed in 0.4.0.

**The cheat sheet (0.2.1, 2026-10-03).** The inline-body bug, fixed in 0.2.3;
and *Harsh at a glance* still teaching `;`, `impl Point:` and `#:`, rewritten
in 0.2.2.

**The books after 0.5.0.** Five programs and two sentences wrote a tuple's
item as `<- 0`, a second spelling (0.5.1); chapter 22's prose still showed
`Message.Move { x, y }` (0.6.0).

**The Book's builder (0.7.3).** It still read the generated Rust from
`target/hrs/` after 0.7.0 moved it, so it stopped short and the website kept
0.6.4's Book until 0.7.3.

**The developer book (0.7.3 to 0.8.4).** Its findings are listed under the
oracles; one, `take (1)..(split n) 2`, is open.

## Principles

### Those that held

From `GOVERNANCE.md`, the roadmap's *Principles that decide new items*, and
the notes' *Practices that mattered*:

- **No semantic analysis in the transpiler.** No types, no name resolution.
  `GOVERNANCE.md` calls it "the single most valuable property Harsh has". Held
  throughout. The tables that came later are bounded by it: the constructor
  table (0.7.0) is read from the files' declarations, never from types; the
  converter's binding table reads Rust and nothing it holds reaches the
  transpiler; the arity table counts parameters, never types.
- **No divergence from Rust semantics.** Harsh changes how Rust is written,
  never what it means.
- **Harsh is a superset of Rust.** It states rules only for what it changes.
  The one recorded exception is Rust's empty generic list, dropped by
  `hrs-from` (0.1.21). The question whether `macro_rules!` must leave Harsh
  files was raised on 2026-09-24 and answered on 2026-09-25: it stays.
- **Rules are simple bricks applied recursively**, not big rules with
  exceptions. The author's framing for the comma-in-generics fix: small
  bricks, composed, generalised only where safe.
- **A rule holds in every form it applies to** (2026-09-11). Its first test
  was the inline `;`; its largest application was 0.1.14, retiring every
  spelling in both its forms; the turbofish head that did not apply to a
  juxtaposed argument (fixed by 2026-09-25) was recorded as this rule failing
  in one of its forms.
- **Strict over permissive.** `docs/TUTORIAL.md` names three bugs from an
  accommodating lexer (a `.` carve-out after literals, `::` passing through,
  Rust's call syntax inside macros accepted) and none from the converter,
  which applies rules uniformly. There is a converter for translating; the
  language does not need to be forgiving.
- **Isolating parentheses resolve ambiguity.** The author's stated principle,
  and the answer to S2 on 2026-10-12.
- **Verify against a real compiler.** `GOVERNANCE.md`'s standard of evidence.
  `docs/TUTORIAL.md` lists the claims it overturned: that `Vec<i32>::new()`
  works in an expression, that `include!` breaks span mapping, that indented
  closure bodies were impossible, that closures cost something at run time
  (one assembly dump showed the closure forms aliased to the direct call).
- **Keep the round trip byte-exact.** Tolerated differences hide real ones.
- **A macro is a function of one token stream** (2026-09-20); a rule about the
  contents of a macro's argument is not Harsh's to make.

### Those that were amended

| Principle as first stated | Amendment | Date |
| --- | --- | --- |
| "The syntax is complete and frozen" | Amended for one parameter per group, removing a spelling and adding none; many breaking changes followed | Before 0.1.0 |
| "No second spelling" | Reworded as a prescription: one spelling is preferred, not enforced; a spelling is refused only for consistency or because it would compile to something else | Before 0.1.0 |
| Semicolons optional, inserted from layout, an explicit one honoured | A `;` ending a line is refused; a discarded value is a line `()` | 2026-09-30 (0.2.0) |
| The turbofish inserted, "never written by hand" | Written `.<T>`, as `::` is written `.`; then nothing inserted and nothing guessed | 2026-10-04 (0.4.0, 0.5.0) |
| `<-` read with any spacing | A space on each side, refused otherwise: tight, `t<-below` compiled silently as a field | 2026-10-04 (0.5.0) |
| The arity table is for the pipes alone | Also read by the editor's Enter, for the cursor only; never changes the Rust | 2026-10-12 (0.7.4) |
| No symbol table, the arity table excepted | The converter keeps a binding table of its own, over the Rust it reads, to place holes; the transpiler still keeps none | 2026-10-12 (0.7.6) |
| The converter marks plain names as holes, `{@: count :@}` (pinned 2026-09-23) | A single word is no hole | 2026-10-12 (0.7.6) |
| A chain of one link is never broken nor moved (2026-09-10) | With long arguments it moves after its `=`, arguments listed beneath the method | 2026-10-12 (0.7.4, H9) |
| The same | For a long literal argument after a long receiver, the link goes to its own line first | 2026-10-13 (0.8.4) |
| Leave to rustc what rustc rejects; "rustc's error is enough" for a bare literal argument (2026-10-05) | A bare literal argument is refused with the isolated form named, a clearer message and no form added, for this form only | 2026-10-12 (0.8.2, S2) |
| A version is spent when uploaded (2026-09-11) | Also when handed over more than once | 2026-09-18 |

Two other rulings show the "leave to rustc" test at work. A bracket after a
bang written the Rust way, `vec![1, 2, 3]`, was refused (0.1.23) because it
would otherwise compile to a vector of one array: the test itself. A
statement at a file's top level was made a Harsh error (0.1.36) although rustc
caught a plain `let x = 3` there: the website's Converter has no rustc, and a
line holding Harsh passed through untransformed, so the refusal went beyond
the test.

## Lessons

Stated from the records, with the evidence for each.

**Run the thing; do not reason about it.** Every impossibility claim made
without trying was wrong: indented closure bodies, juxtaposition without an
expression parser, partial application without name resolution. The comprehension design was settled by writing the hard case in
both shapes and running it. The notes' working rule: when two designs are
argued, write the hard case in both.

**A silent wrong program is the worst outcome, and the converter is where it
hides.** The bugs the records treat most seriously compiled: `f arr[1]` as
`f(arr)[1]`; `t<-below` as a field; a tuple literal swallowing `msg`;
`a && match …` written as a statement; a matrix row turned into a column; a
`~` stream continued over lines emitting `[1, 2]3(4)`. Each led to a refusal,
and refusal was preferred over miscompiling where both were possible: a fix
for two `if` arguments that made the input accepted but wrong was reverted at
once (2026-09-25).

**Silent fallbacks cost the most time.** The formatter left files as written
without a word until 0.7.0; the self-host gate printed `DIFFERS` and carried
on until 0.7.3; the corpus guards walked an empty directory for nine days;
the Book's builder read a moved folder and stopped short. The fixes share a
shape: a gate that counts what it saw, and a failure that stops the run.

**The binary under test must be the one just built.** For 0.1.13 every gate
passed against a release binary one change old, because the gates run
`target/release` and `cargo test` runs `target/debug`. Packing now refuses a
release binary older than any source. Before 0.1.0 a newly installed `hrs`
reused the older one's output for unchanged files; the first fix compared the
binary's time with each output's and so retranspiled on every build, and the
stamp of 2026-09-24 settled it.

**Shared state breaks under parallel tests.** The constructor table only grew
across a process until 0.7.2, so one file's names leaked into the next; then
it was process-wide, so two transpilations at once reset each other's, and a
test failed once in 300 runs. It is one per thread since 0.8.2.

**Two copies of one logic drift.** A duplicated `ALWAYS_SEMI` shadowed the
shared one. The emitter's path for synthetic tokens, a hand-written copy of
the normal path, lacked the turbofish and `>>`, so every macro with
`collect<Vec<_>>$` in its transcriber was broken (0.1.16). The language
server reuses the transpiler's `logical_lines` and the formatter reuses
`columns.rs` for this reason, and `driver::convert_str` makes `hrs-from` and
the website one path.

**A guard that walks a path must assert what it found.** The corpus guards'
nine blind days, and a tree-sitter loop that passed on an npm error.

**A test must be shown to fail.** The proc-macro tests were checked by
mutation, and one was found unable to fail. Tests that asserted a retired
spelling (four fixtures asserting inline `match x:`) were how that spelling
survived its migration.

**Records drift faster than code.** The guide's tutorial was on retired
spellings when its checker first ran; the resuming notes once taught
`match x:` to every new session; release sheets were copied forward with
examples nobody ran (0.1.13) and commands from another release (0.7.1). The
answers were checkers for every document that shows Harsh, and a release
sheet that names its own version.

**Releases and archives.** A `tar` overlay never deletes, so dropped files
stayed in the repository (2026-09-11); `tar xzf` without its dash ignored
`--strip-components` on macOS (2026-09-18); the disk filled repeatedly during
the gates and showed up as mass failures with empty messages. The notes'
rules: run the gates one at a time, delete each cache after it, and check `df`
before believing a mass failure.

**Divide the work by who can verify it.** The container could not run a
current rust-analyzer, `dx`, VS Code, Miri or a symlinked temporary folder;
the author's Mac could. Every finding in that list came from the Mac, and
from 0.1.8 every delivery was built so the author could test it.

## Where it stands, 2026-10-13

From the roadmap. Waiting on the author's ruling: `take (1)..(split n) 2`;
`a[i, j]` beside `a[(i, j)]`; whether `&a[range, range]` returns as a view.
His to run: publishing, Miri over `hrs_std` and the real rust-analyzer on his
Mac, Zed's publication, the Rouge pull request. Next to last, a Harsh crate
registry served as static files; last, analytics for the website.
