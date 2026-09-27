# Releasing Harsh — every product, in order

One file for the whole release: what to check, what to publish and where, in
the order that works, then how anyone installs each product. It is kept at
the root of the project and updated with every delivery; the version numbers
below are this release's.

**Nothing is published until you decide to.** A version is spent the moment
it is uploaded anywhere: a fix afterwards gets the next number.

**Verified for this delivery:** 277 tests, 0 ignored, no warnings;
`check.sh`, with the self-host and the website's four gates; the public
tree's own 277 tests; the Jupyter kernel's 6 tests. Under 0.1.31, whose
transpiler 0.1.35 keeps unchanged: the guide's 99 blocks, the Book's 248
snippets and *By Example*'s 62 built and run, Harshlings' 65 exercises with
0 problems.

## Where the files are

In the delivery bundle, this file sits in `_Installation & Deployment/` with
`TESTING.md` (what is new, and how to try it) and `DEPLOY-WEBSITE.md` (a copy
of `site/DEPLOY.md`). Beside that folder: `harsh-public-0.1.35.tar.gz` (the public tree: the
crate, the kernel's sources in `kernel/`, the website in `site/`),
`harshlings-0.1.6.tar.gz`, `editors/` (the VS Code `.vsix`, the Zed
repositories), `jupyter/` (the kernel's wheel). Everything else is in the
public tree: what is new in `CHANGELOG.md`; the website's full guide in
`site/DEPLOY.md`; the Jupyter kernel's in `docs/JUPYTER.md`.

## The products

| Product | Version | Where it is published | From |
|---|---|---|---|
| `harsh-lang` -- `hrs`, `hrs-from`, `hrs-remap`, `hrs-lsp` | **0.1.35** | crates.io | this repository |
| Harsh's standard distribution -- `hrs_std` 0.1.2, `hrs_proc_macro` 0.2.0, `hrs_quote` 0.1.0, `hrs_syn` 0.1.0 | inside `hrs` | nowhere: shipped inside `harsh-lang` | this repository |
| The Jupyter kernel, `harsh-kernel` | **0.1.3** | PyPI | `kernel/` |
| The VS Code extension, `harsh-lang.harsh-lang` | **0.1.5** | the VS Code Marketplace (Open VSX: not yet) | `editors/vscode-harsh/`; the `.vsix` in the bundle |
| Harshlings, the exercises | **0.1.6** | its own GitLab repository | the bundle's `harshlings-0.1.6.tar.gz` |
| The website, **https://harsh-lang.com/** | -- | GitHub Pages, from the GitHub mirror | `site/` |
| The Zed extension and its tree-sitter grammar | 0.1.0 | not yet: the Zed registry, when ready | `editors/zed-harsh/`, `editors/tree-sitter-harsh/` |

The home repository is GitLab,
`https://gitlab.com/bahiminin.benoit.dah.opensource/harsh-lang`; GitHub,
`https://github.com/Harsh-Programming-Language/harsh-lang`, is its mirror,
fed by GitLab, and the only place the website is built.

## Part 1 — Publishing

### 0. Once: accounts and tokens

Turn on two-factor authentication everywhere first. Tokens stay in each
tool's own login, never in the repository.

| Account | Token | Used by |
|---|---|---|
| GitLab | your usual login | `git push` |
| GitHub | the mirror's **fine-grained** token, scoped to `harsh-lang`: *Contents: read and write* **and** *Workflows: read and write* | GitLab's push mirror (GitLab: Settings → Repository → Mirroring repositories) |
| crates.io | an API token: `cargo login` | step 3 |
| PyPI | an API token (the first upload needs scope *Entire account*; afterwards, one scoped to `harsh-kernel`) | step 4 |
| Azure DevOps (Marketplace) | a Personal Access Token, scope *Marketplace: Manage*, for publisher `harsh-lang`: `npx @vscode/vsce login harsh-lang` | step 5 |
| Eclipse (Open VSX) | an access token | step 6, later |

On GitHub, once: repository → Settings → Pages → *Build and deployment* →
Source: **GitHub Actions** (step 8).

### 1. Check on your Mac

From a fresh copy of the public tree:

```sh
mkdir -p ~/harsh-release && cd ~/harsh-release
tar -xzf /path/to/bundle/harsh-public-0.1.35.tar.gz        # makes harsh/
cd harsh
cargo test                                          # 277 passed, 0 ignored
./check.sh                                          # the examples, the self-host, the site's gates (the editor's needs node)
```

**Miri over `hrs_std`**, which ships inside `harsh-lang`:

```sh
cd hrs_std
cargo test                                                   # 1. 25 passed
MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test       # 2. 25 passed
cargo +nightly miri test -- --skip slicing --skip broadcast  # 3. 15 passed
cargo +nightly miri test                                     # 4. must stop at src/slicing.rs, View::parts
```

Runs 1-3 pass; run 4 fails, only at the view -- the known issue (undefined
behaviour under Stacked Borrows, accepted by Tree Borrows). Anywhere else,
stop: publishing waits.

**`hrs_std` against nalgebra 0.35**, the version the Rust Playground has (the
website's Playground sends `hrs_std` there):

```sh
sed -i '' 's/^nalgebra = "0.33"/nalgebra = "0.35"/' Cargo.toml
cargo test                                          # 25 passed, as with 0.33
sed -i '' 's/^nalgebra = "0.35"/nalgebra = "0.33"/' Cargo.toml
cd ..
```

If it fails, the Playground's matrices wait for a fix; nothing else does.

### 1b. Try it on your Mac, before publishing anything

From the same `harsh/` folder, your own build first:

```sh
cargo install --path . --force                      # hrs and its tools, from this tree
hrs --version                                       # 0.1.35
```

**The website, locally:**

```sh
cargo install dioxus-cli@0.7 --locked               # once (or: cargo binstall dioxus-cli@0.7)
rustup target add wasm32-unknown-unknown            # once
cd site && hrs build && dx serve                    # hrs build transpiles, then lets cargo build
```

Open **http://127.0.0.1:8080/**. Then:

- **Learn** → the Book, *By Example*, the guide (served from `site/public/`);
- **Converter**: type Harsh, read the Rust; **Format**; the sample chips;
  then **Rust → Harsh** (the Rust on the right becomes the input, its Harsh on
  the right), and back with **Harsh → Rust**;
- **Playground**: Run the word count (`the: 3`, `and: 2`, `cat: 1`); then
  `let vector = v~ [1,3,4]` / `println! "{}" vector` -- the matrix check,
  against the Rust Playground's nalgebra;
- **the editor**, in both pages: Enter after `fn main$:`, after `let x =`,
  after a continued line; Tab and Shift-Tab through the legal columns; `(`,
  `[`, `{`, `"` closing themselves, a closer stepped over, Backspace in
  `()`, Enter between brackets; Tab never leaves the editor (Escape, then
  Tab, does); Ctrl-Z undoes each.

**The Jupyter kernel, from the bundle's wheel.** `python -m …` for every
command, so pip, the kernel and Jupyter are the same Python (on a Mac,
`python3` can be Xcode's while `pip` is conda's); `--no-deps`, so the
reinstall touches only Harsh's kernel, not Jupyter's own packages; and from
outside the repository (`cd ~`), so Python finds the installed kernel, not
the tree's `kernel/` folder:

```sh
cd ~
python -m pip install --force-reinstall --no-deps /path/to/bundle/jupyter/harsh_kernel-0.1.3-py3-none-any.whl
python -m harsh_kernel.install                      # needs evcxr_jupyter, installed once
python -m jupyter kernelspec list                   # lists harsh (and rust, from evcxr)
python -m jupyter lab                               # the "Harsh" kernel: a cell with a type error
                                                    # reports it at the cell's own line
```

Matrices work in the notebook: a cell with `let v = v~ [1, 3, 4]` then `v`
prints the vector. The first such cell says `hrs_std` was added, and takes a
minute or two while evcxr compiles nalgebra; later ones are quick.

**The VS Code extension:**
`code --install-extension /path/to/bundle/editors/harsh-lang-0.1.5.vsix`.

What is new, with commands to try each: `TESTING.md`, next to this file.

### 2. GitLab — the home repository (and, through it, GitHub)

The mirror's token must have *Workflows* (step 0) **before** this push: it
carries `.github/workflows/pages.yml`, which GitHub refuses otherwise.

```sh
cd harsh-lang                                       # your clone of the GitLab home
tar -xzf /path/to/bundle/harsh-public-0.1.35.tar.gz --strip-components=1
test ! -d harsh && grep '^version' Cargo.toml       # version = "0.1.35", and no harsh/ folder
git status
git add -A
git commit -m "0.1.35: the Converter both ways (driver::convert_str); the new home page"
git tag -a v0.1.35 -m "0.1.35"
git push origin main v0.1.35                        # if it times out: push main, then the tag
```

Then check GitLab's pipeline is green, and the GitHub mirror has the commit
(GitLab: Settings → Repository → Mirroring repositories shows the last sync
and any error).

### 3. crates.io — `harsh-lang`, and only it

From the same clone:

```sh
cargo publish --dry-run
cargo publish
```

The four crates of the standard distribution are **not** published: they
travel inside `harsh-lang`, and `hrs` serves them.

### 4. PyPI — the Jupyter kernel, `harsh-kernel`

After step 3 (the kernel runs `hrs`, which users install from crates.io):

```sh
cd kernel
python -m pip install --upgrade build twine
rm -rf dist *.egg-info
python -m build                                     # dist/harsh_kernel-0.1.3-py3-none-any.whl and .tar.gz
python -m twine upload dist/*                       # paste the PyPI token at the prompt
rm -rf dist *.egg-info build
cd ..
```

A 403 *"not allowed to upload"* means the name `harsh-kernel` is taken on
PyPI: stop, and choose another name. Warnings about PATH, OpenSSL or
*trusted publishing* are harmless.

### 5. The VS Code Marketplace — the extension

First, what the Marketplace has:

```sh
npx @vscode/vsce show harsh-lang.harsh-lang | grep -i version
```

If it shows **0.1.5**, nothing to do. If it shows an older version:

```sh
npx @vscode/vsce publish --packagePath /path/to/bundle/editors/harsh-lang-0.1.5.vsix
```

### 6. Open VSX — not yet published

For VSCodium, Cursor and other editors built on VS Code. When you decide,
once: `npx ovsx create-namespace harsh-lang -p <token>`; then each version:

```sh
npx ovsx publish /path/to/bundle/editors/harsh-lang-0.1.5.vsix -p <token>
```

### 7. Harshlings

After step 3 (Harshlings' CI installs `harsh-lang` from crates.io). If its
GitLab repository already shows 0.1.6, nothing to do. Otherwise:

```sh
cd harshlings                                       # your clone
git rm -r -q exercises solutions                    # a tarball adds, it never deletes
tar -xzf /path/to/bundle/harshlings-0.1.6.tar.gz --strip-components=1
test ! -d harshlings
python3 verify.py                                   # 65 exercises, 0 problems
git add -A
git commit -m "0.1.6: macro exercises by who wrote them; matrix exercises built through hrs"
git push origin main
```

### 8. The website

Nothing to run: GitHub builds and deploys it from the push of step 2, once
Pages' source is *GitHub Actions* (step 0). Then GitHub → Actions →
*Publish the site* is green, and the site is at
**https://harsh-lang.com/**.

Everything else -- committing `site/Cargo.lock`, the checks after a deploy,
what to do when a step fails, and how the domain is set up -- is in
**`site/DEPLOY.md`** in the public tree.

### 9. Zed — not yet published

The extension and its grammar go to the Zed registry as their own GitHub
repositories, when the two known Enter issues are settled. When you decide:

1. From the bundle's `editors/harsh-zed-repos.tar.gz`: push
   `tree-sitter-harsh` to GitHub, note its commit SHA, set it as `rev` in
   `zed-harsh/extension.toml`, push `zed-harsh`.
2. In Zed, *zed: install dev extension* on `zed-harsh` (after
   `rustup target add wasm32-wasip1`) -- its first real build.
3. Fork `zed-industries/extensions`; `git submodule add
   https://github.com/Harsh-Programming-Language/zed-harsh.git
   extensions/harsh`; append `extensions-entry.toml` to `extensions.toml`;
   `pnpm sort-extensions`; open the pull request.

### 10. Check it all landed

- `cargo search harsh-lang` shows 0.1.35
- GitLab: the `v0.1.35` tag, a green pipeline; GitHub: the same commit
- GitHub → Actions: *Publish the site* green; the site passes the checks of step 8
- `pip index versions harsh-kernel` shows 0.1.3
- the Marketplace shows the extension at 0.1.5
- Harshlings' pipeline is green against the `harsh-lang` just published

### What comes next

Not in this release, and waiting on your ruling or your testing (the
roadmap has each): matrix views as values; hover and go-to-definition
through rust-analyzer; the website's static generation for search
engines; your changes to the site.

## Part 2 — Installing

What anyone does, once the products are published.

**Harsh itself** (needs Rust, from `https://rustup.rs`):

```sh
cargo install harsh-lang          # hrs, hrs-from, hrs-remap, hrs-lsp
hrs --version                     # 0.1.35
hrs new hello && cd hello && hrs run
```

**VS Code:** search *Harsh* in the Extensions view, or
`code --install-extension harsh-lang.harsh-lang` (from a file:
`code --install-extension editors/harsh-lang-0.1.5.vsix` from the bundle). It finds `hrs-lsp` on the
PATH for formatting, indentation and diagnostics.

**Jupyter:**

```sh
cargo install evcxr_jupyter && evcxr_jupyter --install   # Rust's kernel, which runs the code
cd ~
python -m pip install harsh-kernel                       # or --no-deps with the bundle's wheel, as in step 1b
python -m harsh_kernel.install                           # registers "Harsh"
```

Then pick the **Harsh** kernel. More in `docs/JUPYTER.md`.

**Harshlings:**

```sh
git clone https://gitlab.com/bahiminin.benoit.dah.opensource/harshlings
cd harshlings && hrs run
```

**The website:** nothing to install -- **https://harsh-lang.com/**. To run it
locally: `cargo install dioxus-cli@0.7 --locked`, `rustup target add
wasm32-unknown-unknown`, then in `site/`: `hrs build`, `dx serve`, and open
`http://127.0.0.1:8080/`.

**Zed** (until it is in the registry): *zed: install dev extension* on
`zed-harsh` from the bundle's `editors/harsh-zed-repos.tar.gz`.
