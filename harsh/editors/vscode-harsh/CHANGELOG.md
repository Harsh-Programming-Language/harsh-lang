# Changelog

## 0.1.4

- The search log has its own output channel, **"Harsh: server search"**. In 0.1.3 it was called "Harsh", which is the name the language client gives its own channel, so the log was hidden behind it. The channel now opens itself when the extension activates, and every line is mirrored to the developer console.

## 0.1.3

- The search for `hrs-lsp` now also asks a **login shell** (`$SHELL -lc 'command -v hrs-lsp'`), which is what finally fixes an editor started from the Dock or Finder: the login shell has the PATH you actually use, and the extension host does not.
- The **Harsh output channel** records the search: every directory tried, why each was rejected, and where the server was found. A failure to start is now diagnosable instead of mysterious.

## 0.1.2

- The extension finds `hrs-lsp` itself: `harsh.serverPath` if set, then your PATH, then where `cargo install` puts it (`$CARGO_HOME/bin`, `~/.cargo/bin`), then `/usr/local/bin` and `/opt/homebrew/bin`. An editor started from the Dock or Finder does not inherit a login shell's PATH, so a perfectly good install could fail with `spawn hrs-lsp ENOENT`; now it does not.
- When the server still cannot start, the message says what is lost (Enter, Tab, format on save — highlighting keeps working) and offers two buttons: how to install, or open the `harsh.serverPath` setting.

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
