# 6. Functions

## 6.1 Declaring and calling

```rust harsh
// One group per parameter. A single parameter may drop its parentheses.
fn double n: i32 -> i32:
    n * 2

fn add (a: i32) (b: i32) -> i32:
    a + b

// No parameters at all: `$`.
fn greeting$ -> &'static str:
    "hello"

// Nothing returned: no arrow.
fn announce (what: &str):
    println! "-- {what} --"

fn main$:
    announce (greeting$)
    println! "{}" (double 21)
    println! "{}" (add 1 2)

    // The last expression is the value; `return` is for leaving early.
    // `-3` is isolated: without the parens, `abs - 3` is a subtraction.
    println! "{}" (abs (-3))

fn abs n: i32 -> i32:
    if n < 0:
        return -n
    n
```

```text
-- hello --
42
3
3
```

**One group per parameter**: `fn add (a: i32) (b: i32)`. A single parameter
may drop its parentheses, `fn double n: i32`. A comma list is refused, because
it would be a second spelling of the same thing.

`fn greeting$` takes nothing — the same `$` as every call that applies a name
to nothing.

Calling juxtaposes: `add 1 2`. An argument that is not one atom is isolated:
`(greeting$)`, `(abs (-3))`. That last one is the case to remember — without
its parentheses, `abs -3` reads as a subtraction, because that is what it
looks like.

## 6.2 Methods

```rust harsh
struct Rect
    w: f64
    h: f64

impl Rect
    // An associated function: no `self`, called through the type.
    fn square s: f64 -> Rect:
        Rect\ w = s, h = s

    // A method: `self` is the first parameter, and alone it may be bare.
    fn area (&self) -> f64:
        (self <- w) * (self <- h)

    // Taking `&mut self` to change the value.
    fn grow (&mut self) (by: f64):
        self <- w += by
        self <- h += by

fn main$:
    let mut r = Rect.square 2.0
    println! "{}" (r <- area$)
    r <- grow 1.0
    println! "{}" (r <- area$)
```

```text
4
9
```

An `impl` block takes no mark at all: the items follow the header. `self` is a
parameter like any other — alone it may be bare, `fn area (&self)`, and beside
others it is a group of its own.

`<-` reaches into a value, so a field is `self <- w` and a method call on a
value is `r <- area$`. An associated function is reached through the type,
`Rect.square 2.0`, because `.` is the path separator.

## 6.3 Closures

```rust harsh
fn apply (f: impl Fn i32 -> i32) (to: i32) -> i32:
    f to

fn main$:
    // A closure: parameters between bars, then the body.
    let twice = |x: i32| x * 2
    println! "{}" (apply twice 5)

    // A block body: `|x|:` with the body beneath, or `|x| do:` inline.
    let describe = |n: i32|:
        if n > 10:
            "big"
        else:
            "small"
    println! "{}" (describe 40)

    // A closure captures what it uses.
    let base = 100
    let offset = |x: i32| x + base
    println! "{}" (offset 5)

    // In a chain, a closure with a block body is isolated in parentheses:
    // a bare block would swallow the rest of the chain.
    let v = vec! 1 2 3 4
    let evens: Vec<i32> =
        v <- iter$
          <- filter (|n|: *n % 2 == 0)
          <- cloned$
          <- collect$
    println! "{:?}" evens

    // When the chain is a statement rather than a value being bound, the
    // last closure may keep its block, with the body beneath.
    v <- iter$ <- for_each |n|:
        print! "{n} "
    println! ""
```

```text
10
big
105
[2, 4]
1 2 3 4
```

`|x| body` on one line; `|x|:` with the body beneath; `|x| do:` for the same
thing inline. As a trailing argument a closure may own its block, and the
chain closes after it — but only when the chain is a statement. When the chain
is a value being bound, isolate the closure in parentheses, as `filter` does
here, so that the block cannot swallow what follows.
