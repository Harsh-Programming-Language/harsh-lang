# Harsh

**Rust without the braces.** Harsh is Rust spelled with indentation for structure, `f a b` for applying a function and `<-` for reaching into a value; everything else — ownership, traits, lifetimes, every crate — is Rust, unchanged, because `hrs` is a layout transformation and not a new compiler. This extension makes `.hrs` files first-class in VSCode.

```
use std.collections.HashMap

fn word_counts text: &str -> Vec<(String, usize)>:
    let mut counts = HashMap.new$
    for word in text <- split_whitespace$:
        *counts <- entry (word <- to_lowercase$) <- or_insert 0 += 1
    let mut v: Vec<(String, usize)> = counts <- into_iter$ <- collect$
    v <- sort_by (|a, b| b.1 <- cmp (&a.1))
    v
```

## What you get

- **Highlighting** that is Rust's own, plus the four things Harsh adds — `<-`, the pipes `|>` and `<|`, `do:`, and `[where …]`.
- **Enter that knows the layout.** After a chain line the new line lands under the previous `<-`; after `let x =`, one level in; after `println!`, on the first argument's column, then on each argument's sibling. One keystroke, one place.
- **Tab and Shift-Tab between legal columns** — the columns the layout would accept on that line, and only those.
- **Format on save**, with `hrs fmt`'s rules: chains of three or more links vertical with the arrows aligned, a block literal's fields one unit past its name, arguments beneath a callee that no longer fits its line. The formatter changes no token, ever.
- **Errors on your lines.** Build with `hrs check` or `hrs build` and rustc's diagnostics point at the `.hrs` line and column, not at the generated Rust.
- **Folding** by indentation.

## Setup

The editing features come from a language server, `hrs-lsp`, which is part of the Harsh toolchain:

```
cargo install harsh-lang
```

That installs `hrs` (transpiler and build driver), `hrs-from` (Rust to Harsh), `hrs-remap` and `hrs-lsp`. The extension finds `hrs-lsp` on your `PATH`, or where `harsh.serverPath` points. Without it you still get highlighting and folding, with one warning.

Then: `hrs new hello && cd hello && hrs run`.

## Learn Harsh

- *The Harsh Language Guide* — every construct beside its Rust spelling, for Rust programmers: the fastest way in.
- *The Harsh Programming Language* — Rust taught in Harsh, from the first program to async, for readers who do not know Rust.

Both are in the repository, every code sample built and run: https://gitlab.com/bahiminin.benoit.dah.opensource/harsh-lang

---

## For contributors

## How it works

The grammar includes `source.rust` wholesale and adds only what Harsh changes:

- `<-` — `keyword.operator.member.harsh`
- `<|` and `|>` — `keyword.operator.pipe.harsh`
- `do:` — `keyword.control.block.harsh`
- `[where …]` — a bracketed clause, its contents highlighted as Rust

Everything else — keywords, strings, types, lifetimes, comments, macros — comes from the Rust grammar unchanged, because none of it differs.

`language-configuration.json` is Harsh-specific, since Rust's is brace-based:

- `folding.offSide: true` — folding follows indentation
- no `indentationRules` and no `onEnterRules`, on purpose. The server owns every column (below); pattern rules are a second, cruder reading of the same layout and can only disagree with it — VSCode applied them on *Move Line Up/Down* too, and re-indented the moved line wrongly. For the same reason the extension sets `editor.autoIndent: "keep"` for Harsh: a moved line keeps its indentation, and Enter keeps the previous line's until the server's edit lands.

## The language server

`client/extension.js` launches `hrs-lsp` (from `PATH`, or `harsh.serverPath`) and the server does the part no `onEnterRules` can:

- **Enter** at the end of a line — the extension asks the server (`harsh/columns`) where the next line starts and inserts newline and indentation as one edit, so the cursor lands once. After `println!` — a macro with its arguments still to come — it lands on the first argument's column, and after that argument on its sibling's. Enter mid-line, including between an auto-closed pair, is VSCode's own; there the server's on-type formatting on `\n` (the extension turns on `editor.formatOnType` for Harsh) places the new line and the closer. After a chain line the new line lands under the previous `<-`; after a line ending in `=`, one level in; after a block opener, one level in from the opener's line.
- **Tab / Shift-Tab** — when the cursor is in a line's leading whitespace, the extension asks the server (`harsh/columns`) for the legal columns and moves to the next or previous one. Past the last legal column Tab keeps going a unit at a time — there is no right boundary, since a more-indented line always continues — and Shift-Tab comes back to the previous legal column when one is within a unit (the earlier arrows of a chain line), else a unit at a time, to column 0. Anywhere else, VSCode's own Tab runs.

Without `hrs-lsp` installed the extension still loads; you get highlighting and folding, plus one warning — no layout help, since there are no pattern rules to fall back on.

## Building from source

```
npm install
npx @vscode/vsce package
code --install-extension harsh-lang-0.1.1.vsix
```

The dependency on `vscode-languageclient` is what makes the `npm install` step necessary.

## Checked

The four operator patterns match what they should and reject what they shouldn't — in particular `x < -1` is not a member access, `$a:expr` inside a macro body is not a defer marker, and `done` is not `do`.
