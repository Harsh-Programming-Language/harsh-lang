# The language server

Design for `hrs-lsp`, the minimal language server whose first feature is layout-aware on-type formatting. Written before the code; the code follows it.

## What it is for

Editors indent by pattern: VSCode's `onEnterRules` and Zed's `indents.scm` can say "indent after a line ending in `:`", and that is all they can say. Harsh's layout style asks for three things no pattern expresses:

- Enter after a line of a vertical chain — `<- map (…)` — lands **under the previous arrow**, whatever column that is. The column is a property of the previous logical line's tokens, not of its indentation.
- Enter after a line ending in `=` lands **one level in**: the next line is a continuation of the binding, and the block that follows indents from there.
- Tab on the fresh line **moves between the layout's legal columns** — the body of the innermost block, the indent of each enclosing block, the continuation column, the arrow column — and nothing in between, since every other column is either an error or a shape the style rejects.

The server does this, and since 0.1.38 **hover and go-to-definition**, through rust-analyzer (below).

## Hover and go-to-definition, through rust-analyzer (0.1.38)

The server still learns no names or types: it asks rust-analyzer, which knows them, about the Rust a `.hrs` became, and maps positions both ways through the source maps `hrs` writes -- `.hrs` to `.rs` with `SourceMap::find`, back with `locate` (design: `LSP-RA-DESIGN`, in the development notes).

- rust-analyzer is started on the first hover or definition request -- not at startup, so formatting stays instant -- one per project, as a child process; `HRS_RA` names another program, else `rust-analyzer` on the PATH. Harsh's standard distribution is handed to its cargo as `--config` arguments, so a project naming `hrs_std` resolves for it as for `hrs`.
- Hover answers in Harsh's words (0.1.43): rust-analyzer's signatures and paths, converted by Harsh's own converter (`driver::convert_str`, as `hrs-from`) -- `fn adding (x: i32) (y: i32) -> i32`, `std.slice.Iter` -- in `harsh` blocks the editor colours as Harsh; a line the converter cannot take, and a long documentation example, stay Rust. At the `.hrs` range. A definition inside the project opens at its `.hrs` line; one elsewhere (the standard library, a Rust dependency) is a Rust file and opens as such.
- rust-analyzer loads nothing under a project's `target/` from disk, so the server hands it the text of every generated file (`target/src/**/*.rs`, the distribution's crates included), as an editor hands it an open file -- when it starts, and again after each save, which re-transpiles the project first (0.1.40). While typing, answers come from the last good translation, for lines unchanged since; a changed line is answered with nothing rather than something wrong. After the next good save everything is current.
- No rust-analyzer installed: said once -- `rustup component add rust-analyzer` -- and formatting carries on.
- **Completion** (0.1.39) works on the text as you type it, so it cannot wait for a save: a marker is put at the cursor, the text is transpiled on the side (`target/src/.complete/`), rust-analyzer is shown that Rust in memory, asked at the marker, and set back to the file on disk. Rust's snippets (`len()`) come as plain names (`len`), since Harsh calls differently.
- **Which rust-analyzer** (0.1.39): `HRS_RA` if set; else the one on the PATH (`rustup component add rust-analyzer`); else the one inside an editor's rust-analyzer extension (VS Code, Insiders, Cursor, VSCodium). One that exits at once -- rustup's stand-in when the component is missing -- is passed over.
- It waits for rust-analyzer to finish loading a project (its `experimental/serverStatus`), a minute at most, instead of answering nothing meanwhile. Harsh's standard distribution is handed to it only for a project that uses it. A name passed to a macro -- `apply~ adding (4)` -- maps to its Rust.
- **The log:** set `HRS_LSP_LOG` to a file path in the environment the editor starts `hrs-lsp` from -- for VS Code, quit it and start it from a terminal: `HRS_LSP_LOG=/tmp/hrs-lsp.log code .` -- and every question to rust-analyzer and every answer is written there -- the way to see why an answer is empty.

## Name-blind by construction

The transpiler is a layout transformation: no types, no name resolution, no arity outside the pipes. The server is held to the same rule, and the way it is held to it is that it reads only three things:

- **Token kinds and spans** from `lex::lex`. Identifiers are opaque; the server never compares one to another.
- **The two layout rules** — a line whose last significant token is `:` or `=>` opens a block; a more-indented line anywhere else continues the current logical line — applied through `layout::logical_lines`, the same function the transpiler uses.
- **The keyword table** in `rules.rs`, and only to the extent the layout pass already reads it. In the first version not even that: `:`, `=>`, `=` and `<-` decide everything.

The arity table is for the pipes alone (`docs/GOVERNANCE.md`); the server does not read it. Nothing in the server would change if every identifier in the buffer were renamed.

## What the server needs to know about a line

For the line the cursor has just been placed on, the server needs the structure of everything **above** it, and nothing below. That is the whole of its model:

- **The block stack** at that point: each open block's header line and the column its body lines sit at. This gives the *statement columns* — the innermost block's body column, then each enclosing block's, outward to the margin. A new line at one of these columns starts a new statement in that block; at any other column below the innermost it is a layout error.
- **The previous logical line**, with its physical lines still distinguishable (tokens carry `line_start`, so the layout pass loses nothing). From it:
    - Does it **open a block** (`:` or `=>` last)? Then the only sensible column is the body column, and there is no continuation option.
    - Does it **end in `=`**? Then the style says the right-hand side starts the next line, one level in from the statement.
    - Does it contain a **top-level `<-`** — one at bracket depth zero? Then the column of its first such arrow is a legal continuation column, and if the line's *last physical line* began with `<-`, the chain is vertical and that column is the default.
    - Is it a **complete statement** otherwise? Then the statement column is the default and the continuation column (statement indent plus one unit) is offered second.
- **The indentation unit**: the smallest positive difference between the indents of consecutive logical lines in the buffer, or 4 when the buffer has none yet. Harsh files use spaces; the lexer counts columns and the server writes spaces.

Everything is a column; the server never produces text other than spaces.

## Reusing `layout::logical_lines`

Yes, directly. Three facts make it the right choice and are the reason the design does not write a second line classifier:

- **It cannot fail.** `logical_lines` returns a `Vec<Line>`, not a `Result`; the layout errors come from the validation passes that run afterwards inside `layout::build`, and the server never calls those. An editor buffer is malformed most of the time it is being typed into, and a classifier that stopped at the first error would stop at the cursor.
- **It runs on a prefix.** The server lexes the buffer up to the end of the line before the cursor's, so nothing the user is in the middle of typing is in scope. The lexer's only errors are unterminated literals and comments; if one occurs in the prefix, the server truncates to the start of the offending token and lexes again, losing only the fragment that could not have opened or closed anything.
- **It is the same function the transpiler runs.** The server's idea of "which line is this a continuation of" is the transpiler's by construction. A second implementation would drift, exactly as the duplicated `ALWAYS_SEMI` once did.

`physical_lines`, `logical_lines` and `opens_block` become `pub` and are wrapped in one `pub fn lines(toks) -> Vec<Line>` so the server has a single entry point and the internals stay private. The `Line` it returns already carries indent, tokens and comments; the server adds nothing to it.

The block stack is computed from the returned lines by a short walk that reapplies rule one: a line that opens a block pushes a frame; a line indented at or below a frame's header pops it. This walk is nine lines and lives in the new module rather than in `layout.rs`, because the transpiler's own tree-building does the same walk with error reporting the server must not have.

## The new module: `src/columns.rs`

One public function and one public type. `columns(src: &str, line: usize) -> Columns` returns, for a cursor on the given zero-based line:

- `legal: Vec<usize>` — every column a new line may start at, ascending, deduplicated.
- `default: usize` — the column Enter should land on.
- `unit: usize` — the detected indentation unit.

It is a library function with no LSP in it, and it is where the formatter will get its line-structure questions answered later; `hrs fmt` and `hrs-lsp` must agree on what a chain looks like, and one function is how they agree.

The rule for `default`, in order, first match wins:

| Previous logical line | `default` |
|---|---|
| a whitespace-only line, whatever its column | that column — blank lines mean nothing to the transpiler, but to the editor they are where overrides live: the cursor was put there by the server or moved by Tab, Shift-Tab or spaces, and Enter repeats it |
| none (top of file) | 0 |
| the `)` tail of a paren-block | the header's chain arrow, or one unit past its last physical line |
| opens a block (`:` or `=>` last) | column of the construct that opened it — the first token on its **last physical line**, or the token after the innermost open bracket (`<- map (|x|:` indents from `|x|`) — plus one unit |
| ends in `(` or `[` | the group's **anchor** plus one unit: the start of the callee path before the bracket (`map (`, `foo.bar (`, `m! [`), else the bracket itself |
| ends in `=` | its indent plus one unit |
| inside an open `(` or `[`, several physical lines | the column of its last physical line: the next element is a peer |
| its last physical line is a lone `)` or `]` | the statement is complete: under its top-level `<-` if it is a chain link, else its indent |
| several physical lines otherwise (a continuation in progress) | under the last physical line's top-level `<-`, else one unit past that line |
| anything else | its indent |

`Columns` also carries `closer`: where a `)` or `]` typed as the first character of the line belongs — under the anchor of the innermost open `(` or `[`, whether that bracket is open on the previous logical line or on the header of an enclosing paren-block. `{` gets no answer; braces are Rust's. A closer that ends an expression (`x * 10)`, the compact form) is not touched. On Enter the server also looks one line down: an editor that auto-closes brackets splits `(` and `)` onto three lines on Enter, so the closer arrives without ever being typed; when the line below the cursor is nothing but `)` or `]`, it is placed under its anchor in the same response.

The rule for `legal`: the statement columns of every open block from innermost outward, the default, and — when the previous line neither opens a block nor ends in `=` — the continuation column (indent plus one unit) and the column of the first top-level `<-` on the previous line if it has one. Sorted; Tab moves to the next greater, and past the greatest advances one unit at a time without limit — layout puts no bound on the right, since a deeper line is always a continuation. Shift-Tab comes back one unit at a time and stops at column 0. Wrapping was tried first and read as a toggle; a right limit was tried next and read as Tab stopping.

The second row is the style rule from "Layout style" — a block indents past the construct that opened it, and when the style is followed that construct starts its physical line. For a mid-line opener (`let d = match n:` on one line) it gives the header's indent plus one unit, which is the layout rule; the style's answer would be the column of `match`, but the guide's own counter-examples show that form being *avoided* rather than indented that way, so the server does not reward it. This is the one place the table chooses; it is recorded here so it can be changed in one row.

## Protocol

- `textDocument/onTypeFormatting`, trigger characters `\n`, `)` and `]`. The editor sends the document *after* the newline and any indentation its own rules inserted; the server computes `columns(src, line).default` and returns one `TextEdit` replacing the new line's leading whitespace. Returning an edit rather than a position keeps the editor's undo stack whole: Enter and the re-indent are one operation.
- `harsh/columns` (custom request), params `{ textDocument, position }`, result `{ legal, default, unit, current }` where `current` is the line's present indent. The VSCode client binds Tab and Shift-Tab to it when the cursor is inside the leading whitespace; the client applies the edit. It also binds Enter at the end of a line to it: asking for the column *before* inserting the newline and writing both in one edit makes the cursor land once, where the on-type route — VSCode's guess first, the server's correction a few milliseconds later — showed a double jump wherever the two disagreed, which is after every closer. Mid-line Enter stays VSCode's, so bracket splitting runs, and on-type formatting places those lines. This request exists because Tab does not pass through `type` in VSCode, so on-type formatting never sees it; a custom request is the honest shape rather than abusing a trigger character.
- `textDocument/didOpen`, `didChange` (full sync), `didClose` — the server keeps the current text of each open file and nothing else.
- `textDocument/hover` and `textDocument/definition`, through rust-analyzer (above); `didSave` re-transpiles.
- `textDocument/completion`, on the text being typed (above).
- No diagnostics yet. Capabilities advertise only what exists.

Transport is stdio via the `lsp-server` crate, the one rust-analyzer uses; `lsp-types` for the wire types. Both are pinned to versions that build on rustc 1.75 so the container and the Mac see the same server.

## Launching it

- **VSCode**: `editors/vscode-harsh` gains a `client/extension.js` using `vscode-languageclient`, which spawns `hrs-lsp` from `PATH` (configurable as `harsh.serverPath`) and registers the two keybindings. The extension stops being a copy-the-folder install — it needs `npm install` before `vsce package` — but `cargo install --path .` already puts `hrs-lsp` next to `hrs`, so the user-facing install is unchanged: install the crate, install the extension.
- **Zed**: `editors/zed-harsh` gains the Rust WASM shim Zed requires for any language server (`src/lib.rs` implementing `zed_extension_api::Extension` and returning the `hrs-lsp` command), and `extension.toml` declares the server. Zed supports on-type formatting, so Enter works there; Tab cycling does not — Zed has no way to bind a key to a language-server request — and `indents.scm` keeps doing what it does today.

## What is deliberately not here

- **Reindenting on `else`.** VSCode's `decreaseIndentPattern` dedents it one level, which is right for every `else` that follows an `if` at the statement's own column; the cases it gets wrong are the same mid-line-opener forms the guide avoids.
- **Anything below the cursor.** Reindenting the rest of a block when its header moves is the formatter's job, not on-type's.
- **The tree-sitter grammar.** It is loose where the transpiler is strict and knows nothing of columns; Zed's indent queries stay as the fallback when the server is not installed.
