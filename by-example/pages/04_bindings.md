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
 2 |     let x = 1
   |         - first assignment to `x`
 3 |     x = 2
   |     ^^^^^ cannot assign twice to immutable variable
   = help: consider making this binding mutable (hrs 2:9)
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
