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
