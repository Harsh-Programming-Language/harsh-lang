# Anti-patterns

Arrangements that look like solutions and create more problems than they
solve. Knowing them is half of avoiding them. The original chapter:
[Anti-patterns](https://rust-unofficial.github.io/patterns/anti_patterns/index.html).

## 7.1 Clone to satisfy the borrow checker

*Original: [Clone to satisfy the borrow checker](https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html)*

When the borrow checker refuses code, cloning the value makes the error go
away — and often hides the real mistake: two copies now drift apart, and the
change made to one is missing from the other. First the refusal:

```rust harsh
fn main$:
    let mut names = vec! (String.from "Ada")
    let first = &names[0]
    // Refused: `names` is borrowed by `first` while we push.
    names <- push (String.from "Grace")
    println! "{first}"
```

```text
error[E0502]: cannot borrow `names` as mutable because it is also borrowed as immutable
 --> clone_refused.hrs:5:5
  |
3 |     let first = &names[0];
  |                  ----- immutable borrow occurs here
4 |     // Refused: `names` is borrowed by `first` while we push.
5 |     names.push(String::from("Grace"));
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
6 |     println!("{first}")
  |               ------- immutable borrow later used here

error: aborting due to previous error

For more information about this error, try `rustc --explain E0502`.
```

then the fix that keeps one value, by ending the first borrow before the
second begins, instead of cloning:

```rust harsh
fn main$:
    let mut names = vec! (String.from "Ada")
    // The first borrow ends before the second begins: one value, no clone.
    println! "{}" (&names[0])
    names <- push (String.from "Grace")
    println! "{:?}" names
```

```text
Ada
["Ada", "Grace"]
```

**In Harsh:** rustc's error points at the `.hrs` line it concerns, so the
borrow to shorten is found where you wrote it.

## 7.2 #[deny(warnings)]

*Original: [#[deny(warnings)]](https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html)*

Denying all warnings in the source makes a build fail on the next compiler
that adds a warning — breaking code that did nothing wrong, and every crate
that depends on it. Deny specific lints you care about, or deny warnings in
continuous integration only (`RUSTFLAGS="-D warnings"`), never in the crate.

```rust harsh
// Deny the specific lints you mean, not every warning a future compiler adds.
#![deny unused_must_use]

fn checked (n: i32) -> Result<i32, String>:
    if n >= 0: Ok n else: Err (String.from "negative")

fn main$:
    // With `unused_must_use` denied, ignoring this Result would not compile.
    let r = checked 5
    println! "{:?}" r
```

```text
Ok(5)
```

**In Harsh:** attributes take their arguments by juxtaposition, `#![deny
unused_must_use]`; and `hrs check` passes cargo's flags through, so CI's
`-D warnings` works as for Rust.

## 7.3 Deref Polymorphism

*Original: [Deref Polymorphism](https://rust-unofficial.github.io/patterns/anti_patterns/deref.html)*

Implementing `Deref` to make one struct "inherit" another's methods imitates
inheritance, and misleads: `Deref` is for smart pointers, the target's traits
are not inherited, and generic code does not see through it. Compose instead,
and forward the few methods that should be shared.

```rust harsh
struct Animal
    name: String

impl Animal
    fn name (&self) -> &str:
        &self <- name

// Composition, not `Deref` to Animal: the shared method is forwarded.
struct Dog
    inner: Animal
    good: bool

impl Dog
    fn name (&self) -> &str:
        self <- inner <- name$
    fn describe (&self) -> String:
        format!
            "{} ({})"
            (self <- name$)
            (if self <- good: "good dog" else: "dog")

fn main$:
    let d =
        Dog\
            inner = Animal\ name = String.from "Rex"
            good = true
    println! "{}" (d <- describe$)
```

```text
Rex (good dog)
```

**In Harsh:** the forwarding method is one line, `self <- inner <- name$`.
