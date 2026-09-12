# 14. More about cargo

Cargo has done the building so far, and it does more: release builds, documentation, publishing, and workspaces for a project of several crates. This chapter is short and mostly about commands, because there is little to show that compiles — but one thing, documentation comments, is code in your files and worth seeing through the transpiler.

## 14.1 Build profiles

`cargo build` (and `hrs build`) uses the *dev* profile: fast to compile, unoptimised, with debug assertions and overflow checks on. `cargo build --release` uses the *release* profile: slow to compile, optimised, checks off, and this is the binary you ship or benchmark. The profiles are configurable in `Cargo.toml`:

```text
[profile.dev]
opt-level = 0

[profile.release]
opt-level = 3
```

`opt-level` runs from 0 to 3; those are the defaults, and you change them when you have a reason — a dev build of a program that is too slow to test unoptimised, for instance. Harsh adds nothing here: `hrs build --release` passes the flag through.

## 14.2 Documentation comments

Rust has a comment form that becomes documentation. `///` documents the item that follows; `//!` documents the enclosing item, and at the top of a file that is the crate or module itself:

`src/lib.hrs`

```
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
test target/hrs/lib.rs - add_one (line 10) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
```

`cargo doc --open` renders every `///` and `//!` in the crate as HTML, with the Markdown inside them — headings, code blocks, links — laid out. The conventional sections are `# Examples`, `# Panics` (when the function can), `# Errors` (what `Err`s it returns) and `# Safety` (for `unsafe` functions). The example in the `///` is Harsh, like everything else in the file, and `cargo test` *runs it* — every code block in a doc comment is a test, so documentation cannot drift from the code without failing the build.

> **Harsh —** A fenced block in a doc comment is the one kind of comment the transpiler reads. The documentation tool lifts such a block out, compiles it and runs it — it is code, not prose — so it is written in Harsh and the transpiler writes out the Rust that tool expects, the same translation it does everywhere else. A fence tagged with another language — `text`, `toml` — is prose and is left exactly as you wrote it. A line beginning `#` inside the fence is the documentation tool's own mark for *compile this but do not show it*, and it survives the translation.

### Re-exports for a public API

Chapter 7 mentioned `pub use` for presenting a different structure to users than the code has. In a library it is what makes the docs usable:

```
//! Art: a library for modelling artistic concepts.

pub use self.kinds.PrimaryColor
pub use self.kinds.SecondaryColor
pub use self.utils.mix

pub mod kinds:
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

pub mod utils:
    use crate.kinds.*

    /// Combines two primary colors in equal amounts to create a secondary color.
    pub fn mix (c1: PrimaryColor) (c2: PrimaryColor) -> SecondaryColor:
        match (c1, c2):
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

## 14.3 Publishing

`crates.io` is the registry `cargo` fetches dependencies from, and publishing to it is `cargo publish` after `cargo login` once with a token from the site. `Cargo.toml` needs `name` (unique on the registry), `version`, `description` and `license` before the registry will accept it; a publish is permanent, since other crates may depend on it — versions can be *yanked* (`cargo yank --vers 1.0.1`) to stop new projects picking them up, but never deleted. For a Harsh crate, `hrs export` writes the transpiled Rust as a plain crate under `target/export`, which is what you publish: the registry, and the people who depend on you, see Rust.

## 14.4 Workspaces

A *workspace* is a set of packages sharing one `Cargo.lock` and one `target/` directory, for a project that has grown to several crates — a binary and the libraries it is split into, say. The root `Cargo.toml` lists the members:

```text
[workspace]
members = ["adder", "add_one"]
```

and each member is an ordinary package in its own directory, depending on its siblings by path (`add_one = { path = "../add_one" }`). `cargo build` at the root builds them all; `cargo test -p add_one` tests one. `hrs` walks each member's `src/` in turn.

## 14.5 Installing binaries

`cargo install crate_name` fetches a crate from the registry, builds its binary and puts it in `~/.cargo/bin`, which is on your `PATH` once Rust is installed — it is how command-line tools written in Rust get distributed, and how `hrs` itself is installed from its repository (`cargo install --path .`). Any binary named `cargo-something` on the `PATH` also becomes a subcommand, `cargo something`, which is how cargo's own set of commands is extended.

## 14.6 What you have

`--release` for the optimised build; `///` and `//!` for documentation that `cargo doc` renders and `cargo test` runs; `pub use` to give a crate a public shape; `cargo publish` for the registry, from `hrs export`'s Rust; workspaces for several crates; `cargo install` for tools. None of it is Harsh's, and all of it works on the transpiled tree.

Next: smart pointers — `Box`, `Rc` and `RefCell`, which are how Rust does the data structures a garbage-collected language takes for granted.
