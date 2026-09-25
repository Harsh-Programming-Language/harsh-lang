# 9. Generics and traits

## 9.1 Generics

```
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

```
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

```
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

```
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
