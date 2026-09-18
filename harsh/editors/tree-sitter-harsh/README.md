# tree-sitter-harsh

Tree-sitter grammar for [Harsh](../..) — Rust with indentation instead of braces.

## What it parses

Block structure comes from an external scanner (`src/scanner.c`) emitting `INDENT`, `DEDENT` and `NEWLINE`, the same approach `tree-sitter-python` uses, with two rules that follow from Harsh's own:

- Layout is suppressed inside `(`, `[` and `{`, so bracketed regions span lines freely.
- A colon or fat arrow ending a line opens a block; the scanner remembers the column in `pending` because emitting `NEWLINE` consumes the line break that introduced it.
- Parens are transparent to layout: `(|x|:` opens a `paren_block` whose body is indented, closed by the matching `)` wherever it sits. When the `)` ends the body's last line, the scanner emits the `DEDENT` on seeing `)` with `DEDENT` valid — the parser's own valid-symbol mask says whether a `)` closes a paren-block or an ordinary group, so the scanner tracks no parens at all.
- Newlines are in `extras`. The scanner is consulted first and claims them wherever `NEWLINE`, `INDENT` or `DEDENT` is valid; inside a bracket group it declines, and they become whitespace, which is what lets a group span lines.

Expressions, types and patterns are deliberately loose. This grammar exists for highlighting and outline, not type checking, so it accepts more than the transpiler does rather than producing `ERROR` nodes on valid code — in Zed a parse error removes highlighting entirely rather than degrading it.

## Verified

All 46 `.hrs` files in `examples/` and `examples/guide/` — `brackets.hrs` with the bracket-group shapes included — parse with **zero** `ERROR` or `MISSING` nodes, checked with the committed `src/parser.c` (a loop that also verifies each file actually produced a tree, since a missing CLI greps as zero errors). `tree-sitter.json` is the grammar's configuration for tree-sitter CLI 0.24+, which no longer reads the `tree-sitter` field in `package.json`.

```
tree-sitter generate
for f in ../../examples/*.hrs; do tree-sitter parse "$f" | grep -c ERROR; done
```

## Queries

- `queries/highlights.scm` — distinguishes the block-opening `:` (`punctuation.special`) from a type annotation (`punctuation.delimiter`), and covers `<-`, `<|`, `|>` and the `$` defer marker.
- `queries/indents.scm`, `queries/folds.scm`

## Known limits

- The grammar does not distinguish expression from type position, so `identifier` is used for both. Highlighting falls back to a capitalisation heuristic for types.
- Macro bodies are parsed as ordinary token groups.
