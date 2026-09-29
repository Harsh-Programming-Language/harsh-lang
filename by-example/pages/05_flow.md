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
