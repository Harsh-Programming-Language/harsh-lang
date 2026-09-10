# Installing and updating Harsh on your machine

Everything you run after unpacking a new bundle, in order. Each step is
idempotent: running it again over an older install just replaces it.

## 1. The four binaries

From the published crate ([`harsh-lang`](https://crates.io/crates/harsh-lang)):

```
cargo install harsh-lang
```

or from a checkout, to run what is in the tree rather than the last release:

```
cd harsh
cargo install --path .
```

Either puts `hrs`, `hrs-from`, `hrs-remap` and `hrs-lsp` in `~/.cargo/bin` (on your
`PATH` if Rust is). This is the step the editors depend on: both extensions
launch `hrs-lsp` from `PATH`. Run it again after every bundle — the server's
behaviour (Enter, Tab, `)`, format on save) lives in the binary, not in the
extensions.

Check: `hrs --version`. The version carries the bundle's date as build metadata (`0.1.0+2026.09.07`), so a fresh install is told from the one before it. (The `+…` is dropped for a published release.) The next `hrs build` in any project retranspiles every file, since the new binary is newer than the Rust the old one wrote.

## 2. Verify the tree

```
cargo test
./check.sh
```

Both must be green before anything else is trusted. `cargo test` covers the
transpiler, the converter, the formatter over the whole corpus, and the
language server over stdio; `check.sh` builds the examples with rustc, round
trips them through `hrs-from`, and self-hosts.

## 3. The VSCode extension

From the Marketplace ([`harsh-lang.harsh-lang`](https://marketplace.visualstudio.com/items?itemName=harsh-lang.harsh-lang)):

```
code --install-extension harsh-lang.harsh-lang
```

or, when `editors/vscode-harsh` has changed and you want that version:

```
cd editors/vscode-harsh
npm install                       # first time only
npx @vscode/vsce package          # produces harsh-lang-0.1.1.vsix
code --install-extension harsh-lang-0.1.1.vsix
```

Then reload the window. Format on save is VSCode's own setting
(`editor.formatOnSave`); the extension supplies the formatter through
`hrs-lsp`. If the binary is not on VSCode's `PATH`, set `harsh.serverPath`.

## 4. The Zed extension

Until it is published: **zed: install dev extension** from the command
palette, pointing at `editors/zed-harsh`. First time: `rustup target add
wasm32-wasip1`. Zed launches `hrs-lsp` from `PATH`; format on save is Zed's
`format_on_save` setting.

One thing a dev install does not do: read the grammar from a folder. Zed
clones the repository named in `extension.toml` at the `rev` given, and that
file ships with a placeholder. Before the first install, make the grammar a
repository and point the extension at it:

```sh
cd editors/tree-sitter-harsh && git init -q && git add -A && git commit -qm grammar
git rev-parse HEAD        # the rev
```

then in `editors/zed-harsh/extension.toml` set `repository` to
`file:///<absolute path>/editors/tree-sitter-harsh` and `rev` to that
commit. (Not yet verified on a Mac; the alternative is to push the grammar to
any GitHub namespace and name that.)

## 5. Day to day

```
hrs new myproject            # a project laid out for Harsh
hrs check | hrs build | hrs run | hrs test | hrs lint
hrs watch [check]            # rebuild on every save
hrs fmt --check              # what the formatter would change (CI-friendly, exit 1)
hrs fmt                      # reformat src/**.hrs in place
hrs fmt file.hrs             # one file
hrs-from file.rs -o file.hrs # Rust in
hrs export                   # the project as a plain Rust crate, cargo fmt'd
hrs cargo leptos build       # transpile, then any cargo subcommand (Leptos, doc, ...)
```

`hrs <file.hrs> -o out.rs` transpiles one file; `--map out.map.json` writes
the source map that `hrs-remap` and `hrs check` use to put rustc's errors on
your `.hrs` lines.
