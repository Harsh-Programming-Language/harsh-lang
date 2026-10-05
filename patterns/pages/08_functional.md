# Functional Programming

Rust borrows much from functional languages — closures, iterators, algebraic
types — and Harsh leans further that way: pipes, partial application and
comprehensions are its own. This chapter is where the two books differ most.
The original chapter: [Functional usage of Rust](https://rust-unofficial.github.io/patterns/functional/index.html).

## 8.1 Programming paradigms

*Original: [Programming paradigms](https://rust-unofficial.github.io/patterns/functional/paradigms.html)*

Imperative code says *how*: a counter, a loop, a mutation per step.
Declarative code says *what*: the result as a composition of operations. Rust
supports both; iterators make the declarative form idiomatic. Harsh adds two
more declarative forms — the pipe and the comprehension.

```rust harsh
fn square (n: u32) -> u32:
    n * n

fn report (total: u32) -> String:
    format! "the sum of squares is {total}"

fn main$:
    // Imperative: how.
    let mut total = 0
    for n in 1..=10:
        total += n * n
    println! "{total}"
    // Declarative: what -- an iterator chain.
    let total: u32 = (1..=10) <- map square <- sum$
    println! "{total}"
    // A pipe: the result flows into the next function.
    let line = ((1..=10) <- map square <- sum$) |> report
    println! "{line}"
    // A comprehension.
    let squares = list~ n * n for n in 1..=10u32
    let total: u32 = squares <- iter$ <- sum$
    println! "{total}"
```

```text
385
385
the sum of squares is 385
385
```

**In Harsh:** the same sum four ways — a loop, an iterator chain, a pipe
through a function, and a comprehension. The last three say what is computed;
only the first says how.

## 8.2 Generics as Type Classes

*Original: [Generics as Type Classes](https://rust-unofficial.github.io/patterns/functional/generics-type-classes.html)*

A generic type's parameters can select which methods exist: an `impl` for one
instantiation gives methods to that instantiation only — the way type classes
work in functional languages. The compiler then refuses, at compile time, a
method on the wrong kind of value.

```rust harsh
use std.marker.PhantomData

struct Http
struct Ftp

// One request type; its parameter selects what it can do.
struct Request<P>
    url: String
    protocol: PhantomData<P>

impl<P> Request<P>
    fn url (&self) -> &str:
        &self <- url

impl Request<Http>
    fn new_http (url: &str) -> Self:
        Self\ url = url <- to_string$, protocol = PhantomData
    fn post (&self) (body: &str) -> String:
        format! "POST {} with {body:?}" (self <- url$)

impl Request<Ftp>
    fn new_ftp (url: &str) -> Self:
        Self\ url = url <- to_string$, protocol = PhantomData
    fn list (&self) -> String:
        format! "LIST {}" (self <- url$)

fn main$:
    let h = Request.new_http "http://harsh-lang.com"
    let f = Request.new_ftp "ftp://example.com"
    println! "{}" (h <- post "hi")
    println! "{}" (f <- list$)
    // `h <- list$` would not compile: `list` exists for FTP requests only.
```

```text
POST http://harsh-lang.com with "hi"
LIST ftp://example.com
```

**In Harsh:** the two `impl` blocks, for `Request<Http>` and `Request<Ftp>`,
read as two short lists of what each protocol allows.

## 8.3 Functional Optics

*Original: [Functional Optics](https://rust-unofficial.github.io/patterns/functional/optics.html)*

*Optics* are the functional vocabulary for conversions between types. An
**iso** converts both ways and loses nothing; a **poly iso** is an iso that
works for every type parameter; a **prism** converts one way always and the
other way only sometimes — a parse that may fail, and the rendering that
always succeeds. The original uses them to explain the design of Serde's API,
whose deserializer drives a visitor that may fail — a prism in all but name.

```rust harsh
use std.collections.VecDeque

// An iso: two conversions that undo each other.
#[derive Debug Clone Copy PartialEq]
struct Celsius f64
#[derive Debug Clone Copy PartialEq]
struct Fahrenheit f64

fn to_f (c: Celsius) -> Fahrenheit:
    Fahrenheit (c.0 * 9.0 / 5.0 + 32.0)
fn to_c (f: Fahrenheit) -> Celsius:
    Celsius ((f.0 - 32.0) * 5.0 / 9.0)

// A poly iso: the same, for every element type.
fn to_deque<T> (v: Vec<T>) -> VecDeque<T>:
    VecDeque.from v
fn to_vec<T> (d: VecDeque<T>) -> Vec<T>:
    Vec.from d

// A prism: one way may fail, the other always succeeds.
fn preview (s: &str) -> Option<u16>:
    s <- parse$ <- ok$
fn review (port: u16) -> String:
    port <- to_string$

fn main$:
    let boiling = Celsius 100.0
    println!
        "{:?} and back: {}"
        (to_f boiling)
        (to_c (to_f boiling) == boiling)
    println! "{:?}" (to_vec (to_deque (vec! 'a' 'b')))
    println! "{:?} {:?}" (preview "8080") (preview "http")
    println! "{}" (preview (&review 443) == Some 443)
```

```text
Fahrenheit(212.0) and back: true
['a', 'b']
Some(8080) None
true
```

**In Harsh:** each optic is a pair of plain functions; the round trips at the
end are the laws they obey.
