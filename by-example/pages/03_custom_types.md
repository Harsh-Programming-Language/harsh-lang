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
    println! "{} {}" pair.0 pair.1

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
