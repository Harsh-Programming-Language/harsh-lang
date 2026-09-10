# Start here

**Harsh.** *Rust without the braces.*

Harsh is Rust with indentation instead of braces. This bundle contains the complete toolchain, the language documentation, and working examples.

## Read in this order

| File | What it is |
|---|---|
| `docs/page-head.html`, `docs/page-tail.html` | The rendered pages' chrome: the CSS and the contents-panel script. Edited by hand; every page is built from them. |
| `docs/build.py` | Verifies the guide's code blocks (`docs/check-guide.py`), then renders every document in `docs/` and the book to `.ipynb` and `.html` beside its `.md`. The `.md` is the source; the rendered forms are never edited by hand. Run after any document change (`book/build.py` runs it too). |
| `book/` | *The Harsh Programming Language*: Rust taught in Harsh, for readers who do not know it. `python3 book/build.py` builds `book/HARSH-BOOK.md` from the chapter files and the snippets in `book/src/`, transpiling, compiling and running every one, then renders `harsh-book.ipynb` and `harsh-book.html` — the three forms are always produced together, for the guide and the book alike. One edition, and no Rust code in it: the generated Rust is built and run, and stripped from the page. |
| `` | Regenerates `language.ipynb` and `language.html` from `docs/LANGUAGE.md`. The `.md` is the source; the rendered forms are never edited by hand. Run after any guide change. Development bundle only. |
| `README.md` | What the project is, how to build it, how to run the examples. |
| `language.ipynb` | The language guide as a notebook, one cell per heading. Same content as `docs/LANGUAGE.md`. |
| `tutorial.ipynb` | The tutorial as a notebook, one cell per heading. Same content as `docs/TUTORIAL.md`. |
| `docs/LANGUAGE.md` | **The language guide.** Every construct, with the Rust it produces. Start here to learn Harsh. |
| `docs/INSTALL.md` | The commands: install the binaries, verify the tree, install the editor extensions, the day-to-day `hrs` commands. |
| `docs/ROADMAP.md` | What is left to build, sized and ordered, plus the one decision that is blocking three items. |
| `docs/TUTORIAL.md` | The techniques used to build this, and the lessons that generalise. Worth reading before extending it. |
| `docs/GOVERNANCE.md` | Who decides language direction, and what the project will not do. |
| `CONTRIBUTING.md` | How to propose changes; the contributor agreement. |
| `docs/BRAND.md` | The mark: what it says, how it is set, and why. |
| `docs/TRADEMARK.md` | The name is reserved; the code is not. |
| `docs/SPEC.md` | Implementation notes: the layout rules, block-kind table, source map format, and the bug list the self-hosting loop produced. |
| `docs/LSP.md` and `docs/FMT.md` | The language server: what it knows about a line, why it reuses the layout pass, how it stays name-blind, and the protocol. |

## Quick start

```
cargo build --release
./check.sh
```

`check.sh` builds the toolchain, transpiles every example, compiles and runs each one, round-trips them back through the converter, rebuilds the transpiler from its own converted source, and demonstrates diagnostic remapping. It needs network access on first run for `serde_json`.

## The three binaries

| Binary | Direction | Purpose |
|---|---|---|
| `hrs` | Harsh → Rust | The transpiler. Writes Rust plus an optional source map. |
| `hrs-from` | Rust → Harsh | Brings existing Rust code in. |
| `hrs-remap` | — | Rewrites rustc and clippy diagnostics to point at your `.hrs` source, with correct line *and* column. |

```
hrs src/main.hrs -o generated/main.rs --map generated/main.map.json
cargo build --message-format=json | hrs-remap --map generated/main.map.json
hrs-from existing.rs -o existing.hrs
```

## Source layout

```
src/lex.rs        tokens with byte spans; knows nothing about blocks
src/layout.rs     logical lines, then a tree of blocks
src/rules.rs      keyword tables shared by both directions
src/emit.rs       Harsh -> Rust, plus the source map
src/unbrace.rs    Rust -> Harsh
src/lib.rs        exposes the above to the binaries
src/main.rs       the `hrs` binary
src/bin/          `hrs-from` and `hrs-remap`
```

Both directions live in one crate deliberately: they must agree on the carve-out list and the block-kind table, and separate crates would let those drift apart silently.

## Examples

Each `.hrs` file has its generated Rust beside it.

| Example | Covers |
|---|---|
| `general.hrs` | std only. Traits with required and default methods, generics with lifetimes, multi-line `where`, nested modules, closures, `Result`, `while let`, `if let`. |
| `edge.hrs` | `match` with inline and block arms, `impl`, `enum`, tuple indexing, turbofish, `do:`, doc comments. |
| `params.hrs` | Curried parameter groups, bare parameters, `$` for no parameters, function-typed parameters. |
| `hello.hrs` | An axum web server. Attributes, `async`/`.await`, macros, extractor parameters, multi-line use trees. Needs `axum`, `tokio`, `serde`. |
| `general.roundtrip.hrs` | What `hrs-from` produces from `general.generated.rs`. Compare against `general.hrs` to see where the converter is conservative rather than idiomatic. |

## Current state

Working: layout, token substitution, curried parameter declarations, source map, diagnostic remapping, and the Rust-to-Harsh converter. The transpiler round-trips through its own converter to a fixed point — converted to Harsh, transpiled back, rebuilt, and the resulting binaries produce byte-identical output.

Deferred, because each needs an expression parser rather than a layout pass:

- Juxtaposed application at call sites (`f x y`). Parameter declarations are curried; arguments are not.
- Indentation-form struct literals (`Point:`). Use `Point { x: 1 }`.
- `<-` for type paths (`Router <- new$`). Use `Router.new$`.

## Resolved

- **Paste compatibility is not a language concern.** `hrs-from` converts Rust to Harsh and `hrs` converts back; the language itself makes no accommodation for Rust syntax.
- **`::` is a syntax error in Harsh.** The path separator is `.`; there is one spelling, not two.
- **The path/member mapping is a plain substitution.** `::` becomes `.` and `.` becomes `<-`, both directions, no context and no exceptions. Floats, tuple indexes and ranges are separate tokens, not special cases. The transpiler does not try to detect a misplaced `.` -- it cannot come from the converter, only from a typo, and rustc reports those.

## Open decisions

See `docs/ROADMAP.md` for what remains and `docs/SPEC.md` for the one implementation decision still open (the call-paren style in generated Rust). The two that used to be listed here are settled: block-bodied closures inside a call are written `f (|x|:` with the body indented and `)` on its own line, and `<-` for type paths was dropped -- `Type.method` is the path form and `<-` is member access only.
