# 17. More about cargo

Cargo has done the building so far, and it does more: release builds, documentation, publishing, and workspaces for a project of several crates. This chapter is short and mostly about commands, because there is little to show that compiles — but one thing, documentation comments, is code in your files and worth seeing through the transpiler.

## 17.1 Build profiles

`cargo build` (and `hrs build`) uses the *dev* profile: fast to compile, unoptimised, with debug assertions and overflow checks on. `cargo build --release` uses the *release* profile: slow to compile, optimised, checks off, and this is the binary you ship or benchmark. The profiles are configurable in `Cargo.toml`:

```text
[profile.dev]
opt-level = 0

[profile.release]
opt-level = 3
```

`opt-level` runs from 0 to 3; those are the defaults, and you change them when you have a reason — a dev build of a program that is too slow to test unoptimised, for instance. Harsh adds nothing here: `hrs build --release` passes the flag through.

## 17.2 Documentation comments

Rust has a comment form that becomes documentation. `///` documents the item that follows; `//!` documents the enclosing item, and at the top of a file that is the crate or module itself:

`src/lib.hrs`

```rust harsh
//! # My Crate
//!
//! `my_crate` is a collection of utilities to make performing certain
//! calculations more convenient. This comment documents the crate itself.

/// Adds one to the number given.
///
/// # Examples
///
/// ```
/// let arg = 5
/// let answer = my_crate.add_one arg
///
/// assert_eq! 6 answer
/// ```
pub fn add_one x: i32 -> i32:
    x + 1
```

```text
$ hrs test
   Doc-tests my_crate

running 1 test
test src/lib.rs - add_one (line 10) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```

`cargo doc --open` renders every `///` and `//!` in the crate as HTML, with the Markdown inside them — headings, code blocks, links — laid out. The conventional sections are `# Examples`, `# Panics` (when the function can), `# Errors` (what `Err`s it returns) and `# Safety` (for `unsafe` functions). The example in the `///` is Harsh, like everything else in the file, and `cargo test` *runs it* — every code block in a doc comment is a test, so documentation cannot drift from the code without failing the build.

> **Harsh —** A fenced block in a doc comment is the one kind of comment the transpiler reads. The documentation tool lifts such a block out, compiles it and runs it — it is code, not prose — so it is written in Harsh and the transpiler writes out the Rust that tool expects, the same translation it does everywhere else. A fence tagged with another language — `text`, `toml` — is prose and is left exactly as you wrote it. A line beginning `#` inside the fence is the documentation tool's own mark for *compile this but do not show it*, and it survives the translation.

### Re-exports for a public API

Chapter 7 mentioned `pub use` for presenting a different structure to users than the code has. In a library it is what makes the docs usable:

```rust harsh
//! Art: a library for modelling artistic concepts.

pub use self.kinds.PrimaryColor
pub use self.kinds.SecondaryColor
pub use self.utils.mix

pub mod kinds
    /// The primary colors according to the RYB color model.
    #[derive Debug Clone Copy]
    pub enum PrimaryColor
        Red
        Yellow
        Blue

    /// The secondary colors according to the RYB color model.

    #[derive Debug]
    pub enum SecondaryColor
        Orange
        Green
        Purple

pub mod utils
    use crate.kinds.*

    /// Combines two primary colors in equal amounts to create a secondary color.
    pub fn mix (c1: PrimaryColor) (c2: PrimaryColor) -> SecondaryColor:
        match (c1, c2)\
            (PrimaryColor.Red, PrimaryColor.Yellow) | (PrimaryColor.Yellow, PrimaryColor.Red) => SecondaryColor.Orange
            (PrimaryColor.Yellow, PrimaryColor.Blue) | (PrimaryColor.Blue, PrimaryColor.Yellow) => SecondaryColor.Green
            _ => SecondaryColor.Purple

fn main$:
    // With the re-exports, a user writes `art::mix` and `art::PrimaryColor`,
    // never `art::utils::mix`. Inside the crate they are reachable both ways.
    let red = PrimaryColor.Red
    let yellow = PrimaryColor.Yellow
    println! "{:?}" (mix red yellow)
```

```text
Orange
```

Inside, `PrimaryColor` lives in `kinds` and `mix` in `utils`, which is a sensible arrangement for the authors. A user does not care, and without the re-exports would have to write `art.utils.mix` and `art.kinds.PrimaryColor` and know which was where. The three `pub use` lines put all of them at the top of the crate; `cargo doc` lists the re-exports on the front page, and the user's `use art.(mix, PrimaryColor)` is the whole of what they learn about the layout.

## 17.3 Publishing

`crates.io` is the registry `cargo` fetches dependencies from, and publishing to it is `cargo publish` after `cargo login` once with a token from the site. `Cargo.toml` needs `name` (unique on the registry), `version`, `description` and `license` before the registry will accept it; a publish is permanent, since other crates may depend on it — versions can be *yanked* (`cargo yank --vers 1.0.1`) to stop new projects picking them up, but never deleted. For a Harsh crate, what you publish is its `export/` folder: every `hrs build` writes the whole project there as a plain Rust crate — its `Cargo.toml`, `src/` transpiled, your other files beside them — and `hrs -c publish` runs `cargo publish` in it. The registry, and the people who depend on you, see Rust.

That is the way to share Harsh code *with Rust users*: they get Rust with no Harsh anywhere — the parts of `hrs_std` you use become the crate's own `hrs_std` module, its dependencies `nalgebra` and `num-traits` — and they never need `hrs`. The Rust keeps your file's lines, one for one, so it reads in the order your Harsh does. It is not the way to share it with Harsh users, because of what a Rust crate cannot carry: Harsh's own macros. A `~` macro unfolds in Harsh, before anything is transpiled, and leaves nothing in the Rust; so `export/` has no `macro_rules~` in it, and a Harsh program that depends on the published crate cannot call them. (Rust's `!` macros, which are Rust, are there as written.)

To share a library *as Harsh*, macros and all, share its Harsh: another project depends on it by path, as the next section shows, and `hrs` transpiles it first — its `~` macros, and those it re-exports with `pub use`, reach the program that uses it. A registry of Harsh's own is planned, so that a Harsh library can be published and added by name as a Rust crate is; until it exists, a path is how Harsh travels as Harsh.

**Adding dependencies.** A Harsh project has two manifests side by side. `Cargo.toml` is Cargo's, and lists Rust's crates: `cargo add serde` writes there, as in Rust. `Hrs.toml` is Harsh's, and lists Harsh's dependencies — `hrs_std`, and Harsh crates — which `hrs` handles before Cargo runs: it reads each one's Harsh macros and signatures, transpiles it, and builds the project from the `Cargo.toml` it writes in `export/`, yours plus them. `hrs add hrs_std` writes to `Hrs.toml`. A Harsh crate of your own, in a folder beside the project, is one line:

```toml
# Hrs.toml
[package]
cargo = "Cargo.toml"        # optional: the Cargo.toml beside this file is the default

[dependencies]
hrs_std = "0.1"
mylib = { path = "../mylib" }
```

Paths in `Hrs.toml` are relative to its own folder. `export/Cargo.toml` names each Harsh crate the project uses by that crate's own `export/` and its version: the path while you build, the version once both are on crates.io, so `hrs -c publish` in each, the library first, publishes them in order. Harsh files inside a Rust project are transpiled together — `hrs src/parser.hrs src/lexer.hrs` — each `.rs` beside its `.hrs`, headed with the name of its source; `hrs --check src/*.hrs` in CI fails when a committed `.rs` has fallen behind. Each command speaks to one registry, so a name is never looked up in the wrong one: `hrs add serde` answers that `serde` is a Rust crate, for `cargo add`. The crates for writing procedural macros — `hrs_proc_macro`, `hrs_quote`, `hrs_syn` — are compile-time tools, as `syn` and `quote` are Rust's, and stay in `Cargo.toml`. A project from before 0.3.0, with `hrs_std` in its `Cargo.toml`, is stopped with a message; `hrs migrate` moves it. In the rare project that wants a Harsh crate and a Rust crate of the same name, Cargo's rename keeps both: `foo_rs = { version = "1", package = "foo" }`, and the Rust one is `foo_rs` in your code.

**Three intents.** How a project is made, and where its code is published, give three cases — and the commands for each already exist:

| | Published to Harsh's registry | Published to crates.io |
| --- | --- | --- |
| **A Harsh project** (`hrs new`) | #1: a Harsh crate | #2: a Rust crate written in Harsh |
| **A Rust project** (`cargo new`) | — | #3: Harsh files in a Rust crate |

A program is made with `hrs new app`; a library — a crate others use, with no `main` — with `hrs new --lib geometry` (or `hrs new geometry --lib`), which writes `src/lib.hrs` with one public function and a test, so `hrs test` passes at once. #1 and #2 start the same way: which one a library becomes is chosen when it is published, not when it is made, and the same library can be published both ways.

*#1, a Harsh crate,* is shared as Harsh, with its `~` macros, for Harsh users:

```console
hrs new --lib geometry
hrs build
hrs publish
```

Harsh's registry is not open yet, so `hrs publish` checks the package as a registry would — `name`, `version`, `description` and `license` in `Cargo.toml`, and every Harsh dependency named by version rather than by path, since a published crate cannot point at a folder on your disk — and then stops, saying how to share today. Until it opens, another project lists the library in its own `Hrs.toml` by path, and uses it as any crate, `geometry.add 2 3`:

```toml
[dependencies]
geometry = { path = "../geometry" }
```

`hrs add` adds the crates of Harsh's distribution, like `hrs_std`; a crate on a path is written into `Hrs.toml` by hand.

*#2, a Rust crate written in Harsh,* is shared with Rust users, who never need `hrs`. The project is written in Harsh, and published as the Rust in its `export/`:

```console
hrs new --lib geometry
hrs build                       # export/: plain Rust, what crates.io will get
hrs -c publish --dry-run        # cargo packages export/ and builds the package
hrs -c publish
```

`hrs -c <command>` is `cd export && cargo <command>`, after `hrs` has written `export/` afresh: `hrs -c publish` is `cargo publish` on the Rust. It warns first when `description` or `license` is missing from `Cargo.toml`, which crates.io requires, and it refuses a Harsh macro crate, whose macros exist only in Harsh — that one goes to Harsh's registry, #1.

*#3, Harsh files in a Rust crate,* keeps a Rust project and writes some of its files in Harsh. Each `.hrs` file is transpiled to the `.rs` beside it, headed with the name of its source; files transpiled together see each other's functions; and `--check` lets CI fail when a `.rs` is stale:

```console
cargo new parser
hrs src/lexer.hrs src/grammar.hrs      # writes src/lexer.rs and src/grammar.rs
hrs --check src/lexer.hrs src/grammar.hrs   # in CI: writes nothing, fails if stale
cargo publish
```

The edge cases: a procedural-macro crate stays in `Cargo.toml` in every case, as a Rust dependency; in #2 the Harsh crates a project uses are published first, each by `hrs -c publish` in its own folder; and in #3 the `.rs` files are what Cargo builds, so they are committed with the `.hrs` they come from.

## 17.4 Workspaces

A *workspace* is a set of packages sharing one `Cargo.lock` and one `target/` directory, for a project that has grown to several crates — a binary and the libraries it is split into, say. The root `Cargo.toml` lists the members:

```text
[workspace]
members = ["adder", "add_one"]
```

and each member is an ordinary package in its own directory, depending on its siblings by path (`add_one = { path = "../add_one" }`). A member may be written in Harsh or in Rust. Run `hrs build` or `hrs run` in the member you want: `hrs` transpiles every Harsh member whose sources changed, and cargo then builds what that one needs.

A workspace has one `export/`, at its root, and it is the whole workspace as Rust: the root's `Cargo.toml`, each Harsh member transpiled at its place — `export/adder/src/main.rs` — and each Rust member as it is. Cargo builds them there into one `target/`, with one `Cargo.lock`, which `hrs` keeps at the root beside yours. A member has no `export/` of its own, and `hrs`'s own files are in the root's `.hrs/`:

```text
Cargo.toml          [workspace] members = ["adder", "add_one"]
adder/              Cargo.toml  Hrs.toml  src/main.hrs
add_one/            Cargo.toml  Hrs.toml  src/lib.hrs
export/             Cargo.toml  Cargo.lock  adder/  add_one/  target/
```

At the workspace's root, `hrs build`, `hrs check`, `hrs test`, `hrs lint` and `hrs fmt` go through the members in the order `members` lists them (a trailing `*` takes every package in that folder; `exclude` leaves one out). Each Harsh member is built by `hrs`, each Rust member by cargo, and `hrs fmt` skips the Rust ones. The first failure stops the run:

```console
$ hrs test
hrs: workspace member adder
...
hrs: workspace member add_one
...
```

`hrs run` acts on one program, so at the root it is refused, and the message names the members: run it in the one you want. `hrs -c …` at the root is cargo in the root's `export/`, for the whole workspace: `hrs -c build --release` builds every member, as `cargo build --release` does in a Rust workspace.

## 17.5 Installing binaries

`cargo install crate_name` fetches a crate from the registry, builds its binary and puts it in `~/.cargo/bin`, which is on your `PATH` once Rust is installed — it is how command-line tools written in Rust get distributed, and how `hrs` itself is installed from its repository (`cargo install --path .`). Any binary named `cargo-something` on the `PATH` also becomes a subcommand, `cargo something`, which is how cargo's own set of commands is extended.

## 17.6 What you have

`--release` for the optimised build; `///` and `//!` for documentation that `cargo doc` renders and `cargo test` runs; `pub use` to give a crate a public shape; `hrs -c publish` for the registry, from the Rust in `export/`; workspaces for several crates; `cargo install` for tools. None of it is Harsh's, and all of it works on the transpiled tree.

Next: smart pointers — `Box`, `Rc` and `RefCell`, which are how Rust does the data structures a garbage-collected language takes for granted.
