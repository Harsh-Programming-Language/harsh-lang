# 7. Packages, crates and modules

Every program so far fitted in one file. Real ones do not, and Rust's answer to "where does this code live and who may call it" has three layers: a *package* is what `hrs new` makes — a `Cargo.toml` and a `src/` directory; a *crate* is what the compiler builds from it, a binary or a library; and inside a crate, *modules* group items and decide which of them are visible from outside. This chapter is the module system: how to declare a module, how to name a thing inside one, what `pub` opens, what `use` shortens, and how a module moves into its own file.

## 7.1 Packages and crates

A crate is the unit of compilation: `rustc` is given one file — the *crate root*, `src/main.hrs` for a binary or `src/lib.hrs` for a library — and everything the crate contains is reached from there through `mod` declarations. A package holds at most one library crate and any number of binaries, and `Cargo.toml` names them; `hrs new hello` writes a package with one binary whose root is `src/main.hrs`, and `hrs run` transpiles the tree under `src/` and hands `cargo` the result. You met the arrangement in chapter 1; this chapter is what goes *in* the tree.

## 7.2 Modules

A module is a named scope for items — functions, structs, enums, other modules — declared with `mod` and a block:

@@ restaurant

`mod front_of_house:` opens a module; `mod hosting:` inside it opens a nested one, and the functions sit inside that. The layout is the same as everywhere: a heading, and the contents indented beneath. The result is a *module tree* rooted at the crate:

```text
crate
 └── front_of_house
     ├── hosting
     │   ├── add_to_waitlist
     │   └── seat_at_table
     └── serving
         ├── take_order
         ├── serve_order
         └── take_payment
```

Modules are for organising code the way directories organise files, and — the part that matters — for *privacy*: everything inside a module is private to that module and its descendants unless it is marked otherwise.

## 7.3 Paths

To call something in a module you name its path, the way you name a file in a directory. A path is either *absolute*, starting from `crate`, or *relative*, starting from the current module; the segments are joined with `.`:

@@ private_path !error

Both paths are correctly spelled and the program does not build, because `hosting` is private. The rule: a child module can see everything in its ancestors, but a parent cannot see inside a child unless the child says so. `eat_at_restaurant` lives in the crate root, which is `front_of_house`'s parent, so it can see `front_of_house` (siblings are visible); but `hosting` is inside `front_of_house` and was not made public, so the path stops there. The error names the exact segment. The reason for the default is that a module's insides are its implementation; what it chooses to expose is its interface, and changing an interface should be a decision, not an accident.

`pub` is that decision:

@@ pub_path

`pub mod hosting` lets the parent see the module; `pub fn add_to_waitlist` lets it call the function. Both are needed — making a module public does not make its contents public, only reachable. Now the absolute path `crate.front_of_house.hosting.add_to_waitlist$` and the relative `front_of_house.hosting.add_to_waitlist$` both work, and end in `$` because they are calls. Prefer the absolute path when the caller and the callee are likely to move independently; prefer the relative one when they will move together.

### `super`

A path may also start from the *parent* module, with `super`:

@@ super_path

`super.deliver_order$` from inside `back_of_house` reaches the crate root, where `deliver_order` lives. It is the module system's `..`: use it when a child depends on something in its parent and the two will stay together, so the relation "one level up" is the stable one to write down.

### Public structs and enums

`pub` on a struct makes the *type* public, and each field is still private unless it too is marked:

@@ pub_struct

`toast` is public and `seasonal_fruit` is not, so outside `back_of_house` a `Breakfast` can be read and written through `toast` alone — and cannot be constructed with a literal at all, since a literal has to give every field. That is why `summer` exists: a public associated function is the way to build a struct that has a private field, and it is where the module gets to choose the default. Try the private field from outside:

@@ private_field !error

An enum is the opposite: `pub enum` makes every variant public, because an enum with hidden variants would be one you could not match on:

@@ pub_enum

## 7.4 `use`

Writing the full path at every call is tedious, and `use` brings a path into scope once:

@@ use_path

After `use crate.front_of_house.hosting`, the name `hosting` is in scope in this module and `hosting.add_to_waitlist$` works. This is also the idiomatic *depth* to import at for a function: bring in the parent module, not the function, so that every call says where it came from — `hosting.add_to_waitlist$` reads as "hosting's", where a bare `add_to_waitlist$` would look local. For structs, enums and traits the convention is the opposite — import the type itself, `use std.collections.HashMap` — since a type's name is meant to be used bare.

A `use` is scoped to the module it appears in. It does not reach into a child module:

@@ use_scope !error

The `use` is in the crate root; `customer` is a child; the name is not in scope there. Either move the `use` into `customer` or write `super.hosting` from inside it.

### `as`

When two imports would have the same name, rename one:

@@ use_as

Both `std.fmt` and `std.io` define a `Result`. `use std.io.Result as IoResult` brings the second in under a different name; the `as` is Rust's, unchanged.

### Re-exporting with `pub use`

`use` brings a name in for *this* module. `pub use` brings it in and passes it on, so that users of this module see it as if it had been defined here:

@@ reexport

`restaurant`'s user calls `restaurant.hosting.add_to_waitlist$` and never learns that `hosting` is really inside `front_of_house`. Re-exporting is how a library presents a public structure different from its internal one — the way the code is organised for its authors and the way it is presented to its users need not be the same tree.

### Nested paths and globs

Several imports from one place can share their prefix:

@@ nested_use

`use std.(cmp.Ordering, collections.HashMap)` is two imports; the parentheses group the branches of a `use` tree, which is one of the four things parentheses do in Harsh. `use std.io.(self, Write)` imports the module `io` itself *and* one item from it — `self` in a `use` tree means "the thing before the parentheses". And `use std.collections.*` imports everything the module exports: the *glob*. Globs make it hard to see where a name came from, so they are for two places — tests, which import everything from the module under test, and *preludes*, modules designed to be imported whole.

## 7.5 Separating modules into files

So far every module has had its body inline. As a program grows, a module goes into a file of its own, and the tree of modules becomes a tree of files:

@@ garden

Three files. `src/main.hrs` declares `pub mod garden` with no body — just the declaration on a line — and the compiler looks for the body in `src/garden.hrs`. That file declares `pub mod vegetables` the same way, and the body of *that* is found in `src/garden/vegetables.hrs`: a module's children go in a directory named after it. The `use` at the top of `main.hrs` is then the ordinary thing, a path through the tree to the item wanted.

Only the crate root ever needs to know the tree's shape; a file that declares a child module does not say where the child's file is, because there is only one place it can be. And nothing in the calling code changes when a module moves from inline to its own file — the path `crate.garden.vegetables.Asparagus` is the same either way. Moving code into files is a filing decision, and the module system keeps it from being anything more.

@harsh The file layout is not Harsh's invention and Harsh does not touch it: `hrs build` transpiles every file under `src/` into `target/hrs/`, tree for tree and name for name, so the compiler finds each module exactly where the declaration says it is.

## 7.6 What you have

A package is what `hrs new` makes and a crate is what the compiler builds from its root. `mod` opens a module; everything in it is private until `pub` says otherwise, and `pub` on a struct still leaves each field private. Paths are `crate.` from the root, or relative, or `super.` from the parent, with `.` between segments. `use` shortens a path for one module — import the parent for a function, the type for a type — `as` renames, `pub use` re-exports, `(…)` groups, `*` takes all. A bodiless `mod name` puts the body in `name.hrs`, and its children in `name/`.

Next: the collections the standard library gives you — `Vec`, `String` and `HashMap` — which is where ownership starts to earn its keep in daily code.
