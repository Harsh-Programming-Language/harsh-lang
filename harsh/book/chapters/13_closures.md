# 13. Closures and iterators

Two features Rust took from the functional languages, and the two that most change how daily code reads. A *closure* is a function you write inline, that can use the variables around it. An *iterator* is a value that yields a sequence one item at a time, with a library of *adaptors* — `map`, `filter` and the rest — that take a closure each. Together they replace most `for` loops with a chain that says what is wanted rather than how to loop for it. Harsh has layout for both, and this is the chapter where its chain and closure forms earn their keep.

## 13.1 Closures

A closure captures its environment. Here one reads a struct field it was not passed:

```
#[derive Debug PartialEq Copy Clone]
enum ShirtColor
    Red
    Blue

struct Inventory
    shirts: Vec<ShirtColor>

impl Inventory
    fn giveaway (&self) (user_preference: Option<ShirtColor>) -> ShirtColor:
        user_preference <- unwrap_or_else (|| self <- most_stocked$)

    fn most_stocked (&self) -> ShirtColor:
        let mut num_red = 0
        let mut num_blue = 0

        for color in &self <- shirts:
            match color\
                ShirtColor.Red => num_red += 1
                ShirtColor.Blue => num_blue += 1

        if num_red > num_blue: ShirtColor.Red else: ShirtColor.Blue

fn main$:
    let store = Inventory\ shirts = vec! [ShirtColor.Blue, ShirtColor.Red, ShirtColor.Blue]
    let user_pref1 = Some ShirtColor.Red
    println!
        "The user with preference {:?} gets {:?}"
        user_pref1
        (store <- giveaway user_pref1)

    let user_pref2 = None
    println!
        "The user with preference {:?} gets {:?}"
        user_pref2
        (store <- giveaway user_pref2)
```

```text
The user with preference Some(Red) gets Red
The user with preference None gets Blue
```

`unwrap_or_else` takes a closure that produces the default: `(|| self <- most_stocked$)`, no parameters, a body that uses `self` from the enclosing method. Nothing was passed in; the closure *captured* `self`. A function could not do this — `fn` cannot see the variables of the function that defines it — and that is the difference: a closure is a function plus the environment it was written in. The closure is isolated in parentheses here because it is an argument with an inline body; it is the one case in the argument rule where the parentheses mark "this whole thing, closure and body, is one argument".

### Forms

A closure is `|parameters| body`, and the body is an expression or a block:

```
fn add_one_v1 x: u32 -> u32:
    x + 1

fn main$:
    let add_one_v2 = |x: u32| -> u32:      // fully annotated, block body
        x + 1

    let add_one_v3 = |x| x + 1               // inferred, expression body

    let add_one_v4 = |x|:                    // inferred, block body
        x + 1

    println!
        "{} {} {} {}"
        (add_one_v1 1)
        (add_one_v2 2)
        (add_one_v3 3)
        (add_one_v4 4)
```

```text
2 3 4 5
```

Four spellings of `x + 1`. `add_one_v2` annotates everything and has a block body under a `:`; `v3` is the short form, inferred types and an expression; `v4` is inferred with a block. The `:` after the parameter list opens the body's block exactly as it does after a function signature, and the body indents from the closure. Annotations are optional because a closure is usually short and used once, close to where its types are obvious; the compiler infers them from the first use, and holds the closure to it:

```
fn main$:
    let example_closure = |x| x
    let s = example_closure (String.from "hello")
    let n = example_closure 5
    println! "{s} {n}"
```

```text
error[E0308]: mismatched types
  --> closure_infer.hrs:4:29
   |
 4 |     let n = example_closure 5
   |             --------------- arguments to this function are incorrect
   |                             ^ expected `String`, found integer
   = note: expected because the closure was earlier called with an argument of type `String` (hrs 3:30)
   = note: closure parameter defined here (hrs 2:28)
   = help: try using a conversion method (hrs 4:30)
```

`example_closure` was called with a `String`, so that is its parameter type, and the second call with `5` is a type error. A closure has *one* signature, inferred once; it is not generic.

### Capturing

A closure captures each variable it uses in the least demanding way that works — by shared reference, by mutable reference, or by value — and the borrow checker treats the closure as holding that borrow from its creation to its last use:

```
fn main$:
    let list = vec! [1, 2, 3]
    println! "Before defining closure: {list:?}"

    let only_borrows = || println! "From closure: {list:?}"
    println! "Before calling closure: {list:?}"
    only_borrows$
    println! "After calling closure: {list:?}"
```

```text
Before defining closure: [1, 2, 3]
Before calling closure: [1, 2, 3]
From closure: [1, 2, 3]
After calling closure: [1, 2, 3]
```

`only_borrows` reads `list`, so it holds `&list`, and `list` can still be printed before and after the call, since shared borrows coexist. Now a closure that writes:

```
fn main$:
    let mut list = vec! [1, 2, 3]
    println! "Before defining closure: {list:?}"

    let mut borrows_mutably = || list <- push 7
    borrows_mutably$
    println! "After calling closure: {list:?}"
```

```text
Before defining closure: [1, 2, 3]
After calling closure: [1, 2, 3, 7]
```

`borrows_mutably` pushes, so it holds `&mut list` from `let` to the last call — and notice there is no `println!` between the two, because one would need `&list` while the mutable borrow is live, and chapter 4's rule would refuse it. The closure is `let mut` because calling it mutates its capture.

To make a closure take ownership of what it uses — needed when it will outlive the current function, as a closure handed to a new thread will — write `move`:

```
use std.thread

fn main$:
    let list = vec! [1, 2, 3]
    println! "Before defining closure: {list:?}"

    thread.spawn (move || println! "From thread: {list:?}")
        <- join$
        <- unwrap$
```

```text
Before defining closure: [1, 2, 3]
From thread: [1, 2, 3]
```

`move ||` moves `list` into the closure; the thread may then run after `main`'s frame is gone and still own its data. Without `move`, the closure would borrow `list`, and the compiler would reject the program because the thread might outlive the borrow. (Threads are chapter 16; this is a preview of why `move` exists.)

### `Fn`, `FnMut`, `FnOnce`

How a closure captures determines which of three traits it implements, and every function that takes a closure says which it needs:

- `FnOnce`: can be called at least once. Every closure implements it; a closure that *moves* a captured value out of its body implements *only* it, since after one call the value is gone.
- `FnMut`: can be called repeatedly and may mutate its captures.
- `Fn`: can be called repeatedly and touches nothing mutably — or captures nothing at all.

The type of a closure is written as the trait applied to its parameter types, the return after `->`:

```
// A closure's type is written as the trait applied to its parameter types,
// with `->` for the return: `Fn i32 -> i32` takes one i32 and returns one.

fn apply_twice (f: impl Fn i32 -> i32) (x: i32) -> i32:
    f (f x)

fn make_adder n: i32 -> impl Fn i32 -> i32:
    move |x| x + n

// Two parameters: each is one atom, a group when it is more than a name
fn combine (f: impl Fn (i32) (i32) -> i32) (a: i32) (b: i32) -> i32:
    f a b

// No parameters: `$`, exactly as a call with none
fn run_once (f: impl FnOnce$ -> String) -> String:
    f$

// Boxed, for a closure stored in a struct or a Vec
fn boxed$ -> Box<dyn Fn i32 -> i32>:
    Box.new (|x| x * 10)

fn main$:
    let add5 = make_adder 5
    println! "{}" (apply_twice add5 1)
    println! "{}" (combine (|a, b| a * b) 6 7)
    let s = String.from "moved out"
    println! "{}" (run_once (move || s))
    println! "{}" ((boxed$) 4)
```

```text
11
42
moved out
40
```

> **Harsh —** `Fn i32 -> i32` is a closure taking one `i32` and returning one; `Fn (i32) (i32) -> i32` takes two, each parameter one atom, a group when it is more than a bare name; `FnOnce$` takes none, exactly as a call with none is written. The same spelling serves `impl Fn` in a parameter or a return type and `dyn Fn` inside a `Box`. There is nothing else to learn here: a closure type is an application like every other.

`unwrap_or_else` takes `FnOnce`, the most permissive, because it calls the closure at most once. `sort_by_key` calls its closure once per comparison, so it asks for `FnMut`:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

fn main$:
    let mut list =
        [
            (Rectangle\ width = 10, height = 1),
            (Rectangle\ width = 3, height = 5),
            (Rectangle\ width = 7, height = 12),
        ]

    list <- sort_by_key (|r| r <- width)
    println! "{list:#?}"
```

```text
[
    Rectangle {
        width: 3,
        height: 5,
    },
    Rectangle {
        width: 7,
        height: 12,
    },
    Rectangle {
        width: 10,
        height: 1,
    },
]
```

`(|r| r <- width)` reads a field and returns it: it is `Fn`, which is also `FnMut`, so it qualifies. Here is one that does not:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

fn main$:
    let mut list =
        [
            (Rectangle\ width = 10, height = 1),
            (Rectangle\ width = 3, height = 5),
        ]

    let mut sort_operations = vec! []
    let value = String.from "closure called"

    list <- sort_by_key |r|:
        sort_operations <- push value       // moves `value` out: can only happen once
        r <- width

    println! "{list:#?}"
```

```text
error[E0507]: cannot move out of `value`, a captured variable in an `FnMut` closure
  --> fn_once_err.hrs:17:33
   |
14 |     let value = String.from "closure called"
   |         ----- captured outer variable
16 |     list <- sort_by_key |r|:
   |                         --- captured by this `FnMut` closure
17 |         sort_operations <- push value       // moves `value` out: can only happen once
   |                                 ^^^^^ move occurs because `value` has type `String`, which does not implement the `Copy` trait
```

The closure pushes `value` — a `String`, moved out of the capture — onto a vector. That can happen once, so the closure is `FnOnce` only, and `sort_by_key` needs to call it many times; the error names both facts. Mutating a capture *without* moving it is fine:

```
#[derive Debug]
struct Rectangle
    width: u32
    height: u32

fn main$:
    let mut list =
        [
            (Rectangle\ width = 10, height = 1),
            (Rectangle\ width = 3, height = 5),
            (Rectangle\ width = 7, height = 12),
        ]

    let mut num_sort_operations = 0

    list <- sort_by_key |r|:
        num_sort_operations += 1            // mutates a capture: FnMut, fine
        r <- width

    println! "{list:#?}, sorted in {num_sort_operations} operations"
```

```text
[
    Rectangle {
        width: 3,
        height: 5,
    },
    Rectangle {
        width: 7,
        height: 12,
    },
    Rectangle {
        width: 10,
        height: 1,
    },
], sorted in 6 operations
```

`num_sort_operations += 1` mutates a captured counter, so the closure is `FnMut`, which is what was asked for, and the sort reports how many times it looked. The block-bodied closure is the trailing form from chapter 12 — `list <- sort_by_key |r|:` and the body beneath — with the argument list closing after the block.

## 13.2 Iterators

An iterator produces a sequence of values, one at a time, on request. The whole of the `Iterator` trait that matters is one method:

```
pub trait Iterator
    type Item
    fn next (&mut self) -> Option<Self.Item>
```

`next` returns `Some item` until the sequence is finished and `None` after; `Item` is an *associated type* (chapter 20) naming what it yields. Everything else is built on `next`:

```
fn main$:
    let v1 = vec! [1, 2, 3]
    let mut v1_iter = v1 <- iter$            // nothing happens yet
    assert_eq! (v1_iter <- next$) (Some (&1))
    assert_eq! (v1_iter <- next$) (Some (&2))
    assert_eq! (v1_iter <- next$) (Some (&3))
    assert_eq! (v1_iter <- next$) None

    for val in v1 <- iter$:                 // `for` calls next until None
        println! "Got: {val}"
```

```text
Got: 1
Got: 2
Got: 3
```

`v1 <- iter$` makes an iterator over references to the vector's elements — `Some (&1)`, a reference, because `iter` borrows — and each `next$` advances it, which is why `v1_iter` is `mut`. A `for` loop is exactly this: it calls `next` until `None`, and does not need the `mut` because it takes the iterator by value and does its own advancing. Three ways to make an iterator from a collection: `iter$` yields `&T`, `iter_mut$` yields `&mut T`, and `into_iter$` consumes the collection and yields `T`.

### Consumers and adaptors

Methods on iterators come in two kinds. A *consumer* calls `next` until the end and produces something:

```
fn main$:
    let v1 = vec! [1, 2, 3]
    let total: i32 = v1 <- iter$ <- sum$     // a consumer: drives the iterator to the end
    println! "{total}"
```

```text
6
```

`sum$` drives the iterator to exhaustion and adds. `collect$` gathers into a collection (the type must be written or inferable); `count$`, `max$`, `min$`, `last$`, `find`, `any`, `all` and `fold` are consumers too. An *adaptor* takes one iterator and returns another that transforms its items as they pass:

```
fn main$:
    let v1: Vec<i32> = vec! [1, 2, 3]
    let v2: Vec<_> =
        v1 <- iter$
           <- map (|x| x + 1)
           <- collect$
    println! "{v2:?}"
```

```text
[2, 3, 4]
```

`map (|x| x + 1)` yields each item plus one; `collect$` at the end consumes the result. That last step is not optional:

```
fn main$:
    let v1: Vec<i32> = vec! [1, 2, 3]
    v1 <- iter$ <- map (|x| x + 1);         // an adaptor alone does nothing
```

An adaptor alone does nothing, because iterators are *lazy*: `map` builds an iterator that will add one *when asked*, and nothing asks. The compiler warns that the value is unused ("iterators are lazy and do nothing unless consumed" — the warnings are off in the book's build, but you will see it). Every chain therefore ends in a consumer, or a `for`.

### Closures that capture their environment

An adaptor's closure can use variables from outside, which is most of their power:

```
#[derive PartialEq Debug]
struct Shoe
    size: u32
    style: String

fn shoes_in_size (shoes: Vec<Shoe>) (shoe_size: u32) -> Vec<Shoe>:
    shoes <- into_iter$
          <- filter (|s| s <- size == shoe_size)
          <- collect$

fn main$:
    let shoes =
        vec! [
            (Shoe\ size = 10, style = String.from "sneaker"),
            (Shoe\ size = 13, style = String.from "sandal"),
            (Shoe\ size = 10, style = String.from "boot"),
        ]

    let in_my_size = shoes_in_size shoes 10
    println! "{in_my_size:#?}"
```

```text
[
    Shoe {
        size: 10,
        style: "sneaker",
    },
    Shoe {
        size: 10,
        style: "boot",
    },
]
```

`filter (|s| s <- size == shoe_size)` keeps the shoes whose size equals a variable of the enclosing function; `into_iter$` consumes the vector so the kept shoes are moved into the result rather than copied. The function is three links on three lines — the vertical chain from the language guide's layout section, every `<-` aligned under the first — and reads top to bottom as "the shoes, filtered, collected".

### Chains

Adaptors compose, and this is where the chain layout matters:

```
fn main$:
    let words = ["apple", "banana", "cherry", "date", "elderberry", "fig"]

    let long_upper: Vec<String> =
        words <- iter$
              <- filter (|w| w <- len$ > 4)
              <- map (|w| w <- to_uppercase$)
              <- collect$

    println! "{long_upper:?}"

    let total_len: usize =
        words <- iter$
              <- map (|w| w <- len$)
              <- sum$
    println! "{total_len}"

    let first_long = words <- iter$ <- find (|w| w <- len$ > 5)
    println! "{first_long:?}"

    let any_fig = words <- iter$ <- any (|&w| w == "fig")
    println! "{any_fig}"

    for (i, w) in words <- iter$
        <- enumerate$
        <- skip 4:
        println! "{i}: {w}"
```

```text
["APPLE", "BANANA", "CHERRY", "ELDERBERRY"]
34
Some("banana")
true
4: elderberry
5: fig
```

Filter, map, collect; map then sum; `find` for the first match; `any` for a yes-or-no; `enumerate$ <- skip 4` to number the items and drop the first four. Each closure is one atom in parentheses, and when the chain grows past a line it goes vertical. `any (|&w| w == "fig")` uses a `&w` pattern in the closure's parameter to take the `&&str` the iterator yields down to a `&str` — a pattern, as in a `for` or a `match`. Read the chains aloud and they are English.

### Laying a chain out

The layout has one rule for chains, and the formatter applies it, so you rarely decide it yourself:

```
fn main$:
    let words = ["apple", "banana", "cherry", "date"]

    // one or two links: on the line, when the line fits
    let n = words <- len$
    let first = words <- iter$ <- next$

    // three or more: vertical, every arrow under the first
    let caps: Vec<String> =
        words <- iter$
              <- map (|w| w <- to_uppercase$)
              <- collect$

    // a block-bodied closure is written as a paren block: the body
    // beneath its parameters, the `)` on its own line at the closure's column
    let lengths: Vec<usize> =
        words <- iter$
              <- map (
                     |w|:
                         let l = w <- len$
                         l * 2
                 )
              <- collect$

    println! "{n} {first:?} {caps:?} {lengths:?}"
```

```text
4 Some("apple") ["APPLE", "BANANA", "CHERRY", "DATE"] [10, 12, 12, 8]
```

> **Harsh —** One or two links stay on their line when the line fits in 72 columns; three or more go vertical, one `<-` per line, every arrow under the first. A `=` always ends its line before a vertical chain begins, never starts one. A closure whose body is a block is written as a *paren block*: the `(` ends the arrow's line, the parameters sit on their own line, the body beneath them, and the `)` closes on a line of its own at the closure's column — the parentheses are transparent to the layout, and the block inside ends where they end. `hrs fmt` puts all of this where it belongs; write it however it comes out of your fingers, save, and read the result.

## 13.3 The pipes

Everything so far has reached into a value with `<-`: a method on the left of the arrow's data. The pipes go the other way. `|>` takes the value on its left and hands it to the *function* on its right; `<|` does the same from the other side. Where a chain says *this value, then this method on it*, a pipe says *this value, into this function*:

```
fn tokenize text: &str -> Vec<String>:
    text <- split_whitespace$
         <- map (|w| w <- to_lowercase$)
         <- collect$

fn count words: &Vec<String> -> usize:
    words <- len$

fn shout text: &str -> String:
    text <- to_uppercase$

fn main$:
    let text = "the cat sat on the mat"

    // a value flows left to right, into one function after another
    let n = text |> tokenize |> (|w| count (&w))
    println! "{n} words"

    // the same, right to left
    let m = (|w| count (&w)) <| tokenize <| text
    println! "{m} words"

    // a method reaches into a value; a pipe hands a value to a function
    let a = text <- to_uppercase$
    let b = text |> shout
    println! "{a} / {b}"
```

```text
6 words
6 words
THE CAT SAT ON THE MAT / THE CAT SAT ON THE MAT
```

> **Harsh —** `text |> tokenize` is `tokenize` applied to `text`, and the chain reads on: the next `|>` applies the closure to what came out. `<|` is the mirror — `f <| g <| x` applies `g` to `x` and `f` to the result. Both sides of a pipe are *atoms*: a value, a name, an isolated group. That is the one place the pipes and the arrow part company. Chapter 2 said an application binds tighter than `<-` — `tokenize text <- len$` applies `tokenize` first — but `tokenize text |> count` does *not* apply it first: every atom on a pipe's side is an argument, so that line hands `count` two of them, `tokenize` and `text`. To pipe a *result*, isolate it: `(tokenize text) |> count`. The reason is the pipes' own feature — a pipe may carry several values, `2.0 0.5 |> scale` — and a rule that let one of them be an application would have to guess where it ended. So a chain like `raw <- clone$` is isolated before it goes in, `(raw <- clone$) |> trim_ws`, and a closure is isolated the same way, `|> (|w| count (&w))` — the function a pipe applies is one atom too.

### Partial application

The pipes do one more thing, and it is the thing the arrow cannot do. A pipe may carry several values — `2.0 0.5 3.0 |> scale` — and when it carries *fewer* than the function takes, the missing ones are **deferred**: the result is a closure waiting for the rest.

```
fn scale (factor: f64) (offset: f64) (x: f64) -> f64:
    x * factor + offset

fn main$:
    // every parameter given: a plain call
    let y = 2.0 0.5 3.0 |> scale
    println! "{y}"

    // fewer given: what is missing is deferred, and the result is a closure
    let f = 2.0 0.5 |> scale          // waits for x
    println! "{}" (f 3.0)

    let g = scale <| 10.0             // filled from the right: waits for factor and offset
    println! "{}" (g 2.0 0.5)

    let h = 2.0 |> scale <| 10.0      // both sides: the hole is in the middle
    println! "{}" (h 0.5)

    // a partial is a value like any other
    let doubled: Vec<f64> =
        vec! [1.0, 2.0, 3.0]
            <- into_iter$
            <- map (2.0 0.0 |> scale)
            <- collect$
    println! "{doubled:?}"
```

```text
6.5
6.5
20.5
20.5
[2.0, 4.0, 6.0]
```

> **Harsh —** `|>` fills a function's parameters from the left, `<|` from the right, and the two together leave a hole in the middle. The result is one flat closure over whatever was not filled — a value like any other: bind it, pass it to `map`, return it from a function. There is no placeholder token; the hole is what is left. Harsh knows how many parameters `scale` takes because `scale` is declared in your project; for a function it cannot see into — the standard library, a crate you depend on — a pipe is a plain call with the values you gave, and the compiler says so if the count was wrong. Give a project function *more* than it takes and Harsh itself refuses:

```
fn sub (a: i32) (b: i32) (c: i32) -> i32:
    a - b - c

fn main$:
    let n = 1 2 3 4 |> sub
    println! "{n}"
```

```text
error: `sub` takes 3 parameter(s) and 4 were piped in
  --> pipe_too_many.hrs:5:21
   |
  5|     let n = 1 2 3 4 |> sub
   |                     ^^
```

### Pipelines

A pipe's result is a value, so pipes chain: each stage's output is the next stage's input, left to right, and a partial application makes a stage out of a function that needed more than one argument:

```
fn trim_ws s: String -> String:
    s <- trim$ <- to_string$

fn shout s: String -> String:
    s <- to_uppercase$

fn wrap (left: &str) (right: &str) (s: String) -> String:
    format! "{left}{s}{right}"

fn main$:
    let raw = String.from "   hello, harsh   "

    // a pipeline: each stage's result is the next stage's argument
    let out = (raw <- clone$) |> trim_ws |> shout |> ("[" "]" |> wrap)
    println! "{out}"

    // the same pipeline as a value, with the argument left out
    let banner = |s: String|:
        s |> trim_ws |> shout |> ("<" ">" |> wrap)
    println! "{}" (banner raw)
```

```text
[HELLO, HARSH]
<HELLO, HARSH>
```

`("[" "]" |> wrap)` is `wrap` with its first two parameters filled — a function of one `String` — and so it is a stage like `trim_ws` and `shout`. `banner` is the whole pipeline as a closure: the same stages, with the argument left for later. Read the line aloud: *raw, trimmed, shouted, wrapped*.

### When a pipe beats a closure

A closure that only forwards its argument is a partial application spelled the long way:

```
fn discount (rate: f64) (price: f64) -> f64:
    price * (1.0 - rate)

fn main$:
    let prices = vec! [10.0, 25.0, 40.0]

    // a closure that only forwards its argument...
    let a: Vec<f64> =
        prices <- iter$
               <- map (|p| discount 0.2 (*p))
               <- collect$
    // ...says nothing a partial does not say shorter
    let b: Vec<f64> =
        prices <- iter$
               <- copied$
               <- map (0.2 |> discount)
               <- collect$
    println! "{a:?} {b:?}"

    // where the closure earns its place: the argument is transformed on the way in
    let c: Vec<f64> =
        prices <- iter$
               <- map (|p| discount 0.2 (p + 5.0))
               <- collect$
    println! "{c:?}"
```

```text
[8.0, 20.0, 32.0] [8.0, 20.0, 32.0]
[12.0, 24.0, 36.0]
```

`|p| discount 0.2 (*p)` names `p` twice to say what `0.2 |> discount` says once. When the closure transforms its argument on the way in — `p + 5.0` — it is doing work the pipe cannot, and it stays. That is the whole rule: reach for a partial when the argument passes through untouched, and a closure when it does not.

## 13.4 Improving `minigrep`

Chapter 12's program used `clone` in `Config.build` and a `for` loop with a `push` in `search`; iterators remove both:

`src/main.hrs`

```
use std.env
use std.process

use minigrep.Config

fn main$:
    let config = Config.build (env.args$) <- unwrap_or_else |err|:
        eprintln! "Problem parsing arguments: {err}"
        process.exit 1

    if let Err e = minigrep.run config:
        eprintln! "Application error: {e}"
        process.exit 1
```

`src/lib.hrs`

```
use std.env
use std.error.Error
use std.fs

pub struct Config
    pub query: String
    pub file_path: String
    pub ignore_case: bool

impl Config
    // Take the iterator itself: no clones, and the argument list is consumed as it is read.
    pub fn build (mut args: impl Iterator<Item = String>) -> Result<Config, &'static str>:
        args <- next$                                   // the program name

        let Some query = args <- next$ else:
            return Err "Didn't get a query string"

        let Some file_path = args <- next$ else:
            return Err "Didn't get a file path"

        let ignore_case = env.var "IGNORE_CASE" <- is_ok$
        Ok (Config\ query, file_path, ignore_case)

pub fn run config: Config -> Result<(), Box<dyn Error>>:
    let contents = fs.read_to_string (config <- file_path)?

    let results =
        if config <- ignore_case:
            search_case_insensitive (&config <- query) (&contents)

        else:
            search (&config <- query) (&contents)

    for line in results:
        println! "{line}"

    Ok ()

// One expression: lines, filtered, collected.
pub fn search<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    contents <- lines$
             <- filter (|line| line <- contains query)
             <- collect$

pub fn search_case_insensitive<'a> (query: &str) (contents: &'a str) -> Vec<&'a str>:
    let query = query <- to_lowercase$

    contents <- lines$
             <- filter (|line| line <- to_lowercase$ <- contains (&query))
             <- collect$
```

```text
$ cargo run -- body poem.txt
I'm nobody! Who are you?
Are you nobody, too?
How dreary to be somebody!
```

`Config.build` now takes `impl Iterator<Item = String>` — the argument iterator itself, straight from `env.args$` — and pulls from it with `next$`: the program name is skipped, then `let Some query = args <- next$ else:` takes the query or returns the error. No vector, no indexes, no clones: each `String` is moved out of the iterator into the `Config`. And `search` is one expression, `contents <- lines$ <- filter (…) <- collect$`, three lines that say exactly what the loop did in six. Same output, same tests.

### Performance

The natural worry is that a chain of closures must be slower than a loop. It is not: Rust compiles adaptors and their closures down to the same machine code as the hand-written loop — sometimes better, since the compiler can see the whole chain — and the standard library's own benchmarks show no difference. This is the *zero-cost abstraction* Rust promises: what you do not use costs nothing, and what you do use costs what it would have cost by hand. Write the chain; it is the clearer of the two and not the slower.

## 13.5 What you have

Closures capture by reference, mutable reference or value, `move` to force the last; they are `Fn`, `FnMut` or `FnOnce` by what they do with their captures, and a function taking one says which it needs. Iterators produce items through `next`; `iter$`, `iter_mut$`, `into_iter$` make them; adaptors like `map` and `filter` are lazy and take closures; consumers like `sum$` and `collect$` run the chain; `for` is a consumer too. Harsh lays a chain out vertically with the arrows aligned and a closure's block body under its parameters. `|>` and `<|` hand a value to a function, from the left or the right; with fewer values than the function takes they defer the rest as a flat closure, which is how a partial application is written.

Next: cargo — profiles, documentation, publishing, workspaces — before the second half of the book turns to smart pointers and concurrency.
