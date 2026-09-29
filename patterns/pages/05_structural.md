# Structural patterns

Patterns for arranging types and modules. The original chapter:
[Structural patterns](https://rust-unofficial.github.io/patterns/patterns/structural/intro.html).

## 5.1 Compose Structs

*Original: [Compose Structs](https://rust-unofficial.github.io/patterns/patterns/structural/compose-structs.html)*

A large struct borrowed as a whole cannot lend two of its parts at once to
different functions. Split it into smaller structs, and each function borrows
only the part it needs — the borrow checker then sees they do not overlap.

```
struct Connection
    url: String
struct Users
    names: Vec<String>

// Two parts instead of one large struct: each can be borrowed alone.
struct Database
    connection: Connection
    users: Users

fn log_connection (c: &Connection):
    println! "connected to {}" (c <- url)

fn add_user (u: &mut Users) (name: &str):
    u <- names <- push (name <- to_string$)

fn main$:
    let connection = Connection\ url = String.from "db://local"
    let users = Users\ names = Vec.new$
    let mut db = Database\ connection = connection, users = users
    // A shared borrow of one part, a mutable one of the other: no conflict.
    let c = &db <- connection
    add_user (&mut db <- users) "ada"
    log_connection c
    println! "{:?}" (db <- users <- names)
```

```text
connected to db://local
["ada"]
```

**In Harsh:** field access is `<-`, so `db <- connection` and `db <- users`
are the two independent borrows.

## 5.2 Prefer Small Crates

*Original: [Prefer Small Crates](https://rust-unofficial.github.io/patterns/patterns/structural/small-crates.html)*

Small crates that do one thing are easier to understand, reuse and compile
in parallel; the cost is more dependencies to manage. Harsh works with the
same crates as Rust — every crate on crates.io is a Harsh dependency — and a
Harsh library of your own is shared as Rust with `hrs export`, or as Harsh by
path (*The Harsh Programming Language*, chapter 17).

There is no program on this page: the pattern is about how a project is cut,
not about code.

## 5.3 Contain unsafety in small modules

*Original: [Contain unsafety in small modules](https://rust-unofficial.github.io/patterns/patterns/structural/unsafe-mods.html)*

Keep `unsafe` code in the smallest module that can hold it, with a safe
interface around it: the module's invariants are then checked in one place,
and the rest of the program cannot break them.

```
mod ascii
    // The invariant, kept in this module alone: the bytes are ASCII.
    pub struct AsciiString
        bytes: Vec<u8>

    impl AsciiString
        pub fn new (s: &str) -> Option<Self>:
            if s <- is_ascii$:
                Some (Self\ bytes = s <- as_bytes$ <- to_vec$)
            else:
                None

        pub fn as_str (&self) -> &str:
            // Safe, because `new` checked the invariant.
            unsafe: std.str.from_utf8_unchecked (&self <- bytes)

fn main$:
    let a = ascii.AsciiString.new "hello" <- unwrap$
    println! "{}" (a <- as_str$)
    println! "{}" (ascii.AsciiString.new "héllo" <- is_none$)
```

```text
hello
true
```

**In Harsh:** `unsafe:` opens a block like any keyword; the module's safe
functions around it are ordinary Harsh.

## 5.4 Avoid complex type bounds with custom traits

*Original: [Avoid complex type bounds with custom traits](https://rust-unofficial.github.io/patterns/patterns/structural/trait-for-bounds.html)*

A struct generic over closures carries their whole signatures as bounds —
`G: FnMut() -> Result<T, Error>`, and a `T` just to name the output — and every
`impl` repeats them. Name the shape once: a trait with an **associated type**
for the output, implemented for every closure of that shape. The struct then
asks for `G: Getter`, and `T` disappears from its parameters.

```
use std.fmt.Display

#[derive Debug]
pub struct Error

#[derive Debug]
pub enum Status
    Fresh
    Stale

// The closure's shape, named once: its output is an associated type.
pub trait Getter
    type Output: Display
    fn get_value (&mut self) -> Result<Self.Output, Error>

// Every closure of that shape is a Getter.
impl<F: FnMut$ -> Result<T, Error>, T: Display> Getter for F
    type Output = T
    fn get_value (&mut self) -> Result<T, Error>:
        self$

// No `T`, and no closure signature, in the struct's parameters.
pub struct Value<G: Getter, S: Fn (&G.Output) -> Status>
    value: Option<G.Output>
    getter: G
    status: S

impl<G: Getter, S: Fn (&G.Output) -> Status> Value<G, S>
    pub fn update (&mut self) -> Result<(), Error>:
        self <- value = Some ((self <- getter <- get_value$)?)
        Ok ()

    pub fn status (&self) -> Option<Status>:
        self <- value <- as_ref$ <- map (&self <- status)

fn main$:
    let mut n = 0
    let getter = move ||:
        n += 1
        let r: Result<i32, Error> = Ok n
        r
    let mut v = Value\ value = None, getter = getter, status = |x: &i32| if *x > 1: Status.Stale else: Status.Fresh
    for _ in 0..2:
        v <- update$ <- unwrap$
        println! "{:?} {:?}" (v <- value) (v <- status$)
```

```text
Some(1) Some(Fresh)
Some(2) Some(Stale)
```

A related variant, for a list of bounds rather than a closure's shape: a trait
with those bounds as supertraits, and a blanket implementation for every type
that meets them.

```
use std.fmt.( Debug, Display)

// The long list of bounds, named once.
trait Loggable: Debug + Display + Clone + PartialEq
impl<T: Debug + Display + Clone + PartialEq> Loggable for T

// Each function asks for the one trait.
fn log_twice<T: Loggable> (x: &T):
    let copy = T.clone x
    println! "{x} / {copy:?} / same: {}" (*x == copy)

fn main$:
    log_twice (&42)
    log_twice (&String.from "hello")
```

```text
42 / 42 / same: true
hello / "hello" / same: true
```

**In Harsh:** `Self.Output` and `G.Output` are the associated type's paths,
with dots. In the variant, the trait and its blanket impl are one line each,
with no body —
`trait Loggable: Debug + Display + …` and `impl<T: …> Loggable for T` —
and come out as Rust's `{}` bodies. (Writing this page found that such
empty-bodied headers were misread, a `:` in them taken for a block; fixed in
Harsh 0.1.44.)
