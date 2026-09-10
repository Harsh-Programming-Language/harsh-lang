# Harsh for VSCode

Syntax highlighting and layout-aware editing for `.hrs` files — Rust with indentation instead of braces.

## How it works

The grammar includes `source.rust` wholesale and adds only what Harsh changes:

| Pattern | Scope |
|---|---|
| `<-` | `keyword.operator.member.harsh` |
| `<\|`, `\|>` | `keyword.operator.pipe.harsh` |
| `do:` | `keyword.control.block.harsh` |
| `[where …]` | bracketed clause, contents highlighted as Rust |

Everything else — keywords, strings, types, lifetimes, comments, macros — comes from the Rust grammar unchanged, because none of it differs.

`language-configuration.json` is Harsh-specific, since Rust's is brace-based:

- `folding.offSide: true` — folding follows indentation
- no `indentationRules` and no `onEnterRules`, on purpose. The server owns every column (below); pattern rules are a second, cruder reading of the same layout and can only disagree with it — VSCode applied them on *Move Line Up/Down* too, and re-indented the moved line wrongly. For the same reason the extension sets `editor.autoIndent: "keep"` for Harsh: a moved line keeps its indentation, and Enter keeps the previous line's until the server's edit lands.

## The language server

`client/extension.js` launches `hrs-lsp` (from `PATH`, or `harsh.serverPath`) and the server does the part no `onEnterRules` can:

- **Enter** at the end of a line — the extension asks the server (`harsh/columns`) where the next line starts and inserts newline and indentation as one edit, so the cursor lands once. After `println!` — a macro with its arguments still to come — it lands on the first argument's column, and after that argument on its sibling's. Enter mid-line, including between an auto-closed pair, is VSCode's own; there the server's on-type formatting on `\n` (the extension turns on `editor.formatOnType` for Harsh) places the new line and the closer. After a chain line the new line lands under the previous `<-`; after a line ending in `=`, one level in; after a block opener, one level in from the opener's line.
- **Tab / Shift-Tab** — when the cursor is in a line's leading whitespace, the extension asks the server (`harsh/columns`) for the legal columns and moves to the next or previous one. Past the last legal column Tab keeps going a unit at a time — there is no right boundary, since a more-indented line always continues — and Shift-Tab comes back to the previous legal column when one is within a unit (the earlier arrows of a chain line), else a unit at a time, to column 0. Anywhere else, VSCode's own Tab runs.

Without `hrs-lsp` installed the extension still loads; you get highlighting and folding, plus one warning — no layout help, since there are no pattern rules to fall back on.

## Install

```
npm install
npx @vscode/vsce package
code --install-extension harsh-0.1.0.vsix
```

The dependency on `vscode-languageclient` is what makes the `npm install` step necessary.

## Checked

The four operator patterns match what they should and reject what they shouldn't — in particular `x < -1` is not a member access, `$a:expr` inside a macro body is not a defer marker, and `done` is not `do`.
