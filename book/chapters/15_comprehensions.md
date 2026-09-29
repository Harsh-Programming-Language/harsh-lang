# 15. Generator comprehensions

A *comprehension* builds a sequence by describing its contents: *the square of every x from 1 to 5*, *every pair (x, y) with y greater than x*. Python, Haskell and Julia have them, and data work leans on them, because they say what a collection holds instead of how to fill it. Rust has none. Harsh has four, named as Python names them: `g~`, the **generator comprehension**, which is lazy and yields its values one at a time; and the **list**, **set** and **dict comprehensions**, `list~`, `set~` and `dict~`, which collect it. The `g` is for *generator*. (A generator here means a lazy iterator; it is not Rust's unstable feature of the same name, now called coroutines.)

All four are in Harsh's *prelude*: every file can use them with no `use`. They are macros — the `~` says so — written in Harsh and expanded by `hrs`, so what they become is ordinary Rust iterator code, with nothing added at run time.

## 15.1 The shape

```rust harsh
fn main$:
    // The value first, then where it comes from.
    let squares: Vec<i32> = (g~ x * x for x in 1..6) <- collect$
    println! "{squares:?}"

    // Any pattern a `for` accepts, any iterable.
    let pairs = vec! ("ada", 36) ("alan", 41)
    let names: Vec<String> = (g~ (name <- to_uppercase$) for (name, _) in pairs) <- collect$
    println! "{names:?}"
```

```text
[1, 4, 9, 16, 25]
["ADA", "ALAN"]
```

Read `g~ x * x for x in 1..6` aloud and it says what it means: *x times x, for x in 1 to 6*. The value comes first, then `for` a pattern `in` an iterable. The pattern is any pattern a `for` loop accepts — `(name, _)` takes a tuple apart — and the iterable is anything a `for` loop accepts.

The whole call is in parentheses because the result is chained on: `(g~ …) <- collect$`. A macro call takes the rest of its line, so without the parentheses `<- collect$` would be read as part of the comprehension.

## 15.2 Conditions

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

`if` keeps only the values that satisfy it. There may be several, and they all hold: `if x % 3 == 0 if x % 2 == 1` is odd multiples of three.

Each `if` belongs to the `for` it follows. That is the one rule of the syntax, and everything else follows from it. A condition can use every name bound up to that point — `if y > x` sees both `x` and `y` — and nothing bound after it.

## 15.3 Several `for`s

```rust harsh
fn main$:
    // Three `for`s: every right triangle with sides under 30.
    let triangles: Vec<(u32, u32, u32)> =
        (g~ (a, b, c)
            for a in 1..30
            for b in a..30
            for c in b..30 if a * a + b * b == c * c
        ) <- collect$
    println!
        "{} triangles, the first {:?}"
        (triangles <- len$)
        triangles[0]

    // A condition on an outer `for` filters before the inner one runs.
    let grid: Vec<String> = (g~ (format! "{r}{c}") for r in 'a'..='c' if r != 'b' for c in 1..=2) <- collect$
    println! "{grid:?}"
```

```text
10 triangles, the first (3, 4, 5)
["a1", "a2", "c1", "c2"]
```

Each `for` runs inside the one before it, so three of them visit every `a`, then every `b` from `a` on, then every `c` from `b` on. The condition at the end sees all three. Written one clause per line, the comprehension reads like the definition it implements.

Because an `if` belongs to its `for`, where you put it decides when it runs. In the grid, `if r != 'b'` is attached to the outer loop, so the letter `b` is rejected once, before any of its numbers are looked at. Put a condition as early as its names allow and the inner loops do less work.

## 15.4 Where a condition may go

The rule that each `if` belongs to the `for` before it has a consequence: every `for` makes one more name available, and a condition can use only the names bound up to its place.

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

The test needs `a`, `b` and `c`, so it goes after the `for` that binds `c`. This, though it looks as if it says the same thing, does not compile:

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
  --> comp_too_early.hrs:6:39
   |
 6 |             for a in 1..20 if a * a + b * b == c * c
   |                                       ^
   = help: a local variable with a similar name exists (hrs 6:39)

error[E0425]: cannot find value `b` in this scope
  --> comp_too_early.hrs:6:43
   |
 6 |             for a in 1..20 if a * a + b * b == c * c
   |                                           ^
   = help: a local variable with a similar name exists (hrs 6:43)

error[E0425]: cannot find value `c` in this scope
  --> comp_too_early.hrs:6:48
   |
 6 |             for a in 1..20 if a * a + b * b == c * c
   |                                                ^
   = help: a local variable with a similar name exists (hrs 6:48)

error[E0425]: cannot find value `c` in this scope
  --> comp_too_early.hrs:6:52
   |
 6 |             for a in 1..20 if a * a + b * b == c * c
   |                                                    ^
   = help: a local variable with a similar name exists (hrs 6:52)
```

Attached to `for a`, the test runs once per `a`, before `b` and `c` have been chosen, so there is no `b` for it to read. The compiler says so on the Harsh line, with the caret under the `b`. The fix is not to rewrite the test but to move it to a `for` that comes after every name it uses — here the last one. The same rule tells you where a condition *should* go when it could go in several places: as early as its names allow, because a test on an outer `for` skips whole inner loops.

## 15.5 Lazy, and by value

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

`g~` is an iterator, and like every iterator it is lazy: nothing is computed until something asks for a value. That is why `for n in 1..` — an endless range — is fine: `take 4` asks for four values and then stops asking.

`for` takes its iterable by value, as a `for` loop does. Given the vector `words`, it would consume it. To keep the collection, give the comprehension a borrow: `for w in (words <- iter$)`. The rule is Rust's; the comprehension adds nothing to it.

## 15.6 Collecting: `list~`, `set~`, `dict~`

```rust harsh
use std.collections.(HashMap, HashSet)

fn main$:
    // The shorthands collect: a Vec, a HashSet, a HashMap.
    let evens = list~ x for x in 0..10 if x % 2 == 0
    let digits: HashSet<u32> = set~ n % 10 for n in [12, 22, 35, 45]
    let ages: HashMap<&str, u32> = dict~ name => age for (name, age) in [("ada", 36), ("alan", 41)]

    println! "{evens:?}"
    let mut d: Vec<_> = digits <- into_iter$ <- collect$
    d <- sort$
    println! "{d:?}"
    println! "{}" ages["alan"]

    // A repeated key keeps the last value, as HashMap's insert does.
    let last = dict~ (w <- len$) => w for w in ["one", "two", "three"]
    println! "{}" last[&3]
```

```text
[0, 2, 4, 6, 8]
[2, 5]
41
two
```

`list~` collects into a `Vec`, `set~` into a `HashSet`, `dict~` into a `HashMap`. They take everything `g~` takes, and a result that is collected straight away needs no parentheses: the call is the whole value.

`dict~` takes its entry as `key => value`. A key that turns up twice keeps its last value — that is what a `HashMap` does when a key is inserted again. Here "one" and "two" both have length 3, so the entry for 3 is "two". If you want every value for a key, you want a grouping, which is a different thing from a map.

`g~` with `collect$` does the same as any of them, with the type chosen by the annotation: `let v: Vec<i32> = (g~ …) <- collect$`.

## 15.7 What it stands for

A generator comprehension is a chain written another way. Here is what `g~` writes for a two-level comprehension — the Harsh that `hrs expand` shows, laid out:

```rust harsh
g~ x * 10 + y
    for x in (0..6) if x % 2 == 0 if x > 0
    for y in (0..6) if y > x if y % 2 == 1
```

becomes

```rust harsh
(0..6) <- into_iter$
<- flat_map (move |x|
    ((true && (x % 2 == 0) && (x > 0)) <- then (||
        (0..6) <- into_iter$
        <- flat_map (move |y|
            ((true && (y > x) && (y % 2 == 1)) <- then (|| x * 10 + y))))))
<- flatten$
```

Each `for` becomes a `flat_map` whose closure binds the pattern. That level's `if`s are folded into one test, and `then` turns the test into an `Option`: the value when it holds, nothing when it does not. A `for` nested inside another lives in the outer closure's `then` — which is why its conditions can see `x`. The leading `true &&` is what makes a level with no `if` at all still read as a test.

Two things are easy to misplace. **The `flatten$` is part of the generator.** The outer level yields one inner sequence per `x`, and `g~` flattens its own levels — every level but the innermost ends with it — so you never write it; `(g~ …) <- flatten$` would try to flatten the numbers themselves, and the compiler stops it. **The `collect$` is not part of it.** A generator is lazy, so it collects nothing; you write `(g~ …) <- collect$`, or use `list~`, `set~` or `dict~`, which collect for you.

The proof that the two are the same:

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

And for a single level, the chain you would have written by hand:

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

The expansion is exactly what you would write yourself, so a comprehension costs nothing that the chain does not.

Which to write is a question of reading. A comprehension says what the result contains; a chain says what is done to the input, step by step. Nested loops, a filter on each level, a value built from several names — those read better as a comprehension. A long pipeline of transformations reads better as a chain.

## 15.8 Your own `g`

The prelude's names are defaults, not reserved words. If a file defines its own `macro_rules~ g`, that one is used in the file. The prelude's is still there under its full name, `hrs_std.g~`. The shorthands use the full name themselves, so they keep working whatever you define.

## 15.9 What you have

`g~ value for pattern in iterable if condition for pattern in iterable if condition …`, the generator comprehension — as many `for`s as you like, each with its own `if`s, each condition seeing only the names bound before it, so a test goes after the last `for` whose name it uses. It is a lazy iterator: it flattens its own levels, and it collects nothing until asked. `for` takes its iterable by value. `list~`, `set~` and `dict~ key => value for …` collect into a `Vec`, a `HashSet` and a `HashMap`. A comprehension is written on one line or one clause per line, and is isolated in parentheses when chained on.

Next: matrices, and the linear algebra that makes Harsh a language for data.
