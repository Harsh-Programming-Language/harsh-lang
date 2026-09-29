# Harsh by Example

Short programs, one idea each, every one of them compiled and run to produce
the output you see. If you know how to program, you can read this book from
the top and write Harsh by the end of it.

It is the *lookup* book: when you want to know how something is written, find
the page and copy the shape. *The Harsh Programming Language* is the one that
teaches the ideas — ownership, borrowing, traits, lifetimes — and is the place
to go when an example here works but you cannot see why. *The Harsh Language
Guide* is the third: it sets every Harsh spelling beside the Rust it stands
for, for readers who already know Rust.

**Rust without braces.**
**Rust with pipes, partial application, comprehensions and linear algebra.**
**Rust for functional programming, data science and machine learning.**

The last three lines are pages 7, 16 and 17: the pipes and partial
application, comprehensions, and matrices with linear algebra — what Harsh
adds to Rust, and the reason to use it.

There is no Rust in this book. Every program here is transpiled, compiled and
run before it reaches the page, so what you read is what the machine did.

The topics, and many of the example programs, follow *Rust by Example*,
rewritten in Harsh. `ATTRIBUTION.md` has the licence and the details.

# 1. Hello and printing

## 1.1 Hello, world!

The whole program:

```rust harsh
fn main$:
    println! "Hello, world!"
```

```text
Hello, world!
```

`fn main$` is the function every program starts from. The `$` says it takes no
arguments — Harsh writes *apply to nothing* that way, and you will see it
again on every call that takes none, like `String.new$`.

`println!` is a macro, which the `!` tells you, and its arguments are simply
written after it with spaces between them. No parentheses, no commas: that is
how every call in Harsh is written, macro or function.

## 1.2 Comments

```rust harsh
fn main$:
    // A line comment runs from the slashes to the end of the line.
    let x = 5

    /* A block comment can sit inside an expression, */
    let y = x + /* like this */ 5

    println! "x = {x}, y = {y}"
```

```text
x = 5, y = 10
```

Two kinds, both Rust's: `//` to the end of the line, and `/* … */` anywhere,
including in the middle of an expression. Block comments nest.

## 1.3 Printing

The string comes first, then the values that fill its holes:

```rust harsh
fn main$:
    // Each `{}` is filled by the next argument, juxtaposed after the string.
    println! "{} days" 31

    // A name in the braces takes it from a variable of that name.
    let actor = "the sun"
    println! "{actor} rises"

    // `{n}` picks the nth argument, counting from zero.
    println! "{0} is {1}, {1} is {0}" "this" "that"

    // A width after `:` pads the value; `>` right-aligns it.
    println! "[{:>5}]" 42

    // And a `{{` is a literal brace.
    println! "{{ not a hole }}"
```

```text
31 days
the sun rises
this is that, that is this
[   42]
{ not a hole }
```

The hole is `{}`. What goes between the braces is a small language of its own —
a name, a number, a width, an alignment — and all of it is Rust's, unchanged.
The thing to notice is what is *outside* the braces: the arguments are
juxtaposed, so `println! "{} {}" a b` needs no commas.

An argument that is not a single atom is isolated in parentheses, exactly as
it would be for a function:

    println! "{}" (a + 1)
    println! "{}" (p <- name)

That second one is a field access. `<-` is how Harsh reads into a value, and
it is an operator, so it needs the parentheses when used as an argument.

## 1.4 Debug

Most types cannot be printed until you say how. The quickest way is to derive
the programmer's spelling:

```rust harsh
// `Debug` is derived: it prints a value the way a programmer reads it.
#[derive Debug]
struct Point\ x: i32, y: i32

fn main$:
    let p = Point\ x = 3, y = 4

    // `{:?}` asks for the Debug spelling...
    println! "{:?}" p

    // ...and `{:#?}` for the same thing laid out over several lines.
    println! "{:#?}" p
```

```text
Point { x: 3, y: 4 }
Point {
    x: 3,
    y: 4,
}
```

`#[derive Debug]` is an attribute, and its arguments juxtapose like everything
else. `{:?}` asks for that spelling and `{:#?}` for the same thing over several
lines.

Note the struct declaration: `struct Point\ x: i32, y: i32`. The `\` opens a
*specification block* — Rust's `{ a, b }` groupings — and it always follows
the thing it specifies. The same `\` builds the value: `Point\ x = 3, y = 4`.
Written over several lines, the fields simply go beneath:

    struct Point
        x: i32
        y: i32

## 1.5 Display

`Display` is the spelling meant for whoever reads the output, and you write it
yourself:

```rust harsh
use std.fmt

struct Point\ x: i32, y: i32

// `Display` is written by hand: it is the spelling meant for a reader.
impl fmt.Display for Point
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        write! f "({}, {})" (self <- x) (self <- y)

fn main$:
    let p = Point\ x = 3, y = 4
    println! "{}" p
```

```text
(3, 4)
```

`impl fmt.Display for Point` takes no mark at all — an item body follows its
header directly, with the items beneath it. `.` is the path separator, so
`fmt.Display` is Rust's `fmt::Display`. Inside, `write!` takes the formatter
and the same string-and-holes as `println!`.

Once a type has `Display`, `{}` works on it, and `to_string$` comes free.


# 2. Primitives

## 2.1 Literals and operators

```rust harsh
fn main$:
    // A suffix names the type; an underscore is a spacer.
    let big = 1_000_000u64
    let small: i8 = -7
    let ratio = 2.5f32

    // Integer, float, bool, char.
    let yes = true
    let letter = 'H'

    println! "{big} {small} {ratio} {yes} {letter}"

    // The operators are Rust's, unchanged.
    println! "{}" (1 + 2 * 3)
    println! "{}" (7 / 2)          // integer division truncates
    println! "{}" (7 % 2)
    println! "{}" (1u32 << 5)
    println! "{}" (true && false)
```

```text
1000000 -7 2.5 true H
7
3
1
32
false
```

Numbers, `bool` and `char` are Rust's, and so is every operator in that list.
Harsh changes none of it. The one thing to watch is the parentheses: `1 + 2 * 3`
is an operator expression, and an operator expression handed to a call is
isolated, so it is `println! "{}" (1 + 2 * 3)`.

## 2.2 Tuples

```rust harsh
// A tuple struct: its fields juxtapose, like any application.
// (Parens would hold *one* payload: `struct Wrapped (i32, i32)` has a
// single field that is a pair.)
#[derive Debug]
struct Matrix f32 f32 f32 f32

fn transpose (m: Matrix) -> Matrix:
    // Fields of a tuple are reached by number.
    Matrix (m <- 0) (m <- 2) (m <- 1) (m <- 3)

fn main$:
    // A tuple value: one construct, commas inside.
    let pair = (1, "one")
    println! "{:?}" pair

    // Taking it apart binds both halves.
    let (n, name) = pair
    println! "{n} is {name}"

    // Constructing juxtaposes, as every application does.
    let m = Matrix 1.1 1.2 2.1 2.2
    println! "{:?}" m
    println! "{:?}" (transpose m)
```

```text
(1, "one")
1 is one
Matrix(1.1, 1.2, 2.1, 2.2)
Matrix(1.1, 2.1, 1.2, 2.2)
```

Two different things share the parentheses here, and it is worth keeping them
apart.

A **tuple value** is `(1, "one")` — one construct, commas inside, exactly as in
Rust. A **tuple struct** declares its fields by juxtaposition:
`struct Matrix f32 f32 f32 f32`. Parentheses in a declaration would hold a
single payload, so `struct Wrapped (i32, i32)` is one field that happens to be
a pair, not two fields.

Constructing follows the same rule as any call — `Matrix 1.1 1.2 2.1 2.2` —
and reaching a field by number is `m <- 0`.

## 2.3 Arrays and slices

```rust harsh
fn main$:
    // An array: fixed length, all one type.
    let xs: [i32; 5] = [1, 2, 3, 4, 5]

    // `[value; count]` fills it.
    let zeros = [0; 3]

    // Indexing is tight against the name.
    println! "{} {}" xs[0] xs[4]
    println! "{:?} {:?}" xs zeros
    println! "{}" (xs <- len$)

    // A slice borrows part of it.
    let middle = &xs[1..4]
    println! "{:?}" middle

    // Out of range is a panic, not a wrong answer -- see the next example.
    for x in xs <- iter$:
        print! "{x} "
    println! ""
```

```text
1 5
[1, 2, 3, 4, 5] [0, 0, 0]
5
[2, 3, 4]
1 2 3 4 5
```

An index is written tight against the name, `xs[0]`. That tightness is what
makes it part of the atom: in an application, `f xs[0]` passes the element,
while a spaced `f xs [0]` is refused rather than guessed at.

A slice borrows a run of it: `&xs[1..4]`.

Reading past the end is a panic, which is to say it stops rather than
returning something wrong:

```rust harsh
fn main$:
    let xs = [1, 2, 3]
    let n = 5
    println! "reaching for element {n}"
    println! "{}" xs[n]
```

```text
reaching for element 5
thread 'main' panicked at out_of_range.hrs:5:20:
```


# 3. Custom types

## 3.1 Structures

```rust harsh
#[derive Debug]
struct Person
    name: String
    age: u8

// A unit struct: no fields at all.
struct Unit

// A tuple struct: its fields juxtapose.
struct Pair i32 f32

fn main$:
    let peter = Person\ name = String.from "Peter", age = 27
    println! "{:?}" peter

    // The same thing over several lines: the fields go beneath.
    let jane =
        Person\
            name = String.from "Jane"
            age = 31
    println! "{} is {}" (jane <- name) (jane <- age)

    let pair = Pair 1 0.1
    println! "{} {}" (pair <- 0) (pair <- 1)

    // A struct is taken apart by naming its fields, the same `\`.
    let Pair x y = pair
    println! "{x} {y}"

    let _u = Unit
```

```text
Person { name: "Peter", age: 27 }
Jane is 31
1 0.1
1 0.1
```

Three shapes, and the mark is the same in all of them: **none**. A
declaration's body follows its name directly, with the fields beneath it, or
inline after a `\`.

    struct Person              struct Person\ name: String, age: u8
        name: String
        age: u8

Building a value uses the same `\`, with `=` for each field:
`Person\ name = …, age = 27`. Over several lines the fields go beneath, and
the opener starts its own line after the `=`. Taking one apart is the mirror
image — `let Pair x y = pair` — which is the general rule that **matching is
constructing**, read backwards.

## 3.2 Enums

```rust harsh
// A variant can carry nothing, a payload, or named fields.
#[derive Debug]
enum Event
    PageLoad
    Click i64 i64
    KeyPress char
    Paste
        text: String
        at: usize

fn describe (e: Event) -> String:
    match e\
        Event.PageLoad => String.from "the page loaded"
        Event.Click x y => format! "clicked at {x}, {y}"
        Event.KeyPress c => format! "pressed '{c}'"
        Event.Paste\ text, at => format! "pasted {:?} at {}" text at

fn main$:
    let events = [
        Event.PageLoad,
        Event.Click 20 80,
        Event.KeyPress 'q',
        Event.Paste\ text = String.from "hi", at = 3,
    ]
    for e in events:
        println! "{}" (describe e)
```

```text
the page loaded
clicked at 20, 80
pressed 'q'
pasted "hi" at 3
```

A variant carries nothing, a payload (`Click i64 i64`, juxtaposed like a tuple
struct), or named fields (`Paste` with its fields beneath). Constructing and
matching use the shape the variant was declared with, and the record variant
takes the `\` it was declared with:

    Event.Paste\ text = String.from "hi", at = 3        building
    Event.Paste\ text, at => …                          matching

`match e\` opens the arms with `\`, because a match's arms are a specification
block like a struct's fields — Rust writes both with braces and commas. The
arms then go beneath, or inline after the `\`:

    match n\ 0 => "zero", _ => "more"

## 3.3 Constants

```rust harsh
// A `const` is inlined wherever it is used; a `static` has one address.
const THRESHOLD: i32 = 10
static LANGUAGE: &str = "Harsh"

fn is_big (n: i32) -> bool:
    n > THRESHOLD

fn main$:
    println! "{LANGUAGE}: threshold is {THRESHOLD}"
    println! "{} {}" (is_big 5) (is_big 50)
```

```text
Harsh: threshold is 10
false true
```

`const` is inlined at each use; `static` names one place in memory. Both need
their type, and both are written at the top level or inside a function.


# 4. Bindings and types

## 4.1 Bindings

```rust harsh
fn main$:
    // A binding is immutable unless it says otherwise.
    let x = 1
    let mut y = 1
    y += 1
    println! "{x} {y}"

    // Shadowing: a second `let` of the same name, even of another type.
    let spaces = "   "
    let spaces = spaces <- len$
    println! "{spaces}"

    // A block has its own scope, and is an expression.
    let outer = 1
    do:
        let outer = 2
        println! "inner sees {outer}"
    println! "outer is still {outer}"

    // Declared first, given a value later.
    let answer
    if y > 1:
        answer = "big"
    else:
        answer = "small"
    println! "{answer}"
```

```text
1 2
3
inner sees 2
outer is still 1
big
```

A `let` binds a name, and the binding is immutable unless it says `mut`. A
second `let` of the same name *shadows* the first, and may give it another
type — which is how `spaces` goes from a string to its length without a second
name.

`do:` opens a block of statements. It is the mark for "this is a block", and
you will meet it wherever a block is needed but no keyword has already said
so: as a scope of its own, as a closure's body, as a value.

Assigning to a binding that never said `mut` is refused before the program
runs:

```rust harsh
fn main$:
    let x = 1
    x = 2
    println! "{x}"
```

```text
error[E0384]: cannot assign twice to immutable variable `x`
 --> immutable.hrs:3:5
  |
2 |     let x = 1;
  |         -
  |         |
  |         first assignment to `x`
  |         help: consider making this binding mutable: `mut x`
3 |     x = 2;
  |     ^^^^^ cannot assign twice to immutable variable

error: aborting due to previous error

For more information about this error, try `rustc --explain E0384`.
```

## 4.2 Types

```rust harsh
fn main$:
    // An annotation names the type; otherwise it is inferred.
    let n: u8 = 200
    let guessed = 200        // i32 by default

    // A cast is explicit, with `as`.
    let wide = n as u32 + 1
    let narrowed = 300i32 as u8      // wraps: 300 - 256
    println! "{n} {guessed} {wide} {narrowed}"

    // A float to an integer truncates towards zero.
    println! "{}" (2.9f64 as i32)

    // An alias is another name for a type, not a new type.
    let total: Meters = 5
    println! "{total}"

// Aliases are items, so they live outside the function too.
type Meters = u32
```

```text
200 200 201 44
2
5
```

Every value has a type, written after `:` when it is not obvious. `as` casts,
truncating or wrapping rather than guessing. `type` makes an alias — another
name for the same type, not a new one.

Note where the `type` item sits in that program: **after** the function that
uses it. Items are not statements, and their order does not matter.


# 5. Flow control

## 5.1 if and else

```rust harsh
fn main$:
    let n = 7

    // `if` is an expression, so it has a value.
    let size =
        if n < 5:
            "small"
        else if n < 10:
            "medium"
        else:
            "large"
    println! "{size}"

    // On one line, the body follows the colon.
    let parity = if n % 2 == 0: "even" else: "odd"
    println! "{parity}"
```

```text
medium
odd
```

`if` is an expression, so it can be the value of a `let`. Written over several
lines, the opener ends its line and the body goes beneath; written on one
line, the body follows the colon.

Each `else` answers the nearest open `if`.

## 5.2 Loops

```rust harsh
fn main$:
    // `loop` runs until something breaks out of it -- and `break` may
    // carry a value, which makes the loop an expression.
    let mut n = 0
    let stopped_at =
        loop:
            n += 1
            if n == 5:
                break n * 10
    println! "stopped at {stopped_at}"

    // `while` tests first.
    let mut count = 3
    while count > 0:
        print! "{count} "
        count -= 1
    println! "go"

    // `for` walks anything iterable; `..` excludes the end, `..=` includes it.
    for i in 1..4:
        print! "{i} "
    for i in 1..=3:
        print! "{i} "
    println! ""

    // A label lets `break` leave the outer loop.
    'rows: for r in 0..3:
        for c in 0..3:
            if r * c == 4:
                break 'rows
            print! "{r}{c} "
    println! ""
```

```text
stopped at 50
3 2 1 go
1 2 3 1 2 3 
00 01 02 10 11 12 20 21
```

`loop`, `while` and `for`, all with their bodies beneath a `:`. Two things are
worth pointing out.

`break` may carry a value, which makes `loop` an expression — that is where
`stopped_at` comes from. And a loop may be labelled, `'rows:`, so that `break`
can leave an outer loop rather than the one it stands in.

## 5.3 match

```rust harsh
fn describe (n: i32) -> String:
    match n\
        0 => String.from "zero"
        1 | 2 => String.from "a couple"
        3..=9 => String.from "a few"
        _ if n < 0 => String.from "negative"
        other => format! "{other}, which is a lot"

fn main$:
    for n in [0, 2, 5, -1, 40]:
        println! "{}" (describe n)

    // Destructuring in a match arm.
    let point = (3, 0)
    let where_ =
        match point\
            (0, 0) => "the origin"
            (_, 0) => "the x axis"
            (0, _) => "the y axis"
            _ => "somewhere else"
    println! "{where_}"
```

```text
zero
a couple
a few
negative
40, which is a lot
the x axis
```

`match x\` opens the arms with `\`, because they are a specification block:
Rust writes them with braces and commas, and `\` is how Harsh spells that.
The arms go beneath, or inline after the `\`.

An arm's pattern is read exactly as an application is — `Some n`,
`Event.Click x y` — which is the rule that **matching is constructing**, used
backwards. A guard is an `if` after the pattern, and `_` catches the rest.

## 5.4 if let, let else, while let

```rust harsh
fn main$:
    let some: Option<i32> = Some 7
    let none: Option<i32> = None

    // `if let` runs the block only when the pattern fits.
    if let Some n = some:
        println! "got {n}"

    // With an `else` for when it does not.
    if let Some n = none:
        println! "got {n}"
    else:
        println! "got nothing"

    // `let ... else` binds or leaves: the `else` block must not fall through.
    let Some value = some else:
        println! "nothing to do"
        return
    println! "value is {value}"

    // `while let` keeps going as long as the pattern fits.
    let mut stack = vec! 1 2 3
    while let Some top = stack <- pop$:
        print! "{top} "
    println! ""
```

```text
got 7
got nothing
value is 7
3 2 1
```

Three shorthands for the same idea: run this only if the pattern fits.

`let … else:` is the one to reach for when the rest of the function makes no
sense without the value — the `else` block must leave, so after it the binding
is simply there.


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


# 7. What Harsh adds

Everything so far has been Rust with the braces taken out. This page is the
part that is Harsh's own.

## 7.1 The pipes

```rust harsh
fn sub (a: i32) (b: i32) (c: i32) -> i32:
    a - b - c

fn double n: i32 -> i32:
    n * 2

fn main$:
    // `|>` fills a function's parameters from the left.
    println! "{}" (10 5 1 |> sub)

    // Fewer arguments than it takes: what comes back is a closure waiting
    // for the rest. This is partial application, and it is flat -- no
    // nesting, no `move |x| move |y|`.
    let from_ten = 10 |> sub
    println! "{}" (from_ten 5 1)

    // `<|` fills from the right instead.
    let take_one = sub <| 1
    println! "{}" (take_one 10 5)

    // Both at once: `1 |> sub <| 3` puts 1 first and 3 last.
    let middle = 10 |> sub <| 1
    println! "{}" (middle 5)

    // Chained, a pipe reads left to right: the value, then what happens next.
    println! "{}" (3 |> double |> double)
```

```text
4
4
4
4
12
```

`|>` fills a function's parameters from the left, `<|` from the right. When
the count reaches what the function takes, it is simply a call. When it falls
short, what comes back is a closure waiting for the rest — partial
application, and a flat one: a single closure over the remaining parameters,
not a nest of them.

The arity comes from the declaration, which `hrs` has already read, so nothing
has to be spelled out at the call.

Left of `|>` is a run of atoms, all of them arguments. To pipe the *result* of
an application, isolate it: `(f a) |> g`.

## 7.2 Partial application

```rust harsh
fn label (prefix: &str) (name: &str) (suffix: &str) -> String:
    format! "{prefix}{name}{suffix}"

fn main$:
    // `|>` fills the parameters from the left.
    println! "{}" ("Dr. " "Ada" "," |> label)
    // `<|` fills them from the right: this waits for a prefix.
    let with_phd = label <| "Ada" " PhD"
    println! "{}" (with_phd "Prof. ")
    // Both together leave a hole in the middle: a function of the name.
    let formal = "Dr. " |> label <| "."
    println! "{}" (formal "Ada")
    println! "{}" (formal "Grace")
```

```text
Dr. Ada,
Prof. Ada PhD
Dr. Ada.
Dr. Grace.
```

Give a function fewer values than it takes and the result is a function
waiting for the rest. `|>` fills parameters from the left, `<|` from the
right, and both together leave a hole in the middle: `"Dr. " |> label <| "."`
is a function of the name alone. There is no placeholder; the hole is what is
left. Chapter 14 of the Book has the whole story.

## 7.3 Macros

Harsh's macros are organised by who wrote them: in Harsh or in Rust -- the
mark says so, `name~` or `name!` -- and yours or imported from a library.
The Book's sections 23.5 to 23.8 take each part in full, and follow the Rust
Book's own chapter on macros.

### Harsh macros (~)

Your own, written in Harsh.

#### Declarative macros

```rust harsh
// A Harsh macro is matched and expanded by `hrs`, in Harsh, before anything
// is transpiled. The mark is `~`.
macro_rules~ greet
    (($who:expr)) => do:
        println! "hello, {}" $who

// A repetition takes `*`, `+` or `?`, exactly as Rust's does.
macro_rules~ list_of
    ($( ($x:expr) )*) => do: [$( $x ),*]

fn main$:
    greet~ "world"

    // The arguments are a token stream: everything to the end of the line,
    // and every line indented beneath it.
    let small = list_of~ 1 2 3
    let bigger =
        list_of~ 1 2 3
                     4 5 6
    println! "{:?} {:?}" small bigger

    // To chain on the result, isolate the call.
    println! "{}" ((list_of~ 1 2 3) <- len$)
```

```text
hello, world
[1, 2, 3] [1, 2, 3, 4, 5, 6]
3
```

`macro_rules~` defines a macro that `hrs` expands itself, in Harsh, before
anything is transpiled — so the generated Rust holds no macro at all, and what
a macro produces is Harsh you could have written by hand.

A call passes a **token stream**, not arguments: everything after `name~` to
the end of the line, and every line indented deeper beneath it. Nothing in
that stream is interpreted — a comma is the DSL's, a `\` is the DSL's — which
is why a macro can invent a grammar of its own. The stream ends at the close
of a group the call sits inside, so `(list_of~ 1 2 3) <- len$` chains on the
result.

#### Procedural macros

Programs, written in Harsh in a crate of their own, mirroring Rust's three
forms; each needs two crates to show, so the Book teaches them (23.5).

##### Custom derive Macros

A `pub fn` marked `#[proc_macro_derive~ Describe]`, called `#[derive~ Describe]`
above a `struct`, an `enum` or a `union`: it receives the item and adds code
after it.

##### Attribute-Like Macros

`#[proc_macro_attribute~]` on a `pub fn` taking two streams, the attribute's
arguments and the item, called `#[route~ GET "/"]` or `#[route~]`, and
replacing the item (the Book's 23.5 has the Rust Book's `route` example).

##### Function-Like Macros

A `pub fn` marked `#[proc_macro~]`, called like a declarative macro,
`hello_macro~ world`: it receives `world` and returns the Harsh that takes the
call's place.

### Rust macros (!)

Your own, written in Rust, still supported (23.6).

#### Declarative macros

A `macro_rules!` in a Harsh file is a zone of Rust, copied as written; its
calls are Harsh, `square! n`.

#### Procedural macros

A Rust proc-macro crate beside your Harsh crates, brought in with `use`.

##### Custom derive Macros

`proc_macro_derive`, called `#[derive Name]`; the Book's 23.6 has the Rust
Book's `HelloMacro` used from Harsh.

##### Attribute-Like Macros

`proc_macro_attribute`, two streams, called `#[route GET "/"]` -- Rust's
`#[route(GET, "/")]` -- replacing the item.

##### Function-Like Macros

`proc_macro`, called `name! args`.

### Harsh DSLs (~)

Imported from Harsh libraries, Harsh throughout; the library's guide is the
reference. The prelude's `g~` and `m~` are the first (pages 16 and 17).

### Rust DSLs (!)

Imported from Rust libraries, called by Harsh's rules: `vec! 1 2 3`,
`#[derive Debug Serialize]`, `#[tokio.main]`; a stream that is a language of
its own goes in braces, with Harsh in holes, `@: … :@` (23.8).

## Precedence

Application binds tighter than anything but an atom: `add 1 2 * 10` adds,
then multiplies. The chain `<-` comes next, then `?` and the prefix operators
— so `!v <- is_empty$` is "not empty", and `(*r) <- len$` dereferences `r`
before calling. Rust's operators follow in Rust's order, then the pipes. A
negative or dereferenced argument is parenthesised: `f (-1)`, `f (*x)`. The
full table is the Book's appendix *Precedence*.

```rust harsh
struct Switch<'a>
    flag: &'a mut bool

fn add (a: i32) (b: i32) -> i32:
    a + b

fn double (x: i32) -> i32:
    x * 2

fn main$:
    let v = vec! 3 1 2
    // Application first: `add 1 2`, then `* 10`.
    println! "{}" (add 1 2 * 10)
    // A negative argument is parenthesised.
    println! "{}" (double (-4))
    // A chain before an operator: the length, plus one.
    println! "{}" (v <- len$ + 1)
    // A prefix operator takes the whole chain: not (v is empty).
    println! "{}" (!v <- is_empty$)
    // An application before a chain: `add 1 2`, then its method.
    println! "{}" (add 1 2 <- pow 2)
    // Dereference first, then the method: parenthesised.
    let r = &v
    println! "{}" ((*r) <- len$)
    // Through a field: `*s <- flag` is the bool the field points to.
    let mut on = false
    let mut s = Switch\ flag = &mut on
    *s <- flag = true
    println! "{on}"
    // Pipes, left to right; several values fill several parameters.
    println! "{}" (3 |> double |> double)
    println! "{}" (1 2 |> add)
    // To pipe a call's result, parenthesise the call.
    println! "{}" ((add 1 2) |> double)
    // Fewer values than parameters: a partial application.
    let add10 = 10 |> add
    println! "{}" (add10 5)
    // The backward pipe, right to left.
    println! "{}" (double <| double <| 5)
```

```text
30
-8
4
true
9
3
true
12
3
6
15
20
```


# 8. Ownership and borrowing

This is the part of Rust that Harsh keeps entirely. Nothing here is a Harsh
idea; the only thing to learn is how the spellings look.

## 8.1 Moving

```rust harsh
fn consume (s: String) -> usize:
    s <- len$

fn main$:
    // A `String` owns its text. Passing it hands the ownership over.
    let a = String.from "hello"
    let n = consume a
    println! "{n}"
    // `a` is gone here: it was moved into `consume`.

    // `clone` makes a second owner.
    let b = String.from "world"
    let c = b <- clone$
    println! "{b} {c}"

    // Small copyable values are copied instead of moved.
    let x = 5
    let y = x
    println! "{x} {y}"
```

```text
5
world world
5 5
```

A value that owns something — a `String` owns its text — has exactly one
owner. Passing it to a function hands the ownership over, and the old name
cannot be used afterwards. `clone$` makes a second owner when you want one,
and small copyable values such as integers are copied rather than moved.

Using a name after it has been moved is refused before the program runs:

```rust harsh
fn consume (s: String):
    println! "{s}"

fn main$:
    let a = String.from "hello"
    consume a
    println! "{a}"
```

```text
error[E0382]: borrow of moved value: `a`
 --> use_after_move.hrs:8:15
  |
6 |     let a = String::from("hello");
  |         - move occurs because `a` has type `String`, which does not implement the `Copy` trait
7 |     consume(a);
  |             - value moved here
8 |     println!("{a}")
  |               ^^^ value borrowed here after move
  |
note: consider changing this parameter type in function `consume` to borrow instead if owning the value isn't necessary
 --> use_after_move.hrs:1:15
  |
1 | fn consume(s: String) {
  |    -------    ^^^^^^ this parameter takes ownership of the value
  |    |
  |    in this function
  = note: this error originates in the macro `$crate::format_args_nl` which comes from the expansion of the macro `println` (in Nightly builds, run with -Z macro-backtrace for more info)
help: consider cloning the value if the performance cost is acceptable
  |
7 |     consume(a.clone());
  |              ++++++++

error: aborting due to previous error

For more information about this error, try `rustc --explain E0382`.
```

The error arrives on the Harsh line that caused it, not on the generated Rust.

## 8.2 Borrowing

```rust harsh
// A borrow reads without taking ownership.
fn length (s: &String) -> usize:
    s <- len$

// A mutable borrow may change what it points at.
fn shout (s: &mut String):
    s <- push_str "!"

fn main$:
    let mut greeting = String.from "hi"

    // `&` is an operator, so a borrowed argument is isolated.
    println! "{}" (length (&greeting))

    shout (&mut greeting)
    println! "{greeting}"

    // Many readers, or one writer, never both at once.
    let r1 = &greeting
    let r2 = &greeting
    println! "{r1} {r2}"

    let w = &mut greeting
    w <- push_str "?"
    println! "{greeting}"
```

```text
2
hi!
hi! hi!
hi!?
```

`&x` borrows, `&mut x` borrows so it can change. The rule is one writer *or*
any number of readers, never both, and the compiler checks it.

The Harsh part is only the parentheses: `&` is an operator, so a borrowed
argument is isolated — `length (&greeting)`, `shout (&mut greeting)`.

## 8.3 Slices

```rust harsh
// A slice borrows a run of something rather than the whole of it.
fn first_word (s: &str) -> &str:
    let bytes = s <- as_bytes$
    for (i, &b) in bytes <- iter$ <- enumerate$:
        if b == b' ':
            return &s[0..i]
    s

fn main$:
    let sentence = String.from "hello wide world"
    println! "{}" (first_word (&sentence))

    let numbers = [1, 2, 3, 4, 5]
    let middle = &numbers[1..4]
    println! "{:?} {}" middle (middle <- len$)
```

```text
hello
[2, 3, 4] 3
```

A slice borrows a run of a value: `&s[0..i]` of a string, `&numbers[1..4]` of
an array. Note `&s[0..i]` needs no extra parentheses when it is a `return`
value, but would as an argument, by the same operator rule.


# 9. Generics and traits

## 9.1 Generics

```rust harsh
// A generic function: one definition, many types.
fn largest<T: PartialOrd + Copy> (items: &[T]) -> T:
    let mut best = items[0]
    for &item in items <- iter$:
        if item > best:
            best = item
    best

// A generic type.
struct Pair<T>
    left: T
    right: T

impl<T: std.fmt.Display> Pair<T>
    fn show (&self):
        println! "({}, {})" (self <- left) (self <- right)

fn main$:
    println! "{}" (largest (&[1, 7, 3]))
    println! "{}" (largest (&[1.5, 0.2]))

    let p = Pair\ left = "a", right = "b"
    p <- show$
```

```text
7
1.5
(a, b)
```

Generic parameters sit in `<>` after the name, with their bounds, exactly as
in Rust. A bound long enough to be awkward goes in a bracketed `where` clause, which is
Harsh's own spelling:

```rust harsh
// A bound long enough to be awkward goes in a bracketed `where` clause.
// Brackets suppress layout, so the clause may span lines and its own `:`
// cannot be mistaken for a block opener.
fn describe_all<T> (items: &[T]) -> String
    [where T: std.fmt.Debug + Clone]:
    format! "{:?}" (items <- to_vec$)

fn main$:
    println! "{}" (describe_all (&[1, 2, 3]))
    println! "{}" (describe_all (&["a", "b"]))
```

```text
[1, 2, 3]
["a", "b"]
```

The brackets are there because a bound contains a `:`, and a `:` at the end of
a line opens a block. Brackets suppress layout, so the clause may span lines
and its colons cannot be mistaken for openers.

## 9.2 Traits

```rust harsh
trait Greet
    // A method with no body: whoever implements the trait writes it.
    fn name (&self) -> String

    // One with a body is a default, and may be overridden.
    fn hello (&self) -> String:
        format! "hello, {}" (self <- name$)

struct Cat
struct Dog\ called: String

impl Greet for Cat
    fn name (&self) -> String:
        String.from "cat"

impl Greet for Dog
    fn name (&self) -> String:
        self <- called <- clone$

    fn hello (&self) -> String:
        format! "woof, says {}" (self <- name$)

// Taking any type that implements the trait.
fn greet_all (who: &[&dyn Greet]):
    for one in who <- iter$:
        println! "{}" (one <- hello$)

fn main$:
    let d = Dog\ called = String.from "Rex"
    greet_all (&[&Cat, &d])
```

```text
hello, cat
woof, says Rex
```

A trait declares methods; an `impl … for …` provides them. Both take no mark:
the items follow the header. A method with a body in the trait is a default.

`&dyn Greet` is a trait object — any type implementing the trait, decided at
run time. `impl Greet` as a parameter type is the other way, decided at
compile time.

## 9.3 Operators are traits

```rust harsh
use std.ops.Add

#[derive Debug Clone Copy]
struct Point
    x: i32
    y: i32

// Operators are traits, so `+` is written by implementing `Add`.
impl Add for Point
    type Output = Point

    fn add (self) (other: Point) -> Point:
        Point\ x = (self <- x) + (other <- x), y = (self <- y) + (other <- y)

fn main$:
    let a = Point\ x = 1, y = 2
    let b = Point\ x = 10, y = 20
    println! "{:?}" (a + b)
```

```text
Point { x: 11, y: 22 }
```

`+` is `Add`, `*` is `Mul`, `==` is `PartialEq`. Implementing the trait is
what gives a type the operator, and `type Output = …` names what comes back.

This is why Harsh does not need operator syntax of its own for matrices or
money or anything else: the operator is already a trait, and the trait is
already implementable.


# 10. Errors

Harsh has no exceptions, because Rust has none. A function that can fail says
so in its return type, and the caller has to deal with it.

## 10.1 Option

```rust harsh
fn first_even (xs: &[i32]) -> Option<i32>:
    for &x in xs <- iter$:
        if x % 2 == 0:
            return Some x
    None

fn main$:
    println! "{:?}" (first_even (&[1, 3, 4]))
    println! "{:?}" (first_even (&[1, 3]))

    // Reading what is inside, safely.
    match first_even (&[5, 6])\
        Some n => println! "found {n}"
        None => println! "found nothing"

    // Or with a default.
    println! "{}" (first_even (&[1]) <- unwrap_or 0)

    // `?` on an Option leaves the function early when it is None.
    println! "{:?}" (doubled_first (&[4, 5]))
    println! "{:?}" (doubled_first (&[1, 5]))

fn doubled_first (xs: &[i32]) -> Option<i32>:
    let n = first_even xs?
    Some (n * 2)
```

```text
Some(4)
None
found 6
0
Some(8)
None
```

`Option<T>` is either `Some value` or `None`. `match` reads it; `unwrap_or`
gives a default; `?` leaves the function early when it is `None`.

Note `Some n` and `Some (n * 2)`: constructing a variant is an application
like any other, so the payload is juxtaposed and an expression is isolated.

## 10.2 Result

```rust harsh
fn parse_age (text: &str) -> Result<u32, String>:
    // `map_err` turns one error into another.
    let n: u32 =
        text <- trim$
             <- parse$
             <- map_err (|e| format! "{text:?} is not a number ({e})")?
    if n > 150:
        return Err (format! "{n} is too old")
    Ok n

fn main$:
    for text in ["31", " 7 ", "abc", "900"]:
        match parse_age text\
            Ok n => println! "age {n}"
            Err why => println! "rejected: {why}"
```

```text
age 31
age 7
rejected: "abc" is not a number (invalid digit found in string)
rejected: 900 is too old
```

`Result<T, E>` is `Ok value` or `Err why`. `?` is the same shorthand: it
returns the error to the caller, converting it if the types allow.

The chain in `parse_age` is worth reading slowly —
`text <- trim$ <- parse$ <- map_err (…)?` — because it shows the two
mechanisms side by side: `<-` walks the chain, and `?` at the end takes the
error out of it.

## 10.3 An error type of your own

```rust harsh
use std.fmt

// An error of one's own is a type with a `Display`.
#[derive Debug]
enum ConfigError
    Missing String
    Bad
        key: String
        value: String

impl fmt.Display for ConfigError
    fn fmt (&self) (f: &mut fmt.Formatter) -> fmt.Result:
        match self\
            ConfigError.Missing key => write! f "{key} is missing"
            ConfigError.Bad\ key, value => write! f "{key} cannot be {value}"

// An impl with no items of its own keeps Rust's braces: there is no body
// to put beneath it.
impl std.error.Error for ConfigError {}

fn port (raw: Option<&str>) -> Result<u16, ConfigError>:
    let Some text = raw else:
        return Err (ConfigError.Missing (String.from "port"))
    text <- parse$ <- map_err |_|:
        ConfigError.Bad\ key = String.from "port", value = String.from text

fn main$:
    for raw in [Some "8080", Some "wat", None]:
        match port raw\
            Ok n => println! "port {n}"
            Err e => println! "error: {e}"
```

```text
port 8080
error: port cannot be wat
error: port is missing
```

An error is a type with a `Display`. Giving it `std.error.Error` as well lets
it travel as a `Box<dyn Error>`, which is what a program that has several
kinds of failure usually returns.

Two Harsh details in that program. An `impl` with no items of its own keeps
Rust's braces — `impl std.error.Error for ConfigError {}` — since there is no
body to put beneath it. And a record variant is matched with the `\` it was
declared with: `ConfigError.Bad\ key, value`.


# 11. Collections

## 11.1 Vectors

```rust harsh
fn main$:
    // `vec!` builds one; brackets are Rust's and pass through.
    let mut v = vec! 1 2 3
    v <- push 4

    println! "{:?} {}" v (v <- len$)
    println! "{}" v[0]

    // `get` returns an Option rather than panicking.
    println! "{:?} {:?}" (v <- get 1) (v <- get 99)

    // Iterating, and building a new one.
    let doubled: Vec<i32> =
        v <- iter$
          <- map (|n| n * 2)
          <- collect$
    println! "{:?}" doubled

    let total: i32 = v <- iter$ <- sum$
    println! "{total}"
```

```text
[1, 2, 3, 4] 4
1
Some(2) None
[2, 4, 6, 8]
10
```

`vec! 1 2 3` builds one: a macro is applied like a function, one argument per
value. `vec! { 0; 4 }` repeats one value, four times -- that stream is not a
list of values, so it goes in braces and reaches the macro as written.

`v[0]` indexes and panics if it is out of range; `v <- get 0` returns an
`Option` instead.

## 11.2 Maps

```rust harsh
use std.collections.HashMap

fn main$:
    let mut ages = HashMap.new$
    ages <- insert "Ada" 36
    ages <- insert "Alan" 41

    // Looking up gives an Option.
    match ages <- get "Ada"\
        Some n => println! "Ada is {n}"
        None => println! "no Ada"

    // `entry` inserts only when the key is absent.
    ages <- entry "Grace" <- or_insert 45
    ages <- entry "Ada" <- or_insert 0

    // A HashMap has no order, so sort before printing.
    let mut pairs: Vec<_> = ages <- iter$ <- collect$
    pairs <- sort$
    println! "{:?}" pairs
```

```text
Ada is 36
[("Ada", 36), ("Alan", 41), ("Grace", 45)]
```

`HashMap.new$`, then `insert`, `get`, `entry`. A map has no order, so a
program that prints one should sort first — which is why the last two lines
collect into a `Vec` and sort it.

## 11.3 Strings

```rust harsh
fn main$:
    // `&str` borrows; `String` owns.
    let borrowed = "hello"
    let mut owned = String.from borrowed
    owned <- push_str ", world"
    println! "{owned}"

    // Slicing is by bytes, so it must fall on a character boundary.
    println! "{}" (&owned[0..5])

    // Splitting, trimming, joining.
    let csv = " a,b,c "
    let parts: Vec<&str> =
        csv <- trim$
            <- split ','
            <- collect$
    println! "{:?}" parts
    println! "{}" (parts <- join " + ")

    // Characters, not bytes, when you mean characters.
    println! "{}" ("héllo" <- chars$ <- count$)
```

```text
hello, world
hello
["a", "b", "c"]
a + b + c
5
```

`&str` borrows text; `String` owns it. Indexing is by byte and must land on a
character boundary, so counting characters is `chars$ <- count$` rather than
`len$`.


# 12. Modules

```rust harsh
// A module groups items. Its body follows the header, with no mark.
mod shapes
    // Items are private to the module unless they say `pub`.
    pub struct Circle
        pub radius: f64

    impl Circle
        pub fn area (&self) -> f64:
            3.14159 * (self <- radius) * (self <- radius)

    pub mod units
        pub const NAME: &str = "cm"

// `use` brings a path into scope; `.` is the path separator.
use shapes.Circle
use shapes.units.NAME

fn main$:
    let c = Circle\ radius = 2.0
    println! "{:.2} square {}" (c <- area$) NAME

    // Or written out in full, without a `use`.
    println! "{}" shapes.units.NAME
```

```text
12.57 square cm
cm
```

A `mod` groups items, and its body follows the header with no mark, like every
other item body. Everything inside is private unless it says `pub`.

`.` is the path separator, so Rust's `shapes::units::NAME` is
`shapes.units.NAME`, and `use shapes.Circle` brings it into scope. A grouped
import is written with parentheses: `use std.io.(Read, Write)`.

Privacy is checked, so reaching a field that is not `pub` is refused:

```rust harsh
mod counter
    pub struct Counter
        // Not `pub`: only this module may touch it.
        value: i32

    impl Counter
        pub fn new$ -> Counter:
            Counter\ value = 0

fn main$:
    let c = counter.Counter.new$
    println! "{}" (c <- value)
```

```text
error[E0616]: field `value` of struct `Counter` is private
  --> privacy.hrs:18:22
   |
18 |     println!("{}", c.value)
   |                      ^^^^^ private field

error: aborting due to previous error

For more information about this error, try `rustc --explain E0616`.
```

In a real project the modules live in files rather than in one: `src/main.hrs`
plus `src/shapes.hrs`, and `mod shapes` in the main file. `hrs build`
transpiles the tree and cargo compiles it.


# 13. Threads and channels

```rust harsh
use std.thread
use std.sync.mpsc

fn main$:
    // A thread takes a closure. `move` hands it what it captures.
    let handle = thread.spawn move ||:
        let mut total = 0
        for i in 1..=10:
            total += i
        total

    // `join` waits for it and gives back what it returned.
    let total = handle <- join$ <- unwrap$
    println! "the thread counted {total}"

    // A channel carries values between threads.
    let (tx, rx) = mpsc.channel$
    for id in 0..3:
        let tx = tx <- clone$
        // Bound to `_`: the block's last expression is its value, and here
        // the handle is not wanted, so the binding discards it.
        let _ = thread.spawn move ||:
            tx <- send (id * id) <- unwrap$
    drop tx

    let mut got: Vec<i32> = rx <- iter$ <- collect$
    got <- sort$
    println! "{:?}" got
```

```text
the thread counted 55
[0, 1, 4]
```

`thread.spawn` takes a closure, and `move` hands it what it captured. Note the
spelling: `thread.spawn move ||:` with the body beneath — a closure is a
trailing argument that owns its block, and `move` before the bars is part of
it.

`join$` waits and gives back whatever the closure returned.

A channel is a pair: `tx` to send, `rx` to receive. Every thread gets its own
clone of `tx`, and the original is dropped so the receiver knows when the last
one is gone.

One Harsh detail worth the line it costs: `let _ = thread.spawn …`. A block's
last expression is its value, and here the handle is not wanted, so binding it
to `_` discards it. Without the binding the loop body would be trying to
return a `JoinHandle`, and rustc would say so.


# 14. Tests

```rust harsh
fn add (a: i32) (b: i32) -> i32:
    a + b

fn divide (a: i32) (b: i32) -> Result<i32, String>:
    if b == 0:
        return Err (String.from "divide by zero")
    Ok (a / b)

// A test module is compiled only when testing.
#[cfg (test)]
mod tests
    use super.*

    #[test]
    fn addition_works$:
        assert_eq! (add 2 2) 4

    #[test]
    fn division_reports_its_error$:
        assert! (divide 1 0 <- is_err$)
        assert_eq! (divide 10 2) (Ok 5)

    #[test]
    fn a_message_of_your_own$:
        let n = add 2 2
        assert! (n > 0) "expected a positive number, got {}" n
```

```text
$ hrs test
running 3 tests
test tests::a_message_of_your_own ... ok
test tests::addition_works ... ok
test tests::division_reports_its_error ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished
```

A test is a function marked `#[test]`, usually inside a
`#[cfg (test)] mod tests`, which is compiled only when testing. `use super.*`
brings in what the file above defines.

Three things read differently from Rust, and all three are the ordinary rules:

- `#[cfg (test)]` — an attribute is an application, so its argument is
  isolated in parentheses when it is not a single atom.
- `fn addition_works$` — a test takes no parameters, so it takes `$`.
- `assert_eq! (add 2 2) 4` — a macro's arguments juxtapose, and `add 2 2` is
  an application, so it is isolated.

`hrs test` runs them, passing the arguments through to cargo.


# 15. The tools

Harsh is a transpiler, so the tools are a thin layer over cargo's.

    hrs new NAME          a project: Cargo.toml, src/main.hrs, .gitignore
    hrs run               transpile what changed, then cargo run
    hrs build             transpile, then cargo build
    hrs test              transpile, then cargo test
    hrs check             transpile, then cargo check
    hrs lint              the same, through clippy, with its notes remapped
    hrs fmt               lay out the Harsh in the canonical form
    hrs watch             rebuild as files change
    hrs expand FILE       the file with its Harsh macros unfolded
    hrs export DIR        the project as a plain Rust crate

A project keeps its Harsh in `src/**.hrs`. The generated Rust goes to
`target/hrs/`, and `Cargo.toml` points at it:

    [[bin]]
    name = "myapp"
    path = "target/hrs/main.rs"

One manifest, ordinary dependencies, nothing extra to ignore — cargo already
ignores `/target`.

```rust harsh
// This is what `hrs new` writes for you, and what `hrs run` builds.
fn main$:
    let args: Vec<String> = std.env.args$ <- collect$
    let name =
        if (args <- len$) > 1:
            args[1] <- clone$
        else:
            String.from "world"
    println! "hello, {name}"
```

```text
hello, world
```

Every diagnostic comes back on the `.hrs` line and column that produced it,
not on the generated Rust, because the transpiler writes a source map and the
driver runs cargo's output back through it. That is true of rustc's errors,
its suggestions, and clippy's lints alike.

`hrs-from` goes the other way, turning Rust into Harsh — which is how an
existing crate is brought across.


# 16. Generator comprehensions

## 16.1 g~

```rust harsh
fn main$:
    // A comprehension: the value, then where it comes from.
    let squares: Vec<i32> = (g~ x * x for x in 1..6) <- collect$
    println! "{:?}" squares

    // `if`s filter; each belongs to the `for` it follows.
    let odd_big: Vec<i32> = (g~ x for x in 0..20 if x % 2 == 1 if x > 10) <- collect$
    println! "{:?}" odd_big

    // Several `for`s nest, and an inner condition sees the outer name.
    let pairs: Vec<(i32, i32)> = (g~ (x, y) for x in 1..4 for y in 1..4 if y > x) <- collect$
    println! "{:?}" pairs
```

```text
[1, 4, 9, 16, 25]
[11, 13, 15, 17, 19]
[(1, 2), (1, 3), (2, 3)]
```

`g~` is Harsh's comprehension, and it is always available — it lives in the
prelude, so there is nothing to `use`. It reads the way the idea reads: the
value first, then `for` a pattern `in` an iterable, then any conditions.

Each `if` belongs to the `for` it follows, so a condition can use every name
bound up to that point — `if y > x` sees both — and a condition on the outer
loop filters before the inner one runs.

`g~` produces an iterator, and it is lazy: nothing is computed until something
asks for the values. `for` means what it means everywhere in the language, so
it takes the iterable by value; write `xs <- iter$` to borrow instead.

## 16.2 Conditions, and several `for`s

```rust harsh
fn main$:
    // `if`s filter, and every `if` belongs to the `for` before it.
    let picked: Vec<i32> = (g~ x for x in 0..30 if x % 3 == 0 if x % 2 == 1) <- collect$
    println! "{picked:?}"

    // A condition sees every name bound so far: `y > x` uses both.
    let rising: Vec<(i32, i32)> = (g~ (x, y) for x in 1..4 for y in 1..4 if y > x) <- collect$
    println! "{rising:?}"
```

```text
[3, 9, 15, 21, 27]
[(1, 2), (1, 3), (2, 3)]
```

Each `if` belongs to the `for` before it, so a condition sees every name
bound up to that point — `y > x` uses both — and runs as early as it can.

## 16.3 Where a condition may go

```rust harsh
fn main$:
    let triples =
        list~ (a, b, c)
            for a in 1..20        // only a is available for the condition
            for b in a..20        // only a, b are available for the condition
            for c in b..20        // a, b, c are available for the condition
            if a * a + b * b == c * c
    println! "{triples:?}"
```

```text
[(3, 4, 5), (5, 12, 13), (6, 8, 10), (8, 15, 17), (9, 12, 15)]
```

Every `for` makes one more name available, and a condition can use only the
names bound up to its place — so a test goes after the last `for` whose name it
uses. Attached to `for a`, the same test cannot see `b`:

```rust harsh
fn main$:
    // The test needs b and c, but it is attached to `for a`,
    // where neither exists yet.
    let triples =
        list~ (a, b, c)
            for a in 1..20 if a * a + b * b == c * c
            for b in a..20
            for c in b..20
    println! "{triples:?}"
```

```text
error[E0425]: cannot find value `b` in this scope
 --> too_early.hrs:4:80
  |
4 | ....flat_map(move | a | ((true &&(a * a + b * b == c * c)).then(|| ((a .. 20).into_iter().flat_map(move | b | ((true).then(|| ((b .. 20)....
  |                                           ^ help: a local variable with a similar name exists: `a`

error[E0425]: cannot find value `b` in this scope
 --> too_early.hrs:4:84
  |
4 | ...t_map(move | a | ((true &&(a * a + b * b == c * c)).then(|| ((a .. 20).into_iter().flat_map(move | b | ((true).then(|| ((b .. 20).into...
  |                                           ^ help: a local variable with a similar name exists: `a`

error[E0425]: cannot find value `c` in this scope
 --> too_early.hrs:4:89
  |
4 | ...(move | a | ((true &&(a * a + b * b == c * c)).then(|| ((a .. 20).into_iter().flat_map(move | b | ((true).then(|| ((b .. 20).into_iter...
  |                                           ^ help: a local variable with a similar name exists: `a`

error[E0425]: cannot find value `c` in this scope
 --> too_early.hrs:4:93
  |
4 | ...e | a | ((true &&(a * a + b * b == c * c)).then(|| ((a .. 20).into_iter().flat_map(move | b | ((true).then(|| ((b .. 20).into_iter().f...
  |                                           ^ help: a local variable with a similar name exists: `a`

error: aborting due to 4 previous errors

For more information about this error, try `rustc --explain E0425`.
```

## 16.4 Collecting: list~, set~, dict~

```rust harsh
use std.collections.HashMap
use std.collections.HashSet

fn main$:
    // `g~` is lazy -- a plain iterator. The shorthands collect it.
    let v = list~ x * 10 for x in 1..4
    println! "{:?}" v

    let remainders: HashSet<i32> = set~ x % 3 for x in 0..10
    let mut seen: Vec<_> = remainders <- into_iter$ <- collect$
    seen <- sort$
    println! "{:?}" seen

    // `dict~` takes `key => value`.
    let lengths: HashMap<&str, usize> = dict~ w => (w <- len$) for w in ["sun", "moon"]
    let mut pairs: Vec<_> = lengths <- into_iter$ <- collect$
    pairs <- sort$
    println! "{:?}" pairs
```

```text
[10, 20, 30]
[0, 1, 2]
[("moon", 4), ("sun", 3)]
```

The shorthands collect a comprehension into a `Vec`, a `HashSet` or a
`HashMap`. `dict~` takes its entry as `key => value`. Or keep `g~` and let the
type decide — `let v: Vec<_> = (g~ …) <- collect$` — which is the same thing
written out.

## 16.5 Laying one out

```rust harsh
fn main$:
    // Every line indented beneath the call belongs to it, so a long one is
    // laid out one clause per line. Pythagorean triples:
    let triples =
        list~ (a, b, c)
            for a in 1..20
            for b in a..20
            for c in b..20 if a * a + b * b == c * c
    println! "{:?}" triples
```

```text
[(3, 4, 5), (5, 12, 13), (6, 8, 10), (8, 15, 17), (9, 12, 15)]
```

A call takes the rest of its line **and every line indented beneath it**, so a
comprehension with several clauses is written one clause per line. Nothing
changes but the layout: the same tokens reach the macro either way.

If you define a macro of your own called `g` (or `list`, `set`, `dict`), yours
wins in that file. The prelude's is still there as `hrs_std.g~`.

## 16.6 Lazy, and by value

```rust harsh
fn main$:
    // `g~` is an iterator: nothing runs until something asks for values.
    // So an endless source is fine, as long as something stops asking.
    let first: Vec<u64> =
        (g~ n * n for n in 1.. if n % 7 == 3) <- take 4 <- collect$
    println! "{first:?}"

    // `for` takes its iterable by value, as it does everywhere in Rust.
    // Borrow to keep the collection.
    let words = vec! (String.from "pipe") (String.from "matrix")
    let lengths: Vec<usize> = (g~ (w <- len$) for w in (words <- iter$)) <- collect$
    println! "{lengths:?} -- and still {words:?}"
```

```text
[9, 100, 289, 576]
[4, 6] -- and still ["pipe", "matrix"]
```

`g~` is an iterator: nothing runs until something asks, so an endless
source is fine when something stops asking. `for` takes its iterable by
value; give it a borrow, `(words <- iter$)`, to keep the collection.

## 16.7 What it stands for

```rust harsh
fn main$:
    let wanted: Vec<i32> =
        (g~ x * 10 + y
            for x in (0..6) if x % 2 == 0 if x > 0
            for y in (0..6) if y > x if y % 2 == 1
        ) <- collect$

    // The same by hand: one `flat_map` per `for`, each level's conditions
    // folded into one test, and the outer level flattened.
    let inner =
        |x: i32| (0..6) <- into_iter$
            <- flat_map (move |y| ((true && (y > x) && (y % 2 == 1)) <- then (|| x * 10 + y)))
    let written: Vec<i32> =
        (0..6) <- into_iter$
               <- flat_map (move |x| ((true && (x % 2 == 0) && (x > 0)) <- then (|| inner x)))
               <- flatten$
               <- collect$

    println! "{wanted:?} {}" (wanted == written)
```

```text
[23, 25, 45] true
```

A generator comprehension is a chain written another way: each `for` a
`flat_map`, its conditions folded into one test under `then`, every level but
the innermost flattened. The `flatten$` is part of the generator, so it is
never written by hand; the `collect$` is not, because a generator is lazy —
write it, or use `list~`. Chapter 15 of the Book shows the whole expansion.

```rust harsh
fn main$:
    let scores = vec! 72 45 91 60 88

    // A comprehension...
    let a: Vec<i32> = (g~ s + 5 for s in (scores <- iter$) if *s >= 60) <- collect$
    // ...and the chain it stands for.
    let b: Vec<i32> =
        scores <- iter$
               <- filter (|s| **s >= 60)
               <- map (|s| s + 5)
               <- collect$
    println! "{a:?} {}" (a == b)
```

```text
[77, 96, 65, 93] true
```


# 17. Matrices

Harsh's matrices are Julia's, for anyone who knows Julia: the same literal,
the same meaning for `*`, the same errors. They live in `hrs_std`, Harsh's
standard library, which a project adds to its dependencies once. The literals
themselves, `m~` and `v~`, need no `use`: they are part of the language.

## 17.1 Writing them

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
fn main$:
    // A matrix: spaces between entries, `;` between rows.
    let a = m~ [1 2 3; 4 5 6]
    println! "{a}"
    // Rows may be blocks of their own, stacked with `;`.
    let b = m~ [[1 2 3]; [4 5 6]]
    println! "{}" (a == b)

    // A vector is a column: commas, or one entry per line.
    let v = v~ [1.5, 2.5, 3.5]
    println! "{v}"

    // A bracket inside is a block. Side by side, columns make a matrix.
    let c = m~ [[1, 4] [2, 5] [3, 6]]
    println! "{}" (a == c)
```

```text
2×3 Matrix<i32>:
 1 2 3
 4 5 6
true
3-element Vector<f64>:
 1.5
 2.5
 3.5
true
```

Three rules make every matrix literal, and they compose. A **space** puts
things side by side. A **`;`** puts them one above another — and so does a
line break, so a matrix may be written one row per line, as it reads. A
**comma** makes the entries of a vector, which is a column.

A bracket inside the literal is a block built by the same rules, so
`[[1, 4] [2, 5] [3, 6]]` is three columns side by side — the same matrix as
`[1 2 3; 4 5 6]`. What the literal refuses, it refuses with Julia's reason:
`m~ [[1 2], [3 4]]` is two matrices in a vector, not one matrix, because a
comma never joins; and `v~ [1 2 3]` is a row, which is a matrix, not a vector.

A matrix prints its shape and its element type, as Julia does.

## 17.2 Arithmetic

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
fn main$:
    let a = m~ [1.0 2.0; 3.0 4.0]
    let b = m~ [0.0 1.0; 1.0 0.0]
    let x = v~ [1.0, 1.0]

    // One `*`, three meanings, chosen from what it multiplies.
    println! "{}" (&a * &b)        // the matrix product
    println! "{}" (2.0 * (&a + &b))  // scaling a sum
    println! "{}" (&a * &x)        // a matrix times a vector is a vector

    println! "{:?} {}" (a <- size$) (a <- transpose$)
```

```text
2×2 Matrix<f64>:
 2 1
 4 3
2×2 Matrix<f64>:
 2 6
 8 8
2-element Vector<f64>:
 3
 7
(2, 2) 2×2 Matrix<f64>:
 1 3
 2 4
```

`*` between two matrices is the matrix product; between a number and a matrix
it is scaling; between a matrix and a vector it gives a vector. The compiler
chooses, from what is being multiplied — nothing has to be spelled out.

The `&` is ownership, Rust's rule: `&a * &b` borrows both, so `a` and `b` are
still there on the next line. Without it, the product would consume them.

## 17.3 The identity

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
use hrs_std.(UniformScaling, I)

fn main$:
    let a = m~ [1 2; 3 4]
    let u = UniformScaling 2
    // As in Julia: `u` is 2 times an identity of whatever size is needed.
    println! "{}" (&a + u)
    println! "{}" (&a * u)
    println! "{}" (&a + I)
```

```text
2×2 Matrix<i32>:
 3 2
 3 6
2×2 Matrix<i32>:
 2 4
 6 8
2×2 Matrix<i32>:
 2 2
 3 5
```

`UniformScaling k` is `k` times an identity of whatever size the other side
needs, and `I` is the identity itself — Julia's own names.

## 17.4 Joining matrices

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
fn main$:
    let a = m~ [1 2; 3 4]
    // Any matrix is a block, so the literal joins matrices as it joins numbers.
    println! "{}" (m~ [(&a) (&a)])
    println! "{}" (m~ [(&a); (&a)])
```

```text
2×4 Matrix<i32>:
 1 2 1 2
 3 4 3 4
4×2 Matrix<i32>:
 1 2
 3 4
 1 2
 3 4
```

A block may be any matrix, so the literal that builds a matrix from numbers
also joins matrices: side by side with a space, stacked with a `;`. Each block
is isolated in parentheses, `(&a)`, as any argument with an operator is.

## 17.5 Solving

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
fn main$:
    // 2x + y = 5 and x + y = 3, as a matrix and a vector.
    let a = m~ [2.0 1.0; 1.0 1.0]
    let b = v~ [5.0, 3.0]
    // Julia's `a \ b`.
    let x = a <- solve (&b)
    println! "x = {}, y = {}" x[0] x[1]

    println! "det {}" (a <- det$)
    println! "{}" (a <- inv$)

    let v = v~ [3.0, 4.0]
    println! "length {}, dot {}" (v <- norm$) (v <- dot (&v))
```

```text
x = 2, y = 1
det 1
2×2 Matrix<f64>:
 1 -1
 -1 2
length 5, dot 25
```

`a <- solve (&b)` is Julia's `a \ b`: the `x` with `a * x == b`. `det$`,
`inv$`, `norm$` and `dot` carry Julia's names. Indexes start at 0, as for
every collection in Harsh.

`solve` and `inv` stop the program on a singular matrix, as Julia does.
When the matrix comes from data you have not checked, `try_solve` and
`try_inv` return a `Result` instead — `Err (LinAlgError.Singular)`, or
`Err (LinAlgError.DimensionMismatch msg)` — as `RefCell`'s `try_borrow`
pairs with `borrow`:

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
use hrs_std.LinAlgError

fn main$:
    let a = m~ [2.0 1.0; 1.0 3.0]
    let flat = m~ [1.0 2.0; 2.0 4.0]
    let b = v~ [5.0, 10.0]
    // Sure of the matrix: Julia's way -- the answer, or a panic.
    let x = a <- solve (&b)
    println! "x = {:.1} {:.1}" x[0] x[1]
    // Not sure: the caller decides what a failure means.
    for m in [&a, &flat]:
        match m <- try_solve (&b)\
            Ok x => println! "solved: {:.1} {:.1}" x[0] x[1]
            Err LinAlgError.Singular => println! "no single solution: the matrix is singular"
            Err e => println! "{e}"
    let short = v~ [1.0, 2.0, 3.0]
    if let Err e = a <- try_solve (&short):
        println! "{e}"
```

```text
x = 1.0 3.0
solved: 1.0 3.0
no single solution: the matrix is singular
DimensionMismatch: matrix has 2 rows, right-hand side has length 3
```

## 17.6 Fitting a line

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
use hrs_std.Matrix

fn main$:
    // Hours studied, and the score each student got.
    let hours = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    let score = v~ [52.0, 55.0, 61.0, 64.0, 70.0, 73.0]

    // The design matrix, one row per student: a 1 for the intercept,
    // then the hours. A comprehension builds the rows.
    let x = Matrix.from_rows (list~ (vec! 1.0 h) for h in hours)

    // Julia's `β = X \ y`: for a tall matrix, the least-squares fit.
    let beta = x <- solve (&score)
    println! "score = {:.2} + {:.2} × hours" beta[0] beta[1]

    let fitted = &x * &beta
    println! "residual {:.2}" ((&score - &fitted) <- norm$)
    println! "7 hours: {:.1}" (beta[0] + beta[1] * 7.0)
```

```text
score = 47.20 + 4.37 × hours
residual 1.76
7 hours: 77.8
```

With more rows than columns, `solve` returns the least-squares fit, as Julia's
`X \ y` does: linear regression in one line, with the design matrix built by
a comprehension. Chapter 16 of the Book explains each step.

## 17.7 Slicing

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
fn main$:
    let mut a = m~ [1 2 3; 4 5 6; 7 8 9]
    // The method copies, as Julia's `a[1:2, :]` does. `..` alone is Julia's `:`.
    println! "{}" (a <- slice (0..2) (..))
    // An axis taken by a number is dropped: a row is a vector.
    println! "{}" (a <- slice 1 (..))
    // A view borrows, as Julia's `view(a, 2:3, 1:2)`: nothing copied.
    let window = a <- view (1..) (..=1)
    println! "{}" window
    let corner = window <- copy$
    println! "{}" corner
    // An element by its index; a whole row through a view that writes.
    a[1, 1] = 50
    a <- view_mut 2 (..) <- fill 0
    println! "{}" a
```

```text
2×3 Matrix<i32>:
 1 2 3
 4 5 6
3-element Vector<i32>:
 4
 5
 6
2×2 view of Matrix<i32>:
 4 5
 7 8
2×2 Matrix<i32>:
 4 5
 7 8
3×3 Matrix<i32>:
 1 2 3
 4 50 6
 0 0 0
```

Julia writes `a[1:2, :]`. Harsh keeps its own ranges -- 0-based, the end left
out, as for every collection -- and `..` alone is Julia's `:`. There are two
ways to take a part. `a <- slice (0..2) (..)` *copies*, as Julia's `a[1:2, :]`
does. `a <- view (0..2) (..)` *borrows*, as Julia's `view(a, 1:2, :)`: a
window onto `a`, guarded by the borrow checker like any borrow, and a value you
can name, print or use in `.*`; `<- copy$` makes it a matrix of its own. An
axis taken by a number is dropped, so `slice 1 (..)` and `view 1 (..)` are a
row, read as a vector. To write, `a[1, 1] = 50` sets an element, and
`view_mut` is the view that writes through: `a <- view_mut 2 (..) <- fill 0`.

An index takes its axes with a comma, `a[i, j]`; the parenthesised `a[(i, j)]`
means the same and is what the comma stands for. An index is one element: a
part of a matrix is a `slice` or a `view`.

## 17.8 Broadcasting

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
fn relu (x: f64) -> f64:
    x <- max 0.0

fn main$:
    let a = m~ [1.0 -2.0; -3.0 4.0]
    let b = m~ [10.0 20.0; 30.0 40.0]
    let row = m~ [100.0 200.0]
    // `*` is the matrix product; `.*` multiplies element by element.
    println! "{}" (&a * &b)
    println! "{}" (&a .* &b)
    // Shapes stretch as in Julia: a number, a row, a column.
    println! "{}" (&a .* 2.0 .+ &row)
    // `f<>` applies `f` to each element: Julia's `f.(a)`.
    println! "{}" (relu<> a)
    println! "{}" (f64.powf<> a 2.0)
    println! "{}" ((|x, y| x > y)<> a b)
    // It pipes, and nothing above consumed `a`.
    println! "{}" (a |> relu<> |> f64.sqrt<>)
```

```text
2×2 Matrix<f64>:
 -50 -60
 90 100
2×2 Matrix<f64>:
 10 -40
 -90 160
2×2 Matrix<f64>:
 102 196
 94 208
2×2 Matrix<f64>:
 1 0
 0 4
2×2 Matrix<f64>:
 1 4
 9 16
2×2 Matrix<bool>:
 false false
 false false
2×2 Matrix<f64>:
 1 0
 0 2
```

A dot before an operator applies it element by element: `.*`, `.+`, `.-`, `./`,
Julia's own spellings. Shapes stretch as Julia stretches them -- along an axis
two lengths must be equal, or one of them 1 -- so a number, a row or a column
combines with a matrix. Precedence is the operator's own: `x + a .* b`
multiplies first.

`f<>` is *apply to each*, Julia's `f.(a)`: a mark written tight against a
function, as `$` and `!` are, saying how it is applied. It takes up to three
arguments, borrows them, works on a closure in parentheses, and pipes. With it
the operators that have no dotted form are ordinary functions: `f64.powf<> a 2.0`
is Julia's `a .^ 2`, and a comparison gives a matrix of `bool`. Beneath it is a
method, `a <- map f`, as `slice` is beneath the index.

## 17.9 When sizes do not fit

```toml
[dependencies]
hrs_std = "0.1"
```

```rust harsh
use hrs_std.UniformScaling

fn main$:
    let b = m~ [1 2 3; 4 5 6]
    // Adding a scaled identity needs a square matrix, and this is 2×3.
    println! "{}" (&b - (UniformScaling 2))
```

```text
DimensionMismatch: matrix is not square: dimensions are (2, 3)
```

A matrix's size is a value, known when the program runs, not a type. So a
mismatch is caught then, and reported in Julia's words: `DimensionMismatch`,
with the sizes that did not fit.
