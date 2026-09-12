# 7. Packages, crates and modules

Every program so far fitted in one file. Real ones do not, and Rust's answer to "where does this code live and who may call it" has three layers: a *package* is what `hrs new` makes — a `Cargo.toml` and a `src/` directory; a *crate* is what the compiler builds from it, a binary or a library; and inside a crate, *modules* group items and decide which of them are visible from outside. This chapter is the module system: how to declare a module, how to name a thing inside one, what `pub` opens, what `use` shortens, and how a module moves into its own file.

## 7.1 Packages and crates

A crate is the unit of compilation: `rustc` is given one file — the *crate root*, `src/main.hrs` for a binary or `src/lib.hrs` for a library — and everything the crate contains is reached from there through `mod` declarations. A package holds at most one library crate and any number of binaries, and `Cargo.toml` names them; `hrs new hello` writes a package with one binary whose root is `src/main.hrs`, and `hrs run` transpiles the tree under `src/` and hands `cargo` the result. You met the arrangement in chapter 1; this chapter is what goes *in* the tree.

## 7.2 Modules

A module is a named scope for items — functions, structs, enums, other modules — declared with `mod` and a block:

```
mod front_of_house:
    mod hosting:
        fn add_to_waitlist$:
            println! "added to waitlist"

        fn seat_at_table$:
            println! "seated"

    mod serving:
        fn take_order$:
            println! "order taken"

        fn serve_order$:
            println! "served"

        fn take_payment$:
            println! "paid"

fn main$:
    println! "the restaurant is open"
```

```text
the restaurant is open
```

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

```
mod front_of_house:
    mod hosting:
        fn add_to_waitlist$:
            println! "added to waitlist"

fn eat_at_restaurant$:
    crate.front_of_house.hosting.add_to_waitlist$    // absolute path
    front_of_house.hosting.add_to_waitlist$          // relative path

fn main$:
    eat_at_restaurant$
```

```text
error[E0603]: module `hosting` is private
  --> private_path.hrs:7:26
   |
 7 |     crate.front_of_house.hosting.add_to_waitlist$    // absolute path
   |                          ^^^^^^^ private module
   |                                  --------------- function `add_to_waitlist` is not publicly re-exported
   = note: the module `hosting` is defined here (hrs 2:5)

error[E0603]: module `hosting` is private
  --> private_path.hrs:8:20
   |
 8 |     front_of_house.hosting.add_to_waitlist$          // relative path
   |                    ^^^^^^^ private module
   |                            --------------- function `add_to_waitlist` is not publicly re-exported
   = note: the module `hosting` is defined here (hrs 2:5)
```

Both paths are correctly spelled and the program does not build, because `hosting` is private. The rule: a child module can see everything in its ancestors, but a parent cannot see inside a child unless the child says so. `eat_at_restaurant` lives in the crate root, which is `front_of_house`'s parent, so it can see `front_of_house` (siblings are visible); but `hosting` is inside `front_of_house` and was not made public, so the path stops there. The error names the exact segment. The reason for the default is that a module's insides are its implementation; what it chooses to expose is its interface, and changing an interface should be a decision, not an accident.

`pub` is that decision:

```
mod front_of_house:
    pub mod hosting:
        pub fn add_to_waitlist$:
            println! "added to waitlist"

fn eat_at_restaurant$:
    crate.front_of_house.hosting.add_to_waitlist$    // absolute path, from the crate root
    front_of_house.hosting.add_to_waitlist$          // relative path, from here

fn main$:
    eat_at_restaurant$
```

```text
added to waitlist
added to waitlist
```

`pub mod hosting` lets the parent see the module; `pub fn add_to_waitlist` lets it call the function. Both are needed — making a module public does not make its contents public, only reachable. Now the absolute path `crate.front_of_house.hosting.add_to_waitlist$` and the relative `front_of_house.hosting.add_to_waitlist$` both work, and end in `$` because they are calls. Prefer the absolute path when the caller and the callee are likely to move independently; prefer the relative one when they will move together.

### `super`

A path may also start from the *parent* module, with `super`:

```
fn deliver_order$:
    println! "delivered"

mod back_of_house:
    pub fn fix_incorrect_order$:
        cook_order$
        super.deliver_order$      // one level up: the crate root

    fn cook_order$:
        println! "cooked"

fn main$:
    back_of_house.fix_incorrect_order$
```

```text
cooked
delivered
```

`super.deliver_order$` from inside `back_of_house` reaches the crate root, where `deliver_order` lives. It is the module system's `..`: use it when a child depends on something in its parent and the two will stay together, so the relation "one level up" is the stable one to write down.

### Public structs and enums

`pub` on a struct makes the *type* public, and each field is still private unless it too is marked:

```
mod back_of_house:
    pub struct Breakfast
        pub toast: String
        seasonal_fruit: String

    impl Breakfast:
        pub fn summer toast: &str -> Breakfast:
            Breakfast\
                toast = String.from toast
                seasonal_fruit = String.from "peaches"

fn main$:
    // A public constructor is the only way in, since one field is private.
    let mut meal = back_of_house.Breakfast.summer "Rye"
    meal <- toast = String.from "Wheat"       // public field: readable and settable
    println! "I'd like {} toast please" (meal <- toast)
```

```text
I'd like Wheat toast please
```

`toast` is public and `seasonal_fruit` is not, so outside `back_of_house` a `Breakfast` can be read and written through `toast` alone — and cannot be constructed with a literal at all, since a literal has to give every field. That is why `summer` exists: a public associated function is the way to build a struct that has a private field, and it is where the module gets to choose the default. Try the private field from outside:

```
mod back_of_house:
    pub struct Breakfast
        pub toast: String
        seasonal_fruit: String

    impl Breakfast:
        pub fn summer toast: &str -> Breakfast:
            Breakfast\
                toast = String.from toast
                seasonal_fruit = String.from "peaches"

fn main$:
    let mut meal = back_of_house.Breakfast.summer "Rye"
    meal <- seasonal_fruit = String.from "blueberries"
```

```text
error[E0616]: field `seasonal_fruit` of struct `Breakfast` is private
  --> private_field.hrs:14:13
   |
14 |     meal <- seasonal_fruit = String.from "blueberries"
   |             ^^^^^^^^^^^^^^ private field
```

An enum is the opposite: `pub enum` makes every variant public, because an enum with hidden variants would be one you could not match on:

```
mod back_of_house:
    #[derive Debug]
    pub enum Appetizer
        Soup
        Salad

fn main$:
    let order1 = back_of_house.Appetizer.Soup     // variants are public with the enum
    let order2 = back_of_house.Appetizer.Salad
    println! "{:?} {:?}" order1 order2
```

```text
Soup Salad
```

## 7.4 `use`

Writing the full path at every call is tedious, and `use` brings a path into scope once:

```
mod front_of_house:
    pub mod hosting:
        pub fn add_to_waitlist$:
            println! "added to waitlist"

use crate.front_of_house.hosting          // bring the module in, not the function

fn eat_at_restaurant$:
    hosting.add_to_waitlist$              // the parent says where it came from

fn main$:
    eat_at_restaurant$
```

```text
added to waitlist
```

After `use crate.front_of_house.hosting`, the name `hosting` is in scope in this module and `hosting.add_to_waitlist$` works. This is also the idiomatic *depth* to import at for a function: bring in the parent module, not the function, so that every call says where it came from — `hosting.add_to_waitlist$` reads as "hosting's", where a bare `add_to_waitlist$` would look local. For structs, enums and traits the convention is the opposite — import the type itself, `use std.collections.HashMap` — since a type's name is meant to be used bare.

A `use` is scoped to the module it appears in. It does not reach into a child module:

```
mod front_of_house:
    pub mod hosting:
        pub fn add_to_waitlist$:
            println! "added to waitlist"

use crate.front_of_house.hosting

mod customer:
    pub fn eat_at_restaurant$:
        hosting.add_to_waitlist$          // `use` above is in the parent, not here

fn main$:
    customer.eat_at_restaurant$
```

```text
error[E0433]: failed to resolve: use of undeclared crate or module `hosting`
  --> use_scope.hrs:10:9
   |
10 |         hosting.add_to_waitlist$          // `use` above is in the parent, not here
   |         ^^^^^^^ use of undeclared crate or module `hosting`
   = help: consider importing this module through its public re-export (hrs 9:5)
```

The `use` is in the crate root; `customer` is a child; the name is not in scope there. Either move the `use` into `customer` or write `super.hosting` from inside it.

### `as`

When two imports would have the same name, rename one:

```
use std.fmt.Result
use std.io.Result as IoResult             // two `Result`s: rename one

fn function1$ -> Result:
    Ok ()

fn function2$ -> IoResult<()>:
    Ok ()

fn main$:
    println! "{:?} {:?}" (function1$) (function2$ <- is_ok$)
```

```text
Ok(()) true
```

Both `std.fmt` and `std.io` define a `Result`. `use std.io.Result as IoResult` brings the second in under a different name; the `as` is Rust's, unchanged.

### Re-exporting with `pub use`

`use` brings a name in for *this* module. `pub use` brings it in and passes it on, so that users of this module see it as if it had been defined here:

```
mod restaurant:
    mod front_of_house:
        pub mod hosting:
            pub fn add_to_waitlist$:
                println! "added to waitlist"

    pub use front_of_house.hosting        // re-export: visible to users of `restaurant`

fn main$:
    restaurant.hosting.add_to_waitlist$   // the internal `front_of_house` is not mentioned
```

```text
added to waitlist
```

`restaurant`'s user calls `restaurant.hosting.add_to_waitlist$` and never learns that `hosting` is really inside `front_of_house`. Re-exporting is how a library presents a public structure different from its internal one — the way the code is organised for its authors and the way it is presented to its users need not be the same tree.

### Nested paths and globs

Several imports from one place can share their prefix:

```
use std.(cmp.Ordering, collections.HashMap)   // two paths sharing a prefix
use std.io.(self, Write)                      // the module itself, and one item from it
use std.collections.*                         // everything: the glob, for tests and preludes

fn main$:
    let mut m: HashMap<&str, i32> = HashMap.new$
    m <- insert "a" 1

    let s: HashSet<i32> = HashSet.new$        // from the glob
    let ord = 1 <- cmp (&2)
    let mut out = io.stdout$
    writeln! out "{:?} {:?} {}" ord (m <- get "a") (s <- len$)
        <- unwrap$

    let _ = Ordering.Less
```

```text
Less Some(1) 0
```

`use std.(cmp.Ordering, collections.HashMap)` is two imports; the parentheses group the branches of a `use` tree, which is one of the four things parentheses do in Harsh. `use std.io.(self, Write)` imports the module `io` itself *and* one item from it — `self` in a `use` tree means "the thing before the parentheses". And `use std.collections.*` imports everything the module exports: the *glob*. Globs make it hard to see where a name came from, so they are for two places — tests, which import everything from the module under test, and *preludes*, modules designed to be imported whole.

## 7.5 Separating modules into files

So far every module has had its body inline. As a program grows, a module goes into a file of its own, and the tree of modules becomes a tree of files:

`src/main.hrs`

```
use crate.garden.vegetables.Asparagus

pub mod garden      // the body is in src/garden.hrs

fn main$:
    let plant = Asparagus
    println! "I'm growing {plant:?}!"
```

`src/garden.hrs`

```
pub mod vegetables  // the body is in src/garden/vegetables.hrs
```

`src/garden/vegetables.hrs`

```
#[derive Debug]
pub struct Asparagus
```

```text
$ cargo run
I'm growing Asparagus!
```

Three files. `src/main.hrs` declares `pub mod garden` with no body — just the declaration on a line — and the compiler looks for the body in `src/garden.hrs`. That file declares `pub mod vegetables` the same way, and the body of *that* is found in `src/garden/vegetables.hrs`: a module's children go in a directory named after it. The `use` at the top of `main.hrs` is then the ordinary thing, a path through the tree to the item wanted.

Only the crate root ever needs to know the tree's shape; a file that declares a child module does not say where the child's file is, because there is only one place it can be. And nothing in the calling code changes when a module moves from inline to its own file — the path `crate.garden.vegetables.Asparagus` is the same either way. Moving code into files is a filing decision, and the module system keeps it from being anything more.

> **Harsh —** The file layout is not Harsh's invention and Harsh does not touch it: `hrs build` transpiles every file under `src/` into `target/hrs/`, tree for tree and name for name, so the compiler finds each module exactly where the declaration says it is.

## 7.6 What you have

A package is what `hrs new` makes and a crate is what the compiler builds from its root. `mod` opens a module; everything in it is private until `pub` says otherwise, and `pub` on a struct still leaves each field private. Paths are `crate.` from the root, or relative, or `super.` from the parent, with `.` between segments. `use` shortens a path for one module — import the parent for a function, the type for a type — `as` renames, `pub use` re-exports, `(…)` groups, `*` takes all. A bodiless `mod name` puts the body in `name.hrs`, and its children in `name/`.

Next: the collections the standard library gives you — `Vec`, `String` and `HashMap` — which is where ownership starts to earn its keep in daily code.
