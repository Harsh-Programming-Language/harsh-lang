# 19. Patterns and matching

You have been writing patterns since chapter 2: `Ok n`, `Some max`, `(key, value)`, `Point { x, y }`. A *pattern* is a shape that a value is tested against and, when it fits, taken apart into names. This chapter collects every place a pattern can appear and every form one can take, so that the next time a value has a shape you can write the shape directly instead of reaching for it with methods. Harsh's contribution is the one you know: a variant with fields is matched by juxtaposition, `Coin.Quarter state`, the way it is constructed.

## 19.1 Where patterns appear

```
fn main$:
    // match arms
    let x = Some 3

    match x\
        None => println! "none"
        Some i => println! "some {i}"
    // if let, with else if and else if let

    let favorite_color: Option<&str> = None
    let is_tuesday = false
    let age: Result<u8, _> = "34" <- parse$

    if let Some color = favorite_color:
        println! "Using your favorite color, {color}, as the background"
    else if is_tuesday:
        println! "Tuesday is green day!"
    else if let Ok age = age:
        if age > 30: println! "Using purple as the background color" else: println! "Using orange"
    else:
        println! "Using blue as the background color"
    // while let

    let mut stack = vec! [1, 2, 3]

    while let Some top = stack <- pop$:
        println! "{top}"
    // for

    let v = vec! ['a', 'b', 'c']

    for (index, value) in v <- iter$ <- enumerate$:
        println! "{value} is at index {index}"
    // let

    let (a, b, c) = (1, 2, 3)
    println! "{a} {b} {c}"

    // function parameters
    fn print_coordinates (&(x, y): &(i32, i32)):
        println! "Current location: ({x}, {y})"

    print_coordinates (&(3, 5))
```

```text
some 3
Using purple as the background color
3
2
1
a is at index 0
b is at index 1
c is at index 2
1 2 3
Current location: (3, 5)
```

Six places. `match` arms, where every case must be covered. `if let`, for one case, with `else if` and `else if let` chaining conditions and patterns freely — the example mixes a plain boolean between two pattern tests. `while let`, looping while a pattern keeps matching — popping a stack until it is empty. `for`, whose loop variable is a pattern: `(index, value)` takes apart the pairs `enumerate$` yields. `let`, whose left side is a pattern: `let (a, b, c) = …` is three bindings at once, and the plain `let x = 5` you have written a thousand times is a pattern too, one that matches anything. And function parameters, which are patterns as well: `&(x, y): &(i32, i32)` takes a reference to a tuple apart in the signature. The nested `fn` is legal Rust — an item inside a function, visible only there.

## 19.2 Refutability

A pattern that can fail to match is *refutable*; one that always matches is *irrefutable*. `Some x` is refutable, `x` and `(a, b)` are not. `let`, `for` and function parameters need an irrefutable pattern, because they have nothing to do when it fails; `if let`, `while let` and `match` arms accept a refutable one, because failing is what their `else`, their end, and their next arm are for:

```
fn main$:
    let some_option_value: Option<i32> = None
    let Some x = some_option_value
    println! "{x}"
```

```text
error[E0005]: refutable pattern in local binding
  --> refutable.hrs:3:9
   |
 3 |     let Some x = some_option_value
   |         ^^^^^^^^ pattern `None` not covered
   = note: `let` bindings require an "irrefutable pattern", like a `struct` or an `enum` with only one variant
   = note: for more information, visit https://doc.rust-lang.org/book/ch18-02-refutability.html
   = note: the matched value is of type `Option<i32>`
   = help: you might want to use `let else` to handle the variant that isn't matched (hrs 3:35)
```

`let Some x = value` has nowhere to go when `value` is `None`, and the help offers the fix from chapter 6, `let … else`. The other direction is a warning rather than an error: `if let x = 5` is legal and pointless, and the compiler says so.

## 19.3 The forms

### Literals, names, ranges, alternatives

```
fn main$:
    let x = 1

    match x\
        1 => println! "one"
        2 => println! "two"
        3 => println! "three"
        _ => println! "anything"
    // named variables shadow inside an arm

    let x = Some 5
    let y = 10

    match x\
        Some 50 => println! "Got 50"
        Some y => println! "Matched, y = {y}"     // a new y, bound to 5
        _ => println! "Default case, x = {x:?}"

    println! "at the end: x = {x:?}, y = {y}"

    // or-patterns and ranges
    let x = 5

    match x\
        1 | 2 => println! "one or two"
        3..=5 => println! "three through five"
        _ => println! "anything"

    let c = 'c'

    match c\
        'a'..='j' => println! "early ASCII letter"
        'k'..='z' => println! "late ASCII letter"
        _ => println! "something else"
```

```text
one
Matched, y = 5
at the end: x = Some(5), y = 10
three through five
early ASCII letter
```

A literal matches itself. A name matches anything and binds it — and inside an arm it is a *new* variable that shadows any outer one, which is the trap in the second `match`: `Some y` does not compare against the outer `y`, it binds a fresh `y` to `5`, and the message says so. (The guard in 19.4 is how to compare against an outer variable.) `|` gives alternatives; `..=` matches an inclusive range of numbers or characters — ranges are the one pattern form that is not just a literal or a structure, and they are allowed only for those two types, where the compiler can check that a set of ranges covers everything.

### Destructuring

```
struct Point
    x: i32
    y: i32

enum Color
    Rgb i32 i32 i32
    Hsv i32 i32 i32

enum Message
    Quit

    Move\
        x: i32
        y: i32

    Write String
    ChangeColor Color

fn main$:
    let p = Point\ x = 0, y = 7
    let Point\ x: a, y: b = p           // fields into new names
    println! "{a} {b}"

    let Point\ x, y = p                 // shorthand: same names
    println! "{x} {y}"

    match p\
        Point\ x, y: 0 => println! "On the x axis at {x}"
        Point\ x: 0, y => println! "On the y axis at {y}"
        Point\ x, y => println! "On neither axis: ({x}, {y})"

    let msgs =
        [
            Message.Quit,
            (Message.Move\ x = 1, y = 2),
            (Message.Write (String.from "hi")),
            (Message.ChangeColor (Color.Hsv 0 160 255)),
        ]

    for msg in msgs:
        match msg\
            Message.Quit => println! "The Quit variant has no data to destructure."
            Message.Move\ x, y => println! "Move in the x direction {x} and in the y direction {y}"
            Message.Write text => println! "Text message: {text}"
            Message.ChangeColor (Color.Rgb r g b) => println! "Change color to red {r}, green {g}, blue {b}"
            Message.ChangeColor (Color.Hsv h s v) => println! "Change color to hue {h}, saturation {s}, value {v}"

    // nested, all at once

    let ((feet, inches), Point\ x, y) = ((3, 10), Point\ x = 3, y = -10)
    println! "{feet} {inches} {x} {y}"
```

```text
0 7
0 7
On the y axis at 7
The Quit variant has no data to destructure.
Move in the x direction 1 and in the y direction 2
Text message: hi
Change color to hue 0, saturation 160, value 255
3 10 3 -10
```

A struct pattern is a field list like any other, marked with `\`: `Point\ x: a, y: b` binds the fields to new names — a rename keeps the colon, since here the field is not being given a value but matched — `Point\ x, y` is the shorthand for the same names, and a field may hold a literal to test it: `Point\ x, y: 0` matches only points on the x axis and binds `x`. Enum variants are matched by the shape that built them: `Message.Quit` bare, `Message.Move { x, y }` with the record's fields, `Message.Write text` with one juxtaposed name, and `Message.ChangeColor (Color.Hsv h s v)` with the inner variant's pattern *isolated in parentheses*, because it is one argument and it has structure. The isolating parentheses are Harsh's argument rule, applied to a pattern — the same rule as `Some (i + 1)` on the constructing side. And patterns nest to any depth: the last `let` takes a tuple of a tuple and a struct apart in one line.

### Ignoring

```
fn foo (_: i32) (y: i32):                // an unused parameter, by name
    println! "This code only uses the y parameter: {y}"

fn main$:
    foo 3 4

    // `_` in a nested position
    let mut setting_value = Some 5
    let new_setting_value = Some 10

    match (setting_value, new_setting_value)\
        (Some _, Some _) => println! "Can't overwrite an existing customized value"
        _ => setting_value = new_setting_value

    println! "setting is {setting_value:?}"

    let numbers = (2, 4, 8, 16, 32)

    match numbers\
        (first, _, third, _, fifth) => println! "Some numbers: {first}, {third}, {fifth}"
    // `_x` binds and silences the warning; `_` does not bind at all

    let s = Some (String.from "Hello!")

    if let Some _ = s:                     // `Some _s` here would move the String out
        println! "found a string"

    println! "{s:?}"

    // `..` for the rest
    struct Point
        x: i32
        y: i32
        z: i32

    let origin = Point\ x = 0, y = 0, z = 0

    match origin\
        Point\ x, .. => println! "x is {x}"

    match numbers\
        (first, .., last) => println! "Some numbers: {first}, {last}"
```

```text
This code only uses the y parameter: 4
Can't overwrite an existing customized value
setting is Some(5)
Some numbers: 2, 8, 32
found a string
Some("Hello!")
x is 0
Some numbers: 2, 32
```

`_` matches anything and binds nothing. In a parameter it is a value the function ignores (useful when a trait signature requires it); nested, it tests a shape without taking it apart — `(Some _, Some _)` is "both set" without caring what to; in a tuple it skips positions. `_` and a name starting with `_` differ in one way that matters: `_x` *binds* (and only silences the unused-variable warning), so `if let Some _s = s` would move the `String` out of `s`, while `if let Some _ = s` does not, and `s` is still printable after. `..` ignores *all remaining* parts: `Point { x, .. }` for a struct, `(first, .., last)` for a tuple, and it must be unambiguous — `(.., second, ..)` is an error, since the compiler cannot tell which position `second` means.

## 19.4 Guards and bindings

```
enum Message
    Hello\
        id: i32

fn main$:
    // a match guard: an extra condition after the pattern
    let num = Some 4

    match num\
        Some x if x % 2 == 0 => println! "The number {x} is even"
        Some x => println! "The number {x} is odd"
        None => ()
    // a guard solves the shadowing problem from the literals example

    let x = Some 5
    let y = 10

    match x\
        Some 50 => println! "Got 50"
        Some n if n == y => println! "Matched, n = {n}"
        _ => println! "Default case, x = {x:?}"
    // a guard applies to the whole or-pattern

    let x = 4
    let y = false

    match x\
        4 | 5 | 6 if y => println! "yes"
        _ => println! "no"
    // `@` binds a value while also testing it against a range

    let msg = Message.Hello\ id = 5

    match msg\
        Message.Hello { id: id_variable @ 3..=7 } => println! "Found an id in range: {id_variable}"
        Message.Hello { id: 10..=12 } => println! "Found an id in another range"
        Message.Hello\ id => println! "Found some other id: {id}"
```

```text
The number 4 is even
Default case, x = Some(5)
no
Found an id in range: 5
```

A *match guard* is an `if` after the pattern: the arm matches only if the pattern fits *and* the condition holds. `Some x if x % 2 == 0` tests the bound value; `Some n if n == y` compares against the outer `y` — the answer to the shadowing trap, since a guard is an expression and sees the enclosing scope. A guard applies to the whole of an or-pattern, `4 | 5 | 6 if y`, not just the last alternative. Guards are not counted for exhaustiveness — the compiler cannot see through an arbitrary condition — so a `match` whose arms all have guards still needs a catch-all.

`@` binds a name to a value *while* testing it: `id_variable @ 3..=7` matches ids from 3 to 7 and gives the arm the actual id, where `3..=7` alone would test without binding and `id` alone would bind without testing. It is the form for "I want to know it is in this range, and I want the value".

## 19.5 What you have

Patterns appear in `match`, `if let`, `while let`, `for`, `let` and parameters; `let`, `for` and parameters need irrefutable ones. The forms: literals, names (which shadow), `|`, `..=` ranges, tuple and struct and variant destructuring to any depth, `_` and `_name` and `..` to ignore, `if` guards, and `x @ pattern` to bind and test. Harsh writes a variant's payload by juxtaposition and isolates a nested pattern in parentheses, exactly as it writes the constructing expression.

Next: the advanced features — `unsafe`, the corners of traits and types, function pointers and returned closures, and macros — which most programs never need and every Rust programmer eventually meets.
