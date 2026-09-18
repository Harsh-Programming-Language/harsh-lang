# 6. Enums and pattern matching

A struct says "all of these, together". An *enum* says "exactly one of these". A value of an enum type is one of a fixed set of named *variants*, and each variant may carry data of its own — so an enum is both a set of alternatives and, when the variants have payloads, a way of saying "this value is one of several shapes, and here is which". Rust programs are built on two of them, `Option` and `Result`, and on `match`, the construct that takes an enum apart and refuses to let you forget a case.

## 6.1 Defining an enum

An IP address is version four or version six — never both, never neither. That is an enum:

```
#[derive Debug]
enum IpAddrKind
    V4
    V6

fn route ip_kind: IpAddrKind:
    println! "routing {:?}" ip_kind

fn main$:
    let four = IpAddrKind.V4
    let six = IpAddrKind.V6
    route four
    route six
```

```text
routing V4
routing V6
```

`enum IpAddrKind` and two variants, one per line, no commas, the same layout as a struct's fields, and markless for the same reason: only a name can follow `enum`. A variant is named through the type, `IpAddrKind.V4`, with a dot, because it is a path: the variant lives inside the type's namespace. Both values have the type `IpAddrKind`, so one function takes either.

### Variants with data

The kind alone is not much use; an address has a value. A variant can carry one:

```
#[derive Debug]
enum IpAddr
    V4 u8 u8 u8 u8
    V6 String

fn main$:
    let home = IpAddr.V4 127 0 0 1
    let loopback = IpAddr.V6 (String.from "::1")
    println! "{:?} {:?}" home loopback
```

```text
V4(127, 0, 0, 1) V6("::1")
```

`V4 u8 u8 u8 u8` is a variant with four fields and `V6 String` a variant with one — the variant applied to its payload's types, exactly as a tuple struct is declared. Constructing is application: `IpAddr.V4 127 0 0 1` applies the variant to four arguments, and `IpAddr.V6 (String.from "::1")` to one, isolated because it is a call. Each variant is a constructor function for the enum, and the two variants may carry different types — something no struct can express.

Variants can take any shape a struct can — and that sentence is the whole of what an enum is. An enum is a *union* of variants, and each variant is one of chapter 5's three forms: a unit-like struct, a record struct, or a tuple struct, spelled exactly as the struct of that form is spelled, with the word `struct` and the name's own line taken away. (A type built this way — a choice among bundles — is what Haskell and its relatives call an *algebraic data type*; Rust's enums are that, with the word left out.)

```
#[derive Debug]
enum Message
    Quit                          // a unit-like struct

    Move                          // a record struct
        x: i32
        y: i32

    Write String                  // a tuple struct
    ChangeColor i32 i32 i32       // a tuple struct

impl Message
    fn call (&self):
        println! "calling {:?}" self

fn main$:
    let msgs =
        [
            Message.Quit,
            (Message.Move\ x = 1, y = 2),
            (Message.Write (String.from "hi")),
            (Message.ChangeColor 0 160 255),
        ]

    for m in &msgs:
        m <- call$
```

```text
calling Quit
calling Move { x: 1, y: 2 }
calling Write("hi")
calling ChangeColor(0, 160, 255)
```

> **Harsh —** `Quit` is a unit-like struct; `Move` with `x` and `y` beneath it is a record struct; `Write String` and `ChangeColor i32 i32 i32` are tuple structs, the name applied to the payload's types. Each is written as chapter 5 wrote it, and each has the inline form chapter 5 had too — `Move\ x: i32, y: i32` on one line — so the same enum reads either way, and which to use is the same judgement as for a struct: what the names and their lengths make clearest.

```
enum Message
    Quit
    Move\ x: i32, y: i32
    Write String
    ChangeColor i32 i32 i32

fn main$:
    let m = Message.Move\ x = 1, y = 2
    if let Message.Move\ x, y = m:
        println! "moved to {x},{y}"
```

```text
moved to 1,2
```

`Quit` carries nothing; `Move` opens a record variant with named fields beneath it, exactly as a struct's are laid out; `Write String` and `ChangeColor i32 i32 i32` are tuple variants, their payload types juxtaposed after the name. A record variant's value is a literal like any other, `Message.Move\ x = 1, y = 2`. Without the enum this would be four separate struct types, and no function could take "a message" — with it, `impl Message:` gives all four a method, and `m <- call$` works on any of them. (`for m in &msgs` borrows the array so the messages are not moved out of it; chapter 8 makes that habit.)

### `Option`

Rust has no null. Where another language returns a value that might be null, Rust returns a value that might be *absent*, and says so in the type:

```
fn main$:
    let some_number = Some 5
    let some_char = Some 'e'
    let absent_number: Option<i32> = None
    println! "{:?} {:?} {:?}" some_number some_char absent_number
```

```text
Some(5) Some('e') None
```

`Option<T>` is an enum with two variants, `Some T` — there is a value, here it is — and `None`. It is so central that both variants are in scope without a prefix: you write `Some 5` and `None`, not `Option.Some 5`. The type of `absent_number` has to be written, because `None` alone does not say what kind of value is absent.

What `Option` buys you is that an `Option<i8>` is *not* an `i8`, and the compiler will not let you treat it as one:

```
fn main$:
    let x: i8 = 5
    let y: Option<i8> = Some 5
    let sum = x + y
    println! "{}" sum
```

```text
error[E0277]: cannot add `Option<i8>` to `i8`
  --> option_add.hrs:4:17
   |
 4 |     let sum = x + y
   |                 ^ no implementation for `i8 + Option<i8>`
   = help: the trait `Add<Option<i8>>` is not implemented for `i8`
   = help: the following other types implement trait `Add<Rhs>`:
  <i8 as Add>
  <i8 as Add<&i8>>
  <&'a i8 as Add<i8>>
  <&i8 as Add<&i8>>
```

In a language with null, `x + y` compiles and fails at runtime when `y` happens to be null. Here it does not compile, because a value that might be absent cannot be added until you have said what happens when it is. To use the `i8` inside, you have to take the `Option` apart, and handling the `None` case is part of taking it apart. That is the whole idea, and `match` is how it is done.

## 6.2 `match`

`match` takes a value and a list of *arms*, each a pattern and the code to run when the pattern fits. The first arm whose pattern matches wins:

```
enum Coin
    Penny
    Nickel
    Dime
    Quarter

fn value_in_cents coin: Coin -> u8:
    match coin:
        Coin.Penny => do:
            println! "Lucky penny!"
            1
        Coin.Nickel => 5
        Coin.Dime => 10
        Coin.Quarter => 25

fn main$:
    println! "{}" (value_in_cents Coin.Penny)
    println! "{}" (value_in_cents Coin.Quarter)
```

```text
Lucky penny!
1
25
```

`match coin:` opens the arms; each is `pattern => expression`, one per line. An arm's expression is its value, and the `match` is worth whichever arm ran — so `value_in_cents` returns the `u8` from the arm that matched. An arm that needs several statements opens a block with `=> do:`, and the block's last line is its value, as everywhere.

> **Harsh —** One arm per line, and no comma after it: the line ending is what separates arms, and a `,` at the end of one is an error naming the newline. Commas separate arms only when several share a line.

### Patterns that bind

When a variant carries data, the pattern names it and the arm can use it:

```
#[derive Debug]
enum UsState
    Alabama
    Alaska

enum Coin
    Penny
    Nickel
    Dime
    Quarter UsState

fn value_in_cents coin: Coin -> u8:
    match coin:
        Coin.Penny => 1
        Coin.Nickel => 5
        Coin.Dime => 10
        Coin.Quarter state => do:
            println! "State quarter from {:?}!" state
            25

fn main$:
    println! "{}" (value_in_cents (Coin.Quarter UsState.Alaska))
```

```text
State quarter from Alaska!
25
```

`Coin.Quarter state` — the pattern has the shape of the constructor, with a variable where the value goes. When a `Quarter` matches, `state` is bound to the `UsState` inside it for the length of the arm. This is the same juxtaposition that constructs: `Coin.Quarter UsState.Alaska` builds one, `Coin.Quarter state` takes one apart. A pattern is an application throughout — `Some n`, `Ok value`, `Circle r` — and a record variant is taken apart with the same mark that builds it, `Move\ x, y`, as chapter 5 did for a struct.

### Matching `Option`

Now the `Option` from the previous section can be used:

```
fn plus_one x: Option<i32> -> Option<i32>:
    match x:
        None => None
        Some i => Some (i + 1)

fn main$:
    let five = Some 5
    let six = plus_one five
    let none = plus_one None
    println! "{:?} {:?}" six none
```

```text
Some(6) None
```

`None => None` and `Some i => Some (i + 1)`: two arms, two variants, and inside the second `i` is the `i32`, so `i + 1` is ordinary arithmetic; `Some (i + 1)` wraps the result back up, its argument isolated because of the `+`. Combining `match` and enums this way is how most Rust code handles values that might not be there; after a while you stop noticing it.

### Matches are exhaustive

Leave a case out and the compiler stops you:

```
fn plus_one x: Option<i32> -> Option<i32>:
    match x:
        Some i => Some (i + 1)

fn main$:
    println! "{:?}" (plus_one (Some 5))
```

```text
error[E0004]: non-exhaustive patterns: `None` not covered
  --> non_exhaustive.hrs:2:11
   |
 2 |     match x:
   |           ^ pattern `None` not covered
   = note: `Option<i32>` defined here
   = note: the matched value is of type `Option<i32>`
   = help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown (hrs 3:31)
```

Every possible value must be covered by some arm. This is what makes `Option` safe: you cannot forget the `None` case, because forgetting it is a compile error. The same holds for your own enums — add a variant, and every `match` that does not handle it fails to build, which is exactly the list of places you needed to look.

### Catch-all patterns

When only some values matter, the last arm can catch the rest:

```
fn add_fancy_hat$:
    println! "fancy hat"
fn remove_fancy_hat$:
    println! "no hat"
fn move_player spaces: u8:
    println! "move {} spaces" spaces

fn main$:
    for dice_roll in [3, 7, 4]:
        match dice_roll:
            3 => add_fancy_hat$
            7 => remove_fancy_hat$
            other => move_player other

    // when the value is not needed, `_` matches anything and binds nothing

    let roll = 9

    match roll:
        3 => add_fancy_hat$
        7 => remove_fancy_hat$
        _ => ()
```

```text
fancy hat
no hat
move 4 spaces
```

`other => move_player other` binds whatever did not match `3` or `7` to `other` and uses it. When the value is not needed, `_` matches anything and binds nothing, and `_ => ()` says "and do nothing" — `()` is the unit value, and a `match` whose arms are all unit is a statement. The catch-all must be last, since arms are tried in order and nothing after it could ever run.

## 6.3 `if let` and `let … else`

A `match` with one arm that matters and a `_ => ()` is a lot of lines for "if this is a `Some`, do this":

```
fn main$:
    let config_max = Some 3u8

    // The match way: one arm that matters, one that does nothing.
    match config_max:
        Some max => println! "The maximum is configured to be {max}"
        _ => ()

    // The `if let` way: the same, in one line.

    if let Some max = config_max:
        println! "The maximum is configured to be {max}"
```

```text
The maximum is configured to be 3
The maximum is configured to be 3
```

`if let Some max = config_max:` is a condition that is also a pattern: if `config_max` matches `Some max`, bind `max` and run the block. It is `match` with one arm and no exhaustiveness check — you are choosing to ignore the other cases, and the syntax says so. Use it when one case is all you want; use `match` when forgetting a case would be a bug.

`if let` takes an `else`, which runs for everything the pattern did not match:

```
#[derive Debug]
enum UsState
    Alabama
    Alaska

enum Coin
    Penny
    Quarter UsState

fn main$:
    let coins = [Coin.Penny, (Coin.Quarter UsState.Alabama), Coin.Penny]
    let mut count = 0

    for coin in coins:
        if let Coin.Quarter state = coin:
            println! "State quarter from {:?}!" state

        else:
            count += 1

    println! "{} non-quarters" count
```

```text
State quarter from Alabama!
2 non-quarters
```

Sometimes the pattern is a guard at the top of a function: if the value has the right shape, carry on with its contents; otherwise leave. `let … else` is that, without nesting the rest of the function inside an `if`:

```
#[derive Debug]
enum UsState
    Alabama
    Alaska

impl UsState
    fn existed_in (&self) (year: u16) -> bool:
        match self:
            UsState.Alabama => year >= 1819
            UsState.Alaska => year >= 1959

enum Coin
    Penny
    Quarter UsState

fn describe_state_quarter coin: Coin -> Option<String>:
    let Coin.Quarter state = coin else:
        return None

    if state <- existed_in 1900:
        Some (format! "{state:?} is pretty old, for America!")
    else:
        Some (format! "{state:?} is relatively new.")

fn main$:
    println!
        "{:?}"
        (describe_state_quarter (Coin.Quarter UsState.Alabama))
    println!
        "{:?}"
        (describe_state_quarter (Coin.Quarter UsState.Alaska))
    println! "{:?}" (describe_state_quarter Coin.Penny)
```

```text
Some("Alabama is pretty old, for America!")
Some("Alaska is relatively new.")
None
```

`let Coin.Quarter state = coin else:` — if `coin` is a `Quarter`, `state` is bound *for the rest of the function*, not just for a block; if it is not, the `else` runs, and the `else` must leave (here with `return`), because there is no `state` to continue with. The happy path stays flat, which is the reason this form exists. The `else` block may also be inline, `else: return None`, when it is one expression.

## 6.4 What you have

An enum is one of several named variants, each carrying nothing, a tuple, or named fields; variants are constructed and matched by juxtaposition, `IpAddr.V4 127 0 0 1`, `Coin.Quarter state`. `Option<T>` is `Some T` or `None` and is Rust's null, made safe by the type. `match` takes a value apart with patterns, binds what is inside, and must cover every case; `_` and `other` catch the rest. `if let` for one case, `let … else` to guard and continue.

Next: modules — how a program larger than one file is organised, and what `pub` and `use` actually do.
