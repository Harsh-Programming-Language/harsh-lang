# Changelog

## 0.1.1

- The extension icon is the Harsh mark (the `h` between `<` and `>`, with the dot); the listing text and the repository link are the published ones.

## 0.1.0

First public release.

- `hrs`: the transpiler and build driver — `new`, `build`, `run`, `test`, `check`, `lint`, `watch`, and single-file mode with a source map.
- `hrs-from`: Rust to Harsh; the transpiler's own source round-trips through it byte-exact.
- `hrs-remap`: cargo diagnostics mapped back to `.hrs` lines and columns, including zero-width suggestion spans.
- `hrs-lsp`: a language server with layout-aware on-type formatting and a `harsh/columns` request; no names, no types, no arity.
- Editors: VSCode extension (highlighting, folding, Enter and Tab through `hrs-lsp`), tree-sitter grammar, Zed extension.
- Documentation: the language guide with every snippet transpiled, compiled and run; the specification; the start of the Harsh Book.
