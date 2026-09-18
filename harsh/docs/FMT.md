# `hrs fmt` — design

Written before the code, as `docs/LSP.md` was. The formatter is the last tool; it is written once, against a language that has been used at length — the 205 Book snippets, the guide's snippets and the examples are its corpus, all hand-written, all green.

## What it is

`hrs fmt [path…]` rewrites `.hrs` files into the layout the guide's "Layout style" section prescribes, and nothing else. It is a *layout* formatter: it moves tokens between lines and columns. It does not rename, reorder, add or remove tokens, or change what the file means — the transpiled Rust before and after must be token-identical, and that is its first test.

## What it must not touch

- **Authoring.** Blank lines are the author's. `fmt` never inserts or removes one, except to collapse three or more into two. Where a block begins and ends is visible from indentation; where a paragraph begins is the author's decision, and the Book shows what an author does with it.
- **Comments.** Every comment stays on the line it was on, at the end of the line it ended. A comment on its own line keeps its position among the statements and takes the indentation of the next statement.
- **Line breaks inside a continuation.** An author who broke a chain, a literal, or a signature across lines meant to. `fmt` keeps every physical break and fixes only the columns. (It may *add* breaks — a chain wider than the line goes vertical — but never joins lines the author broke.)
- **Rust's braces.** What is inside `{ … }` is Rust's and is left alone, byte for byte: macro bodies, `extern` blocks, struct literals, inline `{ a; b }` blocks.
- **Spelling.** `f (x)` and `f x` are both legal; `fmt` does not choose between them. Nor between `fn top &self:` and `fn top (&self):`, nor between an inline block and an indented one.

## What it does

Every rule below is already stated in the guide; `fmt` is their implementation. **Step 3 (done) is indentation only:** the formatter changes no token and no line break; it assigns a column to every physical line and re-emits. The rules, as they proved out against the whole corpus (the no-op test found where the guide's wording and the Book's practice disagreed, and the Book won each time):

1. **Indentation unit.** Four spaces. Tabs become spaces.
2. **Block bodies** indent one unit past **the physical line holding the construct that opened them** — `let guess = match n:` indents from `let`, `let sign =` / `if n > 0:` from the `if` line, `fn f (a: T)` / `[where …]:` from the `fn` line, a `=>` arm from itself. (`FMT.md` first said "the opener's own column when mid-line"; the Book indents from the line, everywhere.) One exception, decided for the editor and kept: a block opened inside a group on the same line as its `(` — the compact `<- map (|x|:` — indents from the construct after the `(`.
3. **Break after `=`** (step 4, done): `let row =` on its own line when what follows on the `=` line is a group left open past the line (`let row = vec! [` → `let row =` / `vec! [`; `let u = compute (1 + 1) (`; `let leaf = Rc.new (Node {`), or a chain that begins on the `=` line and continues for two or more links. A receiver with a single continuation link stays with its `=` (`let f = File.open "x"` / `<- expect …`), as the Book writes it four times. A block after `=` (`let guess = match n:`) is not broken: the Book keeps it. The moved part carries the rest of its logical line with it, so what is inside braces keeps its shape.
4. **Chains** (decided by the author from the corpus, with the widths measured): **a chain is a run of arrows each applied to the result of the one before** — `self <- width > other <- width` is two chains of one link, on two operands, and is left alone. **Three or more links: vertical**, every link after the first on its own line, whatever the length. **One or two links: one line**, unless the code of that line would pass **72 columns** (measured without a trailing comment; 80 is the terminal, 72 leaves room for an indent or a short note, and the author's `thread.spawn (…)` / `<- join$` / `<- unwrap$` at 79 stays vertical as he wanted); then every link stands alone. A chain that is the value of a `=` is measured on the line after the `=` first: `let statuses: Vec<Status> = (0u32..3) <- map Status.Value <- collect$` (74) becomes `let statuses: Vec<Status> =` / `(0u32..3) <- map … <- collect$` (49, horizontal); too long even there, it goes vertical under the receiver. A paren block's header counts its tail's links (`v <- iter$ <- map (` … `) <- collect$` is three). Joining never crosses a comment or a group that spans lines. **The receiver decides the vertical shape** (the author's algorithm, 2026-09-09; `LINK_ALIGN = 12`): links two-to-last go on their own lines; the first link stays on the receiver's line when its arrow sits within 12 columns of that line's start, and the links align under it; past that the receiver stands alone and every link, the first included, hangs one unit in. `receiver <- very_long_method_name` / `<- …` aligned; `very_long_receiver_name` / `<- very_long_method_name` / `<- …` hanging. What alignment costs is the arrow column, so the receiver's width is the whole test — his words: "it's the receiver length that decides everything". After a paren block's `)` the chain resumes at that column. Pinned in `tests/fmt.rs::the_receiver_decides`; six corpus files moved. **Recursive** (2026-09-10): a chain inside an argument is judged by the same count from its own receiver (`map (|x| x <- a$` / `<- b$` aligned under the inner arrow), and a paren block's tail continues its header's chain -- `)` / `<- fallback` / `<- with_state` go vertical when the whole is three links. Pinned in `chains_recurse_and_tails_continue`.
4b. **A paren block's closure prototype on its own line** (step 4): `<- map (|x|:` with the body beneath and the `)` on its own line becomes `<- map (` / `|x|:` / body / `)`. The compact form whose `)` ends the body's last line is kept.
5. **Bracket groups.** Anchor = the callee path before the bracket (over the complete groups of earlier arguments: `compute (1 + 1) (` anchors on `compute`; `vec! [` on `vec`; `$(` on `$`), else the bracket itself; `[where` always anchors on its `[`. Contents one unit past the anchor; a closer first on its line under the anchor. The compact form (construct on the `(` line, one-line body, closer ending the line) is kept.
6. **Closers** as in 5.
7. **Continuations** otherwise sit one unit past the statement's first line, and keep their relative nesting: a line the author indented past the previous continuation goes one unit past it (`where` / its bounds; multi-argument calls).
8. **Braces are Rust's**, and markup is the framework's: lines inside `{ … }` and the markup lines of an HSX body keep their shape, shifted with the line that opened them.
9. **Trailing whitespace** removed; one newline at end of file.

Nothing here changes a token, so rule 0 — same Rust out — is checkable after every rule, and is (`tests/fmt.rs`). **It is also enforced at run time:** `fmt::format` transpiles its input and its output and compares the tokens; if they differ, or the output no longer transpiles, the file is returned unchanged. A formatter that reads structure from indentation can misread a shape it has not met (a paren block inside a paren block with markup inside was the first), and when it does, leaving the file alone is the only right answer. `hrs fmt --check` counts such a file as formatted.

## Architecture

The layout tree (`layout::build`) discards too much: pipes are rewritten before it exists, comments are split off, blank lines collapse to a count, inline and indented blocks are indistinguishable. The formatter needs a **lossless line model**, not a new parser:

- `lex::lex` — tokens with byte spans (already lossless).
- `layout::lines()` — `physical_lines` + `logical_lines`, the never-failing part (already what `columns.rs` uses). A logical line knows its physical lines, its indent, its comments, its blank lines before, whether it opens a block, whether it is a tail.
- **New: `fmt.rs`** builds a `Doc` — a tree of blocks whose leaves are logical lines with their physical breaks — from `lines()`, then assigns a column to every physical line by rules 2–7, then re-emits tokens with the original inter-token whitespace within a physical line and the computed indentation at its start.
- `columns.rs` already answers "where may a physical line start" for the editor; `fmt` reuses `anchor_col`, `innermost_open`, the chain-arrow rule and the closer rule. The formatter is `columns()` applied to every line at once, plus the chain and break-after-`=` decisions that need the whole statement.

Validation: for every file, `transpile(fmt(src))` must be token-identical to `transpile(src)`; `fmt(fmt(src)) == fmt(src)` (idempotent); and over the corpus, `fmt(src) == src` for every file already in the recommended layout — the no-op test, which is the Book. A `--check` flag exits nonzero when a file would change, for CI.

## Decisions for the author, before code

1. **May `fmt` insert pipes?** (`f (g x)` → `x |> g |> f`.) Recommendation: **no** in `fmt`; yes as an option of `hrs-from`, where there is no author intent to preserve. A formatter that rewrites expressions is a refactoring tool. **Decided: no.**
2. **May `fmt` expand an inline block to the indented form**, or collapse one? Recommendation: **no**. Both are legal spellings; the choice is the author's. `fmt` only fixes an inline block's *columns* if it spans lines (it cannot). **Decided: no.**
3. **Config file or one fixed style?** Recommendation: **one fixed style**, no options, like `gofmt`. The unit is 4, the width is 100, and `[where …]` goes on its own line. A language this small gains more from every file looking the same than from a knob. **Decided: one fixed style.**

4. **Inline blocks inside parentheses** — `map (|p|: if c: a else: b)` does not work today, because the layout treats the inside of a group as token soup until the matching `)`. That is a *layout* change, not a formatter one, and it is the first step of the formatter session, before `fmt` fixes columns around a form that would change. **Decided, in the author's terms:**
   - **Parens isolate, or raise precedence. They neither open nor close anything.** A block opens where its opener says and ends where its own form says. A block owns no token outside the group that contains its opener — so when a `)` closes, every block opened inside it is over, not because the paren ended it but because there was nothing more for the block to hold. The boundary is a consequence, not a rule.
   - **In a prototype**, `fn f (a: T) (b: U) -> R:`, the colons inside the groups are annotations: the `fn` header is still open, it has not reached its own `:`. Same for `let x: T` and struct fields. This is already how the layout reasons (a `:` opens a block only after a block keyword or a closure prototype).
   - **A closure** is a prototype `|x|` followed by a body — a block in any of its forms, `:`, `do:` or `{ … }`, inline or indented — or an inline expression. A prototype without a body is an error or something else. `|x|:` and `|x| do:` both produce `|x| { … }`: `do:` produces exactly one pair of braces wherever it stands. With braces the body ends at the matching `}` and the `)` must follow; with `:` or `do:` the body runs to the end of the tokens it can own — the line, or sooner the `)` of its group: `(|p|: if c: a else: b)` → `(|p| { if c { a } else { b } })`; line-final `:` inside `(` with the body beneath and `)` as the tail is the paren block that exists today, now a case of the same statement.
   - **Lines inside an open group** follow the layout's column rules as everywhere: a deeper line is a continuation, the `)` at the opener's column is the tail, anything else is an error. This is the strict half and may flag Book snippets with Rust-style aligned argument lists; each is a snippet to reindent or a rule to sharpen, and the harness lists them.
   - `[ … ]` and `{ … }` stay Rust's. Tuples and patterns are unaffected. First case to test: a closure with both colons, `(|p: &i32|: *p + 1)`.
   - See `docs/GOVERNANCE.md`: a spelling is rejected for consistency or because it would otherwise compile to something else, and what rustc rejects is left to rustc.

## Order of work

1. Decisions above recorded here. **Done.**
2. Decision 4: parens transparent to layout. **Done** — implemented, corpus clean under the strict rule with nothing flagged, pinned by `blocks_inside_groups`; a pre-existing bug fixed on the way (`fold 0 (|acc, x|:` emitted `fold(0) (…)`).
3. `fmt.rs`, indentation only, with the three validations in `tests/fmt.rs` over the whole corpus. **Done.** The no-op test flagged 24 files at first; the diffs were read one by one: seven were the formatter's own bugs (`else` after a break-after-`=` `if`; `[where` anchoring on the type before it; `use` read as a callee; the chain column after a paren block's `)`; `where` bounds flattened; `$(` as an anchor; the pipes' synthetic positions), the rest were the guide's wording losing to the Book's practice (rules 2, 4, 5 above). Twelve corpus files were then brought in line — six Book files by hand into the preferred `let row =` / `vec! [` form, six example files by `hrs fmt` itself — and the corpus is a no-op.
4. Re-breaking. **Done** as `fmt::rebreak`, a pass before indentation: after `=` before a multi-line group or a chain; a paren block's prototype onto its own line; and the chain rule — three or more links vertical, one or two on one line within 72 columns (joining short vertical chains, splitting long or long-enough ones). Every shape is pinned in `tests/fmt.rs::fmt_shapes`.
5. `hrs fmt` and `--check` in the driver. **Done.** `hrs-from` pipes through `fmt`. **Done.**
6. Format on save: `textDocument/formatting` in `hrs-lsp`, one whole-document edit. **Done**, pinned in `tests/lsp.rs`; VSCode and Zed use it with their own format-on-save setting.

## Arguments beneath the callee -- built (2026-09-10)

**The rule (the author's, parallel to the chain rule):** (1) a call's arguments stay on the callee's line when the whole line fits within `CHAIN_WIDTH` and no argument is a block; (2) otherwise every argument takes its own line, one unit past the callee's start -- all or none, never a staircase; (3) recursive -- a call or a chain inside an argument is judged by the same rules from where it now stands (an inner application is measured from its new column, not its old); (4) a block argument is `(do:` with the body one unit past the `(` and the `)` under it, and the converter writes that form for a multi-statement block argument. One argument stays with its callee (nothing to list); a lone `(|x|:` keeps the paren block's own shape; a `=` ends its line when its value goes vertical. Built in `fmt::rebreak` (`argument_heads` from the juxtaposition fixups, `extent_end`, `moved`) and `plan` (an argument first on its line: one unit past its callee). 37 corpus files moved, mostly long `println!` lines. Pinned in `arguments_beneath_the_callee`. Also: **a chain of one link is never broken nor moved, whatever its length** (rule 1 of the chain algorithm, now literal), and the re-break's shifts no longer compound when several breaks land on one line.

## Earlier notes (2026-09-07), superseded by the section above

**Arguments listed beneath the callee.** When a call's arguments do not fit
its line, each argument takes its own line beneath the callee, one unit in.
A continuation, never a block: the callee's line ends in an argument or the
callee itself, not in `:` or `do:`, so nothing can mistake it for one.
Combined with the chain rule: a link goes vertical, and if its arguments
still pass the width, they go vertical under it.

```
let app =
    Router.new$
        <- leptos_routes
               (&leptos_options)
               routes
               ({let leptos_options = leptos_options <- clone$; move || shell (leptos_options <- clone$)})
        <- fallback (leptos_axum.file_and_error_handler shell)
        <- with_state leptos_options
```

rather than the one long link line the converter writes today. The layout
reads this form already (deeper lines continue the statement); the formatter
and `hrs-from` have to produce it. The width is the chain's, 72 columns of
code. The author's words: "when the multiple arguments become too long together
on one line, we can list them on lines below the function binding."

**A block-bodied argument beneath its callee** (same rule, same session).
When the argument is a paren block, the callee ends its line, the block's
`(||:` sits one unit in beneath it, its body and `)` under that, and the
chain resumes under the first arrow:

```
pub fn tips$ -> &'static [Tip]:
    TIPS <- get_or_init
                (||:
                    let parsed: TipFile = …
                    parsed <- tip
                )
         <- …
```

rather than `TIPS <- get_or_init (||:` with the `)` at the statement's column
and the next link beneath it, which the converter writes today.

**Converter spacing:** `#[ derive Deserialize]` — a space after `#[` and none
before `]`. Either `#[derive Deserialize]` or `#[ derive Deserialize ]`, never
one side. A converter bug; the transpiler and the editor take both.

**A chain of two links goes vertical when an argument carries a chain of
its own** (the author, 2026-09-07 evening). Three links are vertical by the
count rule already; this is the two-link case that fits on a line but
reads badly because the inner arrows compete with the outer ones:

```
tips$ <- iter$                                 rather than
      <- find (|t| t <- slug == slug)          tips$ <- iter$ <- find (|t| t <- slug == slug)
```

The one-line form stays accepted; the formatter writes the vertical one. In
the author's words: "put in evidence the method call nested in the closure."
